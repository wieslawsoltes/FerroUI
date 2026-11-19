use crate::document::DesignDocument;
use ferroui_designer_animation::{KeyframeEdit, KeyframeGroupEdit, KeyframeLocation, VisualStateGroup};
use ferroui_designer_core::{NodeId, SourceSpan, TextPatch};
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::{RsxAnimation, RsxAnimationIssue, RsxStateGroup};
use ferroui_remote_protocol::designer::{DesignAnimationProgram, DesignAnimationSourcePatch};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(crate) struct AuthoredRsxFrame {
    source: Box<str>,
    invocation: SourceSpan,
    clips: BTreeMap<NodeId, RsxAnimation>,
    states: BTreeMap<(NodeId, String), RsxStateGroup>,
    issues: Vec<RsxAnimationIssue>,
}

pub(crate) fn keyframe_edit(tool: &str, action: Option<&str>) -> bool {
    tool == "designer_program" && matches!(action, Some("upsert_keyframe" | "move_keyframes"))
}

pub(crate) fn state_edit(tool: &str, action: Option<&str>) -> bool {
    tool == "designer_state" && action == Some("upsert")
}

impl AuthoredRsxFrame {
    pub(crate) fn capture(design: &DesignDocument<'_>) -> Option<Self> {
        let rsx = design.rsx.as_ref()?;
        Some(Self {
            source: design.source.into(),
            invocation: rsx.invocation,
            clips: design.rsx_animations.clone(),
            states: design.rsx_states.clone(),
            issues: rsx.animation_issues.clone(),
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
        self.annotate_issues(models);
    }

    pub(crate) fn annotate_states(&self, models: &mut Value, owner: Option<NodeId>) {
        let index = LineIndex::new(&self.source);
        models["sources"] = json!(self.states.iter().filter_map(|((id, _), group)| {
            if owner.is_some_and(|owner| owner != *id) { return None; }
            group.source_ranges(*id, &index)
        }).collect::<Vec<_>>());
    }

    pub(crate) fn state_route_range(&self, args: &Value) -> Option<ferroui_designer_language::Range> {
        let group = self.states.get(&(args["owner"].as_u64()?, args["group"].as_str()?.to_owned()))?;
        let to = args["to"].as_str()?;
        let span = group.transition_span(args["from"].as_str(), to).or_else(|| group.transition_span(None, to))?;
        LineIndex::new(&self.source).range(span).ok()
    }

    pub(crate) fn state_patches(&self, args: &Value) -> Result<Vec<TextPatch>, String> {
        let model: VisualStateGroup = serde_json::from_value(args["model"].clone())
            .map_err(|error| error.to_string())?;
        self.states.get(&(model.owner, model.name.clone()))
            .ok_or("Choose an existing mapped Rust state group")?
            .patch_model(&model)
    }

    pub(crate) fn annotate_issues(&self, catalog: &mut Value) {
        if self.issues.is_empty() {
            return;
        }
        let index = LineIndex::new(&self.source);
        catalog["sourceIssues"] = Value::Array(
            self.issues
                .iter()
                .filter_map(|issue| {
                    let range = index.range(issue.declaration).ok()?;
                    Some(json!({ "range": range, "reason": issue.reason }))
                })
                .collect(),
        );
    }

    pub(crate) fn patches(&self, args: &Value) -> Result<Vec<TextPatch>, String> {
        let clip = self.clip(args)?;
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

    pub(crate) fn move_keys(
        &self, session: &ferroui_designer::DesignerSession, args: &Value,
    ) -> Result<Vec<TextPatch>, String> {
        let clip = self.clip(args)?;
        let program = session.animation_programs().iter()
            .find(|program| Some(program.id) == args["program"].as_u64())
            .and_then(|program| program.timeline.as_ref())
            .ok_or("Choose a typed native timeline")?;
        let keys: Vec<KeyframeLocation> = serde_json::from_value(args["keyframes"].clone())
            .map_err(|error| error.to_string())?;
        let offset = args["offset"].as_f64().ok_or("Supply a keyframe offset")?;
        let (_, moved) = program.with_keyframe_edit(&keys, KeyframeGroupEdit::Move(offset))
            .map_err(|error| error.to_string())?;
        clip.patch_key_times(keys.iter().zip(moved).map(|(from, to)| (from.property.as_str(), from.time, to.time)))
    }

    fn clip(&self, args: &Value) -> Result<&RsxAnimation, String> {
        args["program"].as_u64().and_then(|id| self.clips.get(&id))
            .ok_or_else(|| "Choose an animation with a mapped Rust declaration".into())
    }

    pub(crate) fn map_hosted_program(&self, program: &mut DesignAnimationProgram) {
        let clip = u64::try_from(program.id)
            .ok()
            .and_then(|id| self.clips.get(&id));
        program.source_utf16 = clip
            .and_then(|clip| {
                i32::try_from(self.source[..clip.declaration.start].encode_utf16().count()).ok()
            })
            .unwrap_or(-1);
        if let Some(authoring) = &mut program.authoring {
            authoring.tracks.retain_mut(|track| {
                track.keys.retain(|key| {
                    let kind = serde_json::from_value(Value::String(key.value_kind.clone())).ok();
                    kind.zip(clip)
                        .is_some_and(|(kind, clip)| clip.maps_key(&track.property, key.time, kind))
                });
                !track.keys.is_empty()
            });
            if authoring.tracks.is_empty() {
                program.authoring = None;
            }
        }
    }

    pub(crate) fn hosted_patch(
        &self,
        draft: &crate::hosted_animation::KeyframeDraft,
    ) -> Result<Option<DesignAnimationSourcePatch>, String> {
        let clip = u64::try_from(draft.program)
            .ok()
            .and_then(|id| self.clips.get(&id))
            .ok_or("Choose an animation with a mapped Rust declaration")?;
        let patches = clip.patch_keyframe(
            &draft.property,
            KeyframeEdit::Upsert {
                previous: draft.previous,
                time: draft.time,
                value: &draft.value,
                easing: &draft.easing,
            },
        )?;
        let edited = TextPatch::apply_all(&self.source, patches).map_err(|error| error.to_string())?;
        TextPatch::between(&self.source, &edited)
            .map(|patch| {
                DesignAnimationSourcePatch::from_utf8(
                    &self.source,
                    patch.span.range(),
                    patch.replacement,
                )
            })
            .transpose()
    }
}
