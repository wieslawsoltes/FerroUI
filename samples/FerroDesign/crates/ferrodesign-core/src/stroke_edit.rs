//! Atomic variable-width stroke authoring. Geometry stays editable; the
//! renderer prepares the outline only after a document revision changes.
use crate::{Color, DocumentError, Editor, NodeId, Stroke, StrokeCap, StrokeWidthProfile};

fn geometry_stroke(stroke: &Stroke) -> Stroke {
    Stroke { paint: crate::Paint::Solid(Color::BLACK), width: stroke.width,
        opacity: stroke.opacity, visible: stroke.visible, align: stroke.align,
        cap: stroke.cap, join: stroke.join, dash: stroke.dash.clone(),
        width_profile: stroke.width_profile.clone() }
}

impl Editor {
    /// Apply a profile to the same stroke-stack entry on each editable target.
    /// Choosing a nonuniform profile removes an arrow cap in the same edit.
    /// Dashed or branched paths fail before any node or history is changed.
    pub fn set_stroke_width_profile(&mut self, ids: &[NodeId], index: usize,
        profile: StrokeWidthProfile) -> Result<usize, DocumentError>
    {
        profile.validate()?;
        let mut changed = std::collections::BTreeSet::new();
        for &id in ids {
            let node = self.document.nodes.get(&id).ok_or(DocumentError::MissingNode(id))?;
            if self.document.effective_locked(id) { continue; }
            let stroke = node.strokes.get(index).ok_or_else(|| DocumentError::Invalid(
                format!("Layer {} has no stroke {}.", node.name, index + 1)))?;
            let cap = if !profile.is_uniform() && stroke.cap == StrokeCap::Arrow { StrokeCap::Butt } else { stroke.cap };
            if stroke.width_profile == profile && stroke.cap == cap { continue; }
            let mut candidate = geometry_stroke(stroke);
            candidate.width_profile = profile.clone();
            candidate.cap = cap;
            if !profile.is_uniform() { crate::variable_width_outline(node, &candidate)?; }
            changed.insert(id);
        }
        if changed.is_empty() { return Ok(0); }
        let ids: Vec<_> = changed.into_iter().collect();
        self.update_nodes(&ids, "Stroke width profile", |node| {
            let stroke = &mut node.strokes[index];
            stroke.width_profile = profile.clone();
            if !profile.is_uniform() && stroke.cap == StrokeCap::Arrow { stroke.cap = StrokeCap::Butt; }
            node.overridden_properties.insert("strokes".into());
        });
        Ok(ids.len())
    }

    /// Change a stroke's base width without allowing an otherwise finite input
    /// to overflow its custom multipliers or prepared outline. Validate every
    /// target before creating one history entry; paint data is never copied.
    pub fn set_stroke_width(&mut self, ids: &[NodeId], index: usize, width: f64) -> Result<usize, DocumentError> {
        if !width.is_finite() || width < 0.0 {
            return Err(DocumentError::Invalid("Stroke width must be finite and nonnegative.".into()));
        }
        let mut changed = std::collections::BTreeSet::new();
        for &id in ids {
            let node = self.document.nodes.get(&id).ok_or(DocumentError::MissingNode(id))?;
            if self.document.effective_locked(id) { continue; }
            let mut candidate = if let Some(stroke) = node.strokes.get(index) {
                if stroke.width == width { continue; }
                geometry_stroke(stroke)
            } else if index == 0 && node.strokes.is_empty() {
                Stroke::new(Color::BLACK, width)
            } else {
                return Err(DocumentError::Invalid(format!("Layer {} has no stroke {}.", node.name, index + 1)));
            };
            candidate.width = width;
            crate::validate_variable_width_support(node, &candidate)?;
            if !candidate.width_profile.is_uniform() { crate::variable_width_outline(node, &candidate)?; }
            changed.insert(id);
        }
        if changed.is_empty() { return Ok(0); }
        let ids: Vec<_> = changed.into_iter().collect();
        self.update_nodes(&ids, "Stroke width", |node| {
            if node.strokes.is_empty() { node.strokes.push(Stroke::new(Color::BLACK, width)); }
            else { node.strokes[index].width = width; }
            node.overridden_properties.insert("strokes".into());
        });
        Ok(ids.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Document, Node, NodeKind, Stroke, StrokeWidthPoint};

    #[test]
    fn base_width_preflights_every_profile_and_preserves_undo_redo_on_error() {
        let mut document = Document::new("Base stroke width");
        let page = document.pages[0].id;
        let a = document.insert_node(page, Node::new(NodeKind::Line, "Uniform", 0.0, 0.0, 100.0, 0.0)).unwrap();
        let mut wide = Node::new(NodeKind::Line, "Custom", 0.0, 40.0, 100.0, 0.0);
        wide.strokes[0].width_profile = StrokeWidthProfile::Custom { points: vec![StrokeWidthPoint::new(0.5, 2.0)] };
        let b = document.insert_node(page, wide).unwrap();
        let mut editor = Editor::new(document);
        let before = editor.document.to_json().unwrap();
        assert_eq!(editor.set_stroke_width(&[a, b], 0, 20.0).unwrap(), 2);
        let after = editor.document.to_json().unwrap();
        assert_eq!(editor.history_len(), 1);
        assert!(editor.undo()); assert_eq!(editor.document.to_json().unwrap(), before);
        for value in [-1.0, f64::INFINITY, f64::MAX] {
            assert!(editor.set_stroke_width(&[a, b], 0, value).is_err());
            assert_eq!(editor.document.to_json().unwrap(), before);
            assert!(editor.can_redo());
        }
        assert!(editor.redo()); assert_eq!(editor.document.to_json().unwrap(), after);
    }

    #[test]
    fn stroke_profiles_are_atomic_round_trip_and_undoable_with_arrow_conversion() {
        let mut document = Document::new("Width profiles");
        let page = document.pages[0].id;
        let mut line = Node::new(NodeKind::Line, "Line", 0.0, 0.0, 100.0, 0.0);
        line.strokes = vec![Stroke::new(Color::BLACK, 20.0)];
        line.strokes[0].cap = StrokeCap::Arrow;
        let id = document.insert_node(page, line).unwrap();
        let mut editor = Editor::new(document);
        let before = editor.document.to_json().unwrap();
        assert_eq!(editor.set_stroke_width_profile(&[id, id], 0, StrokeWidthProfile::Wedge).unwrap(), 1);
        assert_eq!(editor.document.nodes[&id].strokes[0].cap, StrokeCap::Butt);
        assert!(editor.document.nodes[&id].overridden_properties.contains("strokes"));
        let after = editor.document.to_json().unwrap();
        assert_eq!(Document::from_json(&after).unwrap().to_json().unwrap(), after);
        editor.undo(); assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.set_stroke_width_profile(&[id, u64::MAX], 0, StrokeWidthProfile::Eye).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        editor.redo(); assert_eq!(editor.document.to_json().unwrap(), after);
        editor.undo();
        assert_eq!(editor.set_stroke_width_profile(&[id], 0, StrokeWidthProfile::Uniform).unwrap(), 0);
        editor.redo(); assert_eq!(editor.document.to_json().unwrap(), after);
    }

    #[test]
    fn invalid_or_dashed_width_edits_preserve_all_targets_and_redo() {
        let mut document = Document::new("Width validation");
        let page = document.pages[0].id;
        let a = document.insert_node(page, Node::new(NodeKind::Line, "A", 0.0, 0.0, 100.0, 0.0)).unwrap();
        let mut dashed = Node::new(NodeKind::Line, "Dashed", 0.0, 20.0, 100.0, 0.0);
        dashed.strokes[0].dash = vec![4.0, 2.0];
        let b = document.insert_node(page, dashed).unwrap();
        let mut editor = Editor::new(document);
        editor.set_stroke_width_profile(&[a], 0, StrokeWidthProfile::Eye).unwrap();
        editor.undo();
        let before = editor.document.to_json().unwrap();
        assert!(editor.set_stroke_width_profile(&[a, b], 0, StrokeWidthProfile::Eye).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        let invalid = StrokeWidthProfile::Custom { points: vec![
            StrokeWidthPoint { position: 0.5, width: 1.0 },
            StrokeWidthPoint { position: 0.5, width: 2.0 },
        ] };
        assert!(editor.set_stroke_width_profile(&[a], 0, invalid).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        editor.redo(); assert_eq!(editor.document.nodes[&a].strokes[0].width_profile, StrokeWidthProfile::Eye);
        editor.document.nodes.get_mut(&a).unwrap().locked = true;
        assert_eq!(editor.set_stroke_width_profile(&[a], 0, StrokeWidthProfile::Wedge).unwrap(), 0);
    }
}
