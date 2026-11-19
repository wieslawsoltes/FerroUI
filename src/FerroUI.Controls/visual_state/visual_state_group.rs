//! WinUI's `VisualStateGroup` (`Microsoft.UI.Xaml.VisualStateGroup`; `dxaml/xcp/dxaml/lib/VisualStateGroup_Partial.cpp`,
//! the engine `dxaml/xcp/components/vsm/VisualStateManagerActuator.cpp` and
//! `StandardVisualStateManagerDataSource.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::transitions::{self, StateValues};
use super::{AppliedSetters, Names, VisualState, VisualStateChangedEventArgs, VisualTransition};
use crate::animation::Storyboard;
use ferroui_base::collections::FerroList;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use crate::Control;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Ref, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

type StateChangedHandler = dyn Fn(&Ref<VisualStateGroup>, &VisualStateChangedEventArgs);

/// A group of mutually exclusive [`VisualState`]s: at most one of them is the
/// [`current_state`](Self::current_state) of the group, null at first.
///
/// The groups of an element are the attached
/// [`VisualStateManager::visual_state_groups_property`](super::VisualStateManager::visual_state_groups_property);
/// `VisualStateManager::go_to_state` changes the state of the group that has the state of the
/// name it is given.
#[repr(C)]
pub struct VisualStateGroup {
    base: FerroObject,
    name: RefCell<String>,
    states: FerroList<Ref<VisualState>>,
    current_state: RefCell<Option<Ref<VisualState>>>,
    // The values the current state applied (removed when the state is left).
    applied: RefCell<AppliedSetters>,
    current_state_changing: HandlerList<StateChangedHandler>,
    current_state_changed: HandlerList<StateChangedHandler>,
    // The element of the group while it is in a tree (the triggers select states then), and the
    // state the triggers selected last (`None` inside: no state; `None` outside: not evaluated).
    trigger_root: RefCell<Option<WeakRef<Control>>>,
    trigger_selection: RefCell<Option<Option<WeakRef<VisualState>>>>,
    transitions: FerroList<Ref<VisualTransition>>,
    // The transition that runs, until its storyboards complete.
    pending: RefCell<Option<PendingTransition>>,
}

/// A transition that runs: the new state applies its setters and begins its storyboard when the
/// storyboards of the transition have completed.
struct PendingTransition {
    state: Ref<VisualState>,
    root: Option<WeakRef<Control>>,
    storyboards: Vec<Ref<Storyboard>>,
    remaining: usize,
    _completed: Vec<Subscription>,
    // The values of the old state that the new state sets too: kept until the new ones apply.
    retained: AppliedSetters,
    on_complete: Option<Box<dyn FnOnce()>>,
}

ferro_class!(VisualStateGroup: FerroObject);
ferro_class_info!(VisualStateGroup {
    new: VisualStateGroup::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: States,
        properties: [
            Name: String { get: VisualStateGroup::name, set: |this: &Ref<VisualStateGroup>, value: String| this.set_name(value) },
            States: FerroList<Ref<VisualState>> { get: VisualStateGroup::states },
            Transitions: FerroList<Ref<VisualTransition>> { get: VisualStateGroup::transitions },
            CurrentState: Option<Ref<VisualState>> { get: VisualStateGroup::current_state },
        ],
    },
});
ferro_impl_classes!(VisualStateGroup: FerroObjectImpl);

impl VisualStateGroup {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            name: RefCell::new(String::new()),
            states: FerroList::new(),
            current_state: RefCell::new(None),
            applied: RefCell::new(AppliedSetters::default()),
            current_state_changing: HandlerList::new(),
            current_state_changed: HandlerList::new(),
            trigger_root: RefCell::new(None),
            trigger_selection: RefCell::new(None),
            transitions: FerroList::new(),
            pending: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a group with a name.
    pub fn with_name(name: impl Into<String>) -> Ref<Self> {
        let group = Self::new();
        group.set_name(name);
        group
    }

    /// The name of the group.
    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    /// Sets the name (WinUI's `x:Name`).
    pub fn set_name(&self, value: impl Into<String>) {
        *self.name.borrow_mut() = value.into();
    }

    /// The states of the group.
    pub fn states(&self) -> FerroList<Ref<VisualState>> {
        self.states.clone()
    }

    /// The transitions between the states of the group.
    pub fn transitions(&self) -> FerroList<Ref<VisualTransition>> {
        self.transitions.clone()
    }

    /// The current state of the group: `None` until a state is entered.
    pub fn current_state(&self) -> Option<Ref<VisualState>> {
        self.current_state.borrow().clone()
    }

    /// The state of the group named `name` (case-sensitive), if any.
    pub(crate) fn find_state(&self, name: &str) -> Option<Ref<VisualState>> {
        self.states.snapshot().iter().find(|state| state.is_named(name)).cloned()
    }

    /// Adds a handler of `CurrentStateChanging`, raised when a change of state begins.
    ///
    /// Dropping the returned guard removes the handler; [`Subscription::detach`] keeps it for
    /// the life of the group.
    pub fn current_state_changing(
        &self,
        handler: impl Fn(&Ref<VisualStateGroup>, &VisualStateChangedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.current_state_changing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.current_state_changing.remove(token);
            }
        }))
    }

    /// Adds a handler of `CurrentStateChanged`, raised when a change of state has completed.
    ///
    /// Dropping the returned guard removes the handler; [`Subscription::detach`] keeps it
    /// for the life of the group.
    pub fn current_state_changed(
        &self,
        handler: impl Fn(&Ref<VisualStateGroup>, &VisualStateChangedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.current_state_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.current_state_changed.remove(token);
            }
        }))
    }

    /// Enters `state` (or no state): applies its setters, then removes the values of the state
    /// it leaves (WinUI's `VisualStateManagerActuator::ChangeVisualState` for a change without a
    /// transition). A transition that runs is stopped. Raises nothing; the manager raises the
    /// events.
    pub(crate) fn set_state(&self, state: Option<&Ref<VisualState>>, template_root: Option<&Ref<crate::Control>>) {
        let interrupted = self.cancel_transition();
        let applied = match state {
            Some(state) => AppliedSetters::apply(state, template_root),
            None => AppliedSetters::default(),
        };
        // The storyboard of the new state starts, then the one of the old state stops (WinUI's
        // `VisualStateManagerActuator::ChangeVisualState`): a property both animate goes on
        // from the value it has.
        let old_storyboard = self.current_state().and_then(|old| old.storyboard());
        let new_storyboard = state.and_then(|state| state.storyboard());
        if let Some(storyboard) = &new_storyboard {
            let names = template_root.map(Names::of);
            storyboard.begin_with(names.as_ref());
        }
        if let Some(old) = old_storyboard.filter(|old| Some(old) != new_storyboard.as_ref()) {
            old.stop();
        }
        for storyboard in interrupted {
            storyboard.stop();
        }
        // The new values are applied before the old ones are removed, so that a property both
        // states set changes once, from the old value to the new one.
        let old = std::mem::replace(&mut *self.applied.borrow_mut(), applied);
        *self.current_state.borrow_mut() = state.cloned();
        old.remove();
    }

    /// Stops a transition that runs (another change of state interrupts it): its storyboards are
    /// returned to be stopped by the caller after the new values start, the values it kept are
    /// the values of the group again, and it completes nothing.
    fn cancel_transition(&self) -> Vec<Ref<Storyboard>> {
        let pending = self.pending.borrow_mut().take();
        match pending {
            Some(pending) => {
                self.applied.borrow_mut().append(pending.retained);
                pending.storyboards
            }
            None => Vec::new(),
        }
    }

    /// Enters `state` through `transition` (WinUI's `ChangeVisualState` with a transition of
    /// non-zero length): the storyboard of the transition and the generated storyboard run; the
    /// setters and the storyboard of the new state apply when they have completed, and then
    /// `on_complete` runs. The group is in the new state while the transition runs.
    pub(crate) fn begin_transition(
        &self,
        state: &Ref<VisualState>,
        template_root: Option<&Ref<crate::Control>>,
        transition: &Ref<VisualTransition>,
        on_complete: Box<dyn FnOnce()>,
    ) {
        let interrupted = self.cancel_transition();
        let names = template_root.map(Names::of);
        let old_state = self.current_state();
        let old_storyboard = old_state.as_ref().and_then(|old| old.storyboard());
        let resolved = |storyboard: Option<&Ref<Storyboard>>| {
            storyboard.map(|storyboard| storyboard.resolved_animations(names.as_ref())).unwrap_or_default()
        };
        // The storyboards that run now: the old state's and an interrupted transition's.
        let mut old_animations = resolved(old_storyboard.as_ref().filter(|storyboard| storyboard.is_running()));
        for storyboard in &interrupted {
            old_animations.extend(resolved(Some(storyboard)));
        }
        let old_setters = old_state
            .as_ref()
            .map(|old| AppliedSetters::resolve(old, template_root))
            .unwrap_or_default();
        let new_setters = AppliedSetters::resolve(state, template_root);
        let new_animations = resolved(state.storyboard().as_ref());
        let transition_storyboard = transition.storyboard();
        let transition_animations = resolved(transition_storyboard.as_ref());
        let generated = transitions::generate(
            transition,
            &transition_animations,
            StateValues { animations: &old_animations, setters: &old_setters },
            StateValues { animations: &new_animations, setters: &new_setters },
        );
        let mut storyboards: Vec<Ref<Storyboard>> = transition_storyboard.into_iter().collect();
        storyboards.push(generated);
        // The values of the old state the new one sets too stay until the new ones apply; the
        // others are removed now (the generated storyboard animates them back).
        let new_keys: Vec<_> = new_setters
            .iter()
            .map(|setter| super::applied_setters::key_of(&setter.object, setter.property))
            .collect();
        let old_values = std::mem::take(&mut *self.applied.borrow_mut());
        let mut old_values = old_values;
        let retained = old_values.take_matching(&new_keys);
        let weak = self.to_ref().downgrade();
        let completed = storyboards
            .iter()
            .map(|storyboard| {
                let weak = weak.clone();
                storyboard.completed(move |_| {
                    if let Some(group) = weak.upgrade() {
                        group.transition_storyboard_completed();
                    }
                })
            })
            .collect();
        *self.current_state.borrow_mut() = Some(state.clone());
        *self.pending.borrow_mut() = Some(PendingTransition {
            state: state.clone(),
            root: template_root.map(|root| root.downgrade()),
            storyboards: storyboards.clone(),
            remaining: storyboards.len(),
            _completed: completed,
            retained,
            on_complete: Some(on_complete),
        });
        // The new storyboards start before the old ones stop, for a smooth handoff.
        for storyboard in &storyboards {
            storyboard.begin_with(names.as_ref());
        }
        if let Some(old) = old_storyboard {
            old.stop();
        }
        for storyboard in interrupted {
            storyboard.stop();
        }
        old_values.remove();
    }

    /// A storyboard of the running transition completed; the last one finishes the transition
    /// (WinUI's `FinishVisualTransition`).
    fn transition_storyboard_completed(&self) {
        let finished = {
            let mut pending = self.pending.borrow_mut();
            match pending.as_mut() {
                Some(transition) => {
                    transition.remaining = transition.remaining.saturating_sub(1);
                    transition.remaining == 0
                }
                None => false,
            }
        };
        if !finished {
            return;
        }
        let Some(mut transition) = self.pending.borrow_mut().take() else {
            return;
        };
        let root = transition.root.as_ref().and_then(WeakRef::upgrade);
        let applied = AppliedSetters::apply(&transition.state, root.as_ref());
        std::mem::replace(&mut *self.applied.borrow_mut(), applied).remove();
        std::mem::take(&mut transition.retained).remove();
        if let Some(storyboard) = transition.state.storyboard() {
            let names = root.as_ref().map(Names::of);
            storyboard.begin_with(names.as_ref());
        }
        for storyboard in &transition.storyboards {
            storyboard.stop();
        }
        if let Some(on_complete) = transition.on_complete.take() {
            on_complete();
        }
    }

    /// Whether a transition runs.
    #[cfg(test)]
    pub(crate) fn is_transitioning(&self) -> bool {
        self.pending.borrow().is_some()
    }

    /// Applies the setters of the current state again (`GoToState` to the current state without
    /// transitions).
    pub(crate) fn reapply_current_state(&self, template_root: Option<&Ref<crate::Control>>) {
        // A running transition completes first (its storyboards skip to their fill).
        let running = self.pending.borrow().as_ref().map(|pending| pending.storyboards.clone()).unwrap_or_default();
        for storyboard in running {
            storyboard.skip_to_fill();
        }
        let state = self.current_state();
        let applied = match &state {
            Some(state) => AppliedSetters::apply(state, template_root),
            None => AppliedSetters::default(),
        };
        std::mem::replace(&mut *self.applied.borrow_mut(), applied).remove();
        // The running storyboard of the state completes (WinUI's `GoToState` to the current
        // state without transitions).
        if let Some(storyboard) = state.and_then(|state| state.storyboard()) {
            storyboard.skip_to_fill();
        }
    }

    pub(crate) fn raise_current_state_changing(&self, args: &VisualStateChangedEventArgs) {
        let this = self.to_ref();
        for (_, handler) in self.current_state_changing.snapshot().iter() {
            handler(&this, args);
        }
    }

    pub(crate) fn raise_current_state_changed(&self, args: &VisualStateChangedEventArgs) {
        let this = self.to_ref();
        for (_, handler) in self.current_state_changed.snapshot().iter() {
            handler(&this, args);
        }
    }

    /// The element whose groups this group is, while it is in a tree.
    pub(crate) fn trigger_root(&self) -> Option<Ref<Control>> {
        self.trigger_root.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_trigger_root(&self, root: Option<&Ref<Control>>) {
        *self.trigger_root.borrow_mut() = root.map(|root| root.downgrade());
    }

    /// Records the state the triggers select; `false` when it is the one they selected last.
    pub(crate) fn set_trigger_selection(&self, state: Option<&Ref<VisualState>>) -> bool {
        let mut selection = self.trigger_selection.borrow_mut();
        let same = match &*selection {
            Some(previous) => match (previous.as_ref().and_then(WeakRef::upgrade), state) {
                (Some(previous), Some(state)) => &previous == state,
                (None, None) => true,
                _ => false,
            },
            None => false,
        };
        *selection = Some(state.map(|state| state.downgrade()));
        !same
    }

    /// Leaves the current state without raising events: the values are removed (the groups are
    /// removed from their element).
    pub(crate) fn detach(&self) {
        *self.trigger_root.borrow_mut() = None;
        *self.trigger_selection.borrow_mut() = None;
        self.set_state(None, None);
    }
}

impl Drop for VisualStateGroup {
    fn drop(&mut self) {
        if let Some(mut transition) = self.pending.get_mut().take() {
            for storyboard in &transition.storyboards {
                storyboard.stop();
            }
            std::mem::take(&mut transition.retained).remove();
        }
        if let Some(storyboard) = self.current_state.get_mut().as_ref().and_then(|state| state.storyboard()) {
            storyboard.stop();
        }
        std::mem::take(self.applied.get_mut()).remove();
    }
}
