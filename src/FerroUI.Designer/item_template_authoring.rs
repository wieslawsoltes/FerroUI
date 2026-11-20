//! Shared editor and native Studio access to authored item templates.
use crate::{
    DefinitionPreviewOptions, DesignerSession, NodeId, component_session::property_schema,
};
use ferroui_controls::ItemsControl;
use ferroui_designer_core::{Document, Node, NodeKind, design_fixtures::DESIGN_NAMESPACE};
use ferroui_designer_tools::{
    ItemTemplateChange, ItemTemplateDeclaration, ItemTemplateMember, ItemTemplateSlot,
};

impl DesignerSession {
    pub fn item_template_members(&self, owner: NodeId) -> Vec<ItemTemplateMember> {
        if self.uses_rust_source() || !self.document().is_valid() {
            return Vec::new();
        }
        let Some(node) = self
            .document()
            .ast()
            .node(owner)
            .filter(|node| node.kind == NodeKind::Element)
        else {
            return Vec::new();
        };
        let schema = property_schema();
        let namespace = node.namespace_uri.as_deref();
        let Some(info) = schema.type_info(&node.name, namespace) else {
            return Vec::new();
        };
        let properties = schema.properties(&info.name, Some(&info.namespace));
        ItemTemplateMember::ALL
            .into_iter()
            .filter(|member| {
                // TableView keeps this deprecated property for compatibility. Its cells own the templates.
                if info.name == "TableView" && *member == ItemTemplateMember::ItemTemplate {
                    return false;
                }
                properties.get(member.name()).is_some_and(|property| {
                    !property.read_only
                        && (property.type_name == "Option<Rc<dyn IDataTemplate>>"
                            || (info.name == "ItemsRepeater"
                                && *member == ItemTemplateMember::ItemTemplate))
                })
            })
            .collect()
    }

    pub fn item_template_slots(&self, owner: NodeId) -> Result<Vec<ItemTemplateSlot>, String> {
        self.item_template_members(owner)
            .into_iter()
            .map(|property| self.item_template_slot(owner, property))
            .collect()
    }

    pub(crate) fn item_template_slot(
        &self,
        owner: NodeId,
        property: ItemTemplateMember,
    ) -> Result<ItemTemplateSlot, String> {
        if !self.item_template_members(owner).contains(&property) {
            return Err("Select an authored control or column with this template property.".into());
        }
        let document = self.document();
        let node = document
            .ast()
            .node(owner)
            .ok_or("The template owner was removed.")?;
        let declaration = authored_member(document, node, property)?;
        let context = self.component_resource_context(owner);
        let mut slot =
            ItemTemplateSlot::read(document, owner, property, declaration, context.as_ref())
                .map_err(|error| error.to_string())?;
        if property == ItemTemplateMember::CellTemplate
            && property_schema()
                .type_info(&node.name, node.namespace_uri.as_deref())
                .is_some_and(|info| info.name == "TreeDataGridTemplateColumn")
        {
            slot.can_reset = false;
        }
        if !self.can_manipulate() || self.ensure_layer_manipulable(owner).is_err() {
            slot.reason.get_or_insert_with(|| {
                "Open an editable, current preview and unlock the owning layer.".into()
            });
        } else if self.has_display_member_binding(owner, property) {
            slot.reason.get_or_insert_with(|| {
                "Clear DisplayMemberBinding before assigning an item template.".into()
            });
        }
        Ok(slot)
    }

    pub fn edit_item_template(
        &mut self,
        owner: NodeId,
        property: ItemTemplateMember,
        change: ItemTemplateChange,
    ) -> Result<(), String> {
        let label = change.label();
        let slot = self.item_template_slot(owner, property)?;
        let edits = match change {
            ItemTemplateChange::AssignIncludedResource { key } => {
                self.included_template_node(owner, &key, None)?
                    .ok_or("Select an included data template that resolves across themes")?;
                slot.assign_key(self.document(), &key).map_err(|error| error.to_string())?
            }
            ItemTemplateChange::CopyInline => self.plan_inline_template_copy(&slot)?,
            change => slot.plan(self.document(), change).map_err(|error| error.to_string())?,
        };
        if !edits.is_empty() {
            self.apply(label, edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn preview_item_template(
        &mut self,
        owner: NodeId,
        property: ItemTemplateMember,
    ) -> Result<(), String> {
        let template = self
            .item_template_slot(owner, property)?
            .value
            .definition()
            .ok_or("Open a local authored template to preview its contents.")?;
        self.enter_definition(
            template,
            DefinitionPreviewOptions::default(),
            Some(crate::item_template_samples::ItemTemplatePreview::Assigned { owner, property }),
        )
    }

    fn has_display_member_binding(&self, owner: NodeId, property: ItemTemplateMember) -> bool {
        property == ItemTemplateMember::ItemTemplate
            && self
                .preview
                .visual(owner)
                .and_then(|visual| visual.cast::<ItemsControl>())
                .is_some_and(|control| control.display_member_binding().is_some())
    }
}

fn authored_member(
    document: &Document,
    owner: &Node,
    property: ItemTemplateMember,
) -> Result<ItemTemplateDeclaration, String> {
    let schema = property_schema();
    let name = property.name();
    let matches = |spelling: &str| {
        !spelling.starts_with("Design.") && spelling.rsplit([':', '.']).next() == Some(name)
    };
    let attributes = owner
        .attributes
        .iter()
        .filter(|attribute| {
            matches(&attribute.name)
                && attribute.namespace_uri.as_deref() != Some(DESIGN_NAMESPACE)
                && schema
                    .attribute_property(document, owner, attribute)
                    .is_some_and(|property| property.name == name)
        })
        .map(|attribute| ItemTemplateDeclaration::Attribute(attribute.name.clone()));
    let elements = document
        .ast()
        .element_children(owner.id)
        .into_iter()
        .filter_map(|id| document.ast().node(id))
        .filter(|child| {
            child.kind == NodeKind::PropertyElement
                && matches(&child.name)
                && child.namespace_uri.as_deref() != Some(DESIGN_NAMESPACE)
                && schema
                    .element_property(owner, child)
                    .is_some_and(|property| property.name == name)
        })
        .map(|child| ItemTemplateDeclaration::Element(child.id));
    let mut declarations = attributes.chain(elements);
    let declaration = declarations
        .next()
        .unwrap_or(ItemTemplateDeclaration::Unset);
    if declarations.next().is_some() {
        return Err(format!(
            "{name} has more than one authored value. Repair the XAML first."
        ));
    }
    Ok(declaration)
}
