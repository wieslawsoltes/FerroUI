//! Port of `Helpers/CommandCanExecuteIsEnabledBinder.cs`.

use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElement;
use ferroui_base::reactive::{IDisposable, IObservable, ObservableExt};
use ferroui_base::{ferro_properties, ferro_static_type, AttachedProperty, FerroProperty, Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Makes the `IsEnabled` property of an input element follow whether a command can execute: while
/// it cannot, an `IsEnabled = false` value of the animation priority overlays the value of the
/// element (one overlay per element, counted across the binders that block it).
#[derive(Default)]
pub struct CommandCanExecuteIsEnabledBinder {
    state: Rc<BinderState>,
}

#[derive(Default)]
struct BinderState {
    target: RefCell<Option<WeakRef<InputElement>>>,
    can_execute_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    is_blocking: Cell<bool>,
}

ferro_static_type!(CommandCanExecuteIsEnabledBinder);

ferro_properties! {
    impl CommandCanExecuteIsEnabledBinder {
        fn overlay_state_property() -> AttachedProperty<Option<CommandCanExecuteIsEnabledOverlayState>> {
            FerroProperty::register_attached::<
                CommandCanExecuteIsEnabledBinder,
                InputElement,
                Option<CommandCanExecuteIsEnabledOverlayState>,
            >("CommandCanExecuteIsEnabledOverlayState", None)
        }
    }
}

impl CommandCanExecuteIsEnabledBinder {
    /// Creates a binder that blocks nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes the `IsEnabled` property of `target` follow `can_execute` when `enabled`; stops
    /// otherwise.
    pub fn update(&self, target: Option<Ref<InputElement>>, enabled: bool, can_execute: Rc<dyn IObservable<bool>>) {
        let target = match target {
            Some(target) if enabled => target,
            _ => {
                self.stop();
                return;
            }
        };

        let same_target =
            self.state.target.borrow().as_ref().and_then(WeakRef::upgrade).is_some_and(|current| current == target);
        if same_target && self.state.can_execute_subscription.borrow().is_some() {
            return;
        }

        self.stop();

        *self.state.target.borrow_mut() = Some(target.downgrade());
        let state = Rc::downgrade(&self.state);
        let subscription = can_execute.subscribe_fn(move |can_execute: bool| {
            if let Some(state) = state.upgrade() {
                state.update_can_execute(can_execute);
            }
        });
        *self.state.can_execute_subscription.borrow_mut() = Some(subscription);
    }

    /// Stops following the command and removes the overlay this binder added.
    pub fn stop(&self) {
        let subscription = self.state.can_execute_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        let target = self.state.target.borrow_mut().take().and_then(|target| target.upgrade());
        if self.state.is_blocking.get() {
            if let Some(target) = &target {
                Self::remove_disabled_overlay(target);
            }
        }

        self.state.is_blocking.set(false);
    }

    fn add_disabled_overlay(target: &InputElement) {
        let state = match target.get_value(Self::overlay_state_property()) {
            Some(state) => state,
            None => {
                let state = CommandCanExecuteIsEnabledOverlayState::default();
                target.set_value(Self::overlay_state_property(), Some(state.clone()));
                state
            }
        };

        state.0.block_count.set(state.0.block_count.get() + 1);
        if state.0.block_count.get() == 1 {
            let overlay =
                target.set_value_with_priority(InputElement::is_enabled_property(), false, BindingPriority::Animation);
            *state.0.disabled_overlay.borrow_mut() = overlay;
        }
    }

    fn remove_disabled_overlay(target: &InputElement) {
        let Some(state) = target.get_value(Self::overlay_state_property()) else {
            return;
        };

        state.0.block_count.set(state.0.block_count.get() - 1);
        if state.0.block_count.get() > 0 {
            return;
        }

        let overlay = state.0.disabled_overlay.borrow_mut().take();
        if let Some(overlay) = overlay {
            overlay.dispose();
        }
        target.clear_value(Self::overlay_state_property());
    }
}

impl BinderState {
    fn update_can_execute(&self, can_execute: bool) {
        let Some(target) = self.target.borrow().as_ref().and_then(WeakRef::upgrade) else {
            return;
        };

        if can_execute {
            if self.is_blocking.get() {
                CommandCanExecuteIsEnabledBinder::remove_disabled_overlay(&target);
                self.is_blocking.set(false);
            }
        } else if !self.is_blocking.get() {
            CommandCanExecuteIsEnabledBinder::add_disabled_overlay(&target);
            self.is_blocking.set(true);
        }
    }
}

impl IDisposable for CommandCanExecuteIsEnabledBinder {
    fn dispose(&self) {
        self.stop();
    }
}

/// The overlay of an element: how many binders block it and the value that disables it.
#[derive(Clone, Default)]
struct CommandCanExecuteIsEnabledOverlayState(Rc<OverlayState>);

#[derive(Default)]
struct OverlayState {
    block_count: Cell<i32>,
    disabled_overlay: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl PartialEq for CommandCanExecuteIsEnabledOverlayState {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
