//! The top-level implementation shared by windows, popups and embeddable
//! top-levels, its native handle and the receiver of the native top-level
//! events.

use crate::clipboard_data_transfer::ClipboardDataTransfer;
use crate::clipboard_read_session::ClipboardReadSession;
use crate::cursor::FerroNativeCursor;
use crate::deferred_framebuffer::DeferredFramebuffer;
use crate::ferro_native_drag_source::{data_transfer_from_handle, free_data_transfer_handle};
use crate::ferro_native_text_input_method::FerroNativeTextInputMethod;
use crate::helpers::*;
use crate::interop::*;
use crate::metal::MetalPlatformSurface;
use crate::platform_behavior_inhibition::PlatformBehaviorInhibition;
use ferroui_base::input::platform::{IClipboard, PlatformDataTransfer};
use ferroui_base::input::raw::{
    IDragDropDevice, RawDragEvent, IRawInputEventArgs, RawKeyEventArgs, RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType,
    RawPointerGestureEventArgs, RawPointerPoint, RawTextInputEventArgs,
};
use ferroui_base::input::text_input::ITextInputMethodImpl;
use ferroui_base::input::{
    DragDropEffects, IDataTransfer, CaptureSource, IInputDevice, IInputRoot, IKeyboardDevice, IPointer, KeyDeviceType, MouseDevice, PenDevice,
    StandardCursorType,
};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
    IPlatformRenderSurfaceRenderTarget,
};
use ferroui_base::platform::{
    ICursorFactory, ICursorImpl, ILockedFramebuffer, IMacOSTopLevelPlatformHandle, IPlatformBehaviorInhibition,
    RenderTargetSceneInfo,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Point, Rect, Ref, Size, Vector, Visual};
use ferroui_controls::platform::{IPlatformHandle, IPopupImpl, IScreenImpl, ITopLevelImpl, PlatformThemeVariant};
use ferroui_controls::{AcrylicPlatformCompensationLevels, TopLevel, WindowResizeReason, WindowTransparencyLevel};
use ferroui_microcom::{ComPtr, HResult};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::ffi::{c_void, CStr};
use std::rc::{Rc, Weak};

/// The platform handle of a top-level of this backend.
///
/// A handle obtained from `ITopLevelImpl::handle` of this backend can be
/// downcast (through `as_any`) to this type, which implements
/// [`IMacOSTopLevelPlatformHandle`].
pub struct MacOSTopLevelHandle {
    native: ComPtr<IFrnTopLevel>,
    window_base: Option<ComPtr<IFrnWindowBase>>,
    handle: isize,
    handle_descriptor: &'static str,
}

impl MacOSTopLevelHandle {
    /// The handle of a top-level without a window; describes its `NSView`.
    pub(crate) fn from_top_level(native: ComPtr<IFrnTopLevel>) -> Rc<Self> {
        let mut this = Self { native, window_base: None, handle: 0, handle_descriptor: "NSView" };
        this.handle = this.ns_view();
        Rc::new(this)
    }

    /// The handle of a window or popup; describes its `NSWindow`.
    pub(crate) fn from_window_base(native: ComPtr<IFrnWindowBase>) -> Rc<Self> {
        let top_level = ComPtr::<IFrnTopLevel>::from_ref(&native);
        let mut this =
            Self { native: top_level, window_base: Some(native), handle: 0, handle_descriptor: "NSWindow" };
        this.handle = this.ns_window();
        Rc::new(this)
    }

    pub(crate) fn native(&self) -> &ComPtr<IFrnTopLevel> {
        &self.native
    }

    pub(crate) fn window_base(&self) -> Option<&ComPtr<IFrnWindowBase>> {
        self.window_base.as_ref()
    }
}

impl IMacOSTopLevelPlatformHandle for MacOSTopLevelHandle {
    fn ns_view(&self) -> isize {
        self.native.obtain_ns_view_handle().check() as isize
    }

    fn get_ns_view_retained(&self) -> isize {
        self.native.obtain_ns_view_handle_retained().check() as isize
    }

    fn ns_window(&self) -> isize {
        self.window_base.as_ref().map_or(0, |native| native.obtain_ns_window_handle().check() as isize)
    }

    fn get_ns_window_retained(&self) -> isize {
        self.window_base.as_ref().map_or(0, |native| native.obtain_ns_window_handle_retained().check() as isize)
    }
}

impl IPlatformHandle for MacOSTopLevelHandle {
    fn handle(&self) -> isize {
        self.handle
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(self.handle_descriptor)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// What the concrete top-level types (window, popup, embeddable top-level)
/// provide to the shared code: the base object and the members the
/// reference implementation overrides per type.
pub(crate) trait TopLevelParent: 'static {
    /// The shared top-level state.
    fn top_level(&self) -> &Rc<TopLevelImpl>;

    /// Releases the native top-level.
    fn dispose_top_level(&self) {
        self.top_level().dispose();
    }

    /// Lets a window handle a pointer event on its chrome; `true` swallows
    /// the event.
    fn chrome_hit_test(&self, _e: &RawPointerEventArgs) -> bool {
        false
    }

    /// Creates a popup that belongs to the top-level.
    fn create_popup_core(&self) -> Option<Rc<dyn IPopupImpl>>;

    fn set_frame_theme_variant_core(&self, _theme_variant: Option<PlatformThemeVariant>) {
        // no-op
    }

    fn try_get_feature_core(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.top_level().try_get_feature(feature_type)
    }
}

/// The state and behaviour every top-level of this backend shares. It also
/// is the software (framebuffer) render surface of the top-level.
pub struct TopLevelImpl {
    weak_self: Weak<TopLevelImpl>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    platform_behavior_inhibition: RefCell<Option<Rc<PlatformBehaviorInhibition>>>,
    input_method: RefCell<Option<Rc<FerroNativeTextInputMethod>>>,

    mouse: Rc<MouseDevice>,
    pen: Rc<PenDevice>,

    keyboard: Option<Rc<dyn IKeyboardDevice>>,
    cursor_factory: Option<Rc<dyn ICursorFactory>>,

    factory: ComPtr<IFerroNativeFactory>,

    saved_logical_size: Cell<Size>,
    saved_scaling: Cell<f64>,
    transparency_level: Cell<WindowTransparencyLevel>,

    handle: RefCell<Option<Rc<MacOSTopLevelHandle>>>,

    surfaces: RefCell<Option<Vec<Rc<dyn IPlatformRenderSurface>>>>,

    input: RefCell<Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>>,
    paint: RefCell<Option<Rc<dyn Fn(Rect)>>>,
    resized: RefCell<Option<Rc<dyn Fn(Size, WindowResizeReason)>>>,
    scaling_changed: RefCell<Option<Rc<dyn Fn(f64)>>>,
    transparency_level_changed: RefCell<Option<Rc<dyn Fn(WindowTransparencyLevel)>>>,
    closed: RefCell<Option<Rc<dyn Fn()>>>,
    lost_focus: RefCell<Option<Rc<dyn Fn()>>>,
}

impl TopLevelImpl {
    pub(crate) fn new(factory: ComPtr<IFerroNativeFactory>) -> Rc<TopLevelImpl> {
        let locator = FerroLocator::current();
        Rc::new_cyclic(|weak_self| TopLevelImpl {
            weak_self: weak_self.clone(),
            input_root: RefCell::new(None),
            platform_behavior_inhibition: RefCell::new(None),
            input_method: RefCell::new(None),
            mouse: MouseDevice::primary(),
            pen: PenDevice::new(false),
            keyboard: locator.get_service::<dyn IKeyboardDevice>(),
            cursor_factory: locator.get_service::<dyn ICursorFactory>(),
            factory,
            saved_logical_size: Cell::new(Size::default()),
            saved_scaling: Cell::new(0.0),
            transparency_level: Cell::new(WindowTransparencyLevel::none()),
            handle: RefCell::new(None),
            surfaces: RefCell::new(None),
            input: RefCell::new(None),
            paint: RefCell::new(None),
            resized: RefCell::new(None),
            scaling_changed: RefCell::new(None),
            transparency_level_changed: RefCell::new(None),
            closed: RefCell::new(None),
            lost_focus: RefCell::new(None),
        })
    }

    pub(crate) fn init(&self, handle: Rc<MacOSTopLevelHandle>) {
        let native = handle.native().clone();
        *self.handle.borrow_mut() = Some(handle);
        self.saved_logical_size.set(self.client_size());
        self.saved_scaling.set(native.get_scaling().check());
        *self.platform_behavior_inhibition.borrow_mut() = self
            .factory
            .create_platform_behavior_inhibition()
            .check()
            .map(|native| Rc::new(PlatformBehaviorInhibition::new(native)));

        *self.input_method.borrow_mut() = Some(FerroNativeTextInputMethod::new(&native));
        let metal_surface: Rc<dyn IPlatformRenderSurface> = MetalPlatformSurface::new(native);
        let this: Rc<dyn IPlatformRenderSurface> =
            self.weak_self.upgrade().expect("the top-level is alive while it is initialized");
        *self.surfaces.borrow_mut() = Some(vec![metal_surface, this]);
    }

    /// The top-level of this backend that hosts `visual`, if any.
    pub(crate) fn find_for_visual(visual: &Visual) -> Option<Rc<TopLevelImpl>> {
        let platform_impl = TopLevel::get_top_level(Some(visual))?.platform_impl()?;
        top_level_of(&*platform_impl).cloned()
    }

    pub(crate) fn begin_dragging_session(
        &self,
        effects: FrnDragDropEffects,
        point: FrnPoint,
        source: &IFrnClipboardDataSource,
        callback: &IFrnDndResultCallback,
        source_handle: *mut c_void,
    ) {
        match self.native() {
            // SAFETY: `source_handle` is an opaque value for native code,
            // which hands it back in drag events and to the handle
            // deallocator.
            Some(native) => unsafe {
                native.begin_drag_and_drop_operation(effects, point, Some(source), Some(callback), source_handle)
            }
            .check(),
            // Nothing took the handle over.
            // SAFETY: the handle was made for this call and is not used again.
            None => unsafe { free_data_transfer_handle(source_handle) },
        }
    }

    /// The text input method of the top-level.
    pub fn input_method(&self) -> Option<Rc<FerroNativeTextInputMethod>> {
        self.input_method.borrow().clone()
    }

    pub(crate) fn set_input_method(&self, value: Option<Rc<FerroNativeTextInputMethod>>) {
        let old = self.input_method.replace(value);
        drop(old);
    }

    fn on_drag_event(
        &self,
        type_: FrnDragEventType,
        position: FrnPoint,
        modifiers: FrnInputModifiers,
        effects: FrnDragDropEffects,
        clipboard: Option<&IFrnClipboard>,
        data_transfer_handle: *mut c_void,
    ) -> FrnDragDropEffects {
        let Some(device) = FerroLocator::current().get_service::<dyn IDragDropDevice>() else {
            return FrnDragDropEffects::None;
        };

        let Some(input_root) = self.input_root() else {
            return FrnDragDropEffects::None;
        };

        // SAFETY: a non-null handle in a drag event is the handle the drag
        // source of this backend started the operation with.
        let data_transfer = unsafe { data_transfer_from_handle(data_transfer_handle) };

        // The dragging pasteboard, read for the duration of the event.
        let clipboard_data_transfer = clipboard.map(|clipboard| {
            let change_count = clipboard.get_change_count().check();
            ClipboardDataTransfer::new(ClipboardReadSession::new(ComPtr::from_ref(clipboard), change_count))
        });
        struct DisposeOnExit(Option<Rc<PlatformDataTransfer>>);
        impl Drop for DisposeOnExit {
            fn drop(&mut self) {
                if let Some(data_transfer) = self.0.take() {
                    data_transfer.dispose();
                }
            }
        }
        let _dispose = DisposeOnExit(clipboard_data_transfer.clone());

        let data_transfer: Rc<dyn IDataTransfer> = match (data_transfer, clipboard_data_transfer) {
            (Some(data_transfer), _) => data_transfer,
            (None, Some(clipboard_data_transfer)) => clipboard_data_transfer,
            // The native side always passes the dragging pasteboard.
            (None, None) => return FrnDragDropEffects::None,
        };

        let args = Rc::new(RawDragEvent::new(
            device,
            to_raw_drag_event_type(type_),
            input_root,
            to_ferro_point(position),
            data_transfer,
            DragDropEffects::from_bits_retain(effects.0),
            to_raw_input_modifiers(modifiers),
        ));
        self.invoke_input(args.clone());
        FrnDragDropEffects(args.effects().bits())
    }

    pub(crate) fn factory(&self) -> &ComPtr<IFerroNativeFactory> {
        &self.factory
    }

    pub(crate) fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    pub(crate) fn mouse(&self) -> &Rc<MouseDevice> {
        &self.mouse
    }

    pub fn desktop_scaling(&self) -> f64 {
        1.0
    }

    /// The native top-level; `None` once it is disposed.
    pub fn native(&self) -> Option<ComPtr<IFrnTopLevel>> {
        self.handle.borrow().as_ref().map(|handle| handle.native().clone())
    }

    /// The native window base, when the top-level is a window or a popup
    /// and is not disposed.
    pub(crate) fn native_window_base(&self) -> Option<ComPtr<IFrnWindowBase>> {
        self.handle.borrow().as_ref().and_then(|handle| handle.window_base().cloned())
    }

    /// The platform handle with its concrete type.
    pub fn mac_os_handle(&self) -> Option<Rc<MacOSTopLevelHandle>> {
        self.handle.borrow().clone()
    }

    pub fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        self.mac_os_handle().map(|handle| handle as Rc<dyn IPlatformHandle>)
    }

    pub fn client_size(&self) -> Size {
        match self.native() {
            None => Size::default(),
            Some(native) => to_ferro_size(native.get_client_size().check()),
        }
    }

    pub fn render_scaling(&self) -> f64 {
        self.saved_scaling.get()
    }

    pub fn surfaces(&self) -> Vec<Rc<dyn IPlatformRenderSurface>> {
        self.surfaces.borrow().clone().unwrap_or_default()
    }

    pub fn transparency_level(&self) -> WindowTransparencyLevel {
        self.transparency_level.get()
    }

    fn set_transparency_level(&self, value: WindowTransparencyLevel) {
        if self.transparency_level.get() != value {
            self.transparency_level.set(value);
            let changed = self.transparency_level_changed.borrow().clone();
            if let Some(changed) = changed {
                changed(value);
            }
        }
    }

    pub fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        AcrylicPlatformCompensationLevels::new(1.0, 0.0, 0.0)
    }

    fn invoke_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        let input = self.input.borrow().clone();
        if let Some(input) = input {
            input(args);
        }
    }

    /// Runs the dispatcher jobs that must not be overtaken by input.
    fn run_jobs_before_input() {
        Dispatcher::ui_thread()
            .run_jobs(Some(DispatcherPriority::from_value(DispatcherPriority::INPUT.value() + 1)));
    }

    pub fn raw_text_input_event(&self, time_stamp: u64, text: &str) -> bool {
        let Some(input_root) = self.input_root() else {
            return false;
        };

        let Some(keyboard) = self.keyboard.clone() else {
            return false;
        };

        Self::run_jobs_before_input();

        let device: Rc<dyn IInputDevice> = keyboard;
        let args = Rc::new(RawTextInputEventArgs::new(device, time_stamp, input_root, text));

        self.invoke_input(args.clone());

        args.handled()
    }

    pub fn raw_key_event(
        &self,
        type_: FrnRawKeyEventType,
        time_stamp: u64,
        modifiers: FrnInputModifiers,
        key: FrnKey,
        physical_key: FrnPhysicalKey,
        key_symbol: Option<String>,
    ) -> bool {
        let Some(input_root) = self.input_root() else {
            return false;
        };

        let Some(keyboard) = self.keyboard.clone() else {
            return false;
        };

        Self::run_jobs_before_input();

        let device: Rc<dyn IInputDevice> = keyboard;
        let args = Rc::new(RawKeyEventArgs::new(
            device,
            time_stamp,
            input_root,
            to_raw_key_event_type(type_),
            to_key(key),
            to_raw_input_modifiers(modifiers),
            to_physical_key(physical_key),
            key_symbol,
            KeyDeviceType::Keyboard,
        ));

        self.invoke_input(args.clone());

        args.handled()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn raw_mouse_event(
        &self,
        type_: FrnRawMouseEventType,
        device_type: FrnPointerDeviceType,
        time_stamp: u64,
        modifiers: FrnInputModifiers,
        point: FrnPoint,
        delta: FrnVector,
        pressure: f32,
        x_tilt: f32,
        y_tilt: f32,
        chrome_hit_test: &dyn Fn(&RawPointerEventArgs) -> bool,
    ) {
        let Some(input_root) = self.input_root() else {
            return;
        };

        Self::run_jobs_before_input();

        let mouse: Rc<dyn IInputDevice> = self.mouse.clone();
        let position = to_ferro_point(point);
        let delta = Vector::new(delta.x, delta.y);
        let modifiers = to_raw_input_modifiers(modifiers);

        if type_ == FrnRawMouseEventType::Wheel {
            self.invoke_input(Rc::new(RawMouseWheelEventArgs::new(
                mouse, time_stamp, input_root, position, delta, modifiers,
            )));
        } else if type_ == FrnRawMouseEventType::Magnify {
            self.invoke_input(Rc::new(RawPointerGestureEventArgs::new(
                mouse,
                time_stamp,
                input_root,
                RawPointerEventType::Magnify,
                position,
                delta,
                modifiers,
            )));
        } else if type_ == FrnRawMouseEventType::Rotate {
            self.invoke_input(Rc::new(RawPointerGestureEventArgs::new(
                mouse,
                time_stamp,
                input_root,
                RawPointerEventType::Rotate,
                position,
                delta,
                modifiers,
            )));
        } else if type_ == FrnRawMouseEventType::Swipe {
            self.invoke_input(Rc::new(RawPointerGestureEventArgs::new(
                mouse,
                time_stamp,
                input_root,
                RawPointerEventType::Swipe,
                position,
                delta,
                modifiers,
            )));
        } else {
            // A value the toolkit does not know has nothing to be delivered as.
            let Some(type_) = to_raw_pointer_event_type(type_) else {
                return;
            };
            let device: Rc<dyn IInputDevice> =
                if device_type == FrnPointerDeviceType::Pen { self.pen.clone() } else { mouse };
            let mut raw_point = RawPointerPoint::new();
            raw_point.position = position;
            raw_point.pressure = pressure;
            raw_point.x_tilt = x_tilt;
            raw_point.y_tilt = y_tilt;
            let e =
                Rc::new(RawPointerEventArgs::with_point(device, time_stamp, input_root, type_, raw_point, modifiers));

            if !chrome_hit_test(&e) {
                self.invoke_input(e);
            }
        }
    }

    pub fn invalidate(&self) {
        if let Some(native) = self.native() {
            native.invalidate().check();
        }
    }

    pub fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        let old = self.input_root.replace(Some(input_root));
        drop(old);
    }

    pub fn point_to_client(&self, point: PixelPoint) -> Point {
        match self.native() {
            Some(native) => to_ferro_point(native.point_to_client(pixel_point_to_frn_point(point)).check()),
            None => Point::default(),
        }
    }

    pub fn point_to_screen(&self, point: Point) -> PixelPoint {
        match self.native() {
            Some(native) => to_ferro_pixel_point(native.point_to_screen(to_frn_point(point)).check()),
            None => PixelPoint::default(),
        }
    }

    pub fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
        let Some(native) = self.native() else {
            return;
        };

        let native_cursor = match cursor.as_deref().and_then(FerroNativeCursor::from_cursor_impl) {
            Some(new_cursor) => new_cursor.cursor(),
            None => self.cursor_factory.as_ref().and_then(|cursor_factory| {
                let arrow = cursor_factory.get_cursor(StandardCursorType::Arrow);
                FerroNativeCursor::from_cursor_impl(&*arrow).and_then(FerroNativeCursor::cursor)
            }),
        };
        native.set_cursor(native_cursor.as_deref()).check();
    }

    pub fn set_transparency_level_hint(&self, transparency_levels: &[WindowTransparencyLevel]) {
        for &level in transparency_levels {
            let mut mode = None;

            if level == WindowTransparencyLevel::none() {
                mode = Some(FrnWindowTransparencyMode::Opaque);
            }
            if level == WindowTransparencyLevel::transparent() {
                mode = Some(FrnWindowTransparencyMode::Transparent);
            } else if level == WindowTransparencyLevel::acrylic_blur() {
                mode = Some(FrnWindowTransparencyMode::Blur);
            }

            if let Some(mode) = mode {
                if level != self.transparency_level() {
                    if let Some(native) = self.native() {
                        native.set_transparency_mode(mode).check();
                    }
                    self.set_transparency_level(level);
                    return;
                }
            }
        }

        // If we get here, we didn't find a supported level. Use the default of None.
        if self.transparency_level() != WindowTransparencyLevel::none() {
            if let Some(native) = self.native() {
                native.set_transparency_mode(FrnWindowTransparencyMode::Opaque).check();
            }
            self.set_transparency_level(WindowTransparencyLevel::none());
        }
    }

    /// The optional features of every top-level of this backend.
    ///
    /// The features whose contracts are not ported yet (native control host,
    /// launcher) are absent.
    pub fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn ITextInputMethodImpl>() {
            let input_method: Rc<dyn ITextInputMethodImpl> = self.input_method()?;
            return Some(Rc::new(input_method));
        }

        if feature_type == TypeId::of::<dyn IClipboard>() {
            let clipboard = FerroLocator::current().get_required_service::<dyn IClipboard>();
            return Some(Rc::new(clipboard));
        }

        if feature_type == TypeId::of::<dyn IPlatformBehaviorInhibition>() {
            let inhibition: Rc<dyn IPlatformBehaviorInhibition> = self.platform_behavior_inhibition.borrow().clone()?;
            return Some(Rc::new(inhibition));
        }

        if feature_type == TypeId::of::<dyn IScreenImpl>() {
            let screens = FerroLocator::current().get_required_service::<dyn IScreenImpl>();
            return Some(Rc::new(screens));
        }

        None
    }

    /// The screens service, as the top-level feature.
    pub(crate) fn screens(&self) -> Rc<dyn IScreenImpl> {
        FerroLocator::current().get_required_service::<dyn IScreenImpl>()
    }

    pub(crate) fn dispose(&self) {
        let handle = self.handle.borrow_mut().take();
        drop(handle);
    }

    // --- the native events that only touch the shared state ---------------

    fn on_paint(&self) {
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::UI_THREAD_RENDER));
        let s = self.client_size();
        let paint = self.paint.borrow().clone();
        if let Some(paint) = paint {
            paint(Rect::new(0.0, 0.0, s.width, s.height));
        }
    }

    fn on_resized(&self, size: &FrnSize, reason: FrnPlatformResizeReason) {
        if self.native().is_none() {
            return;
        }

        let s = Size::new(size.width, size.height);
        self.saved_logical_size.set(s);
        let resized = self.resized.borrow().clone();
        if let Some(resized) = resized {
            resized(s, to_window_resize_reason(reason));
        }
    }

    fn on_scaling_changed(&self, scaling: f64) {
        self.saved_scaling.set(scaling);
        let scaling_changed = self.scaling_changed.borrow().clone();
        if let Some(scaling_changed) = scaling_changed {
            scaling_changed(scaling);
        }
    }

    fn on_lost_focus(&self) {
        let lost_focus = self.lost_focus.borrow().clone();
        if let Some(lost_focus) = lost_focus {
            lost_focus();
        }

        // macOS doesn't have the concept of mouse capture. If we're losing the focus during an implicit capture
        // (standard mouse down), we should release it to avoid mouse events going to an old window.
        let pointer = self.mouse.pointer();
        let Some(captured) = pointer.captured() else {
            return;
        };

        let captured: Ref<Visual> = captured.upcast();
        if pointer.capture_source() == CaptureSource::Implicit
            && Self::find_for_visual(&captured).is_some_and(|owner| std::ptr::eq(&*owner, self))
        {
            self.mouse.platform_capture_lost();
        }
    }
}

macro_rules! callback_property {
    ($getter:ident, $setter:ident, $field:ident, $ty:ty) => {
        pub fn $getter(&self) -> Option<Rc<$ty>> {
            self.$field.borrow().clone()
        }

        pub fn $setter(&self, value: Option<Rc<$ty>>) {
            let old = self.$field.replace(value);
            drop(old);
        }
    };
}
pub(crate) use callback_property;

impl TopLevelImpl {
    callback_property!(input, set_input, input, dyn Fn(Rc<dyn IRawInputEventArgs>));
    callback_property!(paint, set_paint, paint, dyn Fn(Rect));
    callback_property!(resized, set_resized, resized, dyn Fn(Size, WindowResizeReason));
    callback_property!(scaling_changed, set_scaling_changed, scaling_changed, dyn Fn(f64));
    callback_property!(
        transparency_level_changed,
        set_transparency_level_changed,
        transparency_level_changed,
        dyn Fn(WindowTransparencyLevel)
    );
    callback_property!(closed, set_closed, closed, dyn Fn());
    callback_property!(lost_focus, set_lost_focus, lost_focus, dyn Fn());
}

impl IPlatformRenderSurface for TopLevelImpl {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for TopLevelImpl {
    /// # Panics
    /// Panics when called off the UI thread or when the top-level has no
    /// native render target (it is closed): the render target is not ready.
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        if !Dispatcher::ui_thread().check_access() {
            panic!("The render target is not ready.");
        }

        let native_render_target = self.native().and_then(|native| native.create_software_render_target().check());

        let Some(native_render_target) = native_render_target else {
            panic!("The render target is not ready.");
        };

        Rc::new(FramebufferRenderTarget {
            parent: self.weak_self.upgrade().expect("the top-level is alive while it is borrowed"),
            target: Rc::new(RefCell::new(Some(native_render_target))),
        })
    }
}

/// The software render target of a top-level.
struct FramebufferRenderTarget {
    parent: Rc<TopLevelImpl>,
    target: Rc<RefCell<Option<ComPtr<IFrnSoftwareRenderTarget>>>>,
}

impl IPlatformRenderSurfaceRenderTarget for FramebufferRenderTarget {}

impl IFramebufferRenderTarget for FramebufferRenderTarget {
    fn lock(&self, _scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        let Some(target) = self.target.borrow().clone() else {
            panic!("Cannot access a disposed object: FramebufferRenderTarget");
        };
        let size = self.parent.saved_logical_size.get();
        let scaling = self.parent.saved_scaling.get();
        let w = (size.width * scaling).max(1.0);
        let h = (size.height * scaling).max(1.0);
        let dpi = scaling * 96.0;

        let parent = self.parent.clone();
        let current_target = self.target.clone();
        let framebuffer = DeferredFramebuffer::new(
            target,
            Box::new(move |cb: &mut dyn FnMut()| {
                // Keeps the native top-level alive for the duration of the call.
                if let Some(_native) = parent.native() {
                    if current_target.borrow().is_some() {
                        cb();
                    }
                }
            }),
            w as i32,
            h as i32,
            Vector::new(dpi, dpi),
        );
        (Rc::new(framebuffer), FramebufferLockProperties::default())
    }

    fn retains_frame_contents(&self) -> bool {
        false
    }

    fn dispose(&self) {
        let target = self.target.borrow_mut().take();
        drop(target);
    }
}

/// Receives the events of a native top-level for the top-level `P`.
///
/// One type serves every kind of top-level: it implements the window base
/// and window event interfaces when `P` provides the matching members.
pub(crate) struct TopLevelEvents<P>(pub(crate) Rc<P>);

/// Disposes the top-level when the `Closed` callback is done, whether it
/// returned or panicked.
struct DisposeOnExit<'a, P: TopLevelParent> {
    parent: &'a P,
    native: Option<ComPtr<IFrnTopLevel>>,
}

impl<P: TopLevelParent> Drop for DisposeOnExit<'_, P> {
    fn drop(&mut self) {
        self.parent.dispose_top_level();
        drop(self.native.take());
    }
}

fn c_str_to_string(s: Option<&CStr>) -> Option<String> {
    s.map(|s| s.to_string_lossy().into_owned())
}

impl<P: TopLevelParent> IFrnTopLevelEventsImpl for TopLevelEvents<P> {
    fn closed(&self) {
        crate::callback_base::guard((), || {
            let top_level = self.0.top_level();
            let _dispose = DisposeOnExit { parent: &*self.0, native: top_level.native() };
            let closed = top_level.closed();
            if let Some(closed) = closed {
                closed();
            }
        })
    }

    fn paint(&self) -> Result<(), HResult> {
        crate::callback_base::guard(Ok(()), || {
            self.0.top_level().on_paint();
            Ok(())
        })
    }

    fn resized(&self, size: &FrnSize, reason: FrnPlatformResizeReason) {
        crate::callback_base::guard((), || self.0.top_level().on_resized(size, reason))
    }

    fn raw_mouse_event(
        &self,
        type_: FrnRawMouseEventType,
        device_type: FrnPointerDeviceType,
        time_stamp: u64,
        modifiers: FrnInputModifiers,
        point: FrnPoint,
        delta: FrnVector,
        pressure: f32,
        x_tilt: f32,
        y_tilt: f32,
    ) {
        crate::callback_base::guard((), || {
            self.0.top_level().raw_mouse_event(
                type_,
                device_type,
                time_stamp,
                modifiers,
                point,
                delta,
                pressure,
                x_tilt,
                y_tilt,
                &|e| self.0.chrome_hit_test(e),
            )
        })
    }

    fn raw_key_event(
        &self,
        type_: FrnRawKeyEventType,
        time_stamp: u64,
        modifiers: FrnInputModifiers,
        key: FrnKey,
        physical_key: FrnPhysicalKey,
        key_symbol: Option<&CStr>,
    ) -> bool {
        crate::callback_base::guard(false, || {
            self.0.top_level().raw_key_event(
                type_,
                time_stamp,
                modifiers,
                key,
                physical_key,
                c_str_to_string(key_symbol),
            )
        })
    }

    fn raw_text_input_event(&self, time_stamp: u64, text: Option<&CStr>) -> bool {
        crate::callback_base::guard(false, || {
            let text = c_str_to_string(text).unwrap_or_default();
            self.0.top_level().raw_text_input_event(time_stamp, &text)
        })
    }

    fn scaling_changed(&self, scaling: f64) {
        crate::callback_base::guard((), || self.0.top_level().on_scaling_changed(scaling))
    }

    fn run_render_priority_jobs(&self) {
        crate::callback_base::guard((), || {
            Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::UI_THREAD_RENDER));
        })
    }

    fn lost_focus(&self) {
        crate::callback_base::guard((), || self.0.top_level().on_lost_focus())
    }

    fn get_automation_peer(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        // Automation peers are not ported yet: the top-level has none.
        None
    }

    fn drag_event(
        &self,
        type_: FrnDragEventType,
        position: FrnPoint,
        modifiers: FrnInputModifiers,
        effects: FrnDragDropEffects,
        clipboard: Option<&IFrnClipboard>,
        data_transfer_handle: *mut c_void,
    ) -> FrnDragDropEffects {
        crate::callback_base::guard(FrnDragDropEffects::None, || {
            self.0.top_level().on_drag_event(type_, position, modifiers, effects, clipboard, data_transfer_handle)
        })
    }
}

/// Implements `IOptionalFeatureProvider`, `IDisposable` and `ITopLevelImpl`
/// for a type that is a [`TopLevelParent`]; the braces take the `as_*`
/// overrides of the type.
macro_rules! impl_top_level_contract {
    ($ty:ty { $($extra:tt)* }) => {
        impl ferroui_base::platform::IOptionalFeatureProvider for $ty {
            fn try_get_feature(&self, feature_type: std::any::TypeId) -> Option<std::rc::Rc<dyn std::any::Any>> {
                $crate::top_level_impl::TopLevelParent::try_get_feature_core(self, feature_type)
            }
        }

        impl ferroui_base::reactive::IDisposable for $ty {
            fn dispose(&self) {
                $crate::top_level_impl::TopLevelParent::dispose_top_level(self)
            }
        }

        impl ferroui_controls::platform::ITopLevelImpl for $ty {
            fn desktop_scaling(&self) -> f64 {
                $crate::top_level_impl::TopLevelParent::top_level(self).desktop_scaling()
            }

            fn handle(&self) -> Option<std::rc::Rc<dyn ferroui_controls::platform::IPlatformHandle>> {
                $crate::top_level_impl::TopLevelParent::top_level(self).handle()
            }

            fn client_size(&self) -> ferroui_base::Size {
                $crate::top_level_impl::TopLevelParent::top_level(self).client_size()
            }

            fn render_scaling(&self) -> f64 {
                $crate::top_level_impl::TopLevelParent::top_level(self).render_scaling()
            }

            fn surfaces(&self) -> Vec<std::rc::Rc<dyn ferroui_base::platform::surfaces::IPlatformRenderSurface>> {
                $crate::top_level_impl::TopLevelParent::top_level(self).surfaces()
            }

            fn compositor(&self) -> Option<std::rc::Rc<ferroui_base::rendering::composition::Compositor>> {
                Some($crate::ferro_native_platform::FerroNativePlatform::compositor())
            }

            fn input(
                &self,
            ) -> Option<std::rc::Rc<dyn Fn(std::rc::Rc<dyn ferroui_base::input::raw::IRawInputEventArgs>)>> {
                $crate::top_level_impl::TopLevelParent::top_level(self).input()
            }

            fn set_input(
                &self,
                value: Option<std::rc::Rc<dyn Fn(std::rc::Rc<dyn ferroui_base::input::raw::IRawInputEventArgs>)>>,
            ) {
                $crate::top_level_impl::TopLevelParent::top_level(self).set_input(value)
            }

            fn paint(&self) -> Option<std::rc::Rc<dyn Fn(ferroui_base::Rect)>> {
                $crate::top_level_impl::TopLevelParent::top_level(self).paint()
            }

            fn set_paint(&self, value: Option<std::rc::Rc<dyn Fn(ferroui_base::Rect)>>) {
                $crate::top_level_impl::TopLevelParent::top_level(self).set_paint(value)
            }

            fn resized(
                &self,
            ) -> Option<std::rc::Rc<dyn Fn(ferroui_base::Size, ferroui_controls::WindowResizeReason)>> {
                $crate::top_level_impl::TopLevelParent::top_level(self).resized()
            }

            fn set_resized(
                &self,
                value: Option<std::rc::Rc<dyn Fn(ferroui_base::Size, ferroui_controls::WindowResizeReason)>>,
            ) {
                $crate::top_level_impl::TopLevelParent::top_level(self).set_resized(value)
            }

            fn scaling_changed(&self) -> Option<std::rc::Rc<dyn Fn(f64)>> {
                $crate::top_level_impl::TopLevelParent::top_level(self).scaling_changed()
            }

            fn set_scaling_changed(&self, value: Option<std::rc::Rc<dyn Fn(f64)>>) {
                $crate::top_level_impl::TopLevelParent::top_level(self).set_scaling_changed(value)
            }

            fn transparency_level_changed(
                &self,
            ) -> Option<std::rc::Rc<dyn Fn(ferroui_controls::WindowTransparencyLevel)>> {
                $crate::top_level_impl::TopLevelParent::top_level(self).transparency_level_changed()
            }

            fn set_transparency_level_changed(
                &self,
                value: Option<std::rc::Rc<dyn Fn(ferroui_controls::WindowTransparencyLevel)>>,
            ) {
                $crate::top_level_impl::TopLevelParent::top_level(self).set_transparency_level_changed(value)
            }

            fn set_input_root(&self, input_root: std::rc::Rc<dyn ferroui_base::input::IInputRoot>) {
                $crate::top_level_impl::TopLevelParent::top_level(self).set_input_root(input_root)
            }

            fn point_to_client(&self, point: ferroui_base::PixelPoint) -> ferroui_base::Point {
                $crate::top_level_impl::TopLevelParent::top_level(self).point_to_client(point)
            }

            fn point_to_screen(&self, point: ferroui_base::Point) -> ferroui_base::PixelPoint {
                $crate::top_level_impl::TopLevelParent::top_level(self).point_to_screen(point)
            }

            fn set_cursor(&self, cursor: Option<std::rc::Rc<dyn ferroui_base::platform::ICursorImpl>>) {
                $crate::top_level_impl::TopLevelParent::top_level(self).set_cursor(cursor)
            }

            fn closed(&self) -> Option<std::rc::Rc<dyn Fn()>> {
                $crate::top_level_impl::TopLevelParent::top_level(self).closed()
            }

            fn set_closed(&self, value: Option<std::rc::Rc<dyn Fn()>>) {
                $crate::top_level_impl::TopLevelParent::top_level(self).set_closed(value)
            }

            fn lost_focus(&self) -> Option<std::rc::Rc<dyn Fn()>> {
                $crate::top_level_impl::TopLevelParent::top_level(self).lost_focus()
            }

            fn set_lost_focus(&self, value: Option<std::rc::Rc<dyn Fn()>>) {
                $crate::top_level_impl::TopLevelParent::top_level(self).set_lost_focus(value)
            }

            fn create_popup(&self) -> Option<std::rc::Rc<dyn ferroui_controls::platform::IPopupImpl>> {
                $crate::top_level_impl::TopLevelParent::create_popup_core(self)
            }

            fn set_transparency_level_hint(&self, transparency_levels: &[ferroui_controls::WindowTransparencyLevel]) {
                $crate::top_level_impl::TopLevelParent::top_level(self)
                    .set_transparency_level_hint(transparency_levels)
            }

            fn transparency_level(&self) -> ferroui_controls::WindowTransparencyLevel {
                $crate::top_level_impl::TopLevelParent::top_level(self).transparency_level()
            }

            fn acrylic_compensation_levels(&self) -> ferroui_controls::AcrylicPlatformCompensationLevels {
                $crate::top_level_impl::TopLevelParent::top_level(self).acrylic_compensation_levels()
            }

            fn set_frame_theme_variant(
                &self,
                theme_variant: Option<ferroui_controls::platform::PlatformThemeVariant>,
            ) {
                $crate::top_level_impl::TopLevelParent::set_frame_theme_variant_core(self, theme_variant)
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            $($extra)*
        }
    };
}
pub(crate) use impl_top_level_contract;

/// The shared top-level state of a top-level implementation of this
/// backend; `None` for an implementation of another backend.
pub(crate) fn top_level_of(top_level: &dyn ITopLevelImpl) -> Option<&Rc<TopLevelImpl>> {
    let any = top_level.as_any();
    if let Some(window) = any.downcast_ref::<crate::window_impl::WindowImpl>() {
        return Some(window.top_level());
    }
    if let Some(popup) = any.downcast_ref::<crate::popup_impl::PopupImpl>() {
        return Some(popup.top_level());
    }
    if let Some(embeddable) = any.downcast_ref::<crate::embeddable_top_level_impl::EmbeddableTopLevelImpl>() {
        return Some(embeddable.top_level());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::raw::{RawKeyEventType, RawPointerEventType};
    use ferroui_base::input::{FocusManager, InputElement, Key, KeyboardDevice, PhysicalKey, RawInputModifiers};
    struct InputRoot {
        root: Ref<InputElement>,
    }

    impl IInputRoot for InputRoot {
        fn focus_manager(&self) -> Option<Rc<FocusManager>> {
            None
        }
        fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
            None
        }
        fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}
        fn cursor_element(&self) -> Option<Ref<InputElement>> {
            None
        }
        fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}
        fn root_element(&self) -> Ref<InputElement> {
            self.root.clone()
        }
        fn focus_root(&self) -> Ref<InputElement> {
            self.root.clone()
        }
        fn pointer_over_invalidated(&self) {}
    }

    /// A top-level without a native object (as after it is disposed), with
    /// a keyboard device, an input root and an input callback that records
    /// what it is given. Creating the factory does not need a window server.
    fn top_level(with_input_root: bool) -> (Rc<TopLevelImpl>, Rc<RefCell<Vec<Rc<dyn IRawInputEventArgs>>>>) {
        let keyboard: Rc<dyn IKeyboardDevice> = KeyboardDevice::new();
        FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(keyboard);
        let top_level = TopLevelImpl::new(create_ferro_native().expect("factory"));
        if with_input_root {
            top_level.set_input_root(Rc::new(InputRoot { root: InputElement::new() }));
        }
        let received = Rc::new(RefCell::new(Vec::new()));
        let sink = received.clone();
        top_level.set_input(Some(Rc::new(move |e| sink.borrow_mut().push(e))));
        (top_level, received)
    }

    fn mouse(top_level: &TopLevelImpl, type_: FrnRawMouseEventType, device: FrnPointerDeviceType, swallow: bool) {
        top_level.raw_mouse_event(
            type_,
            device,
            42,
            FrnInputModifiers::Shift | FrnInputModifiers::LeftMouseButton,
            FrnPoint { x: 10.5, y: 20.25 },
            FrnVector { x: 1.0, y: -2.0 },
            0.75,
            0.1,
            0.2,
            &|_| swallow,
        );
    }

    #[test]
    fn pointer_events_become_raw_pointer_args() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let _scope = FerroLocator::enter_scope();
        let (top_level, received) = top_level(true);

        mouse(&top_level, FrnRawMouseEventType::LeftButtonDown, FrnPointerDeviceType::Mouse, false);

        let received = received.borrow();
        assert_eq!(received.len(), 1);
        let e = received[0].downcast_ref::<RawPointerEventArgs>().expect("pointer args");
        assert_eq!(e.type_(), RawPointerEventType::LeftButtonDown);
        assert_eq!(e.position(), Point::new(10.5, 20.25));
        assert_eq!(e.input_modifiers(), RawInputModifiers::SHIFT | RawInputModifiers::LEFT_MOUSE_BUTTON);
        assert_eq!(e.timestamp(), 42);
        assert_eq!(e.point().pressure, 0.75);
        assert_eq!(e.point().x_tilt, 0.1);
        assert_eq!(e.point().y_tilt, 0.2);
        assert!(e.device().as_any().is::<MouseDevice>());
    }

    #[test]
    fn pen_events_come_from_the_pen_device() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let _scope = FerroLocator::enter_scope();
        let (top_level, received) = top_level(true);

        mouse(&top_level, FrnRawMouseEventType::Move, FrnPointerDeviceType::Pen, false);

        let received = received.borrow();
        let e = received[0].downcast_ref::<RawPointerEventArgs>().expect("pointer args");
        assert_eq!(e.type_(), RawPointerEventType::Move);
        assert!(e.device().as_any().is::<PenDevice>());
    }

    #[test]
    fn wheel_and_gesture_events_carry_their_delta() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let _scope = FerroLocator::enter_scope();
        let (top_level, received) = top_level(true);

        // Wheel and gestures are never offered to the chrome hit test.
        mouse(&top_level, FrnRawMouseEventType::Wheel, FrnPointerDeviceType::Mouse, true);
        mouse(&top_level, FrnRawMouseEventType::Magnify, FrnPointerDeviceType::Mouse, true);
        mouse(&top_level, FrnRawMouseEventType::Rotate, FrnPointerDeviceType::Mouse, true);
        mouse(&top_level, FrnRawMouseEventType::Swipe, FrnPointerDeviceType::Mouse, true);

        let received = received.borrow();
        assert_eq!(received.len(), 4);
        let wheel = received[0].downcast_ref::<RawMouseWheelEventArgs>().expect("wheel args");
        assert_eq!(wheel.delta(), Vector::new(1.0, -2.0));
        assert_eq!(wheel.position(), Point::new(10.5, 20.25));
        for (index, type_) in
            [(1, RawPointerEventType::Magnify), (2, RawPointerEventType::Rotate), (3, RawPointerEventType::Swipe)]
        {
            let gesture = received[index].downcast_ref::<RawPointerGestureEventArgs>().expect("gesture args");
            assert_eq!(gesture.type_(), type_);
            assert_eq!(gesture.delta(), Vector::new(1.0, -2.0));
        }
    }

    #[test]
    fn chrome_hit_test_can_swallow_pointer_events() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let _scope = FerroLocator::enter_scope();
        let (top_level, received) = top_level(true);

        mouse(&top_level, FrnRawMouseEventType::LeftButtonDown, FrnPointerDeviceType::Mouse, true);
        assert!(received.borrow().is_empty());

        // A type the toolkit does not know is dropped.
        mouse(&top_level, FrnRawMouseEventType(99), FrnPointerDeviceType::Mouse, false);
        assert!(received.borrow().is_empty());
    }

    #[test]
    fn key_and_text_events_report_whether_they_were_handled() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let _scope = FerroLocator::enter_scope();
        let (top_level, received) = top_level(true);

        let handled = top_level.raw_key_event(
            FrnRawKeyEventType::KeyUp,
            7,
            FrnInputModifiers::Windows,
            FrnKey::FrnKeyA,
            FrnPhysicalKey::FrnPhysicalKeyA,
            Some("a".to_string()),
        );
        assert!(!handled);
        assert!(!top_level.raw_text_input_event(8, "ä"));

        {
            let received = received.borrow();
            let key = received[0].downcast_ref::<RawKeyEventArgs>().expect("key args");
            assert_eq!(key.type_(), RawKeyEventType::KeyUp);
            assert_eq!(key.key(), Key::A);
            assert_eq!(key.physical_key(), PhysicalKey::A);
            assert_eq!(key.modifiers(), RawInputModifiers::META);
            assert_eq!(key.key_symbol().as_deref(), Some("a"));
            assert_eq!(key.timestamp(), 7);
            let text = received[1].downcast_ref::<RawTextInputEventArgs>().expect("text args");
            assert_eq!(text.text(), "ä");
        }

        top_level.set_input(Some(Rc::new(|e| e.set_handled(true))));
        assert!(top_level.raw_key_event(
            FrnRawKeyEventType::KeyDown,
            9,
            FrnInputModifiers::FrnInputModifiersNone,
            FrnKey::FrnKeyEscape,
            FrnPhysicalKey::FrnPhysicalKeyNone,
            None,
        ));
        assert!(top_level.raw_text_input_event(10, "x"));
    }

    #[test]
    fn drag_events_need_a_drag_drop_device() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let _scope = FerroLocator::enter_scope();
        let (top_level, received) = top_level(true);

        let effects = top_level.on_drag_event(
            FrnDragEventType::Enter,
            FrnPoint { x: 1.0, y: 2.0 },
            FrnInputModifiers::FrnInputModifiersNone,
            FrnDragDropEffects::Copy,
            None,
            std::ptr::null_mut(),
        );
        assert_eq!(effects, FrnDragDropEffects::None);
        assert!(received.borrow().is_empty());
    }

    #[test]
    fn nothing_is_delivered_without_an_input_root() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let _scope = FerroLocator::enter_scope();
        let (top_level, received) = top_level(false);

        mouse(&top_level, FrnRawMouseEventType::Move, FrnPointerDeviceType::Mouse, false);
        assert!(!top_level.raw_text_input_event(1, "x"));
        assert!(!top_level.raw_key_event(
            FrnRawKeyEventType::KeyDown,
            1,
            FrnInputModifiers::FrnInputModifiersNone,
            FrnKey::FrnKeyA,
            FrnPhysicalKey::FrnPhysicalKeyA,
            None,
        ));
        assert!(received.borrow().is_empty());
    }

    #[test]
    fn transparency_level_hint_picks_the_first_supported_level() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let _scope = FerroLocator::enter_scope();
        let (top_level, _received) = top_level(false);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let sink = changes.clone();
        top_level.set_transparency_level_changed(Some(Rc::new(move |level| sink.borrow_mut().push(level))));

        // Mica and Blur are not supported on macOS; AcrylicBlur is.
        top_level.set_transparency_level_hint(&[
            WindowTransparencyLevel::mica(),
            WindowTransparencyLevel::blur(),
            WindowTransparencyLevel::acrylic_blur(),
            WindowTransparencyLevel::transparent(),
        ]);
        assert_eq!(top_level.transparency_level(), WindowTransparencyLevel::acrylic_blur());

        // The current level is skipped, so the next supported one is taken.
        top_level
            .set_transparency_level_hint(&[WindowTransparencyLevel::acrylic_blur(), WindowTransparencyLevel::transparent()]);
        assert_eq!(top_level.transparency_level(), WindowTransparencyLevel::transparent());

        // Nothing supported: back to None.
        top_level.set_transparency_level_hint(&[WindowTransparencyLevel::mica()]);
        assert_eq!(top_level.transparency_level(), WindowTransparencyLevel::none());

        // Already None: no change is reported.
        top_level.set_transparency_level_hint(&[WindowTransparencyLevel::none()]);
        assert_eq!(
            *changes.borrow(),
            [
                WindowTransparencyLevel::acrylic_blur(),
                WindowTransparencyLevel::transparent(),
                WindowTransparencyLevel::none()
            ]
        );
    }

    #[test]
    fn a_top_level_without_native_object_answers_with_defaults() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let _scope = FerroLocator::enter_scope();
        let (top_level, _received) = top_level(false);

        assert_eq!(top_level.desktop_scaling(), 1.0);
        assert!(top_level.native().is_none());
        assert!(top_level.handle().is_none());
        assert_eq!(top_level.client_size(), Size::default());
        assert!(top_level.surfaces().is_empty());
        assert_eq!(top_level.point_to_client(PixelPoint::new(3, 4)), Point::default());
        assert_eq!(top_level.point_to_screen(Point::new(3.0, 4.0)), PixelPoint::default());
        assert_eq!(top_level.acrylic_compensation_levels(), AcrylicPlatformCompensationLevels::new(1.0, 0.0, 0.0));
        // No-ops without a native object.
        top_level.invalidate();
        top_level.set_cursor(None);
    }
}
