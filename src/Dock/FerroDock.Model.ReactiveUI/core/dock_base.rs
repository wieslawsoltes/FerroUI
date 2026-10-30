// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{DockableBase, DockableBaseImpl};
use ferrodock_model::adapters::NavigateAdapter;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{
    reactive_object, ICommand, IReactiveObjectExt, ReactiveCommand, ReactiveObject,
    ReactiveUIBindingExtensions,
};
use ferroui_system::{prop, IDisposable};
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};

type DockableList = ListHandle<Option<Rc<dyn IDockable>>>;

/// The abstract dock model state and commands, composed into concrete containers.
/// Construct the base during Rc::new_cyclic, then initialize observations after
/// the enclosing model's Rc identity is available.
pub struct DockBase {
    base: DockableBase,
    adapter: Rc<NavigateAdapter>,
    go_back: Rc<dyn ICommand>,
    go_forward: Rc<dyn ICommand>,
    navigate: Rc<dyn ICommand>,
    close: Rc<dyn ICommand>,
    initialized: Cell<bool>,
    subscriptions: OnceCell<Vec<Rc<dyn IDisposable>>>,
    visible_dockables: RefCell<Option<DockableList>>,
    active_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    default_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    focused_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    is_active: RefCell<bool>,
    opened_dockables_count: RefCell<i32>,
    can_close_last_dockable: RefCell<bool>,
    dock_capability_policy: RefCell<Option<Rc<dyn DockCapabilityPolicyImpl>>>,
    enable_global_docking: RefCell<bool>,
}

impl AsRef<ReactiveObject> for DockBase {
    fn as_ref(&self) -> &ReactiveObject {
        self.base.as_ref()
    }
}
impl std::ops::Deref for DockBase {
    type Target = DockableBase;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
reactive_object!(DockBase, base);

impl DockBase {
    /// Initializes source defaults and ReactiveCommands with a weak enclosing identity.
    pub fn construct<T: DockableBaseImpl + IDock + 'static>(runtime: Weak<T>) -> Self {
        let dock: Weak<dyn IDock> = runtime.clone();
        let base: Weak<dyn DockableBaseImpl> = runtime;
        let adapter = Rc::new(NavigateAdapter::from_weak(dock));
        let weak = Rc::downgrade(&adapter);
        let go_back = Rc::new(ReactiveCommand::create(move || {
            if let Some(adapter) = weak.upgrade() {
                adapter.go_back();
            }
        }));
        let weak = Rc::downgrade(&adapter);
        let go_forward = Rc::new(ReactiveCommand::create(move || {
            if let Some(adapter) = weak.upgrade() {
                adapter.go_forward();
            }
        }));
        let weak = Rc::downgrade(&adapter);
        let close = Rc::new(ReactiveCommand::create(move || {
            if let Some(adapter) = weak.upgrade() {
                adapter.close();
            }
        }));
        let navigate = Rc::new(NavigateCommand::new(Rc::downgrade(&adapter)));
        Self {
            base: DockableBase::construct(base),
            adapter,
            go_back,
            go_forward,
            close,
            navigate,
            initialized: Cell::new(false),
            subscriptions: OnceCell::new(),
            visible_dockables: RefCell::new(None),
            active_dockable: RefCell::new(None),
            default_dockable: RefCell::new(None),
            focused_dockable: RefCell::new(None),
            is_active: RefCell::new(false),
            opened_dockables_count: RefCell::new(0),
            can_close_last_dockable: RefCell::new(true),
            dock_capability_policy: RefCell::new(None),
            enable_global_docking: RefCell::new(true),
        }
    }

    /// Attaches the source's active/focused WhenAnyValue observations exactly once.
    /// Custom containers must call this after Rc::new_cyclic has returned.
    pub fn initialize<T: DockableBaseImpl + IDock + 'static>(&self, runtime: &Rc<T>) {
        if self.initialized.replace(true) {
            return;
        }
        let dock = Rc::downgrade(runtime);
        let active = runtime
            .when_any_value(prop!(T => active_dockable))
            .subscribe_next(move |value: Option<Rc<dyn IDockable>>| {
                if let Some(dock) = dock.upgrade() {
                    if let Some(factory) = dock.factory() {
                        factory.init_active_dockable(value, Some(dock.clone()));
                    }
                    dock.raise_property_changed("CanGoBack");
                    dock.raise_property_changed("CanGoForward");
                }
            });
        let dock = Rc::downgrade(runtime);
        let focused = runtime
            .when_any_value(prop!(T => focused_dockable))
            .subscribe_next(move |value: Option<Rc<dyn IDockable>>| {
                if let Some(factory) = dock.upgrade().and_then(|dock| dock.factory()) {
                    factory.on_focused_dockable_changed(value);
                }
            });
        assert!(
            self.subscriptions.set(vec![active, focused]).is_ok(),
            "dock subscriptions initialized once"
        );
    }

    pub(crate) fn navigation_adapter(&self) -> &Rc<NavigateAdapter> {
        &self.adapter
    }
    /// Gets `VisibleDockables`.
    pub fn visible_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.visible_dockables.borrow().clone()
    }
    /// Sets `VisibleDockables`, notifying only when changed.
    pub fn set_visible_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.visible_dockables,
            value,
            "VisibleDockables",
        );
    }
    /// Gets `ActiveDockable`.
    pub fn active_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.active_dockable.borrow().clone()
    }
    /// Sets `ActiveDockable`, notifying only when changed.
    pub fn set_active_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.active_dockable,
            value,
            "ActiveDockable",
        );
    }
    /// Gets `DefaultDockable`.
    pub fn default_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.default_dockable.borrow().clone()
    }
    /// Sets `DefaultDockable`, notifying only when changed.
    pub fn set_default_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.default_dockable,
            value,
            "DefaultDockable",
        );
    }
    /// Gets `FocusedDockable`.
    pub fn focused_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.focused_dockable.borrow().clone()
    }
    /// Sets `FocusedDockable`, notifying only when changed.
    pub fn set_focused_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.focused_dockable,
            value,
            "FocusedDockable",
        );
    }
    /// Gets `IsActive`.
    pub fn is_active(&self) -> bool {
        *self.is_active.borrow()
    }
    /// Sets `IsActive`, notifying only when changed.
    pub fn set_is_active(&self, value: bool) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.is_active, value, "IsActive");
    }
    /// Gets `OpenedDockablesCount`.
    pub fn opened_dockables_count(&self) -> i32 {
        *self.opened_dockables_count.borrow()
    }
    /// Sets `OpenedDockablesCount`, notifying only when changed.
    pub fn set_opened_dockables_count(&self, value: i32) {
        self.base.runtime().raise_and_set_if_changed(
            &self.opened_dockables_count,
            value,
            "OpenedDockablesCount",
        );
    }
    /// Gets `CanCloseLastDockable`.
    pub fn can_close_last_dockable(&self) -> bool {
        *self.can_close_last_dockable.borrow()
    }
    /// Sets `CanCloseLastDockable`, notifying only when changed.
    pub fn set_can_close_last_dockable(&self, value: bool) {
        self.base.runtime().raise_and_set_if_changed(
            &self.can_close_last_dockable,
            value,
            "CanCloseLastDockable",
        );
    }
    /// Gets `DockCapabilityPolicy`.
    pub fn dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>> {
        self.dock_capability_policy.borrow().clone()
    }
    /// Sets `DockCapabilityPolicy`, notifying only when changed.
    pub fn set_dock_capability_policy(&self, value: Option<Rc<dyn DockCapabilityPolicyImpl>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.dock_capability_policy,
            value,
            "DockCapabilityPolicy",
        );
    }
    /// Gets `EnableGlobalDocking`.
    pub fn enable_global_docking(&self) -> bool {
        *self.enable_global_docking.borrow()
    }
    /// Sets `EnableGlobalDocking`, notifying only when changed.
    pub fn set_enable_global_docking(&self, value: bool) {
        self.base.runtime().raise_and_set_if_changed(
            &self.enable_global_docking,
            value,
            "EnableGlobalDocking",
        );
    }
    /// Gets `CanGoBack` from the navigation history.
    pub fn can_go_back(&self) -> bool {
        self.adapter.can_go_back()
    }
    /// Gets `CanGoForward` from the navigation history.
    pub fn can_go_forward(&self) -> bool {
        self.adapter.can_go_forward()
    }
    /// Gets the `GoBack` ReactiveCommand.
    pub fn go_back(&self) -> Rc<dyn ICommand> {
        self.go_back.clone()
    }
    /// Gets the `GoForward` ReactiveCommand.
    pub fn go_forward(&self) -> Rc<dyn ICommand> {
        self.go_forward.clone()
    }
    /// Gets the `Navigate` ReactiveCommand.
    pub fn navigate(&self) -> Rc<dyn ICommand> {
        self.navigate.clone()
    }
    /// Gets the `Close` ReactiveCommand.
    pub fn close(&self) -> Rc<dyn ICommand> {
        self.close.clone()
    }
}

impl Drop for DockBase {
    fn drop(&mut self) {
        if let Some(subscriptions) = self.subscriptions.get_mut() {
            for subscription in subscriptions {
                subscription.dispose();
            }
        }
    }
}

// ICommand carries an object parameter in the source. Keep ReactiveCommand as
// the executor while translating borrowed native object handles to its typed
// parameter. No copied command engine or reflection is introduced.
#[derive(Clone)]
enum NavigateParameter {
    Root(Option<BoxedValue>),
    Invalid(TypeId),
}
impl Default for NavigateParameter {
    fn default() -> Self {
        Self::Root(None)
    }
}
impl NavigateParameter {
    fn from_object(parameter: Option<&dyn Any>) -> Self {
        let Some(parameter) = parameter else {
            return Self::default();
        };
        if let Some(value) = parameter.downcast_ref::<Rc<dyn IDockable>>() {
            return Self::Root(Some(Rc::new(value.clone())));
        }
        if let Some(value) = parameter.downcast_ref::<String>() {
            return Self::Root(Some(Rc::new(value.clone())));
        }
        if let Some(value) = parameter.downcast_ref::<BoxedValue>() {
            return Self::from_object(Some(value.as_ref()));
        }
        if let Some(value) = parameter.downcast_ref::<Option<BoxedValue>>() {
            return Self::from_object(value.as_ref().map(|value| value.as_ref()));
        }
        Self::Invalid(parameter.type_id())
    }
}
#[derive(Debug)]
struct NavigationArgumentError(TypeId);
impl std::fmt::Display for NavigationArgumentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Invalid root object type: {:?}", self.0)
    }
}
impl std::error::Error for NavigationArgumentError {}

struct NavigateCommand(ReactiveCommand<NavigateParameter, ()>);
impl NavigateCommand {
    fn new(adapter: Weak<NavigateAdapter>) -> Self {
        Self(ReactiveCommand::try_create_param(
            move |parameter: NavigateParameter| {
                if let Some(adapter) = adapter.upgrade() {
                    match parameter {
                        NavigateParameter::Root(root) => adapter.navigate(root, true),
                        NavigateParameter::Invalid(type_id) => {
                            return Err(Rc::new(NavigationArgumentError(type_id))
                                as ferroui_system::ObservableError)
                        }
                    }
                }
                Ok(())
            },
        ))
    }
}
impl ICommand for NavigateCommand {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        ICommand::can_execute(&self.0, parameter)
    }
    fn execute(&self, parameter: Option<&dyn Any>) {
        let parameter = NavigateParameter::from_object(parameter);
        ICommand::execute(&self.0, Some(&parameter));
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.can_execute_changed(handler)
    }
}

/// Implements a concrete container's complete inherited dock contracts.
/// The container embeds a DockBase field named `base`, implements DockableBaseImpl,
/// and initializes the base after its Rc construction. Extra items expose its
/// additional optional interface memberships.
#[macro_export]
macro_rules! impl_dock {
    ($type:ty $(, $extra:item)*) => {
        $crate::impl_dock!(@base $type, $crate::core::DockBase $(, $extra)*);
    };
    (@base $type:ty, $base_type:ty $(, $extra:item)*) => {
        $crate::impl_dockable!(@base $type, $base_type,
            fn as_dock(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::IDock>> { Some(self) }
            $(, $extra)*
        );
        impl $crate::__private::IDock for $type {
            fn visible_dockables(&self) -> Option<$crate::__private::ListHandle<Option<std::rc::Rc<dyn $crate::__private::IDockable>>>> { self.base.visible_dockables() }
            fn set_visible_dockables(&self, value: Option<$crate::__private::ListHandle<Option<std::rc::Rc<dyn $crate::__private::IDockable>>>>) { self.base.set_visible_dockables(value); }
            fn active_dockable(&self) -> Option<std::rc::Rc<dyn $crate::__private::IDockable>> { self.base.active_dockable() }
            fn set_active_dockable(&self, value: Option<std::rc::Rc<dyn $crate::__private::IDockable>>) { self.base.set_active_dockable(value); }
            fn default_dockable(&self) -> Option<std::rc::Rc<dyn $crate::__private::IDockable>> { self.base.default_dockable() }
            fn set_default_dockable(&self, value: Option<std::rc::Rc<dyn $crate::__private::IDockable>>) { self.base.set_default_dockable(value); }
            fn focused_dockable(&self) -> Option<std::rc::Rc<dyn $crate::__private::IDockable>> { self.base.focused_dockable() }
            fn set_focused_dockable(&self, value: Option<std::rc::Rc<dyn $crate::__private::IDockable>>) { self.base.set_focused_dockable(value); }
            fn is_active(&self) -> bool { self.base.is_active() }
            fn set_is_active(&self, value: bool) { self.base.set_is_active(value); }
            fn opened_dockables_count(&self) -> i32 { self.base.opened_dockables_count() }
            fn set_opened_dockables_count(&self, value: i32) { self.base.set_opened_dockables_count(value); }
            fn can_close_last_dockable(&self) -> bool { self.base.can_close_last_dockable() }
            fn set_can_close_last_dockable(&self, value: bool) { self.base.set_can_close_last_dockable(value); }
            fn dock_capability_policy(&self) -> Option<std::rc::Rc<dyn $crate::__private::DockCapabilityPolicyImpl>> { self.base.dock_capability_policy() }
            fn set_dock_capability_policy(&self, value: Option<std::rc::Rc<dyn $crate::__private::DockCapabilityPolicyImpl>>) { self.base.set_dock_capability_policy(value); }
            fn enable_global_docking(&self) -> bool { self.base.enable_global_docking() }
            fn set_enable_global_docking(&self, value: bool) { self.base.set_enable_global_docking(value); }
            fn can_go_back(&self) -> bool { self.base.can_go_back() }
            fn can_go_forward(&self) -> bool { self.base.can_go_forward() }
            fn go_back(&self) -> std::rc::Rc<dyn $crate::__private::ICommand> { self.base.go_back() }
            fn go_forward(&self) -> std::rc::Rc<dyn $crate::__private::ICommand> { self.base.go_forward() }
            fn navigate(&self) -> std::rc::Rc<dyn $crate::__private::ICommand> { self.base.navigate() }
            fn close(&self) -> std::rc::Rc<dyn $crate::__private::ICommand> { self.base.close() }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controls::{Document, StackDock};

    #[test]
    fn invalid_navigation_parameters_flow_through_reactive_command_errors_and_allow_next_execution()
    {
        let dock = StackDock::new();
        let adapter = Rc::new(NavigateAdapter::new(dock.clone()));
        let command = NavigateCommand::new(Rc::downgrade(&adapter));
        let errors = Rc::new(RefCell::new(Vec::new()));
        let log = errors.clone();
        let subscription = command.0.thrown_exceptions().subscribe_next(
            move |error: ferroui_system::ObservableError| log.borrow_mut().push(error.to_string()),
        );
        command.execute(Some(&42_i32));
        assert_eq!(errors.borrow().len(), 1);
        assert!(errors.borrow()[0].starts_with("Invalid root object type:"));
        assert!(command.can_execute(None));
        command.execute(Some(&Document::new().boxed()));
        assert!(dock.active_dockable().is_some());
        command.execute(Some(&None::<BoxedValue>));
        assert!(dock.active_dockable().is_none());
        subscription.dispose();
    }
}
