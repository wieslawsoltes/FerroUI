use super::{RustDocument, invalid};
use ferroui_designer_animation::{KeyframeEdit, KeyframeGroupEdit, KeyframeLocation, KeyframePaste, Timeline, VisualStateGroup};
use ferroui_designer_core::{
    ChangeSet, DesignerError, Document, Edit, NodeId, NodeKind, SourceSpan, TextPatch,
};
use ferroui_designer_rsx::{RsxAnimation, RsxAnimationEdit, RsxAnimationPlan, RsxChildEdit};
use std::collections::{BTreeMap, BTreeSet};

impl RustDocument {
    pub(crate) fn source_span(&self, document: &Document, node: NodeId) -> Option<SourceSpan> {
        let offset = document.ast().node(node)?.span.start;
        let map = self.map()?;
        map.control(offset)
            .map(|(_, span)| span)
            .or_else(|| map.animation(offset).map(|clip| clip.declaration))
    }

    pub(crate) fn animation(
        &self,
        document: &Document,
        program: NodeId,
    ) -> Result<&RsxAnimation, String> {
        let offset = document
            .ast()
            .node(program)
            .ok_or("Choose an animation program")?
            .span
            .start;
        self.map()
            .and_then(|map| map.animation(offset))
            .ok_or_else(|| "Choose an animation with a mapped Rust declaration".into())
    }

    pub(crate) fn keyframe(
        &self,
        document: &Document,
        program: NodeId,
        property: &str,
        edit: KeyframeEdit<'_>,
    ) -> Result<Vec<TextPatch>, String> {
        self.animation(document, program)?
            .author_keyframe(&self.source, self.catalog(), property, edit)
    }

    pub(crate) fn keyframes(
        &self,
        document: &Document,
        program: NodeId,
        timeline: &Timeline,
        keys: &[KeyframeLocation],
        edit: KeyframeGroupEdit,
    ) -> Result<(Vec<TextPatch>, Vec<KeyframeLocation>), String> {
        self.animation(document, program)?
            .author_keyframes(&self.source, self.catalog(), timeline, keys, edit)
    }

    pub(crate) fn mirror_keys(
        &self, document: &Document, program: NodeId, timeline: &Timeline,
        keys: &[KeyframeLocation], pivot: f64,
    ) -> Result<(Vec<TextPatch>, Vec<KeyframeLocation>), String> {
        self.animation(document, program)?
            .mirror_keyframes(&self.source, self.catalog(), timeline, keys, pivot)
    }

    pub(crate) fn paste_keys(
        &self, document: &Document, program: NodeId, timeline: &Timeline, paste: &KeyframePaste,
    ) -> Result<(Vec<TextPatch>, Vec<KeyframeLocation>), String> {
        self.animation(document, program)?
            .paste_keyframes(&self.source, self.catalog(), timeline, paste)
    }

    pub(crate) fn create_animation(
        &self, document: &Document, owner: NodeId, name: &str, selector: &str, duration: f64,
    ) -> Result<RsxAnimationPlan, String> {
        let node = document.ast().node(owner).ok_or("Choose an authored Rust control")?;
        let map = self.map().ok_or("Open the current Rust view")?;
        let (ordinal, _) = map.control(node.span.start).ok_or("Choose a mapped Rust control")?;
        self.catalog().project(&self.source, map.invocation.start)?
            .create_animation(ordinal, name, selector, duration)
    }

    pub(crate) fn edit_animation(
        &self, document: &Document, program: NodeId, edit: RsxAnimationEdit<'_>,
    ) -> Result<RsxAnimationPlan, String> {
        let declaration = self.animation(document, program)?.declaration;
        let map = self.map().ok_or("Open the current Rust view")?;
        self.catalog().project(&self.source, map.invocation.start)?
            .edit_animation(declaration, edit)
    }

    pub(crate) fn place_animation_clips(
        &self, document: &Document, placements: &[(NodeId, f64)],
    ) -> Result<Vec<TextPatch>, String> {
        let map = self.map().ok_or("Open the current Rust view")?;
        let placements = placements.iter().map(|&(program, delay)| {
            Ok((self.animation(document, program)?.declaration, delay))
        }).collect::<Result<Vec<_>, String>>()?;
        self.catalog().project(&self.source, map.invocation.start)?.place_animation_clips(&placements)
    }

    pub(crate) fn edit_state_group(
        &self, document: &Document, owner: NodeId, name: &str, model: Option<&VisualStateGroup>,
    ) -> Result<Vec<TextPatch>, String> {
        let node = document.ast().node(owner).ok_or("Choose an authored Rust control")?;
        let map = self.map().ok_or("Open the current Rust view")?;
        let (ordinal, _) = map.control(node.span.start).ok_or("Choose a mapped Rust control")?;
        let projection = self.catalog().project(&self.source, map.invocation.start)?;
        match model {
            Some(model) => projection.upsert_state_group(ordinal, model),
            None => projection.remove_state_group(ordinal, name),
        }
    }

    pub(crate) fn plan_child_placement(
        &self,
        document: &Document,
        label: &str,
        edits: Vec<Edit>,
        after: Option<NodeId>,
    ) -> Result<(RsxChildEdit, Vec<NodeId>), DesignerError> {
        let invocation = self.map().ok_or_else(|| {
            DesignerError::InvalidEdit("Repair the Rust buffer before placing controls".into())
        })?.invocation;
        let projection = self.catalog().project(self.source(), invocation.start)
            .map_err(DesignerError::InvalidEdit)?;
        let mut draft = document.snapshot_without_history();
        let change = draft.apply(label, edits.clone())?;
        if change.is_empty() {
            return Ok((RsxChildEdit { patches: Vec::new(), controls: BTreeMap::new() }, Vec::new()));
        }
        let inserted: BTreeSet<_> = change.inserted.into_iter().collect();
        let roots = draft.ast().nodes_in_source_order().filter(|node| {
            node.kind == NodeKind::Element && inserted.contains(&node.id)
                && node.parent.is_none_or(|parent| !inserted.contains(&parent))
        }).map(|node| node.id).collect();
        let plan = projection.plan_child_edit(document, &draft, &edits, after)
            .map_err(DesignerError::InvalidEdit)?;
        Ok((plan, roots))
    }

    pub(crate) fn plan_edit(
        &self,
        document: &Document,
        label: &str,
        edits: Vec<Edit>,
    ) -> Result<Vec<TextPatch>, DesignerError> {
        if crate::rsx_placement::is_child_placement(&edits) {
            return self.plan_child_placement(document, label, edits, None).map(|(plan, _)| plan.patches);
        }
        let map = self
            .map()
            .ok_or_else(|| invalid("Repair the Rust buffer before making visual edits"))?;
        let projection = self
            .catalog()
            .project(&self.source, map.invocation.start)
            .map_err(invalid)?;
        if edits.iter().all(|edit| {
            matches!(
                edit,
                Edit::SetAttribute { .. } | Edit::RemoveAttribute { .. }
            )
        }) {
            let mut patches = Vec::new();
            let mut assigned_names = BTreeSet::new();
            for edit in &edits {
                let (node, name, value) = match edit {
                    Edit::SetAttribute { node, name, value } => (*node, name, Some(value.as_str())),
                    Edit::RemoveAttribute { node, name } => (*node, name, None),
                    _ => unreachable!("attribute edits"),
                };
                let offset = document
                    .ast()
                    .node(node)
                    .ok_or(DesignerError::UnknownNode(node))?
                    .span
                    .start;
                let (index, _) = map.control(offset).ok_or_else(|| {
                    invalid("The selected control has no Rust source mapping")
                })?;
                if name == "Name" {
                    if value.is_some_and(|value| !assigned_names.insert(value)) {
                        return Err(invalid(
                            "Select distinct names for controls in this RSX invocation",
                        ));
                    }
                    patches.extend(
                        projection
                            .edit_control_names(&[index], value)
                            .map_err(invalid)?,
                    );
                } else {
                    patches.push(projection.edit_property(index, name, value).map_err(invalid)?);
                }
            }
            return Ok(patches);
        }
        let mut candidate = document.snapshot_without_history();
        candidate.apply(label, edits)?;
        projection.apply_xaml(candidate.source()).map_err(invalid)
    }

    pub(crate) fn apply(
        &mut self,
        document: &mut Document,
        label: &str,
        patches: Vec<TextPatch>,
    ) -> Result<ChangeSet, DesignerError> {
        let invocation = self
            .map()
            .ok_or_else(|| invalid("Repair the Rust buffer before editing"))?
            .invocation;
        if patches
            .iter()
            .any(|patch| patch.span.start < invocation.start || patch.span.end > invocation.end)
        {
            return Err(invalid(
                "A visual edit must stay within the active RSX invocation",
            ));
        }
        let source = TextPatch::apply_all(&self.source, patches)?;
        self.update(document, source.into(), label, true)
            .map(|update| update.change)
    }
}
