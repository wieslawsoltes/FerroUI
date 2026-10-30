// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::ToolChromeControl;
use crate::{
    automation::peers::HostWindowAutomationPeer,
    internal::{
        dock_window_state_helper::DockWindowStateHelper, event_type::EventType,
        host_window_content_cleanup::HostWindowContentCleanup, host_window_state::HostWindowState,
        window_activation_helper::WindowActivationHelper,
    },
    model_projection,
};
use ferrodock_model::{
    core::{
        DockManagerOptions, DockWindowOwnerMode, DockWindowState, IDock, IDockControl, IDockWindow,
        IDockable, IHostWindow, IHostWindowCloseState, IHostWindowState,
    },
    DockManager, DockService,
};
use ferrodock_settings::{
    DockFloatingWindowOwnerPolicy, DockLogger, DockSettings, WindowDragDockScope,
};
use ferroui_base::{
    data::core::ValueTypes,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        InputElementImpl, InputElementImplExt, PointerEventArgs, PointerPressedEventArgs,
        PointerReleasedEventArgs,
    },
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventArgs, RoutingStrategies},
    layout::LayoutableImpl,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, PixelPoint,
    Point, Rect, Ref, StyledElement, StyledElementImpl, StyledProperty, VisualImpl, WeakRef,
};
use ferroui_controls::{
    automation::peers::AutomationPeer, platform::PlatformManager, primitives::TemplatedControlImpl,
    Button, ContentControlImpl, Control, ControlImpl, TopLevelImpl, TopLevelImplExt, Window,
    WindowBaseImpl, WindowClosingEventArgs, WindowImpl, WindowImplExt, WindowState,
};
use ferroui_system::IDisposable;
use std::{
    any::TypeId,
    cell::{Cell, OnceCell, RefCell},
    rc::{Rc, Weak},
};
struct ChromeClickSubscription {
    button: WeakRef<Button>,
    subscription: Rc<dyn IDisposable>,
}
/// Original native floating host with tracked layout, close policy and window dragging.
#[repr(C)]
pub struct HostWindow {
    base: Window,
    host_window_state: OnceCell<Rc<HostWindowState>>,
    construction_options: RefCell<Option<Rc<DockManagerOptions>>>,
    chrome_grips: RefCell<Vec<Ref<Control>>>,
    chrome_grips_version: Cell<u32>,
    chrome_click_subscriptions: RefCell<Vec<ChromeClickSubscription>>,
    mouse_down: Cell<bool>,
    dragging_window: Cell<bool>,
    is_closed: Cell<bool>,
    normal_x: Cell<f64>,
    normal_y: Cell<f64>,
    normal_width: Cell<f64>,
    normal_height: Cell<f64>,
    window_drag_dock_scope: Cell<WindowDragDockScope>,
    is_tracked: Cell<bool>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
    interface_handle: RefCell<Weak<HostWindowHandle>>,
}
ferro_class!(HostWindow:Window);
ferro_class_info!(HostWindow {
    new:HostWindow::new,
    interfaces:[Rc<dyn IHostWindow> => HostWindow::as_host_window,Rc<dyn IHostWindowCloseState> => HostWindow::as_close_state],
    markup:{
        constructors:[(Rc<DockManagerOptions>)=>HostWindow::with_options],
        properties:[
            HostWindowState:Rc<dyn IHostWindowState>{get:|this:&Ref<HostWindow>|this.host_window_state()},
            IsTracked:bool{get:|this:&Ref<HostWindow>|this.is_tracked(),set:|this:&Ref<HostWindow>,v|this.set_is_tracked(v)},
            Window:Option<Rc<dyn IDockWindow>>{get:|this:&Ref<HostWindow>|this.window(),set:|this:&Ref<HostWindow>,v|this.set_window(v)}
        ],
        methods:[
            fn Present(bool)=>(|this:&Ref<HostWindow>,v|this.present(v)),fn Exit()=>(|this:&Ref<HostWindow>|this.exit()),
            fn SetPosition(f64,f64)=>(|this:&Ref<HostWindow>,x,y|this.set_position_xy(x,y)),
            fn SetSize(f64,f64)=>(|this:&Ref<HostWindow>,w,h|this.set_size(w,h)),
            fn SetWindowState(DockWindowState)=>(|this:&Ref<HostWindow>,v|this.set_dock_window_state(v)),
            fn GetWindowState()->DockWindowState=>(|this:&Ref<HostWindow>|this.get_window_state()),
            fn SetTitle(Option<String>)=>(|this:&Ref<HostWindow>,v|this.set_host_title(v)),
            fn SetLayout(Rc<dyn IDock>)=>(|this:&Ref<HostWindow>,v|this.set_layout(v)),
            fn AttachGrip(Ref<ToolChromeControl>)=>(|this:&Ref<HostWindow>,v:Ref<ToolChromeControl>|this.attach_chrome_grip(&v)),
            fn AttachGrip(Ref<Control>,String)=>(|this:&Ref<HostWindow>,v:Ref<Control>,p:String|this.attach_grip(v,&p)),
            fn DetachGrip(Ref<ToolChromeControl>)=>(|this:&Ref<HostWindow>,v:Ref<ToolChromeControl>|this.detach_chrome_grip(&v)),
            fn DetachGrip(Ref<Control>,String)=>(|this:&Ref<HostWindow>,v:Ref<Control>,p:String|this.detach_grip(&v,&p)),
            fn TryBeginMoveDrag(PointerPressedEventArgs,WindowDragDockScope)->bool=>(|this:&Ref<HostWindow>,e,s|this.try_begin_move_drag(&e,s))
        ],
        attributes:[PseudoClasses(":toolwindow",":dragging",":toolchromecontrolswindow",":documentchromecontrolswindow")]
    }
});
ferro_impl_classes!(HostWindow:VisualImpl,LayoutableImpl,InteractiveImpl,TemplatedControlImpl,ContentControlImpl,WindowBaseImpl);
impl StyledElementImpl for HostWindow {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl FerroObjectImpl for HostWindow {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let owner = this.to_ref();
        let _ = this.position_changed(move |_| owner.host_position_changed());
        let owner = this.to_ref();
        let _ = this.layout_updated(move || owner.host_layout_updated());
        let owner = this.to_ref();
        let _ = this.activated(move || owner.host_activated());
        let owner = this.to_ref();
        let _ = this.deactivated(move || owner.host_deactivated());
        let options = this
            .construction_options
            .borrow_mut()
            .take()
            .expect("source host constructor options");
        let state = HostWindowState::new(
            DockManager::with_options(DockService::new(), options),
            this.to_ref(),
            None,
        );
        assert!(
            this.host_window_state.set(state).is_ok(),
            "source single state initialization"
        );
        this.update_pseudo_classes(
            this.is_tool_window(),
            this.tool_chrome_controls_whole_window(),
            this.document_chrome_controls_whole_window(),
        );
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_tool_window_property().as_property() {
            this.update_pseudo_classes(
                change.get_new_value::<bool>(),
                this.tool_chrome_controls_whole_window(),
                this.document_chrome_controls_whole_window(),
            );
        } else if change.property()
            == Self::tool_chrome_controls_whole_window_property().as_property()
        {
            this.update_pseudo_classes(
                this.is_tool_window(),
                change.get_new_value::<bool>(),
                this.document_chrome_controls_whole_window(),
            );
        } else if change.property()
            == Self::document_chrome_controls_whole_window_property().as_property()
        {
            this.update_pseudo_classes(
                this.is_tool_window(),
                this.tool_chrome_controls_whole_window(),
                change.get_new_value::<bool>(),
            );
        } else if change.property() == Window::window_state_property().as_property()
            && this.window().is_some()
            && this.is_tracked()
        {
            this.window().expect("source window re-read").save();
        }
    }
}
impl ControlImpl for HostWindow {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        HostWindowAutomationPeer::new(this).upcast()
    }
}
impl InputElementImpl for HostWindow {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if e.handled() {
            return;
        }
        let mut index = 0;
        let mut over = false;
        let version = this.chrome_grips_version.get();
        loop {
            assert_eq!(
                version,
                this.chrome_grips_version.get(),
                "Collection was modified; enumeration operation may not execute."
            );
            let grip = this.chrome_grips.borrow().get(index).cloned();
            index += 1;
            let Some(grip) = grip else {
                break;
            };
            if grip.is_pointer_over() {
                over = true;
                break;
            }
        }
        if over
            && e.get_current_point(Some(this))
                .properties
                .is_left_button_pressed
        {
            this.move_drag(e);
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if e.handled() {
            return;
        }
        if this.dragging_window.get() {
            this.end_window_drag(e);
        }
    }
}
impl TopLevelImpl for HostWindow {
    fn on_opened(this: &Self) {
        Self::parent_on_opened(this);
        if let Some(hosts) = this
            .window()
            .and_then(|w| w.factory())
            .map(|f| f.host_windows())
        {
            let current = Self::as_host_window(this.to_ref());
            if !hosts.contains(&current) {
                hosts.add(current);
            }
        }
    }
    fn on_closed(this: &Self) {
        let closing_window = this.window();
        let closing_factory = closing_window.as_ref().and_then(|w| w.factory());
        let preserve = closing_window.as_ref().is_some_and(|w| {
            closing_factory
                .clone()
                .and_then(|f| f.as_dock_window_close_policy())
                .is_some_and(|p| p.preserve_layout_on_host_close(w.clone()))
        });
        this.is_closed.set(true);
        let content = model_projection::dockable(this.content().as_ref()).and_then(|d| d.as_dock());
        let layout = content.or_else(|| {
            closing_window
                .as_ref()
                .and_then(|w| w.layout())
                .map(|r| r as Rc<dyn IDock>)
        });
        HostWindowContentCleanup::release_models(this, layout);
        let content = this.content();
        if model_projection::dockable(content.as_ref())
            .and_then(|d| d.as_dock())
            .is_some()
        {
            if let Some(logical) = content
                .as_ref()
                .and_then(|v| ValueTypes::as_object(&**v))
                .and_then(|o| o.cast::<StyledElement>())
            {
                this.logical_children().remove(&logical);
            }
        }
        this.cancel_external_window_drag();
        Self::parent_on_closed(this);
        if preserve {
            let factory = closing_factory.expect("source preserve factory");
            factory
                .host_windows()
                .remove(&Self::as_host_window(this.to_ref()));
            this.set_is_tracked(false);
            let window = closing_window.expect("source preserve window");
            if window
                .host()
                .and_then(native_host_window)
                .is_some_and(|w| w.ptr_eq(&this.to_ref()))
            {
                window.set_host(None);
            }
            factory.on_window_closed(Some(window));
            return;
        }
        if let Some(factory) = this.window().and_then(|w| w.factory()) {
            factory
                .host_windows()
                .remove(&Self::as_host_window(this.to_ref()));
        }
        if this.window().is_some() {
            if let Some(factory) = this.window().expect("source window re-read").factory() {
                factory.close_window(Some(this.window().expect("source window re-read")));
            }
            if let Some(factory) = this.window().expect("source window re-read").factory() {
                factory.on_window_closed(this.window());
            }
            if this.is_tracked() {
                if let Some(factory) = this.window().and_then(|w| w.factory()) {
                    factory.remove_window(Some(this.window().expect("source window re-read")));
                }
            }
        }
    }
}
impl WindowImpl for HostWindow {
    fn on_closing(this: &Self, e: &WindowClosingEventArgs) {
        Self::parent_on_closing(this, e);
        if e.cancel() {
            return;
        }
        if this.window().is_some()
            && this
                .window()
                .expect("source window re-read")
                .factory()
                .is_some_and(|f| !f.on_window_closing(this.window()))
        {
            e.set_cancel(true);
            return;
        }
        if this.window().is_some() && this.is_tracked() {
            this.window().expect("source window re-read").save();
            let preserve = this
                .window()
                .expect("source window re-read")
                .factory()
                .and_then(|f| f.as_dock_window_close_policy())
                .is_some_and(|p| {
                    p.preserve_layout_on_host_close(this.window().expect("source window re-read"))
                });
            if !preserve {
                if let Some(root) = this.window().expect("source window re-read").layout() {
                    if root.close().can_execute(None) {
                        root.close().execute(None);
                    }
                }
            }
        }
    }
}
ferro_properties! {impl HostWindow {
    /// Identifies IsToolWindow.
    pub fn is_tool_window_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsToolWindow",false)}
    /// Identifies ToolChromeControlsWholeWindow.
    pub fn tool_chrome_controls_whole_window_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("ToolChromeControlsWholeWindow",false)}
    /// Identifies DocumentChromeControlsWholeWindow.
    pub fn document_chrome_controls_whole_window_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("DocumentChromeControlsWholeWindow",false)}
}}
impl HostWindow {
    fn static_constructor() {
        ValueTypes::register_interface_object::<Rc<dyn IHostWindow>>(|h| {
            h.clone()
                .boxed_interface(TypeId::of::<Ref<Window>>())
                .and_then(|v| v.downcast_ref::<Ref<Window>>().cloned())
                .map(|w| w.upcast())
        });
        ValueTypes::register_interface_object::<Rc<dyn IHostWindowCloseState>>(|h| {
            h.clone()
                .boxed_interface(TypeId::of::<Ref<Window>>())
                .and_then(|v| v.downcast_ref::<Ref<Window>>().cloned())
                .map(|w| w.upcast())
        });
    }
    /// Constructs the native source class with fresh shared manager options.
    pub fn construct() -> Self {
        Self::construct_with_options(DockManagerOptions::new())
    }
    /// Constructs the native source class retaining caller-supplied manager options.
    pub fn construct_with_options(options: Rc<DockManagerOptions>) -> Self {
        Self {
            base: Window::construct(PlatformManager::create_window()),
            host_window_state: OnceCell::new(),
            construction_options: RefCell::new(Some(options)),
            chrome_grips: RefCell::new(Vec::new()),
            chrome_grips_version: Cell::new(0),
            chrome_click_subscriptions: RefCell::new(Vec::new()),
            mouse_down: Cell::new(false),
            dragging_window: Cell::new(false),
            is_closed: Cell::new(false),
            normal_x: Cell::new(f64::NAN),
            normal_y: Cell::new(f64::NAN),
            normal_width: Cell::new(f64::NAN),
            normal_height: Cell::new(f64::NAN),
            window_drag_dock_scope: Cell::new(WindowDragDockScope::FullWindow),
            is_tracked: Cell::new(false),
            window: RefCell::new(None),
            interface_handle: RefCell::new(Weak::new()),
        }
    }
    /// Creates the original host with new manager options.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Creates the original host using the supplied shared manager options.
    pub fn with_options(options: Rc<DockManagerOptions>) -> Ref<Self> {
        instantiate(Self::construct_with_options(options))
    }
    /// Gets the original host drag engine.
    pub fn host_window_state(&self) -> Rc<dyn IHostWindowState> {
        self.state()
    }
    fn state(&self) -> Rc<HostWindowState> {
        self.host_window_state
            .get()
            .expect("source constructed host state")
            .clone()
    }
    /// Gets IsToolWindow.
    pub fn is_tool_window(&self) -> bool {
        self.get_value(Self::is_tool_window_property())
    }
    /// Sets IsToolWindow.
    pub fn set_is_tool_window(&self, value: bool) {
        self.set_value(Self::is_tool_window_property(), value);
    }
    /// Gets ToolChromeControlsWholeWindow.
    pub fn tool_chrome_controls_whole_window(&self) -> bool {
        self.get_value(Self::tool_chrome_controls_whole_window_property())
    }
    /// Sets ToolChromeControlsWholeWindow.
    pub fn set_tool_chrome_controls_whole_window(&self, value: bool) {
        self.set_value(Self::tool_chrome_controls_whole_window_property(), value);
    }
    /// Gets DocumentChromeControlsWholeWindow.
    pub fn document_chrome_controls_whole_window(&self) -> bool {
        self.get_value(Self::document_chrome_controls_whole_window_property())
    }
    /// Sets DocumentChromeControlsWholeWindow.
    pub fn set_document_chrome_controls_whole_window(&self, value: bool) {
        self.set_value(
            Self::document_chrome_controls_whole_window_property(),
            value,
        );
    }
    pub(crate) fn window_drag_dock_scope(&self) -> WindowDragDockScope {
        self.window_drag_dock_scope.get()
    }
    pub(crate) fn set_window_drag_dock_scope(&self, value: WindowDragDockScope) {
        self.window_drag_dock_scope.set(value);
    }
    /// Gets whether geometry is tracked by the original model.
    pub fn is_tracked(&self) -> bool {
        self.is_tracked.get()
    }
    /// Sets geometry tracking.
    pub fn set_is_tracked(&self, value: bool) {
        self.is_tracked.set(value);
    }
    /// Gets the original dock-window model.
    pub fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window.borrow().clone()
    }
    /// Sets the original dock-window model without adding notifications.
    pub fn set_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        let old = self.window.replace(value);
        drop(old);
    }
    fn interface_handle(host: Ref<Self>) -> Rc<HostWindowHandle> {
        let cached = host.interface_handle.borrow().upgrade();
        if let Some(cached) = cached {
            return cached;
        }
        let handle = Rc::new(HostWindowHandle(host.clone()));
        host.interface_handle.replace(Rc::downgrade(&handle));
        handle
    }
    /// Returns the original host contract for this native receiver.
    pub fn as_host_window(host: Ref<Self>) -> Rc<dyn IHostWindow> {
        Self::interface_handle(host)
    }
    /// Returns the original close-state view of this native receiver.
    pub fn as_close_state(host: Ref<Self>) -> Rc<dyn IHostWindowCloseState> {
        Self::interface_handle(host)
    }
    fn client_point_to_screen_relative_to_window(&self, point: Point) -> PixelPoint {
        let absolute = self.point_to_screen(point);
        let origin = self.point_to_screen(Point::default());
        absolute - origin
    }
    fn try_begin_window_drag(
        &self,
        e: &PointerPressedEventArgs,
        scope: WindowDragDockScope,
        owns_release: bool,
    ) -> bool {
        if self
            .window()
            .and_then(|w| w.factory())
            .map(|f| f.on_window_move_drag_begin(self.window()))
            != Some(true)
        {
            return false;
        }
        if DockSettings::bring_windows_to_front_on_drag() {
            if let Some(factory) = self.window().and_then(|w| w.factory()) {
                WindowActivationHelper::activate_all_windows(&*factory, Some(self));
            }
        }
        self.mouse_down.set(true);
        self.dragging_window.set(owns_release);
        self.set_window_drag_dock_scope(scope);
        self.state().process(
            self.client_point_to_screen_relative_to_window(e.get_position(Some(self))),
            EventType::Pressed,
        );
        self.pseudo_classes().set(":dragging", true);
        true
    }
    fn end_window_drag(&self, e: &PointerEventArgs) {
        let was_dragging = self.mouse_down.get();
        self.mouse_down.set(false);
        self.dragging_window.set(false);
        self.state().process(
            self.client_point_to_screen_relative_to_window(e.get_position(Some(self))),
            EventType::Released,
        );
        self.set_window_drag_dock_scope(WindowDragDockScope::FullWindow);
        self.pseudo_classes().set(":dragging", false);
        if was_dragging {
            if let Some(factory) = self.window().and_then(|w| w.factory()) {
                factory.on_window_move_drag_end(self.window());
            }
        }
    }
    pub(crate) fn try_begin_external_window_drag(
        &self,
        e: &PointerPressedEventArgs,
        scope: WindowDragDockScope,
    ) -> bool {
        self.try_begin_window_drag(e, scope, false)
    }
    pub(crate) fn end_external_window_drag(&self, e: &PointerEventArgs) {
        self.end_window_drag(e);
    }
    pub(crate) fn cancel_external_window_drag(&self) {
        let was_dragging = self.mouse_down.get();
        self.mouse_down.set(false);
        self.dragging_window.set(false);
        self.set_window_drag_dock_scope(WindowDragDockScope::FullWindow);
        self.state()
            .process(PixelPoint::default(), EventType::CaptureLost);
        self.pseudo_classes().set(":dragging", false);
        if was_dragging {
            if let Some(factory) = self.window().and_then(|w| w.factory()) {
                factory.on_window_move_drag_end(self.window());
            }
        }
    }
    /// Starts a native tracked move; failures cancel original external drag state before propagating.
    pub fn try_begin_move_drag(
        &self,
        e: &PointerPressedEventArgs,
        scope: WindowDragDockScope,
    ) -> bool {
        if !self.try_begin_window_drag(e, scope, true) {
            return false;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Window::begin_move_drag(self, e)
        }));
        match result {
            Ok(()) => true,
            Err(error) => {
                self.cancel_external_window_drag();
                std::panic::resume_unwind(error)
            }
        }
    }
    fn move_drag(&self, e: &PointerPressedEventArgs) {
        if self.tool_chrome_controls_whole_window() {
            self.try_begin_move_drag(e, WindowDragDockScope::FullWindow);
        }
    }
    fn host_position_changed(&self) {
        if self.window().is_some() && self.is_tracked() {
            self.capture_normal_bounds();
            self.window().expect("source window re-read").save();
            if self.mouse_down.get() {
                if let Some(factory) = self.window().expect("source window re-read").factory() {
                    factory.on_window_move_drag(self.window());
                }
                self.state().process(self.position(), EventType::Moved);
                if DockSettings::enable_window_magnetism() {
                    if let Some(windows) = self
                        .window()
                        .and_then(|w| w.factory())
                        .map(|f| f.host_windows())
                    {
                        let snap = DockSettings::window_magnet_distance();
                        let mut x = self.position().x;
                        let mut y = self.position().y;
                        let rect = Rect::new(
                            self.position().x as f64,
                            self.position().y as f64,
                            self.width(),
                            self.height(),
                        );
                        for host in windows.iter().filter_map(native_host_window) {
                            if host.ptr_eq(&self.to_ref()) || !host.is_visible() {
                                continue;
                            }
                            let other = Rect::new(
                                host.position().x as f64,
                                host.position().y as f64,
                                host.width(),
                                host.height(),
                            );
                            let vertical = rect.y < other.bottom() && rect.bottom() > other.y;
                            let horizontal = rect.x < other.right() && rect.right() > other.x;
                            if vertical {
                                if (rect.x - other.right()).abs() <= snap {
                                    x = other.right() as i32;
                                } else if (rect.right() - other.x).abs() <= snap {
                                    x = (other.x - rect.width) as i32;
                                }
                            }
                            if horizontal {
                                if (rect.y - other.bottom()).abs() <= snap {
                                    y = other.bottom() as i32;
                                } else if (rect.bottom() - other.y).abs() <= snap {
                                    y = (other.y - rect.height) as i32;
                                }
                            }
                        }
                        if x != self.position().x || y != self.position().y {
                            self.set_position(PixelPoint::new(x, y));
                        }
                    }
                }
            }
        }
    }
    fn host_layout_updated(&self) {
        if self.window().is_some() && self.is_tracked() {
            self.capture_normal_bounds();
            self.window().expect("source window re-read").save();
        }
    }
    fn host_activated(&self) {
        if self.window().is_some() {
            if let Some(factory) = self.window().expect("source window re-read").factory() {
                factory.on_window_activated(self.window());
            }
            if let Some(active) = self
                .window()
                .expect("source window re-read")
                .layout()
                .and_then(|l| l.active_dockable())
            {
                if let Some(factory) = self.window().expect("source window re-read").factory() {
                    factory.on_dockable_activated(Some(active));
                }
            }
        }
    }
    fn host_deactivated(&self) {
        if self.window().is_some() {
            if let Some(factory) = self.window().expect("source window re-read").factory() {
                factory.on_window_deactivated(self.window());
            }
            if let Some(active) = self
                .window()
                .expect("source window re-read")
                .layout()
                .and_then(|l| l.active_dockable())
            {
                if let Some(factory) = self.window().expect("source window re-read").factory() {
                    factory.on_dockable_deactivated(Some(active));
                }
            }
        }
    }
    /// Attaches the original close command to tool chrome and marks it floating.
    pub fn attach_chrome_grip(&self, chrome: &ToolChromeControl) {
        if let Some(button) = chrome.close_button() {
            let owner = self.to_ref();
            let subscription = button.add_disposable_handler_with(
                Button::click_event(),
                move |_, e| owner.chrome_close_click(e),
                RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
                false,
            );
            self.chrome_click_subscriptions
                .borrow_mut()
                .push(ChromeClickSubscription {
                    button: button.downgrade(),
                    subscription,
                });
        }
        chrome.pseudo_classes().set(":floating", true);
        self.set_is_tool_window(true);
    }
    /// Adds the supplied source chrome grip and pseudo class.
    pub fn attach_grip(&self, grip: Ref<Control>, pseudo_class: &str) {
        self.pseudo_classes().set(pseudo_class, true);
        self.chrome_grips.borrow_mut().push(grip);
        self.chrome_grips_version
            .set(self.chrome_grips_version.get().wrapping_add(1));
    }
    /// Removes the last source close subscription for the chrome's current button.
    pub fn detach_chrome_grip(&self, chrome: &ToolChromeControl) {
        if let Some(button) = chrome.close_button() {
            let index = self
                .chrome_click_subscriptions
                .borrow()
                .iter()
                .rposition(|s| s.button.upgrade().is_some_and(|b| b.ptr_eq(&button)));
            if let Some(index) = index {
                let removed = self.chrome_click_subscriptions.borrow_mut().remove(index);
                removed.subscription.dispose();
            }
        }
    }
    /// Removes the first matching source grip and clears its pseudo class.
    pub fn detach_grip(&self, grip: &Control, pseudo_class: &str) {
        self.pseudo_classes().set(pseudo_class, false);
        let index = self
            .chrome_grips
            .borrow()
            .iter()
            .position(|g| g.ptr_eq(&grip.to_ref()));
        if let Some(index) = index {
            let removed = self.chrome_grips.borrow_mut().remove(index);
            self.chrome_grips_version
                .set(self.chrome_grips_version.get().wrapping_add(1));
            drop(removed);
        }
    }
    fn update_pseudo_classes(&self, tool: bool, tool_chrome: bool, document_chrome: bool) {
        self.pseudo_classes().set(":toolwindow", tool);
        self.pseudo_classes()
            .set(":toolchromecontrolswindow", tool_chrome);
        self.pseudo_classes()
            .set(":documentchromecontrolswindow", document_chrome);
    }
    fn count_visible_tools_and_documents(&self, dockable: Option<Rc<dyn IDockable>>) -> i32 {
        let Some(dockable) = dockable else {
            return 0;
        };
        if dockable.clone().as_tool().is_some() || dockable.clone().as_document().is_some() {
            return 1;
        }
        if let Some(dock) = dockable.as_dock() {
            if let Some(items) = dock.visible_dockables() {
                let mut count = 0_i32;
                for item in items.iter() {
                    count = count
                        .checked_add(self.count_visible_tools_and_documents(item))
                        .expect("source checked Sum overflow");
                }
                return count;
            }
        }
        0
    }
    fn chrome_close_click(&self, _e: &RoutedEventArgs) {
        if self.count_visible_tools_and_documents(
            model_projection::root(self.data_context().as_ref()).map(|r| r as Rc<dyn IDockable>),
        ) <= 1
        {
            self.exit();
        }
    }
    fn resolve_owner_window(&self, window: Rc<dyn IDockWindow>) -> (Option<Ref<Window>>, bool) {
        let mut parent = window.parent_window();
        let uses_default = window.owner_mode() == DockWindowOwnerMode::Default;
        let mode = if window.owner_mode() == DockWindowOwnerMode::Default {
            DockSettings::default_floating_window_owner_mode()
        } else {
            window.owner_mode()
        };
        if parent.as_ref() == Some(&window) {
            parent = None;
        }
        if mode == DockWindowOwnerMode::None {
            return (None, false);
        }
        if uses_default
            && DockSettings::floating_window_owner_policy()
                == DockFloatingWindowOwnerPolicy::NeverOwned
        {
            return (None, false);
        }
        if mode == DockWindowOwnerMode::ParentWindow || mode == DockWindowOwnerMode::DockableWindow
        {
            if let Some(owner) = parent.and_then(|p| p.host()).and_then(native_window) {
                return (Some(owner), false);
            }
            return (
                if uses_default {
                    self.resolve_fallback_owner_window(window)
                } else {
                    None
                },
                false,
            );
        }
        if mode == DockWindowOwnerMode::RootWindow {
            if let Some(root_window) = window
                .owner()
                .and_then(|o| o.as_root_dock())
                .and_then(|r| r.window())
            {
                if !Rc::ptr_eq(&root_window, &window) {
                    return (root_window.host().and_then(native_window), false);
                }
            }
            if let Some(factory) = window.factory() {
                if let Some(owner) = window.owner() {
                    if let Some(root_window) = factory
                        .find_root(Some(owner), None)
                        .and_then(|r| r.window())
                        .filter(|w| !Rc::ptr_eq(w, &window))
                    {
                        return (root_window.host().and_then(native_window), false);
                    }
                }
            }
            return (None, false);
        }
        if mode == DockWindowOwnerMode::Default {
            if let Some(owner) = parent.and_then(|p| p.host()).and_then(native_window) {
                return (Some(owner), false);
            }
        }
        if mode == DockWindowOwnerMode::Default
            && DockSettings::should_use_owner_for_floating_windows()
        {
            let owner = window
                .layout()
                .and_then(|l| l.factory())
                .and_then(|f| f.dock_controls().iter().next())
                .and_then(native_control)
                .and_then(|c| c.visual_root())
                .and_then(|v| v.cast::<Window>());
            if owner.is_some() {
                return (owner, true);
            }
        }
        (None, false)
    }
    fn resolve_fallback_owner_window(&self, window: Rc<dyn IDockWindow>) -> Option<Ref<Window>> {
        if let Some(owner) = window
            .parent_window()
            .and_then(|p| p.host())
            .and_then(native_window)
        {
            return Some(owner);
        }
        if let Some(root_window) = window
            .owner()
            .and_then(|o| o.as_root_dock())
            .and_then(|r| r.window())
        {
            if !Rc::ptr_eq(&root_window, &window) {
                return root_window.host().and_then(native_window);
            }
        }
        if let Some(factory) = window.factory() {
            if let Some(owner) = window.owner() {
                if let Some(root_window) = factory
                    .find_root(Some(owner), None)
                    .and_then(|r| r.window())
                    .filter(|w| !Rc::ptr_eq(w, &window))
                {
                    return root_window.host().and_then(native_window);
                }
            }
        }
        window
            .layout()
            .and_then(|l| l.factory())
            .and_then(|f| f.dock_controls().iter().next())
            .and_then(native_control)
            .and_then(|c| c.visual_root())
            .and_then(|v| v.cast::<Window>())
    }
    fn apply_presentation_options(&self, window: Rc<dyn IDockWindow>) {
        if let Some(show) = window.show_in_taskbar() {
            self.set_show_in_taskbar(show);
        }
    }
    /// Presents using the original owner resolution, taskbar options and modal fallback.
    pub fn present(&self, is_dialog: bool) {
        if is_dialog {
            if !self.is_visible() {
                if let Some(window) = self.window() {
                    if let Some(factory) = window.factory() {
                        factory.on_window_opened(Some(window.clone()));
                    }
                    self.apply_presentation_options(window);
                }
                let mut owner = self.window().and_then(|w| self.resolve_owner_window(w).0);
                if owner.is_none() {
                    if let Some(window) = self.window() {
                        let mode = if window.owner_mode() == DockWindowOwnerMode::Default {
                            DockSettings::default_floating_window_owner_mode()
                        } else {
                            window.owner_mode()
                        };
                        let disallow = mode == DockWindowOwnerMode::None
                            || DockSettings::floating_window_owner_policy()
                                == DockFloatingWindowOwnerPolicy::NeverOwned;
                        if !disallow {
                            owner = self.resolve_fallback_owner_window(window);
                        }
                    }
                }
                if let Some(owner) = owner {
                    drop(self.show_dialog(&owner));
                } else {
                    DockLogger::log_debug(
                        "Windowing",
                        "Modal window has no owner; presenting non-modally.",
                    );
                    self.show();
                }
            }
        } else if !self.is_visible() {
            if let Some(window) = self.window() {
                if let Some(factory) = window.factory() {
                    factory.on_window_opened(Some(window.clone()));
                }
                self.apply_presentation_options(window);
            }
            let (owner, copy) = self
                .window()
                .map_or((None, false), |w| self.resolve_owner_window(w));
            if let Some(owner) = owner {
                if copy {
                    self.set_title(owner.title());
                    self.set_icon(owner.icon());
                }
                self.show_with_owner(&owner);
            } else {
                self.show();
            }
        }
    }
    /// Runs the original cancelable native close pipeline once.
    pub fn exit(&self) {
        if let Some(window) = self.window().filter(|w| w.factory().is_none()) {
            if !window.on_close() {
                return;
            }
        }
        self.close();
    }
    /// Applies source double coordinates while retaining their original normal-bound values.
    pub fn set_position_xy(&self, x: f64, y: f64) {
        if !x.is_nan() && !y.is_nan() {
            self.set_position(PixelPoint::new(x as i32, y as i32));
            self.normal_x.set(x);
            self.normal_y.set(y);
        }
    }
    /// Gets normal bounds when available for a non-normal state, otherwise current native position.
    pub fn get_position(&self) -> (f64, f64) {
        if self.window_state() != WindowState::Normal {
            if let Some((x, y, _, _)) = self.try_get_normal_bounds() {
                return (x, y);
            }
        }
        (self.position().x as f64, self.position().y as f64)
    }
    /// Applies each non-NaN source size dimension independently.
    pub fn set_size(&self, width: f64, height: f64) {
        if !width.is_nan() {
            self.set_width(width);
            self.normal_width.set(width);
        }
        if !height.is_nan() {
            self.set_height(height);
            self.normal_height.set(height);
        }
    }
    /// Gets retained normal size when valid for a non-normal state, otherwise current dimensions.
    pub fn get_size(&self) -> (f64, f64) {
        if self.window_state() != WindowState::Normal {
            if let Some((_, _, w, h)) = self.try_get_normal_bounds() {
                return (w, h);
            }
        }
        (self.width(), self.height())
    }
    /// Applies the original dock/native window-state conversion.
    pub fn set_dock_window_state(&self, state: DockWindowState) {
        self.set_window_state(DockWindowStateHelper::to_ferro_window_state(state));
    }
    /// Gets the original dock window state.
    pub fn get_window_state(&self) -> DockWindowState {
        DockWindowStateHelper::to_dock_window_state(self.window_state())
    }
    /// Sets a nonempty explicit title, preserving the current title/binding for None or empty input.
    pub fn set_host_title(&self, title: Option<String>) {
        if title.as_ref().is_some_and(|s| !s.is_empty()) {
            self.set_title(title);
        }
    }
    /// Sets the source layout DataContext and updates the current tool-window flag.
    pub fn set_layout(&self, layout: Rc<dyn IDock>) {
        self.set_current_value(
            Self::is_tool_window_property(),
            Self::resolve_hosted_dock(layout.clone()).is_some_and(|d| d.as_tool_dock().is_some()),
        );
        self.set_data_context(Some(model_projection::to_dockable_value(layout)));
    }
    fn resolve_hosted_dock(layout: Rc<dyn IDock>) -> Option<Rc<dyn IDock>> {
        if let Some(root) = layout.clone().as_root_dock() {
            return root
                .active_dockable()
                .and_then(|d| d.as_dock())
                .or_else(|| root.default_dockable().and_then(|d| d.as_dock()))
                .or_else(|| {
                    root.visible_dockables()
                        .and_then(|l| l.iter().find_map(|d| d.and_then(|item| item.as_dock())))
                });
        }
        Some(layout)
    }
    fn capture_normal_bounds(&self) {
        if self.window_state() != WindowState::Normal {
            return;
        }
        if self.width().is_nan() || self.height().is_nan() {
            return;
        }
        self.normal_x.set(self.position().x as f64);
        self.normal_y.set(self.position().y as f64);
        self.normal_width.set(self.width());
        self.normal_height.set(self.height());
    }
    fn try_get_normal_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let (x, y, w, h) = (
            self.normal_x.get(),
            self.normal_y.get(),
            self.normal_width.get(),
            self.normal_height.get(),
        );
        if !x.is_nan() && !y.is_nan() && !w.is_nan() && !h.is_nan() && w > 0.0 && h > 0.0 {
            Some((x, y, w, h))
        } else {
            None
        }
    }
    fn set_active_host(&self) {
        if self.window_state() == WindowState::Minimized {
            self.set_window_state(WindowState::Normal);
        }
        self.activate();
    }
}
struct HostWindowHandle(Ref<HostWindow>);
impl IHostWindowCloseState for HostWindowHandle {
    fn boxed_interface(self: Rc<Self>, target: TypeId) -> Option<ferrodock_model::BoxedValue> {
        <Self as IHostWindow>::boxed_interface(self, target)
    }
    fn is_closed(&self) -> bool {
        self.0.is_closed.get()
    }
}
impl IHostWindow for HostWindowHandle {
    fn runtime_type_name(&self) -> &'static str {
        self.0.get_type().name()
    }

    fn boxed_interface(self: Rc<Self>, target: TypeId) -> Option<ferrodock_model::BoxedValue> {
        if target == TypeId::of::<Ref<HostWindow>>() {
            Some(Rc::new(self.0.clone()))
        } else if target == TypeId::of::<Ref<Window>>() {
            Some(Rc::new(self.0.clone().upcast::<Window>()))
        } else if target == TypeId::of::<Ref<Control>>() {
            Some(Rc::new(self.0.clone().upcast::<Control>()))
        } else {
            None
        }
    }
    fn as_host_window_close_state(&self) -> Option<&dyn IHostWindowCloseState> {
        Some(self)
    }
    fn host_window_state(&self) -> Option<Rc<dyn IHostWindowState>> {
        Some(self.0.host_window_state())
    }
    fn is_tracked(&self) -> bool {
        self.0.is_tracked()
    }
    fn set_is_tracked(&self, v: bool) {
        self.0.set_is_tracked(v);
    }
    fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.0.window()
    }
    fn set_window(&self, v: Option<Rc<dyn IDockWindow>>) {
        self.0.set_window(v);
    }
    fn present(&self, v: bool) {
        self.0.present(v);
    }
    fn exit(&self) {
        self.0.exit();
    }
    fn set_position(&self, x: f64, y: f64) {
        self.0.set_position_xy(x, y);
    }
    fn get_position(&self) -> (f64, f64) {
        self.0.get_position()
    }
    fn set_size(&self, w: f64, h: f64) {
        self.0.set_size(w, h);
    }
    fn get_size(&self) -> (f64, f64) {
        self.0.get_size()
    }
    fn set_window_state(&self, v: DockWindowState) {
        self.0.set_dock_window_state(v);
    }
    fn get_window_state(&self) -> DockWindowState {
        self.0.get_window_state()
    }
    fn set_title(&self, v: Option<String>) {
        self.0.set_host_title(v);
    }
    fn set_layout(&self, v: Rc<dyn IDock>) {
        self.0.set_layout(v);
    }
    fn set_active(&self) {
        self.0.set_active_host();
    }
}
fn native_window(host: Rc<dyn IHostWindow>) -> Option<Ref<Window>> {
    host.boxed_interface(TypeId::of::<Ref<Window>>())?
        .downcast_ref::<Ref<Window>>()
        .cloned()
}
fn native_host_window(host: Rc<dyn IHostWindow>) -> Option<Ref<HostWindow>> {
    host.boxed_interface(TypeId::of::<Ref<HostWindow>>())?
        .downcast_ref::<Ref<HostWindow>>()
        .cloned()
}
fn native_control(control: Rc<dyn IDockControl>) -> Option<Ref<Control>> {
    control
        .boxed_interface(TypeId::of::<Ref<Control>>())?
        .downcast_ref::<Ref<Control>>()
        .cloned()
}
