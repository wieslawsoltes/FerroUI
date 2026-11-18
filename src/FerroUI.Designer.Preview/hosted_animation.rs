use crate::hosted_relay::Frame;
use ferroui_remote_protocol::designer::DesignAnimationCatalogMessage;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
pub(crate) struct Command {
    pub(crate) revision: i64,
    pub(crate) sequence: i64,
    pub(crate) program: Option<i64>,
    pub(crate) targets: Option<Vec<i64>>,
    pub(crate) elapsed: f64,
    pub(crate) restart: bool,
}

#[derive(Serialize)]
pub(crate) struct Sample {
    pub(crate) frame: Frame,
    pub(crate) elapsed: f64,
    pub(crate) completed: bool,
}

pub(crate) fn catalog(catalog: DesignAnimationCatalogMessage) -> Value {
    let programs = catalog
        .programs
        .into_iter()
        .map(|program| {
            json!({
                "id": program.id,
                "name": program.name,
                "selector": program.selector,
                "duration": program.duration,
                "totalDuration": program.total_duration,
                "defaultTargets": program.default_targets,
                "compatibleTargets": program.compatible_targets,
                "sourceOffset": program.source_utf16,
                "customAnimator": program.custom_animator,
            })
        })
        .collect::<Vec<_>>();
    let targets = catalog
        .targets
        .into_iter()
        .map(|target| {
            json!({
                "id": target.id,
                "label": target.label,
                "typeName": target.type_name,
            })
        })
        .collect::<Vec<_>>();
    json!({"programs": programs, "targets": targets})
}
