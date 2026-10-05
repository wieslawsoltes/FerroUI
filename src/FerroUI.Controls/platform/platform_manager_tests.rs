use super::*;
use crate::{
    AcrylicPlatformCompensationLevels, WindowCloseReason, WindowDecorations, WindowEdge, WindowResizeReason,
    WindowState, WindowTransparencyLevel,
};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::{IInputRoot, PointerPressedEventArgs};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroLocator, PixelPoint, Point, Rect, Size, Thickness};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A window implementation that records what the tests need.
#[derive(Default)]
pub(super) struct TestWindowImpl {
    pub embeddable: bool,
    pub is_window: Cell<bool>,
    pub position: Cell<PixelPoint>,
    pub client_size: Cell<Size>,
    pub window_state: Cell<WindowState>,
    pub disposed: Cell<bool>,
    pub input_pane: RefCell<Option<Rc<dyn IInputPane>>>,
    paint: RefCell<Option<Rc<dyn Fn(Rect)>>>,
    resized: RefCell<Option<Rc<dyn Fn(Size, WindowResizeReason)>>>,
    closed: RefCell<Option<Rc<dyn Fn()>>>,
    closing: RefCell<Option<Rc<dyn Fn(WindowCloseReason) -> bool>>>,
}

impl TestWindowImpl {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { is_window: Cell::new(true), ..Self::default() })
    }

    fn new_embeddable() -> Rc<Self> {
        Rc::new(Self { embeddable: true, is_window: Cell::new(true), ..Self::default() })
    }
}

impl IOptionalFeatureProvider for TestWindowImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IInputPane>() {
            let input_pane = self.input_pane.borrow().clone()?;
            return Some(Rc::new(input_pane));
        }

        None
    }
}

impl IDisposable for TestWindowImpl {
    fn dispose(&self) {
        self.disposed.set(true);
    }
}

impl ITopLevelImpl for TestWindowImpl {
    fn desktop_scaling(&self) -> f64 {
        1.0
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        Some(Rc::new(PlatformHandle::new(1, Some("Test"))))
    }

    fn client_size(&self) -> Size {
        self.client_size.get()
    }

    fn render_scaling(&self) -> f64 {
        1.0
    }

    fn surfaces(&self) -> Vec<Rc<dyn IPlatformRenderSurface>> {
        Vec::new()
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        None
    }

    fn set_input(&self, _value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {}

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
        None
    }

    fn set_scaling_changed(&self, _value: Option<Rc<dyn Fn(f64)>>) {}

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        None
    }

    fn set_transparency_level_changed(&self, _value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {}

    fn set_input_root(&self, _input_root: Rc<dyn IInputRoot>) {}

    fn point_to_client(&self, point: PixelPoint) -> Point {
        Point::new(f64::from(point.x - self.position.get().x), f64::from(point.y - self.position.get().y))
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::new(point.x as i32 + self.position.get().x, point.y as i32 + self.position.get().y)
    }

    fn set_cursor(&self, _cursor: Option<Rc<dyn ICursorImpl>>) {}

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        self.closed.borrow().clone()
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        *self.closed.borrow_mut() = value;
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        None
    }

    fn set_lost_focus(&self, _value: Option<Rc<dyn Fn()>>) {}

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        None
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
        if self.is_window.get() {
            Some(self)
        } else {
            None
        }
    }
}

impl IWindowBaseImpl for TestWindowImpl {
    fn frame_size(&self) -> Option<Size> {
        None
    }

    fn show(&self, _activate: bool, _is_dialog: bool) {}

    fn hide(&self) {}

    fn position(&self) -> PixelPoint {
        self.position.get()
    }

    fn position_changed(&self) -> Option<Rc<dyn Fn(PixelPoint)>> {
        None
    }

    fn set_position_changed(&self, _value: Option<Rc<dyn Fn(PixelPoint)>>) {}

    fn activate(&self) {}

    fn deactivated(&self) -> Option<Rc<dyn Fn()>> {
        None
    }

    fn set_deactivated(&self, _value: Option<Rc<dyn Fn()>>) {}

    fn activated(&self) -> Option<Rc<dyn Fn()>> {
        None
    }

    fn set_activated(&self, _value: Option<Rc<dyn Fn()>>) {}

    fn max_auto_size_hint(&self) -> Size {
        Size::new(f64::INFINITY, f64::INFINITY)
    }

    fn set_topmost(&self, _value: bool) {}
}

impl IWindowImpl for TestWindowImpl {
    fn window_state(&self) -> WindowState {
        self.window_state.get()
    }

    fn set_window_state(&self, value: WindowState) {
        self.window_state.set(value);
    }

    fn window_state_getter_is_usable(&self) -> bool {
        true
    }

    fn window_state_changed(&self) -> Option<Rc<dyn Fn(WindowState)>> {
        None
    }

    fn set_window_state_changed(&self, _value: Option<Rc<dyn Fn(WindowState)>>) {}

    fn set_title(&self, _title: Option<&str>) {}

    fn set_parent(&self, _parent: Option<Rc<dyn IWindowImpl>>) {}

    fn set_enabled(&self, _enable: bool) {}

    fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>> {
        None
    }

    fn set_got_input_when_disabled(&self, _value: Option<Rc<dyn Fn()>>) {}

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
        None
    }

    fn set_extend_client_area_to_decorations_changed(&self, _value: Option<Rc<dyn Fn(bool)>>) {}

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

    fn resize(&self, client_size: Size, reason: WindowResizeReason) {
        self.client_size.set(client_size);
        let resized = self.resized.borrow().clone();
        if let Some(resized) = resized {
            resized(client_size, reason);
        }
    }

    fn move_(&self, point: PixelPoint) {
        self.position.set(point);
    }

    fn set_min_max_size(&self, _min_size: Size, _max_size: Size) {}

    fn set_extend_client_area_to_decorations_hint(&self, _extend_into_client_area_hint: bool) {}

    fn set_extend_client_area_title_bar_height_hint(&self, _title_bar_height: f64) {}
}

struct TestTrayIcon;

impl IDisposable for TestTrayIcon {
    fn dispose(&self) {}
}

impl ITrayIconImpl for TestTrayIcon {
    fn set_icon(&self, _icon: Option<Rc<dyn IWindowIconImpl>>) {}

    fn set_tool_tip_text(&self, _text: Option<&str>) {}

    fn set_is_visible(&self, _visible: bool) {}

    fn menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>> {
        None
    }

    fn on_clicked(&self) -> Option<Rc<dyn Fn()>> {
        None
    }

    fn set_on_clicked(&self, _value: Option<Rc<dyn Fn()>>) {}
}

struct TestWindowingPlatform;

impl IWindowingPlatform for TestWindowingPlatform {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        TestWindowImpl::new()
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        TestWindowImpl::new_embeddable()
    }

    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        TestWindowImpl::new_embeddable()
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        Some(Rc::new(TestTrayIcon))
    }

    fn get_windows_z_order(&self, windows: &[Rc<dyn IWindowImpl>], z_order: &mut [i64]) {
        for (index, z) in z_order.iter_mut().enumerate().take(windows.len()) {
            *z = index as i64;
        }
    }
}

fn is_embeddable(top_level: &dyn ITopLevelImpl) -> bool {
    top_level.as_any().downcast_ref::<TestWindowImpl>().unwrap().embeddable
}

#[test]
fn creates_through_the_registered_windowing_platform() {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IWindowingPlatform>().to_constant(Rc::new(TestWindowingPlatform));

    assert!(!is_embeddable(&*PlatformManager::create_window()));
    assert!(is_embeddable(&*PlatformManager::create_embeddable_window()));
    assert!(is_embeddable(&*PlatformManager::create_embeddable_top_level()));
    assert!(PlatformManager::create_tray_icon().is_some());

    scope.dispose();
}

#[test]
fn designer_mode_creates_embeddable_windows_and_no_tray_icons() {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IWindowingPlatform>().to_constant(Rc::new(TestWindowingPlatform));

    let designer_mode = PlatformManager::designer_mode();
    assert!(is_embeddable(&*PlatformManager::create_window()));
    assert!(PlatformManager::create_tray_icon().is_none());

    designer_mode.dispose();
    assert!(!is_embeddable(&*PlatformManager::create_window()));
    assert!(PlatformManager::create_tray_icon().is_some());

    scope.dispose();
}

#[test]
fn create_tray_icon_is_none_without_a_windowing_platform() {
    let scope = FerroLocator::enter_scope();
    assert!(PlatformManager::create_tray_icon().is_none());
    scope.dispose();
}

#[test]
#[should_panic(expected = "Unable to locate")]
fn create_window_panics_without_a_windowing_platform() {
    let _scope = FerroLocator::enter_scope();
    PlatformManager::create_window();
}

#[test]
fn callback_members_round_trip_and_defaults_apply() {
    let window = TestWindowImpl::new();
    let window: Rc<dyn IWindowImpl> = window;

    let resized = Rc::new(Cell::new(None));
    let r = resized.clone();
    window.set_resized(Some(Rc::new(move |size, reason| r.set(Some((size, reason))))));
    window.resize(Size::new(10.0, 20.0), WindowResizeReason::Application);
    assert_eq!(resized.get(), Some((Size::new(10.0, 20.0), WindowResizeReason::Application)));
    window.set_resized(None);
    assert!(window.resized().is_none());

    window.set_closing(Some(Rc::new(|reason| reason == WindowCloseReason::WindowClosing)));
    let closing = window.closing().unwrap();
    assert!(closing(WindowCloseReason::WindowClosing));
    assert!(!closing(WindowCloseReason::OSShutdown));

    // Members with a reference default.
    assert_eq!(window.allowed_window_actions(), PlatformAllowedWindowActions::ALL);
    assert!(window.allowed_window_actions_changed().is_none());
    assert!(window.drawn_decorations_request_changed().is_none());
    assert!(window.platform_specific_scene_info().is_none());
    assert!(window.platform_specific_scene_info_changed().is_none());
    window.set_shadow_extents(Thickness::default());

    window.dispose();
    assert!(window.as_any().downcast_ref::<TestWindowImpl>().unwrap().disposed.get());
}

#[test]
fn optional_features_are_queried_through_the_feature_provider() {
    let window = TestWindowImpl::new();
    let top_level: Rc<dyn ITopLevelImpl> = window.clone();

    let features: &dyn IOptionalFeatureProvider = &*top_level;
    assert!(features.try_get::<dyn IInputPane>().is_none());

    let input_pane: Rc<dyn IInputPane> = Rc::new(InputPaneBase::new());
    *window.input_pane.borrow_mut() = Some(input_pane.clone());
    let found = features.try_get::<dyn IInputPane>().unwrap();
    assert!(Rc::ptr_eq(&found, &input_pane));
    assert!(features.try_get::<dyn IInsetsManager>().is_none());
}

#[test]
fn top_levels_downcast_to_the_window_contracts() {
    let window = TestWindowImpl::new();
    let top_level: Rc<dyn ITopLevelImpl> = window.clone();

    assert!(top_level.as_window_base_impl().is_some());
    assert!(top_level.as_popup_impl().is_none());
    let as_window = top_level.as_window_impl().unwrap();
    as_window.set_window_state(WindowState::Maximized);
    assert_eq!(window.window_state.get(), WindowState::Maximized);

    let platform = TestWindowingPlatform;
    let windows: Vec<Rc<dyn IWindowImpl>> = vec![TestWindowImpl::new(), TestWindowImpl::new()];
    let mut z_order = [0i64; 2];
    platform.get_windows_z_order(&windows, &mut z_order);
    assert_eq!(z_order, [0, 1]);
}
