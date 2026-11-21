//! Source anchors for data templates whose bodies are authored as RSX.
mod syntax;
#[cfg(test)]
mod tests;

use crate::{
    RsxProjection,
    animation::syntax::span,
    blocks::keep_comments,
    columns::ColumnPosition,
    registered::Class,
    source_edits::{insert_field, reset_field},
};
use ferroui_designer_core::{Document, NodeId, SourceSpan, TextPatch};
use ferroui_designer_tools::{ColumnKind, ItemTemplateChange, ItemTemplateMember};
use syn::Expr;

use ItemTemplateMember::{
    CellEditingTemplate, CellTemplate, ItemTemplate, MenuItemTemplate, SelectionBoxItemTemplate,
};

/// One item or cell template member, in coordinates of the original Rust buffer.
pub struct RsxItemTemplate {
    pub property: ItemTemplateMember,
    pub expression: Option<SourceSpan>,
    pub body: Option<SourceSpan>,
    pub can_create: bool,
    pub can_reset: bool,
    pub reason: Option<&'static str>,
}

#[derive(Clone, Copy)]
pub(crate) struct TemplateSource {
    expression: SourceSpan,
    body: Option<SourceSpan>,
    removal: Option<SourceSpan>,
}

impl TemplateSource {
    pub(crate) fn read(expression: &Expr, removal: Option<SourceSpan>) -> Self {
        Self {
            expression: span(expression),
            body: syntax::template_body(expression),
            removal,
        }
    }
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

    pub(crate) fn record_call(
        &mut self,
        setter: &str,
        field: Option<SourceSpan>,
        argument: Option<&Expr>,
    ) {
        if self.member(setter).is_none() {
            return;
        }
        if let Some(argument) = argument {
            self.record(setter, argument, field);
        } else {
            self.opaque_setter = true;
        }
    }

    fn member(&self, setter: &str) -> Option<ItemTemplateMember> {
        self.members
            .iter()
            .find_map(|(property, name)| (*name == setter).then_some(*property))
    }

    pub(crate) fn record(
        &mut self,
        setter: &str,
        expression: &Expr,
        field: Option<SourceSpan>,
    ) -> bool {
        let Some(property) = self.member(setter) else {
            return false;
        };
        if let Some((_, previous)) = self.authored.iter_mut().find(|(name, _)| *name == property) {
            *previous = TemplateSource {
                expression: span(expression),
                body: None,
                removal: None,
            };
        } else {
            self.authored
                .push((property, TemplateSource::read(expression, field)));
        }
        true
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

struct TemplateSlotSource {
    property: ItemTemplateMember,
    existing: Option<TemplateSource>,
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
        let (owner, binding) = self
            .template_owners(document, node)
            .into_iter()
            .filter_map(|owner| Some((owner, self.template_binding(owner)?)))
            .find(|(_, binding)| binding.property == property)
            .ok_or("Select a mapped Rust item or cell template")?;
        let slot = self.template_slot(&binding);
        let column = matches!(owner, TemplateOwner::Column(..));
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
                        self.source,
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
                        replacement: keep_comments(&self.source[removal.range()])?,
                    },
                    TemplateOwner::Control(index, _) => {
                        reset_field(self.source, removal, self.nodes[index].close)?
                    }
                }
            }
            _ => return Err("Choose an empty slot or a mapped template assignment to edit".into()),
        };
        let candidate = TextPatch::apply_all(self.source, vec![patch.clone()])
            .map_err(|error| error.to_string())?;
        let modules = self.draft_module_sources(&candidate)?;
        let next = self.catalog.project_with_modules(&candidate, self.invocation.start, modules.as_ref())?;
        let next_binding = next
            .template_binding(owner)
            .ok_or("The template owner changed")?;
        let expected = match change {
            ItemTemplateChange::CreateInline => next.template_slot(&next_binding).body.is_some(),
            ItemTemplateChange::Reset => next_binding.existing.is_none(),
            _ => false,
        };
        if !expected || next.invocation.start != self.invocation.start {
            return Err(
                "The edit could not preserve the current Rust view and template slot".into(),
            );
        }
        Ok(vec![patch])
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
        let properties: &[_] = if kind == ColumnKind::Template {
            &[CellTemplate, CellEditingTemplate]
        } else {
            &[CellTemplate]
        };
        properties
            .iter()
            .map(|property| TemplateOwner::Column(table, column, *property))
            .collect()
    }

    fn template_binding(&self, owner: TemplateOwner) -> Option<TemplateSlotSource> {
        match owner {
            TemplateOwner::Control(index, property) => {
                let node = self.nodes.get(index)?;
                let fields = &node.templates;
                let &(_, setter) = fields.members.iter().find(|(name, _)| *name == property)?;
                let existing = fields
                    .authored
                    .iter()
                    .find_map(|(name, source)| (*name == property).then_some(*source));
                let blocked = fields.opaque_setter
                    || (property == ItemTemplate && fields.display_member_binding);
                Some(TemplateSlotSource {
                    property,
                    existing,
                    insert_at: node.open,
                    setter,
                    writable: !blocked,
                })
            }
            TemplateOwner::Column(table, column, property) => {
                let (insert_at, existing, writable) =
                    self.column_template(table, column, property)?;
                Some(TemplateSlotSource {
                    property,
                    existing,
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

    fn template_slot(&self, binding: &TemplateSlotSource) -> RsxItemTemplate {
        let body = binding
            .existing
            .and_then(|source| source.body)
            .filter(|_| binding.writable)
            .filter(|body| self.blocks.contains(body))
            .filter(|body| {
                self.catalog
                    .project_with_modules(self.source, body.start, self.modules)
                    .is_ok_and(|projection| projection.invocation == *body)
            });
        RsxItemTemplate {
            property: binding.property,
            expression: binding.existing.map(|source| source.expression),
            body,
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
}
