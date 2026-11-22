//! Authored source mappings for the inline RSX visual-state builder.

mod authoring;
mod markup;
mod syntax;

use crate::animation::{RsxAnimationIssue, ValueSource};
use crate::tree::Element;
use ferroui_designer_animation::{
    native_key_time, Easing, StateSetter, StateTransition, VisualStateGroup,
};
use ferroui_designer_core::{NodeId, SourceSpan, TextPatch};
use ferroui_designer_language::{LineIndex, Range};
use serde::Serialize;
use std::collections::BTreeSet;
use syn::{punctuated::Pair, Expr};

pub(crate) struct ProjectedStates {
    pub(crate) anchor: String,
    pub(crate) markup: Vec<String>,
    pub(crate) groups: Vec<RsxStateGroup>,
}

#[derive(Clone, Debug)]
pub struct RsxStateGroup {
    model: VisualStateGroup,
    pub declaration: SourceSpan,
    comma: Option<SourceSpan>,
    routes: Vec<RouteSource>,
    states: Vec<StateSource>,
}

#[derive(Clone, Debug)]
struct StateSource {
    suffix: SourceSpan,
    body: SourceSpan,
    setters: Vec<SetterSource>,
}

#[derive(Clone, Debug)]
struct SetterSource {
    suffix: SourceSpan,
    value: ValueSource,
}

#[derive(Clone, Debug)]
struct RouteSource {
    suffix: SourceSpan,
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
    pub fn name(&self) -> &str {
        &self.model.name
    }

    pub(crate) fn model(&self) -> &VisualStateGroup {
        &self.model
    }

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

    /// Edit the typed values of an existing group while preserving its source shape.
    /// Use the owning projection to plan edits that add or remove declarations.
    pub fn patch_model(&self, model: &VisualStateGroup) -> Result<Vec<TextPatch>, String> {
        model.validate().map_err(|error| error.to_string())?;
        if model.name != self.model.name
            || model.states.len() != self.model.states.len()
            || model.transitions.len() != self.model.transitions.len()
            || self.model.states.iter().any(|state| {
                !model.states.iter().any(|next| {
                    next.name == state.name
                        && next.setters.len() == state.setters.len()
                        && next.setters.iter().all(|next| {
                            state.setters.iter().any(|setter| same_setter(setter, next))
                        })
                })
            })
            || self
                .model
                .transitions
                .iter()
                .any(|route| !model.transitions.iter().any(|next| same_route(route, next)))
        {
            return Err(
                "Use the active Rust projection to change the structure of this state group".into(),
            );
        }
        let mut patches = Vec::new();
        self.patch_values(model, &mut patches)?;
        patches.sort_by_key(|edit| edit.span.start);
        Ok(patches)
    }

    fn patch_values(
        &self,
        model: &VisualStateGroup,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        for (state, source) in self.model.states.iter().zip(&self.states) {
            let Some(next) = model.states.iter().find(|next| next.name == state.name) else {
                continue;
            };
            for (setter, source) in state.setters.iter().zip(&source.setters) {
                if let Some(next) = next
                    .setters
                    .iter()
                    .find(|next| same_setter(setter, next) && next.value != setter.value)
                {
                    source.value.plan(&next.value, patches)?;
                }
            }
        }
        for (route, source) in self.model.transitions.iter().zip(&self.routes) {
            let Some(next) = model
                .transitions
                .iter()
                .find(|next| same_route(route, next))
            else {
                continue;
            };
            if next.duration != route.duration {
                let milliseconds = milliseconds(next.duration)?;
                let suffix = if source.explicit_u64 { "u64" } else { "" };
                patches.push(TextPatch {
                    span: source.milliseconds,
                    replacement: format!("{milliseconds}{suffix}"),
                });
            }
            if next.easing != route.easing {
                patches.push(TextPatch {
                    span: source.easing,
                    replacement: easing_literal(&next.easing)?,
                });
            }
        }
        Ok(())
    }
}

fn same_setter(first: &StateSetter, second: &StateSetter) -> bool {
    first.property == second.property && first.target == second.target
}

fn same_route(first: &StateTransition, second: &StateTransition) -> bool {
    first.from == second.from && first.to == second.to
}

fn milliseconds(duration: f64) -> Result<u64, String> {
    let duration = native_key_time(duration).map_err(|error| error.to_string())?;
    let milliseconds = (duration * 1000.0).round() as u64;
    if duration != milliseconds as f64 / 1000.0 {
        return Err("A mapped Rust transition uses a whole number of milliseconds".into());
    }
    Ok(milliseconds)
}

fn mapped_easing(easing: &Easing) -> bool {
    !matches!(easing, Easing::Spring { .. })
}

fn easing_literal(easing: &Easing) -> Result<String, String> {
    if !mapped_easing(easing) {
        return Err("Choose a native easing name or cubic Bézier for this Rust route".into());
    }
    Ok(format!("{:?}", easing.to_xaml()))
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
        let mut previous_comma = None;
        for pair in items.elems.pairs() {
            let (expr, comma) = match pair {
                Pair::Punctuated(expr, punctuation) => {
                    (expr, Some(crate::animation::syntax::span(punctuation)))
                }
                Pair::End(expr) => (expr, None),
            };
            let removal_comma = comma.or(previous_comma);
            previous_comma = comma;
            let Some(mut group) = syntax::group(expr, catalog) else {
                issues.push(issue(
                    expr,
                    "This Rust state group contains an unmapped declaration or value.",
                ));
                continue;
            };
            group.comma = removal_comma;
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
