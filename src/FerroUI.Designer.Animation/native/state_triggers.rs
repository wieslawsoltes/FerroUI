use crate::{ProjectedStateGroup, ProjectedStateTrigger, VisualStateGroup};
use ferroui_base::interactivity::{Interactive, RoutedEventRegistry};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::{Ref, StyledElement};
use ferroui_designer_core::{Document, NodeId};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Routed subscriptions owned by one accepted native preview.
#[derive(Default)]
pub struct NativeStateTriggers {
    _subscriptions: Vec<Subscription>,
    pending: Rc<RefCell<Option<Vec<StateTriggerSelection>>>>,
}

#[derive(Clone)]
pub struct StateTriggerSelection {
    pub owner: NodeId,
    pub group: String,
    pub state: String,
}

pub struct StateTriggerCapture(Rc<RefCell<Option<Vec<StateTriggerSelection>>>>);

impl StateTriggerCapture {
    pub fn finish(&self) -> Vec<StateTriggerSelection> {
        self.0.borrow_mut().take().unwrap_or_default()
    }
}

impl Drop for StateTriggerCapture {
    fn drop(&mut self) {
        self.0.borrow_mut().take();
    }
}

impl NativeStateTriggers {
    /// Collect selections while dispatching input in a one-frame preview.
    /// The host can render the selected endpoints using its normal state sampler.
    pub fn capture_input(&self) -> StateTriggerCapture {
        *self.pending.borrow_mut() = Some(Vec::new());
        StateTriggerCapture(self.pending.clone())
    }

    pub fn bind(
        document: &Document,
        mut resolve: impl FnMut(NodeId) -> Option<Ref<StyledElement>>,
    ) -> Self {
        let triggers = ProjectedStateTrigger::scan(document);
        if triggers.is_empty() {
            return Self::default();
        }
        let mut identities = BTreeMap::new();
        for trigger in &triggers {
            *identities
                .entry((
                    trigger.source,
                    trigger.event.as_str(),
                    trigger.group.as_str(),
                ))
                .or_insert(0) += 1;
        }
        let groups = ProjectedStateGroup::scan(document);
        let pending = Rc::<RefCell<Option<Vec<StateTriggerSelection>>>>::default();
        let subscriptions = triggers
            .iter()
            .filter_map(|trigger| {
                if identities.get(&(
                    trigger.source,
                    trigger.event.as_str(),
                    trigger.group.as_str(),
                )) != Some(&1)
                {
                    return None;
                }
                let group = enclosing_group(document, &groups, trigger)?;
                if !group.states.iter().any(|state| state.name == trigger.state) {
                    return None;
                }
                let owner = resolve(group.owner)?.downgrade();
                let source = resolve(trigger.source)?.cast::<Interactive>()?;
                let event = RoutedEventRegistry::instance()
                    .resolve_by_name(source.get_type(), &trigger.event)?;
                let selection = StateTriggerSelection {
                    owner: group.owner,
                    group: group.name.clone(),
                    state: trigger.state.clone(),
                };
                let group = group.clone();
                let pending = pending.clone();
                let token = source.add_route_handler(
                    &event,
                    Rc::new(move |_, _| {
                        if let Some(captured) = pending.borrow_mut().as_mut() {
                            captured.push(selection.clone());
                            return;
                        }
                        if let Some(owner) = owner.upgrade() {
                            enter_state(&owner, &group, &selection.state);
                        }
                    }),
                    Interactive::DEFAULT_ROUTES,
                    false,
                );
                let source = source.downgrade();
                Some(Subscription::new(Disposable::create(move || {
                    if let Some(source) = source.upgrade() {
                        source.remove_handler(&event, token);
                    }
                })))
            })
            .collect();
        Self {
            _subscriptions: subscriptions,
            pending,
        }
    }
}

fn enclosing_group<'a>(
    document: &Document,
    groups: &'a [ProjectedStateGroup],
    trigger: &ProjectedStateTrigger,
) -> Option<&'a VisualStateGroup> {
    let mut ancestor = Some(trigger.source);
    while let Some(id) = ancestor {
        if let Some(group) = groups
            .iter()
            .find(|group| group.model.owner == id && group.model.name == trigger.group)
        {
            return Some(&group.model);
        }
        ancestor = document.ast().node(id).and_then(|node| node.parent);
    }
    None
}

fn enter_state(owner: &StyledElement, group: &VisualStateGroup, destination: &str) {
    let classes = owner.classes();
    let current = group
        .states
        .iter()
        .find(|state| classes.contains(&group.class_name(&state.name)));
    let current = current.map(|state| state.name.as_str());
    if current == Some(destination) {
        return;
    }
    let route = group.transition(current, destination);
    for class in group.classes() {
        classes.remove(&class);
    }
    if let Some(from) = route.and_then(|route| route.from.as_deref()) {
        classes.add(&group.transition_class(from, destination));
    }
    classes.add(&group.class_name(destination));
}
