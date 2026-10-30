use super::*;
use crate::overlays::services::*;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::services::*;
use ferrodock_model::{BoxedValue, DockManager, DockService};
use ferrodock_model_reactiveui::controls::RootDock;
use ferrodock_model_reactiveui::core::DockableBaseImpl;
use ferrodock_model_reactiveui::{impl_dock, Factory};
use ferroui_reactiveui::{reactive_object, ICommand, IRoutableViewModel, IScreen, RoutingState};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::threading::tasks::Task;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(crate) trait Marker {
    fn label(&self) -> &str;
}
pub(crate) struct Screen {
    pub router: Rc<RoutingState>,
    pub name: String,
}
impl Screen {
    pub fn new(name: &str) -> Rc<Self> {
        Rc::new(Self {
            router: RoutingState::new(),
            name: name.into(),
        })
    }
}
impl IScreen for Screen {
    fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }
}

pub(crate) struct ScreenRoot {
    pub base: RootDock,
    router: Rc<RoutingState>,
    pub host: RefCell<Option<Weak<dyn IScreen>>>,
    pub disposed: Cell<usize>,
    pub log: Rc<RefCell<Vec<&'static str>>>,
    name: String,
}
reactive_object!(ScreenRoot, base);
impl DockableBaseImpl for ScreenRoot {}
impl_dock!(@base ScreenRoot,RootDock,fn as_root_dock(self:Rc<Self>)->Option<Rc<dyn IRootDock>> { Some(self) });
macro_rules! root_contract {
    ($type:ty) => {
        impl IRootDock for $type {
            fn is_focusable_root(&self) -> bool {
                self.base.is_focusable_root()
            }
            fn set_is_focusable_root(&self, v: bool) {
                self.base.set_is_focusable_root(v);
            }
            fn hidden_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.hidden_dockables()
            }
            fn set_hidden_dockables(&self, v: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
                self.base.set_hidden_dockables(v);
            }
            fn left_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.left_pinned_dockables()
            }
            fn set_left_pinned_dockables(&self, v: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
                self.base.set_left_pinned_dockables(v);
            }
            fn right_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.right_pinned_dockables()
            }
            fn set_right_pinned_dockables(&self, v: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
                self.base.set_right_pinned_dockables(v);
            }
            fn top_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.top_pinned_dockables()
            }
            fn set_top_pinned_dockables(&self, v: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
                self.base.set_top_pinned_dockables(v);
            }
            fn bottom_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.bottom_pinned_dockables()
            }
            fn set_bottom_pinned_dockables(
                &self,
                v: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
            ) {
                self.base.set_bottom_pinned_dockables(v);
            }
            fn pinned_dock(&self) -> Option<Rc<dyn IToolDock>> {
                self.base.pinned_dock()
            }
            fn set_pinned_dock(&self, v: Option<Rc<dyn IToolDock>>) {
                self.base.set_pinned_dock(v);
            }
            fn pinned_dock_display_mode(&self) -> PinnedDockDisplayMode {
                self.base.pinned_dock_display_mode()
            }
            fn set_pinned_dock_display_mode(&self, v: PinnedDockDisplayMode) {
                self.base.set_pinned_dock_display_mode(v);
            }
            fn window(&self) -> Option<Rc<dyn IDockWindow>> {
                self.base.window()
            }
            fn set_window(&self, v: Option<Rc<dyn IDockWindow>>) {
                self.base.set_window(v);
            }
            fn windows(&self) -> Option<ListHandle<Option<Rc<dyn IDockWindow>>>> {
                self.base.windows()
            }
            fn set_windows(&self, v: Option<ListHandle<Option<Rc<dyn IDockWindow>>>>) {
                self.base.set_windows(v);
            }
            fn floating_window_host_mode(&self) -> DockFloatingWindowHostMode {
                self.base.floating_window_host_mode()
            }
            fn set_floating_window_host_mode(&self, v: DockFloatingWindowHostMode) {
                self.base.set_floating_window_host_mode(v);
            }
            fn root_dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>> {
                self.base.root_dock_capability_policy()
            }
            fn set_root_dock_capability_policy(&self, v: Option<Rc<dyn DockCapabilityPolicyImpl>>) {
                self.base.set_root_dock_capability_policy(v);
            }
            fn show_windows(&self) -> Rc<dyn ICommand> {
                self.base.show_windows()
            }
            fn exit_windows(&self) -> Rc<dyn ICommand> {
                self.base.exit_windows()
            }
            fn enable_adaptive_global_dock_targets(&self) -> bool {
                self.base.enable_adaptive_global_dock_targets()
            }
            fn set_enable_adaptive_global_dock_targets(&self, v: bool) {
                self.base.set_enable_adaptive_global_dock_targets(v);
            }
        }
    };
}
root_contract!(ScreenRoot);
impl ScreenRoot {
    pub fn construct<T: DockableBaseImpl + IRootDock + 'static>(
        runtime: &Weak<T>,
        name: &str,
    ) -> Self {
        Self {
            base: RootDock::construct(runtime),
            router: RoutingState::new(),
            host: RefCell::new(None),
            disposed: Cell::new(0),
            log: Rc::new(RefCell::new(Vec::new())),
            name: name.into(),
        }
    }
    pub fn new(name: &str) -> Rc<Self> {
        let model = Rc::new_cyclic(|weak| Self::construct(weak, name));
        model.base.initialize(&model);
        model
    }
    pub fn register(this: &Rc<Self>, types: &HostTypeRegistry) {
        types.register_dockable_screen(this);
        types.register_routable(this);
        types.register_interface(this, this.clone() as Rc<dyn Marker>);
        types.register_interface(this, this.clone() as Rc<dyn IDisposable>);
    }
}
impl IScreen for ScreenRoot {
    fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }
}
impl IRoutableViewModel for ScreenRoot {
    fn url_path_segment(&self) -> Option<String> {
        Some(self.name.clone())
    }
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host.borrow().as_ref().and_then(Weak::upgrade)
    }
}
impl IDisposable for ScreenRoot {
    fn dispose(&self) {
        self.disposed.set(self.disposed.get() + 1);
        self.log.borrow_mut().push("dispose");
    }
}
impl Marker for ScreenRoot {
    fn label(&self) -> &str {
        &self.name
    }
}

pub(crate) struct OverlayRoot {
    pub base: ScreenRoot,
    pub services: Rc<dyn IHostOverlayServices>,
}
reactive_object!(OverlayRoot, base);
impl DockableBaseImpl for OverlayRoot {}
impl_dock!(@base OverlayRoot,ScreenRoot,fn as_root_dock(self:Rc<Self>)->Option<Rc<dyn IRootDock>> { Some(self) });
root_contract!(OverlayRoot);
impl OverlayRoot {
    pub fn new(name: &str, services: Rc<dyn IHostOverlayServices>) -> Rc<Self> {
        let model = Rc::new_cyclic(|weak| Self {
            base: ScreenRoot::construct(weak, name),
            services,
        });
        model.base.base.initialize(&model);
        model
    }
    pub fn register(this: &Rc<Self>, types: &HostTypeRegistry) {
        types.register_dockable_screen(this);
        types.register_routable(this);
        types.register_interface(this, this.clone() as Rc<dyn Marker>);
        types.register_interface(this, this.clone() as Rc<dyn IDisposable>);
        types.register_interface(this, this.clone() as Rc<dyn IHostOverlayServices>);
    }
}
impl IScreen for OverlayRoot {
    fn router(&self) -> Rc<RoutingState> {
        self.base.router()
    }
}
impl IRoutableViewModel for OverlayRoot {
    fn url_path_segment(&self) -> Option<String> {
        self.base.url_path_segment()
    }
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.base.host_screen()
    }
}
impl IDisposable for OverlayRoot {
    fn dispose(&self) {
        self.base.dispose();
    }
}
impl Marker for OverlayRoot {
    fn label(&self) -> &str {
        self.base.label()
    }
}
impl IHostOverlayServices for OverlayRoot {
    fn busy(&self) -> Rc<dyn IDockBusyService> {
        self.services.busy()
    }
    fn dialogs(&self) -> Rc<dyn IDockDialogService> {
        self.services.dialogs()
    }
    fn confirmations(&self) -> Rc<dyn IDockConfirmationService> {
        self.services.confirmations()
    }
    fn global_busy_service(&self) -> Rc<dyn IDockGlobalBusyService> {
        self.services.global_busy_service()
    }
    fn global_dialog_service(&self) -> Rc<dyn IDockGlobalDialogService> {
        self.services.global_dialog_service()
    }
    fn global_confirmation_service(&self) -> Rc<dyn IDockGlobalConfirmationService> {
        self.services.global_confirmation_service()
    }
}

pub(crate) struct CountingDialogs {
    pub inner: Rc<DockDialogService>,
    pub calls: Cell<usize>,
    pub log: Rc<RefCell<Vec<&'static str>>>,
    pub fail: Cell<bool>,
}
impl INotifyPropertyChanged for CountingDialogs {
    fn property_changed(&self) -> &Event<str> {
        self.inner.property_changed()
    }
}
impl IDockDialogService for CountingDialogs {
    fn dialogs(
        &self,
    ) -> ferrodock_model::collections::ReadOnlyObservableCollection<Rc<DialogRequest>> {
        self.inner.dialogs()
    }
    fn active_dialog(&self) -> Option<Rc<DialogRequest>> {
        self.inner.active_dialog()
    }
    fn has_dialogs(&self) -> bool {
        self.inner.has_dialogs()
    }
    fn show_async_boxed(
        &self,
        content: DialogContent,
        title: Option<String>,
    ) -> Task<Option<BoxedValue>> {
        self.inner.show_async_boxed(content, title)
    }
    fn close(&self, request: Rc<DialogRequest>, result: Option<BoxedValue>) {
        self.inner.close(request, result);
    }
    fn cancel_all(&self) {
        self.calls.set(self.calls.get() + 1);
        self.log.borrow_mut().push("dialogs");
        assert!(!self.fail.get(), "dialog cancel failed");
        self.inner.cancel_all();
    }
}
pub(crate) struct CountingConfirmations {
    pub inner: Rc<DockConfirmationService>,
    pub calls: Cell<usize>,
    pub log: Rc<RefCell<Vec<&'static str>>>,
}
impl INotifyPropertyChanged for CountingConfirmations {
    fn property_changed(&self) -> &Event<str> {
        self.inner.property_changed()
    }
}
impl IDockConfirmationService for CountingConfirmations {
    fn confirmations(
        &self,
    ) -> ferrodock_model::collections::ReadOnlyObservableCollection<Rc<ConfirmationRequest>> {
        self.inner.confirmations()
    }
    fn active_confirmation(&self) -> Option<Rc<ConfirmationRequest>> {
        self.inner.active_confirmation()
    }
    fn has_confirmations(&self) -> bool {
        self.inner.has_confirmations()
    }
    fn confirm_async(&self, t: String, m: String, c: String, x: String) -> Task<bool> {
        self.inner.confirm_async(t, m, c, x)
    }
    fn close(&self, r: Rc<ConfirmationRequest>, v: bool) {
        self.inner.close(r, v);
    }
    fn cancel_all(&self) {
        self.calls.set(self.calls.get() + 1);
        self.log.borrow_mut().push("confirmations");
        self.inner.cancel_all();
    }
}
pub(crate) struct Overlays {
    pub host: Rc<dyn IHostOverlayServices>,
    pub dialogs: Rc<CountingDialogs>,
    pub confirmations: Rc<CountingConfirmations>,
    pub log: Rc<RefCell<Vec<&'static str>>>,
}
impl Overlays {
    pub fn new() -> Self {
        let log = Rc::new(RefCell::new(Vec::new()));
        let gb = DockGlobalBusyService::new(None, None);
        let gd = DockGlobalDialogService::new(None, None);
        let gc = DockGlobalConfirmationService::new(None, None);
        let dialogs = Rc::new(CountingDialogs {
            inner: DockDialogService::new(Some(gd.clone()), None),
            calls: Cell::new(0),
            log: log.clone(),
            fail: Cell::new(false),
        });
        let confirmations = Rc::new(CountingConfirmations {
            inner: DockConfirmationService::new(Some(gc.clone()), None),
            calls: Cell::new(0),
            log: log.clone(),
        });
        let host = HostOverlayServicesAdapter::new(
            DockBusyService::new(Some(gb.clone()), None),
            dialogs.clone(),
            confirmations.clone(),
            gb,
            gd,
            gc,
        );
        Self {
            host,
            dialogs,
            confirmations,
            log,
        }
    }
}

pub(crate) struct Resolver {
    pub types: Rc<HostTypeRegistry>,
    pub value: RefCell<Option<Rc<dyn IHostOverlayServices>>>,
    pub calls: Cell<usize>,
}
impl Resolver {
    pub fn new(types: Rc<HostTypeRegistry>) -> Rc<Self> {
        Rc::new(Self {
            types,
            value: RefCell::new(None),
            calls: Cell::new(0),
        })
    }
}
impl IHostServiceResolver for Resolver {
    fn resolve_erased(
        &self,
        _screen: Rc<dyn IScreen>,
        target: std::any::TypeId,
    ) -> Option<HostServiceValue> {
        self.calls.set(self.calls.get() + 1);
        if target != std::any::TypeId::of::<dyn IHostOverlayServices>() {
            return None;
        }
        self.value.borrow().clone().map(HostServiceValue::new)
    }
    fn native_types(&self) -> Option<Rc<HostTypeRegistry>> {
        Some(self.types.clone())
    }
}
pub(crate) struct Provider {
    pub host: Rc<dyn IHostOverlayServices>,
    pub types: Rc<HostTypeRegistry>,
    pub calls: Cell<usize>,
}
impl IHostOverlayServicesProvider for Provider {
    fn get_services(&self, _screen: Rc<dyn IScreen>) -> Rc<dyn IHostOverlayServices> {
        self.calls.set(self.calls.get() + 1);
        self.host.clone()
    }
    fn get_services_for_dockable(
        &self,
        screen: Rc<dyn IScreen>,
        _dockable: Rc<dyn IDockable>,
    ) -> Rc<dyn IHostOverlayServices> {
        self.get_services(screen)
    }
    fn native_types(&self) -> Option<Rc<HostTypeRegistry>> {
        Some(self.types.clone())
    }
}

struct State;
impl IDockManagerState for State {}
impl IDockControlState for State {}
pub(crate) struct Control {
    layout: RefCell<Option<Rc<dyn IDock>>>,
    context: RefCell<Option<BoxedValue>>,
    factory: RefCell<Option<Weak<dyn IFactory>>>,
    init_layout: Cell<bool>,
    init_factory: Cell<bool>,
    manager: Rc<dyn IDockManager>,
    state: Rc<dyn IDockControlState>,
}
impl Control {
    pub fn new(layout: Rc<dyn IDock>) -> Rc<Self> {
        Rc::new(Self {
            layout: RefCell::new(Some(layout)),
            context: RefCell::new(None),
            factory: RefCell::new(None),
            init_layout: Cell::new(true),
            init_factory: Cell::new(true),
            manager: DockManager::new(DockService::new()),
            state: Rc::new(State),
        })
    }
}
impl IDockControl for Control {
    fn dock_manager(&self) -> Rc<dyn IDockManager> {
        self.manager.clone()
    }
    fn dock_control_state(&self) -> Rc<dyn IDockControlState> {
        self.state.clone()
    }
    fn layout(&self) -> Option<Rc<dyn IDock>> {
        self.layout.borrow().clone()
    }
    fn set_layout(&self, v: Option<Rc<dyn IDock>>) {
        self.layout.replace(v);
    }
    fn default_context(&self) -> Option<BoxedValue> {
        self.context.borrow().clone()
    }
    fn set_default_context(&self, v: Option<BoxedValue>) {
        self.context.replace(v);
    }
    fn initialize_layout(&self) -> bool {
        self.init_layout.get()
    }
    fn set_initialize_layout(&self, v: bool) {
        self.init_layout.set(v);
    }
    fn initialize_factory(&self) -> bool {
        self.init_factory.get()
    }
    fn set_initialize_factory(&self, v: bool) {
        self.init_factory.set(v);
    }
    fn factory(&self) -> Option<Rc<dyn IFactory>> {
        self.factory.borrow().as_ref().and_then(Weak::upgrade)
    }
    fn set_factory(&self, v: Option<Rc<dyn IFactory>>) {
        self.factory.replace(v.as_ref().map(Rc::downgrade));
    }
}
pub(crate) struct Host {
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
    tracked: Cell<bool>,
    position: Cell<(f64, f64)>,
    size: Cell<(f64, f64)>,
    state: Cell<DockWindowState>,
    title: RefCell<Option<String>>,
    layout: RefCell<Option<Rc<dyn IDock>>>,
    pub calls: Cell<usize>,
}
impl Host {
    pub fn new(window: Rc<dyn IDockWindow>) -> Rc<Self> {
        Rc::new(Self {
            window: RefCell::new(Some(window)),
            tracked: Cell::new(false),
            position: Cell::new((0.0, 0.0)),
            size: Cell::new((0.0, 0.0)),
            state: Cell::new(DockWindowState::Normal),
            title: RefCell::new(None),
            layout: RefCell::new(None),
            calls: Cell::new(0),
        })
    }
}
impl IHostWindow for Host {
    fn host_window_state(&self) -> Option<Rc<dyn IHostWindowState>> {
        None
    }
    fn is_tracked(&self) -> bool {
        self.tracked.get()
    }
    fn set_is_tracked(&self, v: bool) {
        self.tracked.set(v);
    }
    fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.calls.set(self.calls.get() + 1);
        self.window.borrow().clone()
    }
    fn set_window(&self, v: Option<Rc<dyn IDockWindow>>) {
        self.window.replace(v);
    }
    fn present(&self, _dialog: bool) {
        self.calls.set(self.calls.get() + 1);
    }
    fn exit(&self) {
        self.calls.set(self.calls.get() + 1);
    }
    fn set_position(&self, x: f64, y: f64) {
        self.position.set((x, y));
    }
    fn get_position(&self) -> (f64, f64) {
        self.position.get()
    }
    fn set_size(&self, w: f64, h: f64) {
        self.size.set((w, h));
    }
    fn get_size(&self) -> (f64, f64) {
        self.size.get()
    }
    fn set_window_state(&self, v: DockWindowState) {
        self.state.set(v);
    }
    fn get_window_state(&self) -> DockWindowState {
        self.state.get()
    }
    fn set_title(&self, v: Option<String>) {
        self.title.replace(v);
    }
    fn set_layout(&self, v: Rc<dyn IDock>) {
        self.layout.replace(Some(v));
    }
    fn set_active(&self) {
        self.calls.set(self.calls.get() + 1);
    }
}

pub(crate) fn factory() -> Rc<Factory> {
    Factory::new()
}
