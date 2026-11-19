//! Authored source mappings for the inline RSX visual-state builder.

mod markup;
mod syntax;

use crate::animation::{RsxAnimationIssue, ValueSource};
use crate::tree::Element;
use ferroui_designer_animation::{native_key_time, Easing, VisualStateGroup};
use ferroui_designer_core::{NodeId, SourceSpan, TextPatch};
use ferroui_designer_language::{LineIndex, Range};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use syn::Expr;

pub(crate) struct ProjectedStates {
    pub(crate) anchor: String,
    pub(crate) markup: Vec<String>,
    pub(crate) groups: Vec<RsxStateGroup>,
}

#[derive(Clone, Debug)]
pub struct RsxStateGroup {
    pub model: VisualStateGroup,
    pub declaration: SourceSpan,
    routes: Vec<RouteSource>,
    values: BTreeMap<(String, String, String), ValueSource>,
}

#[derive(Clone, Debug)]
struct RouteSource {
    declaration: SourceSpan,
    milliseconds: SourceSpan,
    explicit_u64: bool,
    easing: SourceSpan,
}

#[derive(Serialize)]
pub struct RsxStateSource {
    owner: NodeId,
    group: String,
    range: Range,
    transitions: Vec<RsxTransitionSource>,
}

#[derive(Serialize)]
struct RsxTransitionSource {
    from: Option<String>,
    to: String,
    range: Range,
}

impl RsxStateGroup {
    pub fn source_ranges(&self, owner: NodeId, index: &LineIndex) -> Option<RsxStateSource> {
        Some(RsxStateSource {
            owner,
            group: self.model.name.clone(),
            range: index.range(self.declaration).ok()?,
            transitions: self
                .model
                .transitions
                .iter()
                .zip(&self.routes)
                .filter_map(|(route, source)| {
                    Some(RsxTransitionSource {
                        from: route.from.clone(),
                        to: route.to.clone(),
                        range: index.range(source.declaration).ok()?,
                    })
                })
                .collect(),
        })
    }

    pub fn transition_span(&self, from: Option<&str>, to: &str) -> Option<SourceSpan> {
        let index = self
            .model
            .transitions
            .iter()
            .position(|route| route.from.as_deref() == from && route.to == to)?;
        Some(self.routes[index].declaration)
    }

    /// Edit only the existing typed values and route fields in the accepted declaration.
    pub fn patch_model(&self, model: &VisualStateGroup) -> Result<Vec<TextPatch>, String> {
        model.validate().map_err(|error| error.to_string())?;
        let shape_error =
            || "Add or remove Rust state declarations in the source editor".to_owned();
        if model.name != self.model.name
            || model.states.len() != self.model.states.len()
            || model.transitions.len() != self.model.transitions.len()
        {
            return Err(shape_error());
        }
        let mut patches = Vec::new();
        for state in &self.model.states {
            let next = model
                .states
                .iter()
                .find(|next| next.name == state.name && next.setters.len() == state.setters.len())
                .ok_or_else(shape_error)?;
            for setter in &state.setters {
                let value = next
                    .setters
                    .iter()
                    .find(|item| item.target == setter.target && item.property == setter.property)
                    .ok_or_else(shape_error)?;
                if value.value != setter.value {
                    self.values[&(
                        state.name.clone(),
                        setter.target.clone(),
                        setter.property.clone(),
                    )]
                        .plan(&value.value, &mut patches)?;
                }
            }
        }
        for (route, source) in self.model.transitions.iter().zip(&self.routes) {
            let next = model
                .transitions
                .iter()
                .find(|next| next.from == route.from && next.to == route.to)
                .ok_or_else(shape_error)?;
            if next.duration != route.duration {
                let duration = native_key_time(next.duration).map_err(|error| error.to_string())?;
                let milliseconds = (duration * 1000.0).round() as u64;
                if duration != milliseconds as f64 / 1000.0 {
                    return Err(
                        "A mapped Rust transition uses a whole number of milliseconds".into(),
                    );
                }
                let suffix = if source.explicit_u64 { "u64" } else { "" };
                patches.push(TextPatch {
                    span: source.milliseconds,
                    replacement: format!("{milliseconds}{suffix}"),
                });
            }
            if next.easing != route.easing {
                if !mapped_easing(&next.easing) {
                    return Err(
                        "Choose a native easing name or cubic Bézier for this Rust route".into(),
                    );
                }
                patches.push(TextPatch {
                    span: source.easing,
                    replacement: format!("{:?}", next.easing.to_xaml()),
                });
            }
        }
        patches.sort_by_key(|edit| edit.span.start);
        Ok(patches)
    }
}

fn mapped_easing(easing: &Easing) -> bool {
    !matches!(easing, Easing::Spring { .. })
}

pub(crate) fn project(
    fields: &[&Expr],
    owner: &Element,
    index: usize,
    catalog: &crate::RsxCatalog,
) -> (Option<ProjectedStates>, Vec<RsxAnimationIssue>) {
    let anchor = markup::anchor(owner, index);
    let mut result = ProjectedStates {
        anchor,
        markup: Vec::new(),
        groups: Vec::new(),
    };
    let mut issues = Vec::new();
    let mut names = BTreeSet::new();
    for field in fields {
        let Expr::Array(items) = field else {
            issues.push(issue(
                field,
                "Use an inline array to map Rust visual state groups.",
            ));
            continue;
        };
        for expr in &items.elems {
            let Some(group) = syntax::group(expr, catalog) else {
                issues.push(issue(
                    expr,
                    "This Rust state group contains an unmapped declaration or value.",
                ));
                continue;
            };
            if !names.insert(group.model.name.clone()) {
                issues.push(issue(
                    expr,
                    "State group names must be unique on the owner.",
                ));
                continue;
            }
            if let Some(xml) = markup::group_markup(&group.model, owner, &result.anchor) {
                result.markup.push(xml);
                result.groups.push(group);
            } else {
                issues.push(issue(
                    expr,
                    "This Rust state group needs unambiguous targets and native values.",
                ));
            }
        }
    }
    ((!result.groups.is_empty()).then_some(result), issues)
}

fn issue(expr: &Expr, reason: &'static str) -> RsxAnimationIssue {
    RsxAnimationIssue {
        declaration: crate::animation::syntax::span(expr),
        reason,
    }
}
