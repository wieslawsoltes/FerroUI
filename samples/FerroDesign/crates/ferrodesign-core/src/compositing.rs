//! Atomic layer compositing and sibling-mask authoring.
//!
//! Figma's `isMask` role applies to all subsequent siblings within a parent.
//! Grouping contains that scope while preserving independent source/content
//! transforms. https://developers.figma.com/docs/plugins/api/properties/nodes-ismask/
use crate::{BlendMode, DocumentError, Editor, Node, NodeId, NodeKind, Transform};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskType {
    #[default]
    Alpha,
    Vector,
    Luminance,
}

impl MaskType {
    pub fn is_alpha(&self) -> bool {
        *self == Self::Alpha
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositingEdit {
    MaskEnabled(bool),
    MaskType(MaskType),
    BlendMode(BlendMode),
}

impl CompositingEdit {
    fn label(self) -> &'static str {
        match self {
            Self::MaskEnabled(true) => "Use as mask",
            Self::MaskEnabled(false) => "Remove mask",
            Self::MaskType(_) => "Change mask type",
            Self::BlendMode(_) => "Change layer blend",
        }
    }

    fn apply(self, node: &mut Node) {
        match self {
            Self::MaskEnabled(value) => node.is_mask = value,
            Self::MaskType(value) => node.mask_type = value,
            Self::BlendMode(value) => node.blend_mode = value,
        }
    }

    fn property(self) -> &'static str {
        match self {
            Self::MaskEnabled(_) => "is_mask",
            Self::MaskType(_) => "mask_type",
            Self::BlendMode(_) => "blend_mode",
        }
    }
}

fn invalid(message: &str) -> DocumentError {
    DocumentError::Invalid(message.into())
}

impl Editor {
    /// Prepare all changes before capturing history. Locked layers are skipped;
    /// a stale ID rejects the entire batch. A no-op keeps redo and revision.
    pub fn edit_compositing(
        &mut self,
        ids: &[NodeId],
        edit: CompositingEdit,
    ) -> Result<usize, DocumentError> {
        let mut prepared = Vec::new();
        for id in ids.iter().copied().collect::<BTreeSet<_>>() {
            let node = self
                .document
                .nodes
                .get(&id)
                .ok_or(DocumentError::MissingNode(id))?;
            if self.document.effective_locked(id) {
                continue;
            }
            let changed = match edit {
                CompositingEdit::MaskEnabled(value) => node.is_mask != value,
                CompositingEdit::MaskType(value) => node.mask_type != value,
                CompositingEdit::BlendMode(value) => node.blend_mode != value,
            };
            if changed {
                prepared.push(id);
            }
        }
        let count = prepared.len();
        if count == 0 {
            return Ok(0);
        }
        self.update_nodes(&prepared, edit.label(), |node| {
            edit.apply(node);
            node.overridden_properties.insert(edit.property().into());
        });
        Ok(count)
    }

    /// Wrap selected siblings and use the bottommost selected layer as their
    /// alpha mask. The new group is pass-through; existing mask roles on other
    /// selected layers remain explicit and preserve their relative ordering.
    /// All geometry and validation happens on a candidate before the single
    /// history entry, so an invalid grouping never consumes IDs or redo.
    pub fn create_mask_group(&mut self) -> Result<NodeId, DocumentError> {
        let selected: BTreeSet<_> = self.selection.iter().copied().collect();
        for id in &selected {
            if !self.document.nodes.contains_key(id) {
                return Err(DocumentError::MissingNode(*id));
            }
            if self.document.effective_locked(*id) {
                return Err(invalid(
                    "Unlock selected layers before creating a mask group.",
                ));
            }
        }
        let roots: BTreeSet<_> = selected
            .iter()
            .copied()
            .filter(|id| {
                !selected
                    .iter()
                    .any(|ancestor| ancestor != id && self.document.is_ancestor(*ancestor, *id))
            })
            .collect();
        if roots.len() < 2 {
            return Err(invalid(
                "Select a mask source and at least one content layer.",
            ));
        }
        let parent = self.document.nodes[roots.first().unwrap()]
            .parent
            .ok_or_else(|| invalid("Selected layers must belong to a parent."))?;
        if roots
            .iter()
            .any(|id| self.document.nodes[id].parent != Some(parent))
        {
            return Err(invalid(
                "Select layers with the same parent to create a mask group.",
            ));
        }
        let siblings = self
            .document
            .children(parent)
            .ok_or(DocumentError::InvalidParent(parent))?;
        let ordered: Vec<_> = siblings
            .iter()
            .copied()
            .filter(|id| roots.contains(id))
            .collect();
        if ordered.len() != roots.len() {
            return Err(invalid("The selected layer order is inconsistent."));
        }
        let insertion = siblings.iter().position(|id| *id == ordered[0]).unwrap();
        let parent_world = self.document.world_transform(parent).unwrap_or_default();
        let parent_inverse = parent_world
            .inverse()
            .ok_or_else(|| invalid("The parent transform cannot be inverted."))?;
        let transforms: Vec<_> = ordered
            .iter()
            .map(|id| {
                self.document
                    .world_transform(*id)
                    .map(|world| (*id, world))
                    .ok_or_else(|| invalid("A selected layer has an invalid transform."))
            })
            .collect::<Result<_, _>>()?;
        let bounds = transforms
            .iter()
            .map(|(id, world)| {
                (parent_inverse * *world).transform_rect(self.document.nodes[id].local_bounds())
            })
            .reduce(|a, b| a.union(b))
            .unwrap();
        if !bounds.is_finite() {
            return Err(invalid("The selected layer bounds are invalid."));
        }
        let mut candidate = self.document.clone();
        let mut group = Node::new(
            NodeKind::Group,
            "Mask group",
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
        );
        group.fills.clear();
        group.blend_mode = BlendMode::PassThrough;
        let group = candidate.insert_node(parent, group)?;
        let children = candidate.children_mut(parent).unwrap();
        children.retain(|id| *id != group);
        children.insert(insertion, group);
        let group_inverse = (parent_world * Transform::translation(bounds.x, bounds.y))
            .inverse()
            .ok_or_else(|| invalid("The mask group transform cannot be inverted."))?;
        for (id, world) in transforms {
            candidate.reparent(id, group, usize::MAX)?;
            candidate
                .nodes
                .get_mut(&id)
                .unwrap()
                .set_local_transform(group_inverse * world);
        }
        let source = candidate.nodes.get_mut(&ordered[0]).unwrap();
        source.is_mask = true;
        source.mask_type = MaskType::Alpha;
        source.overridden_properties.insert("is_mask".into());
        source.overridden_properties.insert("mask_type".into());
        candidate.validate()?;
        self.edit("Create mask group", |editor| {
            editor.document = candidate;
            editor.selection = vec![group];
        });
        Ok(group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Document, EditorRenderChanges};

    fn fixture() -> (Editor, NodeId, NodeId) {
        let mut document = Document::new("Masks");
        let source = document
            .insert_node(
                1,
                Node::new(NodeKind::Ellipse, "Source", 20.0, 30.0, 60.0, 50.0),
            )
            .unwrap();
        let target = document
            .insert_node(
                1,
                Node::new(NodeKind::Rectangle, "Content", 10.0, 20.0, 100.0, 100.0)
                    .with_fill(Color::BLACK),
            )
            .unwrap();
        let mut editor = Editor::new(document);
        editor.selection = vec![target, source];
        (editor, source, target)
    }

    #[test]
    fn compositing_new_groups_pass_through_but_legacy_omitted_blends_stay_normal() {
        let group = Node::new(NodeKind::Group, "New group", 0.0, 0.0, 40.0, 30.0);
        assert_eq!(group.blend_mode, BlendMode::PassThrough);
        let legacy: Node =
            serde_json::from_str(r#"{"kind":"group","name":"Existing group"}"#).unwrap();
        assert_eq!(legacy.blend_mode, BlendMode::Normal);
        assert_eq!(Node::default().blend_mode, BlendMode::Normal);
        let (mut editor, _, _) = fixture();
        let group = editor.group_selection().unwrap();
        assert_eq!(
            editor.document.nodes[&group].blend_mode,
            BlendMode::PassThrough
        );
        let encoded = editor.document.to_json().unwrap();
        assert_eq!(
            Document::from_json(&encoded).unwrap().nodes[&group].blend_mode,
            BlendMode::PassThrough
        );
    }

    #[test]
    fn compositing_legacy_defaults_and_all_modes_round_trip() {
        let (mut editor, source, _) = fixture();
        let legacy = editor.document.to_json().unwrap();
        assert!(!legacy.contains("is_mask"));
        assert!(!legacy.contains("mask_type"));
        assert_eq!(
            Document::from_json(&legacy).unwrap().to_json().unwrap(),
            legacy
        );
        editor
            .edit_compositing(&[source], CompositingEdit::MaskEnabled(true))
            .unwrap();
        for kind in [MaskType::Alpha, MaskType::Vector, MaskType::Luminance] {
            editor
                .edit_compositing(&[source], CompositingEdit::MaskType(kind))
                .unwrap();
            editor
                .edit_compositing(
                    &[source],
                    CompositingEdit::BlendMode(BlendMode::PassThrough),
                )
                .unwrap();
            let serialized = editor.document.to_json().unwrap();
            let decoded = Document::from_json(&serialized).unwrap();
            assert!(decoded.nodes[&source].is_mask);
            assert_eq!(decoded.nodes[&source].mask_type, kind);
            assert_eq!(decoded.nodes[&source].blend_mode, BlendMode::PassThrough);
            assert_eq!(decoded.to_json().unwrap(), serialized);
        }
    }

    #[test]
    fn compositing_edits_are_atomic_scoped_history_and_keep_disabled_mask_type() {
        let (mut editor, source, target) = fixture();
        let original = editor.document.to_json().unwrap();
        let selection = editor.selection.clone();
        let token = editor.render_change_token();
        assert_eq!(
            editor
                .edit_compositing(
                    &[source, target, source],
                    CompositingEdit::MaskEnabled(true)
                )
                .unwrap(),
            2
        );
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.selection, selection);
        assert!(
            matches!(editor.render_changes_since(&token), EditorRenderChanges::Nodes { nodes, .. } if nodes.contains(&source) && nodes.contains(&target))
        );
        let changed = editor.document.to_json().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), original);
        assert!(editor.redo());
        assert_eq!(editor.document.to_json().unwrap(), changed);
        editor
            .edit_compositing(&[source], CompositingEdit::MaskType(MaskType::Luminance))
            .unwrap();
        editor
            .edit_compositing(&[source], CompositingEdit::MaskEnabled(false))
            .unwrap();
        assert_eq!(
            editor.document.nodes[&source].mask_type,
            MaskType::Luminance
        );
    }

    #[test]
    fn compositing_stale_targets_noops_and_locked_layers_preserve_history() {
        let (mut editor, source, target) = fixture();
        editor
            .edit_compositing(&[source], CompositingEdit::MaskEnabled(true))
            .unwrap();
        editor.undo();
        let original = editor.document.to_json().unwrap();
        let token = editor.render_change_token();
        assert!(editor
            .edit_compositing(&[source, u64::MAX], CompositingEdit::MaskEnabled(true))
            .is_err());
        assert_eq!(
            editor
                .edit_compositing(&[source], CompositingEdit::MaskEnabled(false))
                .unwrap(),
            0
        );
        assert_eq!(editor.document.to_json().unwrap(), original);
        assert_eq!(editor.render_change_token(), token);
        assert!(!editor.can_undo());
        assert!(editor.can_redo());
        editor.document.nodes.get_mut(&target).unwrap().locked = true;
        assert_eq!(
            editor
                .edit_compositing(&[target], CompositingEdit::MaskEnabled(true))
                .unwrap(),
            0
        );
    }

    #[test]
    fn compositing_mask_group_preserves_order_world_transforms_and_one_step_undo() {
        let (mut editor, source, target) = fixture();
        editor.document.nodes.get_mut(&source).unwrap().rotation = 27.0;
        editor
            .document
            .nodes
            .get_mut(&target)
            .unwrap()
            .set_local_transform(Transform::translation(70.0, 10.0) * Transform::scale(-1.0, 0.8));
        editor.refresh();
        let before = editor.document.to_json().unwrap();
        let selection = editor.selection.clone();
        let world: Vec<_> = [source, target]
            .into_iter()
            .map(|id| (id, editor.document.world_transform(id).unwrap()))
            .collect();
        let group = editor.create_mask_group().unwrap();
        assert_eq!(editor.selection, vec![group]);
        assert_eq!(editor.document.nodes[&group].children, vec![source, target]);
        assert_eq!(
            editor.document.nodes[&group].blend_mode,
            BlendMode::PassThrough
        );
        assert!(editor.document.nodes[&source].is_mask);
        for (id, expected) in world {
            let actual = editor.document.world_transform(id).unwrap();
            for p in [crate::Point::default(), crate::Point::new(13.0, 29.0)] {
                let (a, b) = (actual.apply(p), expected.apply(p));
                assert!((a.x - b.x).abs() < 1e-8 && (a.y - b.y).abs() < 1e-8);
            }
        }
        assert_eq!(editor.history_len(), 1);
        let grouped = editor.document.to_json().unwrap();
        editor.undo();
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.selection, selection);
        editor.redo();
        assert_eq!(editor.document.to_json().unwrap(), grouped);
    }

    #[test]
    fn compositing_invalid_mask_group_does_not_allocate_or_discard_redo() {
        let (mut editor, source, target) = fixture();
        editor
            .edit_compositing(&[source], CompositingEdit::MaskEnabled(true))
            .unwrap();
        editor.undo();
        for selection in [vec![], vec![source], vec![source, u64::MAX]] {
            editor.selection = selection;
            let before = editor.document.to_json().unwrap();
            let revision = editor.revision;
            assert!(editor.create_mask_group().is_err());
            assert_eq!(editor.document.to_json().unwrap(), before);
            assert_eq!(editor.revision, revision);
            assert!(editor.can_redo());
        }
        editor.selection = vec![source, target];
        editor.document.nodes.get_mut(&target).unwrap().locked = true;
        let before = editor.document.to_json().unwrap();
        assert!(editor.create_mask_group().is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
    }
}
