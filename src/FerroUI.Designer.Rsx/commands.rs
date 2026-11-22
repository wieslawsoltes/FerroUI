//! Source-backed builder fields whose Rust expressions remain in the authored view.
pub(crate) mod schema;
#[cfg(test)]
mod tests;

use crate::{
    animation::syntax::span,
    blocks::keep_comments,
    source_edits::{insert_field, merge_insertions, reset_field},
    RsxNode, RsxProjection,
};
use ferroui_designer_core::{Document, NodeId, SourceSpan, TextPatch};
use schema::CommandMember;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use syn::Expr;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RsxCommandEdit {
    pub property: String,
    pub command: RsxCommandSelection,
    pub parameter: RsxCommandSelection,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum RsxCommandSelection {
    Keep,
    Clear,
    Set { expression: String },
}

pub struct RsxCommandSlot {
    pub binding: RsxCommandValue,
    pub parameter: Option<RsxCommandValue>,
}

pub struct RsxCommandValue {
    pub property: String,
    pub type_name: String,
    pub expression: Option<SourceSpan>,
    pub editable: bool,
}

#[derive(Clone, Copy)]
enum CommandSource {
    Field {
        expression: SourceSpan,
        field: SourceSpan,
    },
    Unavailable(Option<SourceSpan>),
}

impl CommandSource {
    fn expression(self) -> Option<SourceSpan> {
        match self {
            Self::Field { expression, .. } => Some(expression),
            Self::Unavailable(expression) => expression,
        }
    }
}

#[derive(Default)]
pub(crate) struct CommandFields {
    authored: BTreeMap<String, CommandSource>,
    indirect: bool,
}

impl CommandFields {
    pub(crate) fn note_setter(&mut self, setter: &str) {
        self.indirect |= matches!(setter, "prop" | "set" | "with");
    }

    pub(crate) fn record(
        &mut self,
        property: &str,
        expression: Option<SourceSpan>,
        field: Option<SourceSpan>,
    ) {
        self.authored
            .entry(property.into())
            .and_modify(|previous| {
                *previous = CommandSource::Unavailable(expression.or(previous.expression()));
            })
            .or_insert(match expression.zip(field) {
                Some((expression, field)) => CommandSource::Field { expression, field },
                None => CommandSource::Unavailable(expression),
            });
    }

    fn source(&self, member: &CommandMember) -> Option<CommandSource> {
        self.authored.get(&member.property).copied()
    }

    fn editable(&self, member: &CommandMember) -> bool {
        !self.indirect && !matches!(self.source(member), Some(CommandSource::Unavailable(_)))
    }

    fn value(&self, member: &CommandMember) -> RsxCommandValue {
        RsxCommandValue {
            property: member.property.clone(),
            type_name: member.type_name.clone(),
            expression: self.source(member).and_then(CommandSource::expression),
            editable: self.editable(member),
        }
    }
}

impl RsxNode {
    pub fn supports_commands(&self) -> bool {
        !self.class.commands().is_empty()
    }
}

impl RsxProjection<'_> {
    pub fn command_slots(&self, document: &Document, node: NodeId) -> Vec<RsxCommandSlot> {
        let Some(index) = self.control_index(document, node) else {
            return Vec::new();
        };
        self.nodes[index]
            .class
            .commands()
            .iter()
            .map(|slot| RsxCommandSlot {
                binding: self.nodes[index].commands.value(&slot.binding),
                parameter: slot
                    .parameter
                    .as_ref()
                    .map(|member| self.nodes[index].commands.value(member)),
            })
            .collect()
    }

    pub fn plan_command(
        &self,
        document: &Document,
        owner: NodeId,
        change: &RsxCommandEdit,
    ) -> Result<Vec<TextPatch>, String> {
        let index = self
            .control_index(document, owner)
            .ok_or("Choose an authored RSX control")?;
        let node = &self.nodes[index];
        let slot = node
            .class
            .commands()
            .iter()
            .find(|slot| slot.binding.property == change.property)
            .ok_or("Choose a mapped Rust command property")?;
        let mut changes = vec![(&slot.binding, &change.command)];
        if !matches!(change.parameter, RsxCommandSelection::Keep) {
            let parameter = slot
                .parameter
                .as_ref()
                .ok_or("This Rust command has no mapped parameter")?;
            changes.push((parameter, &change.parameter));
        }

        let mut edits = Vec::new();
        let mut expected = Vec::new();
        for (member, selection) in changes {
            if matches!(selection, RsxCommandSelection::Keep) {
                continue;
            }
            if !node.commands.editable(member) {
                return Err(format!(
                    "Edit {} in Rust source to preserve its builder assignments",
                    member.property
                ));
            }
            let from = node.commands.source(member);
            let (edit, value) = plan_field(self.source, node, member, from, selection)?;
            edits.extend(edit);
            expected.push((member, value));
        }
        if edits.is_empty() {
            return Ok(edits);
        }
        merge_insertions(&mut edits);
        let candidate =
            TextPatch::apply_all(self.source, edits.clone()).map_err(|error| error.to_string())?;
        let modules = self.draft_module_sources(&candidate)?;
        let next = self.catalog.project_with_modules(
            &candidate,
            self.invocation.start,
            modules.as_ref(),
        )?;
        let owner = next
            .nodes
            .get(index)
            .filter(|next_node| {
                next.invocation.start == self.invocation.start
                    && next.nodes.len() == self.nodes.len()
                    && next_node.span.start == node.span.start
                    && next_node.class == node.class
            })
            .ok_or("The command edit changed the selected RSX structure")?;
        for (member, value) in expected {
            let actual = owner
                .commands
                .source(member)
                .and_then(CommandSource::expression)
                .map(|range| &candidate[range.range()]);
            if !owner.commands.editable(member) || actual != value {
                return Err(format!(
                    "The Rust expression did not round-trip through {}",
                    member.property
                ));
            }
        }
        Ok(edits)
    }
}

fn plan_field<'a>(
    source: &str,
    node: &RsxNode,
    member: &CommandMember,
    from: Option<CommandSource>,
    selection: &'a RsxCommandSelection,
) -> Result<(Option<TextPatch>, Option<&'a str>), String> {
    let field = match from {
        Some(CommandSource::Field { expression, field }) => Some((expression, field)),
        _ => None,
    };
    let text = match selection {
        RsxCommandSelection::Keep => return Ok((None, None)),
        RsxCommandSelection::Clear => {
            let patch = field
                .map(|(_, field)| reset_field(source, field, node.close))
                .transpose()?;
            return Ok((patch, None));
        }
        RsxCommandSelection::Set { expression } => expression.trim(),
    };
    let parsed = syn::parse_str::<Expr>(text)
        .map_err(|error| format!("Invalid Rust expression: {error}"))?;
    let operand = span(&parsed);
    let expected = text
        .get(operand.range())
        .ok_or("Invalid Rust expression range")?;
    // A closing delimiter or separator must survive a trailing line comment.
    let end = if text[operand.end..].contains("//") {
        if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        }
    } else {
        ""
    };
    let patch = if let Some((expression, field)) = field {
        let previous = &source[expression.range()];
        if previous == text {
            return Ok((None, Some(expected)));
        }
        let comments = if keep_comments(text)?.is_empty() {
            keep_comments(previous)?
        } else {
            String::new()
        };
        let prefix = if expression == field {
            format!("{previous}: ")
        } else {
            String::new()
        };
        TextPatch {
            span: expression,
            replacement: format!("{prefix}{comments}{text}{end}"),
        }
    } else {
        insert_field(
            source,
            node.open,
            &format!("{}: {text}{end}", member.setter),
        )
    };
    Ok((Some(patch), Some(expected)))
}
