use super::*;
use crate::graphics_layout::{align_boxes, distribute_boxes, AlignMode, DistributeAxis, GraphicBox};
use crate::project::{Graphic, GraphicFill, GraphicKind, GraphicStroke, GraphicShadow, GraphicTemplate};

fn next_id(engine: &mut EditorEngine) -> u64 {
    let id = engine.document.project.next_id;
    engine.document.project.next_id = id.saturating_add(1);
    id
}

fn graphics_mut(engine: &mut EditorEngine) -> &mut Vec<Graphic> {
    &mut engine.document.project.sequence.graphics
}

fn graphic_index(engine: &EditorEngine, id: u64) -> Option<usize> {
    engine
        .document
        .project
        .sequence
        .graphics
        .iter()
        .position(|graphic| graphic.id == id)
}

fn missing(id: u64) -> DispatchError {
    DispatchError::InvalidParameters(format!("Graphic {id} does not exist"))
}

fn box_of(graphic: &Graphic) -> GraphicBox {
    GraphicBox {
        x: graphic.transform.x,
        y: graphic.transform.y,
        width: graphic.transform.width,
        height: graphic.transform.height,
    }
}

/// Applies partial numeric updates; `None` leaves each field untouched.
fn parse_box(mode: &str) -> Result<AlignMode, DispatchError> {
    AlignMode::parse(mode).ok_or_else(|| {
        DispatchError::InvalidParameters(format!("Unknown align mode {mode:?}"))
    })
}

fn parse_axis(axis: &str) -> Result<DistributeAxis, DispatchError> {
    DistributeAxis::parse(axis).ok_or_else(|| {
        DispatchError::InvalidParameters(format!("Unknown distribute axis {axis:?}"))
    })
}


/// Locks block every ink/geometry mutation; timing and visibility stay free.
fn ensure_unlocked(graphic: &Graphic) -> Result<(), DispatchError> {
    if graphic.locked {
        return Err(DispatchError::InvalidParameters(format!(
            "Graphic {} is locked",
            graphic.id
        )));
    }
    Ok(())
}

/// Validates timing invariants shared by graphic commands that set timing.
fn validate_graphic_timing(start: Tick, duration: Tick) -> Result<(), DispatchError> {
    if start.0 < 0 || duration.0 <= 0 {
        return Err(DispatchError::InvalidParameters(
            "Graphic start must be non-negative and duration positive".into(),
        ));
    }
    Ok(())
}

impl EditorEngine {
    pub(super) fn dispatch_graphics(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "graphic.add" => Some((|| {
                let params: GraphicAddParams = parse_params(params)?;
                let start = params.start_ticks.unwrap_or(Tick(0));
                let duration = params.duration_ticks.unwrap_or(crate::project::Graphic::default().duration_ticks);
                validate_graphic_timing(start, duration)?;
                self.checkpoint();
                let graphic_id = next_id(self);
                let kind = params.kind.unwrap_or(GraphicKind::Text);
                let mut graphic = Graphic {
                    id: graphic_id,
                    name: params.name.unwrap_or_else(|| format!("{kind:?} 1")),
                    kind,
                    start_ticks: start,
                    duration_ticks: duration,
                    ..Graphic::default()
                };
                if kind == GraphicKind::Text {
                    if let Some(text) = graphic.text.as_mut() {
                        text.content = params.text.clone().unwrap_or_else(|| "Text".into());
                    }
                } else {
                    graphic.text = None;
                }
                self.document.project.sequence.graphics.push(graphic);
                Ok(json!({ "added": true, "id": graphic_id }))
            })()),
            "graphic.remove" => Some((|| {
                let params: GraphicRemoveParams = parse_params(params)?;
                if graphic_index(self, params.id).is_none() {
                    return Err(missing(params.id));
                }
                self.checkpoint();
                let graphics = graphics_mut(self);
                graphics.retain(|graphic| graphic.id != params.id);
                Ok(json!({ "removed": true }))
            })()),
            "graphic.duplicate" => Some((|| {
                let params: GraphicRemoveParams = parse_params(params)?;
                let index = graphic_index(self, params.id).ok_or_else(|| missing(params.id))?;
                self.checkpoint();
                let duplicate_id = next_id(self);
                let mut copy = self.document.project.sequence.graphics[index].clone();
                copy.id = duplicate_id;
                copy.name = format!("{} copy", copy.name);
                self.document.project.sequence.graphics.push(copy);
                Ok(json!({ "duplicated": true, "id": duplicate_id }))
            })()),
            "graphic.set_flags" => Some((|| {
                let params: GraphicSetFlagsParams = parse_params(params)?;
                let index = graphic_index(self, params.id).ok_or_else(|| missing(params.id))?;
                self.checkpoint();
                let graphic = &mut self.document.project.sequence.graphics[index];
                if let Some(visible) = params.visible {
                    graphic.visible = visible;
                }
                if let Some(locked) = params.locked {
                    graphic.locked = locked;
                }
                Ok(json!({
                    "updated": true,
                    "id": params.id,
                    "visible": graphic.visible,
                    "locked": graphic.locked,
                }))
            })()),
            "graphic.set_transform" => Some((|| {
                let params: GraphicSetTransformParams = parse_params(params)?;
                let index = graphic_index(self, params.id).ok_or_else(|| missing(params.id))?;
                ensure_unlocked(&self.document.project.sequence.graphics[index])?;
                {
                    let graphic = &self.document.project.sequence.graphics[index];
                    let transform = &graphic.transform;
                    let width = params.width.unwrap_or(transform.width);
                    let height = params.height.unwrap_or(transform.height);
                    if width < 0.0 || height < 0.0 {
                        return Err(DispatchError::InvalidParameters(
                            "Graphic size cannot be negative".into(),
                        ));
                    }
                    let opacity = params.opacity.unwrap_or(transform.opacity);
                    if !(0.0..=1.0).contains(&opacity) {
                        return Err(DispatchError::InvalidParameters(
                            "Graphic opacity must be between 0 and 1".into(),
                        ));
                    }
                }
                self.checkpoint();
                let transform = &mut self.document.project.sequence.graphics[index].transform;
                if let Some(x) = params.x {
                    transform.x = x;
                }
                if let Some(y) = params.y {
                    transform.y = y;
                }
                if let Some(width) = params.width {
                    transform.width = width;
                }
                if let Some(height) = params.height {
                    transform.height = height;
                }
                if let Some(scale) = params.scale {
                    transform.scale = scale;
                }
                if let Some(rotation) = params.rotation {
                    transform.rotation = rotation;
                }
                if let Some(anchor_x) = params.anchor_x {
                    transform.anchor_x = anchor_x;
                }
                if let Some(anchor_y) = params.anchor_y {
                    transform.anchor_y = anchor_y;
                }
                if let Some(opacity) = params.opacity {
                    transform.opacity = opacity;
                }
                Ok(json!({ "updated": true, "id": params.id }))
            })()),
            "graphic.set_text" => Some((|| {
                let params: GraphicSetTextParams = parse_params(params)?;
                let index = graphic_index(self, params.id).ok_or_else(|| missing(params.id))?;
                ensure_unlocked(&self.document.project.sequence.graphics[index])?;
                if self.document.project.sequence.graphics[index].text.is_none() {
                    return Err(DispatchError::InvalidParameters(
                        "Graphic is not a text graphic".into(),
                    ));
                }
                if let Some(content) = params.text.as_ref() {
                    if content.trim().is_empty() {
                        return Err(DispatchError::InvalidParameters(
                            "Graphic text cannot be empty".into(),
                        ));
                    }
                }
                self.checkpoint();
                let text = self.document.project.sequence.graphics[index]
                    .text
                    .as_mut()
                    .expect("verified above");
                if let Some(content) = params.text {
                    text.content = content;
                }
                if let Some(font_family) = params.font_family {
                    text.font_family = font_family;
                }
                if let Some(font_size) = params.font_size {
                    text.font_size = font_size;
                }
                if let Some(tracking) = params.tracking {
                    text.tracking = tracking;
                }
                if let Some(leading) = params.leading {
                    text.leading = leading;
                }
                if let Some(align) = params.align {
                    text.align = align;
                }
                if let Some(vertical) = params.vertical {
                    text.vertical = vertical;
                }
                Ok(json!({ "updated": true, "id": params.id }))
            })()),
            "graphic.set_fill" => Some((|| {
                let params: GraphicSetFillParams = parse_params(params)?;
                let index = graphic_index(self, params.id).ok_or_else(|| missing(params.id))?;
                ensure_unlocked(&self.document.project.sequence.graphics[index])?;
                self.checkpoint();
                let fill = self.document.project.sequence.graphics[index]
                    .fill
                    .get_or_insert_with(GraphicFill::default);
                if let Some(color) = params.color {
                    fill.color = color;
                }
                if let Some(opacity) = params.opacity {
                    fill.opacity = opacity;
                }
                Ok(json!({ "updated": true, "id": params.id }))
            })()),
            "graphic.set_stroke" => Some((|| {
                let params: GraphicSetStrokeParams = parse_params(params)?;
                let index = graphic_index(self, params.id).ok_or_else(|| missing(params.id))?;
                ensure_unlocked(&self.document.project.sequence.graphics[index])?;
                if let Some(width) = params.width {
                    if width < 0.0 {
                        return Err(DispatchError::InvalidParameters(
                            "Graphic stroke width cannot be negative".into(),
                        ));
                    }
                }
                self.checkpoint();
                let stroke = self.document.project.sequence.graphics[index]
                    .stroke
                    .get_or_insert_with(GraphicStroke::default);
                if let Some(color) = params.color {
                    stroke.color = color;
                }
                if let Some(width) = params.width {
                    stroke.width = width;
                }
                Ok(json!({ "updated": true, "id": params.id }))
            })()),
            "graphic.set_shadow" => Some((|| {
                let params: GraphicSetShadowParams = parse_params(params)?;
                let index = graphic_index(self, params.id).ok_or_else(|| missing(params.id))?;
                ensure_unlocked(&self.document.project.sequence.graphics[index])?;
                self.checkpoint();
                let shadow = self.document.project.sequence.graphics[index]
                    .shadow
                    .get_or_insert_with(GraphicShadow::default);
                if let Some(color) = params.color {
                    shadow.color = color;
                }
                if let Some(blur) = params.blur {
                    shadow.blur = blur;
                }
                if let Some(offset_x) = params.offset_x {
                    shadow.offset_x = offset_x;
                }
                if let Some(offset_y) = params.offset_y {
                    shadow.offset_y = offset_y;
                }
                Ok(json!({ "updated": true, "id": params.id }))
            })()),
            "graphic.align" => Some((|| {
                let params: GraphicAlignParams = parse_params(params)?;
                let mode = parse_box(&params.mode)?;
                if params.ids.len() < 2 {
                    return Err(DispatchError::InvalidParameters(
                        "Aligning needs at least two graphics".into(),
                    ));
                }
                let mut boxes = Vec::with_capacity(params.ids.len());
                for graphic_id in &params.ids {
                    let index = graphic_index(self, *graphic_id).ok_or_else(|| missing(*graphic_id))?;
                    boxes.push(box_of(&self.document.project.sequence.graphics[index]));
                }
                let positions = align_boxes(&boxes, mode);
                self.checkpoint();
                for (graphic_id, (_box, new_position)) in
                    params.ids.iter().zip(boxes.iter().zip(positions.iter()))
                {
                    let index = graphic_index(self, *graphic_id).expect("verified above");
                    let transform = &mut self.document.project.sequence.graphics[index].transform;
                    match mode {
                        AlignMode::Left | AlignMode::HCenter | AlignMode::Right => transform.x = *new_position,
                        AlignMode::Top | AlignMode::VCenter | AlignMode::Bottom => transform.y = *new_position,
                    }
                }
                Ok(json!({ "aligned": params.ids.len() }))
            })()),
            "graphic.distribute" => Some((|| {
                let params: GraphicDistributeParams = parse_params(params)?;
                let axis = parse_axis(&params.axis)?;
                if params.ids.len() < 3 {
                    return Err(DispatchError::InvalidParameters(
                        "Distributing needs at least three graphics".into(),
                    ));
                }
                let mut boxes = Vec::with_capacity(params.ids.len());
                for graphic_id in &params.ids {
                    let index = graphic_index(self, *graphic_id).ok_or_else(|| missing(*graphic_id))?;
                    boxes.push(box_of(&self.document.project.sequence.graphics[index]));
                }
                let positions = distribute_boxes(&boxes, axis);
                self.checkpoint();
                for (graphic_id, (_box, new_position)) in
                    params.ids.iter().zip(boxes.iter().zip(positions.iter()))
                {
                    let index = graphic_index(self, *graphic_id).expect("verified above");
                    let transform = &mut self.document.project.sequence.graphics[index].transform;
                    match axis {
                        DistributeAxis::Horizontal => transform.x = *new_position,
                        DistributeAxis::Vertical => transform.y = *new_position,
                    }
                }
                Ok(json!({ "distributed": params.ids.len() }))
            })()),
            "graphic.arrange" => Some((|| {
                let params: GraphicArrangeParams = parse_params(params)?;
                let index = graphic_index(self, params.id).ok_or_else(|| missing(params.id))?;
                let current = self.document.project.sequence.graphics[index].layer;
                let scope: Vec<i32> = self
                    .document
                    .project
                    .sequence
                    .graphics
                    .iter()
                    .map(|graphic| graphic.layer)
                    .collect();
                let new_layer = match params.order.as_str() {
                    // Strictly past the current extremes so "front"/"back"
                    // always change the ordering even when layers tie.
                    "front" => scope.iter().copied().max().unwrap_or(0) + 1,
                    "back" => scope.iter().copied().min().unwrap_or(0) - 1,
                    "forward" => scope.iter().copied().filter(|layer| *layer > current).min().map_or(current, |next| next),
                    "backward" => scope.iter().copied().filter(|layer| *layer < current).max().map_or(current, |previous| previous),
                    other => {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Unknown arrange order {other:?}"
                        )))
                    }
                };
                self.checkpoint();
                self.document.project.sequence.graphics[index].layer = new_layer;
                Ok(json!({ "arranged": true, "layer": new_layer }))
            })()),
            "graphic.template_save" => Some((|| {
                let params: GraphicTemplateSaveParams = parse_params(params)?;
                if params.name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Graphic template name cannot be empty".into(),
                    ));
                }
                let index = graphic_index(self, params.graphic_id).ok_or_else(|| missing(params.graphic_id))?;
                self.checkpoint();
                let template_id = next_id(self);
                let body = self.document.project.sequence.graphics[index].clone();
                self.document
                    .project
                    .graphic_templates
                    .push(GraphicTemplate {
                        id: template_id,
                        name: params.name,
                        body,
                    });
                Ok(json!({ "saved": true, "id": template_id }))
            })()),
            "fonts.list" => Some((|| {
                let mut guard = self
                    .text
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let context = guard.get_or_insert_with(saturn_text::TextContext::new);
                let fonts = context.font_families();
                let count = fonts.len();
                Ok(json!({ "fonts": fonts, "count": count }))
            })()),
            "graphic.template_apply" => Some((|| {
                let params: GraphicTemplateApplyParams = parse_params(params)?;
                let template_index = self
                    .document
                    .project
                    .graphic_templates
                    .iter()
                    .position(|template| template.id == params.template_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Graphic template {} does not exist",
                            params.template_id
                        ))
                    })?;
                self.checkpoint();
                let graphic_id = next_id(self);
                let start = params.start_ticks.unwrap_or(Tick(0));
                validate_graphic_timing(start, self.document.project.graphic_templates[template_index].body.duration_ticks)?;
                let mut body = self.document.project.graphic_templates[template_index].body.clone();
                body.id = graphic_id;
                body.start_ticks = start;
                self.document.project.sequence.graphics.push(body);
                Ok(json!({ "applied": true, "id": graphic_id }))
            })()),
            _ => None,
        }
    }
}

// ---- params ----

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicAddParams {
    #[serde(default)]
    pub(in crate::engine) kind: Option<GraphicKind>,
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
    #[serde(default)]
    pub(in crate::engine) start_ticks: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) duration_ticks: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) text: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicRemoveParams {
    pub(in crate::engine) id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicSetTransformParams {
    pub(in crate::engine) id: u64,
    #[serde(default)]
    pub(in crate::engine) x: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) y: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) width: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) height: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) scale: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) rotation: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) anchor_x: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) anchor_y: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) opacity: Option<f64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicSetTextParams {
    pub(in crate::engine) id: u64,
    #[serde(default)]
    pub(in crate::engine) text: Option<String>,
    #[serde(default)]
    pub(in crate::engine) font_family: Option<String>,
    #[serde(default)]
    pub(in crate::engine) font_size: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) tracking: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) leading: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) align: Option<String>,
    #[serde(default)]
    pub(in crate::engine) vertical: Option<bool>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicSetFillParams {
    pub(in crate::engine) id: u64,
    #[serde(default)]
    pub(in crate::engine) color: Option<String>,
    #[serde(default)]
    pub(in crate::engine) opacity: Option<f64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicSetStrokeParams {
    pub(in crate::engine) id: u64,
    #[serde(default)]
    pub(in crate::engine) color: Option<String>,
    #[serde(default)]
    pub(in crate::engine) width: Option<f64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicSetShadowParams {
    pub(in crate::engine) id: u64,
    #[serde(default)]
    pub(in crate::engine) color: Option<String>,
    #[serde(default)]
    pub(in crate::engine) blur: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) offset_x: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) offset_y: Option<f64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicSetFlagsParams {
    pub(in crate::engine) id: u64,
    #[serde(default)]
    pub(in crate::engine) visible: Option<bool>,
    #[serde(default)]
    pub(in crate::engine) locked: Option<bool>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicAlignParams {
    pub(in crate::engine) ids: Vec<u64>,
    pub(in crate::engine) mode: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicDistributeParams {
    pub(in crate::engine) ids: Vec<u64>,
    pub(in crate::engine) axis: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicArrangeParams {
    pub(in crate::engine) id: u64,
    pub(in crate::engine) order: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicTemplateSaveParams {
    pub(in crate::engine) graphic_id: u64,
    pub(in crate::engine) name: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct GraphicTemplateApplyParams {
    pub(in crate::engine) template_id: u64,
    #[serde(default)]
    pub(in crate::engine) start_ticks: Option<Tick>,
}
