//! Capture group settings when the Rust builder is constructed.
use super::{string_literal, transparent, BuildScript, Builder, DocumentKey, MacroSyntax, Site};
use syn::{Expr, LitStr};

#[derive(Clone)]
pub(super) enum GroupInputs<'source> {
    Discovery,
    Inline(Vec<DocumentKey<'source>>),
    Opaque,
}

#[derive(Clone)]
pub(super) struct GroupSpec<'source> {
    module: Option<&'source LitStr>,
    pub(super) inputs: GroupInputs<'source>,
    exported: bool,
}

impl<'source> BuildScript<'source> {
    pub(super) fn group(
        &self,
        expression: &'source Expr,
        macros: &MacroSyntax,
    ) -> Option<GroupSpec<'source>> {
        let expression = transparent(expression);
        match expression {
            Expr::Path(path) => self.binding(path)?.group.clone(),
            Expr::Call(call) if self.builder(expression) == Some(Builder::Group) => {
                Some(GroupSpec {
                    module: call
                        .args
                        .first()
                        .and_then(|module| string_literal(self.value(module))),
                    inputs: GroupInputs::Discovery,
                    exported: true,
                })
            }
            Expr::MethodCall(call) => {
                let mut group = self.group(&call.receiver, macros)?;
                match call.method.to_string().as_str() {
                    "internal" => group.exported = false,
                    "documents" => {
                        group.inputs = call
                            .args
                            .first()
                            .and_then(|value| self.keys(value, macros))
                            .map_or(GroupInputs::Opaque, GroupInputs::Inline)
                    }
                    "document_files" => group.inputs = GroupInputs::Opaque,
                    _ => {}
                }
                Some(group)
            }
            _ => None,
        }
    }

    pub(super) fn compile_group(
        &self,
        expression: &'source Expr,
        macros: &MacroSyntax,
    ) -> Vec<Site<'source>> {
        let Some(group) = self.group(expression, macros) else {
            return Vec::new();
        };
        match (group.module, group.inputs) {
            (_, GroupInputs::Discovery) => vec![Site::Discovery],
            (Some(module), GroupInputs::Inline(documents)) => documents
                .into_iter()
                .map(|key| Site::Registration(module, key, group.exported))
                .collect(),
            _ => Vec::new(),
        }
    }
}
