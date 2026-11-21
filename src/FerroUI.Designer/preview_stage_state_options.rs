use crate::{PreviewSourceAnchor, PreviewStageSource, PreviewStageTarget};
use ferroui_designer_animation::VisualStateGroup;
use ferroui_designer_core::Document;
use serde::Serialize;

/// State choices for one authored owner in a stage's current definition or document.
#[derive(Clone, Debug, Serialize)]
pub struct PreviewStageStateOptions {
    pub owner: PreviewSourceAnchor,
    pub owner_name: String,
    pub group: String,
    pub states: Vec<String>,
}

pub(crate) fn source_options(
    source: &PreviewStageSource,
    target: &PreviewStageTarget,
) -> Result<Vec<PreviewStageStateOptions>, String> {
    if source.pending || !source.document.is_valid() {
        return Err("Repair the current stage source before choosing its authored states".into());
    }
    Ok(options(&source.document, target))
}

pub(crate) fn options(
    document: &Document,
    target: &PreviewStageTarget,
) -> Vec<PreviewStageStateOptions> {
    let scope = match target {
        PreviewStageTarget::Document => None,
        PreviewStageTarget::Definition { anchor, .. } => match anchor.resolve(document) {
            Ok(node) => Some(node),
            Err(_) => return Vec::new(),
        },
    };
    VisualStateGroup::read(document)
        .into_iter()
        .filter_map(|group| {
            if scope.is_some_and(|scope| {
                !document
                    .ast()
                    .subtree(scope)
                    .any(|node| node.id == group.owner)
            }) {
                return None;
            }
            let owner = PreviewSourceAnchor::capture(document, group.owner).ok()?;
            let node = document.ast().node(group.owner)?;
            Some(PreviewStageStateOptions {
                owner,
                owner_name: node.xaml_name().unwrap_or_else(|| node.local_name()).into(),
                group: group.name,
                states: group.states.into_iter().map(|state| state.name).collect(),
            })
        })
        .collect()
}
