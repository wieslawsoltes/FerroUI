//! Compose a private inline template and its necessary dependencies as one edit.
use super::super::resource_copy::{match_insertion_line_endings, ResourceCopyPlanner};
use super::edits::strip_template_key;
use super::*;
use ferroui_designer_core::TextPatch;

#[cfg(test)]
#[path = "item_template_copy_tests.rs"]
mod tests;

impl ItemTemplateSlot {
    /// Copy the proven source of an effective control template into this
    /// empty slot using the same dependency review as a resource assignment.
    pub fn plan_effective_template_copy(
        &self,
        document: &Document,
        sources: &ResourceCopySources<'_>,
        definition: ComponentResourceLocation<'_>,
        resource_owners: &[NodeId],
        relocate: &impl Fn(&str, &Document) -> Result<String>,
    ) -> Result<Vec<Edit>> {
        self.ensure_editable()?;
        require(
            self.property == ItemTemplateMember::Template
                && matches!(self.value, ItemTemplateValue::Unset),
            "Choose a control with an inherited template and no local assignment.",
        )?;
        let origin = sources.origin(document, definition)?;
        let node = element(origin.document, definition.node)?;
        require(self.property.accepts(node), "The source is no longer a control template.")?;
        let prefix = literal_key(node).unwrap_or_else(|| "ControlTemplate".into());
        self.plan_definition_copy(document, sources, definition, &prefix, resource_owners, relocate)
    }

    /// Copy the assigned template and retain the meaning of its resource uses
    /// in the new scope. The source may be local or in an accepted include.
    pub fn plan_resource_copy(
        &self,
        document: &Document,
        sources: &ResourceCopySources<'_>,
        resource_owners: &[NodeId],
        relocate: &impl Fn(&str, &Document) -> Result<String>,
    ) -> Result<Vec<Edit>> {
        self.plan_scoped_copy(document, sources, resource_owners, relocate, false)
    }

    pub fn plan_included_copy(
        &self,
        document: &Document,
        sources: &ResourceCopySources<'_>,
        resource_owners: &[NodeId],
        relocate: &impl Fn(&str, &Document) -> Result<String>,
    ) -> Result<Vec<Edit>> {
        self.plan_scoped_copy(document, sources, resource_owners, relocate, true)
    }

    fn plan_scoped_copy(
        &self,
        document: &Document,
        sources: &ResourceCopySources<'_>,
        resource_owners: &[NodeId],
        relocate: &impl Fn(&str, &Document) -> Result<String>,
        included_only: bool,
    ) -> Result<Vec<Edit>> {
        self.ensure_editable()?;
        let ItemTemplateValue::Resource { key, .. } = &self.value else {
            return Err(ToolError("Select an assigned template resource.".into()));
        };
        let scope = sources.scope(document);
        let root = references::resolve(scope, self.owner, key, None)?
            .unique()
            .filter(|location| !included_only || location.uri.is_some())
            .ok_or_else(|| {
                ToolError(
                    "Choose a template resource with one authored definition across themes."
                        .into(),
                )
            })?;
        let original = sources.origin(document, root)?.document;
        require(
            keyed_template(element(original, root.node)?, self.property).as_ref() == Some(key),
            "The assigned source is no longer a template.",
        )?;
        self.plan_definition_copy(document, sources, root, key, resource_owners, relocate)
    }

    fn plan_definition_copy(
        &self,
        document: &Document,
        sources: &ResourceCopySources<'_>,
        root: ComponentResourceLocation<'_>,
        prefix: &str,
        resource_owners: &[NodeId],
        relocate: &impl Fn(&str, &Document) -> Result<String>,
    ) -> Result<Vec<Edit>> {
        let dependency_owner = resource_owners.iter().copied().find(|owner| {
            authoring::resource_container(document, *owner).is_ok()
                && component_property_attribute(document, *owner, "Resources")
                    .is_ok_and(|attribute| attribute.is_none())
        });
        let mut planner = ResourceCopyPlanner::new(document, sources, prefix, relocate);
        let mut fragment = planner.fragment(root)?;
        strip_template_key(&mut fragment)?;
        let mut trial = document.clone();
        let copied = self.place_copy(&mut trial, fragment.source())?;
        let template = planner.rewrite(root, fragment, Some((&trial, copied)))?;

        let mut candidate = document.clone();
        let installed = planner.install(&mut candidate, dependency_owner)?;
        let root = self.place_copy(&mut candidate, &template.source)?;
        planner.validate(&candidate, root, &template, &installed)?;

        Ok(TextPatch::between(document.source(), candidate.source())
            .into_iter()
            .map(|patch| Edit::Text {
                span: patch.span,
                replacement: patch.replacement,
            })
            .collect())
    }

    fn place_copy(&self, document: &mut Document, source: &str) -> Result<NodeId> {
        let owner = element(document, self.owner)?;
        let property = self.property.name();
        let tag = format!("{}.{property}", owner.name);
        let mut edits = self.clear();
        // Resource values must be installed before the deferred template is created.
        let resources = property_elements(document, owner, "Resources")?;
        let children = document.ast().element_children(owner.id);
        let index = resources.last().and_then(|resource| {
            children.iter().position(|id| *id == resource.id)
        }).map_or(0, |index| {
            let removed_before = matches!(self.declaration, ItemTemplateDeclaration::Element(removed)
                if children[..index].contains(&removed));
            index + usize::from(!removed_before)
        });
        edits.push(Edit::InsertChild {
            parent: owner.id,
            index,
            source: format!("<{tag}>{source}</{tag}>"),
        });
        match_insertion_line_endings(document, &mut edits);
        document
            .apply("Stage inline template copy", edits)
            .map_err(error)?;
        property_elements(document, element(document, self.owner)?, property)?
            .first()
            .and_then(|property| {
                document
                    .ast()
                    .element_children(property.id)
                    .first()
                    .copied()
            })
            .ok_or_else(|| ToolError("The inline template was not created.".into()))
    }
}
