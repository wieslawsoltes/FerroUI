//! Template source edits based on a host's proven owner and definition locations.
use crate::item_template_authoring::{authored_member, control_template_matches};
use crate::runtime_include_sources::IncludeLookup;
use ferroui_designer_core::{Document, Edit, NodeId, NodeKind};
use ferroui_designer_language::{relocate_xaml_fragment, Schema};
use ferroui_designer_project::PreviewIncludes;
use ferroui_designer_tools::{
    ComponentResourceLocation, IncludedResourceDocument, ItemTemplateDeclaration,
    ItemTemplateMember, ItemTemplateSlot, ResourceCopySources, ToolError,
};

pub struct ScopedControlTemplates<'a> {
    document: &'a Document,
    address: &'a str,
    includes: &'a PreviewIncludes,
    schema: &'a Schema,
}

impl<'a> ScopedControlTemplates<'a> {
    pub fn new(
        document: &'a Document,
        address: &'a str,
        includes: &'a PreviewIncludes,
        schema: &'a Schema,
    ) -> Self {
        Self {
            document,
            address,
            includes,
            schema,
        }
    }

    pub fn can_copy(&self, owner: NodeId) -> bool {
        self.inherited_slot(owner).is_ok()
    }

    pub fn plan_copy(
        &self,
        owner: NodeId,
        address: &str,
        definition: NodeId,
    ) -> Result<Vec<Edit>, String> {
        let slot = self.inherited_slot(owner)?;
        let control = self
            .document
            .ast()
            .node(owner)
            .ok_or("The template owner changed")?;
        let (uri, document) = if address == self.address {
            (None, self.document)
        } else {
            let source = self
                .includes
                .authored_source(address)
                .ok_or("The template is outside the accepted source graph")?;
            (Some(source.uri.as_str()), &source.document)
        };
        let template = document
            .ast()
            .node(definition)
            .ok_or("The template definition changed")?;
        if !control_template_matches(self.schema, control, document, template) {
            return Err("Choose a control template compatible with its authored owner".into());
        }
        let includes = self
            .includes
            .authored_sources()
            .map(|(address, include)| IncludedResourceDocument {
                address,
                uri: &include.uri,
                document: &include.document,
            })
            .collect::<Vec<_>>();
        let sources = ResourceCopySources {
            address: self.address,
            graph: &IncludeLookup(self.includes),
            includes: &includes,
        };
        let owners =
            std::iter::successors(Some(control), |node| self.document.ast().node(node.parent?))
                .filter(|node| {
                    node.kind == NodeKind::Element
                        && self
                            .schema
                            .property(&node.name, node.namespace_uri.as_deref(), "Resources")
                            .is_some()
                })
                .map(|node| node.id)
                .collect::<Vec<_>>();
        slot.plan_effective_template_copy(
            self.document,
            &sources,
            ComponentResourceLocation {
                uri,
                address,
                node: definition,
            },
            &owners,
            &|origin, fragment| {
                relocate_xaml_fragment(fragment, self.schema, origin, self.address)
                    .map_err(|error| ToolError(error.to_string()))
            },
        )
        .map_err(|error| error.to_string())
    }

    fn inherited_slot(&self, owner: NodeId) -> Result<ItemTemplateSlot, String> {
        let node = self
            .document
            .ast()
            .node(owner)
            .ok_or("The template owner changed")?;
        let property = ItemTemplateMember::Template;
        let declared =
            self.schema
                .property(&node.name, node.namespace_uri.as_deref(), property.name());
        if !declared.is_some_and(|entry| {
            !entry.read_only && entry.type_name == "Option<Rc<dyn IControlTemplate>>"
        }) {
            return Err(
                "The XAML type needs a writable Template property in the project schema".into(),
            );
        }
        let declaration = authored_member(self.schema, self.document, node, property)?;
        if declaration != ItemTemplateDeclaration::Unset {
            return Err("The control already has a local Template value".into());
        }
        ItemTemplateSlot::read(self.document, owner, property, declaration, None)
            .map_err(|error| error.to_string())
    }
}
