use ferroui_base::input::platform::{ClipboardError, IClipboard};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::{
    IAsyncDataTransfer, IInputRoot, IMouseDevice, LocalBoxFuture, MouseDevice, PointerPressedEventArgs,
    StandardCursorType,
};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::MediaContext;
use ferroui_base::platform::storage::{IStorageProvider, NoopStorageProvider};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorFactory, ICursorImpl, IOptionalFeatureProvider, SharedBitmapImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::{IRenderTimer, RenderLoop, RenderTimerTick};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelPoint, PixelRect, Point, Rect, Size, Thickness};
use ferroui_controls::platform::{
    IPlatformHandle, IPlatformIconLoader, IPopupImpl, IScreenImpl, ITopLevelImpl, IWindowBaseImpl, IWindowIconImpl,
    IWindowImpl, PlatformHandle, PlatformRequestedDrawnDecoration, PlatformScreen, PlatformThemeVariant, ScreensBase,
    ScreensBaseImpl,
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
use std::io;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};

/// The storage provider and the screens of a window of the previewer: what
/// `TryGetFeature` of the window stub and of the previewer window answers.
pub(crate) fn try_get_stub_feature(feature_type: TypeId) -> Option<Rc<dyn Any>> {
    if feature_type == TypeId::of::<dyn IStorageProvider>() {
        let provider: Rc<dyn IStorageProvider> = Rc::new(NoopStorageProvider);
        return Some(Rc::new(provider));
    }

    if feature_type == TypeId::of::<dyn IScreenImpl>() {
        let screen: Rc<dyn IScreenImpl> = ScreenStub::new();
        return Some(Rc::new(screen));
    }

    None
}

/// A callback property of a platform contract: `Action? Name { get; set; }`.
type Callback<T> = RefCell<Option<Rc<T>>>;

fn get<T: ?Sized>(callback: &Callback<T>) -> Option<Rc<T>> {
    callback.borrow().clone()
}

/// A window that does nothing: what the windowing platform of the previewer
/// creates for every window but the previewed one, and for popups.
pub(crate) struct WindowStub {
    this: Weak<WindowStub>,
    deactivated: Callback<dyn Fn()>,
    activated: Callback<dyn Fn()>,
    input: Callback<dyn Fn(Rc<dyn IRawInputEventArgs>)>,
    paint: Callback<dyn Fn(Rect)>,
    resized: Callback<dyn Fn(Size, WindowResizeReason)>,
    scaling_changed: Callback<dyn Fn(f64)>,
    closing: Callback<dyn Fn(WindowCloseReason) -> bool>,
    closed: Callback<dyn Fn()>,
    lost_focus: Callback<dyn Fn()>,
    mouse_device: Rc<MouseDevice>,
    position: Cell<PixelPoint>,
    position_changed: Callback<dyn Fn(PixelPoint)>,
    window_state: Cell<WindowState>,
    window_state_changed: Callback<dyn Fn(WindowState)>,
    transparency_level_changed: Callback<dyn Fn(WindowTransparencyLevel)>,
    extend_client_area_to_decorations_changed: Callback<dyn Fn(bool)>,
    compositor: Rc<Compositor>,
    popup_positioner: Option<Rc<dyn IPopupPositioner>>,
    got_input_when_disabled: Callback<dyn Fn()>,
}

#[derive(Default)]
struct DummyRenderTimer {
    tick: Mutex<Option<RenderTimerTick>>,
}

impl IRenderTimer for DummyRenderTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.tick.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        *self.tick.lock().unwrap_or_else(PoisonError::into_inner) = value;
    }

    fn runs_in_background(&self) -> bool {
        false
    }
}

impl WindowStub {
    /// `new WindowStub(parent)`: with a parent the stub is a popup, with a
    /// positioner that positions it on the screens of the parent.
    pub(crate) fn new(parent: Option<Rc<dyn ITopLevelImpl>>) -> Rc<WindowStub> {
        Rc::new_cyclic(|this: &Weak<WindowStub>| {
            let popup_positioner = parent.map(|parent| {
                let this = this.clone();
                let helper = ManagedPopupPositionerPopupImplHelper::new(
                    parent,
                    Rc::new(move |_: PixelPoint, size: Size, _: f64| {
                        if let Some(this) = this.upgrade() {
                            this.resize(size, WindowResizeReason::Unspecified);
                        }
                    }),
                );
                let positioner: Rc<dyn IPopupPositioner> = Rc::new(ManagedPopupPositioner::new(Rc::new(helper)));
                positioner
            });
            let timer: Arc<dyn IRenderTimer> = Arc::new(DummyRenderTimer::default());
            // `new Compositor(RenderLoop.FromTimer(new DummyRenderTimer()), null)`.
            let compositor = Compositor::with_scheduler(
                RenderLoop::from_timer(timer),
                None,
                false,
                &MediaContext::instance().scheduler(),
                Dispatcher::ui_thread(),
                None,
                None,
            );
            WindowStub {
                this: this.clone(),
                deactivated: RefCell::new(None),
                activated: RefCell::new(None),
                input: RefCell::new(None),
                paint: RefCell::new(None),
                resized: RefCell::new(None),
                scaling_changed: RefCell::new(None),
                closing: RefCell::new(None),
                closed: RefCell::new(None),
                lost_focus: RefCell::new(None),
                mouse_device: MouseDevice::new(),
                position: Cell::new(PixelPoint::default()),
                position_changed: RefCell::new(None),
                window_state: Cell::new(WindowState::default()),
                window_state_changed: RefCell::new(None),
                transparency_level_changed: RefCell::new(None),
                extend_client_area_to_decorations_changed: RefCell::new(None),
                compositor,
                popup_positioner,
                got_input_when_disabled: RefCell::new(None),
            }
        })
    }

    // The members of the upstream class that no contract of the port asks
    // for.
    #[allow(dead_code)]
    pub(crate) fn mouse_device(&self) -> Rc<dyn IMouseDevice> {
        self.mouse_device.clone()
    }

    #[allow(dead_code)]
    pub(crate) fn set_position(&self, value: PixelPoint) {
        self.position.set(value)
    }

    #[allow(dead_code)]
    pub(crate) fn invalidate(&self, _rect: Rect) {}

    #[allow(dead_code)]
    pub(crate) fn show_dialog(&self, _parent: Rc<dyn IWindowImpl>) {}
}

impl IOptionalFeatureProvider for WindowStub {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        try_get_stub_feature(feature_type)
    }
}

impl IDisposable for WindowStub {
    fn dispose(&self) {}
}

impl ITopLevelImpl for WindowStub {
    fn desktop_scaling(&self) -> f64 {
        1.0
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        None
    }

    fn client_size(&self) -> Size {
        Size::default()
    }

    fn render_scaling(&self) -> f64 {
        1.0
    }

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        Vec::new()
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        Some(self.compositor.clone())
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        get(&self.input)
    }

    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
        *self.input.borrow_mut() = value;
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        get(&self.paint)
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        *self.paint.borrow_mut() = value;
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        get(&self.resized)
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        *self.resized.borrow_mut() = value;
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        get(&self.scaling_changed)
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        *self.scaling_changed.borrow_mut() = value;
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        get(&self.transparency_level_changed)
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        *self.transparency_level_changed.borrow_mut() = value;
    }

    fn set_input_root(&self, _input_root: Rc<dyn IInputRoot>) {}

    fn point_to_client(&self, point: PixelPoint) -> Point {
        point.to_point(1.0)
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::from_point(point, 1.0)
    }

    fn set_cursor(&self, _cursor: Option<Rc<dyn ICursorImpl>>) {}

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.closed)
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        *self.closed.borrow_mut() = value;
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.lost_focus)
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        *self.lost_focus.borrow_mut() = value;
    }

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        let parent: Rc<dyn ITopLevelImpl> = self.this.upgrade()?;
        let popup: Rc<dyn IPopupImpl> = WindowStub::new(Some(parent));
        Some(popup)
    }

    fn set_transparency_level_hint(&self, _transparency_levels: &[WindowTransparencyLevel]) {}

    fn transparency_level(&self) -> WindowTransparencyLevel {
        WindowTransparencyLevel::none()
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        AcrylicPlatformCompensationLevels::new(1.0, 1.0, 1.0)
    }

    fn set_frame_theme_variant(&self, _theme_variant: Option<PlatformThemeVariant>) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

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

impl IWindowBaseImpl for WindowStub {
    fn frame_size(&self) -> Option<Size> {
        None
    }

    fn show(&self, _activate: bool, _is_dialog: bool) {}

    fn hide(&self) {}

    fn position(&self) -> PixelPoint {
        self.position.get()
    }

    fn position_changed(&self) -> Option<Rc<dyn Fn(PixelPoint)>> {
        get(&self.position_changed)
    }

    fn set_position_changed(&self, value: Option<Rc<dyn Fn(PixelPoint)>>) {
        *self.position_changed.borrow_mut() = value;
    }

    fn activate(&self) {}

    fn deactivated(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.deactivated)
    }

    fn set_deactivated(&self, value: Option<Rc<dyn Fn()>>) {
        *self.deactivated.borrow_mut() = value;
    }

    fn activated(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.activated)
    }

    fn set_activated(&self, value: Option<Rc<dyn Fn()>>) {
        *self.activated.borrow_mut() = value;
    }

    fn max_auto_size_hint(&self) -> Size {
        Size::default()
    }

    fn set_topmost(&self, _value: bool) {}
}

impl IWindowImpl for WindowStub {
    fn window_state(&self) -> WindowState {
        self.window_state.get()
    }

    fn set_window_state(&self, value: WindowState) {
        self.window_state.set(value)
    }

    fn window_state_getter_is_usable(&self) -> bool {
        false
    }

    fn window_state_changed(&self) -> Option<Rc<dyn Fn(WindowState)>> {
        get(&self.window_state_changed)
    }

    fn set_window_state_changed(&self, value: Option<Rc<dyn Fn(WindowState)>>) {
        *self.window_state_changed.borrow_mut() = value;
    }

    fn set_title(&self, _title: Option<&str>) {}

    fn set_parent(&self, _parent: Option<Rc<dyn IWindowImpl>>) {}

    fn set_enabled(&self, _enable: bool) {}

    fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.got_input_when_disabled)
    }

    fn set_got_input_when_disabled(&self, value: Option<Rc<dyn Fn()>>) {
        *self.got_input_when_disabled.borrow_mut() = value;
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
        *self.closing.borrow_mut() = value;
    }

    fn is_client_area_extended_to_decorations(&self) -> bool {
        false
    }

    fn extend_client_area_to_decorations_changed(&self) -> Option<Rc<dyn Fn(bool)>> {
        get(&self.extend_client_area_to_decorations_changed)
    }

    fn set_extend_client_area_to_decorations_changed(&self, value: Option<Rc<dyn Fn(bool)>>) {
        *self.extend_client_area_to_decorations_changed.borrow_mut() = value;
    }

    fn needs_managed_decorations(&self) -> bool {
        false
    }

    fn requested_drawn_decorations(&self) -> PlatformRequestedDrawnDecoration {
        if self.is_client_area_extended_to_decorations() {
            PlatformRequestedDrawnDecoration::TITLE_BAR
        } else {
            PlatformRequestedDrawnDecoration::NONE
        }
    }

    fn extended_margins(&self) -> Thickness {
        Thickness::default()
    }

    fn off_screen_margin(&self) -> Thickness {
        Thickness::default()
    }

    fn begin_move_drag(&self, _e: &PointerPressedEventArgs) {}

    fn begin_resize_drag(&self, _edge: WindowEdge, _e: &PointerPressedEventArgs) {}

    fn resize(&self, _client_size: Size, _reason: WindowResizeReason) {}

    fn move_(&self, _point: PixelPoint) {}

    fn set_min_max_size(&self, _min_size: Size, _max_size: Size) {}

    fn set_extend_client_area_to_decorations_hint(&self, _extend_into_client_area_hint: bool) {}

    fn set_extend_client_area_title_bar_height_hint(&self, _title_bar_height: f64) {}
}

impl IPopupImpl for WindowStub {
    fn popup_positioner(&self) -> Option<Rc<dyn IPopupPositioner>> {
        self.popup_positioner.clone()
    }

    fn set_window_manager_add_shadow_hint(&self, _enabled: bool) {}

    fn take_focus(&self) {}

    fn set_hit_test_visible(&self, _is_hit_test_visible: bool) {}
}

// Not registered by the previewer, as in the original.
#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct ClipboardStub;

impl IClipboard for ClipboardStub {
    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn set_data_async(
        &self,
        _data_transfer: Option<Rc<dyn IAsyncDataTransfer>>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn flush_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        Box::pin(std::future::ready(Ok(None)))
    }

    fn try_get_in_process_data_async(
        &self,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        Box::pin(std::future::ready(Ok(None)))
    }
}

#[derive(Default)]
pub(crate) struct CursorFactoryStub;

struct CursorStub;

impl ICursorImpl for CursorStub {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ICursorFactory for CursorFactoryStub {
    fn get_cursor(&self, _cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorStub)
    }

    fn create_cursor(&self, _cursor: &Bitmap, _hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorStub)
    }
}

#[derive(Default)]
pub(crate) struct IconLoaderStub;

struct IconStub;

impl IWindowIconImpl for IconStub {
    fn save(&self, _output_stream: &mut dyn io::Write) -> io::Result<()> {
        Ok(())
    }
}

impl IPlatformIconLoader for IconLoaderStub {
    fn load_icon_from_file(&self, _file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub))
    }

    fn load_icon_from_stream(&self, _stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub))
    }

    fn load_icon_from_bitmap(&self, _bitmap: Arc<SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        Rc::new(IconStub)
    }
}

/// The screens of the previewer: one primary screen of 4000 by 4000.
pub(crate) struct ScreenStub {
    base: ScreensBase<i32, PlatformScreen>,
}

impl ScreenStub {
    pub(crate) fn new() -> Rc<ScreenStub> {
        Rc::new(ScreenStub { base: ScreensBase::new() })
    }
}

impl ScreensBaseImpl for ScreenStub {
    type Key = i32;
    type Screen = PlatformScreen;

    fn screens_base(&self) -> &ScreensBase<i32, PlatformScreen> {
        &self.base
    }

    fn get_all_screen_keys(&self) -> Vec<i32> {
        vec![1]
    }

    fn create_screen_from_key(&self, key: &i32) -> Rc<PlatformScreen> {
        // `PlatformScreenStub`.
        let screen = PlatformScreen::new(Rc::new(PlatformHandle::new(*key as isize, Some("ScreenStub"))));
        screen.set_scaling(1.0);
        let bounds = PixelRect::new(0, 0, 4000, 4000);
        screen.set_bounds(bounds);
        screen.set_working_area(bounds);
        screen.set_is_primary(true);
        Rc::new(screen)
    }
}
