use crate::document::DesignDocument;
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::{NodeId, SourceSpan, TextPatch};
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::RsxAnimation;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(crate) struct AuthoredRsxFrame {
    source: Box<str>,
    invocation: SourceSpan,
    clips: BTreeMap<NodeId, RsxAnimation>,
}

pub(crate) fn keyframe_edit(tool: &str, action: Option<&str>) -> bool {
    tool == "designer_program" && action == Some("upsert_keyframe")
}

impl AuthoredRsxFrame {
    pub(crate) fn capture(design: &DesignDocument<'_>) -> Option<Self> {
        let invocation = design.rsx.as_ref()?.invocation;
        Some(Self {
            source: design.source.into(),
            invocation,
            clips: design.rsx_animations.clone(),
        })
    }

    pub(crate) fn require_source(&self, design: &DesignDocument<'_>) -> Result<(), String> {
        if design.source == self.source.as_ref()
            && design
                .rsx
                .as_ref()
                .is_some_and(|rsx| rsx.invocation == self.invocation)
        {
            Ok(())
        } else {
            Err("The rendered RSX invocation does not match the current source".into())
        }
    }

    pub(crate) fn annotate_programs(&self, models: &mut Value) {
        let index = LineIndex::new(&self.source);
        if let Some(programs) = models["programs"].as_array_mut() {
            for program in programs {
                let Some(clip) = program["id"].as_u64().and_then(|id| self.clips.get(&id)) else {
                    continue;
                };
                if let Ok(range) = index.range(clip.declaration) {
                    program["sourceRange"] = json!(range);
                }
            }
        }
    }

    pub(crate) fn patches(&self, args: &Value) -> Result<Vec<TextPatch>, String> {
        let clip = args["program"]
            .as_u64()
            .and_then(|id| self.clips.get(&id))
            .ok_or("Choose an animation with a mapped Rust declaration")?;
        clip.patch_keyframe(
            args["property"]
                .as_str()
                .ok_or("Choose a mapped property")?,
            KeyframeEdit::Upsert {
                previous: args["previous"].as_f64(),
                time: args["time"].as_f64().ok_or("Supply a keyframe time")?,
                value: args["value"].as_str().ok_or("Supply a keyframe value")?,
                easing: args["easing"].as_str().ok_or("Supply a keyframe easing")?,
            },
        )
    }
}
