//! Port of `HeadlessWindowImpl.cs`: the platform implementation of the
//! windows and popups of the headless platform.

use crate::ferro_headless_platform::{FerroHeadlessPlatform, FerroHeadlessPlatformOptions};
use crate::headless_platform_stubs::HeadlessScreensStub;
use crate::headless_window_surface::HeadlessWindowSurface;
use crate::i_headless_window::IHeadlessWindow;
use ferroui_base::input::platform::IClipboard;
use ferroui_base::input::raw::{
    IDragDropDevice, IRawInputEventArgs, RawDragEvent, RawDragEventType, RawKeyEventArgs, RawKeyEventType,
    RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType, RawTextInputEventArgs, RawTouchEventArgs,
};
use ferroui_base::input::{
    DragDropEffects, IDataTransfer, IInputDevice, IInputRoot, IKeyboardDevice, Key, KeyDeviceType, MouseButton,
    MouseDevice, PhysicalKey, Pointer, PointerPressedEventArgs, PointerType, RawInputModifiers, TouchDevice,
};
use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Point, Rect, Size, Thickness, Vector};
use ferroui_controls::platform::{
    IPlatformHandle, IPopupImpl, IScreenImpl, ITopLevelImpl, IWindowBaseImpl, IWindowIconImpl, IWindowImpl,
    PlatformHandle, PlatformRequestedDrawnDecoration, PlatformThemeVariant,
};
use ferroui_controls::primitives::popup_positioning::{
    IPopupPositioner, ManagedPopupPositioner, ManagedPopupPositionerPopupImplHelper,
};
use ferroui_controls::{
    AcrylicPlatformCompensationLevels, WindowCloseReason, WindowDecorations, WindowEdge, WindowResizeReason,
    WindowState, WindowTransparencyLevel,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Instant;

thread_local! {
    /// `_nextGlobalZOrder`: per thread, as the windows of a headless
    /// application live on its one UI thread.
    static NEXT_GLOBAL_Z_ORDER: Cell<i32> = const { Cell::new(1) };
}

fn next_global_z_order() -> i32 {
    NEXT_GLOBAL_Z_ORDER.with(|next| {
        let value = next.get();
        next.set(value + 1);
        value
    })
}

type Callback<F> = RefCell<Option<Rc<F>>>;

fn get<F: ?Sized>(callback: &Callback<F>) -> Option<Rc<F>> {
    callback.borrow().clone()
}

fn set<F: ?Sized>(callback: &Callback<F>, value: Option<Rc<F>>) {
    let old = callback.replace(value);
    drop(old);
}

pub(crate) struct HeadlessWindowImpl {
    this: Weak<HeadlessWindowImpl>,
    keyboard: Rc<dyn IKeyboardDevice>,
    screen: Rc<dyn IScreenImpl>,
    st: Instant,
    touch_device: Rc<TouchDevice>,
    // What a frame uses of the window (the size and the scaling a framebuffer is created with,
    // and the last rendered frame under its lock): upstream keeps it in the window, which is its
    // own surface; here it is an object the thread that renders shares with the window.
    surface: Arc<HeadlessWindowSurface>,
    options: FerroHeadlessPlatformOptions,
    popup_parent: Option<Rc<HeadlessWindowImpl>>,
    popup_positioner: RefCell<Option<Rc<dyn IPopupPositioner>>>,
    is_popup: bool,

    z_order: Cell<i32>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    mouse_device: Rc<MouseDevice>,
    position: Cell<PixelPoint>,
    handle: Rc<dyn IPlatformHandle>,
    window_state: Cell<WindowState>,
    transparency_level: Cell<WindowTransparencyLevel>,

    input: Callback<dyn Fn(Rc<dyn IRawInputEventArgs>)>,
    paint: Callback<dyn Fn(Rect)>,
    resized: Callback<dyn Fn(Size, WindowResizeReason)>,
    scaling_changed: Callback<dyn Fn(f64)>,
    closed: Callback<dyn Fn()>,
    position_changed: Callback<dyn Fn(PixelPoint)>,
    deactivated: Callback<dyn Fn()>,
    activated: Callback<dyn Fn()>,
    window_state_changed: Callback<dyn Fn(WindowState)>,
    closing: Callback<dyn Fn(WindowCloseReason) -> bool>,
    transparency_level_changed: Callback<dyn Fn(WindowTransparencyLevel)>,
    got_input_when_disabled: Callback<dyn Fn()>,
    extend_client_area_to_decorations_changed: Callback<dyn Fn(bool)>,
    lost_focus: Callback<dyn Fn()>,
}

impl HeadlessWindowImpl {
    pub(crate) fn new(options: &FerroHeadlessPlatformOptions) -> Rc<HeadlessWindowImpl> {
        Self::create(options, None)
    }

    /// The private constructor of the original: a popup of `popup_parent`.
    fn new_popup(popup_parent: Rc<HeadlessWindowImpl>) -> Rc<HeadlessWindowImpl> {
        let options = popup_parent.options.clone();
        let popup = Self::create(&options, Some(popup_parent.clone()));

        let weak = Rc::downgrade(&popup);
        let parent: Rc<dyn ITopLevelImpl> = popup_parent;
        let helper = ManagedPopupPositionerPopupImplHelper::new(
            parent,
            Rc::new(move |position, size: Size, scaling| {
                if let Some(popup) = weak.upgrade() {
                    popup.popup_move_resize(position, size, scaling);
                }
            }),
        );
        let positioner: Rc<dyn IPopupPositioner> = Rc::new(ManagedPopupPositioner::new(Rc::new(helper)));
        *popup.popup_positioner.borrow_mut() = Some(positioner);
        popup
    }

    fn create(
        options: &FerroHeadlessPlatformOptions,
        popup_parent: Option<Rc<HeadlessWindowImpl>>,
    ) -> Rc<HeadlessWindowImpl> {
        let keyboard = FerroLocator::current().get_required_service::<dyn IKeyboardDevice>();
        let screen: Rc<dyn IScreenImpl> = HeadlessScreensStub::new();
        let mouse_device = if options.use_shared_mouse_device == Some(true) {
            MouseDevice::primary()
        } else {
            MouseDevice::with_pointer(Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true))
        };
        let is_popup = popup_parent.is_some();

        Rc::new_cyclic(|this| HeadlessWindowImpl {
            this: this.clone(),
            keyboard,
            screen,
            st: Instant::now(),
            touch_device: TouchDevice::new(),
            surface: HeadlessWindowSurface::new(options.frame_buffer_format, Size::new(1024.0, 768.0), 1.0),
            options: options.clone(),
            popup_parent,
            popup_positioner: RefCell::new(None),
            is_popup,
            z_order: Cell::new(0),
            input_root: RefCell::new(None),
            mouse_device,
            position: Cell::new(PixelPoint::default()),
            handle: Rc::new(PlatformHandle::new(0, Some("STUB"))),
            window_state: Cell::new(WindowState::Normal),
            transparency_level: Cell::new(WindowTransparencyLevel::transparent()),
            input: RefCell::new(None),
            paint: RefCell::new(None),
            resized: RefCell::new(None),
            scaling_changed: RefCell::new(None),
            closed: RefCell::new(None),
            position_changed: RefCell::new(None),
            deactivated: RefCell::new(None),
            activated: RefCell::new(None),
            window_state_changed: RefCell::new(None),
            closing: RefCell::new(None),
            transparency_level_changed: RefCell::new(None),
            got_input_when_disabled: RefCell::new(None),
            extend_client_area_to_decorations_changed: RefCell::new(None),
            lost_focus: RefCell::new(None),
        })
    }

    fn rc(&self) -> Rc<HeadlessWindowImpl> {
        self.this.upgrade().expect("the window implementation is alive while it is used")
    }

    #[allow(dead_code)] // public property of the original, read by its own class only
    pub(crate) fn is_popup(&self) -> bool {
        self.is_popup
    }

    /// The parent of a popup.
    #[allow(dead_code)] // as upstream, the field is kept and not read
    pub(crate) fn popup_parent(&self) -> Option<&Rc<HeadlessWindowImpl>> {
        self.popup_parent.as_ref()
    }

    pub(crate) fn z_order(&self) -> i32 {
        self.z_order.get()
    }

    pub(crate) fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    #[allow(dead_code)] // public property of the original; the tests of the crate read it
    pub(crate) fn mouse_device(&self) -> &Rc<MouseDevice> {
        &self.mouse_device
    }

    fn popup_move_resize(&self, position: PixelPoint, size: Size, _scaling: f64) {
        self.position.set(position);
        if let Some(position_changed) = get(&self.position_changed) {
            position_changed(position);
        }
        self.do_resize(size, WindowResizeReason::Unspecified);
    }

    fn do_resize(&self, client_size: Size, reason: WindowResizeReason) {
        // Uncomment this check and experience a weird bug in layout engine
        if self.surface.client_size() != client_size {
            self.surface.set_client_size(client_size);
            if let Some(resized) = get(&self.resized) {
                resized(client_size, reason);
            }
        }
    }

    /// Posts the activated notification at input priority.
    fn post_activated(&self) {
        let weak = self.this.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(activated) = weak.upgrade().and_then(|this| get(&this.activated)) {
                    activated();
                }
            },
            DispatcherPriority::INPUT,
        );
    }

    fn timestamp(&self) -> u64 {
        self.st.elapsed().as_millis() as u64
    }

    fn invoke_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        if let Some(input) = get(&self.input) {
            input(args);
        }
    }

    /// `InputRoot!`.
    ///
    /// # Panics
    /// Panics if the input root is not set (the null reference of the original).
    fn required_input_root(&self) -> Rc<dyn IInputRoot> {
        match self.input_root() {
            Some(input_root) => input_root,
            None => panic!("The input root of the headless window is not set."),
        }
    }

    fn keyboard_device(&self) -> Rc<dyn IInputDevice> {
        let device: Rc<dyn IInputDevice> = self.keyboard.clone();
        device
    }

    fn mouse_input_device(&self) -> Rc<dyn IInputDevice> {
        let device: Rc<dyn IInputDevice> = self.mouse_device.clone();
        device
    }
}

impl IOptionalFeatureProvider for HeadlessWindowImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IClipboard>() {
            let clipboard = FerroLocator::current().get_required_service::<dyn IClipboard>();
            return Some(Rc::new(clipboard));
        }

        if feature_type == TypeId::of::<dyn IScreenImpl>() {
            return Some(Rc::new(self.screen.clone()));
        }

        None
    }
}

impl IDisposable for HeadlessWindowImpl {
    fn dispose(&self) {
        if let Some(closed) = get(&self.closed) {
            closed();
        }
        self.touch_device.dispose();
        self.surface.dispose_last_rendered_frame();
    }
}

impl ITopLevelImpl for HeadlessWindowImpl {
    fn desktop_scaling(&self) -> f64 {
        self.surface.render_scaling()
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        Some(self.handle.clone())
    }

    fn client_size(&self) -> Size {
        self.surface.client_size()
    }

    fn render_scaling(&self) -> f64 {
        self.surface.render_scaling()
    }

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        let surface: Arc<dyn IPlatformRenderSurface> = self.surface.clone();
        vec![surface]
    }

    // As upstream, whose `Surfaces` is set once and never cleared, the surface is handed out
    // for as long as someone asks: a frame rendered after the window is disposed is kept by
    // the surface and released with it.
    fn render_surfaces(&self) -> Arc<dyn Fn() -> Vec<Arc<dyn IPlatformRenderSurface>> + Send + Sync> {
        let surface = self.surface.clone();
        Arc::new(move || {
            let surface: Arc<dyn IPlatformRenderSurface> = surface.clone();
            vec![surface]
        })
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        FerroHeadlessPlatform::compositor()
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        get(&self.input)
    }

    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
        set(&self.input, value)
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        get(&self.paint)
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        set(&self.paint, value)
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        get(&self.resized)
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        set(&self.resized, value)
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        get(&self.scaling_changed)
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        set(&self.scaling_changed, value)
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        get(&self.transparency_level_changed)
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        set(&self.transparency_level_changed, value)
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        *self.input_root.borrow_mut() = Some(input_root);
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        (point - self.position.get()).to_point(self.surface.render_scaling())
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::from_point(point, self.surface.render_scaling()) + self.position.get()
    }

    fn set_cursor(&self, _cursor: Option<Rc<dyn ICursorImpl>>) {}

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.closed)
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.closed, value)
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.lost_focus)
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.lost_focus, value)
    }

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        if self.options.overlay_popups {
            None
        } else {
            let popup: Rc<dyn IPopupImpl> = HeadlessWindowImpl::new_popup(self.rc());
            Some(popup)
        }
    }

    fn set_transparency_level_hint(&self, transparency_levels: &[WindowTransparencyLevel]) {
        for item in transparency_levels {
            if *item == WindowTransparencyLevel::transparent() {
                self.transparency_level.set(*item);
                return;
            }
        }

        self.transparency_level.set(WindowTransparencyLevel::none());
    }

    fn transparency_level(&self) -> WindowTransparencyLevel {
        self.transparency_level.get()
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        AcrylicPlatformCompensationLevels::new(1.0, 1.0, 1.0)
    }

    fn set_frame_theme_variant(&self, _theme_variant: Option<PlatformThemeVariant>) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    // The class implements the window and the popup interface, whatever it was created as.
    fn as_window_base_impl(&self) -> Option<&dyn IWindowBaseImpl> {
        Some(self)
    }

    fn as_window_impl(&self) -> Option<&dyn IWindowImpl> {
        Some(self)
    }

    fn as_popup_impl(&self) -> Option<&dyn IPopupImpl> {
        Some(self)
    }
}

impl IWindowBaseImpl for HeadlessWindowImpl {
    fn frame_size(&self) -> Option<Size> {
        None
    }

    fn show(&self, activate: bool, _is_dialog: bool) {
        if activate {
            self.z_order.set(next_global_z_order());
            self.post_activated();
        }
    }

    fn hide(&self) {
        let weak = self.this.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(deactivated) = weak.upgrade().and_then(|this| get(&this.deactivated)) {
                    deactivated();
                }
            },
            DispatcherPriority::INPUT,
        );
    }

    fn position(&self) -> PixelPoint {
        self.position.get()
    }

    fn position_changed(&self) -> Option<Rc<dyn Fn(PixelPoint)>> {
        get(&self.position_changed)
    }

    fn set_position_changed(&self, value: Option<Rc<dyn Fn(PixelPoint)>>) {
        set(&self.position_changed, value)
    }

    fn activate(&self) {
        self.z_order.set(next_global_z_order());
        self.post_activated();
    }

    fn deactivated(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.deactivated)
    }

    fn set_deactivated(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.deactivated, value)
    }

    fn activated(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.activated)
    }

    fn set_activated(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.activated, value)
    }

    fn max_auto_size_hint(&self) -> Size {
        Size::new(1920.0, 1080.0)
    }

    fn set_topmost(&self, _value: bool) {}
}

impl IWindowImpl for HeadlessWindowImpl {
    fn window_state(&self) -> WindowState {
        self.window_state.get()
    }

    fn set_window_state(&self, value: WindowState) {
        self.window_state.set(value);
    }

    fn window_state_getter_is_usable(&self) -> bool {
        false
    }

    fn window_state_changed(&self) -> Option<Rc<dyn Fn(WindowState)>> {
        get(&self.window_state_changed)
    }

    fn set_window_state_changed(&self, value: Option<Rc<dyn Fn(WindowState)>>) {
        set(&self.window_state_changed, value)
    }

    fn set_title(&self, _title: Option<&str>) {}

    fn set_parent(&self, _parent: Option<Rc<dyn IWindowImpl>>) {}

    fn set_enabled(&self, _enable: bool) {}

    fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.got_input_when_disabled)
    }

    fn set_got_input_when_disabled(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.got_input_when_disabled, value)
    }

    fn set_window_decorations(&self, _enabled: WindowDecorations) {}

    fn set_icon(&self, _icon: Option<Rc<dyn IWindowIconImpl>>) {}

    fn show_taskbar_icon(&self, _value: bool) {}

    fn can_resize(&self, _value: bool) {}

    fn set_can_minimize(&self, _value: bool) {}

    fn set_can_maximize(&self, _value: bool) {}

    fn closing(&self) -> Option<Rc<dyn Fn(WindowCloseReason) -> bool>> {
        get(&self.closing)
    }

    fn set_closing(&self, value: Option<Rc<dyn Fn(WindowCloseReason) -> bool>>) {
        set(&self.closing, value)
    }

    fn is_client_area_extended_to_decorations(&self) -> bool {
        false
    }

    fn extend_client_area_to_decorations_changed(&self) -> Option<Rc<dyn Fn(bool)>> {
        get(&self.extend_client_area_to_decorations_changed)
    }

    fn set_extend_client_area_to_decorations_changed(&self, value: Option<Rc<dyn Fn(bool)>>) {
        set(&self.extend_client_area_to_decorations_changed, value)
    }

    fn needs_managed_decorations(&self) -> bool {
        false
    }

    fn requested_drawn_decorations(&self) -> PlatformRequestedDrawnDecoration {
        PlatformRequestedDrawnDecoration::empty()
    }

    fn extended_margins(&self) -> Thickness {
        Thickness::default()
    }

    fn off_screen_margin(&self) -> Thickness {
        Thickness::default()
    }

    fn begin_move_drag(&self, _e: &PointerPressedEventArgs) {}

    fn begin_resize_drag(&self, _edge: WindowEdge, _e: &PointerPressedEventArgs) {}

    fn resize(&self, client_size: Size, reason: WindowResizeReason) {
        if self.surface.client_size() == client_size {
            return;
        }

        // Emulate X11 behavior here
        if self.is_popup {
            self.do_resize(client_size, reason);
        } else {
            let weak = self.this.clone();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.do_resize(client_size, reason);
                    }
                },
                DispatcherPriority::SEND,
            );
        }
    }

    fn move_(&self, point: PixelPoint) {
        self.position.set(point);
        if let Some(position_changed) = get(&self.position_changed) {
            position_changed(point);
        }
    }

    fn set_min_max_size(&self, _min_size: Size, _max_size: Size) {}

    fn set_extend_client_area_to_decorations_hint(&self, _extend_into_client_area_hint: bool) {}

    fn set_extend_client_area_title_bar_height_hint(&self, _title_bar_height: f64) {}
}

impl IPopupImpl for HeadlessWindowImpl {
    fn popup_positioner(&self) -> Option<Rc<dyn IPopupPositioner>> {
        self.popup_positioner.borrow().clone()
    }

    fn set_window_manager_add_shadow_hint(&self, _enabled: bool) {}

    fn take_focus(&self) {}

    fn set_hit_test_visible(&self, _is_hit_test_visible: bool) {}
}

impl IHeadlessWindow for HeadlessWindowImpl {
    fn get_last_rendered_frame(&self) -> Option<WriteableBitmap> {
        self.surface.get_last_rendered_frame()
    }

    fn key_press(&self, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<&str>) {
        self.invoke_input(Rc::new(RawKeyEventArgs::new(
            self.keyboard_device(),
            self.timestamp(),
            self.required_input_root(),
            RawKeyEventType::KeyDown,
            key,
            modifiers,
            physical_key,
            key_symbol.map(str::to_owned),
            KeyDeviceType::Keyboard,
        )));
    }

    fn key_release(&self, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<&str>) {
        self.invoke_input(Rc::new(RawKeyEventArgs::new(
            self.keyboard_device(),
            self.timestamp(),
            self.required_input_root(),
            RawKeyEventType::KeyUp,
            key,
            modifiers,
            physical_key,
            key_symbol.map(str::to_owned),
            KeyDeviceType::Keyboard,
        )));
    }

    fn text_input(&self, text: &str) {
        let Some(input_root) = self.input_root() else {
            return;
        };

        self.invoke_input(Rc::new(RawTextInputEventArgs::new(self.keyboard_device(), 0, input_root, text)));
    }

    fn mouse_down(&self, point: Point, button: MouseButton, modifiers: RawInputModifiers) {
        self.invoke_input(Rc::new(RawPointerEventArgs::new(
            self.mouse_input_device(),
            self.timestamp(),
            self.required_input_root(),
            match button {
                MouseButton::Left => RawPointerEventType::LeftButtonDown,
                MouseButton::Right => RawPointerEventType::RightButtonDown,
                MouseButton::Middle => RawPointerEventType::MiddleButtonDown,
                MouseButton::XButton1 => RawPointerEventType::XButton1Down,
                MouseButton::XButton2 => RawPointerEventType::XButton2Down,
                _ => RawPointerEventType::Move,
            },
            point,
            modifiers,
        )));
    }

    fn mouse_move(&self, point: Point, modifiers: RawInputModifiers) {
        self.invoke_input(Rc::new(RawPointerEventArgs::new(
            self.mouse_input_device(),
            self.timestamp(),
            self.required_input_root(),
            RawPointerEventType::Move,
            point,
            modifiers,
        )));
    }

    fn mouse_up(&self, point: Point, button: MouseButton, modifiers: RawInputModifiers) {
        self.invoke_input(Rc::new(RawPointerEventArgs::new(
            self.mouse_input_device(),
            self.timestamp(),
            self.required_input_root(),
            match button {
                MouseButton::Left => RawPointerEventType::LeftButtonUp,
                MouseButton::Right => RawPointerEventType::RightButtonUp,
                MouseButton::Middle => RawPointerEventType::MiddleButtonUp,
                MouseButton::XButton1 => RawPointerEventType::XButton1Up,
                MouseButton::XButton2 => RawPointerEventType::XButton2Up,
                _ => RawPointerEventType::Move,
            },
            point,
            modifiers,
        )));
    }

    fn mouse_wheel(&self, point: Point, delta: Vector, modifiers: RawInputModifiers) {
        self.invoke_input(Rc::new(RawMouseWheelEventArgs::new(
            self.mouse_input_device(),
            self.timestamp(),
            self.required_input_root(),
            point,
            delta,
            modifiers,
        )));
    }

    fn touch(&self, point: Point, touch_point_id: i64, type_: RawPointerEventType, modifiers: RawInputModifiers) {
        let device: Rc<dyn IInputDevice> = self.touch_device.clone();
        self.invoke_input(Rc::new(RawTouchEventArgs::new(
            device,
            self.timestamp(),
            self.required_input_root(),
            type_,
            point,
            modifiers,
            touch_point_id,
        )));
    }

    fn drag_drop(
        &self,
        point: Point,
        type_: RawDragEventType,
        data: Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: RawInputModifiers,
    ) {
        let device = FerroLocator::current().get_required_service::<dyn IDragDropDevice>();
        self.invoke_input(Rc::new(RawDragEvent::new(
            device,
            type_,
            self.required_input_root(),
            point,
            data,
            effects,
            modifiers,
        )));
    }

    fn set_render_scaling(&self, scaling: f64) {
        if scaling <= 0.0 {
            panic!("Scaling must be greater than zero. (Parameter 'scaling')");
        }

        if self.surface.render_scaling() == scaling {
            return;
        }

        let old_scaled_size = self.surface.client_size();
        self.surface.set_render_scaling(scaling);
        if let Some(scaling_changed) = get(&self.scaling_changed) {
            scaling_changed(scaling);
        }
        IWindowImpl::resize(self, old_scaled_size, WindowResizeReason::DpiChange);
    }
}
