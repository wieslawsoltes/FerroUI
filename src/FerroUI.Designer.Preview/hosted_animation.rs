use crate::hosted_relay::Frame;
use ferroui_designer_animation::{AnimationClipEdit, ClipClock, CurveChannel, CurveMetric, CurveWindow, KeyframeSelection};
use ferroui_designer_core::TextPatch;
use ferroui_remote_protocol::designer::{
    DesignAnimationAuthoringResult, DesignAnimationCatalogMessage, DesignAnimationClipBinding,
    DesignAnimationSourcePatch, DesignAnimationTrack,
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
    pub(crate) clips: Option<Vec<ClipBinding>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClipBinding {
    program: i64,
    targets: Option<Vec<i64>>,
}

impl From<ClipBinding> for DesignAnimationClipBinding {
    fn from(binding: ClipBinding) -> Self {
        Self { program: binding.program, targets: binding.targets }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClipDraft {
    pub(crate) revision: i64,
    pub(crate) sequence: i64,
    pub(crate) programs: Vec<i64>,
    pub(crate) start: Option<f64>,
    #[serde(default)]
    pub(crate) gap: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClipEditDraft {
    pub(crate) revision: i64,
    pub(crate) sequence: i64,
    pub(crate) command: AnimationClipEdit,
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
pub(crate) struct KeyframeSelectionDraft {
    pub(crate) revision: i64,
    pub(crate) sequence: i64,
    pub(crate) program: i64,
    pub(crate) command: KeyframeSelection,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StateDraft {
    pub(crate) revision: i64,
    pub(crate) sequence: i64,
    pub(crate) program: i64,
    pub(crate) duration: f64,
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
    #[serde(default)]
    pub(crate) metric: CurveMetric,
    #[serde(default)]
    pub(crate) channel: CurveChannel,
}

fn tracks(tracks: Vec<DesignAnimationTrack>) -> Vec<Value> {
    tracks.into_iter().map(|track| json!({
        "target": track.target,
        "property": track.property,
        "curve_channels": track.keys.first().map(|key| CurveChannel::for_authored_kind(&key.value_kind)).unwrap_or_default(),
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
        "patch": result.patch.map(source_patch),
    }))
}

pub(crate) fn source_patch(patch: DesignAnimationSourcePatch) -> Value {
    json!({ "start": patch.start, "end": patch.end, "expected": patch.expected, "replacement": patch.replacement })
}

pub(crate) fn checked_source_patch(before: &str, after: &str) -> Result<Option<DesignAnimationSourcePatch>, String> {
    TextPatch::between(before, after)
        .map(|patch| DesignAnimationSourcePatch::from_utf8(before, patch.span.range(), patch.replacement))
        .transpose()
}

pub(crate) fn catalog(catalog: DesignAnimationCatalogMessage) -> Value {
    let programs = catalog
        .programs
        .into_iter()
        .map(|program| {
            let clip = program.clip.map(|clip| {
                let clock = ClipClock { delay: clip.delay, speed_ratio: clip.speed_ratio, end: program.total_duration };
                let span = clock.span();
                json!({ "start": span.start, "end": span.end, "editable": clip.editable })
            });
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
                "clip": clip,
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
    json!({"programs": programs, "targets": targets, "keyframeSelection": catalog.keyframe_selection.unwrap_or(false)})
}
