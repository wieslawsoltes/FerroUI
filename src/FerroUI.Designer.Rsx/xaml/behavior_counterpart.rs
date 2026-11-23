//! Literal behavior operands in the generated XAML and RSX source views.
use super::XamlTree;
use crate::{RsxBehavior, RsxProjection};
use ferroui_designer_animation::{
    BehaviorAction, BehaviorEdit, BehaviorModel, BehaviorSource, BehaviorTrigger,
};
use ferroui_designer_core::{Document, Edit, Node, NodeId, NodeKind, TextPatch};

pub(crate) const COLLECTION_EDIT: &str = "Use Behaviors to change behavior and action lists";

pub(super) fn read(
    document: &Document,
    owner: NodeId,
    collection: &Node,
) -> Result<Vec<BehaviorSource>, String> {
    // The generated counterpart uses a direct native collection. Preserve authored
    // wrappers and custom collection expressions in the XAML editor.
    if collection
        .attributes
        .iter()
        .any(|attribute| !attribute.is_namespace())
    {
        return Err(COLLECTION_EDIT.into());
    }
    let mut entries = BehaviorSource::read(document, owner).map_err(|error| error.to_string())?;
    let ids = collection
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|child| !trivia(child))
        .map(|child| child.id);
    if !ids.eq(entries.iter().map(|entry| entry.node)) {
        return Err(COLLECTION_EDIT.into());
    }
    for entry in &mut entries {
        if !entry.can_edit_trigger(document) || !entry.can_edit_actions() {
            return Err(COLLECTION_EDIT.into());
        }
        entry.model.actions.retain(|action| {
            !matches!(action, BehaviorAction::Markup { source }
            if source.starts_with("<!--") && source.ends_with("-->"))
        });
        if entry.model.actions.len() != entry.actions.len() || !xaml_bindings(&entry.model) {
            return Err("This behavior needs its authored XAML bindings or action markup".into());
        }
        crate::behaviors::normalize(&mut entry.model)?;
    }
    Ok(entries)
}

fn trivia(node: &Node) -> bool {
    node.kind == NodeKind::Comment
        || node.kind == NodeKind::Text
            && node
                .text
                .as_deref()
                .is_some_and(|text| text.trim().is_empty())
}

fn xaml_bindings(model: &BehaviorModel) -> bool {
    let named = |value: &Option<String>| {
        value.as_deref().is_none_or(|value| {
            value.starts_with("{Binding #") || value.starts_with("{ReflectionBinding #")
        })
    };
    let path =
        |value: &str| value.starts_with("{Binding ") || value.starts_with("{ReflectionBinding ");
    let trigger = match &model.trigger {
        BehaviorTrigger::Event { source, .. } => named(source),
        BehaviorTrigger::Data { binding, .. } => path(binding),
        BehaviorTrigger::Custom { .. } => false,
    };
    trigger
        && model.actions.iter().all(|item| match item {
            BehaviorAction::ChangeProperty { target, .. }
            | BehaviorAction::CallMethod { target, .. }
            | BehaviorAction::AddClass { target, .. }
            | BehaviorAction::RemoveClass { target, .. } => named(target),
            BehaviorAction::InvokeCommand { command, .. } => path(command),
            _ => false,
        })
}

impl XamlTree {
    pub(crate) fn behavior_models(&self) -> Vec<Vec<BehaviorModel>> {
        self.origins
            .iter()
            .map(|control| {
                control
                    .behaviors
                    .iter()
                    .flatten()
                    .map(|entry| entry.model.clone())
                    .collect()
            })
            .collect()
    }

    pub(super) fn plan_behaviors(
        &self,
        document: &Document,
        draft: &[Vec<BehaviorModel>],
    ) -> Result<Vec<TextPatch>, String> {
        let mut attributes = Vec::new();
        let mut patches = Vec::new();
        let mut plan = |node, edit| -> Result<(), String> {
            for edit in
                BehaviorSource::plan(document, node, edit).map_err(|error| error.to_string())?
            {
                match edit {
                    Edit::SetAttribute { .. } | Edit::RemoveAttribute { .. } => {
                        attributes.push(edit)
                    }
                    Edit::Text { span, replacement } => {
                        patches.push(TextPatch { span, replacement })
                    }
                    _ => return Err(COLLECTION_EDIT.into()),
                }
            }
            Ok(())
        };
        for (control, next) in self.origins.iter().zip(draft) {
            let current = control.behaviors.as_deref().unwrap_or_default();
            if current.len() != next.len() {
                return Err(COLLECTION_EDIT.into());
            }
            for (entry, model) in current.iter().zip(next) {
                if entry.actions.len() != model.actions.len() {
                    return Err(COLLECTION_EDIT.into());
                }
                if entry.model.trigger != model.trigger {
                    plan(entry.node, BehaviorEdit::SetTrigger(model.trigger.clone()))?;
                }
                for ((action, previous), value) in entry
                    .actions
                    .iter()
                    .zip(&entry.model.actions)
                    .zip(&model.actions)
                {
                    if previous != value {
                        plan(
                            entry.node,
                            BehaviorEdit::SetAction {
                                node: action.node,
                                value: value.clone(),
                            },
                        )?;
                    }
                }
            }
        }
        patches.extend(
            document
                .plan_attribute_patches(attributes)
                .map_err(|error| error.to_string())?,
        );
        Ok(patches)
    }
}

impl RsxProjection<'_> {
    pub(crate) fn matches_behavior_models(&self, models: &[Vec<BehaviorModel>]) -> bool {
        self.nodes.len() == models.len()
            && models.iter().enumerate().all(|(owner, next)| {
                self.behaviors(owner)
                    .iter()
                    .map(RsxBehavior::model)
                    .eq(next)
            })
    }

    pub(crate) fn patch_behavior_models(
        &self,
        models: &[Vec<BehaviorModel>],
    ) -> Result<Vec<TextPatch>, String> {
        let mut patches = Vec::new();
        for (owner, next) in models.iter().enumerate() {
            let current = self.behaviors(owner);
            if current.len() != next.len() {
                return Err(COLLECTION_EDIT.into());
            }
            for (entry, model) in current.iter().zip(next) {
                if entry.model().actions.len() != model.actions.len() {
                    return Err(COLLECTION_EDIT.into());
                }
                if entry.model() != model {
                    patches.extend(entry.patch_model(self.source, model)?);
                }
            }
        }
        Ok(patches)
    }
}
