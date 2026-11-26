use super::{FrameIdentity, SourceMap, SourceMaps};
use crate::hosted_animation::{ClipEditDraft, source_patch};
use crate::protocol::MAX_SOURCE_BYTES;
use crate::rsx_animation::AuthoredRsxFrame;
use ferroui_designer::validate_rsx_animation_selector;
use ferroui_designer_animation::{AnimationClipEdit, AnimationPrograms};
use ferroui_designer_core::NodeId;
use ferroui_designer_language::LineIndex;
use ferroui_remote_protocol::designer::{
    DesignAnimationCatalogMessage, DesignAnimationSourcePatch,
};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
pub(super) struct ClipAuthoring {
    owners: Vec<ClipOwner>,
    programs: Vec<NodeId>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClipOwner {
    id: NodeId,
    label: String,
    type_name: String,
    selector: String,
    source_line: u32,
}

impl SourceMap {
    pub(crate) fn capture_clip_authoring(
        &mut self,
        catalog: &DesignAnimationCatalogMessage,
        rust: Option<&AuthoredRsxFrame>,
    ) {
        if catalog.clip_authoring != Some(true) {
            return;
        }
        let index = LineIndex::new(self.document.source());
        let owners = catalog
            .targets
            .iter()
            .filter_map(|target| {
                let id = NodeId::try_from(target.id).ok()?;
                let node = self.document.ast().node(id)?;
                let source_line = match rust {
                    Some(rust) => rust.hosted_clip_owner_line(id)?,
                    None => index.position(node.name_span.start).ok()?.line + 1,
                };
                let local_type = node.local_name();
                let selector = node.xaml_name().map_or_else(
                    || local_type.to_owned(),
                    |name| format!("{local_type}#{name}"),
                );
                Some(ClipOwner {
                    id,
                    selector,
                    label: target.label.clone(),
                    type_name: target.type_name.clone(),
                    source_line,
                })
            })
            .collect();
        let programs = self
            .programs
            .iter()
            .map(|program| program.id)
            .filter(|&id| {
                i64::try_from(id).is_ok_and(|id| {
                    self.clips.contains_key(&id) && !self.states.routes.contains_key(&id)
                }) && rust.is_none_or(|rust| rust.hosted_editable_clip(id))
            })
            .collect();
        self.clip_authoring = Some(ClipAuthoring { owners, programs });
    }

    pub(crate) fn annotate_clip_authoring(&self, catalog: &mut Value) {
        if let Some(authoring) = &self.clip_authoring {
            catalog["clipAuthoring"] = json!(authoring);
        }
    }

    pub(crate) fn clip_with_source_key(&self, key: &str) -> Option<i64> {
        let mut matches = self
            .programs
            .iter()
            .filter(|program| program.source_key.as_deref() == Some(key))
            .filter_map(|program| i64::try_from(program.id).ok())
            .filter(|id| self.clips.contains_key(id));
        matches.next().filter(|_| matches.next().is_none())
    }
}

impl SourceMaps {
    pub(crate) fn plan_clip_edit(
        &self,
        draft: &ClipEditDraft,
        rust: Option<&AuthoredRsxFrame>,
    ) -> Result<Value, String> {
        let frame = FrameIdentity {
            revision: draft.revision,
            sequence: draft.sequence,
        };
        let source = self
            .current
            .as_ref()
            .filter(|source| source.contains(&frame))
            .ok_or("Read the current app-hosted preview frame")?;
        let authoring = source
            .clip_authoring
            .as_ref()
            .ok_or("Use an application preview with clip editing support")?;
        let command = &draft.command;
        if command
            .program()
            .is_some_and(|id| !authoring.programs.contains(&id))
        {
            return Err("Choose an editable source clip in the accepted preview".into());
        }
        if let AnimationClipEdit::Create {
            owner, selector, ..
        } = command
        {
            if !authoring
                .owners
                .iter()
                .any(|candidate| candidate.id == *owner)
            {
                return Err("Choose a live authored style owner in the accepted preview".into());
            }
            validate_rsx_animation_selector(selector)?;
        }
        let (text, plan) = match rust {
            Some(rust) => (rust.source_text(), rust.plan_hosted_clip_edit(command)?),
            None => (
                source.document.source(),
                AnimationPrograms::plan_clip_edit(&source.document, command)
                    .map_err(|error| error.to_string())?,
            ),
        };
        let removed_bytes: usize = plan
            .patches
            .iter()
            .map(|patch| patch.span.range().len())
            .sum();
        let inserted_bytes: usize = plan
            .patches
            .iter()
            .map(|patch| patch.replacement.len())
            .sum();
        if text.len() - removed_bytes + inserted_bytes > MAX_SOURCE_BYTES {
            return Err("The edited animation source exceeds the 1 MiB preview limit".into());
        }
        let patches = plan
            .patches
            .into_iter()
            .map(|patch| {
                DesignAnimationSourcePatch::from_utf8(text, patch.span.range(), patch.replacement)
                    .map(source_patch)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({ "patches": patches, "selectedKey": plan.selected_key }))
    }
}
