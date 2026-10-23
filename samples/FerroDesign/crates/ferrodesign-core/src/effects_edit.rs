//! Validated effect-stack authoring, independent of any renderer or UI.
//!
//! Prepare every changed stack before opening the scoped node transaction. A
//! rejected edit cannot partially change a multi-selection, discard redo, or
//! detach a reusable style. Effects do not participate in auto-layout sizing,
//! so these commands use the editor's incremental node/ancestor invalidation.
use crate::{Color, DocumentError, Editor, Effect, EffectKind, NodeId, Point};
use std::collections::{BTreeMap, BTreeSet};

/// Authoring bounds match the existing effect-radius limit. Signed shadow
/// offsets and spread permit large canvases without accepting infinite bounds.
pub const MAX_EFFECT_RADIUS: f64 = 10_000.0;
pub const MAX_EFFECT_DISTANCE: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectPropertyEdit {
    Kind(EffectKind),
    Visible(bool),
    Radius(f64),
    Spread(f64),
    OffsetX(f64),
    OffsetY(f64),
    /// Set all RGBA channels, for an imported color or a color-picker result.
    Color(Color),
    /// Preserve each selected layer's existing alpha when changing RGB.
    Rgb { r: u8, g: u8, b: u8 },
    /// Normalized opacity, quantized once to the document's 8-bit alpha.
    Opacity(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum EffectStackEdit {
    Add(EffectKind),
    Insert { index: usize, effect: Effect },
    Remove(usize),
    Duplicate(usize),
    /// Move one entry, retaining the relative order of every other entry.
    Move { index: usize, to: usize },
    Property { index: usize, edit: EffectPropertyEdit },
    Replace(Vec<Effect>),
    Clear,
}

fn invalid(message: &str) -> DocumentError { DocumentError::Invalid(message.into()) }

fn validate_radius(value: f64) -> Result<(), DocumentError> {
    if value.is_finite() && (0.0..=MAX_EFFECT_RADIUS).contains(&value) { Ok(()) }
    else { Err(invalid("Blur radius must be a finite number from 0 to 10000.")) }
}

fn validate_distance(value: f64) -> Result<(), DocumentError> {
    if value.is_finite() && value.abs() <= MAX_EFFECT_DISTANCE { Ok(()) }
    else { Err(invalid("Shadow offsets and spread must be finite numbers from -1000000 to 1000000.")) }
}

impl Effect {
    /// Neutral, useful defaults for the four supported effect kinds. Changing
    /// an existing kind preserves its other authored values instead.
    pub fn for_kind(kind: EffectKind) -> Self {
        Self {
            kind,
            color: Color::rgba(0, 0, 0, 64),
            offset: if matches!(kind, EffectKind::DropShadow | EffectKind::InnerShadow) {
                Point::new(0.0, 4.0)
            } else { Point::new(0.0, 0.0) },
            radius: 4.0,
            spread: 0.0,
            visible: true,
        }
    }

    /// Validate latent shadow fields too, so changing an effect kind can never
    /// expose nonfinite coordinates imported while the entry was a blur.
    pub fn validate(&self) -> Result<(), DocumentError> {
        validate_radius(self.radius)?;
        validate_distance(self.spread)?;
        validate_distance(self.offset.x)?;
        validate_distance(self.offset.y)
    }
}

pub fn validate_effect_stack(effects: &[Effect]) -> Result<(), DocumentError> {
    for effect in effects { effect.validate()?; }
    Ok(())
}

impl EffectPropertyEdit {
    fn validate(self) -> Result<(), DocumentError> {
        match self {
            Self::Radius(value) => validate_radius(value),
            Self::Spread(value) | Self::OffsetX(value) | Self::OffsetY(value) => validate_distance(value),
            Self::Opacity(value) if !value.is_finite() || !(0.0..=1.0).contains(&value) => {
                Err(invalid("Effect opacity must be a finite value from 0 to 1."))
            }
            _ => Ok(()),
        }
    }

    fn apply(self, effect: &mut Effect) {
        match self {
            Self::Kind(value) => effect.kind = value,
            Self::Visible(value) => effect.visible = value,
            Self::Radius(value) => effect.radius = value,
            Self::Spread(value) => effect.spread = value,
            Self::OffsetX(value) => effect.offset.x = value,
            Self::OffsetY(value) => effect.offset.y = value,
            Self::Color(value) => effect.color = value,
            Self::Rgb { r, g, b } => effect.color = Color::rgba(r, g, b, effect.color.a),
            Self::Opacity(value) => effect.color.a = (value * 255.0).round() as u8,
        }
    }
}

impl EffectStackEdit {
    fn validate(&self) -> Result<(), DocumentError> {
        match self {
            Self::Insert { effect, .. } => effect.validate(),
            Self::Property { edit, .. } => edit.validate(),
            Self::Replace(effects) => validate_effect_stack(effects),
            _ => Ok(()),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Add(_) | Self::Insert { .. } => "Add effect",
            Self::Remove(_) => "Remove effect",
            Self::Duplicate(_) => "Duplicate effect",
            Self::Move { .. } => "Reorder effects",
            Self::Property { edit, .. } => match edit {
                EffectPropertyEdit::Kind(_) => "Change effect type",
                EffectPropertyEdit::Visible(_) => "Toggle effect visibility",
                EffectPropertyEdit::Radius(_) => "Change effect blur",
                EffectPropertyEdit::Spread(_) => "Change shadow spread",
                EffectPropertyEdit::OffsetX(_) | EffectPropertyEdit::OffsetY(_) => "Move shadow",
                EffectPropertyEdit::Color(_) | EffectPropertyEdit::Rgb { .. } => "Change effect color",
                EffectPropertyEdit::Opacity(_) => "Change effect opacity",
            },
            Self::Replace(_) => "Replace effects",
            Self::Clear => "Clear effects",
        }
    }

    fn apply(&self, effects: &mut Vec<Effect>) {
        match self {
            Self::Add(kind) => effects.push(Effect::for_kind(*kind)),
            Self::Insert { index, effect } if *index <= effects.len() => effects.insert(*index, effect.clone()),
            Self::Remove(index) if *index < effects.len() => { effects.remove(*index); }
            Self::Duplicate(index) if *index < effects.len() => effects.insert(index + 1, effects[*index].clone()),
            Self::Move { index, to } if *index < effects.len() && *to < effects.len() && index != to => {
                let effect = effects.remove(*index);
                effects.insert(*to, effect);
            }
            Self::Property { index, edit } => {
                if let Some(effect) = effects.get_mut(*index) { edit.apply(effect); }
            }
            Self::Replace(value) => *effects = value.clone(),
            Self::Clear => effects.clear(),
            _ => {},
        }
    }
}

impl Editor {
    /// Edit matching stack positions across selected layers. Missing rows and
    /// locked layers are untouched; a missing node reports a stale target.
    /// Duplicate IDs are applied once. The return value counts changed layers.
    ///
    /// The entire batch is validated before touching the live document. One
    /// successful call produces at most one history entry; a no-op retains
    /// redo, selection, bound styles, revision, and render-change history.
    pub fn edit_effects(&mut self, ids: &[NodeId], edit: EffectStackEdit) -> Result<usize, DocumentError> {
        edit.validate()?;
        let mut prepared = BTreeMap::new();
        for id in ids.iter().copied().collect::<BTreeSet<_>>() {
            let node = self.document.nodes.get(&id).ok_or(DocumentError::MissingNode(id))?;
            if self.document.effective_locked(id) { continue; }
            let mut effects = node.effects.clone();
            edit.apply(&mut effects);
            if effects == node.effects { continue; }
            validate_effect_stack(&effects)?;
            prepared.insert(id, effects);
        }
        let count = prepared.len();
        if count == 0 { return Ok(0); }
        let ids: Vec<_> = prepared.keys().copied().collect();
        self.update_nodes(&ids, edit.label(), |node| {
            node.effects = prepared.remove(&node.id).expect("prepared effect target");
            node.effect_style = None;
            node.overridden_properties.insert("effects".into());
        });
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AutoLayout, Document, EditorRenderChanges, Node, NodeKind};

    fn fixture() -> (Editor, NodeId, NodeId, NodeId) {
        let mut document = Document::new("Effects");
        let page = document.pages[0].id;
        let mut frame = Node::new(NodeKind::Frame, "Frame", 20.0, 30.0, 300.0, 100.0);
        frame.layout = Some(AutoLayout::default());
        let frame = document.insert_node(page, frame).unwrap();
        let mut layer = Node::new(NodeKind::Rectangle, "A", 0.0, 0.0, 40.0, 30.0);
        layer.effects.push(Effect::for_kind(EffectKind::DropShadow));
        let a = document.insert_node(frame, layer.clone()).unwrap();
        layer.name = "B".into();
        layer.effects[0].color.a = 200;
        let b = document.insert_node(frame, layer).unwrap();
        let mut editor = Editor::new(document);
        editor.selection = vec![a, b];
        (editor, frame, a, b)
    }

    fn property(index: usize, edit: EffectPropertyEdit) -> EffectStackEdit { EffectStackEdit::Property { index, edit } }

    fn stack(editor: &Editor, id: NodeId) -> &[Effect] { &editor.document.nodes[&id].effects }

    #[test]
    fn every_effect_kind_defaults_and_round_trips_without_new_schema_fields() {
        let (mut editor, _, a, _) = fixture();
        editor.edit_effects(&[a], EffectStackEdit::Clear).unwrap();
        let kinds = [EffectKind::DropShadow, EffectKind::InnerShadow, EffectKind::LayerBlur, EffectKind::BackgroundBlur];
        for kind in kinds {
            let effect = Effect::for_kind(kind);
            assert!(effect.validate().is_ok());
            editor.edit_effects(&[a], EffectStackEdit::Add(kind)).unwrap();
        }
        assert_eq!(stack(&editor, a).iter().map(|effect| effect.kind).collect::<Vec<_>>(), kinds);
        let serialized = editor.document.to_json().unwrap();
        assert_eq!(Document::from_json(&serialized).unwrap().to_json().unwrap(), serialized);
    }

    #[test]
    fn batch_edits_are_one_undo_step_and_preserve_selection_and_layout() {
        let (mut editor, frame, a, b) = fixture();
        let originals = [stack(&editor, a).to_vec(), stack(&editor, b).to_vec()];
        let bounds: Vec<_> = [frame, a, b].iter().map(|id| editor.document.nodes[id].bounds()).collect();
        let selection = editor.selection.clone();
        let token = editor.render_change_token();
        assert_eq!(editor.edit_effects(&[a, b, a], property(0, EffectPropertyEdit::Radius(23.125))).unwrap(), 2);
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.undo_label(), Some("Change effect blur"));
        assert_eq!(editor.selection, selection);
        assert_eq!(bounds, [frame, a, b].iter().map(|id| editor.document.nodes[id].bounds()).collect::<Vec<_>>());
        assert!(matches!(editor.render_changes_since(&token), EditorRenderChanges::Nodes { nodes, ancestors }
            if nodes.contains(&a) && nodes.contains(&b) && ancestors.contains(&frame)));
        assert!(editor.undo());
        assert_eq!(stack(&editor, a), originals[0]);
        assert_eq!(stack(&editor, b), originals[1]);
        assert_eq!(editor.selection, selection);
        assert!(!editor.can_undo());
        assert!(editor.redo());
        assert_eq!(stack(&editor, a)[0].radius, 23.125);
        assert_eq!(stack(&editor, b)[0].radius, 23.125);
    }

    #[test]
    fn invalid_numeric_values_leave_document_redo_and_revision_untouched() {
        let (mut editor, _, a, b) = fixture();
        editor.edit_effects(&[a], property(0, EffectPropertyEdit::Radius(20.0))).unwrap();
        editor.undo();
        let before = editor.document.to_json().unwrap();
        let revision = editor.revision;
        let selection = editor.selection.clone();
        let token = editor.render_change_token();
        for edit in [
            EffectPropertyEdit::Radius(-1.0), EffectPropertyEdit::Radius(MAX_EFFECT_RADIUS + 1.0),
            EffectPropertyEdit::Radius(f64::NAN), EffectPropertyEdit::Radius(f64::INFINITY),
            EffectPropertyEdit::Spread(f64::NEG_INFINITY), EffectPropertyEdit::Spread(MAX_EFFECT_DISTANCE + 1.0),
            EffectPropertyEdit::OffsetX(f64::NAN), EffectPropertyEdit::OffsetY(-MAX_EFFECT_DISTANCE - 1.0),
            EffectPropertyEdit::Opacity(-0.001), EffectPropertyEdit::Opacity(1.001), EffectPropertyEdit::Opacity(f64::NAN),
        ] {
            assert!(editor.edit_effects(&[a, b], property(0, edit)).is_err(), "{edit:?}");
            assert_eq!(editor.document.to_json().unwrap(), before);
            assert_eq!(editor.revision, revision);
            assert_eq!(editor.selection, selection);
            assert_eq!(editor.render_change_token(), token);
            assert!(!editor.can_undo());
            assert!(editor.can_redo());
        }
    }

    #[test]
    fn invalid_later_candidate_cannot_partially_commit_earlier_layer() {
        let (mut editor, _, a, b) = fixture();
        editor.document.nodes.get_mut(&b).unwrap().effects[0].spread = MAX_EFFECT_DISTANCE + 1.0;
        editor.refresh();
        let before = editor.document.to_json().unwrap();
        assert!(editor.edit_effects(&[a, b], property(0, EffectPropertyEdit::Radius(12.0))).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(!editor.can_undo());
        assert!(editor.edit_effects(&[a, u64::MAX], EffectStackEdit::Add(EffectKind::LayerBlur)).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(!editor.can_undo());
    }

    #[test]
    fn insertion_replacement_validate_complete_stack_before_mutation() {
        let (mut editor, _, a, b) = fixture();
        let mut invalid_effect = Effect::for_kind(EffectKind::BackgroundBlur);
        invalid_effect.offset.x = f64::NAN;
        assert!(editor.edit_effects(&[a, b], EffectStackEdit::Insert { index: 0, effect: invalid_effect.clone() }).is_err());
        assert!(editor.edit_effects(&[a, b], EffectStackEdit::Replace(vec![Effect::for_kind(EffectKind::InnerShadow), invalid_effect])).is_err());
        assert_eq!(stack(&editor, a).len(), 1);
        assert!(!editor.can_undo());
        assert_eq!(editor.edit_effects(&[a, b], EffectStackEdit::Insert { index: 1, effect: Effect::for_kind(EffectKind::LayerBlur) }).unwrap(), 2);
        assert_eq!(stack(&editor, a)[1].kind, EffectKind::LayerBlur);
        assert_eq!(editor.history_len(), 1);
    }

    #[test]
    fn rgb_preserves_individual_alpha_and_opacity_preserves_rgb() {
        let (mut editor, _, a, b) = fixture();
        editor.edit_effects(&[a, b], property(0, EffectPropertyEdit::Rgb { r: 12, g: 34, b: 56 })).unwrap();
        assert_eq!(stack(&editor, a)[0].color, Color::rgba(12, 34, 56, 64));
        assert_eq!(stack(&editor, b)[0].color, Color::rgba(12, 34, 56, 200));
        editor.edit_effects(&[a, b], property(0, EffectPropertyEdit::Opacity(0.5))).unwrap();
        for id in [a, b] { assert_eq!(stack(&editor, id)[0].color, Color::rgba(12, 34, 56, 128)); }
        for (opacity, alpha) in [(0.0, 0), (1.0, 255)] {
            editor.edit_effects(&[a], property(0, EffectPropertyEdit::Opacity(opacity))).unwrap();
            assert_eq!(stack(&editor, a)[0].color.a, alpha);
        }
    }

    #[test]
    fn changing_kind_preserves_shadow_settings_and_signed_spread() {
        let (mut editor, _, a, _) = fixture();
        for edit in [EffectPropertyEdit::Spread(-12.5), EffectPropertyEdit::OffsetX(-40.25),
            EffectPropertyEdit::OffsetY(17.125), EffectPropertyEdit::Visible(false),
            EffectPropertyEdit::Color(Color::rgba(11, 22, 33, 44))] {
            editor.edit_effects(&[a], property(0, edit)).unwrap();
        }
        let original = stack(&editor, a)[0].clone();
        for kind in [EffectKind::LayerBlur, EffectKind::BackgroundBlur, EffectKind::InnerShadow, EffectKind::DropShadow] {
            editor.edit_effects(&[a], property(0, EffectPropertyEdit::Kind(kind))).unwrap();
            let mut expected = original.clone();
            expected.kind = kind;
            assert_eq!(stack(&editor, a)[0], expected);
        }
        for value in [-MAX_EFFECT_DISTANCE, MAX_EFFECT_DISTANCE] {
            editor.edit_effects(&[a], property(0, EffectPropertyEdit::Spread(value))).unwrap();
            assert_eq!(stack(&editor, a)[0].spread, value);
        }
        for value in [0.0, MAX_EFFECT_RADIUS] {
            editor.edit_effects(&[a], property(0, EffectPropertyEdit::Radius(value))).unwrap();
            assert_eq!(stack(&editor, a)[0].radius, value);
        }
    }

    #[test]
    fn duplicate_and_move_preserve_stack_order_then_remove_and_clear() {
        let (mut editor, _, a, _) = fixture();
        let kinds = [EffectKind::DropShadow, EffectKind::InnerShadow, EffectKind::LayerBlur, EffectKind::BackgroundBlur];
        editor.edit_effects(&[a], EffectStackEdit::Replace(kinds.into_iter().map(Effect::for_kind).collect())).unwrap();
        editor.edit_effects(&[a], EffectStackEdit::Move { index: 0, to: 3 }).unwrap();
        assert_eq!(stack(&editor, a).iter().map(|effect| effect.kind).collect::<Vec<_>>(),
            [EffectKind::InnerShadow, EffectKind::LayerBlur, EffectKind::BackgroundBlur, EffectKind::DropShadow]);
        editor.edit_effects(&[a], EffectStackEdit::Duplicate(1)).unwrap();
        assert_eq!(stack(&editor, a)[1], stack(&editor, a)[2]);
        assert_eq!(stack(&editor, a).len(), 5);
        editor.edit_effects(&[a], property(2, EffectPropertyEdit::Radius(50.0))).unwrap();
        assert_ne!(stack(&editor, a)[1].radius, stack(&editor, a)[2].radius);
        editor.edit_effects(&[a], EffectStackEdit::Remove(2)).unwrap();
        assert_eq!(stack(&editor, a).len(), 4);
        editor.edit_effects(&[a], EffectStackEdit::Clear).unwrap();
        assert!(stack(&editor, a).is_empty());
        assert!(editor.undo());
        assert_eq!(stack(&editor, a).len(), 4);
    }

    #[test]
    fn only_effect_changes_detach_styles_and_mark_instance_overrides() {
        let (mut editor, _, a, b) = fixture();
        for id in [a, b] {
            let node = editor.document.nodes.get_mut(&id).unwrap();
            node.effect_style = Some(123);
            node.fill_style = Some(456);
        }
        editor.refresh();
        let before = editor.document.to_json().unwrap();
        let revision = editor.revision;
        for edit in [property(0, EffectPropertyEdit::Radius(4.0)), property(99, EffectPropertyEdit::Radius(1.0)),
            EffectStackEdit::Move { index: 0, to: 0 }, EffectStackEdit::Move { index: 0, to: 4 },
            EffectStackEdit::Remove(99), EffectStackEdit::Duplicate(99)] {
            assert_eq!(editor.edit_effects(&[a, b], edit).unwrap(), 0);
        }
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.revision, revision);
        assert!(!editor.can_undo());
        editor.edit_effects(&[a], property(0, EffectPropertyEdit::Radius(8.0))).unwrap();
        assert_eq!(editor.document.nodes[&a].effect_style, None);
        assert_eq!(editor.document.nodes[&a].fill_style, Some(456));
        assert!(editor.document.nodes[&a].overridden_properties.contains("effects"));
        assert_eq!(editor.document.nodes[&b].effect_style, Some(123));
        editor.undo();
        assert_eq!(editor.document.nodes[&a].effect_style, Some(123));
        assert!(!editor.document.nodes[&a].overridden_properties.contains("effects"));
        assert_eq!(editor.edit_effects(&[a], property(0, EffectPropertyEdit::Radius(4.0))).unwrap(), 0);
        assert!(editor.can_redo());
    }

    #[test]
    fn missing_rows_and_locked_ancestors_are_not_mutated() {
        let (mut editor, frame, a, b) = fixture();
        editor.document.nodes.get_mut(&b).unwrap().effects.clear();
        editor.refresh();
        assert_eq!(editor.edit_effects(&[a, b], property(0, EffectPropertyEdit::Radius(10.0))).unwrap(), 1);
        assert!(stack(&editor, b).is_empty());
        editor.document.nodes.get_mut(&frame).unwrap().locked = true;
        editor.refresh();
        let before = editor.document.to_json().unwrap();
        let history = editor.history_len();
        assert_eq!(editor.edit_effects(&[a, b], EffectStackEdit::Add(EffectKind::BackgroundBlur)).unwrap(), 0);
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.history_len(), history);
    }
}
