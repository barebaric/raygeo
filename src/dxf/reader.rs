//! The DXF reader: file text in, geometry out.

use std::collections::HashMap;

use glam::{DMat4, DVec2, DVec3};

use crate::dxf::entity::{Block, Entity, Insert};
use crate::dxf::entity_parser::{parse_records, record_type, SectionContent};
use crate::dxf::geometry_builder::append_shape;
use crate::dxf::group_code::{find, split_records, tokenize, Item};
use crate::dxf::shapes::transform_shape;
use crate::error::RaygeoResult;
use crate::geo::geometry::Geometry;
use crate::geo::read_output::ReadOutput;
use crate::geo::types::Rect;

/// The layer name reported for drawings with no layer structure.
pub const DEFAULT_LAYER: &str = "0";

/// How deep block insertions may nest before the reader stops.
const MAX_BLOCK_DEPTH: usize = 32;

/// What the reader learned that is not geometry.
#[derive(Clone, Debug, Default)]
pub struct DxfHeader {
    /// The `$INSUNITS` value, or `0` when the file does not state units.
    pub insunits: i64,
    /// The CAD application's format marker, from `$ACADVER`.
    pub acad_version: String,
    /// The declared drawing extents, when present.
    pub extents: Option<Rect>,
}

/// Converts a `$INSUNITS` code to the factor that brings a coordinate to
/// millimetres.
///
/// Returns `None` for unitless drawings (`0`) and for codes raygeo does
/// not know, so the caller can decide on a default.
pub fn insunits_to_mm(insunits: i64) -> Option<f64> {
    match insunits {
        1 => Some(25.4),
        2 => Some(304.8),
        3 => Some(1_609_344.0),
        4 => Some(1.0),
        5 => Some(10.0),
        6 => Some(1000.0),
        7 => Some(1_000_000.0),
        8 => Some(25.4e-6),
        9 => Some(25.4e-3),
        10 => Some(914.4),
        11 => Some(1e-7),
        12 => Some(1e-6),
        13 => Some(1e-3),
        14 => Some(100.0),
        _ => None,
    }
}

/// Reads a DXF document.
///
/// Returns the geometry grouped by layer, in native drawing units, Y up.
/// Content the reader cannot represent is listed in the output's issues
/// rather than failing the whole document, so a partly unreadable file
/// still imports.
pub fn read(data: &str) -> RaygeoResult<ReadOutput> {
    let items = tokenize(data)?;
    let sections = collect_sections(&items);
    let empty = Vec::new();
    let header = parse_header(sections.get("HEADER").unwrap_or(&empty));
    let blocks = parse_blocks(sections.get("BLOCKS").unwrap_or(&empty));
    let mut issues = Vec::new();
    // The section's own SECTION record is metadata; only the records
    // after it are entities.
    let entity_records = sections.get("ENTITIES").map(Vec::as_slice);
    let content = parse_records(
        entity_records
            .map(|records| &records[1..])
            .unwrap_or(&empty),
        DMat4::IDENTITY,
        None,
        &mut issues,
    );
    let entities = resolve_blocks(content, &blocks, &mut issues);
    Ok(build_output(entities, &header, issues))
}

/// Splits the file into sections, keyed by section name, each holding
/// that section's records.
fn collect_sections(items: &[Item]) -> HashMap<String, Vec<&[Item]>> {
    let mut sections: HashMap<String, Vec<&[Item]>> = HashMap::new();
    let records = split_records(items);
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        let is_section = record_type(record)
            .is_some_and(|kind| kind.eq_ignore_ascii_case("SECTION"));
        if !is_section {
            index += 1;
            continue;
        }
        let name = find(record, 2)
            .map(|item| item.as_str().to_uppercase())
            .unwrap_or_default();
        let body_start = index + 1;
        let mut body_end = body_start;
        while body_end < records.len() {
            let ends = record_type(records[body_end])
                .is_some_and(|kind| kind.eq_ignore_ascii_case("ENDSEC"));
            if ends {
                break;
            }
            body_end += 1;
        }
        // The SECTION record carries the section name and, for the
        // header, the first variable; the records up to ENDSEC carry the
        // rest. Both go into the section's list.
        let section = sections.entry(name).or_default();
        section.push(record);
        section.extend_from_slice(&records[body_start..body_end]);
        index = body_end + 1;
    }
    sections
}

/// Reads the header variables.
///
/// Header variables are not entities, so they do not start with code 0
/// and do not form records of their own. Each is introduced by a code 9
/// item naming it, followed by one or more value items.
fn parse_header(records: &[&[Item]]) -> DxfHeader {
    let mut header = DxfHeader::default();
    let mut ext_min: Option<glam::DVec2> = None;
    let mut ext_max: Option<glam::DVec2> = None;
    let mut pending: Option<glam::DVec2> = None;
    let mut current: Option<&str> = None;
    for item in records.iter().flat_map(|record| record.iter()) {
        if item.code == 9 {
            current = Some(item.as_str());
            pending = None;
            continue;
        }
        match current {
            Some("$INSUNITS") => header.insunits = item.as_i64_or(0),
            Some("$ACADVER") => header.acad_version = item.as_str().to_string(),
            Some(name @ ("$EXTMIN" | "$EXTMAX")) => {
                let point = pending.get_or_insert(DVec2::ZERO);
                match item.code {
                    10 => point.x = item.as_f64_or(0.0),
                    20 => {
                        point.y = item.as_f64_or(0.0);
                        if name == "$EXTMIN" {
                            ext_min = Some(*point);
                        } else {
                            ext_max = Some(*point);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    if let (Some(min), Some(max)) = (ext_min, ext_max) {
        header.extents = Some(Rect::new(min.x, min.y, max.x, max.y));
    }
    header
}

/// Parses the BLOCKS section into named block definitions.
fn parse_blocks(records: &[&[Item]]) -> HashMap<String, Block> {
    let mut blocks = HashMap::new();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        let is_block = record_type(record)
            .is_some_and(|kind| kind.eq_ignore_ascii_case("BLOCK"));
        if !is_block {
            index += 1;
            continue;
        }
        let name = find(record, 2)
            .map(|item| item.as_str().to_string())
            .unwrap_or_default();
        let base_point = DVec3::new(
            find(record, 10).map(|i| i.as_f64_or(0.0)).unwrap_or(0.0),
            find(record, 20).map(|i| i.as_f64_or(0.0)).unwrap_or(0.0),
            find(record, 30).map(|i| i.as_f64_or(0.0)).unwrap_or(0.0),
        );
        let body_start = index + 1;
        let mut body_end = body_start;
        while body_end < records.len() {
            let is_end = record_type(records[body_end])
                .is_some_and(|kind| kind.eq_ignore_ascii_case("ENDBLK"));
            if is_end {
                break;
            }
            body_end += 1;
        }
        let end = body_end.min(records.len());
        let mut nested_issues = Vec::new();
        let content = parse_records(
            &records[body_start..end],
            DMat4::IDENTITY,
            None,
            &mut nested_issues,
        );
        blocks.insert(
            name.clone(),
            Block {
                name,
                entities: content_entities(content),
                base_point,
            },
        );
        index = end + 1;
    }
    blocks
}

fn content_entities(content: SectionContent) -> Vec<Entity> {
    content
        .shapes
        .into_iter()
        .map(|(layer, shape)| Entity { layer, shape })
        .collect()
}

/// Replaces each insertion with the block's entities, transformed into
/// world space.
fn resolve_blocks(
    content: SectionContent,
    blocks: &HashMap<String, Block>,
    issues: &mut Vec<String>,
) -> Vec<Entity> {
    let mut out = Vec::new();
    for (layer, shape) in content.shapes {
        out.push(Entity { layer, shape });
    }
    for (insert_layer, insert) in content.inserts {
        expand_insert(&insert, &insert_layer, blocks, &mut out, issues, 0);
    }
    out
}

fn expand_insert(
    insert: &Insert,
    insert_layer: &str,
    blocks: &HashMap<String, Block>,
    out: &mut Vec<Entity>,
    issues: &mut Vec<String>,
    depth: usize,
) {
    if depth >= MAX_BLOCK_DEPTH {
        push_issue(
            issues,
            format!("block nesting deeper than {MAX_BLOCK_DEPTH} levels"),
        );
        return;
    }
    let Some(block) = blocks.get(&insert.block_name) else {
        push_issue(
            issues,
            format!(
                "block '{}' is referenced but not defined",
                insert.block_name
            ),
        );
        return;
    };
    let local_transform = DMat4::from_translation(-block.base_point);
    let transform = insert.transform * local_transform;
    for entity in &block.entities {
        let layer = if entity.layer.is_empty() || entity.layer == "0" {
            insert_layer.to_string()
        } else {
            entity.layer.clone()
        };
        out.push(Entity {
            layer,
            shape: transform_shape(&entity.shape, transform),
        });
    }
}

fn push_issue(issues: &mut Vec<String>, message: String) {
    if !issues.contains(&message) {
        issues.push(message);
    }
}

fn build_output(
    entities: Vec<Entity>,
    header: &DxfHeader,
    issues: Vec<String>,
) -> ReadOutput {
    let mut by_layer: HashMap<String, Geometry> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for entity in &entities {
        let name = if entity.layer.is_empty() {
            DEFAULT_LAYER.to_string()
        } else {
            entity.layer.clone()
        };
        if !by_layer.contains_key(&name) {
            order.push(name.clone());
            by_layer.insert(name.clone(), Geometry::new());
        }
        if let Some(geo) = by_layer.get_mut(&name) {
            append_shape(geo, &entity.shape);
        }
        let (shape_min, shape_max) =
            crate::dxf::shapes::shape_z_range(&entity.shape);
        min_z = min_z.min(shape_min);
        max_z = max_z.max(shape_max);
    }
    let mut bounds = Rect::new(
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    let mut layers = Vec::new();
    for name in order {
        let Some(geo) = by_layer.remove(&name) else {
            continue;
        };
        if geo.is_empty() {
            continue;
        }
        let rect = geo.rect();
        bounds.min.x = bounds.min.x.min(rect.min.x);
        bounds.min.y = bounds.min.y.min(rect.min.y);
        bounds.max.x = bounds.max.x.max(rect.max.x);
        bounds.max.y = bounds.max.y.max(rect.max.y);
        layers.push((name, geo));
    }
    if !bounds.min.x.is_finite() {
        bounds = Rect::new(0.0, 0.0, 0.0, 0.0);
    }
    if !min_z.is_finite() {
        min_z = 0.0;
        max_z = 0.0;
    }
    ReadOutput {
        layers,
        bounds,
        units_to_mm: insunits_to_mm(header.insunits).unwrap_or(1.0),
        is_y_down: false,
        z_range: (min_z, max_z),
        issues,
    }
}
