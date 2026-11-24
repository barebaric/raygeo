//! Turns the chunk tree of a CorelDRAW file into page geometry.
//!
//! The drawing lives in `LIST page` > `LIST layr` > `LIST obj ` records.
//! An object carries a `loda` argument table (its kind and arguments) and
//! a `trfd` transform; geometry comes from argument type 30, read by the
//! object's kind. Coordinates are inches from the page centre, Y up; the
//! page origin is moved to the bottom-left corner at the end.
//!
//! Only drawing geometry is extracted. Fills, outlines, text, bitmaps and
//! special objects are not converted: they are counted and reported in
//! [`CdrDocument::issues`].
//!
//! The chunk layout and record encodings are the ones documented in
//! TraceDraw's clean-room `docs/cdr-format.md`.

use std::collections::{BTreeSet, HashMap};

use crate::formats::coreldraw::container;
use crate::formats::coreldraw::error::{Error, Result};
use crate::formats::coreldraw::loda;
use crate::formats::coreldraw::riff::{self, Chunk, Tree};
use crate::formats::coreldraw::stlt;
use crate::formats::coreldraw::text;
use crate::formats::coreldraw::CdrDocument;
use crate::geo::geometry::Geometry;
use crate::geo::types::Point3D;

/// Bezier control offset that approximates a quarter circle.
const KAPPA: f64 = 0.552_284_749_830_793_4;

const KIND_RECTANGLE: u32 = 0x01;
const KIND_ELLIPSE: u32 = 0x02;
const KIND_CURVE: u32 = 0x03;
const KIND_ARTISTIC_TEXT: u32 = 0x04;
const KIND_BITMAP: u32 = 0x05;
const KIND_PARAGRAPH_TEXT: u32 = 0x06;
const KIND_POLYGON: u32 = 0x14;
const KIND_MESH: u32 = 0x20;
const KIND_PATH: u32 = 0x25;
const KIND_CONNECTOR: u32 = 0x26;

/// Argument type of an object's coordinates.
const ARG_COORDINATES: u32 = 30;
/// Argument type of a layer or page name.
const ARG_NAME: u32 = 1000;
/// Argument type of a page's own size.
const ARG_PAGE_SIZE: u32 = 19130;

/// Layer kinds that hold no drawing content.
const LAYER_KIND_GRID: u32 = 0x0b;
const LAYER_KIND_GUIDES: u32 = 0x0c;
const LAYER_KIND_DESKTOP: u32 = 0x11;

/// Number encoding of one file version.
#[derive(Debug, Clone, Copy)]
struct Format {
    version: u16,
    /// Width of the header integers: 2 bytes before version 6, 4 after.
    step: usize,
    /// Coordinate units per inch: 1000 before version 6, 254000 after.
    unit: f64,
}

impl Format {
    fn new(version: u16) -> Self {
        Format {
            version,
            step: if version < 6 { 2 } else { 4 },
            unit: if version < 6 { 1000.0 } else { 254_000.0 },
        }
    }

    fn unsigned(&self, d: &[u8], at: usize) -> Option<u32> {
        if self.version < 6 {
            self.u16(d, at).map(u32::from)
        } else {
            self.u32(d, at)
        }
    }

    fn u16(&self, d: &[u8], at: usize) -> Option<u16> {
        d.get(at..at + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
    }

    fn u32(&self, d: &[u8], at: usize) -> Option<u32> {
        d.get(at..at + 4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    /// A coordinate, in inches.
    fn coord(&self, d: &[u8], at: usize) -> Option<f64> {
        if self.version < 6 {
            self.u16(d, at).map(|v| f64::from(v as i16) / 1000.0)
        } else {
            self.u32(d, at).map(|v| f64::from(v as i32) / 254_000.0)
        }
    }

    /// An angle, in radians, counter-clockwise from the X axis.
    fn angle(&self, d: &[u8], at: usize) -> Option<f64> {
        if self.version < 6 {
            self.u16(d, at)
                .map(|v| (f64::from(v as i16) / 10.0).to_radians())
        } else {
            self.u32(d, at)
                .map(|v| (f64::from(v as i32) / 1_000_000.0).to_radians())
        }
    }

    fn double(&self, d: &[u8], at: usize) -> Option<f64> {
        d.get(at..at + 8).map(|s| {
            f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
        })
    }

    /// An f64 in coordinate units, in inches.
    fn double_coord(&self, d: &[u8], at: usize) -> Option<f64> {
        self.double(d, at).map(|v| v / self.unit)
    }
}

/// A 2D affine transform: `x' = a x + c y + tx`, `y' = b x + d y + ty`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Affine {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

impl Affine {
    const IDENTITY: Affine = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    /// Applies `self` first, then `next`.
    fn then(self, next: Affine) -> Affine {
        Affine {
            a: next.a * self.a + next.c * self.b,
            b: next.b * self.a + next.d * self.b,
            c: next.a * self.c + next.c * self.d,
            d: next.b * self.c + next.d * self.d,
            tx: next.a * self.tx + next.c * self.ty + next.tx,
            ty: next.b * self.tx + next.d * self.ty + next.ty,
        }
    }

    fn apply(self, geometry: &mut Geometry) {
        geometry.transform_2d(self.a, self.c, self.b, self.d, self.tx, self.ty);
    }
}

/// What could not be converted, counted for the issues.
#[derive(Default)]
struct Skipped {
    text: usize,
    bitmaps: usize,
    unreadable: usize,
    kinds: BTreeSet<u32>,
}

struct Reader<'a> {
    format: Format,
    tree: &'a Tree,
    main: &'a [u8],
    skipped: Skipped,
    styles: HashMap<u32, stlt::Style>,
    fonts: HashMap<u16, String>,
}

/// Reads a CorelDRAW file into one page of geometry.
pub(crate) fn parse(data: &[u8]) -> Result<CdrDocument> {
    if data.starts_with(b"WL") {
        return Err(Error::Unsupported(
            "CorelDRAW 1 and 2 (WL) files are not supported".into(),
        ));
    }
    if is_cmx(data) {
        return Err(Error::Unsupported("CMX files are not supported".into()));
    }
    let container = container::detect(data)?;
    let riff = container::riff_stream(data, &container)?;
    let externals = container::external_streams(data, &container);
    let tree =
        riff::parse_with_externals(&riff, externals, container.version.0)?;
    let version = read_version(&tree, &riff).unwrap_or(container.version.0);
    let format = Format::new(version);
    let mut reader = Reader {
        format,
        tree: &tree,
        main: &riff,
        skipped: Skipped::default(),
        styles: HashMap::new(),
        fonts: HashMap::new(),
    };
    reader.styles = reader.read_styles();
    reader.fonts = reader.read_fonts();
    reader.read_document()
}

fn is_cmx(data: &[u8]) -> bool {
    data.get(0..4) == Some(b"RIFF")
        && data
            .get(8..11)
            .is_some_and(|form| form.eq_ignore_ascii_case(b"CMX"))
}

fn read_version(tree: &Tree, main: &[u8]) -> Option<u16> {
    let mut version = None;
    tree.root.walk(&mut |chunk, _| {
        if version.is_none() && &chunk.id == b"vrsn" {
            let data = tree.data(main, chunk);
            if let Some(bytes) = data.get(..2) {
                version = Some(u16::from_le_bytes([bytes[0], bytes[1]]));
            }
        }
    });
    version
        .filter(|value| *value >= 300)
        .map(|value| value / 100)
}

impl<'a> Reader<'a> {
    fn data(&self, chunk: &Chunk) -> &'a [u8] {
        self.tree.data(self.main, chunk)
    }

    fn read_document(&mut self) -> Result<CdrDocument> {
        let root = &self.tree.root;
        let (mut width, mut height) = self.page_size(root);
        let mut pages = Vec::new();
        root.walk(&mut |chunk, _| {
            if chunk.list_type == Some(*b"page") {
                pages.push(chunk);
            }
        });
        let pages: Vec<&Chunk> = pages
            .into_iter()
            .filter(|page| !self.is_master_page(page))
            .collect();
        let page_count = pages.len();
        let mut layers = Vec::new();
        if let Some(page) = pages.first() {
            if let Some((w, h)) = self.page_loda_size(page) {
                width = w;
                height = h;
            }
            layers = self.read_page(page);
        }
        if width <= 0.0 || height <= 0.0 {
            if let Some((w, h, x, y)) = content_bounds(&layers) {
                for (_, geometry) in &mut layers {
                    geometry.transform_2d(1.0, 0.0, 0.0, 1.0, -x, -y);
                }
                width = w;
                height = h;
            }
        }
        if width > 0.0 && height > 0.0 {
            for (_, geometry) in &mut layers {
                geometry.transform_2d(
                    1.0,
                    0.0,
                    0.0,
                    1.0,
                    width / 2.0,
                    height / 2.0,
                );
            }
        }
        layers.retain(|(_, geometry)| !geometry.is_empty());
        Ok(CdrDocument {
            page_width: width,
            page_height: height,
            page_count,
            layers,
            issues: self.issues(),
        })
    }

    fn issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        match self.skipped.text {
            0 => {}
            1 => {
                issues.push("1 text object is not converted to outlines".into())
            }
            count => issues.push(format!(
                "{count} text objects are not converted to outlines"
            )),
        }
        match self.skipped.bitmaps {
            0 => {}
            1 => issues.push("1 bitmap object is not imported".into()),
            count => {
                issues.push(format!("{count} bitmap objects are not imported"))
            }
        }
        for kind in &self.skipped.kinds {
            issues.push(format!("object type 0x{kind:02x} is not read"));
        }
        if self.skipped.unreadable > 0 {
            issues.push(format!(
                "{} object(s) could not be read",
                self.skipped.unreadable
            ));
        }
        issues
    }

    fn is_master_page(&self, page: &Chunk) -> bool {
        page.find(b"flgs")
            .map(|flags| self.data(flags).get(2).is_some_and(|byte| *byte != 0))
            .unwrap_or(false)
    }

    /// Page size from the document's `mcfg` record, in inches.
    fn page_size(&self, root: &Chunk) -> (f64, f64) {
        let Some(mcfg) = find_named(root, b"mcfg", 6) else {
            return (0.0, 0.0);
        };
        let data = self.data(mcfg);
        let skip = if self.format.version >= 13 {
            12
        } else if self.format.version >= 9 {
            4
        } else if self.format.version == 6 {
            28
        } else {
            0
        };
        let width = self.format.coord(data, skip).unwrap_or(0.0);
        let height = self
            .format
            .coord(data, skip + self.format.step)
            .unwrap_or(0.0);
        (width.abs(), height.abs())
    }

    /// Page size from the page's own `loda` (argument 19130).
    fn page_loda_size(&self, page: &Chunk) -> Option<(f64, f64)> {
        let loda = find_named(page, b"loda", 4)?;
        let data = self.data(loda);
        let table = loda::table(data, self.format.version)?;
        for arg in table.args {
            if arg.ty == ARG_PAGE_SIZE {
                let width = self.format.coord(data, arg.off)?;
                let height =
                    self.format.coord(data, arg.off + self.format.step)?;
                if width.abs() > 0.0 && height.abs() > 0.0 {
                    return Some((width.abs(), height.abs()));
                }
            }
        }
        None
    }

    fn read_page(&mut self, page: &Chunk) -> Vec<(String, Geometry)> {
        let mut layers: Vec<(String, Geometry)> = Vec::new();
        let mut lookups = Vec::new();
        page.walk(&mut |chunk, _| {
            if chunk.list_type == Some(*b"layr") {
                lookups.push(chunk);
            }
        });
        // Layers are stored front to back.
        for layer in lookups.into_iter().rev() {
            if self.layer_has_no_content(layer) || self.layer_hidden(layer) {
                continue;
            }
            let mut geometry = Geometry::new();
            self.collect_objects(layer, Affine::IDENTITY, &mut geometry);
            if geometry.is_empty() {
                continue;
            }
            let name = self
                .layer_name(layer)
                .unwrap_or_else(|| "Layer".to_string());
            layers.push((name, geometry));
        }
        layers
    }

    fn layer_has_no_content(&self, layer: &Chunk) -> bool {
        let Some(loda) = find_named(layer, b"loda", 4) else {
            return false;
        };
        let data = self.data(loda);
        let Some(table) = loda::table(data, self.format.version) else {
            return false;
        };
        matches!(
            table.kind,
            LAYER_KIND_GRID | LAYER_KIND_GUIDES | LAYER_KIND_DESKTOP
        )
    }

    fn layer_hidden(&self, layer: &Chunk) -> bool {
        let Some(flags) = layer.find(b"flgs") else {
            return false;
        };
        let data = self.data(flags);
        if data.len() < 4 {
            return false;
        }
        if self.format.version >= 16 {
            return data[1] & 0x01 != 0;
        }
        // Legacy: hidden when flagged hidden and neither shown nor printed.
        let value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        value & 0x100 != 0 && value & 0x00ff_0000 == 0
    }

    fn layer_name(&self, layer: &Chunk) -> Option<String> {
        let loda = find_named(layer, b"loda", 4)?;
        let data = self.data(loda);
        let table = loda::table(data, self.format.version)?;
        for arg in table.args {
            if arg.ty == ARG_NAME {
                return decode_name(
                    &data[arg.off..arg.end],
                    self.format.version,
                );
            }
        }
        None
    }

    /// Walks the objects of a container, applying group transforms.
    fn collect_objects(
        &mut self,
        container: &Chunk,
        transform: Affine,
        out: &mut Geometry,
    ) {
        for child in &container.children {
            match child.list_type.as_ref() {
                Some(b"obj ") => self.read_object(child, transform, out),
                Some(b"grp ") | Some(b"lnkg") => {
                    let group = transform.then(self.read_transform(child));
                    self.collect_objects(child, group, out);
                }
                // Object, layer and group internals only.
                Some(b"lgob") | Some(b"trfl") | Some(b"layr") => {}
                Some(_) => self.collect_objects(child, transform, out),
                None => {}
            }
        }
    }

    fn read_object(
        &mut self,
        object: &Chunk,
        transform: Affine,
        out: &mut Geometry,
    ) {
        if self.object_hidden(object) {
            return;
        }
        let Some(loda_chunk) = find_named(object, b"loda", 4) else {
            self.skipped.unreadable += 1;
            return;
        };
        let data = self.data(loda_chunk);
        let Some(table) = loda::table(data, self.format.version) else {
            self.skipped.unreadable += 1;
            return;
        };
        let Some(arg) = table.args.iter().find(|arg| arg.ty == ARG_COORDINATES)
        else {
            self.skipped.kinds.insert(table.kind);
            return;
        };
        let coords = &data[arg.off..arg.end];
        let mut geometry = Geometry::new();
        match table.kind {
            KIND_RECTANGLE => self.read_rectangle(coords, &mut geometry),
            KIND_ELLIPSE => self.read_ellipse(coords, &mut geometry),
            KIND_CURVE | KIND_MESH => self.read_curve(coords, &mut geometry),
            KIND_PATH => self.read_path(coords, &mut geometry),
            KIND_ARTISTIC_TEXT | KIND_PARAGRAPH_TEXT => {
                let Some(glyphs) = self.read_text(object, coords, table.kind)
                else {
                    self.skipped.text += 1;
                    return;
                };
                geometry = glyphs;
            }
            KIND_BITMAP => {
                self.skipped.bitmaps += 1;
                return;
            }
            KIND_POLYGON | KIND_CONNECTOR => {
                self.skipped.kinds.insert(table.kind);
                return;
            }
            other => {
                self.skipped.kinds.insert(other);
                return;
            }
        }
        if geometry.is_empty() {
            return;
        }
        let object_transform = transform.then(self.read_transform(object));
        object_transform.apply(&mut geometry);
        out.extend(&geometry);
    }

    /// Reads the `stlt` style table of the document.
    fn read_styles(&self) -> HashMap<u32, stlt::Style> {
        let mut styles = None;
        self.tree.root.walk(&mut |chunk, _| {
            if styles.is_none() && chunk.list_type == Some(*b"stlt") {
                styles = Some(stlt::read_styles(
                    self.data(chunk),
                    self.format.version,
                ));
            }
        });
        styles.unwrap_or_default()
    }

    /// The font records of the document: id to family name.
    fn read_fonts(&self) -> HashMap<u16, String> {
        let mut fonts = HashMap::new();
        self.tree.root.walk(&mut |chunk, _| {
            if &chunk.id != b"font" {
                return;
            }
            let data = self.data(chunk);
            let Some(id) = self.format.u16(data, 0) else {
                return;
            };
            if let Some(name) = decode_font_name(data, self.format.version) {
                fonts.entry(id).or_insert(name);
            }
        });
        fonts
    }

    /// Converts a text object to glyph outlines in its own space.
    fn read_text(
        &self,
        object: &Chunk,
        coords: &[u8],
        kind: u32,
    ) -> Option<Geometry> {
        let txsm = find_named(object, b"txsm", 4)?;
        let text = text::read_text(self.data(txsm), self.format.version)?;
        let origin = if kind == KIND_ARTISTIC_TEXT {
            let x = self.format.coord(coords, 0)?;
            let y = self.format.coord(coords, self.format.step)?;
            // Version 15 stores a record id instead of a place in some
            // files; a point farther than 10 m is taken for that.
            if x.abs() > 10.0 || y.abs() > 10.0 {
                (0.0, 0.0)
            } else {
                (x, y)
            }
        } else {
            (0.0, 0.0)
        };
        let geometry =
            text::text_geometry(&text, &self.styles, &self.fonts, origin);
        (!geometry.is_empty()).then_some(geometry)
    }

    fn object_hidden(&self, object: &Chunk) -> bool {
        if self.format.version < 16 {
            return false;
        }
        let Some(flags) = object.find(b"flgs") else {
            return false;
        };
        self.data(flags).get(1).is_some_and(|byte| byte & 0x01 != 0)
    }

    /// The transform of an object or group: its `trfd` entries applied in
    /// order.
    fn read_transform(&self, chunk: &Chunk) -> Affine {
        let Some(trfd) = find_named(chunk, b"trfd", 4) else {
            return Affine::IDENTITY;
        };
        let data = self.data(trfd);
        let mut transform = Affine::IDENTITY;
        let Some(num) = self.format.unsigned(data, self.format.step) else {
            return transform;
        };
        let Some(start) = self.format.unsigned(data, 2 * self.format.step)
        else {
            return transform;
        };
        for index in 0..num as usize {
            let at = start as usize + index * self.format.step;
            let Some(offset) = self.format.unsigned(data, at) else {
                break;
            };
            let mut pos = offset as usize;
            if self.format.version >= 13 {
                pos += 8;
            }
            let Some(kind) = self.format.u16(data, pos) else {
                break;
            };
            if kind != 0x08 {
                continue;
            }
            pos += 2;
            if self.format.version >= 6 {
                pos += 6;
            }
            let Some(a) = self.format.double(data, pos) else {
                break;
            };
            let Some(c) = self.format.double(data, pos + 8) else {
                break;
            };
            let Some(tx) = self.format.double_coord(data, pos + 16) else {
                break;
            };
            let Some(b) = self.format.double(data, pos + 24) else {
                break;
            };
            let Some(d) = self.format.double(data, pos + 32) else {
                break;
            };
            let Some(ty) = self.format.double_coord(data, pos + 40) else {
                break;
            };
            transform = transform.then(Affine { a, b, c, d, tx, ty });
        }
        transform
    }

    fn read_rectangle(&mut self, data: &[u8], out: &mut Geometry) {
        if self.format.version >= 15 {
            self.read_rectangle_15(data, out);
        } else {
            self.read_rectangle_old(data, out);
        }
    }

    fn read_rectangle_old(&mut self, data: &[u8], out: &mut Geometry) {
        let Some(width) = self.format.coord(data, 0) else {
            self.skipped.unreadable += 1;
            return;
        };
        let Some(height) = self.format.coord(data, self.format.step) else {
            self.skipped.unreadable += 1;
            return;
        };
        let first =
            self.format.coord(data, 2 * self.format.step).unwrap_or(0.0);
        let mut radii = [first; 4];
        if self.format.version >= 9 {
            for (index, radius) in radii.iter_mut().enumerate() {
                *radius = self
                    .format
                    .coord(data, (2 + index) * self.format.step)
                    .unwrap_or(first);
            }
        }
        rectangle_path(width, height, &radii, out);
    }

    fn read_rectangle_15(&mut self, data: &[u8], out: &mut Geometry) {
        let Some(width) = self.format.double_coord(data, 0) else {
            self.skipped.unreadable += 1;
            return;
        };
        let Some(height) = self.format.double_coord(data, 8) else {
            self.skipped.unreadable += 1;
            return;
        };
        let mut radii = [0.0_f64; 4];
        let mut pos = 8 + 8 + 8 + 8 + 1 + 7;
        for (index, radius) in radii.iter_mut().enumerate() {
            let Some(value) = self.format.double(data, pos) else {
                break;
            };
            *radius = if value > 100.0 {
                value / self.format.unit
            } else {
                value * (width.abs().min(height.abs()) / 2.0)
            };
            pos += 8;
            if index < 3 {
                pos += 16;
            }
        }
        rectangle_path(width, height, &radii, out);
    }

    fn read_ellipse(&mut self, data: &[u8], out: &mut Geometry) {
        let Some(width) = self.format.coord(data, 0) else {
            self.skipped.unreadable += 1;
            return;
        };
        let Some(height) = self.format.coord(data, self.format.step) else {
            self.skipped.unreadable += 1;
            return;
        };
        let start =
            self.format.angle(data, 2 * self.format.step).unwrap_or(0.0);
        let end = self
            .format
            .angle(data, 3 * self.format.step)
            .unwrap_or(start);
        let pie = self
            .format
            .u32(data, 4 * self.format.step)
            .is_some_and(|value| value != 0);
        ellipse_path(width, height, start, end, pie, out);
    }

    fn read_curve(&mut self, data: &[u8], out: &mut Geometry) {
        let Some(count) = self.format.u32(data, 0).map(|value| value as usize)
        else {
            self.skipped.unreadable += 1;
            return;
        };
        if !self.push_point_list(data, 4, count, out) {
            self.skipped.unreadable += 1;
        }
    }

    fn read_path(&mut self, data: &[u8], out: &mut Geometry) {
        let Some(first) = self.format.u16(data, 4) else {
            self.skipped.unreadable += 1;
            return;
        };
        let Some(second) = self.format.u16(data, 6) else {
            self.skipped.unreadable += 1;
            return;
        };
        let count = usize::from(first) + usize::from(second);
        if !self.push_point_list(data, 24, count, out) {
            self.skipped.unreadable += 1;
        }
    }

    /// Reads `count` points and their type bytes at `at` and appends the
    /// resulting path. Returns false when the data is too short.
    fn push_point_list(
        &mut self,
        data: &[u8],
        at: usize,
        count: usize,
        out: &mut Geometry,
    ) -> bool {
        let step = self.format.step;
        let types_at = at + count * 2 * step;
        let Some(types) = data.get(types_at..types_at + count) else {
            return false;
        };
        let mut points = Vec::with_capacity(count);
        for index in 0..count {
            let offset = at + index * 2 * step;
            let Some(x) = self.format.coord(data, offset) else {
                return false;
            };
            let Some(y) = self.format.coord(data, offset + step) else {
                return false;
            };
            points.push((x, y));
        }
        append_path(&points, types, out);
        true
    }
}

/// Builds a path from points and their type bytes: bits 7..6 select move
/// (0), line (1), cubic end (2) or control point (3); bit 3 closes the
/// subpath.
fn append_path(points: &[(f64, f64)], types: &[u8], out: &mut Geometry) {
    let mut controls: Vec<(f64, f64)> = Vec::with_capacity(2);
    let mut start = (0.0, 0.0);
    let mut open = false;
    for (point, kind) in points.iter().zip(types) {
        let close = kind & 0x08 != 0;
        let (x, y) = *point;
        match kind >> 6 {
            0 => {
                out.move_to(x, y, 0.0);
                start = (x, y);
                open = true;
            }
            1 => {
                if !open {
                    out.move_to(x, y, 0.0);
                    start = (x, y);
                    open = true;
                } else {
                    out.line_to(x, y, 0.0);
                }
            }
            2 => {
                if !open {
                    out.move_to(x, y, 0.0);
                    start = (x, y);
                    open = true;
                } else if controls.len() >= 2 {
                    out.bezier_to(
                        Point3D::new(controls[0].0, controls[0].1, 0.0),
                        Point3D::new(controls[1].0, controls[1].1, 0.0),
                        Point3D::new(x, y, 0.0),
                    );
                } else {
                    out.line_to(x, y, 0.0);
                }
                controls.clear();
            }
            _ => {
                controls.push((x, y));
                if controls.len() > 2 {
                    controls.remove(0);
                }
                continue;
            }
        }
        if close && open {
            let dx = (x - start.0).abs();
            let dy = (y - start.1).abs();
            if dx > 1e-12 || dy > 1e-12 {
                out.close_path();
            }
            controls.clear();
        }
    }
}

/// A rectangle from 0..width and 0..height, its corners rounded by the
/// four radii in counter-clockwise order from (width, 0).
fn rectangle_path(
    width: f64,
    height: f64,
    radii: &[f64; 4],
    out: &mut Geometry,
) {
    let flip_x = width < 0.0;
    let flip_y = height < 0.0;
    let w = width.abs();
    let h = height.abs();
    let mut r = [0.0_f64; 4];
    for (index, value) in radii.iter().enumerate() {
        r[index] = value.abs().min(w).min(h);
    }
    let shrink = |a: f64, b: f64, side: f64| -> (f64, f64) {
        let sum = a + b;
        if sum > side && sum > 0.0 {
            (a * side / sum, b * side / sum)
        } else {
            (a, b)
        }
    };
    let mut r0 = r[0];
    let mut r1 = r[1];
    let mut r2 = r[2];
    let mut r3 = r[3];
    (r0, r1) = shrink(r0, r1, h);
    (r1, r2) = shrink(r1, r2, w);
    (r2, r3) = shrink(r2, r3, h);
    (r3, r0) = shrink(r3, r0, w);
    // The radius of the corner at (right, top) in the positive space,
    // taking the mirroring into account.
    let radius = |right: bool, top: bool| -> f64 {
        let original_right = right != flip_x;
        let original_top = top != flip_y;
        match (original_right, original_top) {
            (true, false) => r0,
            (true, true) => r1,
            (false, true) => r2,
            (false, false) => r3,
        }
    };
    let (ra, rb) = (radius(false, false), radius(true, false));
    let (rc, rd) = (radius(true, true), radius(false, true));
    let mut path = Geometry::new();
    path.move_to(ra, 0.0, 0.0);
    path.line_to(w - rb, 0.0, 0.0);
    quarter_arc(&mut path, (w - rb, 0.0), (w, rb), w - rb, rb);
    path.line_to(w, h - rc, 0.0);
    quarter_arc(&mut path, (w, h - rc), (w - rc, h), w - rc, h - rc);
    path.line_to(rd, h, 0.0);
    quarter_arc(&mut path, (rd, h), (0.0, h - rd), rd, h - rd);
    path.line_to(0.0, ra, 0.0);
    quarter_arc(&mut path, (0.0, ra), (ra, 0.0), ra, ra);
    path.close_path();
    if flip_x || flip_y {
        let sx = if flip_x { -1.0 } else { 1.0 };
        let sy = if flip_y { -1.0 } else { 1.0 };
        path.transform_2d(sx, 0.0, 0.0, sy, 0.0, 0.0);
    }
    out.extend(&path);
}

/// Appends a quarter arc from `from` to `to` around (`cx`, `cy`) as a
/// cubic bezier, turning counter-clockwise.
fn quarter_arc(
    out: &mut Geometry,
    from: (f64, f64),
    to: (f64, f64),
    cx: f64,
    cy: f64,
) {
    let (x0, y0) = from;
    let (x1, y1) = to;
    if (x1 - x0).abs() < 1e-12 && (y1 - y0).abs() < 1e-12 {
        return;
    }
    let c1 = (x0 + KAPPA * (cy - y0), y0 - KAPPA * (cx - x0));
    let c2 = (x1 - KAPPA * (cy - y1), y1 + KAPPA * (cx - x1));
    out.bezier_to(
        Point3D::new(c1.0, c1.1, 0.0),
        Point3D::new(c2.0, c2.1, 0.0),
        Point3D::new(x1, y1, 0.0),
    );
}

/// Appends an ellipse or ellipse arc spanning 0..width and 0..height.
fn ellipse_path(
    width: f64,
    height: f64,
    start: f64,
    end: f64,
    pie: bool,
    out: &mut Geometry,
) {
    let cx = width / 2.0;
    let cy = height / 2.0;
    let rx = cx.abs();
    let ry = cy.abs();
    let point = |a: f64| (cx + rx * a.cos(), cy - ry * a.sin());
    let derivative = |a: f64| (-rx * a.sin(), -ry * a.cos());
    let same = (start - end).abs() < 1e-9;
    if same {
        let quarter = std::f64::consts::FRAC_PI_2;
        let (x0, y0) = point(start);
        out.move_to(x0, y0, 0.0);
        for index in 1..=4 {
            let a0 = start + quarter * (index - 1) as f64;
            let a1 = start + quarter * index as f64;
            let (px, py) = point(a0);
            let (x, y) = point(a1);
            let (dx0, dy0) = derivative(a0);
            let (dx1, dy1) = derivative(a1);
            out.bezier_to(
                Point3D::new(px + KAPPA * dx0, py + KAPPA * dy0, 0.0),
                Point3D::new(x - KAPPA * dx1, y - KAPPA * dy1, 0.0),
                Point3D::new(x, y, 0.0),
            );
        }
        out.close_path();
        return;
    }
    let wrap = |a: f64| a.rem_euclid(std::f64::consts::TAU);
    let a0 = wrap(start);
    let mut a1 = wrap(end);
    if a1 < a0 {
        a1 += std::f64::consts::TAU;
    }
    let (x0, y0) = point(a0);
    out.move_to(x0, y0, 0.0);
    if pie {
        out.line_to(cx, cy, 0.0);
    }
    let span = a1 - a0;
    let steps = ((span.abs() / (std::f64::consts::PI / 16.0)).ceil() as usize)
        .clamp(4, 128);
    for index in 1..=steps {
        let a = a0 + span * index as f64 / steps as f64;
        let (x, y) = point(a);
        out.line_to(x, y, 0.0);
    }
    if pie {
        out.close_path();
    }
}

/// The first descendant (or self) named `name`, depth-limited.
fn find_named<'a>(
    chunk: &'a Chunk,
    name: &[u8; 4],
    depth: usize,
) -> Option<&'a Chunk> {
    if &chunk.id == name {
        return Some(chunk);
    }
    if depth == 0 {
        return None;
    }
    chunk
        .children
        .iter()
        .find_map(|child| find_named(child, name, depth - 1))
}

/// The first name of a font record: id u16, encoding u16, style flags
/// u32, 10 unknown bytes, then the names.
fn decode_font_name(data: &[u8], version: u16) -> Option<String> {
    let name = data.get(18..)?;
    let text = if version >= 12 {
        let (pairs, _) = name.as_chunks::<2>();
        let units: Vec<u16> =
            pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
        String::from_utf16_lossy(&units)
    } else {
        name.iter().map(|byte| char::from(*byte)).collect()
    };
    let first = text.split('\u{0}').next().unwrap_or("").trim();
    (!first.is_empty()).then(|| first.to_string())
}

/// Decodes a layer or page name: single-byte text before version 12,
/// UTF-16LE from then on.
fn decode_name(data: &[u8], version: u16) -> Option<String> {
    if data.is_empty() {
        return None;
    }
    let text = if version >= 12 {
        let (pairs, _) = data.as_chunks::<2>();
        let units: Vec<u16> = pairs
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .take_while(|unit| *unit != 0)
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        data.iter()
            .take_while(|byte| **byte != 0)
            .map(|byte| char::from(*byte))
            .collect()
    };
    let trimmed = text.trim_end_matches('\0').trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// The bounds (width, height, min x, min y) of all layers' geometry.
fn content_bounds(
    layers: &[(String, Geometry)],
) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for (_, geometry) in layers {
        if geometry.is_empty() {
            continue;
        }
        let rect = geometry.rect();
        let (min_x, min_y) = (rect.min.x, rect.min.y);
        let (max_x, max_y) = (rect.max.x, rect.max.y);
        bounds = Some(match bounds {
            Some((w, h, x, y)) => (
                w.max(max_x - min_x),
                h.max(max_y - min_y),
                x.min(min_x),
                y.min(min_y),
            ),
            None => (max_x - min_x, max_y - min_y, min_x, min_y),
        });
    }
    bounds
}
