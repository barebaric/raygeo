//! Text box entity, mirroring the reference `TextBoxEntity` class.

use serde_json::{json, Value};

use crate::geo::geometry::Geometry;
use crate::geo::math::map_geometry_to_frame;
use crate::geo::shape::line::does_line_segment_intersect_rect;
use crate::geo::shape::polygon::is_point_inside_polygon;
use crate::geo::shape::text::{
    get_text_width, text_to_geometry, FontConfig as CoreFontConfig,
};
use crate::geo::types::{Point, Rect};

use super::entity::{Entity, EntityBase};
use crate::sketch::registry::EntityRegistry;

/// A text box: content rendered through raygeo into glyph geometry,
/// framed by origin/width/height points.
#[derive(Clone, Debug)]
pub struct TextBoxEntity {
    pub base: EntityBase,
    pub origin: u64,
    pub width: u64,
    pub height: u64,
    pub content: String,
    pub font_config: CoreFontConfig,
    pub construction_line_ids: Vec<u64>,
    pub fill_color: Option<super::entity::ColorRgba>,
    /// Unknown `font_config` keys preserved for forward compat.
    pub font_extra: serde_json::Map<String, Value>,
}

impl TextBoxEntity {
    pub fn new(
        id: u64,
        origin: u64,
        width: u64,
        height: u64,
        content: String,
        construction: bool,
    ) -> Self {
        TextBoxEntity {
            base: EntityBase {
                id,
                construction,
                invisible: false,
                array_copy: false,
                constrained: false,
            },
            origin,
            width,
            height,
            content,
            font_config: CoreFontConfig::new("sans-serif", 10.0),
            construction_line_ids: Vec::new(),
            fill_color: None,
            font_extra: serde_json::Map::new(),
        }
    }

    pub fn from_json(data: &Value) -> Option<TextBoxEntity> {
        let fill_color = data.get("fill_color").and_then(|value| {
            let array = value.as_array()?;
            Some([
                array.first()?.as_f64()?,
                array.get(1)?.as_f64()?,
                array.get(2)?.as_f64()?,
                array.get(3)?.as_f64()?,
            ])
        });
        Some(TextBoxEntity {
            base: super::entity::base_from_json(data),
            origin: data.get("origin_id")?.as_u64()?,
            width: data.get("width_id")?.as_u64()?,
            height: data.get("height_id")?.as_u64()?,
            content: data
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            font_config: core_font_from_dict(data.get("font_config")),
            font_extra: {
                let mut extra = serde_json::Map::new();
                if let Some(map) =
                    data.get("font_config").and_then(Value::as_object)
                {
                    let known = ["font_family", "font_size", "bold", "italic"];
                    for (key, value) in map {
                        if !known.contains(&key.as_str()) {
                            extra.insert(key.clone(), value.clone());
                        }
                    }
                }
                extra
            },
            construction_line_ids: data
                .get("construction_line_ids")
                .and_then(Value::as_array)
                .map(|array| array.iter().filter_map(Value::as_u64).collect())
                .unwrap_or_default(),
            fill_color,
        })
    }

    pub fn to_json(&self) -> Value {
        let base = self.base.to_base_json();
        let mut font = json!({
            "font_family": self.font_config.family,
            "font_size": self.font_config.size,
            "bold": self.font_config.bold,
            "italic": self.font_config.italic,
        });
        if let Some(map) = font.as_object_mut() {
            for (key, value) in &self.font_extra {
                map.insert(key.clone(), value.clone());
            }
        }
        let mut extra = json!({
            "type": self.type_name(),
            "origin_id": self.origin,
            "width_id": self.width,
            "height_id": self.height,
            "content": self.content,
            "font_config": font,
            "construction_line_ids": self.construction_line_ids,
        });
        if let Some(color) = self.fill_color {
            extra["fill_color"] = json!(color);
        }
        EntityBase::combined_json(&base, extra)
    }

    /// The 4-corner frame polygon, or `None` if points are missing.
    /// Corner 4 comes from the construction line when present,
    /// otherwise from the parallelogram identity.
    pub fn get_frame_polygon(
        &self,
        model: &EntityRegistry,
    ) -> Option<Vec<(f64, f64)>> {
        let origin = model.point_position(self.origin)?;
        let width = model.point_position(self.width)?;
        let height = model.point_position(self.height)?;

        let (p4_x, p4_y) = match self.get_fourth_corner_id(model) {
            Some(p4_id) => model.point_position(p4_id).unwrap_or((
                width.0 + height.0 - origin.0,
                width.1 + height.1 - origin.1,
            )),
            None => {
                (width.0 + height.0 - origin.0, width.1 + height.1 - origin.1)
            }
        };

        Some(vec![origin, width, (p4_x, p4_y), height])
    }

    /// All 4 corner point ids of the text box frame.
    pub fn get_all_frame_point_ids(&self, model: &EntityRegistry) -> Vec<u64> {
        let mut ids = vec![self.origin, self.width, self.height];
        if let Some(p4_id) = self.get_fourth_corner_id(model) {
            ids.push(p4_id);
        }
        ids
    }

    /// The 4th corner point id, taken from the construction line.
    pub fn get_fourth_corner_id(&self, model: &EntityRegistry) -> Option<u64> {
        for eid in &self.construction_line_ids {
            if let Some(entity) = model.get_entity(*eid) {
                if entity.type_name() == "line" {
                    let endpoints = entity.get_endpoint_ids();
                    let (a, b) = (
                        endpoints.first().copied().unwrap_or(0),
                        endpoints.get(1).copied().unwrap_or(0),
                    );
                    if a == self.width && b != self.origin && b != self.height {
                        return Some(b);
                    }
                    if b == self.width && a != self.origin && a != self.height {
                        return Some(a);
                    }
                }
            }
        }
        None
    }

    /// Natural (width, height) of the text content.  The height is
    /// derived from the glyph bounding box so the frame matches the
    /// ink extent exactly and map_to_frame does not stretch the
    /// text.  raygeo returns Y-UP geometry.
    pub fn get_natural_size(&self, content: Option<&str>) -> (f64, f64) {
        let text = content.unwrap_or(&self.content);
        if text.is_empty() {
            return (10.0, self.font_config.size);
        }
        let Some(geo) = text_to_geometry(text, &self.font_config) else {
            return (10.0, self.font_config.size);
        };
        let rect = geo.rect();
        let geo_height = rect.max.y - rect.min.y;
        (
            get_text_width(text, &self.font_config)
                .unwrap_or(0.0)
                .max(1.0),
            geo_height.max(1.0),
        )
    }

    fn build_frame_for_content(
        &self,
        model: &EntityRegistry,
        content: &str,
    ) -> ((f64, f64), (f64, f64), (f64, f64)) {
        let Some(origin) = model.point_position(self.origin) else {
            return ((0.0, 0.0), (0.0, 0.0), (0.0, 0.0));
        };
        let Some(p_width) = model.point_position(self.width) else {
            return ((0.0, 0.0), (0.0, 0.0), (0.0, 0.0));
        };
        let Some(p_height) = model.point_position(self.height) else {
            return ((0.0, 0.0), (0.0, 0.0), (0.0, 0.0));
        };

        if content == self.content || self.content.is_empty() {
            return (origin, p_width, p_height);
        }

        let (nat_w, nat_h) = self.get_natural_size(Some(content));

        let (dx, dy) = (p_width.0 - origin.0, p_width.1 - origin.1);
        let frame_w = dx.hypot(dy);
        let w_scale = if frame_w < 1e-9 { 1.0 } else { nat_w / frame_w };
        let scaled_width = (origin.0 + dx * w_scale, origin.1 + dy * w_scale);

        let (hx, hy) = (p_height.0 - origin.0, p_height.1 - origin.1);
        let frame_h = hx.hypot(hy);
        let h_scale = if frame_h < 1e-9 { 1.0 } else { nat_h / frame_h };
        let scaled_height = (origin.0 + hx * h_scale, origin.1 + hy * h_scale);

        (origin, scaled_width, scaled_height)
    }
}

impl Entity for TextBoxEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut EntityBase {
        &mut self.base
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn type_name(&self) -> &'static str {
        "text_box"
    }

    fn get_all_frame_point_ids(&self, model: &EntityRegistry) -> Vec<u64> {
        TextBoxEntity::get_all_frame_point_ids(self, model)
    }

    fn to_json(&self) -> Value {
        TextBoxEntity::to_json(self)
    }

    fn get_point_ids(&self) -> Vec<u64> {
        vec![self.origin, self.width, self.height]
    }

    fn get_junction_point_ids(&self) -> Vec<u64> {
        vec![self.origin, self.width, self.height]
    }

    fn get_helper_ids(&self) -> Vec<u64> {
        self.construction_line_ids.clone()
    }

    fn clear_helper_references(&mut self) {
        self.construction_line_ids = Vec::new();
    }

    fn supports_fill(&self) -> bool {
        true
    }

    fn set_fill_color(
        &mut self,
        color: Option<super::entity::ColorRgba>,
    ) -> bool {
        self.fill_color = color;
        true
    }

    fn hit_test(
        &self,
        mx: f64,
        my: f64,
        _threshold: f64,
        model: &EntityRegistry,
    ) -> bool {
        match self.get_frame_polygon(model) {
            Some(polygon) => is_point_inside_polygon(
                Point::new(mx, my),
                &to_points(&polygon),
            ),
            None => false,
        }
    }

    fn get_state(&self) -> Option<Value> {
        let fill = self.fill_color.map(|c| json!(c));
        Some(json!({
            "construction": self.base.construction,
            "fill_color": fill,
        }))
    }

    fn set_state(&mut self, state: &Value) {
        if let Some(v) = state.get("construction").and_then(Value::as_bool) {
            self.base.construction = v;
        }
        if let Some(array) = state.get("fill_color").and_then(Value::as_array) {
            if array.len() == 4 {
                self.fill_color = Some([
                    array[0].as_f64().unwrap_or(0.0),
                    array[1].as_f64().unwrap_or(0.0),
                    array[2].as_f64().unwrap_or(0.0),
                    array[3].as_f64().unwrap_or(0.0),
                ]);
            }
        } else {
            self.fill_color = None;
        }
    }

    fn compute_constrained(
        &self,
        model: &EntityRegistry,
        _constrains_radius: &dyn Fn(u64) -> bool,
    ) -> bool {
        let o = model.get_point(self.origin).map(|p| p.constrained);
        let w = model.get_point(self.width).map(|p| p.constrained);
        let h = model.get_point(self.height).map(|p| p.constrained);
        o.unwrap_or(false) && w.unwrap_or(false) && h.unwrap_or(false)
    }

    fn geometry_signature(&self, model: &EntityRegistry) -> Value {
        let mut signature =
            match super::entity::points_signature(model, &self.get_point_ids())
            {
                Value::Array(array) => array,
                _ => Vec::new(),
            };
        let font = &self.font_config;
        signature.push(json!(self.content));
        signature.push(json!(font.family));
        signature.push(json!(font.size));
        signature.push(json!(font.bold));
        signature.push(json!(font.italic));
        Value::Array(signature)
    }

    fn to_polylines(
        &self,
        model: &EntityRegistry,
        tolerance: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        // Every glyph contour becomes its own polyline so previews
        // show the full text instead of a single contour.
        self.to_geometry(model, None)
            .to_polygons(tolerance)
            .into_iter()
            .map(|polygon| polygon.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    fn is_contained_by(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let Some(points) = self.get_frame_polygon(model) else {
            return false;
        };
        points.iter().all(|(px, py)| {
            rect.min.x <= *px
                && *px <= rect.max.x
                && rect.min.y <= *py
                && *py <= rect.max.y
        })
    }

    fn intersects_rect(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let Some(points) = self.get_frame_polygon(model) else {
            return false;
        };
        for i in 0..4 {
            let p1 = points[i];
            let p2 = points[(i + 1) % 4];
            if does_line_segment_intersect_rect(
                Point::new(p1.0, p1.1),
                Point::new(p2.0, p2.1),
                rect,
            ) {
                return true;
            }
        }
        points.iter().any(|(px, py)| {
            rect.min.x <= *px
                && *px <= rect.max.x
                && rect.min.y <= *py
                && *py <= rect.max.y
        })
    }

    fn to_geometry(&self, model: &EntityRegistry) -> Geometry {
        // The reference signature takes resolved content; the model
        // uses the stored content.
        self.to_geometry(model, None)
    }

    fn append_to_geometry(
        &self,
        _geo: &mut Geometry,
        _model: &EntityRegistry,
        _forward: bool,
    ) {
    }
}

fn to_points(polygon: &[(f64, f64)]) -> Vec<Point> {
    polygon.iter().map(|(x, y)| Point::new(*x, *y)).collect()
}

/// Reads a reference font_config dictionary into the raygeo core
/// font config (unknown keys are dropped here; the model-level
/// serialization preserves them).
/// Converts the model-level font config to the raygeo core font
/// config used by `text_to_geometry`.
pub fn core_font_from_model(
    font: &super::entity::FontConfig,
) -> CoreFontConfig {
    let mut config = CoreFontConfig::new(&font.family, font.size);
    config.bold = font.bold;
    config.italic = font.italic;
    config
}

fn core_font_from_dict(data: Option<&Value>) -> CoreFontConfig {
    let mut config = CoreFontConfig::new("sans-serif", 10.0);
    let Some(data) = data else {
        return config;
    };
    if let Some(family) = data.get("font_family").and_then(Value::as_str) {
        config.family = family.to_string();
    }
    if let Some(size) = data.get("font_size").and_then(Value::as_f64) {
        config.size = size;
    }
    if let Some(bold) = data.get("bold").and_then(Value::as_bool) {
        config.bold = bold;
    }
    if let Some(italic) = data.get("italic").and_then(Value::as_bool) {
        config.italic = italic;
    }
    config
}

impl TextBoxEntity {
    /// Converts the text box to glyph geometry, framed to the
    /// resolved content.
    pub fn to_geometry(
        &self,
        model: &EntityRegistry,
        resolved_content: Option<&str>,
    ) -> Geometry {
        let text = resolved_content.unwrap_or(self.content.as_str());
        let (origin, p_width, p_height) =
            self.build_frame_for_content(model, text);
        let Some(txt_geo) = text_to_geometry(text, &self.font_config) else {
            return Geometry::new();
        };
        let rect = txt_geo.rect();
        let advance_width = get_text_width(text, &self.font_config)
            .unwrap_or(0.0)
            .max(1.0);
        map_geometry_to_frame(
            &txt_geo,
            Point::new(origin.0, origin.1),
            Point::new(p_width.0, p_width.1),
            Point::new(p_height.0, p_height.1),
            Some(rect.min.y),
            Some(rect.max.y - rect.min.y),
            Some(0.0),
            Some(advance_width),
        )
    }
}
