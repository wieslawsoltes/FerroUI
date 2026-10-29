// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{ManagedDockWindowDocument, ManagedDockableBase, ManagedDockableBaseImpl};
use crate::impl_managed_dockable;
use ferrodock_model::{collections::ListHandle, core::*};
use ferroui_reactiveui::ICommand;
use ferroui_system::IDisposable;
use std::{
    any::Any,
    cell::RefCell,
    rc::{Rc, Weak},
};
ferroui_base::ferro_thread_local! {static NO_OP:Rc<dyn ICommand>=Rc::new(NoOpCommand);
static NO_OP_DISPOSABLE:Rc<dyn IDisposable>=Rc::new(NoOpDisposable);}
struct NoOpCommand;
impl ICommand for NoOpCommand {
    fn can_execute(&self, _: Option<&dyn Any>) -> bool {
        false
    }
    fn execute(&self, _: Option<&dyn Any>) {}
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        NO_OP_DISPOSABLE.with(Clone::clone)
    }
}
struct NoOpDisposable;
impl IDisposable for NoOpDisposable {
    fn dispose(&self) {}
}
type DockableList = ListHandle<Option<Rc<dyn IDockable>>>;

/// Original dock that hosts managed floating-window documents.
pub struct ManagedWindowDock {
    base: ManagedDockableBase,
    runtime: Weak<Self>,
    visible: RefCell<Option<DockableList>>,
    active: RefCell<Option<Rc<dyn IDockable>>>,
    default: RefCell<Option<Rc<dyn IDockable>>>,
    focused: RefCell<Option<Rc<dyn IDockable>>>,
    is_active: RefCell<bool>,
    opened: RefCell<i32>,
    can_close_last: RefCell<bool>,
    policy: RefCell<Option<Rc<dyn DockCapabilityPolicyImpl>>>,
    global: RefCell<bool>,
}
impl_managed_dockable!(
    ManagedWindowDock,
    fn as_dock(self: Rc<Self>) -> Option<Rc<dyn IDock>> {
        Some(self)
    },
    fn boxed_reference(self: Rc<Self>) -> ferrodock_model::BoxedValue {
        Rc::new(self)
    }
);
impl ManagedDockableBaseImpl for ManagedWindowDock {}
impl ManagedWindowDock {
    /// Creates the original observable empty dock with IsCollapsable=false.
    pub fn new() -> Rc<Self> {
        let this = Rc::new_cyclic(|runtime| Self {
            base: ManagedDockableBase::construct(runtime.clone()),
            runtime: runtime.clone(),
            visible: RefCell::new(Some(ListHandle::new())),
            active: RefCell::new(None),
            default: RefCell::new(None),
            focused: RefCell::new(None),
            is_active: RefCell::new(false),
            opened: RefCell::new(0),
            can_close_last: RefCell::new(true),
            policy: RefCell::new(None),
            global: RefCell::new(true),
        });
        this.set_is_collapsable(false);
        this
    }
    /// Adds the original document, assigning owner/factory and selecting it once.
    pub fn add_window(&self, window: Option<Rc<ManagedDockWindowDocument>>) {
        let Some(window) = window else { return };
        if self.visible_dockables().is_none() {
            self.set_visible_dockables(Some(ListHandle::new()));
        }
        let value: Rc<dyn IDockable> = window.clone();
        if !self
            .visible_dockables()
            .expect("source initialized list")
            .contains(&Some((value).clone()))
        {
            window.set_owner(Some(self.runtime.upgrade().expect("live managed dock")));
            window.set_factory(self.factory());
            self.visible_dockables()
                .expect("source list remains present")
                .add(Some(value.clone()));
            self.set_opened_dockables_count(
                self.visible_dockables()
                    .expect("source list remains present")
                    .count() as i32,
            );
            self.set_active_dockable(Some(value));
        }
    }
    /// Removes a document and chooses the original last remaining item if necessary.
    pub fn remove_window(&self, window: Option<Rc<ManagedDockWindowDocument>>) {
        if self.visible_dockables().is_none() || window.is_none() {
            return;
        }
        let value: Rc<dyn IDockable> = window.expect("source window checked");
        if self
            .visible_dockables()
            .expect("source list checked")
            .remove(&Some((value).clone()))
        {
            self.set_opened_dockables_count(
                self.visible_dockables()
                    .expect("source list remains present")
                    .count() as i32,
            );
            if self
                .active_dockable()
                .as_ref()
                .is_some_and(|active| Rc::ptr_eq(active, &value))
            {
                let visible = self
                    .visible_dockables()
                    .expect("source list remains present");
                self.set_active_dockable(if visible.count() > 0 {
                    visible.get(visible.count() - 1)
                } else {
                    None
                });
            }
        }
    }
    fn managed_document(value: Option<Rc<dyn IDockable>>) -> Option<Rc<ManagedDockWindowDocument>> {
        value?
            .boxed_reference()
            .downcast_ref::<Rc<ManagedDockWindowDocument>>()
            .cloned()
    }
    fn notify_window_activation(
        &self,
        previous: Option<Rc<ManagedDockWindowDocument>>,
        current: Option<Rc<ManagedDockWindowDocument>>,
    ) {
        if self.factory().is_none() {
            return;
        }
        if let Some(window) = previous.and_then(|p| p.window()) {
            self.factory()
                .expect("source factory remains present")
                .on_window_deactivated(Some(window.clone()));
            if let Some(dockable) = window.layout().and_then(|l| l.active_dockable()) {
                self.factory()
                    .expect("source factory remains present")
                    .on_dockable_deactivated(Some(dockable));
            }
        }
        if let Some(window) = current.and_then(|p| p.window()) {
            self.factory()
                .expect("source factory remains present")
                .on_window_activated(Some(window.clone()));
            if let Some(dockable) = window.layout().and_then(|l| l.active_dockable()) {
                self.factory()
                    .expect("source factory remains present")
                    .on_dockable_activated(Some(dockable));
            }
        }
    }
}
impl IDock for ManagedWindowDock {
    fn visible_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.visible.borrow().clone()
    }
    fn set_visible_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.base
            .set_property(&self.visible, value, Some("VisibleDockables"));
    }
    fn active_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.active.borrow().clone()
    }
    fn set_active_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        let old = self.active_dockable();
        if match (&old, &value) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        } {
            return;
        }
        let previous = Self::managed_document(old);
        let current = Self::managed_document(value.clone());
        if self
            .base
            .set_property(&self.active, value.clone(), Some("ActiveDockable"))
        {
            if let Some(factory) = self.factory() {
                factory.init_active_dockable(
                    value,
                    Some(self.runtime.upgrade().expect("live managed dock")),
                );
            }
            self.notify_window_activation(previous, current);
        }
    }
    fn default_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.default.borrow().clone()
    }
    fn set_default_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.base
            .set_property(&self.default, value, Some("DefaultDockable"));
    }
    fn focused_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.focused.borrow().clone()
    }
    fn set_focused_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        if self
            .base
            .set_property(&self.focused, value.clone(), Some("FocusedDockable"))
        {
            if let Some(factory) = self.factory() {
                factory.on_focused_dockable_changed(value);
            }
        }
    }
    fn is_active(&self) -> bool {
        *self.is_active.borrow()
    }
    fn set_is_active(&self, value: bool) {
        self.base
            .set_property(&self.is_active, value, Some("IsActive"));
    }
    fn opened_dockables_count(&self) -> i32 {
        *self.opened.borrow()
    }
    fn set_opened_dockables_count(&self, value: i32) {
        self.base
            .set_property(&self.opened, value, Some("OpenedDockablesCount"));
    }
    fn can_close_last_dockable(&self) -> bool {
        *self.can_close_last.borrow()
    }
    fn set_can_close_last_dockable(&self, value: bool) {
        self.base
            .set_property(&self.can_close_last, value, Some("CanCloseLastDockable"));
    }
    fn dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>> {
        self.policy.borrow().clone()
    }
    fn set_dock_capability_policy(&self, value: Option<Rc<dyn DockCapabilityPolicyImpl>>) {
        self.base
            .set_property(&self.policy, value, Some("DockCapabilityPolicy"));
    }
    fn can_go_back(&self) -> bool {
        false
    }
    fn can_go_forward(&self) -> bool {
        false
    }
    fn go_back(&self) -> Rc<dyn ICommand> {
        NO_OP.with(Clone::clone)
    }
    fn go_forward(&self) -> Rc<dyn ICommand> {
        NO_OP.with(Clone::clone)
    }
    fn navigate(&self) -> Rc<dyn ICommand> {
        NO_OP.with(Clone::clone)
    }
    fn close(&self) -> Rc<dyn ICommand> {
        NO_OP.with(Clone::clone)
    }
    fn enable_global_docking(&self) -> bool {
        *self.global.borrow()
    }
    fn set_enable_global_docking(&self, value: bool) {
        self.base
            .set_property(&self.global, value, Some("EnableGlobalDocking"));
    }
}
// The shared 0.3.0 typed-path probe selects a nonnotifying specialization here.
// Fallible setter metadata uses the standard compiled INPC accessor instead;
// source setters and their exceptions/notifications remain unchanged.
ferroui_base::ferro_markup_type!(class ManagedWindowDock {
    namespace:"FerroDock.Ferro.Controls",this:Rc<ManagedWindowDock>,handles:[Rc<ManagedWindowDock>,Option<Rc<ManagedWindowDock>>],
    base:super::ManagedDockableBaseReference,
    notify_property_changed:ManagedWindowDock,
    interfaces:[Rc<dyn IDock>],constructors:[()=>ManagedWindowDock::new],
    properties:[
        VisibleDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&Rc<ManagedWindowDock>|this.visible_dockables(),try_set:|this:&Rc<ManagedWindowDock>,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.set_visible_dockables(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        ActiveDockable:Option<Rc<dyn IDockable>>{get:|this:&Rc<ManagedWindowDock>|this.active_dockable(),try_set:|this:&Rc<ManagedWindowDock>,value|{this.set_active_dockable(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        DefaultDockable:Option<Rc<dyn IDockable>>{get:|this:&Rc<ManagedWindowDock>|this.default_dockable(),try_set:|this:&Rc<ManagedWindowDock>,value|{this.set_default_dockable(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        FocusedDockable:Option<Rc<dyn IDockable>>{get:|this:&Rc<ManagedWindowDock>|this.focused_dockable(),try_set:|this:&Rc<ManagedWindowDock>,value|{this.set_focused_dockable(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        IsActive:bool{get:|this:&Rc<ManagedWindowDock>|this.is_active(),try_set:|this:&Rc<ManagedWindowDock>,value|{this.set_is_active(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        OpenedDockablesCount:i32{get:|this:&Rc<ManagedWindowDock>|this.opened_dockables_count(),try_set:|this:&Rc<ManagedWindowDock>,value|{this.set_opened_dockables_count(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CanCloseLastDockable:bool{get:|this:&Rc<ManagedWindowDock>|this.can_close_last_dockable(),try_set:|this:&Rc<ManagedWindowDock>,value|{this.set_can_close_last_dockable(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        DockCapabilityPolicy:Option<Rc<dyn DockCapabilityPolicyImpl>>{get:|this:&Rc<ManagedWindowDock>|this.dock_capability_policy(),try_set:|this:&Rc<ManagedWindowDock>,value|{this.set_dock_capability_policy(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CanGoBack:bool{get:|this:&Rc<ManagedWindowDock>|this.can_go_back()},CanGoForward:bool{get:|this:&Rc<ManagedWindowDock>|this.can_go_forward()},
        GoBack:Rc<dyn ICommand>{get:|this:&Rc<ManagedWindowDock>|this.go_back()},GoForward:Rc<dyn ICommand>{get:|this:&Rc<ManagedWindowDock>|this.go_forward()},Navigate:Rc<dyn ICommand>{get:|this:&Rc<ManagedWindowDock>|this.navigate()},Close:Rc<dyn ICommand>{get:|this:&Rc<ManagedWindowDock>|this.close()},
        EnableGlobalDocking:bool{get:|this:&Rc<ManagedWindowDock>|this.enable_global_docking(),try_set:|this:&Rc<ManagedWindowDock>,value|{this.set_enable_global_docking(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],methods:[fn AddWindow(Option<Rc<ManagedDockWindowDocument>>)=>(|this:&Rc<ManagedWindowDock>,window|this.add_window(window)),fn RemoveWindow(Option<Rc<ManagedDockWindowDocument>>)=>(|this:&Rc<ManagedWindowDock>,window|this.remove_window(window))],
});
