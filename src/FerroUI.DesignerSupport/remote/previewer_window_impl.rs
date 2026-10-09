use crate::remote::stubs::try_get_stub_feature;
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::{IInputRoot, PointerPressedEventArgs};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::threading::DispatcherPriority;
use ferroui_base::{PixelPoint, Point, Rect, Size, Thickness};
use ferroui_controls::platform::{
    IPlatformHandle, IPopupImpl, ITopLevelImpl, IWindowBaseImpl, IWindowIconImpl, IWindowImpl,
    PlatformRequestedDrawnDecoration, PlatformThemeVariant,
};
use ferroui_controls::remote::server::{
    OnMessageOverride, RemoteServerTopLevelImpl, RemoteServerTopLevelImplOverrides, RemoteServerUiThreadHandle,
};
use ferroui_controls::{
    AcrylicPlatformCompensationLevels, WindowCloseReason, WindowDecorations, WindowEdge, WindowResizeReason,
    WindowState, WindowTransparencyLevel,
};
use ferroui_remote_protocol::viewport::{ClientViewportAllocatedMessage, RequestViewportResizeMessage};
use ferroui_remote_protocol::{IFerroRemoteTransportConnection, Message};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// The window of the previewed document: a remote server top-level that is
/// a window. It sizes itself (the size of the viewport of the client is
/// ignored) and asks the client to resize its viewport.
pub(crate) struct PreviewerWindowImpl {
    base: Rc<RemoteServerTopLevelImpl>,
    transport: Arc<dyn IFerroRemoteTransportConnection>,
    position: Cell<PixelPoint>,
    position_changed: RefCell<Option<Rc<dyn Fn(PixelPoint)>>>,
    deactivated: RefCell<Option<Rc<dyn Fn()>>>,
    activated: RefCell<Option<Rc<dyn Fn()>>>,
    closing: RefCell<Option<Rc<dyn Fn(WindowCloseReason) -> bool>>>,
    window_state: Cell<WindowState>,
    window_state_changed: RefCell<Option<Rc<dyn Fn(WindowState)>>>,
    got_input_when_disabled: RefCell<Option<Rc<dyn Fn()>>>,
    extend_client_area_to_decorations_changed: RefCell<Option<Rc<dyn Fn(bool)>>>,
}

/// The members of the remote server top-level this class overrides on the
/// UI thread.
struct Overrides;

impl RemoteServerTopLevelImplOverrides for Overrides {
    fn desktop_scaling(&self, _base: &RemoteServerTopLevelImpl) -> f64 {
        1.0
    }

    fn try_get_feature(&self, _base: &RemoteServerTopLevelImpl, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        try_get_stub_feature(feature_type)
    }
}

impl PreviewerWindowImpl {
    pub(crate) fn new(transport: Arc<dyn IFerroRemoteTransportConnection>) -> Rc<PreviewerWindowImpl> {
        // `OnMessage`: runs on the reader thread of the connection (or on
        // the UI thread, for the messages the windowing platform replays to
        // a new window); what it does to the window is posted to the UI
        // thread with the number read from the message.
        let on_message: OnMessageOverride = Arc::new(
            |this: &RemoteServerUiThreadHandle, _transport: &dyn IFerroRemoteTransportConnection, obj: &Message| {
            // In previewer mode we completely ignore client-side viewport size
            if let Some(alloc) = obj.downcast_ref::<ClientViewportAllocatedMessage>() {
                let dpi_x = alloc.dpi_x;
                this.post(
                    move |this| {
                        if let Some(this) = this.upgrade() {
                            this.set_render_scaling(dpi_x / 96.0);
                            this.render_and_send_frame_if_needed();
                        }
                    },
                    DispatcherPriority::DEFAULT,
                );
                return true;
            }
            false
        },
        );
        let base = RemoteServerTopLevelImpl::with_overrides(transport.clone(), Some(Rc::new(Overrides)), Some(on_message));
        base.set_client_size(Size::new(1.0, 1.0));
        Rc::new(PreviewerWindowImpl {
            base,
            transport,
            position: Cell::new(PixelPoint::default()),
            position_changed: RefCell::new(None),
            deactivated: RefCell::new(None),
            activated: RefCell::new(None),
            closing: RefCell::new(None),
            window_state: Cell::new(WindowState::default()),
            window_state_changed: RefCell::new(None),
            got_input_when_disabled: RefCell::new(None),
            extend_client_area_to_decorations_changed: RefCell::new(None),
        })
    }

    /// The remote server top-level this class derives from.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn base(&self) -> &Rc<RemoteServerTopLevelImpl> {
        &self.base
    }

    /// `RenderScaling = value` (the setter of the offscreen implementation).
    pub(crate) fn set_render_scaling(&self, value: f64) {
        self.base.set_render_scaling(value)
    }

    #[allow(dead_code)]
    pub(crate) fn set_position(&self, value: PixelPoint) {
        self.position.set(value)
    }

    fn top_level_impl(&self) -> &dyn ITopLevelImpl {
        &**self.base.base()
    }
}

impl IOptionalFeatureProvider for PreviewerWindowImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.base.base().try_get_feature(feature_type)
    }
}

impl IDisposable for PreviewerWindowImpl {
    fn dispose(&self) {
        self.base.base().dispose()
    }
}

// The members of the top-level are the ones of the base class.
impl ITopLevelImpl for PreviewerWindowImpl {
    fn desktop_scaling(&self) -> f64 {
        self.top_level_impl().desktop_scaling()
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        self.top_level_impl().handle()
    }

    fn client_size(&self) -> Size {
        self.top_level_impl().client_size()
    }

    fn render_scaling(&self) -> f64 {
        self.top_level_impl().render_scaling()
    }

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        self.top_level_impl().surfaces()
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        self.top_level_impl().compositor()
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        self.top_level_impl().input()
    }

    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
        self.top_level_impl().set_input(value)
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        self.top_level_impl().paint()
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        self.top_level_impl().set_paint(value)
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        self.top_level_impl().resized()
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        self.top_level_impl().set_resized(value)
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        self.top_level_impl().scaling_changed()
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        self.top_level_impl().set_scaling_changed(value)
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        self.top_level_impl().transparency_level_changed()
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        self.top_level_impl().set_transparency_level_changed(value)
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        self.top_level_impl().set_input_root(input_root)
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        self.top_level_impl().point_to_client(point)
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        self.top_level_impl().point_to_screen(point)
    }

    fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
        self.top_level_impl().set_cursor(cursor)
    }

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        self.top_level_impl().closed()
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        self.top_level_impl().set_closed(value)
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        self.top_level_impl().lost_focus()
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        self.top_level_impl().set_lost_focus(value)
    }

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        self.top_level_impl().create_popup()
    }

    fn set_transparency_level_hint(&self, transparency_levels: &[WindowTransparencyLevel]) {
        self.top_level_impl().set_transparency_level_hint(transparency_levels)
    }

    fn transparency_level(&self) -> WindowTransparencyLevel {
        self.top_level_impl().transparency_level()
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        self.top_level_impl().acrylic_compensation_levels()
    }

    fn set_frame_theme_variant(&self, theme_variant: Option<PlatformThemeVariant>) {
        self.top_level_impl().set_frame_theme_variant(theme_variant)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_window_base_impl(&self) -> Option<&dyn IWindowBaseImpl> {
        Some(self)
    }

    fn as_window_impl(&self) -> Option<&dyn IWindowImpl> {
        Some(self)
    }
}

impl IWindowBaseImpl for PreviewerWindowImpl {
    fn frame_size(&self) -> Option<Size> {
        self.base.base().frame_size()
    }

    fn show(&self, _activate: bool, _is_dialog: bool) {}

    fn hide(&self) {}

    fn position(&self) -> PixelPoint {
        self.position.get()
    }

    fn position_changed(&self) -> Option<Rc<dyn Fn(PixelPoint)>> {
        self.position_changed.borrow().clone()
    }

    fn set_position_changed(&self, value: Option<Rc<dyn Fn(PixelPoint)>>) {
        *self.position_changed.borrow_mut() = value;
    }

    fn activate(&self) {}

    fn deactivated(&self) -> Option<Rc<dyn Fn()>> {
        self.deactivated.borrow().clone()
    }

    fn set_deactivated(&self, value: Option<Rc<dyn Fn()>>) {
        *self.deactivated.borrow_mut() = value;
    }

    fn activated(&self) -> Option<Rc<dyn Fn()>> {
        self.activated.borrow().clone()
    }

    fn set_activated(&self, value: Option<Rc<dyn Fn()>>) {
        *self.activated.borrow_mut() = value;
    }

    fn max_auto_size_hint(&self) -> Size {
        Size::new(4096.0, 4096.0)
    }

    fn set_topmost(&self, _value: bool) {}
}

impl IWindowImpl for PreviewerWindowImpl {
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
        self.window_state_changed.borrow().clone()
    }

    fn set_window_state_changed(&self, value: Option<Rc<dyn Fn(WindowState)>>) {
        *self.window_state_changed.borrow_mut() = value;
    }

    fn set_title(&self, _title: Option<&str>) {}

    fn set_parent(&self, _parent: Option<Rc<dyn IWindowImpl>>) {}

    fn set_enabled(&self, _enable: bool) {}

    fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>> {
        self.got_input_when_disabled.borrow().clone()
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
        self.closing.borrow().clone()
    }

    fn set_closing(&self, value: Option<Rc<dyn Fn(WindowCloseReason) -> bool>>) {
        *self.closing.borrow_mut() = value;
    }

    fn is_client_area_extended_to_decorations(&self) -> bool {
        false
    }

    fn extend_client_area_to_decorations_changed(&self) -> Option<Rc<dyn Fn(bool)>> {
        self.extend_client_area_to_decorations_changed.borrow().clone()
    }

    fn set_extend_client_area_to_decorations_changed(&self, value: Option<Rc<dyn Fn(bool)>>) {
        *self.extend_client_area_to_decorations_changed.borrow_mut() = value;
    }

    fn needs_managed_decorations(&self) -> bool {
        false
    }

    fn requested_drawn_decorations(&self) -> PlatformRequestedDrawnDecoration {
        PlatformRequestedDrawnDecoration::NONE
    }

    fn extended_margins(&self) -> Thickness {
        Thickness::default()
    }

    fn off_screen_margin(&self) -> Thickness {
        Thickness::default()
    }

    fn begin_move_drag(&self, _e: &PointerPressedEventArgs) {}

    fn begin_resize_drag(&self, _edge: WindowEdge, _e: &PointerPressedEventArgs) {}

    fn resize(&self, client_size: Size, _reason: WindowResizeReason) {
        // Don't let it clientSize be unconstrained or risk running Out Of Memory
        let max_auto_size_hint = self.max_auto_size_hint();
        let client_size = Size::new(
            client_size.width.min(max_auto_size_hint.width),
            client_size.height.min(max_auto_size_hint.height),
        );

        let render_scaling = self.base.render_scaling();
        self.transport.send(Arc::new(RequestViewportResizeMessage {
            width: (client_size.width * render_scaling).ceil(),
            height: (client_size.height * render_scaling).ceil(),
        }));
        self.base.set_client_size(client_size);
        self.base.render_and_send_frame_if_needed();
    }

    fn move_(&self, _point: PixelPoint) {}

    fn set_min_max_size(&self, _min_size: Size, _max_size: Size) {}

    fn set_extend_client_area_to_decorations_hint(&self, _extend_into_client_area_hint: bool) {}

    fn set_extend_client_area_title_bar_height_hint(&self, _title_bar_height: f64) {}
}
