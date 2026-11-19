use super::{RustDocument, invalid};
use ferroui_designer_animation::{KeyframeEdit, KeyframeGroupEdit, KeyframeLocation, Timeline};
use ferroui_designer_core::{
    ChangeSet, DesignerError, Document, Edit, NodeId, SourceSpan, TextPatch,
};
use ferroui_designer_rsx::{RsxAnimation, RsxAnimationEdit, RsxAnimationPlan};

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

    pub(crate) fn plan_edit(
        &self,
        document: &Document,
        label: &str,
        edits: Vec<Edit>,
    ) -> Result<Vec<TextPatch>, DesignerError> {
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
            return edits
                .into_iter()
                .map(|edit| {
                    let (node, name, value) = match edit {
                        Edit::SetAttribute { node, name, value } => (node, name, Some(value)),
                        Edit::RemoveAttribute { node, name } => (node, name, None),
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
                    projection
                        .edit_property(index, &name, value.as_deref())
                        .map_err(invalid)
                })
                .collect();
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
