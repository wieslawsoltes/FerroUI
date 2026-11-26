use super::{FrameIdentity, SourceMap, SourceMaps};
use crate::hosted_animation::{StateDraft, StateGroupDraft, source_patch};
use crate::protocol::MAX_SOURCE_BYTES;
use crate::rsx_animation::AuthoredRsxFrame;
use ferroui_designer_animation::{Easing, StateGroupEdit, VisualStateGroup};
use ferroui_designer_core::{Document, TextPatch};
use ferroui_remote_protocol::designer::DesignAnimationSourcePatch;
use serde_json::{Value, json};

impl SourceMaps {
    pub(crate) fn plan_state_route(
        &self,
        draft: &StateDraft,
        rust: Option<&AuthoredRsxFrame>,
    ) -> Result<Value, String> {
        let source = self.state_source(draft.revision, draft.sequence)?;
        let route = source
            .states
            .routes
            .get(&draft.program)
            .filter(|route| !route.endpoint)
            .ok_or("Choose an authored state transition in the current application preview")?;
        let mut model = source
            .states
            .groups
            .iter()
            .find(|group| group.model.owner == route.owner && group.model.name == route.group)
            .ok_or("The state group changed")?
            .model
            .clone();
        let transition = model
            .transitions
            .iter_mut()
            .find(|candidate| candidate.from == route.from && candidate.to == route.to)
            .ok_or("The state transition changed")?;
        transition.duration = draft.duration;
        transition.easing = Easing::parse(&draft.easing).map_err(|error| error.to_string())?;
        if let Some(rust) = rust {
            return source_plan(rust.source_text(), rust.patch_hosted_state_values(&model)?);
        }
        source.plan_state_group_edit(&StateGroupEdit::Upsert { model }, None)
    }

    pub(crate) fn plan_state_group(
        &self,
        draft: &StateGroupDraft,
        rust: Option<&AuthoredRsxFrame>,
    ) -> Result<Value, String> {
        self.state_source(draft.revision, draft.sequence)?
            .plan_state_group_edit(&draft.command, rust)
    }

    fn state_source(&self, revision: i64, sequence: i64) -> Result<&SourceMap, String> {
        let frame = FrameIdentity { revision, sequence };
        self.current
            .as_ref()
            .filter(|source| source.contains(&frame))
            .ok_or_else(|| "Read the current app-hosted preview frame".into())
    }
}

impl SourceMap {
    fn plan_state_group_edit(
        &self,
        edit: &StateGroupEdit,
        rust: Option<&AuthoredRsxFrame>,
    ) -> Result<Value, String> {
        let (owner, name) = edit.identity();
        let owners = self
            .states
            .owners
            .as_ref()
            .ok_or("Use an application preview with state authoring support")?;
        let live = owners
            .iter()
            .find(|entry| entry.id == owner)
            .ok_or("Choose an authored state owner in the accepted preview")?;
        let matches = self
            .states
            .groups
            .iter()
            .any(|group| group.model.owner == owner && group.model.name == name);
        if !matches && (!live.can_create || matches!(edit, StateGroupEdit::Remove { .. })) {
            return Err("Choose an editable state group in the accepted preview".into());
        }
        if let Some(rust) = rust {
            return source_plan(rust.source_text(), rust.plan_hosted_state_group(edit)?);
        }
        let mut staged = self.document.clone();
        let mut patches = xaml_state_group(&mut staged, edit)?;
        let text = self.document.source();
        if text
            .split_once('\n')
            .is_some_and(|(line, _)| line.ends_with('\r'))
        {
            for patch in &mut patches {
                patch.replacement = patch
                    .replacement
                    .replace("\r\n", "\n")
                    .replace('\n', "\r\n");
            }
        }
        source_plan(text, patches)
    }
}

fn xaml_state_group(
    document: &mut Document,
    edit: &StateGroupEdit,
) -> Result<Vec<TextPatch>, String> {
    let (owner, name) = edit.identity();
    let count = VisualStateGroup::read(document)
        .iter()
        .filter(|group| group.owner == owner && group.name == name)
        .count();
    if count > 1 {
        return Err("Choose a uniquely named state group on this owner".into());
    }
    let edits = match edit {
        StateGroupEdit::Upsert { model } => model.plan(document),
        StateGroupEdit::Remove { .. } => VisualStateGroup::remove(document, owner, name),
    }
    .map_err(|error| error.to_string())?;
    Ok(document
        .apply("Edit application visual states", edits)
        .map_err(|error| error.to_string())?
        .patches)
}

fn source_plan(source: &str, patches: Vec<TextPatch>) -> Result<Value, String> {
    let candidate =
        TextPatch::apply_all(source, patches.clone()).map_err(|error| error.to_string())?;
    if candidate.len() > MAX_SOURCE_BYTES {
        return Err("Designer source exceeds 1 MiB".into());
    }
    let patches = patches
        .into_iter()
        .map(|patch| {
            DesignAnimationSourcePatch::from_utf8(source, patch.span.range(), patch.replacement)
                .map(source_patch)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "patches": patches }))
}
