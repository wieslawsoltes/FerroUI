//! The top-level of a view: a surface view of the Java layer, rendered to
//! through EGL or the buffer of its native window.

use super::framebuffer_manager::FramebufferManager;
use super::invalidation_aware_surface_view::{
    InvalidationAwareSurfaceView, SurfaceProperties, SurfaceShared, SurfaceViewHandle,
};
use crate::android_platform::AndroidPlatform;
use crate::android_view_control_handle::AndroidViewControlHandle;
use crate::interop::java::{call_static_int, call_static_long, call_static_void, is_instance_of, JavaClass, JavaObject, JavaValue};
use crate::interop::natives::{next_handle, sdk_int, FERRO_ACTIVITY, PLATFORM_HELPER};
use crate::interop::ndk::NativeWindow;
use crate::platform::android_insets_manager::{ActivityInsetsWindow, IInsetsTopLevel};
use crate::i_init_editor_info::InitEditorInfo;
use crate::platform::input::android_input_method::{AndroidInputMethod, IInputMethodHost, ViewInputMethodHost};
use crate::platform::input::android_keyboard_device::AndroidKeyboardDevice;
use crate::platform::input::text_edit_buffer::IInputConnectionTopLevel;
use crate::platform::specific::helpers::android_keyboard_events_helper::{
    AndroidKeyboardEventsHelper, IKeyboardEventsTopLevel,
};
use crate::platform::specific::helpers::android_motion_events_helper::{
    AndroidMotionEventsHelper, IMotionEventsTopLevel,
};
use crate::ferro_activity::FerroActivity;
use crate::i_android_navigation_service::IActivityNavigationService;
use crate::platform::android_launcher::AndroidLauncher;
use crate::platform::android_platform_feedback::{AndroidPlatformFeedback, ViewFeedback};
use crate::platform::clipboard_impl::ClipboardImpl;
use crate::platform::storage::android_storage_provider::AndroidStorageProvider;
use ferroui_base::input::platform::{Clipboard, IClipboard};
use ferroui_base::platform::storage::{ILauncher, IStorageProvider};
use crate::platform::{
    AndroidInsetsManager, AndroidNativeControlHostImpl, AndroidScreens, AndroidSystemNavigationManagerImpl,
};
use ferroui_base::input::raw::{IRawInputEventArgs, RawTextInputEventArgs};
use ferroui_base::input::text_input::ITextInputMethodImpl;
use ferroui_base::input::{IInputDevice, IInputRoot, NavigationDirection};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    ICursorImpl, IOptionalFeatureProvider, ISystemNavigationManagerImpl, PlatformThemeVariant,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{PixelPoint, PixelSize, Point, Rect, Size};
use ferroui_controls::platform::{
    IInputPane, IInsetsManager, INativeControlHostImpl, IPlatformFeedback, IPlatformHandle, IPopupImpl, IScreenImpl,
    ITopLevelImpl, SystemBarTheme,
};
use ferroui_controls::{AcrylicPlatformCompensationLevels, WindowResizeReason, WindowTransparencyLevel};
use ferroui_opengl::egl::{
    EglGlPlatformSurface, IEglWindowGlPlatformSurfaceInfo, IEglWindowGlPlatformSurfaceInfoWithWaitPolicy,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};

thread_local! {
    static TOP_LEVELS: RefCell<HashMap<i64, Weak<TopLevelImpl>>> = RefCell::new(HashMap::new());
    static INSETS_MANAGERS: RefCell<HashMap<i64, Weak<AndroidInsetsManager>>> = RefCell::new(HashMap::new());
}

type Callback<T> = RefCell<Option<Rc<T>>>;

/// The window of the surface as EGL is given it.
///
/// The members are read by the thread that renders. The window, its size
/// and its scaling are remembered from the last time the surface had a
/// window, with a reference to that window: a frame that begins while the
/// surface is being destroyed still names a live window and the size its
/// surface was created with, instead of a window that is gone.
struct EglSurfaceInfo {
    surface: Arc<SurfaceShared>,
    last: Mutex<Option<(NativeWindow, PixelSize, f64)>>,
}

impl EglSurfaceInfo {
    fn current(&self) -> Option<(isize, PixelSize, f64)> {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(window) = self.surface.native_window() {
            *last = Some((window, self.surface.size(), self.surface.scaling()));
        }
        last.as_ref().map(|(window, size, scaling)| (window.handle(), *size, *scaling))
    }
}

impl IEglWindowGlPlatformSurfaceInfo for EglSurfaceInfo {
    fn handle(&self) -> isize {
        self.current().map_or(0, |(handle, _, _)| handle)
    }

    fn size(&self) -> PixelSize {
        self.current().map_or(PixelSize::default(), |(_, size, _)| size)
    }

    fn scaling(&self) -> f64 {
        self.current().map_or(0.0, |(_, _, scaling)| scaling)
    }

    fn as_info_with_wait_policy(&self) -> Option<&dyn IEglWindowGlPlatformSurfaceInfoWithWaitPolicy> {
        Some(self)
    }
}

impl IEglWindowGlPlatformSurfaceInfoWithWaitPolicy for EglSurfaceInfo {
    fn skip_waits(&self) -> bool {
        true
    }
}

/// The EGL surface of the top-level, which is ready while the surface has a
/// window. (The surface of the OpenGL crate is always ready; a render
/// target is not created for a surface that does not exist yet.)
struct EglWindowSurface {
    surface: Arc<SurfaceShared>,
    gl: Arc<EglGlPlatformSurface>,
}

impl IPlatformRenderSurface for EglWindowSurface {
    fn is_ready(&self) -> bool {
        self.surface.has_native_window()
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        self.gl.try_get_surface_kind(kind)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct TopLevelImpl {
    id: i64,
    context: JavaObject,
    keyboard_helper: AndroidKeyboardEventsHelper,
    text_input_method: Rc<AndroidInputMethod>,
    text_input_host: Rc<ViewInputMethodHost>,
    system_navigation_manager: Rc<AndroidSystemNavigationManagerImpl>,
    native_control_host: Rc<AndroidNativeControlHostImpl>,
    feedback: Rc<AndroidPlatformFeedback>,
    clipboard: Rc<Clipboard>,
    storage_provider: Option<Rc<dyn IStorageProvider>>,
    launcher: Option<Rc<AndroidLauncher>>,
    pointer_helper: AndroidMotionEventsHelper,
    insets_manager: Option<Rc<AndroidInsetsManager>>,
    screens: Rc<AndroidScreens>,
    view: RefCell<Option<Rc<InvalidationAwareSurfaceView>>>,
    transparency_level: Cell<WindowTransparencyLevel>,
    surfaces: Vec<Arc<dyn IPlatformRenderSurface>>,
    handle: Rc<dyn IPlatformHandle>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    // The two fields of the surface view class of the reference.
    old_size: Cell<Size>,
    old_scaling: Cell<f64>,
    focus_change: Callback<dyn Fn(bool)>,

    input: Callback<dyn Fn(Rc<dyn IRawInputEventArgs>)>,
    paint: Callback<dyn Fn(Rect)>,
    resized: Callback<dyn Fn(Size, WindowResizeReason)>,
    scaling_changed: Callback<dyn Fn(f64)>,
    transparency_level_changed: Callback<dyn Fn(WindowTransparencyLevel)>,
    closed: Callback<dyn Fn()>,
    lost_focus: Callback<dyn Fn()>,
}

impl TopLevelImpl {
    /// Creates the top-level of the Java view `ferro_view` in `context`,
    /// with its surface view.
    pub(crate) fn new(ferro_view: &JavaObject, context: &JavaObject, place_on_top: bool) -> Rc<TopLevelImpl> {
        let id = next_handle();
        let view = InvalidationAwareSurfaceView::new(context, id, place_on_top);
        let shared = view.shared().clone();

        let is_activity = is_instance_of(context, "android/app/Activity");
        // `context as IActivityNavigationService`: the activity of this backend the context is.
        let navigation_service = FerroActivity::from_java(context)
            .map(|activity| activity as Rc<dyn IActivityNavigationService>);
        let system_navigation_manager = AndroidSystemNavigationManagerImpl::new(navigation_service);
        let text_input_host = Rc::new(ViewInputMethodHost::new(ferro_view.clone(), context));
        let text_input_method = {
            let host: Rc<dyn IInputMethodHost> = text_input_host.clone();
            AndroidInputMethod::new(host, sdk_int())
        };

        let feedback = Rc::new(AndroidPlatformFeedback::new(Box::new(ViewFeedback::new(ferro_view.clone()))));
        let clipboard = Clipboard::new(ClipboardImpl::new(context));
        let storage_provider: Option<Rc<dyn IStorageProvider>> =
            is_activity.then(|| AndroidStorageProvider::new(context.clone()) as Rc<dyn IStorageProvider>);
        let launcher = is_activity.then(|| Rc::new(AndroidLauncher::new(context.clone())));

        let this = Rc::new_cyclic(|this: &Weak<TopLevelImpl>| {
            let native_control_host = AndroidNativeControlHostImpl::new(ferro_view.clone(), context.clone(), {
                let this = this.clone();
                Box::new(move || this.upgrade().map_or(1.0, |this| ITopLevelImpl::render_scaling(&*this)))
            });
            let keyboard_top_level: Weak<dyn IKeyboardEventsTopLevel> = this.clone();
            let motion_top_level: Weak<dyn IMotionEventsTopLevel> = this.clone();
            let insets_manager = is_activity.then(|| {
                let insets_top_level: Weak<dyn IInsetsTopLevel> = this.clone();
                let handle = next_handle();
                let manager = AndroidInsetsManager::new(
                    Box::new(ActivityInsetsWindow::new(context.clone(), handle)),
                    insets_top_level,
                );
                INSETS_MANAGERS.with(|managers| managers.borrow_mut().insert(handle, Rc::downgrade(&manager)));
                manager
            });

            let gl: Arc<dyn IPlatformRenderSurface> = Arc::new(EglWindowSurface {
                surface: shared.clone(),
                gl: EglGlPlatformSurface::new(Arc::new(EglSurfaceInfo {
                    surface: shared.clone(),
                    last: Mutex::new(None),
                })),
            });
            let framebuffer: Arc<dyn IPlatformRenderSurface> = Arc::new(FramebufferManager::new(shared.clone()));
            let native_window: Arc<dyn IPlatformRenderSurface> = Arc::new(SurfaceViewHandle::new(shared.clone()));

            TopLevelImpl {
                id,
                context: context.clone(),
                keyboard_helper: AndroidKeyboardEventsHelper::new(keyboard_top_level, sdk_int()),
                text_input_method,
                text_input_host,
                system_navigation_manager,
                native_control_host,
                feedback,
                clipboard,
                storage_provider,
                launcher,
                pointer_helper: AndroidMotionEventsHelper::new(motion_top_level),
                insets_manager,
                screens: AndroidScreens::new(context),
                handle: Rc::new(AndroidViewControlHandle::new(view.view().clone())),
                view: RefCell::new(Some(view)),
                transparency_level: Cell::new(WindowTransparencyLevel::none()),
                surfaces: vec![gl, framebuffer, native_window],
                input_root: RefCell::new(None),
                old_size: Cell::new(Size::default()),
                old_scaling: Cell::new(0.0),
                focus_change: RefCell::new(None),
                input: RefCell::new(None),
                paint: RefCell::new(None),
                resized: RefCell::new(None),
                scaling_changed: RefCell::new(None),
                transparency_level_changed: RefCell::new(None),
                closed: RefCell::new(None),
                lost_focus: RefCell::new(None),
            }
        });
        TOP_LEVELS.with(|top_levels| top_levels.borrow_mut().insert(id, Rc::downgrade(&this)));
        this
    }

    /// The top-level whose surface view calls back with `handle`.
    pub(crate) fn from_handle(handle: i64) -> Option<Rc<TopLevelImpl>> {
        TOP_LEVELS.with(|top_levels| top_levels.borrow().get(&handle).and_then(Weak::upgrade))
    }

    /// The insets manager the window reports applied insets to with `handle`.
    pub(crate) fn insets_manager_from_handle(handle: i64) -> Option<Rc<AndroidInsetsManager>> {
        INSETS_MANAGERS.with(|managers| managers.borrow().get(&handle).and_then(Weak::upgrade))
    }

    pub fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    /// The Java surface view; `None` once the top-level is disposed.
    pub fn view(&self) -> Option<JavaObject> {
        self.view.borrow().as_ref().map(|view| view.view().clone())
    }

    pub(crate) fn internal_view(&self) -> Option<Rc<InvalidationAwareSurfaceView>> {
        self.view.borrow().clone()
    }

    fn on_resized(&self, size: Size) {
        let resized = self.resized.borrow().clone();
        if let Some(resized) = resized {
            resized(size, WindowResizeReason::Unspecified);
        }
    }

    pub(crate) fn resize(&self, size: Size) {
        let resized = self.resized.borrow().clone();
        if let Some(resized) = resized {
            resized(size, WindowResizeReason::Layout);
        }
    }

    pub(crate) fn insets_manager(&self) -> Option<&Rc<AndroidInsetsManager>> {
        self.insets_manager.as_ref()
    }

    /// What the input method said the next input connection of the view is
    /// made with (the field of the view class of the reference).
    pub(crate) fn editor_info_init(&self) -> Option<InitEditorInfo> {
        self.text_input_host.editor_info_init()
    }

    pub(crate) fn keyboard_helper(&self) -> &AndroidKeyboardEventsHelper {
        &self.keyboard_helper
    }

    pub(crate) fn pointer_helper(&self) -> &AndroidMotionEventsHelper {
        &self.pointer_helper
    }

    pub(crate) fn screens(&self) -> &Rc<AndroidScreens> {
        &self.screens
    }

    /// Sets what the focus change of the surface view calls
    /// (`View.FocusChange +=`).
    pub(crate) fn set_focus_change(&self, handler: Option<Rc<dyn Fn(bool)>>) {
        *self.focus_change.borrow_mut() = handler;
    }

    // ---- the callbacks of the surface view (`SurfaceViewImpl` of the reference) ----

    pub(crate) fn on_surface_created(&self, properties: &SurfaceProperties) {
        if let Some(view) = self.internal_view() {
            view.surface_created(properties);
        }
    }

    pub(crate) fn on_surface_changed(&self, properties: &SurfaceProperties, format: i32, width: i32, height: i32) {
        let Some(view) = self.internal_view() else {
            return;
        };
        view.surface_changed(properties, format, width, height);

        let new_size = view.size().to_size(view.scaling());
        let new_scaling = view.scaling();

        if new_size != self.old_size.get() {
            self.old_size.set(new_size);
            self.on_resized(new_size);
        }
        if new_scaling != self.old_scaling.get() {
            self.old_scaling.set(new_scaling);
            let scaling_changed = self.scaling_changed.borrow().clone();
            if let Some(scaling_changed) = scaling_changed {
                scaling_changed(new_scaling);
            }
        }
    }

    pub(crate) fn on_surface_destroyed(&self) {
        if let Some(view) = self.internal_view() {
            view.surface_destroyed();
        }
    }

    pub(crate) fn on_surface_redraw_needed(&self) {
        let Some(view) = self.internal_view() else {
            return;
        };
        // The compositor renderer handles the paint event in sync, which is perfect for a
        // synchronous redraw request.
        let paint = self.paint.borrow().clone();
        if let Some(paint) = paint {
            paint(Rect::from_position_size(Point::default(), view.size().to_size(view.scaling())));
        }
        view.surface_redraw_needed();
    }

    /// `drawing_finished` is the `java.lang.Runnable` the system waits for.
    pub(crate) fn on_surface_redraw_needed_async(&self, drawing_finished: JavaObject) {
        let Some(view) = self.internal_view() else {
            crate::interop::java::call_void(&drawing_finished, "run", "()V", &[]);
            return;
        };
        self.required_compositor()
            .request_composition_update(move || crate::interop::java::call_void(&drawing_finished, "run", "()V", &[]));
        view.surface_redraw_needed_async();
    }

    pub(crate) fn on_view_focus_changed(&self, has_focus: bool) {
        let focus_change = self.focus_change.borrow().clone();
        if let Some(focus_change) = focus_change {
            focus_change(has_focus);
        }
    }

    fn required_compositor(&self) -> Rc<Compositor> {
        match AndroidPlatform::compositor() {
            Some(compositor) => compositor,
            None => panic!("Android backend wasn't initialized. Make sure use_android() was executed."),
        }
    }

    fn set_transparency_level(&self, value: WindowTransparencyLevel) {
        if self.transparency_level.get() != value {
            self.transparency_level.set(value);
            let transparency_level_changed = self.transparency_level_changed.borrow().clone();
            if let Some(transparency_level_changed) = transparency_level_changed {
                transparency_level_changed(value);
            }
        }
    }

    fn is_supported(level: WindowTransparencyLevel) -> bool {
        if level == WindowTransparencyLevel::none() {
            return true;
        }
        if level == WindowTransparencyLevel::transparent() {
            return sdk_int() >= 30;
        }
        if level == WindowTransparencyLevel::blur() {
            return sdk_int() >= 31;
        }
        false
    }

    /// Applies a transparency level to the window of the activity: 0 none,
    /// 1 transparent, 2 blur.
    fn apply_window_transparency(&self, level: i32) {
        call_static_void(
            &JavaClass::find(PLATFORM_HELPER),
            "setWindowTransparency",
            "(Landroid/app/Activity;I)V",
            &[JavaValue::Object(Some(&self.context)), JavaValue::Int(level)],
        );
    }

    /// Text the input method commits: a raw text input event at the time of
    /// the uptime clock.
    pub(crate) fn text_input(&self, text: &str) {
        let input = self.input.borrow().clone();
        if let Some(input) = input {
            let (Some(device), Some(input_root)) = (AndroidKeyboardDevice::instance(), self.input_root()) else {
                panic!("Text was input before the top-level had an input root, or without the keyboard device.");
            };
            let device: Rc<dyn IInputDevice> = device;
            let uptime_millis =
                call_static_long(&JavaClass::find("android/os/SystemClock"), "uptimeMillis", "()J", &[]);
            let args = RawTextInputEventArgs::new(device, uptime_millis as u64, input_root, text);

            input(Rc::new(args));
        }
    }

    /// The id of the display the surface view is on.
    pub(crate) fn display_id(&self) -> Option<i32> {
        let view = self.view()?;
        let id = call_static_int(
            &JavaClass::find(PLATFORM_HELPER),
            "getDisplayId",
            "(Landroid/view/View;)I",
            &[JavaValue::Object(Some(&view))],
        );
        (id >= 0).then_some(id)
    }
}

impl IMotionEventsTopLevel for TopLevelImpl {
    fn motion_input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root()
    }

    fn dispatch_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        let input = self.input.borrow().clone();
        if let Some(input) = input {
            input(args);
        }
    }

    fn motion_render_scaling(&self) -> f64 {
        self.render_scaling()
    }
}

impl IKeyboardEventsTopLevel for TopLevelImpl {
    fn key_input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root()
    }

    fn dispatch_key_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        let input = self.input.borrow().clone();
        if let Some(input) = input {
            input(args);
        }
    }
}

impl IInputConnectionTopLevel for TopLevelImpl {
    fn text_input(&self, text: &str) {
        TopLevelImpl::text_input(self, text);
    }

    fn try_move_focus_next(&self) {
        if let Some(focus_manager) = self.input_root().and_then(|input_root| input_root.focus_manager()) {
            focus_manager.try_move_focus(NavigationDirection::Next, None);
        }
    }

    fn uptime_millis(&self) -> i64 {
        call_static_long(&JavaClass::find("android/os/SystemClock"), "uptimeMillis", "()J", &[])
    }

    fn get_caps_mode(&self, text: &str, off: i32, req_modes: i32) -> i32 {
        call_static_int(
            &JavaClass::find("android/text/TextUtils"),
            "getCapsMode",
            "(Ljava/lang/CharSequence;II)I",
            &[JavaValue::String(text), JavaValue::Int(off), JavaValue::Int(req_modes)],
        )
    }
}

impl IInsetsTopLevel for TopLevelImpl {
    fn top_level_render_scaling(&self) -> f64 {
        self.render_scaling()
    }

    fn top_level_client_size(&self) -> Size {
        self.client_size()
    }
}

impl IDisposable for TopLevelImpl {
    fn dispose(&self) {
        self.system_navigation_manager.dispose();
        self.pointer_helper.dispose();
        let view = self.view.borrow_mut().take();
        if let Some(view) = view {
            view.dispose();
        }
        TOP_LEVELS.with(|top_levels| top_levels.borrow_mut().remove(&self.id));
    }
}

impl IOptionalFeatureProvider for TopLevelImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn ITextInputMethodImpl>() {
            let text_input_method: Rc<dyn ITextInputMethodImpl> = self.text_input_method.clone();
            return Some(Rc::new(text_input_method));
        }

        if feature_type == TypeId::of::<dyn ISystemNavigationManagerImpl>() {
            let system_navigation_manager: Rc<dyn ISystemNavigationManagerImpl> = self.system_navigation_manager.clone();
            return Some(Rc::new(system_navigation_manager));
        }

        if feature_type == TypeId::of::<dyn IStorageProvider>() {
            return Some(Rc::new(self.storage_provider.clone()?));
        }

        if feature_type == TypeId::of::<dyn IClipboard>() {
            let clipboard: Rc<dyn IClipboard> = self.clipboard.clone();
            return Some(Rc::new(clipboard));
        }

        if feature_type == TypeId::of::<dyn ILauncher>() {
            let launcher: Rc<dyn ILauncher> = self.launcher.clone()?;
            return Some(Rc::new(launcher));
        }

        if feature_type == TypeId::of::<dyn INativeControlHostImpl>() {
            let native_control_host: Rc<dyn INativeControlHostImpl> = self.native_control_host.clone();
            return Some(Rc::new(native_control_host));
        }

        if feature_type == TypeId::of::<dyn IPlatformFeedback>() {
            let feedback: Rc<dyn IPlatformFeedback> = self.feedback.clone();
            return Some(Rc::new(feedback));
        }

        if feature_type == TypeId::of::<dyn IInsetsManager>() {
            let insets_manager: Rc<dyn IInsetsManager> = self.insets_manager.clone()?;
            return Some(Rc::new(insets_manager));
        }

        if feature_type == TypeId::of::<dyn IInputPane>() {
            let input_pane: Rc<dyn IInputPane> = self.insets_manager.clone()?;
            return Some(Rc::new(input_pane));
        }

        if feature_type == TypeId::of::<dyn IScreenImpl>() {
            let screens: Rc<dyn IScreenImpl> = self.screens.clone();
            return Some(Rc::new(screens));
        }

        None
    }
}

impl ITopLevelImpl for TopLevelImpl {
    fn desktop_scaling(&self) -> f64 {
        self.render_scaling()
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        Some(self.handle.clone())
    }

    fn client_size(&self) -> Size {
        self.internal_view().map_or(Size::default(), |view| view.size().to_size(view.scaling()))
    }

    fn render_scaling(&self) -> f64 {
        self.internal_view().map_or(1.0, |view| view.scaling())
    }

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        self.surfaces.clone()
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        Some(self.required_compositor())
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        self.input.borrow().clone()
    }

    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
        *self.input.borrow_mut() = value;
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        self.paint.borrow().clone()
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        *self.paint.borrow_mut() = value;
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        self.resized.borrow().clone()
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        *self.resized.borrow_mut() = value;
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        self.scaling_changed.borrow().clone()
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        *self.scaling_changed.borrow_mut() = value;
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        self.transparency_level_changed.borrow().clone()
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        *self.transparency_level_changed.borrow_mut() = value;
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        *self.input_root.borrow_mut() = Some(input_root);
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        point.to_point(self.render_scaling())
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::from_point(point, self.render_scaling())
    }

    fn set_cursor(&self, _cursor: Option<Rc<dyn ICursorImpl>>) {
        //still not implemented
    }

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        self.closed.borrow().clone()
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        *self.closed.borrow_mut() = value;
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        self.lost_focus.borrow().clone()
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        *self.lost_focus.borrow_mut() = value;
    }

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        None
    }

    fn set_transparency_level_hint(&self, transparency_levels: &[WindowTransparencyLevel]) {
        if self.view.borrow().is_none() || !is_instance_of(&self.context, FERRO_ACTIVITY) {
            return;
        }

        for level in transparency_levels {
            if !Self::is_supported(*level) {
                continue;
            }

            if *level == self.transparency_level.get() {
                return;
            }

            if *level == WindowTransparencyLevel::none() {
                self.apply_window_transparency(0);
            } else if *level == WindowTransparencyLevel::transparent() {
                if sdk_int() >= 30 {
                    self.apply_window_transparency(1);
                }
            } else if *level == WindowTransparencyLevel::blur() && sdk_int() >= 31 {
                self.apply_window_transparency(2);
            }

            self.set_transparency_level(*level);
            return;
        }

        // If we get here, we didn't find a supported level. Use the default of None.
        self.apply_window_transparency(0);
    }

    fn transparency_level(&self) -> WindowTransparencyLevel {
        self.transparency_level.get()
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        AcrylicPlatformCompensationLevels::new(1.0, 1.0, 1.0)
    }

    fn set_frame_theme_variant(&self, theme_variant: Option<PlatformThemeVariant>) {
        if let Some(insets_manager) = &self.insets_manager {
            insets_manager.set_system_bar_theme(match theme_variant {
                Some(PlatformThemeVariant::Light) => Some(SystemBarTheme::Light),
                Some(PlatformThemeVariant::Dark) => Some(SystemBarTheme::Dark),
                None => None,
            });
        }

        // The reference also sets the local night mode of an AppCompat activity here. The
        // activities of the port are activities of the platform, which have no local night
        // mode (docs/porting/android-platform.md, section 3.3).
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
