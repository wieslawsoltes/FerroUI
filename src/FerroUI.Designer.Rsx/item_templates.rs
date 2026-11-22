//! Source anchors for data templates whose bodies are authored as RSX.
mod syntax;
#[cfg(test)]
mod tests;

use crate::{
    RsxProjection, RsxSourceEdit, RsxSourceLocation,
    animation::syntax::span,
    blocks::keep_comments,
    columns::ColumnPosition,
    local_rust::RustSourceScope,
    registered::Class,
    source_edits::{insert_field, reset_field},
};
use ferroui_designer_core::{Document, NodeId, SourceSpan, TextPatch};
use ferroui_designer_tools::{ColumnKind, ItemTemplateChange, ItemTemplateMember};
use syn::Expr;

use ItemTemplateMember::{
    CellEditingTemplate, CellTemplate, ItemTemplate, MenuItemTemplate, SelectionBoxItemTemplate,
};

/// One item or cell template member and its authored locations.
pub struct RsxItemTemplate {
    pub property: ItemTemplateMember,
    pub expression: Option<RsxSourceLocation>,
    pub body: Option<RsxTemplateBody>,
    pub can_create: bool,
    pub can_reset: bool,
    pub reason: Option<&'static str>,
}

/// The exact RSX invocation inside a local or shared template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RsxTemplateBody {
    pub location: RsxSourceLocation,
    pub factory: Option<String>,
}

pub(crate) struct TemplateSource {
    expression: SourceSpan,
    body: Option<RsxTemplateBody>,
    removal: Option<SourceSpan>,
}

impl TemplateSource {
    pub(crate) fn read(
        expression: &Expr,
        removal: Option<SourceSpan>,
        scope: Option<RustSourceScope<'_>>,
    ) -> Self {
        let body = read_body(expression, scope, None).or_else(|| {
            let returned = scope?.returned_builder(expression)?;
            read_body(
                returned.expression,
                Some(returned.scope),
                Some(returned.name),
            )
        });
        Self {
            expression: span(expression),
            body,
            removal,
        }
    }
}

fn read_body(
    expression: &Expr,
    scope: Option<RustSourceScope<'_>>,
    factory: Option<&str>,
) -> Option<RsxTemplateBody> {
    let span = syntax::template_body(expression, scope)?;
    Some(RsxTemplateBody {
        location: RsxSourceLocation {
            uri: scope.and_then(RustSourceScope::uri).map(str::to_owned),
            span,
        },
        factory: factory.map(str::to_owned),
    })
}

#[derive(Default)]
pub(crate) struct TemplateFields {
    members: &'static [(ItemTemplateMember, &'static str)],
    authored: Vec<(ItemTemplateMember, TemplateSource)>,
    opaque_setter: bool,
    display_member_binding: bool,
}

impl TemplateFields {
    pub(crate) fn for_class(class: &Class) -> Self {
        let members = match class {
            Class::Builtin(name) => members_for_class(name),
            Class::Registered(_) => &[],
        };
        Self {
            members,
            ..Self::default()
        }
    }

    pub(crate) fn note_setter(&mut self, setter: &str, qualified: bool) {
        self.opaque_setter |= matches!(setter, "prop" | "set" | "with")
            || (qualified && self.member(setter).is_some());
        self.display_member_binding |= setter == "display_member_binding";
    }

    pub(crate) fn member(&self, setter: &str) -> Option<ItemTemplateMember> {
        self.members
            .iter()
            .find_map(|(property, name)| (*name == setter).then_some(*property))
    }

    pub(crate) fn record(
        &mut self,
        property: ItemTemplateMember,
        expression: Option<&Expr>,
        field: Option<SourceSpan>,
        scope: Option<RustSourceScope<'_>>,
    ) {
        let Some(expression) = expression else {
            self.opaque_setter = true;
            return;
        };
        if let Some((_, previous)) = self.authored.iter_mut().find(|(name, _)| *name == property) {
            *previous = TemplateSource {
                expression: span(expression),
                body: None,
                removal: None,
            };
        } else {
            self.authored
                .push((property, TemplateSource::read(expression, field, scope)));
        }
    }
}

fn members_for_class(name: &str) -> &'static [(ItemTemplateMember, &'static str)] {
    match name {
        "ComboBox" => &[
            (ItemTemplate, "item_template"),
            (SelectionBoxItemTemplate, "selection_box_item_template"),
        ],
        "ItemsControl" | "ListBox" | "TreeView" => &[(ItemTemplate, "item_template")],
        "NavigationView" => &[(MenuItemTemplate, "menu_item_template")],
        _ => &[],
    }
}

/// A projected native element type that can own a mapped RSX data template.
pub fn is_item_template_owner(name: &str) -> bool {
    matches!(name, "TableViewColumn" | "TreeDataGridTemplateColumn")
        || !members_for_class(name).is_empty()
}

#[derive(Clone, Copy)]
enum TemplateOwner {
    Control(usize, ItemTemplateMember),
    Column(usize, ColumnPosition, ItemTemplateMember),
}

struct TemplateSlotSource<'a> {
    property: ItemTemplateMember,
    existing: Option<&'a TemplateSource>,
    declaration: RsxSourceLocation,
    insert_at: usize,
    setter: &'static str,
    writable: bool,
}

impl RsxProjection<'_> {
    pub fn item_templates(&self, document: &Document, node: NodeId) -> Vec<RsxItemTemplate> {
        self.template_owners(document, node)
            .into_iter()
            .filter_map(|owner| self.template_binding(owner))
            .map(|binding| self.template_slot(&binding))
            .collect()
    }

    /// Source anchors for every mapped template assignment in the invocation.
    /// Browsing a document reuses the projected controls and column models.
    pub fn authored_item_templates(
        &self, document: &Document,
    ) -> Vec<(NodeId, ItemTemplateMember, RsxSourceLocation)> {
        let controls = self.preview_markup().controls;
        let mut locations = Vec::new();
        for node in document.ast().nodes_in_source_order() {
            let Ok(index) = controls.binary_search(&node.span.start) else { continue };
            if node.kind != ferroui_designer_core::NodeKind::Element {
                continue;
            }
            for (property, source) in &self.nodes[index].templates.authored {
                locations.push((node.id, *property, RsxSourceLocation::local(source.expression)));
            }
            for (column, position, kind) in self.template_columns(document, node.id, index) {
                for &property in column_template_members(kind) {
                    if let Some((_, Some(source), _, declaration)) = self.column_template(index, position, property) {
                        locations.push((column, property, RsxSourceLocation {
                            uri: declaration.uri.clone(), span: source.expression,
                        }));
                    }
                }
            }
        }
        locations
    }

    /// Produce checked patches against the original Rust. The owning RSX
    /// invocation and exact slot must resolve again in the edited candidate.
    pub fn edit_item_template(
        &self,
        document: &Document,
        node: NodeId,
        property: ItemTemplateMember,
        change: &ItemTemplateChange,
        row_type: Option<&str>,
    ) -> Result<Vec<TextPatch>, String> {
        let edit = self.plan_item_template(document, node, property, change, row_type)?;
        if edit.source.uri.is_some() {
            return Err("Apply the template edit to its owning Rust module".into());
        }
        Ok(edit.patches)
    }

    /// Produce checked patches for the original assignment, including shared column definitions.
    pub fn plan_item_template(
        &self,
        document: &Document,
        node: NodeId,
        property: ItemTemplateMember,
        change: &ItemTemplateChange,
        row_type: Option<&str>,
    ) -> Result<RsxSourceEdit, String> {
        let (owner, binding) = self
            .template_owners(document, node)
            .into_iter()
            .filter_map(|owner| Some((owner, self.template_binding(owner)?)))
            .find(|(_, binding)| binding.property == property)
            .ok_or("Select a mapped Rust item or cell template")?;
        let slot = self.template_slot(&binding);
        let column = matches!(owner, TemplateOwner::Column(..));
        let source = self
            .declaration_source(&binding.declaration)
            .ok_or("The template assignment changed")?;
        let patch = match change {
            ItemTemplateChange::CreateInline if slot.can_create => {
                let row_type = syntax::type_path(row_type)?;
                let starter = if property == CellEditingTemplate {
                    r#"TextBox { placeholder_text: "Enter a value", width: 240.0, height: 34.0 }"#
                        .to_owned()
                } else {
                    let text = if column { "Cell" } else { "Item" };
                    format!("TextBlock {{ text: {text:?} }}")
                };
                let expression = format!(
                    "::ferroui_view::template::<{row_type}, _>(|_item| ::ferroui_view::rsx! {{ {starter} }})"
                );
                if column {
                    TextPatch {
                        span: SourceSpan::new(binding.insert_at, binding.insert_at),
                        replacement: format!(".{}({expression})", binding.setter),
                    }
                } else {
                    insert_field(
                        source,
                        binding.insert_at,
                        &format!("{}: {expression}", binding.setter),
                    )
                }
            }
            ItemTemplateChange::Reset if slot.can_reset => {
                let removal = binding
                    .existing
                    .and_then(|source| source.removal)
                    .ok_or("The template declaration changed")?;
                match owner {
                    TemplateOwner::Column(..) => TextPatch {
                        span: removal,
                        replacement: keep_comments(&source[removal.range()])?,
                    },
                    TemplateOwner::Control(index, _) => {
                        reset_field(source, removal, self.nodes[index].close)?
                    }
                }
            }
            _ => return Err("Choose an empty slot or a mapped template assignment to edit".into()),
        };
        let edit = RsxSourceEdit {
            source: binding.declaration,
            patches: vec![patch],
        };
        let (candidate, caret, modules) = self.declaration_draft(&edit)?;
        let next = self
            .catalog
            .project_with_modules(&candidate, caret, modules.as_ref())?;
        let next_binding = next
            .template_binding(owner)
            .ok_or("The template owner changed")?;
        let expected = match change {
            ItemTemplateChange::CreateInline => next.template_slot(&next_binding).body.is_some(),
            ItemTemplateChange::Reset => next_binding.existing.is_none(),
            _ => false,
        };
        if !expected
            || next.invocation.start != caret
            || next_binding.declaration.uri != edit.source.uri
        {
            return Err(
                "The edit could not preserve the current Rust view and template slot".into(),
            );
        }
        Ok(edit)
    }

    fn template_owners(&self, document: &Document, node: NodeId) -> Vec<TemplateOwner> {
        if let Some(index) = self.control_index(document, node) {
            return self.nodes[index]
                .templates
                .members
                .iter()
                .map(|(property, _)| TemplateOwner::Control(index, *property))
                .collect();
        }
        let Some((table, column, kind)) = self.template_column(document, node) else {
            return Vec::new();
        };
        column_template_members(kind)
            .iter()
            .map(|property| TemplateOwner::Column(table, column, *property))
            .collect()
    }

    fn template_binding(&self, owner: TemplateOwner) -> Option<TemplateSlotSource<'_>> {
        match owner {
            TemplateOwner::Control(index, property) => {
                let node = self.nodes.get(index)?;
                let fields = &node.templates;
                let &(_, setter) = fields.members.iter().find(|(name, _)| *name == property)?;
                let existing = fields
                    .authored
                    .iter()
                    .find_map(|(name, source)| (*name == property).then_some(source));
                let blocked = fields.opaque_setter
                    || (property == ItemTemplate && fields.display_member_binding);
                Some(TemplateSlotSource {
                    property,
                    existing,
                    declaration: RsxSourceLocation::local(node.span),
                    insert_at: node.open,
                    setter,
                    writable: !blocked,
                })
            }
            TemplateOwner::Column(table, column, property) => {
                let (insert_at, existing, writable, declaration) =
                    self.column_template(table, column, property)?;
                Some(TemplateSlotSource {
                    property,
                    existing,
                    declaration: declaration.clone(),
                    insert_at,
                    setter: if property == CellTemplate {
                        "cell_template"
                    } else {
                        "cell_editing_template"
                    },
                    writable,
                })
            }
        }
    }

    fn template_slot(&self, binding: &TemplateSlotSource<'_>) -> RsxItemTemplate {
        let body = binding
            .existing
            .and_then(|source| source.body.as_ref())
            .filter(|_| binding.writable)
            .filter(|body| self.valid_template_body(&body.location));
        RsxItemTemplate {
            property: binding.property,
            expression: binding.existing.map(|source| RsxSourceLocation {
                uri: binding.declaration.uri.clone(),
                span: source.expression,
            }),
            body: body.cloned(),
            can_create: binding.writable && binding.existing.is_none(),
            can_reset: binding.writable
                && body.is_some()
                && binding
                    .existing
                    .is_some_and(|source| source.removal.is_some()),
            reason: if !binding.writable {
                Some("This template assignment can be affected by another Rust builder or binding.")
            } else if binding.existing.is_some() && body.is_none() {
                Some("This template is defined by Rust. Open its source to edit it.")
            } else {
                None
            },
        }
    }

    fn valid_template_body(&self, body: &RsxSourceLocation) -> bool {
        let Some(source) = self.declaration_source(body) else {
            return false;
        };
        let companion = body
            .uri
            .as_deref()
            .and_then(|uri| self.modules?.for_module(uri));
        self.catalog
            .project_with_modules(source, body.span.start, companion.as_ref().or(self.modules))
            .is_ok_and(|projection| projection.invocation == body.span)
    }
}

fn column_template_members(kind: ColumnKind) -> &'static [ItemTemplateMember] {
    if kind == ColumnKind::Template {
        &[CellTemplate, CellEditingTemplate]
    } else {
        &[CellTemplate]
    }
}
