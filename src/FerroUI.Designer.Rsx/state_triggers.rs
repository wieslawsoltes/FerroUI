//! Source mapping for routed events that enter an inline Rust visual state.

mod markup;
pub(crate) use markup::TriggerMarkup;

use crate::{
    RsxProjection,
    animation::syntax::span,
    source_array::ArraySlot,
    source_edits::{insert_field, reset_field},
};
use ferroui_designer_animation::VisualStateGroup;
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_language::{LineIndex, Range, event_handler_name};
use serde::Serialize;
use std::collections::BTreeSet;
use syn::{Expr, ExprLit, Lit};

/// A supported declaration on the event source in the original Rust buffer.
#[derive(Clone, Debug)]
pub struct RsxStateTrigger {
    pub event: String,
    pub group: String,
    pub state: String,
    pub declaration: SourceSpan,
    destination: SourceSpan,
}

#[derive(Default)]
pub(crate) struct StateTriggers {
    items: Vec<RsxStateTrigger>,
    opaque: bool,
}

#[derive(Serialize)]
pub struct StateTriggerReport {
    pub events: Vec<String>,
    pub groups: Vec<StateTriggerGroup>,
    pub triggers: Vec<StateTriggerLink>,
}

#[derive(Serialize)]
pub struct StateTriggerGroup {
    pub group: String,
    pub states: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct StateTriggerLink {
    pub event: String,
    pub group: String,
    pub state: String,
    pub range: Range,
}

impl StateTriggers {
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty() && !self.opaque
    }

    pub(crate) fn read<'a>(
        &mut self,
        mut args: impl Iterator<Item = &'a Expr>,
        declaration: Option<SourceSpan>,
    ) -> bool {
        let trigger = (|| {
            let (event, _) = string_argument(args.next()?)?;
            let (group, _) = string_argument(args.next()?)?;
            let (state, destination) = string_argument(args.next()?)?;
            if args.next().is_some() || !event_handler_name(&event) {
                return None;
            }
            Some(RsxStateTrigger {
                event,
                group,
                state,
                declaration: declaration?,
                destination,
            })
        })();
        if let Some(trigger) = trigger {
            self.items.push(trigger);
            true
        } else {
            self.opaque = true;
            false
        }
    }
}

impl RsxProjection<'_> {
    pub fn state_triggers(&self, source: usize) -> &[RsxStateTrigger] {
        self.nodes
            .get(source)
            .map_or(&[], |node| &node.state_triggers.items)
    }

    /// The closest owner of each visible group, starting at the event source.
    pub fn state_trigger_groups(&self, source: usize) -> Vec<(usize, &VisualStateGroup)> {
        let mut seen = BTreeSet::new();
        let mut groups = Vec::new();
        for owner in (0..=source).rev() {
            let Some(node) = self
                .nodes
                .get(owner)
                .filter(|node| node.subtree.end > source)
            else {
                continue;
            };
            // A computed enclosing group may shadow any group further out.
            if matches!(node.state_slot, ArraySlot::Unavailable) {
                break;
            }
            if let Some(states) = self.states.get(&owner) {
                groups.extend(states.groups.iter().filter_map(|group| {
                    seen.insert(group.name()).then_some((owner, group.model()))
                }));
            }
        }
        groups
    }

    pub fn state_trigger_report(&self, source: usize) -> Option<StateTriggerReport> {
        let node = self
            .nodes
            .get(source)
            .filter(|node| node.supports_events())?;
        let index = LineIndex::new(self.source);
        Some(StateTriggerReport {
            events: if node.state_triggers.opaque {
                Vec::new()
            } else {
                node.class.state_trigger_events()
            },
            groups: self
                .state_trigger_groups(source)
                .iter()
                .map(|(_, group)| StateTriggerGroup {
                    group: group.name.clone(),
                    states: group
                        .states
                        .iter()
                        .map(|state| state.name.clone())
                        .collect(),
                })
                .collect(),
            triggers: node
                .state_triggers
                .items
                .iter()
                .filter_map(|trigger| {
                    Some(StateTriggerLink {
                        event: trigger.event.clone(),
                        group: trigger.group.clone(),
                        state: trigger.state.clone(),
                        range: index.range(trigger.declaration).ok()?,
                    })
                })
                .collect(),
        })
    }

    pub(crate) fn remove_state_trigger_references(
        &self,
        owner: usize,
        name: &str,
        retained: Option<&VisualStateGroup>,
    ) -> Result<Vec<TextPatch>, String> {
        let mut patches = Vec::new();
        for source in owner..self.nodes[owner].subtree.end {
            let node = &self.nodes[source];
            if !node
                .state_triggers
                .items
                .iter()
                .any(|trigger| trigger.group == name)
            {
                continue;
            }
            if !self
                .state_trigger_groups(source)
                .iter()
                .any(|(index, group)| *index == owner && group.name == name)
            {
                continue;
            }
            for trigger in node
                .state_triggers
                .items
                .iter()
                .filter(|trigger| trigger.group == name)
            {
                if retained.is_none_or(|group| {
                    !group.states.iter().any(|state| state.name == trigger.state)
                }) {
                    patches.push(reset_field(self.source, trigger.declaration, node.close)?);
                }
            }
        }
        Ok(patches)
    }

    pub fn plan_state_trigger(
        &self,
        source: usize,
        event: &str,
        group: &str,
        state: Option<&str>,
    ) -> Result<Vec<TextPatch>, String> {
        let node = self
            .nodes
            .get(source)
            .ok_or("Choose a mapped Rust control")?;
        if !node.supports_events() || node.state_triggers.opaque || !event_handler_name(event) {
            return Err("Choose a mapped routed event or edit its trigger in Rust source".into());
        }
        let mut existing = node
            .state_triggers
            .items
            .iter()
            .filter(|trigger| trigger.event == event && trigger.group == group);
        let before = existing.next();
        if existing.next().is_some() {
            return Err("This event has more than one trigger for the same state group".into());
        }
        let patches = if let Some(state) = state {
            if !node
                .class
                .state_trigger_events()
                .iter()
                .any(|candidate| candidate == event)
            {
                return Err(format!("{event} is unavailable for this control"));
            }
            let (_, model) = self
                .state_trigger_groups(source)
                .into_iter()
                .find(|(_, model)| model.name == group)
                .ok_or("Choose an enclosing inline state group")?;
            if !model.states.iter().any(|candidate| candidate.name == state) {
                return Err("Choose a state declared by the enclosing group".into());
            }
            match before {
                Some(trigger) if trigger.state == state => return Ok(Vec::new()),
                Some(trigger) => vec![TextPatch {
                    span: trigger.destination,
                    replacement: format!("{state:?}"),
                }],
                None => vec![insert_field(
                    self.source,
                    node.open,
                    &format!("state_trigger({event:?}, {group:?}, {state:?})"),
                )],
            }
        } else {
            let old = before.ok_or("Choose an authored state trigger")?;
            vec![reset_field(self.source, old.declaration, node.close)?]
        };
        let draft = TextPatch::apply_all(self.source, patches.clone())
            .map_err(|error| error.to_string())?;
        let modules = self.draft_module_sources(&draft)?;
        let next =
            self.catalog
                .project_with_modules(&draft, self.invocation.start, modules.as_ref())?;
        let mut matches = next
            .state_triggers(source)
            .iter()
            .filter(|trigger| trigger.event == event && trigger.group == group);
        let found = matches.next().map(|trigger| trigger.state.as_str());
        if next.tree != self.tree || found != state || matches.next().is_some() {
            return Err("The event trigger does not round-trip in the active Rust view".into());
        }
        Ok(patches)
    }
}

fn string_argument(expr: &Expr) -> Option<(String, SourceSpan)> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = expr
    else {
        return None;
    };
    Some((value.value(), span(value)))
}
