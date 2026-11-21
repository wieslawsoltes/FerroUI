//! Source locations and checked edits for behaviors attached to one control.

use super::{
    native_behavior_namespace, parse_action, BehaviorAction, BehaviorModel, BehaviorTrigger,
};
use crate::programs::{core_error, native_fragment, validate_fragment};
use crate::{require, AnimationError, Result};
use ferroui_designer_core::{Document, Edit, Node, NodeId, NodeKind};
use std::collections::BTreeMap;

pub struct BehaviorActionSource {
    pub node: NodeId,
    pub value: BehaviorAction,
}

pub struct BehaviorSource {
    pub node: NodeId,
    pub model: BehaviorModel,
    pub actions: Vec<BehaviorActionSource>,
    action_parent: Option<NodeId>,
}

pub enum BehaviorEdit {
    SetTrigger(BehaviorTrigger),
    AppendAction(BehaviorAction),
    SetAction { node: NodeId, value: BehaviorAction },
    MoveAction { node: NodeId, index: usize },
    RemoveAction(NodeId),
}

impl BehaviorSource {
    pub fn read(document: &Document, owner: NodeId) -> Result<Vec<Self>> {
        BehaviorModel::read(document, owner)?
            .into_iter()
            .map(|model| Self::describe(document, model))
            .collect()
    }

    pub fn owner(document: &Document, node: NodeId) -> Result<NodeId> {
        let parent = |id| document.ast().node(id).and_then(|node| node.parent);
        let container = parent(node)
            .and_then(|id| document.ast().node(id))
            .ok_or_else(|| AnimationError("Behavior has no collection".into()))?;
        let collection = if container.local_name() == "BehaviorCollection" {
            container.parent.and_then(|id| document.ast().node(id))
        } else {
            Some(container)
        };
        collection
            .filter(|node| node.local_name() == "Interaction.Behaviors")
            .and_then(|node| node.parent)
            .ok_or_else(|| AnimationError("Select a directly attached behavior".into()))
    }

    /// Bound collections have an application-owned lifetime and no local insertion point.
    pub fn can_attach(document: &Document, owner: NodeId) -> bool {
        document.ast().node(owner).is_some_and(|node| {
            node.is_element()
                && node.kind != NodeKind::PropertyElement
                && !node.attributes.iter().any(|attribute| {
                    attribute.name.rsplit(':').next() == Some("Interaction.Behaviors")
                })
        })
    }

    pub fn can_edit_actions(&self) -> bool {
        self.action_parent.is_some()
    }

    pub fn can_edit_trigger(&self, document: &Document) -> bool {
        let properties: &[&str] = match self.model.trigger {
            BehaviorTrigger::Event { .. } => &["EventName", "SourceObject"],
            BehaviorTrigger::Data { .. } => {
                &["Binding", "ComparisonCondition", "Value", "RevertOnFalse"]
            }
            BehaviorTrigger::Custom { .. } => return false,
        };
        document
            .ast()
            .element_children(self.node)
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .all(|node| {
                node.kind != NodeKind::PropertyElement
                    || !node
                        .local_name()
                        .rsplit('.')
                        .next()
                        .is_some_and(|name| properties.contains(&name))
            })
    }

    pub fn plan(document: &Document, behavior: NodeId, edit: BehaviorEdit) -> Result<Vec<Edit>> {
        let owner = Self::owner(document, behavior)?;
        let source = Self::read(document, owner)?
            .into_iter()
            .find(|source| source.node == behavior)
            .ok_or_else(|| AnimationError("Behavior changed".into()))?;
        match edit {
            BehaviorEdit::SetTrigger(next) => source.trigger_edits(document, next),
            BehaviorEdit::AppendAction(value) => source.append_action(document, value),
            BehaviorEdit::SetAction { node, value } => source.action_edits(document, node, value),
            BehaviorEdit::MoveAction { node, index } => source.move_action(document, node, index),
            BehaviorEdit::RemoveAction(node) => {
                source.action(node)?;
                Ok(vec![Edit::RemoveNode { node }])
            }
        }
    }

    pub fn replace_source(
        document: &Document,
        behavior: NodeId,
        source: &str,
    ) -> Result<Vec<Edit>> {
        let owner = Self::owner(document, behavior)?;
        let model = Self::read(document, owner)?
            .into_iter()
            .find(|entry| entry.node == behavior)
            .ok_or_else(|| AnimationError("Behavior changed".into()))?;
        let node = document.ast().node(model.node).expect("mapped behavior");
        validate_fragment(
            document,
            node.parent.expect("behavior container"),
            source,
            None,
        )?;
        let span = node.span;
        let original = &document.valid_source()[span.start..span.end];
        Ok(if original == source {
            Vec::new()
        } else {
            vec![Edit::Text {
                span,
                replacement: source.into(),
            }]
        })
    }

    fn describe(document: &Document, model: BehaviorModel) -> Result<Self> {
        let node = model.id.expect("authored behavior");
        let action_parent = if matches!(model.trigger, BehaviorTrigger::Custom { .. }) {
            None
        } else {
            action_container(document, node)
        };
        let actions = action_parent
            .map(|parent| {
                document
                    .ast()
                    .element_children(parent)
                    .into_iter()
                    .filter(|id| {
                        document
                            .ast()
                            .node(*id)
                            .is_some_and(|node| node.kind != NodeKind::PropertyElement)
                    })
                    .map(|node| {
                        Ok(BehaviorActionSource {
                            node,
                            value: parse_action(document, node)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            node,
            model,
            actions,
            action_parent,
        })
    }

    fn trigger_edits(&self, document: &Document, next: BehaviorTrigger) -> Result<Vec<Edit>> {
        let previous = &self.model.trigger;
        require(
            self.can_edit_trigger(document) && !matches!(next, BehaviorTrigger::Custom { .. }),
            "Edit this trigger in XAML source",
        )?;
        let (old_name, old) = previous.attributes()?;
        let (name, next) = next.attributes()?;
        require(name == old_name, "Change the trigger type in XAML source")?;
        Ok(property_edits(self.node, &old, &next))
    }

    fn append_action(&self, document: &Document, value: BehaviorAction) -> Result<Vec<Edit>> {
        require(self.actions.len() < 4096, "Too many behavior actions")?;
        let parent = self
            .action_parent
            .ok_or_else(|| AnimationError("Edit this action collection in XAML source".into()))?;
        let source = action_markup(document, parent, &value)?;
        Ok(vec![Edit::InsertChild {
            parent,
            index: document.ast().element_children(parent).len(),
            source,
        }])
    }

    fn action_edits(
        &self,
        document: &Document,
        id: NodeId,
        value: BehaviorAction,
    ) -> Result<Vec<Edit>> {
        let previous = self.action(id)?;
        require(
            !matches!(previous.value, BehaviorAction::Custom { .. })
                && !matches!(
                    value,
                    BehaviorAction::Custom { .. } | BehaviorAction::Markup { .. }
                ),
            "Edit custom actions in XAML source",
        )?;
        let node = document.ast().node(id).expect("mapped action");
        let xml = value.to_xaml()?;
        let parsed = Document::new(xml.as_str()).map_err(core_error)?;
        let next = parsed
            .ast()
            .node(parsed.ast().root())
            .expect("generated action");
        if next.local_name() == node.local_name() {
            return Ok(property_edits(id, &properties(node), &properties(next)));
        }
        let source = action_markup(document, node.parent.expect("action container"), &value)?;
        Ok(vec![Edit::Text {
            span: node.span,
            replacement: source,
        }])
    }

    fn move_action(&self, document: &Document, node: NodeId, index: usize) -> Result<Vec<Edit>> {
        self.action(node)?;
        let target = self
            .actions
            .get(index)
            .ok_or_else(|| AnimationError("Action position changed".into()))?;
        if target.node == node {
            return Ok(Vec::new());
        }
        let parent = self.action_parent.expect("mapped action parent");
        let index = document
            .ast()
            .element_children(parent)
            .iter()
            .position(|id| *id == target.node)
            .expect("mapped action position");
        Ok(vec![Edit::MoveNode {
            node,
            parent,
            index,
        }])
    }

    fn action(&self, node: NodeId) -> Result<&BehaviorActionSource> {
        self.actions
            .iter()
            .find(|item| item.node == node)
            .ok_or_else(|| {
                AnimationError("Action does not belong to this editable behavior".into())
            })
    }
}

fn action_container(document: &Document, id: NodeId) -> Option<NodeId> {
    let children = document.ast().element_children(id);
    if children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .any(|node| node.local_name() == "ActionCollectionTemplate")
    {
        return None;
    }
    let mut wrappers = children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|node| {
            node.local_name().ends_with(".Actions") || node.local_name() == "ActionCollection"
        });
    let Some(wrapper) = wrappers.next() else {
        // Leave collections supplied by the trigger's styles or templates alone.
        return (children.is_empty()
            || children
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .any(|node| node.kind != NodeKind::PropertyElement))
        .then_some(id);
    };
    if wrappers.next().is_some()
        || !native_behavior_namespace(wrapper.namespace_uri.as_deref())
        || wrapper
            .attributes
            .iter()
            .any(|attribute| !attribute.is_namespace())
        || children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .any(|node| node.id != wrapper.id && node.kind != NodeKind::PropertyElement)
    {
        return None;
    }
    action_container(document, wrapper.id)
}

fn action_markup(document: &Document, parent: NodeId, action: &BehaviorAction) -> Result<String> {
    require(
        !matches!(action, BehaviorAction::Markup { .. }),
        "Choose one behavior action",
    )?;
    let xml = native_fragment(document, parent, &action.to_xaml()?)?;
    validate_fragment(document, parent, &xml, None)?;
    Ok(xml)
}

fn property_edits(
    node: NodeId,
    old: &BTreeMap<String, String>,
    new: &BTreeMap<String, String>,
) -> Vec<Edit> {
    old.keys()
        .filter(|name| !new.contains_key(*name))
        .map(|name| Edit::RemoveAttribute {
            node,
            name: name.clone(),
        })
        .chain(
            new.iter()
                .filter(|(name, value)| old.get(*name) != Some(*value))
                .map(|(name, value)| Edit::SetAttribute {
                    node,
                    name: name.clone(),
                    value: value.clone(),
                }),
        )
        .collect()
}

fn properties(node: &Node) -> BTreeMap<String, String> {
    node.attributes
        .iter()
        .filter(|attribute| !attribute.is_namespace())
        .map(|attribute| (attribute.name.clone(), attribute.value.clone()))
        .collect()
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
