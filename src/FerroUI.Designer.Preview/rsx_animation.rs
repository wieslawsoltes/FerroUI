use crate::document::DesignDocument;
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::{NodeId, SourceSpan, TextPatch};
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::{RsxAnimation, RsxAnimationIssue, RsxCatalog, RsxStateGroup};
use ferroui_remote_protocol::designer::{DesignAnimationProgram, DesignAnimationSourcePatch};
use serde_json::{json, Value};
use std::collections::BTreeMap;

mod states;
pub(crate) use states::{state_edit, state_patches};
mod hosted_states;
pub(crate) use hosted_states::HostedStateRoute;

mod keyframes;
pub(crate) use keyframes::keyframe_edit;
mod programs;
pub(crate) use programs::{clip_placement_edit, clip_placement_patches, program_edit, program_patches};

pub(crate) struct AuthoredRsxFrame {
    source: Box<str>,
    invocation: SourceSpan,
    clips: BTreeMap<NodeId, RsxAnimation>,
    catalog: RsxCatalog,
    states: BTreeMap<(NodeId, String), RsxStateGroup>,
    state_owners: Vec<NodeId>,
    clip_owners: BTreeMap<NodeId, usize>,
    issues: Vec<RsxAnimationIssue>,
    pub(crate) hosted_routes: BTreeMap<i64, HostedStateRoute>,
    hosted_groups: BTreeMap<(NodeId, String), ferroui_designer_animation::VisualStateGroup>,
}

impl AuthoredRsxFrame {
    pub(crate) fn capture(design: &DesignDocument<'_>) -> Option<Self> {
        let rsx = design.rsx.as_ref()?;
        Some(Self {
            source: design.source.into(),
            invocation: rsx.invocation,
            clips: design.rsx_animations.clone(),
            catalog: design.rsx_catalog().clone(),
            states: design.rsx_states.clone(),
            clip_owners: design.xaml.ast().nodes_in_source_order().filter_map(|node| {
                let (index, _) = design.rsx_node(node.id)?;
                rsx.can_create_animation(index).then_some((node.id, index))
            }).collect(),
            state_owners: design.xaml.ast().nodes_in_source_order().filter_map(|node| {
                let (owner, _) = design.rsx_node(node.id)?;
                rsx.can_create_state_group(owner).then_some(node.id)
            }).collect(),
            issues: rsx.animation_issues.clone(),
            hosted_routes: BTreeMap::new(),
            hosted_groups: BTreeMap::new(),
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
                    if program["clip"].is_object() {
                        program["clip"]["editable"] = json!(false);
                    }
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
        models["creatableOwners"] = json!(self.state_owners.iter()
            .filter(|id| owner.is_none_or(|owner| owner == **id)).collect::<Vec<_>>());
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

    pub(crate) fn map_hosted_program(&self, program: &mut DesignAnimationProgram) {
        let clip = u64::try_from(program.id)
            .ok()
            .and_then(|id| self.clips.get(&id));
        if let Some(timing) = &mut program.clip {
            timing.editable &= clip.is_some();
        }
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
        crate::hosted_animation::checked_source_patch(&self.source, &edited)
    }
}
