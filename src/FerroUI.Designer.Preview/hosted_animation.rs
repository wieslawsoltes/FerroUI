use crate::hosted_relay::Frame;
use ferroui_designer_animation::CurveWindow;
use ferroui_remote_protocol::designer::{
    DesignAnimationAuthoringResult, DesignAnimationCatalogMessage, DesignAnimationTrack,
};
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

#[derive(Deserialize)]
pub(crate) struct KeyframeDraft {
    pub(crate) revision: i64,
    pub(crate) sequence: i64,
    pub(crate) program: i64,
    pub(crate) target: i64,
    pub(crate) property: String,
    pub(crate) previous: Option<f64>,
    pub(crate) time: f64,
    pub(crate) value: String,
    pub(crate) easing: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurveRequest {
    pub(crate) revision: i64,
    pub(crate) sequence: i64,
    pub(crate) program: i64,
    pub(crate) target: i64,
    pub(crate) property: String,
    pub(crate) window: CurveWindow,
}

fn tracks(tracks: Vec<DesignAnimationTrack>) -> Vec<Value> {
    tracks.into_iter().map(|track| json!({
        "target": track.target,
        "property": track.property,
        "keyframes": track.keys.into_iter().map(|key| json!({
            "time": key.time, "value_text": key.value, "easing_text": key.easing, "value_kind": key.value_kind,
        })).collect::<Vec<_>>(),
    })).collect()
}

pub(crate) fn keyframe(result: DesignAnimationAuthoringResult) -> Result<Value, String> {
    if let Some(error) = result.error {
        return Err(error);
    }
    Ok(json!({
        "samples": result.samples,
        "valueKind": result.value_kind,
        "patch": result.patch.map(|patch| json!({
            "start": patch.start, "end": patch.end, "expected": patch.expected, "replacement": patch.replacement,
        })),
    }))
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
                "sourceOffset": (program.source_utf16 >= 0).then_some(program.source_utf16),
                "customAnimator": program.custom_animator,
                "curveTracks": program.curve_tracks.map(tracks),
                "authoring": program.authoring.map(|authoring| json!({
                    "codec": authoring.codec,
                    "tracks": tracks(authoring.tracks),
                })),
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
