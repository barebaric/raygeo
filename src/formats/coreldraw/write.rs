// Adapted from the TraceDraw project, crates/cdr/src/write.rs (the
// records of versions 7 to 13), MIT OR Apache-2.0, Copyright (c) 2026
// TraceDraw members and contributors. Modified for raygeo: one drawing
// page of version 12, geometry written as curves, no text, bitmaps,
// fills or previews, tests removed (tests live under tests/).

//! Writing CorelDRAW (`.cdr`) files.
//!
//! Emits the version 12 record format in a plain RIFF file: the form
//! `CDRC`, the document tables, a master page and one drawing page with
//! its layers, and the two compressed chunk lists RIFF files keep their
//! contents in. Every shape is written as a curve, which is what the
//! reader turns back into geometry, so a file round-trips through
//! [`write`] and [`parse`](super::parse).
//!
//! Coordinates are millimetres with Y pointing up; the page is sized to
//! the geometry's bounding box and the drawing is moved to its
//! bottom-left corner. Objects carry a black hairline outline and no
//! fill, which is what a laser cutter follows.
//!
//! The record layouts are the ones documented in TraceDraw's clean-room
//! `docs/cdr-format.md` ("Writing").

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::error::RaygeoError;
use crate::geo::geometry::Geometry;
use crate::geo::types::{Command, Point3D};

/// Coordinate units per millimetre (1/254000 inch).
const UNITS_PER_MM: f64 = 254_000.0 / 25.4;
/// Halftone screen of fills and outlines: 45 degrees, 60 lines per inch.
const SCREEN: [u8; 8] = [0x40, 0xa5, 0xae, 0x02, 0x3c, 0x00, 0x00, 0x00];
/// The version written.
const VERSION: u16 = 12;
/// The smallest page edge written, in millimetres.
const MIN_PAGE_MM: f64 = 1.0;
/// The font the document's style table names.
const FONT: &str = "Arial";
/// The layer the geometry is written to.
const LAYER: &str = "Layer 1";

// Argument types of an object record.
const ARG_COORDS: u32 = 30;
const ARG_FILL: u32 = 20;
const ARG_OUTLINE: u32 = 10;
const ARG_STYLE: u32 = 200;
const ARG_ID: u32 = 100;
const ARG_NAME: u32 = 1000;
const ARG_LAYER_COLOR: u32 = 2000;
const ARG_PROPS: u32 = 12000;
const ARG_ROTATION: u32 = 12030;
const ARG_DRAW_FLAGS: u32 = 19170;

/// Object type of a line or curve.
const OBJ_CURVE: u32 = 0x03;
/// Layer types.
const LAYER_PLAIN: u32 = 0;
const LAYER_GRID: u32 = 0x0b;
const LAYER_GUIDES: u32 = 0x0c;
const LAYER_DESKTOP: u32 = 0x11;
/// The flags of a drawing's own layer.
const PLAIN_LAYER_FLAGS: [u8; 4] = [0, 0, 0, 0x98];
/// The style id graphics refer to: after the four text styles.
const GRAPHIC_STYLE_ID: u32 = 5;

/// Writes `geometry` as a CorelDRAW version 12 file.
///
/// The page is the geometry's bounding box in millimetres (at least
/// [`MIN_PAGE_MM`] on each edge), and the drawing is moved so that its
/// bottom-left corner sits on the page origin. The geometry goes into one
/// object on one layer, named [`LAYER`], drawn with a black hairline
/// outline and no fill.
///
/// # Errors
///
/// [`RaygeoError::EmptyGeometry`] when `geometry` holds no command.
pub fn write(geometry: &Geometry) -> Result<Vec<u8>, RaygeoError> {
    if geometry.is_empty() {
        return Err(RaygeoError::EmptyGeometry);
    }
    let rect = geometry.rect();
    let offset = (rect.min.x, rect.min.y);
    let width = (rect.max.x - rect.min.x).max(MIN_PAGE_MM);
    let height = (rect.max.y - rect.min.y).max(MIN_PAGE_MM);
    let nodes = curve_nodes(geometry.data(), offset);
    let coords = curve_coords(&nodes);
    let bounds = (0.0, 0.0, width, height);
    let document = document(&coords, bounds, width, height);
    Ok(riff_file(*b"CDRC", &document))
}

// ---- geometry ----------------------------------------------------------

/// A curve node in page millimetres: the point and the node byte (bits
/// 7..6 the kind, bit 3 closes the subpath).
#[derive(Debug, Clone, Copy)]
struct CurveNode {
    x: f64,
    y: f64,
    kind: u8,
}

const NODE_MOVE: u8 = 0x00;
const NODE_LINE: u8 = 0x40;
const NODE_CUBIC: u8 = 0x80;
const NODE_CONTROL: u8 = 0xc0;
const NODE_CLOSE: u8 = 0x08;

/// Turns the commands into curve nodes, moving them by `offset` (the
/// geometry's bottom-left corner) onto the page.
fn curve_nodes(commands: &[Command], offset: (f64, f64)) -> Vec<CurveNode> {
    let mut nodes = Vec::with_capacity(commands.len());
    let mut last = (0.0, 0.0);
    for command in commands {
        match command {
            Command::Move { end } => {
                let point = shift(end, offset);
                nodes.push(CurveNode {
                    x: point.0,
                    y: point.1,
                    kind: NODE_MOVE,
                });
                last = point;
            }
            Command::Line { end } => {
                let point = shift(end, offset);
                nodes.push(CurveNode {
                    x: point.0,
                    y: point.1,
                    kind: NODE_LINE,
                });
                last = point;
            }
            Command::Bezier {
                end,
                control1,
                control2,
            } => {
                push_cubic(&mut nodes, control1, control2, end, offset);
                last = shift(end, offset);
            }
            Command::Arc {
                end,
                center_offset,
                normal,
            } => {
                let segments = arc_cubics(last, *end, *center_offset, *normal);
                if segments.is_empty() {
                    let point = shift(end, offset);
                    nodes.push(CurveNode {
                        x: point.0,
                        y: point.1,
                        kind: NODE_LINE,
                    });
                }
                for (control1, control2, point) in segments {
                    push_cubic(
                        &mut nodes, &control1, &control2, &point, offset,
                    );
                }
                last = shift(end, offset);
            }
        }
    }
    close_subpaths(&mut nodes);
    nodes
}

/// Adds one cubic segment: both control points and the end point.
fn push_cubic(
    nodes: &mut Vec<CurveNode>,
    control1: &Point3D,
    control2: &Point3D,
    end: &Point3D,
    offset: (f64, f64),
) {
    for (point, kind) in [
        (control1, NODE_CONTROL),
        (control2, NODE_CONTROL),
        (end, NODE_CUBIC),
    ] {
        let point = shift(point, offset);
        nodes.push(CurveNode {
            x: point.0,
            y: point.1,
            kind,
        });
    }
}

/// The control points and end points of the cubic segments that draw the
/// arc from `start` to `end`, at most a quarter circle each.
fn arc_cubics(
    start: (f64, f64),
    end: Point3D,
    center_offset: Point3D,
    normal: Point3D,
) -> Vec<(Point3D, Point3D, Point3D)> {
    let radius = (center_offset.x * center_offset.x
        + center_offset.y * center_offset.y)
        .sqrt();
    if !radius.is_finite() || radius <= 0.0 {
        return Vec::new();
    }
    let centre = (start.0 + center_offset.x, start.1 + center_offset.y);
    let a0 = (start.1 - centre.1).atan2(start.0 - centre.0);
    let mut a1 = (end.y - centre.1).atan2(end.x - centre.0);
    let counter_clockwise = normal.z >= 0.0;
    let same = (a1 - a0).abs() < 1e-12;
    if same {
        a1 = a0 + if counter_clockwise { TAU } else { -TAU };
    } else if counter_clockwise && a1 < a0 {
        a1 += TAU;
    } else if !counter_clockwise && a1 > a0 {
        a1 -= TAU;
    }
    let span = a1 - a0;
    let steps = ((span.abs() / FRAC_PI_2).ceil() as usize).clamp(1, 4);
    let point = |angle: f64| {
        (
            centre.0 + radius * angle.cos(),
            centre.1 + radius * angle.sin(),
        )
    };
    let mut segments = Vec::with_capacity(steps);
    for index in 0..steps {
        let from = a0 + span * index as f64 / steps as f64;
        let to = a0 + span * (index + 1) as f64 / steps as f64;
        let sweep = to - from;
        let k = 4.0 / 3.0 * (sweep / 4.0).tan() * radius;
        let (x0, y0) = point(from);
        let (x1, y1) = point(to);
        let control1 =
            Point3D::new(x0 - k * from.sin(), y0 + k * from.cos(), 0.0);
        let control2 = Point3D::new(x1 + k * to.sin(), y1 - k * to.cos(), 0.0);
        segments.push((control1, control2, Point3D::new(x1, y1, 0.0)));
    }
    segments
}

/// Marks subpaths that end where they started as closed and drops the
/// duplicate closing segment: CorelDRAW closes a subpath with the bit on
/// its last node, not with a zero-length line.
fn close_subpaths(nodes: &mut Vec<CurveNode>) {
    let mut kept: Vec<CurveNode> = Vec::with_capacity(nodes.len());
    let mut start = 0;
    for node in nodes.iter().copied() {
        if node.kind >> 6 == 0 {
            close_subpath(&mut kept, start);
            start = kept.len();
        }
        kept.push(node);
    }
    close_subpath(&mut kept, start);
    *nodes = kept;
}

/// Closes one subpath of `nodes`, from its move node at `start` to the
/// end of the list.
fn close_subpath(nodes: &mut Vec<CurveNode>, start: usize) {
    if nodes.len() <= start + 1 {
        return;
    }
    let last = *nodes.last().unwrap_or(&nodes[start]);
    if last.kind & 0xc0 == NODE_CONTROL || !same_point(nodes[start], last) {
        return;
    }
    if last.kind & 0xc0 == NODE_LINE {
        // A subpath that is only a zero-length line keeps a line of one
        // unit instead, so the object does not vanish.
        if nodes.len() == start + 2 {
            let node = &mut nodes[start + 1];
            node.x = (f64::from(units(node.x)) + 1.0) / UNITS_PER_MM;
            return;
        }
        nodes.pop();
    }
    if let Some(node) = nodes.last_mut() {
        node.kind |= NODE_CLOSE;
    }
}

fn same_point(a: CurveNode, b: CurveNode) -> bool {
    units(a.x) == units(b.x) && units(a.y) == units(b.y)
}

fn shift(point: &Point3D, offset: (f64, f64)) -> (f64, f64) {
    (point.x - offset.0, point.y - offset.1)
}

/// The point list of the curve: a count, the points and their node bytes.
fn curve_coords(nodes: &[CurveNode]) -> Vec<u8> {
    let mut points = Vec::with_capacity(nodes.len() * 8);
    points.extend_from_slice(&(nodes.len() as u32).to_le_bytes());
    for node in nodes {
        points.extend_from_slice(&units(node.x).to_le_bytes());
        points.extend_from_slice(&units(node.y).to_le_bytes());
    }
    for node in nodes {
        points.push(node.kind);
    }
    points
}

// ---- bytes -------------------------------------------------------------

#[derive(Default)]
struct Buf(Vec<u8>);

impl Buf {
    fn u16(&mut self, x: u16) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }

    fn u32(&mut self, x: u32) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }

    fn i32(&mut self, x: i32) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }

    fn f64(&mut self, x: f64) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }

    fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.0.extend_from_slice(b);
        self
    }

    fn zeros(&mut self, n: usize) -> &mut Self {
        self.0.resize(self.0.len() + n, 0);
        self
    }

    fn pad_to(&mut self, len: usize) -> &mut Self {
        if self.0.len() < len {
            self.0.resize(len, 0);
        }
        self
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn done(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

/// Millimetres to coordinate units.
fn units(mm: f64) -> i32 {
    if !mm.is_finite() {
        return 0;
    }
    (mm * UNITS_PER_MM)
        .round()
        .clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

/// A layer or font name: UTF-16 with a terminating NUL.
fn name_bytes(name: &str) -> Vec<u8> {
    let mut out: Vec<u8> =
        name.encode_utf16().flat_map(u16::to_le_bytes).collect();
    out.extend_from_slice(&[0, 0]);
    out
}

// ---- chunk tree --------------------------------------------------------

#[derive(Debug, Clone)]
enum Node {
    Chunk([u8; 4], Vec<u8>),
    List([u8; 4], Vec<Node>),
    /// A list whose body after its type is a record, not chunks (the
    /// style table before version 16).
    RawList([u8; 4], Vec<u8>),
}

fn chunk(id: &[u8; 4], data: Vec<u8>) -> Node {
    Node::Chunk(*id, data)
}

fn list(kind: &[u8; 4], children: Vec<Node>) -> Node {
    Node::List(*kind, children)
}

fn put_chunk(out: &mut Vec<u8>, id: &[u8], payload: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
}

fn serialize(nodes: &[Node], out: &mut Vec<u8>) {
    for node in nodes {
        match node {
            Node::Chunk(id, data) => put_chunk(out, id, data),
            Node::List(kind, children) => {
                let mut body = kind.to_vec();
                serialize(children, &mut body);
                put_chunk(out, b"LIST", &body);
            }
            Node::RawList(kind, data) => {
                let mut body = kind.to_vec();
                body.extend_from_slice(data);
                put_chunk(out, b"LIST", &body);
            }
        }
    }
}

/// `nodes` as the inflated stream of a `cmpr` list: each chunk's size
/// field holds the index of its size in a pool (every chunk and list in
/// order, lists before their children), padding as in the plain stream.
fn serialize_indexed(nodes: &[Node], out: &mut Vec<u8>, pool: &mut Vec<u32>) {
    for node in nodes {
        let index = pool.len();
        pool.push(0);
        let (id, start): (&[u8], usize) = match node {
            Node::Chunk(id, _) => (id, out.len() + 8),
            Node::List(..) | Node::RawList(..) => (b"LIST", out.len() + 8),
        };
        out.extend_from_slice(id);
        out.extend_from_slice(&(index as u32).to_le_bytes());
        match node {
            Node::Chunk(_, data) => out.extend_from_slice(data),
            Node::List(kind, children) => {
                out.extend_from_slice(kind);
                serialize_indexed(children, out, pool);
            }
            Node::RawList(kind, data) => {
                out.extend_from_slice(kind);
                out.extend_from_slice(data);
            }
        }
        let size = out.len() - start;
        pool[index] = size as u32;
        if size % 2 == 1 {
            out.push(0);
        }
    }
}

/// A `CPng` block: the tag, `01 00 04 00` and the zlib stream of `data`.
fn cpng(data: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut block = b"CPng\x01\x00\x04\x00".to_vec();
    let mut encoder = flate2::write::ZlibEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    );
    let _ = encoder.write_all(data);
    block.extend_from_slice(&encoder.finish().unwrap_or_default());
    block
}

/// A `LIST cmpr` holding `nodes` compressed: the two (compressed,
/// inflated) size pairs, then the chunk stream's `CPng` block and the
/// size pool's.
fn cmpr(nodes: &[Node]) -> Node {
    let mut stream = Vec::new();
    let mut pool = Vec::new();
    serialize_indexed(nodes, &mut stream, &mut pool);
    let pool: Vec<u8> =
        pool.iter().flat_map(|size| size.to_le_bytes()).collect();
    let (chunks, sizes) = (cpng(&stream), cpng(&pool));
    let mut body = Vec::with_capacity(16 + chunks.len() + sizes.len());
    for (compressed, inflated) in
        [(chunks.len(), stream.len()), (sizes.len(), pool.len())]
    {
        body.extend_from_slice(&(compressed as u32).to_le_bytes());
        body.extend_from_slice(&(inflated as u32).to_le_bytes());
    }
    body.extend_from_slice(&chunks);
    body.extend_from_slice(&sizes);
    Node::RawList(*b"cmpr", body)
}

/// A RIFF file of the form type and chunks.
fn riff_file(form: [u8; 4], nodes: &[Node]) -> Vec<u8> {
    let mut body = form.to_vec();
    serialize(nodes, &mut body);
    let mut out = Vec::with_capacity(body.len() + 8);
    put_chunk(&mut out, b"RIFF", &body);
    out
}

/// RIFF streams before version 16 keep their contents in two compressed
/// lists after the picture and info: the first with the bitmaps (or
/// nothing), the second with the document tables and the pages.
fn compress_riff(tree: Vec<Node>) -> Vec<Node> {
    const INFO: [u8; 4] = *b"INFO";
    const BMPT: [u8; 4] = *b"bmpt";
    let mut head = Vec::new();
    let mut bitmaps = Vec::new();
    let mut body = Vec::new();
    for node in tree {
        match &node {
            Node::Chunk(id, _) if [b"vrsn", b"DISP"].contains(&id) => {
                head.push(node);
            }
            Node::List(INFO, _) => head.push(node),
            Node::List(BMPT, _) => bitmaps.push(node),
            _ => body.push(node),
        }
    }
    head.push(cmpr(&bitmaps));
    head.push(cmpr(&body));
    head
}

// ---- document records --------------------------------------------------

/// The whole RIFF chunk list: the version and info chunks, the compressed
/// document tables, the master page, the drawing page and `sumi`.
fn document(
    coords: &[u8],
    bounds: (f64, f64, f64, f64),
    width: f64,
    height: f64,
) -> Vec<Node> {
    let (left, bottom, right, top) = bounds;
    let content = (left, bottom, right, top);
    let centre = (width / 2.0, height / 2.0);
    let page_box = box_bytes(relative(&content, centre));
    let objects = object(coords, &page_box, centre);
    let layer = layer(LAYER, objects);
    let doc = vec![
        chunk(b"pfrd", vec![0]),
        chunk(b"mcfg", mcfg_12(width, height)),
        chunk(b"ptrt", page_box.clone()),
        list(b"fntt", vec![chunk(b"font", font_record())]),
        list(
            b"filt",
            vec![
                list(b"filc", vec![chunk(b"fild", fild(1, true))]),
                list(b"filc", vec![chunk(b"fild", fild(2, false))]),
            ],
        ),
        list(b"otlt", vec![chunk(b"outl", outline(1))]),
        Node::RawList(*b"stlt", stlt()),
        list(b"uil ", Vec::new()),
    ];
    let mut tree = Vec::new();
    tree.push(chunk(b"vrsn", (VERSION * 100).to_le_bytes().to_vec()));
    let empty = vec![0u8; 2];
    tree.push(list(
        b"INFO",
        vec![chunk(b"IKEY", empty.clone()), chunk(b"ICMT", empty)],
    ));
    tree.push(list(b"doc ", doc));
    tree.push(master_page(LAYER));
    tree.push(page(&page_box, layer));
    let head = compress_riff(tree);
    let mut out = head;
    out.push(chunk(b"sumi", sumi(content, centre)));
    out
}

/// The box of `content` relative to the page centre, as the left, top,
/// right and bottom coordinates files store.
fn relative(
    content: &(f64, f64, f64, f64),
    centre: (f64, f64),
) -> (f64, f64, f64, f64) {
    (
        content.0 - centre.0,
        content.3 - centre.1,
        content.2 - centre.0,
        content.1 - centre.1,
    )
}

/// The 16-byte box record: left, top, right, bottom.
fn box_bytes(bounds: (f64, f64, f64, f64)) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    for value in [bounds.0, bounds.1, bounds.2, bounds.3] {
        out.extend_from_slice(&units(value).to_le_bytes());
    }
    out
}

/// The master page: its guides, desktop and the drawing's own layers, and
/// the grid layer, all empty.
fn master_page(layer_name: &str) -> Node {
    let layers = vec![
        master_layer(LAYER_GUIDES, [0x0a, 0, 1, 0x98], "Guides"),
        master_layer(LAYER_DESKTOP, [0x08, 0, 1, 0x98], "Desktop"),
        layer(layer_name, Vec::new()),
        master_layer(LAYER_GRID, [0x1a, 1, 1, 0x98], "Document Grid"),
    ];
    list(
        b"page",
        vec![
            chunk(b"flgs", vec![0, 0, 1, 0x90]),
            chunk(b"bbox", vec![0; 16]),
            list(b"gobj", layers),
        ],
    )
}

/// A drawing page listing the master layers and the drawing's layer.
fn page(page_box: &[u8], layer: Node) -> Node {
    let layers = vec![
        master_layer(LAYER_GUIDES, [0x0a, 0, 1, 0x98], "Guides"),
        master_layer(LAYER_DESKTOP, [0x08, 0, 1, 0x98], "Desktop"),
        layer,
        master_layer(LAYER_GRID, [0x1a, 1, 1, 0x98], "Document Grid"),
    ];
    list(
        b"page",
        vec![
            chunk(b"flgs", vec![0, 0, 0, 0x90]),
            chunk(b"bbox", page_box.to_vec()),
            chunk(b"obbx", [page_box, page_box].concat()),
            list(b"gobj", layers),
        ],
    )
}

/// A layer of the master page: its flags and its record.
fn master_layer(kind: u32, flags: [u8; 4], name: &str) -> Node {
    layer_with(flags, kind, Some(name), Vec::new())
}

/// A drawing layer: its flags, its name and its objects.
fn layer(name: &str, objects: Vec<Node>) -> Node {
    layer_with(PLAIN_LAYER_FLAGS, LAYER_PLAIN, Some(name), objects)
}

fn layer_with(
    flags: [u8; 4],
    kind: u32,
    name: Option<&str>,
    objects: Vec<Node>,
) -> Node {
    let mut args = Vec::new();
    if let Some(name) = name {
        args.push((ARG_NAME, name_bytes(name)));
    }
    args.push((ARG_LAYER_COLOR, cmyk_black().to_vec()));
    let mut children = vec![
        chunk(b"flgs", flags.to_vec()),
        list(b"lgob", vec![chunk(b"loda", loda(kind, args))]),
    ];
    children.extend(objects);
    list(b"layr", children)
}

/// A curve object holding the geometry.
fn object(coords: &[u8], page_box: &[u8], centre: (f64, f64)) -> Vec<Node> {
    let args = vec![
        (ARG_COORDS, coords.to_vec()),
        (ARG_FILL, 2u32.to_le_bytes().to_vec()),
        (ARG_OUTLINE, 1u32.to_le_bytes().to_vec()),
        (ARG_ID, 1u32.to_le_bytes().to_vec()),
        (ARG_STYLE, GRAPHIC_STYLE_ID.to_le_bytes().to_vec()),
        (ARG_PROPS, vec![0; 4]),
        (ARG_ROTATION, vec![0; 4]),
        (
            ARG_DRAW_FLAGS,
            [1u32, 1, 0]
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect(),
        ),
    ];
    let body = vec![
        chunk(b"flgs", vec![0, 2, 0, 8]),
        chunk(b"bbox", page_box.to_vec()),
        chunk(b"obbx", [page_box, page_box].concat()),
        chunk(b"usdn", 0u32.to_le_bytes().to_vec()),
        list(
            b"lgob",
            vec![
                chunk(b"loda", loda(OBJ_CURVE, args)),
                chunk(b"ftil", identity()),
                list(b"trfl", vec![chunk(b"trfd", trfd(centre))]),
            ],
        ),
    ];
    vec![list(b"obj ", body)]
}

// ---- record bodies -----------------------------------------------------

/// A `loda` argument table: the header, the offsets, the argument types in
/// reverse order and the bodies.
fn loda(kind: u32, mut args: Vec<(u32, Vec<u8>)>) -> Vec<u8> {
    args.sort_by_key(|(kind, _)| std::cmp::Reverse(*kind));
    let count = args.len();
    let start_args = 20usize;
    let start_types = start_args + (count + 1) * 4;
    let mut position = start_types + count * 4;
    let mut offsets = Vec::with_capacity(count + 1);
    for (_, body) in &args {
        offsets.push(position as u32);
        position += body.len();
    }
    offsets.push(position as u32);
    let mut out = Buf::default();
    out.u32(position as u32)
        .u32(count as u32)
        .u32(start_args as u32)
        .u32(start_types as u32)
        .u32(kind);
    for offset in &offsets {
        out.u32(*offset);
    }
    for (kind, _) in args.iter().rev() {
        out.u32(*kind);
    }
    for (_, body) in &args {
        out.bytes(body);
    }
    out.done()
}

/// `trfd`: the identity matrix that moves the page coordinates to the
/// page centre.
fn trfd(centre: (f64, f64)) -> Vec<u8> {
    let mut matrix = Buf::default();
    matrix.u16(8).zeros(6);
    matrix.f64(1.0).f64(0.0).f64(-f64::from(units(centre.0)));
    matrix.f64(0.0).f64(1.0).f64(-f64::from(units(centre.1)));
    matrix.zeros(104);
    let entry = matrix.done();
    let mut out = Buf::default();
    out.u32(24).u32(1).u32(16).u32(0xffff);
    out.u32(24).u32(24);
    out.bytes(&entry);
    out.done()
}

/// The identity matrix as six doubles.
fn identity() -> Vec<u8> {
    let mut out = Buf::default();
    for value in [1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        out.f64(value);
    }
    out.done()
}

/// `fild`: a fill record, solid black or none.
fn fild(id: u32, solid: bool) -> Vec<u8> {
    let mut out = Buf::default();
    out.u32(id);
    if solid {
        out.u16(1).zeros(2).bytes(&rgb_black());
        out.zeros(8).bytes(&SCREEN);
    } else {
        out.u16(0).zeros(2);
    }
    out.done()
}

/// `outl`: a black hairline outline.
fn outline(id: u32) -> Vec<u8> {
    let mut out = Buf::default();
    out.u32(id);
    out.u16(2).u16(0).u16(0).zeros(2);
    out.i32(0).u16(100).zeros(2);
    out.i32(0).zeros(4);
    out.bytes(&identity())
        .bytes(&rgb_black())
        .zeros(8)
        .bytes(&SCREEN)
        .u16(0);
    let dashes_at = out.len();
    out.pad_to(dashes_at + 20);
    out.u32(0).u32(0).zeros(2);
    out.zeros(4);
    out.done()
}

/// A 12-byte colour record: sRGB black.
fn rgb_black() -> [u8; 12] {
    let mut out = [0u8; 12];
    out[0..2].copy_from_slice(&5u16.to_le_bytes());
    out[2..4].copy_from_slice(&5u16.to_le_bytes());
    out
}

/// A 12-byte colour record: 100 % K.
fn cmyk_black() -> [u8; 12] {
    let mut out = [0u8; 12];
    out[0..2].copy_from_slice(&2u16.to_le_bytes());
    out[2..4].copy_from_slice(&5u16.to_le_bytes());
    out[11] = 100;
    out
}

/// `font`: id 1, character set 0, style flags 0x40, 10 classification
/// bytes and the family name in a 296-byte record.
fn font_record() -> Vec<u8> {
    let mut out = Buf::default();
    out.u16(1).u16(0).u32(0x40).zeros(10);
    for unit in FONT.encode_utf16().take(31) {
        out.u16(unit);
    }
    out.pad_to(160).u32(2);
    out.pad_to(296);
    out.done()
}

/// `stlt`: the style table. Fills, outlines, fonts, alignments, spacing,
/// character settings and, from version 9, the set11 table number their
/// entries in one sequence; four text styles and one graphic style refer
/// to them.
fn stlt() -> Vec<u8> {
    let mut out = Buf::default();
    out.u32(5); // records
    out.u32(2); // fills
    for (entry, record) in [(1u32, 1u32), (2, 2)] {
        out.u32(entry).u32(0).u32(record);
    }
    out.u32(1).u32(3).u32(0).u32(1); // outlines: entry 3, record 1
                                     // Fonts: entry 4, font 1, 12 pt, regular.
    out.u32(1);
    out.u32(4).u32(0).u32(0xffff).u32(0xffff).u32(0xffff).u32(1);
    out.u16(1).u16(0).u32(1).u32(1);
    let size = (12.0 * UNITS_PER_MM / 72.0) as i32;
    out.i32(size).i32(size).i32(size);
    out.u32(0x40).u32(0x40).u32(0x40);
    out.u32(4); // alignments: entries 5 to 8
    for (entry, code) in [(5u32, 1u32), (6, 2), (7, 3), (8, 4)] {
        out.u32(entry).u32(0).u32(code);
    }
    out.u32(1).u32(9).u32(0).u32(0xffff);
    out.u32(0)
        .u32(0)
        .u32(1_000_000)
        .u32(1_000_000)
        .u32(1_000_000);
    out.u32(0).u32(2_000_000).u32(600_000).u32(3_000_000).u32(0);
    out.u32(1).u32(10).u32(0);
    for (position, thickness) in DECORATION_LINES {
        out.u32(0).i32(position).i32(thickness);
    }
    for _ in 0..5 {
        out.u32(0);
    }
    out.u32(1).u32(11).u32(0).u32(0);
    let labels = ["Default", "Centred", "Right", "Justified"];
    for (index, label) in labels.iter().enumerate() {
        out.u32(2).u32(1 + index as u32).u32(0).zeros(8);
        out.u32(label.encode_utf16().count() as u32);
        for unit in label.encode_utf16() {
            out.u16(unit);
        }
        out.u32(1).u32(3);
        out.u32(4).u32(5 + index as u32).u32(9).u32(10).u32(11);
    }
    out.u32(1).u32(GRAPHIC_STYLE_ID).u32(0).zeros(8);
    for unit in "Default Graphic".encode_utf16() {
        out.u16(unit);
    }
    out.u32(2).u32(3);
    out.done()
}

/// Decoration lines of the character settings: position and thickness in
/// millionths of the font size.
const DECORATION_LINES: [(i32, i32); 12] = [
    (-90_909, 45_455),
    (-90_909, 90_909),
    (-90_909, 45_455),
    (-181_819, 45_455),
    (845_455, 45_455),
    (868_182, 90_909),
    (800_000, 45_455),
    (890_910, 45_455),
    (333_333, 45_455),
    (356_060, 90_909),
    (378_788, 45_455),
    (287_878, 45_455),
];

/// `sumi`, the last chunk of RIFF files: the drawing's box relative to the
/// page centre, a flag and the version.
fn sumi(content: (f64, f64, f64, f64), centre: (f64, f64)) -> Vec<u8> {
    let box_ = relative(&content, centre);
    let mut out = Buf::default();
    for value in [box_.0, box_.1, box_.2, box_.3] {
        out.i32(units(value));
    }
    out.u16(1).u16(1).u32(0);
    out.u32(0).u32(0).u32(0);
    out.u32(u32::from(VERSION) * 100);
    out.u32(1).u32(0).u32(1033).u32(0);
    out.u32(u32::from(VERSION) * 100);
    out.done()
}

// ---- configuration record ----------------------------------------------

/// The configuration record of version 12: the version 13 record without
/// its tag, with the fields version 13 added left out.
fn mcfg_12(width: f64, height: f64) -> Vec<u8> {
    const KEEP: u8 = 0;
    const SKIP: u8 = 1;
    const ZEROS: u8 = 2;
    // The steps through the version 13 record: keep, skip or add zeros.
    const STEPS: [(u8, usize); 42] = [
        (SKIP, 8),
        (KEEP, 17),
        (ZEROS, 2),
        (KEEP, 677),
        (ZEROS, 6),
        (KEEP, 58),
        (SKIP, 5),
        (KEEP, 3),
        (SKIP, 26),
        (KEEP, 67),
        (ZEROS, 2),
        (KEEP, 6),
        (SKIP, 8),
        (KEEP, 34),
        (SKIP, 5),
        (KEEP, 9),
        (SKIP, 31),
        (KEEP, 15),
        (SKIP, 31),
        (KEEP, 11),
        (SKIP, 26),
        (KEEP, 155),
        (ZEROS, 2),
        (KEEP, 72),
        (SKIP, 2),
        (ZEROS, 1),
        (KEEP, 15),
        (SKIP, 31),
        (KEEP, 10),
        (SKIP, 26),
        (KEEP, 56),
        (ZEROS, 4),
        (KEEP, 30),
        (ZEROS, 4),
        (KEEP, 9),
        (ZEROS, 4),
        (KEEP, 20),
        (ZEROS, 3),
        (KEEP, 6296),
        (ZEROS, 4),
        (KEEP, 560),
        (SKIP, 4),
    ];
    let source = mcfg_13(width, height);
    let mut out = Vec::with_capacity(8152);
    let mut at = 0;
    for (kind, size) in STEPS {
        match kind {
            KEEP => {
                out.extend_from_slice(
                    source.get(at..at + size).unwrap_or_default(),
                );
                at += size;
            }
            SKIP => at += size,
            _ => out.resize(out.len() + size, 0),
        }
    }
    out.resize(8152, 0);
    out
}

/// The configuration record of versions 13 to 20: the version 21 record
/// with the extra text blocks, tagged 1300.
fn mcfg_13(width: f64, height: f64) -> Vec<u8> {
    let source = mcfg_21(width, height);
    let mut block = [0u8; 371];
    for (at, value) in [
        (3, 1u8),
        (19, 1),
        (20, 12),
        (24, 5),
        (26, 5),
        (32, 196),
        (36, 7),
        (37, 16),
        (62, 1),
        (63, 12),
        (67, 5),
        (69, 5),
        (75, 32),
        (77, 196),
        (79, 7),
        (80, 16),
        (105, 1),
        (106, 12),
        (110, 5),
        (112, 5),
        (118, 196),
        (120, 196),
        (122, 7),
        (123, 16),
        (302, 3),
        (306, 1),
        (310, 1),
        (314, 1),
        (322, 1),
        (346, 2),
        (354, 4),
        (358, 100),
        (370, 1),
    ] {
        block[at] = value;
    }
    let mut out = Vec::with_capacity(8323);
    out.extend_from_slice(source.get(..869).unwrap_or_default());
    out.extend_from_slice(&1000u32.to_le_bytes());
    out.extend_from_slice(&1000u32.to_le_bytes());
    out.extend_from_slice(source.get(869..882).unwrap_or_default());
    out.extend_from_slice(&block);
    out.extend_from_slice(source.get(882..).unwrap_or_default());
    out.resize(8323, 0);
    put(&mut out, 0, &1300u32.to_le_bytes());
    put(&mut out, 4, &(8323u32 - 8).to_le_bytes());
    out
}

/// The configuration record of versions 21 and 22, with the values those
/// files have for a metric drawing.
fn mcfg_21(width: f64, height: f64) -> Vec<u8> {
    const SIZE: usize = 7944;
    const MM: u16 = 2;
    let mut out = vec![0u8; SIZE];
    let (width, height) = (units(width), units(height));
    put(&mut out, 0, &2100u32.to_le_bytes());
    put(&mut out, 4, &(SIZE as u32 - 8).to_le_bytes());
    put(&mut out, 12, &width.to_le_bytes());
    put(&mut out, 16, &height.to_le_bytes());
    put(&mut out, 20, &0u16.to_le_bytes());
    put(&mut out, 22, &u16::from(width > height).to_le_bytes());
    put(&mut out, 24, &1u32.to_le_bytes());
    put(&mut out, 34, &1u32.to_le_bytes());
    put(&mut out, 38, &1u32.to_le_bytes());
    put(&mut out, 42, &31750u32.to_le_bytes());
    put(&mut out, 670, &(-(width / 2)).to_le_bytes());
    put(&mut out, 674, &(-(height / 2)).to_le_bytes());
    put(&mut out, 678, &4.0f64.to_le_bytes());
    put(&mut out, 686, &4.0f64.to_le_bytes());
    put(&mut out, 694, &MM.to_le_bytes());
    put(&mut out, 696, &1.0f64.to_le_bytes());
    put(&mut out, 704, &MM.to_le_bytes());
    put(&mut out, 706, &MM.to_le_bytes());
    put(&mut out, 708, &1u16.to_le_bytes());
    for (at, value) in [
        (716, 11_811_024i32),
        (720, 11_811_024),
        (724, 1),
        (728, 1),
        (732, 1),
        (736, 11_811_024),
        (740, 11_811_024),
        (744, 1),
    ] {
        put(&mut out, at, &value.to_le_bytes());
    }
    put(&mut out, 760, &[0x01, 0x0c]);
    put(&mut out, 767, &[0x05]);
    put(&mut out, 777, &[0x07, 0x10]);
    put(&mut out, 811, &1u32.to_le_bytes());
    put(&mut out, 815, &1u32.to_le_bytes());
    put(&mut out, 819, &[1]);
    for (at, value) in [
        (827, 1i32),
        (831, 50_000),
        (835, 50_000),
        (839, 1_000),
        (843, 2),
        (847, 2),
        (851, 1),
    ] {
        put(&mut out, at, &value.to_le_bytes());
    }
    put(&mut out, 859, &MM.to_le_bytes());
    put(&mut out, 861, &12_700u16.to_le_bytes());
    for at in [869, 873, 877] {
        put(&mut out, at, &[1]);
    }
    put(&mut out, 885, &[0x01, 0x0c, 0, 0, 0, 0x05, 0, 0x05]);
    put(&mut out, 898, &[0xff, 0, 0, 0, 0x07, 0x10]);
    put(&mut out, 928, &[0x01, 0x0c, 0, 0, 0, 0x05, 0, 0x05]);
    put(&mut out, 942, &[0x64, 0, 0, 0x07, 0x10]);
    put(&mut out, 971, &[1]);
    put(&mut out, 1007, &[1]);
    for index in 0..4 {
        put(&mut out, 1019 + 8 * index, &1.0f64.to_le_bytes());
    }
    put(&mut out, 1051, &2u32.to_le_bytes());
    put(&mut out, 1055, &0.2f64.to_le_bytes());
    put(&mut out, 1063, &1u32.to_le_bytes());
    put(&mut out, 1067, &4.0f64.to_le_bytes());
    put(&mut out, 1075, &4.0f64.to_le_bytes());
    put(&mut out, 1083, &[4, 1]);
    put(&mut out, 1100, &[1]);
    put(&mut out, 7360, &12.0f64.to_le_bytes());
    for at in [7368, 7372, 7376] {
        put(&mut out, at, &1u32.to_le_bytes());
    }
    for index in 0..3 {
        put(&mut out, 7380 + 8 * index, &0.8165f64.to_le_bytes());
    }
    for (index, angle) in [330_000_000i32, 90_000_000, 210_000_000]
        .into_iter()
        .enumerate()
    {
        put(&mut out, 7404 + 4 * index, &angle.to_le_bytes());
    }
    let name: Vec<u8> = "Isometric"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    put(&mut out, 7416, &name);
    out
}

fn put(record: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(target) = record.get_mut(at..at + bytes.len()) {
        target.copy_from_slice(bytes);
    }
}
