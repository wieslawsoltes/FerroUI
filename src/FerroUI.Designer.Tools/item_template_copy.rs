//! Compose a private inline template and its necessary dependencies as one edit.
use super::super::resource_copy::{match_insertion_line_endings, ResourceCopyPlanner};
use super::edits::strip_template_key;
use super::*;
use ferroui_designer_core::TextPatch;

#[cfg(test)]
#[path = "item_template_copy_tests.rs"]
mod tests;

impl ItemTemplateSlot {
    pub fn plan_included_copy(
        &self,
        document: &Document,
        sources: &ResourceCopySources<'_>,
        resource_owners: &[NodeId],
        relocate: &impl Fn(&str, &Document) -> Result<String>,
    ) -> Result<Vec<Edit>> {
        self.ensure_editable()?;
        let ItemTemplateValue::Resource { key, .. } = &self.value else {
            return Err(ToolError("Select an assigned template resource.".into()));
        };
        let dependency_owner = resource_owners.iter().copied().find(|owner| {
            authoring::resource_container(document, *owner).is_ok()
                && component_property_attribute(document, *owner, "Resources")
                    .is_ok_and(|attribute| attribute.is_none())
        });
        let scope = sources.scope(document);
        let root = references::resolve(scope, self.owner, key, None)?
            .unique()
            .filter(|location| location.uri.is_some())
            .ok_or_else(|| {
                ToolError(
                    "Choose an included template with one authored definition across themes."
                        .into(),
                )
            })?;
        let include = sources
            .includes
            .iter()
            .find(|include| include.address == root.address && Some(include.uri) == root.uri)
            .ok_or_else(|| {
                ToolError("Open the accepted template source before copying it.".into())
            })?;
        require(
            keyed_template(element(include.document, root.node)?, self.property).as_ref() == Some(key),
            "The assigned source is no longer a template.",
        )?;

        let mut planner = ResourceCopyPlanner::new(document, sources, key, relocate);
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
