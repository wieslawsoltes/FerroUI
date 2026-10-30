// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{
    DockCommandBarHost, DockSelectorOverlay, DocumentTabStrip, DocumentTabStripItem, HostWindow,
    ManagedHostWindow, ManagedWindowLayer, ToolChromeControl, ToolPinItemControl, ToolTabStripItem,
};
use crate::{
    automation::peers::DockControlAutomationPeer,
    command_bars::DockCommandBarManager,
    contract::{IDragOffsetCalculator, IExternalDockSurface},
    diagnostics::DockDiagnosticEvents,
    internal::{
        default_drag_offset_calculator::DefaultDragOffsetCalculator,
        dock_control_state::DockControlState, dock_data_template_helper::DockDataTemplateHelper,
        dock_helpers::DockHelpers, event_type::EventType,
        managed_window_registry::ManagedWindowRegistry,
    },
    model_projection,
    selectors::{DockSelectorItem, DockSelectorMode, IDockSelectorService},
    services::{DockControlFactoryService, IDockControlFactoryService},
};
use ferrodock_controls_recycling::{from_recycling_value, to_recycling_value};
use ferrodock_model::core::DockFloatingWindowHostMode;
use ferrodock_model::{
    collections::{ListHandle, ReadOnlyListHandle},
    controls::IRootDock,
    core::{
        events::ActiveDockableChangedEventArgs, ContextFactory, DockManagerOptions, DragAction,
        HostWindowFactory, IDock, IDockControl, IDockControlState, IDockManager, IDockable,
        IFactory, IHostWindow,
    },
    BoxedValue as ModelValue, DockManager, DockService,
};
use ferrodock_settings::DockSettings;
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        IPointer, InputElement, InputElementImpl, Key, KeyEventArgs, KeyGesture, KeyModifiers,
        MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
        PointerReleasedEventArgs, PointerWheelEventArgs,
    },
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventArgs, RoutingStrategies},
    layout::LayoutableImpl,
    threading::{Dispatcher, DispatcherPriority},
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{
        ScrollBar, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
        TemplatedControlImplExt,
    },
    Control, ControlImpl, ControlImplExt, TopLevel, Window,
};
use ferroui_system::{collections::generic::Dictionary, IDisposable};
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

/// Property-system value for a source host-window callback. Public accessors retain the callback API.
#[derive(Clone)]
pub struct HostWindowFactoryValue(pub HostWindowFactory);
impl PartialEq for HostWindowFactoryValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
type WeakContextCallback = Weak<dyn Fn() -> Option<ModelValue>>;
type WeakHostCallback = Weak<dyn Fn() -> Option<Rc<dyn IHostWindow>>>;
type ActivationEntry = (Weak<dyn IDockable>, i64);
/// The original dock host, engine, selector and factory lifecycle controller.
#[repr(C)]
pub struct DockControl {
    base: TemplatedControl,
    dock_manager_options: Rc<DockManagerOptions>,
    dock_manager: Rc<DockManager>,
    dock_control_state: Rc<DockControlState>,
    factory_service: Rc<dyn IDockControlFactoryService>,
    is_initialized: Cell<bool>,
    content_control: RefCell<Option<Ref<Control>>>,
    managed_window_layer: RefCell<Option<Ref<ManagedWindowLayer>>>,
    command_bar_host: RefCell<Option<Ref<DockCommandBarHost>>>,
    command_bar_manager: RefCell<Option<Rc<DockCommandBarManager>>>,
    selector_overlay: RefCell<Option<Ref<DockSelectorOverlay>>>,
    selector_mode: Cell<DockSelectorMode>,
    selector_gesture: RefCell<Option<KeyGesture>>,
    activation_order: RefCell<HashMap<usize, ActivationEntry>>,
    activation_counter: Cell<i64>,
    subscribed_factory: RefCell<Option<Rc<dyn IFactory>>>,
    factory_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    managed_layer_factory: RefCell<Option<Rc<dyn IFactory>>>,
    attached_window: RefCell<Option<Ref<Window>>>,
    activated_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    closing_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    external_dock_surfaces: RefCell<Vec<ExternalSurfaceReference>>,
    external_surfaces_version: Cell<u32>,
    drag_offset_calculator: RefCell<Rc<dyn IDragOffsetCalculator>>,
    interface_handle: RefCell<Weak<DockControlHandle>>,
    // Metadata is weak. The callbacks themselves retain their original receiver strongly.
    context_callbacks: RefCell<Vec<WeakContextCallback>>,
    host_callbacks: RefCell<Vec<WeakHostCallback>>,
}
ferro_class!(DockControl:TemplatedControl);
ferro_class_info!(DockControl {
    new:DockControl::new,
    interfaces:[Rc<dyn IDockControl> => DockControl::as_dock_control,
                Rc<dyn IDockSelectorService> => DockControl::as_selector_service],
    markup:{content:Layout,
        properties:[
            DockManager:Rc<dyn IDockManager>{get:|this:&Ref<DockControl>|this.dock_manager()},
            DockManagerOptions:Rc<DockManagerOptions>{get:|this:&Ref<DockControl>|this.dock_manager_options()},
            DockControlState:Rc<dyn IDockControlState>{get:|this:&Ref<DockControl>|this.dock_control_state()},
            IsOpen:bool{get:|this:&Ref<DockControl>|this.is_open()},
            DragOffsetCalculator:Rc<dyn IDragOffsetCalculator>{get:|this:&Ref<DockControl>|this.drag_offset_calculator(),set:|this:&Ref<DockControl>,v|this.set_drag_offset_calculator(v)}
        ],
        methods:[
            fn RegisterExternalDockSurface(Rc<dyn IExternalDockSurface>) => (|this:&Ref<DockControl>,v|this.register_external_dock_surface(v)),
            fn UnregisterExternalDockSurface(Rc<dyn IExternalDockSurface>)->bool => (|this:&Ref<DockControl>,v|this.unregister_external_dock_surface(v)),
            fn ShowSelector(DockSelectorMode) => (|this:&Ref<DockControl>,v|this.show_selector(v)),
            fn HideSelector() => (|this:&Ref<DockControl>|this.hide_selector())
        ],
        attributes:[TemplatePart("PART_ContentControl",type(Ref<Control>)),
            TemplatePart("PART_CommandBarHost",type(Ref<DockCommandBarHost>)),
            TemplatePart("PART_SelectorOverlay",type(Ref<DockSelectorOverlay>)),
            TemplatePart("PART_ManagedWindowLayer",type(Ref<ManagedWindowLayer>))]
    }
});
ferro_impl_classes!(DockControl:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
impl FerroObjectImpl for DockControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let routes =
            RoutingStrategies::DIRECT | RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE;
        let control = this.to_ref();
        this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| control.pressed_handler(e),
            routes,
            false,
        );
        let control = this.to_ref();
        this.add_handler_with(
            InputElement::pointer_released_event(),
            move |_, e| control.released_handler(e),
            routes,
            false,
        );
        let control = this.to_ref();
        this.add_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e| control.moved_handler(e),
            routes,
            false,
        );
        let control = this.to_ref();
        this.add_handler_with(
            InputElement::pointer_entered_event(),
            move |_, e| control.entered_handler(e),
            routes,
            false,
        );
        let control = this.to_ref();
        this.add_handler_with(
            InputElement::pointer_exited_event(),
            move |_, e| control.exited_handler(e),
            routes,
            false,
        );
        let control = this.to_ref();
        this.add_handler_with(
            InputElement::pointer_capture_lost_event(),
            move |_, e| control.capture_lost_handler(e),
            routes,
            false,
        );
        let control = this.to_ref();
        this.add_handler_with(
            InputElement::pointer_wheel_changed_event(),
            move |_, e| control.wheel_changed_handler(e),
            routes,
            false,
        );
        let control = this.to_ref();
        this.add_handler_with(
            InputElement::key_down_event(),
            move |_, e| control.key_down_handler(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        let control = this.to_ref();
        this.add_handler_with(
            InputElement::key_up_event(),
            move |_, e| control.key_up_handler(e),
            RoutingStrategies::TUNNEL,
            false,
        );
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::layout_property().as_property() {
            if this.is_initialized.get() {
                this.deinitialize(change.get_old_value::<Option<Rc<dyn IDock>>>().flatten());
            }
            this.initialize(change.get_new_value::<Option<Rc<dyn IDock>>>());
        } else if change.property() == Self::enable_managed_window_layer_property().as_property() {
            this.update_managed_window_layer(this.layout());
        } else if change.property() == Self::is_docking_enabled_property().as_property() {
            this.dock_manager_options
                .set_is_docking_enabled(change.get_new_value::<bool>());
        }
    }
}
impl ControlImpl for DockControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DockControlAutomationPeer::new(this).upcast()
    }
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.attach_main_window_handlers(
            TopLevel::get_top_level(Some(this)).and_then(|t| t.cast::<Window>()),
        );
    }
}
impl TemplatedControlImpl for DockControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.content_control
            .replace(e.name_scope().find_as::<Control>("PART_ContentControl"));
        this.command_bar_host.replace(
            e.name_scope()
                .find_as::<DockCommandBarHost>("PART_CommandBarHost"),
        );
        this.selector_overlay.replace(
            e.name_scope()
                .find_as::<DockSelectorOverlay>("PART_SelectorOverlay"),
        );
        this.managed_window_layer.replace(
            e.name_scope()
                .find_as::<ManagedWindowLayer>("PART_ManagedWindowLayer"),
        );
        this.factory_service.initialize_control_recycling(this);
        if this.content_control().is_some() {
            this.initialize_default_data_templates();
        }
        this.update_managed_window_layer(this.layout());
        this.initialize_command_bars();
    }
}
impl VisualImpl for DockControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        if this.is_initialized.get() {
            this.deinitialize(this.layout());
        }
        this.initialize(this.layout());
        let root = this
            .layout()
            .and_then(|l| l.factory().and_then(|f| f.find_root(Some(l), None)));
        if let Some(root) = root.filter(|r| r.window().is_none()) {
            let window_model = this
                .layout()
                .and_then(|l| l.factory())
                .and_then(|f| f.create_window_from(Some(root.clone())));
            if let Some(window_model) = window_model {
                window_model.set_layout(Some(root.clone()));
                if let Some(window) =
                    TopLevel::get_top_level(Some(this)).and_then(|t| native_host(&t))
                {
                    if let Some(factory) = root.factory() {
                        factory.init_dock_window_with_host(
                            Some(window_model.clone()),
                            Some(root.clone()),
                            Some(window),
                        );
                    }
                }
                root.set_window(Some(window_model));
            }
        }
        this.restore_external_dock_surface_owners();
        this.attach_main_window_handlers(
            TopLevel::get_top_level(Some(this)).and_then(|t| t.cast::<Window>()),
        );
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.detach_main_window_handlers();
        let layout = this.layout();
        if this.is_initialized.get() {
            this.deinitialize(layout.clone());
        }
        this.detach_external_dock_surface_owners();
        this.notify_root_window_closed(layout);
    }
}
ferro_properties! {impl DockControl {
    /// Identifies Layout.
    pub fn layout_property()->StyledProperty<Option<Rc<dyn IDock>>> {FerroProperty::register::<Self,Option<Rc<dyn IDock>>>("Layout",None)}
    /// Identifies DefaultContext.
    pub fn default_context_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,Option<BoxedValue>>("DefaultContext",None)}
    /// Identifies InitializeLayout.
    pub fn initialize_layout_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("InitializeLayout",false)}
    /// Identifies InitializeFactory.
    pub fn initialize_factory_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("InitializeFactory",false)}
    /// Identifies HostWindowFactory. Its value retains the original public callback.
    pub fn host_window_factory_property()->StyledProperty<Option<HostWindowFactoryValue>> {FerroProperty::register::<Self,Option<HostWindowFactoryValue>>("HostWindowFactory",None)}
    /// Identifies Factory.
    pub fn factory_property()->StyledProperty<Option<Rc<dyn IFactory>>> {FerroProperty::register::<Self,Option<Rc<dyn IFactory>>>("Factory",None)}
    /// Identifies IsDockingEnabled.
    pub fn is_docking_enabled_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsDockingEnabled",true)}
    /// Identifies IsDraggingDock.
    pub fn is_dragging_dock_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsDraggingDock",false)}
    /// Identifies EnableManagedWindowLayer.
    pub fn enable_managed_window_layer_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("EnableManagedWindowLayer",true)}
    /// Identifies AutoCreateDataTemplates.
    pub fn auto_create_data_templates_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("AutoCreateDataTemplates",true)}
}}
impl DockControl {
    fn static_constructor() {
        ValueTypes::register_interface_object::<Rc<dyn IDockControl>>(|h| {
            h.clone()
                .boxed_interface(TypeId::of::<Ref<Control>>())
                .and_then(|v| v.downcast_ref::<Ref<Control>>().cloned())
                .map(|c| c.upcast())
        });
        ValueTypes::register_interface_object::<Rc<dyn IDockSelectorService>>(|h| h.as_object());
    }
    /// Constructs the complete source engine and class state for native inheritance.
    pub fn construct() -> Self {
        let options = DockManagerOptions::new();
        let manager = DockManager::with_options(DockService::new(), options.clone());
        let calculator: Rc<dyn IDragOffsetCalculator> = Rc::new(DefaultDragOffsetCalculator);
        let state = DockControlState::new(manager.clone(), calculator.clone(), None);
        Self {
            base: TemplatedControl::construct(),
            dock_manager_options: options,
            dock_manager: manager,
            dock_control_state: state,
            factory_service: Rc::new(DockControlFactoryService::new()),
            is_initialized: Cell::new(false),
            content_control: RefCell::new(None),
            managed_window_layer: RefCell::new(None),
            command_bar_host: RefCell::new(None),
            command_bar_manager: RefCell::new(None),
            selector_overlay: RefCell::new(None),
            selector_mode: Cell::new(DockSelectorMode::Documents),
            selector_gesture: RefCell::new(None),
            activation_order: RefCell::new(HashMap::new()),
            activation_counter: Cell::new(0),
            subscribed_factory: RefCell::new(None),
            factory_subscription: RefCell::new(None),
            managed_layer_factory: RefCell::new(None),
            attached_window: RefCell::new(None),
            activated_subscription: RefCell::new(None),
            closing_subscription: RefCell::new(None),
            external_dock_surfaces: RefCell::new(Vec::new()),
            external_surfaces_version: Cell::new(0),
            drag_offset_calculator: RefCell::new(calculator),
            interface_handle: RefCell::new(Weak::new()),
            context_callbacks: RefCell::new(Vec::new()),
            host_callbacks: RefCell::new(Vec::new()),
        }
    }
    /// Creates the original dock host with an independent engine/options instance.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the original docking engine.
    pub fn dock_manager(&self) -> Rc<dyn IDockManager> {
        self.dock_manager.clone()
    }
    /// Gets the shared options used by this control's floating hosts.
    pub fn dock_manager_options(&self) -> Rc<DockManagerOptions> {
        self.dock_manager_options.clone()
    }
    /// Gets the original drag state engine.
    pub fn dock_control_state(&self) -> Rc<dyn IDockControlState> {
        self.dock_control_state.clone()
    }
    pub(crate) fn concrete_state(&self) -> Rc<DockControlState> {
        self.dock_control_state.clone()
    }
    /// Gets Layout.
    pub fn layout(&self) -> Option<Rc<dyn IDock>> {
        self.get_value(Self::layout_property())
    }
    /// Sets Layout through the original lifecycle callback.
    pub fn set_layout(&self, value: Option<Rc<dyn IDock>>) {
        self.set_value(Self::layout_property(), value);
    }
    /// Gets DefaultContext.
    pub fn default_context(&self) -> Option<BoxedValue> {
        self.get_value(Self::default_context_property())
    }
    /// Sets DefaultContext.
    pub fn set_default_context(&self, value: Option<BoxedValue>) {
        self.set_value(Self::default_context_property(), value);
    }
    /// Gets InitializeLayout.
    pub fn initialize_layout(&self) -> bool {
        self.get_value(Self::initialize_layout_property())
    }
    /// Sets InitializeLayout.
    pub fn set_initialize_layout(&self, value: bool) {
        self.set_value(Self::initialize_layout_property(), value);
    }
    /// Gets InitializeFactory.
    pub fn initialize_factory(&self) -> bool {
        self.get_value(Self::initialize_factory_property())
    }
    /// Sets InitializeFactory.
    pub fn set_initialize_factory(&self, value: bool) {
        self.set_value(Self::initialize_factory_property(), value);
    }
    /// Gets the caller-supplied host creation callback.
    pub fn host_window_factory(&self) -> Option<HostWindowFactory> {
        self.get_value(Self::host_window_factory_property())
            .map(|v| v.0)
    }
    /// Sets the caller-supplied host creation callback.
    pub fn set_host_window_factory(&self, value: Option<HostWindowFactory>) {
        self.set_value(
            Self::host_window_factory_property(),
            value.map(HostWindowFactoryValue),
        );
    }
    /// Gets Factory.
    pub fn factory(&self) -> Option<Rc<dyn IFactory>> {
        self.get_value(Self::factory_property())
    }
    /// Sets Factory.
    pub fn set_factory(&self, value: Option<Rc<dyn IFactory>>) {
        self.set_value(Self::factory_property(), value);
    }
    /// Gets IsDockingEnabled.
    pub fn is_docking_enabled(&self) -> bool {
        self.get_value(Self::is_docking_enabled_property())
    }
    /// Sets IsDockingEnabled and the original shared engine option.
    pub fn set_is_docking_enabled(&self, value: bool) {
        self.set_value(Self::is_docking_enabled_property(), value);
    }
    /// Gets IsDraggingDock.
    pub fn is_dragging_dock(&self) -> bool {
        self.get_value(Self::is_dragging_dock_property())
    }
    /// Sets IsDraggingDock.
    pub fn set_is_dragging_dock(&self, value: bool) {
        self.set_value(Self::is_dragging_dock_property(), value);
    }
    /// Gets EnableManagedWindowLayer.
    pub fn enable_managed_window_layer(&self) -> bool {
        self.get_value(Self::enable_managed_window_layer_property())
    }
    /// Sets EnableManagedWindowLayer.
    pub fn set_enable_managed_window_layer(&self, value: bool) {
        self.set_value(Self::enable_managed_window_layer_property(), value);
    }
    /// Gets AutoCreateDataTemplates.
    pub fn auto_create_data_templates(&self) -> bool {
        self.get_value(Self::auto_create_data_templates_property())
    }
    /// Sets AutoCreateDataTemplates.
    pub fn set_auto_create_data_templates(&self, value: bool) {
        self.set_value(Self::auto_create_data_templates_property(), value);
    }
    /// Gets whether the selector overlay is open.
    pub fn is_open(&self) -> bool {
        self.selector_overlay().is_some_and(|s| s.is_open())
    }
    /// Gets the original drag offset calculator.
    pub fn drag_offset_calculator(&self) -> Rc<dyn IDragOffsetCalculator> {
        self.drag_offset_calculator.borrow().clone()
    }
    /// Updates the calculator on both the control and its original state engine.
    pub fn set_drag_offset_calculator(&self, value: Rc<dyn IDragOffsetCalculator>) {
        let old = self.drag_offset_calculator.replace(value.clone());
        drop(old);
        self.dock_control_state.set_drag_offset_calculator(value);
    }
    fn content_control(&self) -> Option<Ref<Control>> {
        self.content_control.borrow().clone()
    }
    fn selector_overlay(&self) -> Option<Ref<DockSelectorOverlay>> {
        self.selector_overlay.borrow().clone()
    }
    fn managed_window_layer(&self) -> Option<Ref<ManagedWindowLayer>> {
        self.managed_window_layer.borrow().clone()
    }
    fn command_bar_manager(&self) -> Option<Rc<DockCommandBarManager>> {
        self.command_bar_manager.borrow().clone()
    }
    fn interface_handle(control: Ref<Self>) -> Rc<DockControlHandle> {
        let cached = control.interface_handle.borrow().upgrade();
        if let Some(cached) = cached {
            return cached;
        }
        let handle = Rc::new(DockControlHandle(control.clone()));
        control.interface_handle.replace(Rc::downgrade(&handle));
        handle
    }
    /// Views this native object through the original engine contract.
    pub fn as_dock_control(control: Ref<Self>) -> Rc<dyn IDockControl> {
        Self::interface_handle(control)
    }
    /// Views this native object through the selector contract.
    pub fn as_selector_service(control: Ref<Self>) -> Rc<dyn IDockSelectorService> {
        Self::interface_handle(control)
    }
    /// Registers the exact external surface, transferring its former owner first.
    pub fn register_external_dock_surface(&self, surface: Rc<dyn IExternalDockSurface>) {
        if let Some(previous) = surface.dock_control().filter(|c| !c.ptr_eq(&self.to_ref())) {
            previous.unregister_external_dock_surface(surface.clone());
        }
        self.prune_external_dock_surface_references();
        if !self.contains_external_dock_surface(&surface) {
            self.external_dock_surfaces
                .borrow_mut()
                .push(ExternalSurfaceReference::new(&surface));
            self.external_surfaces_version
                .set(self.external_surfaces_version.get().wrapping_add(1));
        }
        surface.set_dock_control(Some(self.to_ref()));
    }
    /// Unregisters the exact surface and clears its owner only when this control owns it.
    pub fn unregister_external_dock_surface(&self, surface: Rc<dyn IExternalDockSurface>) -> bool {
        let removed = self.remove_external_dock_surface_references(&surface);
        if surface
            .dock_control()
            .is_some_and(|c| c.ptr_eq(&self.to_ref()))
        {
            surface.set_dock_control(None);
        }
        removed
    }
    pub(crate) fn enumerate_external_dock_surface_controls(&self) -> ExternalSurfaceControls {
        // Iterator-body pruning starts on first MoveNext, as in the source yield method.
        ExternalSurfaceControls {
            control: self.to_ref(),
            started: false,
            index: 0,
            version: 0,
        }
    }
    fn contains_external_dock_surface(&self, surface: &Rc<dyn IExternalDockSurface>) -> bool {
        let mut index = 0;
        let version = self.external_surfaces_version.get();
        loop {
            self.check_external_surface_version(version);
            let entry = self.external_dock_surfaces.borrow().get(index).cloned();
            index += 1;
            let Some(entry) = entry else {
                return false;
            };
            if entry
                .upgrade()
                .is_some_and(|c| c.as_ref() == surface.as_ref())
            {
                return true;
            }
        }
    }
    fn remove_external_reference(&self, index: usize) {
        let old = self.external_dock_surfaces.borrow_mut().remove(index);
        self.external_surfaces_version
            .set(self.external_surfaces_version.get().wrapping_add(1));
        drop(old);
    }
    fn check_external_surface_version(&self, version: u32) {
        assert_eq!(
            self.external_surfaces_version.get(),
            version,
            "Collection was modified; enumeration operation may not execute."
        );
    }
    fn remove_external_dock_surface_references(
        &self,
        surface: &Rc<dyn IExternalDockSurface>,
    ) -> bool {
        let mut removed = false;
        let count = self.external_dock_surfaces.borrow().len();
        for index in (0..count).rev() {
            let reference = self.external_dock_surfaces.borrow()[index].clone();
            let candidate = reference.upgrade();
            if candidate.is_none() {
                self.remove_external_reference(index);
                continue;
            }
            if candidate.as_ref().expect("source live surface").as_ref() != surface.as_ref() {
                continue;
            }
            self.remove_external_reference(index);
            removed = true;
        }
        removed
    }
    fn prune_external_dock_surface_references(&self) {
        let count = self.external_dock_surfaces.borrow().len();
        for index in (0..count).rev() {
            let reference = self.external_dock_surfaces.borrow()[index].clone();
            let surface = reference.upgrade();
            if surface.is_none() {
                self.remove_external_reference(index);
                continue;
            }
            if !surface
                .expect("source live surface")
                .dock_control()
                .is_some_and(|c| c.ptr_eq(&self.to_ref()))
            {
                self.remove_external_reference(index);
            }
        }
    }
    fn detach_external_dock_surface_owners(&self) {
        let mut index = 0;
        let version = self.external_surfaces_version.get();
        loop {
            self.check_external_surface_version(version);
            let entry = self.external_dock_surfaces.borrow().get(index).cloned();
            index += 1;
            let Some(entry) = entry else {
                return;
            };
            let Some(surface) = entry.upgrade() else {
                continue;
            };
            if surface
                .dock_control()
                .is_some_and(|c| c.ptr_eq(&self.to_ref()))
            {
                surface.set_dock_control(None);
            }
        }
    }
    fn restore_external_dock_surface_owners(&self) {
        let count = self.external_dock_surfaces.borrow().len();
        for index in (0..count).rev() {
            let reference = self.external_dock_surfaces.borrow()[index].clone();
            let Some(surface) = reference.upgrade() else {
                self.remove_external_reference(index);
                continue;
            };
            if surface
                .dock_control()
                .is_some_and(|c| c.ptr_eq(&self.to_ref()))
            {
                continue;
            }
            if surface.dock_control().is_none() {
                surface.set_dock_control(Some(self.to_ref()));
                continue;
            }
            self.remove_external_reference(index);
        }
    }
    fn initialize_default_data_templates(&self) {
        if self.content_control().is_none() {
            return;
        }
        if !self.auto_create_data_templates() {
            return;
        }
        for template in DockDataTemplateHelper::create_default_data_templates() {
            self.content_control()
                .expect("source content re-read")
                .data_templates()
                .add(template);
        }
    }
    fn initialize(&self, layout: Option<Rc<dyn IDock>>) {
        let Some(layout) = layout else {
            return;
        };
        if layout.factory().is_none() {
            if let Some(factory) = self.factory() {
                layout.set_factory(Some(factory));
            } else {
                return;
            }
        }
        layout
            .factory()
            .expect("source layout factory")
            .dock_controls()
            .add(Self::as_dock_control(self.to_ref()));
        if self.content_control().is_some() {
            self.factory_service.initialize_control_recycling(self);
        }
        self.update_managed_window_layer(Some(layout.clone()));
        if self.initialize_factory() {
            if layout
                .factory()
                .expect("source factory re-read")
                .context_locator()
                .is_none()
            {
                layout
                    .factory()
                    .expect("source factory re-read")
                    .set_context_locator(Some(Rc::new(RefCell::new(Dictionary::new()))));
            }
            if layout
                .factory()
                .expect("source factory re-read")
                .dockable_locator()
                .is_none()
            {
                layout
                    .factory()
                    .expect("source factory re-read")
                    .set_dockable_locator(Some(Rc::new(RefCell::new(Dictionary::new()))));
            }
            if layout
                .factory()
                .expect("source factory re-read")
                .default_context_locator()
                .is_none()
            {
                layout
                    .factory()
                    .expect("source factory re-read")
                    .set_default_context_locator(Some(self.default_context_callback()));
            }
            if layout
                .factory()
                .expect("source factory re-read")
                .default_host_window_locator()
                .is_none()
            {
                layout
                    .factory()
                    .expect("source factory re-read")
                    .set_default_host_window_locator(Some(self.default_host_window_callback()));
            }
            if layout
                .factory()
                .expect("source factory re-read")
                .host_window_locator()
                .is_none()
            {
                let mut locators = Dictionary::new();
                locators.insert(
                    "IDockWindow".into(),
                    Some(self.default_host_window_callback()),
                );
                layout
                    .factory()
                    .expect("source factory re-read")
                    .set_host_window_locator(Some(Rc::new(RefCell::new(locators))));
            }
        }
        if self.initialize_layout() {
            layout
                .factory()
                .expect("source factory re-read")
                .init_layout(layout.clone());
        }
        self.attach_factory_events(layout.factory());
        if let Some(active) = layout.active_dockable() {
            self.record_activation(active);
        }
        if let Some(manager) = self.command_bar_manager() {
            manager.attach(layout);
        }
        self.is_initialized.set(true);
    }
    fn deinitialize(&self, layout: Option<Rc<dyn IDock>>) {
        self.unregister_managed_window_layer();
        let Some(layout) = layout.filter(|l| l.factory().is_some()) else {
            return;
        };
        layout
            .factory()
            .expect("source factory")
            .dock_controls()
            .remove(&Self::as_dock_control(self.to_ref()));
        let old = std::mem::take(&mut *self.activation_order.borrow_mut());
        drop(old);
        self.activation_counter.set(0);
        self.factory_service.cleanup_factory(self, layout.clone());
        if self.initialize_layout() && layout.close().can_execute(None) {
            layout.close().execute(None);
        }
        self.detach_factory_events();
        if let Some(manager) = self.command_bar_manager() {
            manager.detach();
        }
        self.is_initialized.set(false);
    }
    fn initialize_command_bars(&self) {
        if let Some(manager) = self.command_bar_manager() {
            manager.detach();
        }
        self.command_bar_manager.replace(None);
        let Some(host) = self.command_bar_host.borrow().clone() else {
            return;
        };
        let manager = DockCommandBarManager::new(host, None);
        self.command_bar_manager.replace(Some(manager));
        if let Some(layout) = self.layout() {
            self.command_bar_manager()
                .expect("source manager")
                .attach(layout);
        }
    }
    pub(crate) fn resolve_default_context(&self) -> Option<ModelValue> {
        self.default_context().map(to_recycling_value)
    }
    pub(crate) fn resolve_default_host_window(&self) -> Option<Rc<dyn IHostWindow>> {
        if let Some(factory) = self.host_window_factory() {
            return factory();
        }
        let mode = DockSettings::resolve_floating_window_host_mode(
            self.layout().and_then(|l| l.as_root_dock()).as_deref(),
        );
        if mode == DockFloatingWindowHostMode::Managed {
            Some(ManagedHostWindow::with_options(
                self.dock_manager_options.clone(),
            ))
        } else {
            Some(HostWindow::as_host_window(HostWindow::with_options(
                self.dock_manager_options.clone(),
            )))
        }
    }
    pub(crate) fn default_context_callback(&self) -> ContextFactory {
        let owner = self.to_ref();
        let callback: ContextFactory = Rc::new(move || owner.resolve_default_context());
        let mut entries = self.context_callbacks.borrow_mut();
        entries.retain(|c| c.strong_count() != 0);
        entries.push(Rc::downgrade(&callback));
        callback
    }
    pub(crate) fn default_host_window_callback(&self) -> HostWindowFactory {
        let owner = self.to_ref();
        let callback: HostWindowFactory = Rc::new(move || owner.resolve_default_host_window());
        let mut entries = self.host_callbacks.borrow_mut();
        entries.retain(|c| c.strong_count() != 0);
        entries.push(Rc::downgrade(&callback));
        callback
    }
    pub(crate) fn owns_context_callback(&self, callback: &ContextFactory) -> bool {
        self.context_callbacks
            .borrow()
            .iter()
            .any(|c| c.ptr_eq(&Rc::downgrade(callback)))
    }
    pub(crate) fn owns_host_callback(&self, callback: &HostWindowFactory) -> bool {
        self.host_callbacks
            .borrow()
            .iter()
            .any(|c| c.ptr_eq(&Rc::downgrade(callback)))
    }
    fn attach_factory_events(&self, factory: Option<Rc<dyn IFactory>>) {
        if self.subscribed_factory.borrow().as_ref() == factory.as_ref() {
            return;
        }
        self.detach_factory_events();
        let Some(factory) = factory else {
            return;
        };
        self.subscribed_factory.replace(Some(factory.clone()));
        let owner = self.to_ref();
        let subscription = factory
            .active_dockable_changed(Rc::new(move |e| owner.factory_active_dockable_changed(e)));
        self.factory_subscription.replace(Some(subscription));
    }
    fn detach_factory_events(&self) {
        if self.subscribed_factory.borrow().is_none() {
            return;
        }
        let subscription = self.factory_subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
        self.subscribed_factory.replace(None);
    }
    fn factory_active_dockable_changed(&self, e: &ActiveDockableChangedEventArgs) {
        if e.dockable().is_none() || self.layout().and_then(|l| l.factory()).is_none() {
            return;
        }
        let layout = self.layout().expect("source layout re-read");
        let layout_root = layout
            .factory()
            .expect("source factory")
            .find_root(Some(layout), Some(Rc::new(|_| true)));
        let layout = self.layout().expect("source layout re-read");
        let dockable_root = layout.factory().expect("source factory").find_root(
            Some(e.dockable().expect("source dockable")),
            Some(Rc::new(|_| true)),
        );
        if let (Some(a), Some(b)) = (layout_root, dockable_root) {
            if !Rc::ptr_eq(&a, &b) {
                return;
            }
        }
        self.record_activation(e.dockable().expect("source dockable"));
    }
    fn update_managed_window_layer(&self, layout: Option<Rc<dyn IDock>>) {
        if self.managed_window_layer().is_none() {
            return;
        }
        if self.enable_managed_window_layer()
            && DockSettings::is_managed_window_hosting_enabled(
                layout.clone().and_then(|l| l.as_root_dock()).as_deref(),
            )
        {
            if let Some(factory) = layout.and_then(|l| l.factory()) {
                if self.managed_layer_factory.borrow().as_ref() != Some(&factory) {
                    self.unregister_managed_window_layer();
                }
                ManagedWindowRegistry::register_layer(
                    &factory,
                    &self.managed_window_layer().expect("source layer re-read"),
                );
                self.managed_layer_factory.replace(Some(factory));
                self.managed_window_layer()
                    .expect("source layer re-read")
                    .set_is_visible(true);
                return;
            }
        }
        self.unregister_managed_window_layer();
        self.managed_window_layer()
            .expect("source layer re-read")
            .set_is_visible(false);
    }
    fn unregister_managed_window_layer(&self) {
        let Some(layer) = self.managed_window_layer() else {
            return;
        };
        let Some(factory) = self.managed_layer_factory.borrow().clone() else {
            return;
        };
        ManagedWindowRegistry::unregister_layer(&factory, &layer);
        self.managed_layer_factory.replace(None);
    }
    /// Shows the original selector if the global selector option is enabled.
    pub fn show_selector(&self, mode: DockSelectorMode) {
        if DockSettings::selector_enabled() {
            self.open_selector(mode, None);
        }
    }
    /// Hides and clears the original selector overlay and gesture.
    pub fn hide_selector(&self) {
        if self.selector_overlay().is_none() {
            return;
        }
        self.selector_overlay()
            .expect("source overlay re-read")
            .set_is_open(false);
        self.selector_overlay()
            .expect("source overlay re-read")
            .set_items(None);
        self.selector_overlay()
            .expect("source overlay re-read")
            .set_selected_item(None);
        self.selector_gesture.replace(None);
    }
    fn key_down_handler(&self, e: &KeyEventArgs) {
        if !DockSettings::selector_enabled() {
            return;
        }
        if self.is_open() && self.handle_selector_navigation(e) {
            e.set_handled(true);
            return;
        }
        if self.try_start_selector(e) {
            e.set_handled(true);
        }
    }
    fn key_up_handler(&self, e: &KeyEventArgs) {
        if !self.is_open() {
            return;
        }
        let Some(gesture) = *self.selector_gesture.borrow() else {
            return;
        };
        let required = gesture.key_modifiers();
        if e.key_modifiers.contains(required) {
            return;
        }
        self.commit_selector_selection();
        e.set_handled(true);
    }
    fn try_start_selector(&self, e: &KeyEventArgs) -> bool {
        if self.selector_overlay().is_none() {
            return false;
        }
        let gesture = DockSettings::document_selector_key_gesture();
        if let Some(reverse) = matches_selector_gesture(e, &gesture) {
            self.open_selector(
                DockSelectorMode::Documents,
                Some(DockSettings::document_selector_key_gesture()),
            );
            self.move_selector_selection(if reverse { -1 } else { 1 });
            return true;
        }
        let gesture = DockSettings::tool_selector_key_gesture();
        if let Some(reverse) = matches_selector_gesture(e, &gesture) {
            self.open_selector(
                DockSelectorMode::Tools,
                Some(DockSettings::tool_selector_key_gesture()),
            );
            self.move_selector_selection(if reverse { -1 } else { 1 });
            return true;
        }
        false
    }
    fn handle_selector_navigation(&self, e: &KeyEventArgs) -> bool {
        if !self.is_open() {
            return false;
        }
        match e.key {
            Key::Escape => self.hide_selector(),
            Key::Enter => self.commit_selector_selection(),
            Key::Tab => {
                self.move_selector_selection(if e.key_modifiers.contains(KeyModifiers::SHIFT) {
                    -1
                } else {
                    1
                })
            }
            Key::Left | Key::Up => self.move_selector_selection(-1),
            Key::Right | Key::Down => self.move_selector_selection(1),
            _ => return false,
        }
        true
    }
    fn open_selector(&self, mode: DockSelectorMode, gesture: Option<KeyGesture>) {
        if self.selector_overlay().is_none() {
            return;
        }
        self.selector_mode.set(mode);
        self.selector_gesture.replace(gesture);
        let items = self.build_selector_items(mode);
        self.selector_overlay()
            .expect("source overlay re-read")
            .set_items(Some(items.clone()));
        self.selector_overlay()
            .expect("source overlay re-read")
            .set_mode(mode);
        self.selector_overlay()
            .expect("source overlay re-read")
            .set_is_open(true);
        let active = self.resolve_active_dockable(mode);
        self.selector_overlay()
            .expect("source overlay re-read")
            .set_selected_item(
                items
                    .iter()
                    .find(|i| {
                        active
                            .as_ref()
                            .is_some_and(|a| Rc::ptr_eq(&i.dockable(), a))
                    })
                    .or_else(|| items.iter().next()),
            );
    }
    fn commit_selector_selection(&self) {
        if let Some(selected) = self.selector_overlay().and_then(|o| o.selected_item()) {
            self.activate_selector_item(selected);
        }
        self.hide_selector();
    }
    fn move_selector_selection(&self, delta: i32) {
        let Some((_overlay, items)) = self
            .selector_overlay()
            .and_then(|o| o.items().map(|i| (o, i)))
        else {
            return;
        };
        if items.count() == 0 {
            return;
        }
        let current = self
            .selector_overlay()
            .expect("source overlay re-read")
            .selected_item();
        let mut index = -1;
        for i in 0..items.count() {
            if current
                .as_ref()
                .is_some_and(|c| Rc::ptr_eq(&items.get(i), c))
            {
                index = i as i32;
                break;
            }
        }
        if index < 0 {
            index = 0;
        }
        let mut next = index.wrapping_add(delta);
        if next < 0 {
            next = items.count() as i32 - 1;
        } else if next >= items.count() as i32 {
            next = 0;
        }
        self.selector_overlay()
            .expect("source overlay re-read")
            .set_selected_item(Some(items.get(next as usize)));
    }
    fn build_selector_items(
        &self,
        mode: DockSelectorMode,
    ) -> ReadOnlyListHandle<Rc<DockSelectorItem>> {
        let empty = || ReadOnlyListHandle::from_items(Vec::new());
        if self.layout().and_then(|l| l.factory()).is_none() {
            return empty();
        }
        let layout = self.layout().expect("source layout");
        let factory = layout.factory().expect("source factory");
        let Some(root) = factory.find_root(Some(layout), Some(Rc::new(|_| true))) else {
            return empty();
        };
        let mut dockables = Vec::new();
        let mut visited = Dictionary::new();
        collect_dockables(Some(root.clone()), &mut dockables, &mut visited);
        let mut items = Vec::new();
        for dockable in dockables {
            if dockable
                .as_dock_selector_info()
                .is_some_and(|i| !i.show_in_selector())
            {
                continue;
            }
            let is_document = dockable.clone().as_document().is_some();
            let is_tool = dockable.clone().as_tool().is_some();
            if mode == DockSelectorMode::Documents && !is_document {
                continue;
            }
            if mode == DockSelectorMode::Tools && !is_tool {
                continue;
            }
            let activation_order = self
                .activation_order
                .borrow()
                .get(&Self::model_identity(&dockable))
                .filter(|(key, _)| key.upgrade().is_some())
                .map_or(0, |(_, v)| *v);
            let dockable_root = factory.find_root(Some(dockable.clone()), Some(Rc::new(|_| true)));
            let floating = dockable_root.is_some_and(|r| !Rc::ptr_eq(&r, &root));
            let hidden = root
                .hidden_dockables()
                .is_some_and(|l| l.contains(&Some((dockable).clone())));
            let pinned = factory.is_dockable_pinned(Some(dockable.clone()), Some(root.clone()));
            items.push(DockSelectorItem::new(
                dockable,
                activation_order,
                is_document,
                is_tool,
                pinned,
                hidden,
                floating,
            ));
        }
        // Stable sort preserves source traversal order for equal keys. This uses the existing
        // captured .NET simple-uppercase map; Rust full uppercase would alter selector order.
        items.sort_by(|a, b| {
            b.activation_order()
                .cmp(&a.activation_order())
                .then_with(|| compare_selector_titles(&a.title(), &b.title()))
        });
        ReadOnlyListHandle::from_items(items)
    }
    fn model_identity(model: &Rc<dyn IDockable>) -> usize {
        Rc::as_ptr(model) as *const () as usize
    }
    fn record_activation(&self, dockable: Rc<dyn IDockable>) {
        let mut order = self.activation_order.borrow_mut();
        order.retain(|_, (key, _)| key.strong_count() != 0);
        let entry = order
            .entry(Self::model_identity(&dockable))
            .or_insert_with(|| (Rc::downgrade(&dockable), 0));
        let next = self.activation_counter.get().wrapping_add(1);
        self.activation_counter.set(next);
        entry.1 = next;
    }
    fn resolve_active_dockable(&self, mode: DockSelectorMode) -> Option<Rc<dyn IDockable>> {
        let layout = self.layout()?;
        let mut active = layout.active_dockable();
        if let Some(dock) = active.clone().and_then(|a| a.as_dock()) {
            if dock.active_dockable().is_some() {
                active = dock.active_dockable();
            }
        }
        match mode {
            DockSelectorMode::Documents => active.filter(|a| a.clone().as_document().is_some()),
            DockSelectorMode::Tools => active.filter(|a| a.clone().as_tool().is_some()),
            _ => active,
        }
    }
    fn activate_selector_item(&self, item: Rc<DockSelectorItem>) {
        let dockable = item.dockable();
        let Some(factory) = dockable.factory() else {
            return;
        };
        let dockable_root = factory.find_root(Some(dockable.clone()), Some(Rc::new(|_| true)));
        if dockable_root
            .as_ref()
            .and_then(|r| r.hidden_dockables())
            .is_some_and(|l| l.contains(&Some((dockable).clone())))
        {
            factory.restore_dockable(Some(dockable.clone()));
        }
        if let Some(root) = &dockable_root {
            if factory.is_dockable_pinned(Some(dockable.clone()), Some(root.clone())) {
                factory.preview_pinned_dockable(Some(dockable.clone()));
            }
        }
        factory.set_active_dockable(Some(dockable.clone()));
        let focus = factory
            .find_root(
                Some(dockable.clone()),
                Some(Rc::new(|r| r.is_focusable_root())),
            )
            .or_else(|| dockable_root.clone());
        if let Some(root) = focus {
            factory.set_focused_dockable(Some(root), Some(dockable));
        }
        self.bring_dockable_window_to_front(dockable_root);
    }
    fn bring_dockable_window_to_front(&self, root: Option<Rc<dyn IRootDock>>) {
        let Some(root) = root else {
            return;
        };
        if self.layout().and_then(|l| l.factory()).is_none() {
            return;
        }
        let layout = self.layout().expect("source layout");
        let layout_root = layout
            .factory()
            .expect("source factory")
            .find_root(Some(layout), Some(Rc::new(|_| true)));
        let Some(layout_root) = layout_root.filter(|r| !Rc::ptr_eq(r, &root)) else {
            return;
        };
        let window = layout_root.windows().and_then(|w| {
            w.iter().find(|c| {
                c.as_ref()
                    .expect("source candidate.Layout access")
                    .layout()
                    .is_some_and(|l| Rc::ptr_eq(&l, &root))
            })
        });
        if let Some(Some(window)) = window {
            window.set_active();
            window.present(window.is_modal());
        }
    }
    fn notify_root_window_closed(&self, layout: Option<Rc<dyn IDock>>) {
        let Some(root) = layout.and_then(|l| l.as_root_dock()) else {
            return;
        };
        if TopLevel::get_top_level(Some(self))
            .and_then(|t| native_host(&t))
            .is_some()
        {
            return;
        }
        if root.window().is_none() {
            return;
        }
        if let Some(factory) = root.factory() {
            factory.on_window_closed(root.window());
        }
    }
    fn attach_main_window_handlers(&self, window: Option<Ref<Window>>) {
        let Some(window) = window.filter(|w| native_host(w).is_none()) else {
            return;
        };
        if self.attached_window.borrow().as_ref() == Some(&window) {
            return;
        }
        self.detach_main_window_handlers();
        self.attached_window.replace(Some(window.clone()));
        let owner = self.to_ref();
        self.activated_subscription.replace(Some(
            window.activated(move || owner.on_main_window_activated()),
        ));
        if DockSettings::close_floating_windows_on_main_window_close() {
            let owner = self.to_ref();
            let sender = window.clone();
            self.closing_subscription.replace(Some(
                window.closing(move |_| owner.on_main_window_closing(sender.clone())),
            ));
        }
    }
    fn detach_main_window_handlers(&self) {
        if self.attached_window.borrow().is_none() {
            return;
        }
        let activated = self.activated_subscription.borrow_mut().take();
        if let Some(s) = activated {
            s.dispose();
        }
        let closing = self.closing_subscription.borrow_mut().take();
        if let Some(s) = closing {
            s.dispose();
        }
        self.attached_window.replace(None);
    }
    fn on_main_window_activated(&self) {
        let Some((layout, factory)) = self.layout().and_then(|l| l.factory().map(|f| (l, f)))
        else {
            return;
        };
        let Some(root) = factory.find_root(Some(layout), Some(Rc::new(|_| true))) else {
            return;
        };
        factory.on_window_activated_with_root(root.window(), Some(root));
    }
    fn on_main_window_closing(&self, window: Ref<Window>) {
        let Some(root) = self.layout().and_then(|l| {
            l.factory()
                .and_then(|f| f.find_root(Some(l), Some(Rc::new(|_| true))))
        }) else {
            return;
        };
        Dispatcher::ui_thread().post_local(
            move || {
                if window.is_visible() {
                    return;
                }
                if root.exit_windows().can_execute(None) {
                    root.exit_windows().execute(None);
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    fn should_ignore_pressed_for_window_drag(source: Option<&Control>) -> bool {
        let Some(source) = source else {
            return false;
        };
        if Self::should_ignore_pressed_for_floating_tool_window_drag(source) {
            return true;
        }
        let Some(item) = source.find_ancestor_of_type::<DocumentTabStripItem>(false) else {
            return false;
        };
        let Some(strip) = item.find_ancestor_of_type::<DocumentTabStrip>(false) else {
            return false;
        };
        strip.items().count() == 1
            && model_projection::dockable(strip.data_context().as_ref())
                .and_then(|d| d.as_dock())
                .is_some_and(|d| !d.can_close_last_dockable())
    }
    fn should_ignore_pressed_for_floating_tool_window_drag(source: &Control) -> bool {
        let chrome = source
            .to_ref()
            .cast::<ToolChromeControl>()
            .or_else(|| source.find_ancestor_of_type::<ToolChromeControl>(false));
        chrome
            .and_then(|c| TopLevel::get_top_level(Some(&c)))
            .and_then(|t| t.cast::<HostWindow>())
            .is_some_and(|w| w.is_tool_window() && w.tool_chrome_controls_whole_window())
    }
    fn should_ignore_pressed_for_item_drag(source: Option<&Control>) -> bool {
        let Some(source) = source else {
            return false;
        };
        source
            .find_ancestor_of_type::<DocumentTabStripItem>(false)
            .is_some()
            || source
                .find_ancestor_of_type::<ToolTabStripItem>(false)
                .is_some()
            || source
                .find_ancestor_of_type::<ToolPinItemControl>(false)
                .is_some()
    }
    fn pressed_handler(&self, e: &PointerPressedEventArgs) {
        if !e
            .get_current_point(Some(self))
            .properties
            .is_left_button_pressed
        {
            return;
        }
        if e.key_modifiers().contains(KeyModifiers::CONTROL)
            && e.key_modifiers().contains(KeyModifiers::SHIFT)
        {
            if let Some(initial) = self
                .input_hit_test(e.get_position(Some(self)))
                .and_then(|c| c.cast::<Control>())
            {
                let mut dockable = None;
                let mut control = Some(initial);
                while control.is_some() && dockable.is_none() {
                    let current = control.expect("source current hit");
                    dockable = model_projection::dockable(current.data_context().as_ref());
                    control = current.find_ancestor_of_type::<Control>(false);
                }
                DockDiagnosticEvents::raise_select_dockable_requested(dockable);
            }
            return;
        }
        if e.source()
            .and_then(|s| s.cast::<Visual>())
            .is_some_and(|v| v.find_ancestor_of_type::<ScrollBar>(false).is_some())
        {
            return;
        }
        let source = e.source().and_then(|s| s.cast::<Control>());
        if Self::should_ignore_pressed_for_window_drag(source.as_deref())
            || Self::should_ignore_pressed_for_item_drag(source.as_deref())
        {
            return;
        }
        if let Some(layout) = self.layout().filter(|l| l.factory().is_some()) {
            let position = e.get_position(Some(self));
            let action = to_drag_action(e.key_modifiers());
            self.dock_control_state.process(
                position,
                Default::default(),
                EventType::Pressed,
                action,
                Some(self),
                layout.factory().expect("source factory").dock_controls(),
            );
        }
    }
    fn released_handler(&self, e: &PointerReleasedEventArgs) {
        if e.initial_press_mouse_button() != MouseButton::Left {
            return;
        }
        self.process_pointer(e, EventType::Released, Default::default());
    }
    fn moved_handler(&self, e: &PointerEventArgs) {
        if !e
            .get_current_point(Some(self))
            .properties
            .is_left_button_pressed
        {
            return;
        }
        self.process_pointer(e, EventType::Moved, Default::default());
    }
    fn entered_handler(&self, e: &PointerEventArgs) {
        self.process_pointer(e, EventType::Enter, Default::default());
    }
    fn exited_handler(&self, e: &PointerEventArgs) {
        self.process_pointer(e, EventType::Leave, Default::default());
    }
    fn capture_lost_handler(&self, e: &PointerCaptureLostEventArgs) {
        if self.try_transfer_drag_capture(e.pointer()) {
            return;
        }
        if let Some(layout) = self.layout().filter(|l| l.factory().is_some()) {
            self.dock_control_state.process(
                Default::default(),
                Default::default(),
                EventType::CaptureLost,
                DragAction::NONE,
                Some(self),
                layout.factory().expect("source factory").dock_controls(),
            );
        }
    }
    pub(crate) fn try_transfer_drag_capture(&self, pointer: &Rc<dyn IPointer>) -> bool {
        if self.visual_root().is_some() || !self.dock_control_state.has_active_drag() {
            return false;
        }
        let Some(controls) = self
            .layout()
            .and_then(|l| l.factory())
            .map(|f| f.dock_controls())
        else {
            return false;
        };
        for target in DockHelpers::get_z_ordered_dock_controls(controls) {
            if target.ptr_eq(&self.to_ref()) || target.visual_root().is_none() {
                continue;
            }
            if !self
                .dock_control_state
                .try_transfer_drag_to(target.concrete_state(), self, &target)
            {
                continue;
            }
            pointer.capture(Some(&target.clone().upcast::<InputElement>()));
            return true;
        }
        false
    }
    fn wheel_changed_handler(&self, e: &PointerWheelEventArgs) {
        self.process_pointer(e, EventType::WheelChanged, e.delta());
    }
    fn process_pointer(&self, e: &PointerEventArgs, phase: EventType, delta: ferroui_base::Vector) {
        if let Some(layout) = self.layout().filter(|l| l.factory().is_some()) {
            let position = e.get_position(Some(self));
            let action = to_drag_action(e.key_modifiers());
            self.dock_control_state.process(
                position,
                delta,
                phase,
                action,
                Some(self),
                layout.factory().expect("source factory").dock_controls(),
            );
        }
    }
}
// Both interfaces share a canonical handle while live and forward to the SAME native object.
// Weak interface-handle lifetime vs CLR WeakReference remains an integration test obligation.
struct DockControlHandle(Ref<DockControl>);
impl IDockControl for DockControlHandle {
    fn boxed_interface(self: Rc<Self>, target: TypeId) -> Option<ModelValue> {
        if target == TypeId::of::<Ref<DockControl>>() {
            Some(Rc::new(self.0.clone()))
        } else if target == TypeId::of::<Ref<Control>>() {
            Some(Rc::new(self.0.clone().upcast::<Control>()))
        } else {
            None
        }
    }
    fn dock_manager(&self) -> Rc<dyn IDockManager> {
        self.0.dock_manager()
    }
    fn dock_control_state(&self) -> Rc<dyn IDockControlState> {
        self.0.dock_control_state()
    }
    fn layout(&self) -> Option<Rc<dyn IDock>> {
        self.0.layout()
    }
    fn set_layout(&self, v: Option<Rc<dyn IDock>>) {
        self.0.set_layout(v);
    }
    fn default_context(&self) -> Option<ModelValue> {
        self.0.default_context().map(to_recycling_value)
    }
    fn set_default_context(&self, v: Option<ModelValue>) {
        self.0
            .set_default_context(v.as_ref().map(from_recycling_value));
    }
    fn initialize_layout(&self) -> bool {
        self.0.initialize_layout()
    }
    fn set_initialize_layout(&self, v: bool) {
        self.0.set_initialize_layout(v);
    }
    fn initialize_factory(&self) -> bool {
        self.0.initialize_factory()
    }
    fn set_initialize_factory(&self, v: bool) {
        self.0.set_initialize_factory(v);
    }
    fn factory(&self) -> Option<Rc<dyn IFactory>> {
        self.0.factory()
    }
    fn set_factory(&self, v: Option<Rc<dyn IFactory>>) {
        self.0.set_factory(v);
    }
}
impl IDockSelectorService for DockControlHandle {
    fn show_selector(&self, mode: DockSelectorMode) {
        self.0.show_selector(mode);
    }
    fn hide_selector(&self) {
        self.0.hide_selector();
    }
    fn is_open(&self) -> bool {
        self.0.is_open()
    }
    fn as_object(&self) -> Option<Ref<ferroui_base::FerroObject>> {
        Some(self.0.clone().upcast())
    }
}
fn native_host(top: &TopLevel) -> Option<Rc<dyn IHostWindow>> {
    let boxed: BoxedValue = Rc::new(top.to_ref());
    ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn IHostWindow>>())?
        .downcast_ref::<Rc<dyn IHostWindow>>()
        .cloned()
}

// A weak native interface tracks the native receiver, not a short-lived Rc adapter.
// Native classes declare the real interface cast through ferro_class_info!.
#[derive(Clone)]
enum ExternalSurfaceReference {
    Native(ferroui_base::WeakRef<ferroui_base::FerroObject>),
    Contract(Weak<dyn IExternalDockSurface>),
}
impl ExternalSurfaceReference {
    fn new(surface: &Rc<dyn IExternalDockSurface>) -> Self {
        match surface.as_object() {
            Some(object) => Self::Native(object.downgrade()),
            None => Self::Contract(Rc::downgrade(surface)),
        }
    }
    fn upgrade(&self) -> Option<Rc<dyn IExternalDockSurface>> {
        match self {
            Self::Contract(reference) => reference.upgrade(),
            Self::Native(reference) => {
                let value: BoxedValue = Rc::new(reference.upgrade()?);
                Some(
                    ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IExternalDockSurface>>())
                        .and_then(|v| v.downcast_ref::<Rc<dyn IExternalDockSurface>>().cloned())
                        .expect("native surface declares its original interface"),
                )
            }
        }
    }
}
pub(crate) struct ExternalSurfaceControls {
    control: Ref<DockControl>,
    started: bool,
    index: usize,
    version: u32,
}
impl Iterator for ExternalSurfaceControls {
    type Item = Ref<Control>;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.started {
            self.control.prune_external_dock_surface_references();
            self.started = true;
            self.version = self.control.external_surfaces_version.get();
        }
        loop {
            self.control.check_external_surface_version(self.version);
            let entry = self
                .control
                .external_dock_surfaces
                .borrow()
                .get(self.index)
                .cloned()?;
            self.index += 1;
            let Some(surface) = entry.upgrade() else {
                continue;
            };
            if surface
                .dock_control()
                .is_some_and(|c| c.ptr_eq(&self.control))
            {
                return Some(surface.surface_control());
            }
        }
    }
}

use std::cmp::Ordering;
pub(crate) fn matches_selector_gesture(e: &KeyEventArgs, gesture: &KeyGesture) -> Option<bool> {
    if e.key != gesture.key() {
        return None;
    }
    let required = gesture.key_modifiers();
    if !e.key_modifiers.contains(required) {
        return None;
    }
    let extras = KeyModifiers::from_bits_retain(e.key_modifiers.bits() & !required.bits());
    if !extras.is_empty() && extras != KeyModifiers::SHIFT {
        return None;
    }
    Some(extras.contains(KeyModifiers::SHIFT))
}

pub(crate) fn collect_dockables(
    item: Option<Rc<dyn IDockable>>,
    dockables: &mut Vec<Rc<dyn IDockable>>,
    visited: &mut Dictionary<Option<Rc<dyn IDockable>>, ()>,
) {
    if visited.contains_key(&item) {
        return;
    }
    visited.insert(item.clone(), ());
    let Some(item) = item else { return };
    if item.clone().as_document().is_some() || item.clone().as_tool().is_some() {
        dockables.push(item.clone());
    }
    if let Some(root) = item.clone().as_root_dock() {
        add_dockables(root.hidden_dockables(), dockables, visited);
        add_dockables(root.left_pinned_dockables(), dockables, visited);
        add_dockables(root.right_pinned_dockables(), dockables, visited);
        add_dockables(root.top_pinned_dockables(), dockables, visited);
        add_dockables(root.bottom_pinned_dockables(), dockables, visited);
        if root.windows().is_some() {
            for window in root.windows().expect("source windows re-read").iter() {
                let window = window.expect("source window.Layout access");
                if window.layout().is_some() {
                    collect_dockables(
                        window.layout().map(|layout| layout as Rc<dyn IDockable>),
                        dockables,
                        visited,
                    );
                }
            }
        }
    }
    if let Some(dock) = item.clone().as_dock() {
        if dock.visible_dockables().is_some() {
            for child in dock
                .visible_dockables()
                .expect("source visible list re-read")
                .iter()
            {
                collect_dockables(child, dockables, visited);
            }
        }
    }
    if let Some(split) = item.as_split_view_dock() {
        if split.pane_dockable().is_some() {
            collect_dockables(
                Some(split.pane_dockable().expect("source pane re-read")),
                dockables,
                visited,
            );
        }
        if let Some(content) = split
            .content_dockable()
            .filter(|c| split.pane_dockable().as_ref() != Some(c))
        {
            collect_dockables(Some(content), dockables, visited);
        }
    }
}

pub(crate) fn add_dockables(
    source: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
    dockables: &mut Vec<Rc<dyn IDockable>>,
    visited: &mut Dictionary<Option<Rc<dyn IDockable>>, ()>,
) {
    if let Some(source) = source {
        for item in source.iter() {
            collect_dockables(item, dockables, visited);
        }
    }
}

pub(crate) fn to_drag_action(modifiers: KeyModifiers) -> DragAction {
    if modifiers.contains(KeyModifiers::ALT) {
        DragAction::LINK
    } else if modifiers.contains(KeyModifiers::SHIFT) {
        DragAction::MOVE
    } else if modifiers.contains(KeyModifiers::CONTROL) {
        DragAction::COPY
    } else {
        DragAction::MOVE
    }
}

#[path = "dock_control_ordinal_data.rs"]
mod ordinal_data;
// The source comparer uses simple uppercase scalars, including supplementary pairs.
// UTF-16 ordinal sorting would reverse supplementary/BMP comparisons here.
pub(crate) fn compare_selector_titles(left: &str, right: &str) -> Ordering {
    fn upper(value: char) -> u32 {
        let value = value as u32;
        match ordinal_data::ORDINAL_UPPER.binary_search_by_key(&value, |&(lower, _)| lower) {
            Ok(index) => ordinal_data::ORDINAL_UPPER[index].1,
            Err(_) => value,
        }
    }
    left.chars().map(upper).cmp(right.chars().map(upper))
}
#[cfg(test)]
#[path = "dock_control_tests.rs"]
mod tests;
