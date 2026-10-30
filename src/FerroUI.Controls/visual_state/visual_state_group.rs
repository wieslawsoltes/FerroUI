//! WinUI's `VisualStateGroup` (`Microsoft.UI.Xaml.VisualStateGroup`; `dxaml/xcp/dxaml/lib/VisualStateGroup_Partial.cpp`,
//! the engine `dxaml/xcp/components/vsm/VisualStateManagerActuator.cpp` and
//! `StandardVisualStateManagerDataSource.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::{AppliedSetters, VisualState, VisualStateChangedEventArgs};
use ferroui_base::collections::FerroList;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Ref};
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

    /// The current state of the group: `None` until a state is entered.
    pub fn current_state(&self) -> Option<Ref<VisualState>> {
        self.current_state.borrow().clone()
    }

    /// The state of the group named `name` (case-sensitive), if any.
    pub(crate) fn find_state(&self, name: &str) -> Option<Ref<VisualState>> {
        self.states.snapshot().iter().find(|state| state.is_named(name)).cloned()
    }

    /// Adds a handler of `CurrentStateChanging`, raised when a change of state begins. Disposing
    /// the returned handle removes it.
    pub fn current_state_changing(
        &self,
        handler: impl Fn(&Ref<VisualStateGroup>, &VisualStateChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.current_state_changing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.current_state_changing.remove(token);
            }
        })
    }

    /// Adds a handler of `CurrentStateChanged`, raised when a change of state has completed.
    /// Disposing the returned handle removes it.
    pub fn current_state_changed(
        &self,
        handler: impl Fn(&Ref<VisualStateGroup>, &VisualStateChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.current_state_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.current_state_changed.remove(token);
            }
        })
    }

    /// Enters `state` (or no state): applies its setters, then removes the values of the state
    /// it leaves (WinUI's `VisualStateManagerActuator::ChangeVisualState` for a change without a
    /// transition). Raises nothing; the manager raises the events.
    pub(crate) fn set_state(&self, state: Option<&Ref<VisualState>>, template_root: Option<&Ref<crate::Control>>) {
        let applied = match state {
            Some(state) => AppliedSetters::apply(state, template_root),
            None => AppliedSetters::default(),
        };
        // The new values are applied before the old ones are removed, so that a property both
        // states set changes once, from the old value to the new one.
        let old = std::mem::replace(&mut *self.applied.borrow_mut(), applied);
        *self.current_state.borrow_mut() = state.cloned();
        old.remove();
    }

    /// Applies the setters of the current state again (`GoToState` to the current state without
    /// transitions).
    pub(crate) fn reapply_current_state(&self, template_root: Option<&Ref<crate::Control>>) {
        let state = self.current_state();
        self.set_state(state.as_ref(), template_root);
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

    /// Leaves the current state without raising events: the values are removed (the groups are
    /// removed from their element).
    pub(crate) fn detach(&self) {
        self.set_state(None, None);
    }
}

impl Drop for VisualStateGroup {
    fn drop(&mut self) {
        std::mem::take(self.applied.get_mut()).remove();
    }
}
