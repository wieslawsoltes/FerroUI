//! Parse inline StyleBuilder calls and retain the source of omitted Rust expressions.
use super::{Deletion, RsxStyle, RsxStyleField, RsxStyleIssue, RsxStyleResource, RsxStyleSetter};
use super::{fields::SetterValue, resources::resource_key};
use crate::animation::{program_comment::AnimationAttachment, syntax as rust, RsxAnimationIssue};
use crate::{tree::MAX_DEPTH, RsxCatalog};
use crate::local_rust::LocalRust;
use ferroui_designer_core::SourceSpan;
use ferroui_markup::markup::parsers::selector_grammar::{
    SelectorGrammar, SelectorOperand, SelectorSyntax,
};
use std::collections::BTreeMap;
use syn::{Expr, Lit};

#[derive(Default)]
pub(crate) struct ProjectedStyles {
    pub(crate) styles: Vec<RsxStyle>,
    pub(crate) issues: Vec<RsxStyleIssue>,
    pub(crate) animation_issues: Vec<RsxAnimationIssue>,
}

#[derive(Debug)]
pub(crate) struct PreviewSelector {
    pub(crate) value: String,
    pub(crate) namespaces: BTreeMap<String, String>,
    needs_target_type: bool,
}

pub(crate) fn projected_styles(
    source: &str,
    value: &Expr,
    catalog: &RsxCatalog,
    bindings: &LocalRust,
) -> ProjectedStyles {
    let mut result = ProjectedStyles::default();
    let Expr::Array(array) = value else {
        result.issue(
            rust::span(value),
            "Reveal the computed Rust style collection",
        );
        return result;
    };
    for pair in array.elems.pairs() {
        let expression = pair.value();
        let deletion = Deletion {
            span: rust::span(expression),
            comma: pair.punct().map(|token| rust::span(*token)),
        };
        if let Some(style) = result.style(source, expression, catalog, bindings, deletion, 0) {
            result.styles.push(style);
        } else {
            result.issue(
                rust::span(expression),
                "Reveal the computed Rust style or selector",
            );
        }
    }
    result
}

impl ProjectedStyles {
    fn issue(&mut self, source: SourceSpan, reason: &'static str) {
        self.issues.push(RsxStyleIssue { source, reason });
    }

    fn style(
        &mut self,
        source: &str,
        expression: &Expr,
        catalog: &RsxCatalog,
        bindings: &LocalRust,
        deletion: Deletion,
        depth: usize,
    ) -> Option<RsxStyle> {
        if depth > MAX_DEPTH {
            return None;
        }
        let (factory, methods) = rust::chain(expression)?;
        let path = bindings.path(&factory.func)?;
        if !matches!(path.as_str(), "ferroui_view::style" | "ferroui_view::prelude::style") || factory.args.len() != 1 {
            return None;
        }
        let Expr::Lit(selector) = &factory.args[0] else {
            return None;
        };
        let Lit::Str(selector) = &selector.lit else {
            return None;
        };
        let declaration = rust::span(expression);
        let value = selector.value();
        let selector_span = rust::span(selector);
        let preview_selector = preview_selector(&value, catalog, depth > 0);
        let needs_target_type = preview_selector
            .as_ref()
            .is_some_and(|selector| selector.needs_target_type);
        let issue_count = self.issues.len();
        let animation_issue_count = self.animation_issues.len();
        if preview_selector.is_none() {
            self.issue(
                selector_span,
                "The Rust selector needs a type mapping for the native projection",
            );
        }
        let lone_style =
            (depth == 0 && methods.len() == 1).then_some((declaration, deletion.comma));
        let mut setters = Vec::new();
        let mut resources = Vec::new();
        let mut children = Vec::new();
        let mut clips = Vec::new();
        for call in methods {
            let method = rust::method_suffix(call);
            if (call.method == "set" || call.method == "set_resource") && call.args.len() == 2 {
                let expression = &call.args[1];
                let field = RsxStyleField::from_expression(&call.args[0], catalog, bindings);
                let value = if call.method == "set_resource" {
                    field.as_ref().and_then(|_| resource_key(expression, bindings)).map(SetterValue::Resource)
                } else {
                    field.as_ref().and_then(|field| field.read(source, expression))
                };
                if value.is_none() {
                    self.issue(
                        rust::span(expression),
                        "Reveal this computed or unregistered Rust setter",
                    );
                }
                setters.push(RsxStyleSetter {
                    declaration: method,
                    source: rust::span(expression),
                    field,
                    method: rust::span(&call.method),
                    value,
                });
            } else if call.method == "resource" || call.method == "theme_resource" {
                let resource = RsxStyleResource::read(call, bindings);
                if resource.as_ref().is_none_or(|resource| resource.color.is_none() || resource.layer.is_none()) {
                    self.issue(method, "Reveal this computed or unmapped Rust resource");
                }
                resources.extend(resource);
            } else if let Some(nested) = rust::argument(call, "nested") {
                let deletion = Deletion {
                    span: method,
                    comma: None,
                };
                if let Some(child) = self.style(source, nested, catalog, bindings, deletion, depth + 1) {
                    children.push(child);
                } else {
                    self.issue(rust::span(nested), "Reveal the computed nested Rust style");
                }
            } else if let Some(animation) = rust::argument(call, "animation") {
                if let Some(mut clip) = rust::clip(animation, catalog) {
                    if needs_target_type {
                        rust::scope_untyped_clip(&mut clip, catalog);
                    }
                    let attachment =
                        AnimationAttachment::read(source, call, rust::span(animation), lone_style);
                    if let Some(identity) = attachment.identity() {
                        identity.annotate_xaml(&mut clip.xaml);
                    }
                    clip.attachment = Some(attachment);
                    clips.push(clip);
                } else {
                    self.animation_issues.push(RsxAnimationIssue {
                        declaration: rust::span(animation),
                        reason: "This clip contains a computed expression or a Rust builder form without a source mapping.",
                    });
                }
            } else {
                self.issue(method, "Reveal the unrecognized Rust style builder call");
            }
        }
        Some(RsxStyle {
            declaration,
            selector: value,
            selector_span,
            setters,
            resources,
            children,
            clips,
            preview_selector,
            deletion,
            removable: issue_count == self.issues.len()
                && animation_issue_count == self.animation_issues.len(),
        })
    }
}

pub(super) fn validate_selector(value: &str, nested: bool) -> Result<(), String> {
    if !rust::valid_selector(value) {
        return Err("Enter a native Rust style selector".into());
    }
    let syntax = SelectorGrammar::parse(value).map_err(|error| error.to_string())?;
    if syntax.is_empty()
        || !nested
            && syntax
                .iter()
                .any(|node| matches!(node, SelectorSyntax::Nesting))
    {
        return Err("Use the nesting selector inside a nested style".into());
    }
    Ok(())
}

fn preview_selector(value: &str, catalog: &RsxCatalog, nested: bool) -> Option<PreviewSelector> {
    validate_selector(value, nested).ok()?;
    let (syntax, operands) = SelectorGrammar::operand_ranges(value).ok()?;
    let needs_target_type = syntax
        .split(|node| matches!(node, SelectorSyntax::Comma))
        .any(|branch| {
            !branch
                .rsplit(|node| {
                    matches!(
                        node,
                        SelectorSyntax::Child
                            | SelectorSyntax::Descendant
                            | SelectorSyntax::Template
                    )
                })
                .next()
                .is_some_and(|subject| {
                    subject.iter().any(|node| {
                        matches!(
                            node,
                            SelectorSyntax::OfType { .. } | SelectorSyntax::Is { .. }
                        )
                    })
                })
        });
    let mut namespaces = BTreeMap::new();
    let mut projected = value.to_owned();
    for operand in operands.into_iter().rev() {
        let range = match operand {
            SelectorOperand::Type(range)
            | SelectorOperand::AttachedProperty { owner: range, .. } => range,
            SelectorOperand::Property(_) => continue,
        };
        let name = &value[range.clone()];
        // The Rust resolver matches type names independently of a textual
        // prefix. The XAML compiler needs the actual project namespace.
        let start = value[..range.start]
            .strip_suffix('|')
            .map_or(range.start, |before| {
                before
                    .char_indices()
                    .rev()
                    .find(|(_, ch)| !ch.is_alphanumeric() && *ch != '_' && *ch != '-')
                    .map_or(0, |(index, ch)| index + ch.len_utf8())
            });
        let replacement = if let Some(uri) = catalog.style_namespace(name)? {
            let alias = format!("rsxStyle{}", namespaces.len());
            namespaces.insert(alias.clone(), uri.to_owned());
            format!("{alias}|{name}")
        } else {
            name.to_owned()
        };
        projected.replace_range(start..range.end, &replacement);
    }
    Some(PreviewSelector {
        value: projected,
        namespaces,
        needs_target_type,
    })
}
