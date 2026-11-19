use ferroui_designer_tools::{
    CanvasGuideEdit, CanvasGuideSettings, CanvasGuideUpdate, CANVAS_RULER,
};
use serde::Deserialize;
use serde_json::{json, Value};

pub(crate) fn catalog() -> Value {
    let defaults = CanvasGuideSettings {
        snap_to_grid: false,
        snap_to_guides: false,
        snap_to_objects: false,
        ..Default::default()
    };
    json!({
        "defaults": defaults, "ruler": CANVAS_RULER, "snapping": true,
        "maxGuides": CanvasGuideSettings::MAX_GUIDES, "maxPosition": CanvasGuideSettings::MAX_POSITION,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    settings: CanvasGuideSettings,
    edit: CanvasGuideEdit,
}

impl Request {
    pub(crate) fn plan(self) -> Result<CanvasGuideUpdate, String> {
        self.edit.plan(&self.settings)
    }
}
