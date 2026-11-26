//! Authored native behavior declarations and their original Rust token locations.
mod edits;
mod render;
mod syntax;
#[cfg(test)]
mod tests;

use crate::animation::syntax::span;
use crate::local_rust::LocalRust;
use crate::{source_array::ArraySlot, RsxProjection};
use ferroui_designer_animation::{BehaviorAction, BehaviorModel, BehaviorTrigger};
use ferroui_designer_core::SourceSpan;
use std::collections::BTreeMap;
use syn::{punctuated::Pair, Expr, ExprArray};

pub use edits::RsxBehaviorEdit;
pub(crate) use render::{declaration, normalize};

pub(crate) struct BehaviorFields {
    slot: ArraySlot,
    pub(crate) entries: Vec<RsxBehavior>,
}

impl BehaviorFields {
    pub(crate) fn new(available: bool) -> Self {
        Self {
            slot: if available {
                ArraySlot::Missing
            } else {
                ArraySlot::Unavailable
            },
            entries: Vec::new(),
        }
    }

    pub(crate) fn read(&mut self, value: Option<&Expr>, rust: &LocalRust) -> bool {
        let Some(Expr::Array(array)) = value.filter(|_| matches!(self.slot, ArraySlot::Missing))
        else {
            self.slot = ArraySlot::Unavailable;
            self.entries.clear();
            return false;
        };
        self.slot = ArraySlot::at(array);
        self.entries = members(array)
            .filter_map(|(item, comma)| RsxBehavior::read(item, comma, rust))
            .collect();
        self.entries.len() == array.elems.len()
    }

    pub(crate) fn retain_resolved_targets(&mut self, names: &BTreeMap<&str, usize>) -> bool {
        let available = |reference: &Option<String>| {
            reference.as_deref().is_none_or(|value| {
                syntax::control_name(value).is_some_and(|name| names.get(name) == Some(&1))
            })
        };
        let old_count = self.entries.len();
        self.entries.retain(|behavior| {
            if let BehaviorTrigger::Event { source, .. } = &behavior.model.trigger {
                if !available(source) {
                    return false;
                }
            }
            behavior.model.actions.iter().all(|action| match action {
                BehaviorAction::ChangeProperty { target, .. }
                | BehaviorAction::CallMethod { target, .. }
                | BehaviorAction::AddClass { target, .. }
                | BehaviorAction::RemoveClass { target, .. } => available(target),
                _ => true,
            })
        });
        self.entries.len() == old_count
    }
}

pub struct RsxBehavior {
    model: BehaviorModel,
    trigger: syntax::Arguments,
    actions: Vec<ActionSource>,
    slot: ArraySlot,
    array: SourceSpan,
    comma: Option<SourceSpan>,
    pub(crate) markup: String,
    pub(crate) action_offsets: Vec<usize>,
}

struct ActionSource {
    call: syntax::Arguments,
    comma: Option<SourceSpan>,
}

impl RsxBehavior {
    fn read(expression: &Expr, comma: Option<SourceSpan>, rust: &LocalRust) -> Option<Self> {
        let (trigger, source, list) = syntax::trigger(expression, rust)?;
        let mut model = BehaviorModel {
            id: None,
            owner: 1,
            trigger,
            actions: Vec::new(),
        };
        let mut actions = Vec::new();
        for (expression, comma) in members(list) {
            let (action, call) = syntax::action(expression, rust)?;
            model.actions.push(action);
            actions.push(ActionSource { call, comma });
        }
        let (markup, action_offsets) = model.to_xaml_with_action_offsets().ok()?;
        Some(Self {
            model,
            trigger: source,
            actions,
            slot: ArraySlot::at(list),
            array: span(list),
            comma,
            markup,
            action_offsets,
        })
    }

    /// The original Rust constructor, including its actions.
    pub fn span(&self) -> SourceSpan {
        self.trigger.span
    }

    pub fn model(&self) -> &BehaviorModel {
        &self.model
    }

    pub fn action_span(&self, index: usize) -> Option<SourceSpan> {
        self.actions.get(index).map(|action| action.call.span)
    }
}

impl RsxProjection<'_> {
    pub fn has_projected_behaviors(&self) -> bool {
        self.nodes
            .iter()
            .any(|node| !node.behaviors.entries.is_empty())
    }

    pub fn behaviors(&self, owner: usize) -> &[RsxBehavior] {
        self.nodes
            .get(owner)
            .map_or(&[], |node| node.behaviors.entries.as_slice())
    }

    pub fn can_attach_behavior(&self, owner: usize) -> bool {
        self.nodes
            .get(owner)
            .is_some_and(|node| !matches!(node.behaviors.slot, ArraySlot::Unavailable))
    }
}

fn members(array: &ExprArray) -> impl Iterator<Item = (&Expr, Option<SourceSpan>)> {
    let mut previous = None;
    array.elems.pairs().map(move |pair| {
        let (item, comma) = match pair {
            Pair::Punctuated(expr, comma) => (expr, Some(span(comma))),
            Pair::End(expr) => (expr, None),
        };
        let removal = comma.or(previous);
        previous = comma;
        (item, removal)
    })
}
