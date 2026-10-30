//! Test-only recording factory for the default parameter adapters.
use crate::test_support::TestNode;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::events::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::IDisposable;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
#[allow(dead_code)] // Variant payloads are inspected by the shared ReactiveUI model suite.
pub(crate) enum ModelCall {
    InitDockable(Rc<dyn IDockable>, Option<Rc<dyn IDockable>>),
    InitActive(Option<Rc<dyn IDockable>>, Rc<dyn IDock>),
    FocusedChanged(Option<Rc<dyn IDockable>>),
    Add(Rc<dyn IDock>, Rc<dyn IDockable>),
    SetActive(Rc<dyn IDockable>),
    SetFocused(Rc<dyn IDock>, Option<Rc<dyn IDockable>>),
    ActiveChanged(Option<Rc<dyn IDockable>>),
    WindowActivated(Option<Rc<dyn IDockWindow>>),
    Move(
        Rc<dyn IDock>,
        Rc<dyn IDock>,
        Rc<dyn IDockable>,
        Option<Rc<dyn IDockable>>,
    ),
}
type ActiveCallback = Rc<dyn Fn(Option<Rc<dyn IDockable>>, Rc<dyn IDock>)>;
type AddCallback = Rc<dyn Fn(Rc<dyn IDock>, Rc<dyn IDockable>)>;
type OperationCallback = Rc<dyn Fn(&str, Rc<dyn IDockable>)>;
type ResolveCallback = Rc<dyn Fn(Rc<dyn IDockable>) -> DockingWindowState>;
type Creator<T> = Rc<dyn Fn() -> Rc<T>>;
type SelectionCallback =
    Rc<dyn Fn(Rc<dyn IDock>, Rc<dyn IDockable>, i32) -> Option<Rc<dyn IDockable>>>;
type RemoveCallback = Rc<dyn Fn(Rc<dyn IDockable>, bool)>;
type CollapseCallback = Rc<dyn Fn(Rc<dyn IDock>)>;
type WindowCallback = Rc<dyn Fn(Option<Rc<dyn IDockWindow>>)>;
type StateCallback = Rc<dyn Fn(Rc<dyn IDockable>)>;
type SourceClosingCallback = Rc<dyn Fn(Option<Rc<dyn IDockable>>) -> bool>;
type TrackItemCallback = Rc<dyn Fn(Rc<dyn IDockable>, Rc<dyn IDockable>, Option<BoxedValue>)>;
type TrackedOwnerCallback = Rc<dyn Fn(Rc<dyn IDockable>) -> Option<Rc<dyn IDockable>>>;

#[derive(Default)]
pub(crate) struct RecordingFactory {
    pub(crate) base: OnceCell<ferrodock_model::FactoryBase>,
    pub(crate) controls: ListHandle<Rc<dyn IDockControl>>,
    pub(crate) create_plain_lists: std::cell::Cell<bool>,
    pub(crate) create_list_calls: std::cell::Cell<usize>,
    pub(crate) host_result: RefCell<Option<Rc<dyn IHostWindow>>>,
    pub(crate) legacy_host_requests: RefCell<Vec<String>>,
    pub(crate) supports_host_factory: std::cell::Cell<bool>,
    pub(crate) host_factory_result: RefCell<Option<Rc<dyn IHostWindow>>>,
    pub(crate) host_factory_requests: RefCell<Vec<Rc<dyn IDockWindow>>>,
    pub(crate) calls: RefCell<Vec<(&'static str, Rc<dyn IDockable>)>>,
    pub(crate) active_dockable_requests: RefCell<Vec<Rc<dyn IDockable>>>,
    pub(crate) model_calls: RefCell<Vec<ModelCall>>,
    pub(crate) on_init_active: RefCell<Option<ActiveCallback>>,
    pub(crate) tool_dock_creator: RefCell<Option<Creator<dyn IToolDock>>>,
    pub(crate) root_dock_creator: RefCell<Option<Creator<dyn IRootDock>>>,
    pub(crate) proportional_dock_creator: RefCell<Option<Creator<dyn IProportionalDock>>>,
    pub(crate) dock_dock_creator: RefCell<Option<Creator<dyn IDockDock>>>,
    pub(crate) document_dock_creator: RefCell<Option<Creator<dyn IDocumentDock>>>,
    pub(crate) splitter_creator: RefCell<Option<Creator<dyn IProportionalDockSplitter>>>,
    pub(crate) window_creator: RefCell<Option<Creator<dyn IDockWindow>>>,
    pub(crate) on_selection_after_remove: RefCell<Option<SelectionCallback>>,
    pub(crate) on_remove: RefCell<Option<RemoveCallback>>,
    pub(crate) on_collapse: RefCell<Option<CollapseCallback>>,
    pub(crate) tabs_layout_requests: RefCell<Vec<(Rc<dyn IDockable>, DocumentTabLayout)>>,
    pub(crate) layout_mode_requests: RefCell<Vec<(Rc<dyn IDockable>, DocumentLayoutMode)>>,
    pub(crate) on_add: RefCell<Option<AddCallback>>,
    pub(crate) resolved_root: RefCell<Option<Rc<dyn IRootDock>>>,
    pub(crate) find_root_calls: std::cell::Cell<usize>,
    pub(crate) supports_state_sync: std::cell::Cell<bool>,
    pub(crate) state_requests: RefCell<Vec<(Rc<dyn IDockable>, DockingWindowStateProperty)>>,
    pub(crate) on_operation: RefCell<Option<OperationCallback>>,
    pub(crate) on_resolve_state: RefCell<Option<ResolveCallback>>,
    pub(crate) on_update_state: RefCell<Option<StateCallback>>,
    pub(crate) supports_close_policy: std::cell::Cell<bool>,
    pub(crate) preserve_layout: std::cell::Cell<bool>,
    pub(crate) close_policy_requests: RefCell<Vec<Rc<dyn IDockWindow>>>,
    pub(crate) on_notify_closed: RefCell<Option<WindowCallback>>,
    pub(crate) on_notify_removed: RefCell<Option<WindowCallback>>,
    pub(crate) execute_base: std::cell::Cell<bool>,
    pub(crate) simulate_close: std::cell::Cell<bool>,
    pub(crate) on_close_core: RefCell<Option<StateCallback>>,
    pub(crate) on_source_closing: RefCell<Option<SourceClosingCallback>>,
    pub(crate) on_tracked_owner: RefCell<Option<TrackedOwnerCallback>>,
    pub(crate) on_track_item: RefCell<Option<TrackItemCallback>>,

    pub(crate) suppress_active_focus: std::cell::Cell<bool>,
}

impl RecordingFactory {
    fn record_operation(&self, name: &'static str, dockable: Rc<dyn IDockable>) {
        self.calls.borrow_mut().push((name, dockable.clone()));
        let callback = self.on_operation.borrow().clone();
        if let Some(callback) = callback {
            callback(name, dockable);
        }
    }
    pub(crate) fn with_base() -> Rc<Self> {
        let factory = Rc::new(Self::default());
        assert!(factory
            .base
            .set(ferrodock_model::FactoryBase::construct(Rc::downgrade(
                &factory
            )))
            .is_ok());
        factory
    }
}

impl ferrodock_model::FactoryBaseImpl for RecordingFactory {
    fn factory_base(&self) -> &ferrodock_model::FactoryBase {
        self.base.get().expect("factory component was constructed")
    }
    fn track_items_source_dockable_with_item(
        &self,
        dockable: Rc<dyn IDockable>,
        owner: Rc<dyn IDockable>,
        item: Option<BoxedValue>,
    ) {
        let callback = self.on_track_item.borrow().clone();
        if let Some(callback) = callback {
            callback(dockable, owner, item);
        } else {
            self.base
                .get()
                .unwrap()
                .track_items_source_dockable_with_item(dockable, owner, item);
        }
    }
    fn close_dockable_core(&self, dockable: Rc<dyn IDockable>) {
        let callback = self.on_close_core.borrow().clone();
        if let Some(callback) = callback {
            callback(dockable);
        } else {
            self.base.get().unwrap().close_dockable_core(dockable);
        }
    }
    fn handle_items_source_dockable_closing(&self, dockable: Rc<dyn IDockable>) -> bool {
        let callback = self.on_source_closing.borrow().clone();
        if let Some(callback) = callback {
            callback(Some(dockable))
        } else {
            self.base
                .get()
                .unwrap()
                .handle_items_source_dockable_closing(dockable)
        }
    }
    fn get_tracked_items_source_owner(
        &self,
        dockable: Rc<dyn IDockable>,
    ) -> Option<Rc<dyn IDockable>> {
        let callback = self.on_tracked_owner.borrow().clone();
        if let Some(callback) = callback {
            callback(dockable)
        } else {
            self.base
                .get()
                .unwrap()
                .get_tracked_items_source_owner(dockable)
        }
    }
    fn notify_window_closed(&self, window: Option<Rc<dyn IDockWindow>>) {
        let callback = self.on_notify_closed.borrow().clone();
        if let Some(callback) = callback {
            callback(window);
        } else {
            self.base.get().unwrap().notify_window_closed(window);
        }
    }
    fn notify_window_removed(&self, window: Option<Rc<dyn IDockWindow>>) {
        let callback = self.on_notify_removed.borrow().clone();
        if let Some(callback) = callback {
            callback(window);
        } else {
            self.base.get().unwrap().notify_window_removed(window);
        }
    }
    fn get_active_dockable_after_removal(
        &self,
        dock: Option<Rc<dyn IDock>>,
        removed: Option<Rc<dyn IDockable>>,
        index: i32,
    ) -> Option<Rc<dyn IDockable>> {
        let callback = self.on_selection_after_remove.borrow().clone();
        if let Some(callback) = callback {
            callback(
                dock.expect("nonnull selection fixture"),
                removed.expect("nonnull selection fixture"),
                index,
            )
        } else {
            self.base
                .get()
                .unwrap()
                .get_active_dockable_after_removal(dock, removed, index)
        }
    }
    fn should_focus_active_dockable(
        &self,
        _dockable: Option<Rc<dyn IDockable>>,
        _owner: Option<Rc<dyn IDock>>,
    ) -> bool {
        !self.suppress_active_focus.get()
    }
    fn resolve_docking_window_state(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> DockingWindowState {
        let callback = self.on_resolve_state.borrow().clone();
        if let Some(callback) = callback {
            callback(dockable.expect("nonnull recording resolver fixture"))
        } else {
            self.base
                .get()
                .unwrap()
                .resolve_docking_window_state(dockable)
        }
    }
    fn update_docking_window_state(&self, dockable: Option<Rc<dyn IDockable>>) {
        let callback = self.on_update_state.borrow().clone();
        if let Some(callback) = callback {
            callback(dockable.expect("nonnull recording update fixture"));
        } else {
            self.base
                .get()
                .unwrap()
                .update_docking_window_state(dockable);
        }
    }
}

impl IDockWindowClosePolicy for RecordingFactory {
    fn preserve_layout_on_host_close(&self, window: Rc<dyn IDockWindow>) -> bool {
        self.close_policy_requests.borrow_mut().push(window);
        self.preserve_layout.get()
    }
}

impl IDockWindowHostFactory for RecordingFactory {
    fn create_host_window(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
    ) -> Option<Rc<dyn IHostWindow>> {
        self.host_factory_requests
            .borrow_mut()
            .push(window.expect("nonnull host recording fixture"));
        self.host_factory_result.borrow().clone()
    }
}

impl IDockingWindowStateSync for RecordingFactory {
    fn on_docking_window_state_property_changed(
        &self,
        dockable: Rc<dyn IDockable>,
        property: DockingWindowStateProperty,
    ) {
        self.state_requests
            .borrow_mut()
            .push((dockable.clone(), property));
        if self.supports_state_sync.get() {
            if let Some(base) = self.base.get() {
                IDockingWindowStateSync::on_docking_window_state_property_changed(
                    base, dockable, property,
                );
            }
        }
    }
}
impl IFactoryEvents for RecordingFactory {
    fn active_dockable_changed(
        &self,
        handler: Rc<dyn Fn(&ActiveDockableChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.active_dockable_changed(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn focused_dockable_changed(
        &self,
        handler: Rc<dyn Fn(&FocusedDockableChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.focused_dockable_changed(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn global_dock_tracking_changed(
        &self,
        handler: Rc<dyn Fn(&GlobalDockTrackingChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.global_dock_tracking_changed(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_init(&self, handler: Rc<dyn Fn(&DockableInitEventArgs)>) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_init(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_added(&self, handler: Rc<dyn Fn(&DockableAddedEventArgs)>) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_added(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_removed(
        &self,
        handler: Rc<dyn Fn(&DockableRemovedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_removed(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_closing(
        &self,
        handler: Rc<dyn Fn(&DockableClosingEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_closing(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_closed(
        &self,
        handler: Rc<dyn Fn(&DockableClosedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_closed(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_moved(&self, handler: Rc<dyn Fn(&DockableMovedEventArgs)>) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_moved(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_docked(
        &self,
        handler: Rc<dyn Fn(&DockableDockedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_docked(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_undocked(
        &self,
        handler: Rc<dyn Fn(&DockableUndockedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_undocked(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_swapped(
        &self,
        handler: Rc<dyn Fn(&DockableSwappedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_swapped(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_pinned(
        &self,
        handler: Rc<dyn Fn(&DockablePinnedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_pinned(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_unpinned(
        &self,
        handler: Rc<dyn Fn(&DockableUnpinnedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_unpinned(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_hidden(
        &self,
        handler: Rc<dyn Fn(&DockableHiddenEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_hidden(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_restored(
        &self,
        handler: Rc<dyn Fn(&DockableRestoredEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_restored(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_opened(&self, handler: Rc<dyn Fn(&WindowOpenedEventArgs)>) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_opened(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_closing(&self, handler: Rc<dyn Fn(&WindowClosingEventArgs)>) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_closing(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_closed(&self, handler: Rc<dyn Fn(&WindowClosedEventArgs)>) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_closed(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_added(&self, handler: Rc<dyn Fn(&WindowAddedEventArgs)>) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_added(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_removed(&self, handler: Rc<dyn Fn(&WindowRemovedEventArgs)>) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_removed(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_move_drag_begin(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragBeginEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_move_drag_begin(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_move_drag(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_move_drag(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_move_drag_end(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragEndEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_move_drag_end(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_activated(
        &self,
        handler: Rc<dyn Fn(&WindowActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_activated(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_activated(
        &self,
        handler: Rc<dyn Fn(&DockableActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_activated(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn window_deactivated(
        &self,
        handler: Rc<dyn Fn(&WindowDeactivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.window_deactivated(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn dockable_deactivated(
        &self,
        handler: Rc<dyn Fn(&DockableDeactivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        if let Some(base) = self.base.get() {
            return base.dockable_deactivated(handler);
        }
        let _ = handler;
        Disposable::empty()
    }
    fn on_active_dockable_changed(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.model_calls
            .borrow_mut()
            .push(ModelCall::ActiveChanged(dockable.clone()));
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .on_active_dockable_changed(dockable);
        }
    }
    fn on_focused_dockable_changed(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.model_calls
            .borrow_mut()
            .push(ModelCall::FocusedChanged(dockable.clone()));
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .on_focused_dockable_changed(dockable);
        }
    }
    fn on_dockable_init(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_init(dockable);
    }
    fn on_dockable_added(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_added(dockable)
    }
    fn on_dockable_removed(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_removed(dockable)
    }
    fn on_dockable_closing(&self, dockable: Option<Rc<dyn IDockable>>) -> bool {
        self.base.get().unwrap().on_dockable_closing(dockable)
    }
    fn on_dockable_closed(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_closed(dockable)
    }
    fn on_dockable_moved(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_moved(dockable)
    }
    fn on_dockable_docked(&self, dockable: Option<Rc<dyn IDockable>>, operation: DockOperation) {
        self.base
            .get()
            .unwrap()
            .on_dockable_docked(dockable, operation)
    }
    fn on_dockable_undocked(&self, dockable: Option<Rc<dyn IDockable>>, operation: DockOperation) {
        self.base
            .get()
            .unwrap()
            .on_dockable_undocked(dockable, operation)
    }
    fn on_dockable_swapped(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_swapped(dockable)
    }
    fn on_dockable_pinned(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_pinned(dockable)
    }
    fn on_dockable_unpinned(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_unpinned(dockable)
    }
    fn on_dockable_hidden(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_hidden(dockable)
    }
    fn on_dockable_restored(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().on_dockable_restored(dockable)
    }
    fn on_window_opened(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.base.get().unwrap().on_window_opened(window)
    }
    fn on_window_closing(&self, window: Option<Rc<dyn IDockWindow>>) -> bool {
        self.base.get().unwrap().on_window_closing(window)
    }
    fn on_window_closed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.base.get().unwrap().on_window_closed(window)
    }
    fn on_window_added(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.base.get().unwrap().on_window_added(window)
    }
    fn on_window_removed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.base.get().unwrap().on_window_removed(window)
    }
    fn on_window_move_drag_begin(&self, window: Option<Rc<dyn IDockWindow>>) -> bool {
        self.base.get().unwrap().on_window_move_drag_begin(window)
    }
    fn on_window_move_drag(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.base.get().unwrap().on_window_move_drag(window)
    }
    fn on_window_move_drag_end(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.base.get().unwrap().on_window_move_drag_end(window)
    }
    fn on_window_activated(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.model_calls
            .borrow_mut()
            .push(ModelCall::WindowActivated(window.clone()));
        if self.execute_base.get() {
            self.base.get().unwrap().on_window_activated(window);
        }
    }
    fn on_window_activated_with_root(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        root_dock: Option<Rc<dyn IRootDock>>,
    ) {
        self.base
            .get()
            .unwrap()
            .on_window_activated_with_root(window, root_dock);
    }
    fn on_dockable_activated(&self, dockable: Option<Rc<dyn IDockable>>) {
        if let Some(dockable) = &dockable {
            self.record_operation("on_dockable_activated", dockable.clone());
        }
        if self.execute_base.get() {
            self.base.get().unwrap().on_dockable_activated(dockable);
        }
    }
    fn on_window_deactivated(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.base.get().unwrap().on_window_deactivated(window);
    }
    fn on_dockable_deactivated(&self, dockable: Option<Rc<dyn IDockable>>) {
        if let Some(dockable) = &dockable {
            self.record_operation("on_dockable_deactivated", dockable.clone());
        }
        if self.execute_base.get() {
            self.base.get().unwrap().on_dockable_deactivated(dockable);
        }
    }
}
impl IFactory for RecordingFactory {
    fn as_factory_base(self: Rc<Self>) -> Option<Rc<dyn ferrodock_model::FactoryBaseImpl>> {
        self.base
            .get()
            .map(|_| self.clone() as Rc<dyn ferrodock_model::FactoryBaseImpl>)
    }
    fn create_dockable_list(
        &self,
        items: Vec<Option<Rc<dyn IDockable>>>,
    ) -> ListHandle<Option<Rc<dyn IDockable>>> {
        self.create_list(items)
    }
    fn as_dock_window_close_policy(self: Rc<Self>) -> Option<Rc<dyn IDockWindowClosePolicy>> {
        self.supports_close_policy.get().then_some(self)
    }

    fn as_dock_window_host_factory(self: Rc<Self>) -> Option<Rc<dyn IDockWindowHostFactory>> {
        if self.supports_host_factory.get() {
            Some(self)
        } else {
            None
        }
    }
    fn as_docking_window_state_sync(self: Rc<Self>) -> Option<Rc<dyn IDockingWindowStateSync>> {
        if self.supports_state_sync.get() {
            Some(self)
        } else {
            None
        }
    }
    fn visible_dockable_controls(&self) -> DockableControlMap {
        panic!("unused recording-factory member: visible_dockable_controls")
    }
    fn visible_root_controls(&self) -> DockableObjectMap {
        panic!("unused recording-factory member: visible_root_controls")
    }
    fn pinned_dockable_controls(&self) -> DockableControlMap {
        panic!("unused recording-factory member: pinned_dockable_controls")
    }
    fn pinned_root_controls(&self) -> DockableObjectMap {
        panic!("unused recording-factory member: pinned_root_controls")
    }
    fn tab_dockable_controls(&self) -> DockableControlMap {
        panic!("unused recording-factory member: tab_dockable_controls")
    }
    fn tab_root_controls(&self) -> DockableObjectMap {
        panic!("unused recording-factory member: tab_root_controls")
    }
    fn tool_controls(&self) -> DockableObjectMap {
        panic!("unused recording-factory member: tool_controls")
    }
    fn document_controls(&self) -> DockableObjectMap {
        panic!("unused recording-factory member: document_controls")
    }
    fn dock_controls(&self) -> ListHandle<Rc<dyn IDockControl>> {
        self.controls.clone()
    }
    fn host_windows(&self) -> ListHandle<Rc<dyn IHostWindow>> {
        panic!("unused recording-factory member: host_windows")
    }
    fn global_dock_tracking_state(&self) -> GlobalDockTrackingState {
        self.base.get().unwrap().global_dock_tracking_state()
    }
    fn current_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.base.get().unwrap().current_dockable()
    }
    fn current_root_dock(&self) -> Option<Rc<dyn IRootDock>> {
        self.base.get().unwrap().current_root_dock()
    }
    fn current_dock_window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.base.get().unwrap().current_dock_window()
    }
    fn current_host_window(&self) -> Option<Rc<dyn IHostWindow>> {
        self.base.get().unwrap().current_host_window()
    }
    fn hide_tools_on_close(&self) -> bool {
        self.base
            .get()
            .expect("factory base installed")
            .hide_tools_on_close()
    }
    fn set_hide_tools_on_close(&self, value: bool) {
        self.base
            .get()
            .expect("factory base installed")
            .set_hide_tools_on_close(value);
    }
    fn hide_documents_on_close(&self) -> bool {
        self.base
            .get()
            .expect("factory base installed")
            .hide_documents_on_close()
    }
    fn set_hide_documents_on_close(&self, value: bool) {
        self.base
            .get()
            .expect("factory base installed")
            .set_hide_documents_on_close(value);
    }
    fn default_context_locator(&self) -> Option<ContextFactory> {
        self.base
            .get()
            .expect("factory base installed")
            .default_context_locator()
    }
    fn set_default_context_locator(&self, value: Option<ContextFactory>) {
        self.base
            .get()
            .expect("factory base installed")
            .set_default_context_locator(value);
    }
    fn default_host_window_locator(&self) -> Option<HostWindowFactory> {
        self.base
            .get()
            .expect("factory base installed")
            .default_host_window_locator()
    }
    fn set_default_host_window_locator(&self, value: Option<HostWindowFactory>) {
        self.base
            .get()
            .expect("factory base installed")
            .set_default_host_window_locator(value);
    }
    fn context_locator(&self) -> Option<ContextLocatorMap> {
        self.base
            .get()
            .expect("factory base installed")
            .context_locator()
    }
    fn set_context_locator(&self, value: Option<ContextLocatorMap>) {
        self.base
            .get()
            .expect("factory base installed")
            .set_context_locator(value);
    }
    fn host_window_locator(&self) -> Option<HostWindowLocatorMap> {
        self.base
            .get()
            .expect("factory base installed")
            .host_window_locator()
    }
    fn set_host_window_locator(&self, value: Option<HostWindowLocatorMap>) {
        self.base
            .get()
            .expect("factory base installed")
            .set_host_window_locator(value);
    }
    fn dockable_locator(&self) -> Option<DockableLocatorMap> {
        self.base
            .get()
            .expect("factory base installed")
            .dockable_locator()
    }
    fn set_dockable_locator(&self, value: Option<DockableLocatorMap>) {
        self.base
            .get()
            .expect("factory base installed")
            .set_dockable_locator(value);
    }
    fn create_list<T: Clone + PartialEq + 'static>(&self, items: Vec<T>) -> ListHandle<T>
    where
        Self: Sized,
    {
        self.create_list_calls.set(self.create_list_calls.get() + 1);
        if self.create_plain_lists.get() {
            ListHandle::from_plain(items)
        } else {
            ListHandle::from_items(items)
        }
    }
    fn create_root_dock(&self) -> Rc<dyn IRootDock> {
        let creator = self
            .root_dock_creator
            .borrow()
            .clone()
            .expect("test supplies concrete root_dock creation");
        creator()
    }
    fn create_proportional_dock(&self) -> Rc<dyn IProportionalDock> {
        let creator = self
            .proportional_dock_creator
            .borrow()
            .clone()
            .expect("test supplies concrete proportional_dock creation");
        creator()
    }
    fn create_dock_dock(&self) -> Rc<dyn IDockDock> {
        let creator = self
            .dock_dock_creator
            .borrow()
            .clone()
            .expect("test supplies concrete dock_dock creation");
        creator()
    }
    fn create_stack_dock(&self) -> Rc<dyn IStackDock> {
        panic!("unused recording-factory member: create_stack_dock")
    }
    fn create_grid_dock(&self) -> Rc<dyn IGridDock> {
        panic!("unused recording-factory member: create_grid_dock")
    }
    fn create_wrap_dock(&self) -> Rc<dyn IWrapDock> {
        panic!("unused recording-factory member: create_wrap_dock")
    }
    fn create_uniform_grid_dock(&self) -> Rc<dyn IUniformGridDock> {
        panic!("unused recording-factory member: create_uniform_grid_dock")
    }
    fn create_proportional_dock_splitter(&self) -> Rc<dyn IProportionalDockSplitter> {
        let creator = self
            .splitter_creator
            .borrow()
            .clone()
            .expect("test supplies concrete proportional_dock_splitter creation");
        creator()
    }
    fn create_grid_dock_splitter(&self) -> Rc<dyn IGridDockSplitter> {
        panic!("unused recording-factory member: create_grid_dock_splitter")
    }
    fn create_tool_dock(&self) -> Rc<dyn IToolDock> {
        let creator = self
            .tool_dock_creator
            .borrow()
            .clone()
            .expect("test supplies concrete tool dock creation");
        creator()
    }
    fn create_document_dock(&self) -> Rc<dyn IDocumentDock> {
        let creator = self
            .document_dock_creator
            .borrow()
            .clone()
            .expect("test supplies concrete document_dock creation");
        creator()
    }
    fn create_split_view_dock(&self) -> Rc<dyn ISplitViewDock> {
        panic!("unused recording-factory member: create_split_view_dock")
    }
    fn create_dock_window(&self) -> Rc<dyn IDockWindow> {
        let creator = self
            .window_creator
            .borrow()
            .clone()
            .expect("test supplies concrete dock_window creation");
        creator()
    }
    fn create_document(&self) -> Rc<dyn IDocument> {
        panic!("unused recording-factory member: create_document")
    }
    fn create_tool(&self) -> Rc<dyn ITool> {
        panic!("unused recording-factory member: create_tool")
    }
    fn create_layout(&self) -> Option<Rc<dyn IRootDock>> {
        panic!("unused recording-factory member: create_layout")
    }
    fn get_context(&self, id: String) -> Option<BoxedValue> {
        self.base
            .get()
            .expect("factory base installed")
            .get_context(id)
    }
    fn get_host_window(&self, id: String) -> Option<Rc<dyn IHostWindow>> {
        self.legacy_host_requests.borrow_mut().push(id.clone());
        if let Some(base) = self.base.get() {
            return base.get_host_window(id);
        }
        self.host_result.borrow().clone()
    }
    fn get_dockable<T: IDockable + ?Sized + 'static>(&self, id: String) -> Option<Rc<T>>
    where
        Self: Sized,
    {
        self.base.get().unwrap().get_dockable(id)
    }
    fn get_container_from_item(&self, item: Option<BoxedValue>) -> Option<Rc<dyn IDockable>> {
        self.base.get().unwrap().get_container_from_item(item)
    }
    fn init_layout(&self, layout: Rc<dyn IDockable>) {
        self.base.get().unwrap().init_layout(layout);
    }
    fn init_dockable(&self, dockable: Option<Rc<dyn IDockable>>, owner: Option<Rc<dyn IDockable>>) {
        let dockable = dockable.expect("nonnull recording fixture");
        self.model_calls
            .borrow_mut()
            .push(ModelCall::InitDockable(dockable.clone(), owner.clone()));
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .init_dockable(Some(dockable), owner);
        }
    }
    fn init_dock_window(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        self.base
            .get()
            .expect("factory base installed")
            .init_dock_window(window, owner);
    }
    fn init_dock_window_with_host(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
        host: Option<Rc<dyn IHostWindow>>,
    ) {
        self.base
            .get()
            .expect("factory base installed")
            .init_dock_window_with_host(window, owner, host);
    }
    fn init_active_dockable(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDock>>,
    ) {
        self.model_calls.borrow_mut().push(ModelCall::InitActive(
            dockable.clone(),
            owner.clone().expect("nonnull recording fixture"),
        ));
        let callback = self.on_init_active.borrow().clone();
        if let Some(callback) = callback {
            callback(dockable, owner.expect("nonnull recording fixture"));
        } else if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .init_active_dockable(dockable, owner);
        }
    }
    fn activate_window(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.base.get().unwrap().activate_window(dockable);
    }
    fn set_active_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        let dockable = dockable.expect("nonnull recording fixture");
        self.active_dockable_requests
            .borrow_mut()
            .push(dockable.clone());
        self.model_calls
            .borrow_mut()
            .push(ModelCall::SetActive(dockable.clone()));
        if self.execute_base.get() {
            self.base.get().unwrap().set_active_dockable(Some(dockable));
        }
    }
    fn set_focused_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
    ) {
        self.model_calls.borrow_mut().push(ModelCall::SetFocused(
            dock.clone().expect("nonnull recording fixture"),
            dockable.clone(),
        ));
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .set_focused_dockable(dock, dockable);
        }
    }
    fn find_root(
        &self,
        _dockable: Option<Rc<dyn IDockable>>,
        _predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        self.find_root_calls.set(self.find_root_calls.get() + 1);
        if let Some(base) = self.base.get() {
            return base.find_root(_dockable, _predicate);
        }
        let root = self.resolved_root.borrow().clone();
        root.filter(|root| {
            _predicate
                .as_ref()
                .is_none_or(|predicate| predicate(root.clone()))
        })
    }
    fn find_dockable(
        &self,
        _dock: Option<Rc<dyn IDock>>,
        _predicate: Option<DockablePredicate>,
    ) -> Option<Rc<dyn IDockable>> {
        self.base
            .get()
            .expect("factory base installed")
            .find_dockable(_dock, _predicate)
    }
    fn find(&self, _predicate: Option<DockablePredicate>) -> ferrodock_model::DockableQuery {
        self.base
            .get()
            .expect("factory base installed")
            .find(_predicate)
    }
    fn find_in_dock(
        &self,
        _dock: Option<Rc<dyn IDock>>,
        _predicate: Option<DockablePredicate>,
    ) -> ferrodock_model::DockableQuery {
        self.base
            .get()
            .expect("factory base installed")
            .find_in_dock(_dock, _predicate)
    }
    fn add_dockable(&self, dock: Option<Rc<dyn IDock>>, dockable: Option<Rc<dyn IDockable>>) {
        let dock = dock.expect("nonnull recording fixture");
        let dockable = dockable.expect("nonnull recording fixture");
        self.model_calls
            .borrow_mut()
            .push(ModelCall::Add(dock.clone(), dockable.clone()));
        let callback = self.on_add.borrow().clone();
        if let Some(callback) = callback {
            callback(dock, dockable);
        } else if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .add_dockable(Some(dock), Some(dockable));
        }
    }
    fn insert_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        index: i32,
    ) {
        self.base
            .get()
            .unwrap()
            .insert_dockable(dock, dockable, index);
    }
    fn remove_dockable(&self, dockable: Option<Rc<dyn IDockable>>, collapse: bool) {
        let callback = self.on_remove.borrow().clone();
        if let Some(callback) = callback {
            callback(
                dockable.expect("nonnull recording removal fixture"),
                collapse,
            );
        } else if self.execute_base.get() {
            self.base.get().unwrap().remove_dockable(dockable, collapse);
        } else {
            panic!("unused recording-factory removal");
        }
    }
    fn move_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        self.base
            .get()
            .unwrap()
            .move_dockable(dock, source_dockable, target_dockable);
    }
    fn move_dockable_between_docks(
        &self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        self.model_calls.borrow_mut().push(ModelCall::Move(
            source_dock.clone().expect("nonnull move fixture"),
            target_dock.clone().expect("nonnull move fixture"),
            source_dockable.clone().expect("nonnull move fixture"),
            target_dockable.clone(),
        ));
        self.record_operation(
            "move_dockable",
            source_dockable.clone().expect("nonnull move fixture"),
        );
        if self.execute_base.get() {
            self.base.get().unwrap().move_dockable_between_docks(
                source_dock,
                target_dock,
                source_dockable,
                target_dockable,
            );
        }
    }
    fn swap_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        self.base
            .get()
            .unwrap()
            .swap_dockable(dock, source_dockable, target_dockable);
    }
    fn swap_dockable_between_docks(
        &self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        self.base.get().unwrap().swap_dockable_between_docks(
            source_dock,
            target_dock,
            source_dockable,
            target_dockable,
        );
    }
    fn pin_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.record_operation(
            "pin_dockable",
            dockable.clone().expect("nonnull pin fixture"),
        );
        if self.execute_base.get() {
            self.base.get().unwrap().pin_dockable(dockable);
        }
    }
    fn unpin_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        if let Some(base) = self.base.get() {
            base.unpin_dockable(dockable);
        } else {
            self.record_operation("unpin_dockable", dockable.expect("nonnull unpin fixture"));
        }
    }
    fn preview_pinned_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.record_operation(
            "preview_pinned_dockable",
            dockable.clone().expect("nonnull preview fixture"),
        );
        if self.execute_base.get() {
            self.base.get().unwrap().preview_pinned_dockable(dockable);
        }
    }
    fn toggle_preview_pinned_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.record_operation(
            "toggle_preview_pinned_dockable",
            dockable.clone().expect("nonnull preview fixture"),
        );
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .toggle_preview_pinned_dockable(dockable);
        }
    }
    fn hide_previewing_dockables(&self, root_dock: Option<Rc<dyn IRootDock>>) {
        self.base
            .get()
            .unwrap()
            .hide_previewing_dockables(root_dock);
    }
    fn is_dockable_pinned(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        root_dock: Option<Rc<dyn IRootDock>>,
    ) -> bool {
        self.base
            .get()
            .expect("factory component was constructed")
            .is_dockable_pinned(dockable, root_dock)
    }
    fn move_pinned_dockable(
        &self,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        self.base
            .get()
            .unwrap()
            .move_pinned_dockable(source_dockable, target_dockable);
    }
    fn float_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.record_operation(
            "float_dockable",
            dockable.clone().expect("nonnull floating fixture"),
        );
        if self.execute_base.get() {
            self.base.get().unwrap().float_dockable(dockable);
        }
    }
    fn float_dockable_with_options(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        self.base
            .get()
            .unwrap()
            .float_dockable_with_options(dockable, options);
    }
    fn float_all_dockables(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.record_operation(
            "float_all_dockables",
            dockable.clone().expect("nonnull floating fixture"),
        );
        if self.execute_base.get() {
            self.base.get().unwrap().float_all_dockables(dockable);
        }
    }
    fn float_all_dockables_with_options(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        self.base
            .get()
            .unwrap()
            .float_all_dockables_with_options(dockable, options);
    }
    fn dock_as_document(&self, dockable: Rc<dyn IDockable>) {
        self.record_operation("dock_as_document", dockable.clone());
        if self.execute_base.get() {
            self.base.get().unwrap().dock_as_document(dockable);
        }
    }
    fn close_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        if let Some(dockable) = dockable.clone() {
            self.record_operation("close_dockable", dockable);
        }
        if self.execute_base.get() && !self.simulate_close.get() {
            self.base.get().unwrap().close_dockable(dockable);
        }
    }
    fn close_other_dockables(&self, dockable: Rc<dyn IDockable>) {
        self.calls
            .borrow_mut()
            .push(("close_other_dockables", dockable.clone()));
        if self.execute_base.get() {
            self.base.get().unwrap().close_other_dockables(dockable);
        }
    }
    fn close_all_dockables(&self, dockable: Rc<dyn IDockable>) {
        self.calls
            .borrow_mut()
            .push(("close_all_dockables", dockable.clone()));
        if self.execute_base.get() {
            self.base.get().unwrap().close_all_dockables(dockable);
        }
    }
    fn close_left_dockables(&self, dockable: Rc<dyn IDockable>) {
        self.calls
            .borrow_mut()
            .push(("close_left_dockables", dockable.clone()));
        if self.execute_base.get() {
            self.base.get().unwrap().close_left_dockables(dockable);
        }
    }
    fn close_right_dockables(&self, dockable: Rc<dyn IDockable>) {
        self.calls
            .borrow_mut()
            .push(("close_right_dockables", dockable.clone()));
        if self.execute_base.get() {
            self.base.get().unwrap().close_right_dockables(dockable);
        }
    }
    fn set_document_dock_tabs_layout(
        &self,
        dockable: Rc<dyn IDockable>,
        layout: DocumentTabLayout,
    ) {
        self.tabs_layout_requests
            .borrow_mut()
            .push((dockable.clone(), layout));
        self.base
            .get()
            .unwrap()
            .set_document_dock_tabs_layout(dockable, layout);
    }
    fn set_document_dock_tabs_layout_left(&self, dockable: Rc<dyn IDockable>) {
        self.calls
            .borrow_mut()
            .push(("set_document_dock_tabs_layout_left", dockable.clone()));
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .set_document_dock_tabs_layout_left(dockable);
        }
    }
    fn set_document_dock_tabs_layout_top(&self, dockable: Rc<dyn IDockable>) {
        self.calls
            .borrow_mut()
            .push(("set_document_dock_tabs_layout_top", dockable.clone()));
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .set_document_dock_tabs_layout_top(dockable);
        }
    }
    fn set_document_dock_tabs_layout_right(&self, dockable: Rc<dyn IDockable>) {
        self.calls
            .borrow_mut()
            .push(("set_document_dock_tabs_layout_right", dockable.clone()));
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .set_document_dock_tabs_layout_right(dockable);
        }
    }
    fn set_document_dock_layout_mode(
        &self,
        dockable: Rc<dyn IDockable>,
        layout_mode: DocumentLayoutMode,
    ) {
        self.layout_mode_requests
            .borrow_mut()
            .push((dockable.clone(), layout_mode));
        self.base
            .get()
            .unwrap()
            .set_document_dock_layout_mode(dockable, layout_mode);
    }
    fn set_document_dock_layout_mode_tabbed(&self, dockable: Rc<dyn IDockable>) {
        self.calls
            .borrow_mut()
            .push(("set_document_dock_layout_mode_tabbed", dockable.clone()));
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .set_document_dock_layout_mode_tabbed(dockable);
        }
    }
    fn set_document_dock_layout_mode_mdi(&self, dockable: Rc<dyn IDockable>) {
        self.calls
            .borrow_mut()
            .push(("set_document_dock_layout_mode_mdi", dockable.clone()));
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .set_document_dock_layout_mode_mdi(dockable);
        }
    }
    fn hide_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.record_operation(
            "hide_dockable",
            dockable.clone().expect("nonnull hidden fixture"),
        );
        if self.execute_base.get() {
            self.base.get().unwrap().hide_dockable(dockable);
        }
    }
    fn hide_dockable_by_id(&self, id: String) {
        self.base.get().unwrap().hide_dockable_by_id(id);
    }
    fn restore_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.record_operation(
            "restore_dockable",
            dockable.clone().expect("nonnull hidden fixture"),
        );
        if self.execute_base.get() {
            self.base.get().unwrap().restore_dockable(dockable);
        }
    }
    fn restore_dockable_by_id(&self, id: String) -> Option<Rc<dyn IDockable>> {
        self.base.get().unwrap().restore_dockable_by_id(id)
    }
    fn add_window(
        &self,
        root_dock: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
    ) {
        self.base.get().unwrap().add_window(root_dock, window);
    }
    fn insert_window(
        &self,
        root_dock: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
        index: i32,
    ) {
        self.base
            .get()
            .unwrap()
            .insert_window(root_dock, window, index);
    }
    fn remove_window(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.base.get().unwrap().remove_window(window);
    }
    fn close_window(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.base.get().unwrap().close_window(window);
    }
    fn collapse_dock(&self, dock: Rc<dyn IDock>) {
        let callback = self.on_collapse.borrow().clone();
        if let Some(callback) = callback {
            callback(dock);
        } else if self.execute_base.get() {
            self.base.get().unwrap().collapse_dock(dock);
        } else {
            panic!("unused recording-factory collapse");
        }
    }
    fn create_split_layout(
        &self,
        dock: Rc<dyn IDock>,
        dockable: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> Rc<dyn IDock> {
        self.base
            .get()
            .unwrap()
            .create_split_layout(dock, dockable, operation)
    }
    fn split_to_dock(
        &self,
        dock: Rc<dyn IDock>,
        dockable: Rc<dyn IDockable>,
        operation: DockOperation,
    ) {
        self.base
            .get()
            .unwrap()
            .split_to_dock(dock, dockable, operation);
    }
    fn create_window_from(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        self.base.get().unwrap().create_window_from(dockable)
    }
    fn create_window_from_with_options(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        self.base
            .get()
            .unwrap()
            .create_window_from_with_options(dockable, options)
    }
    fn split_to_window(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        self.base
            .get()
            .unwrap()
            .split_to_window(dock, dockable, x, y, width, height);
    }
    fn split_to_window_with_options(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        self.base
            .get()
            .unwrap()
            .split_to_window_with_options(dock, dockable, x, y, width, height, options);
    }
    fn new_horizontal_document_dock(&self, dockable: Rc<dyn IDockable>) {
        self.record_operation("new_horizontal_document_dock", dockable.clone());
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .new_horizontal_document_dock(dockable);
        }
    }
    fn new_vertical_document_dock(&self, dockable: Rc<dyn IDockable>) {
        self.record_operation("new_vertical_document_dock", dockable.clone());
        if self.execute_base.get() {
            self.base
                .get()
                .unwrap()
                .new_vertical_document_dock(dockable);
        }
    }
}

#[test]
fn each_default_parameter_adapter_ignores_non_dockables_and_routes_the_original_handle() {
    type Adapter = fn(&dyn IFactory, Option<BoxedValue>);
    let adapters: &[(&str, Adapter)] = &[
        ("pin_dockable", |factory, parameter| {
            factory.pin_dockable_parameter(parameter)
        }),
        ("preview_pinned_dockable", |factory, parameter| {
            factory.preview_pinned_dockable_parameter(parameter)
        }),
        ("toggle_preview_pinned_dockable", |factory, parameter| {
            factory.toggle_preview_pinned_dockable_parameter(parameter)
        }),
        ("float_dockable", |factory, parameter| {
            factory.float_dockable_parameter(parameter)
        }),
        ("float_all_dockables", |factory, parameter| {
            factory.float_all_dockables_parameter(parameter)
        }),
        ("dock_as_document", |factory, parameter| {
            factory.dock_as_document_parameter(parameter)
        }),
        ("close_dockable", |factory, parameter| {
            factory.close_dockable_parameter(parameter)
        }),
        ("close_other_dockables", |factory, parameter| {
            factory.close_other_dockables_parameter(parameter)
        }),
        ("close_all_dockables", |factory, parameter| {
            factory.close_all_dockables_parameter(parameter)
        }),
        ("close_left_dockables", |factory, parameter| {
            factory.close_left_dockables_parameter(parameter)
        }),
        ("close_right_dockables", |factory, parameter| {
            factory.close_right_dockables_parameter(parameter)
        }),
        (
            "set_document_dock_tabs_layout_left",
            |factory, parameter| factory.set_document_dock_tabs_layout_left_parameter(parameter),
        ),
        ("set_document_dock_tabs_layout_top", |factory, parameter| {
            factory.set_document_dock_tabs_layout_top_parameter(parameter)
        }),
        (
            "set_document_dock_tabs_layout_right",
            |factory, parameter| factory.set_document_dock_tabs_layout_right_parameter(parameter),
        ),
        (
            "set_document_dock_layout_mode_tabbed",
            |factory, parameter| factory.set_document_dock_layout_mode_tabbed_parameter(parameter),
        ),
        ("set_document_dock_layout_mode_mdi", |factory, parameter| {
            factory.set_document_dock_layout_mode_mdi_parameter(parameter)
        }),
        ("new_horizontal_document_dock", |factory, parameter| {
            factory.new_horizontal_document_dock_parameter(parameter)
        }),
        ("new_vertical_document_dock", |factory, parameter| {
            factory.new_vertical_document_dock_parameter(parameter)
        }),
    ];
    let factory = RecordingFactory::default();
    let dockable: Rc<dyn IDockable> = TestNode::new(None);
    let parameter: BoxedValue = Rc::new(dockable.clone());
    for (name, call) in adapters {
        factory.calls.borrow_mut().clear();
        call(&factory, None);
        call(&factory, Some(Rc::new("not a dockable".to_owned())));
        assert!(factory.calls.borrow().is_empty());
        call(&factory, Some(parameter.clone()));
        let calls = factory.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, *name);
        assert!(Rc::ptr_eq(&calls[0].1, &dockable));
    }
}

#[test]
fn concrete_factory_generic_lists_preserve_the_original_element_type_and_shared_handle() {
    let factory = RecordingFactory::default();
    let values = factory.create_list(vec![1_i32, 2, 3]);
    let alias = values.clone();
    alias.add(4);
    assert_eq!(values.to_vec(), [1, 2, 3, 4]);
    let node: Rc<dyn IDockable> = TestNode::new(None);
    let dockables = factory.create_list(vec![node.clone()]);
    assert!(Rc::ptr_eq(&dockables.get(0), &node));
}
