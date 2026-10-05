use super::mock_screen::{mock_screen, MockScreenImpl};
use crate::platform::{
    IPlatformHandle, IPopupImpl, IScreenImpl, ITopLevelImpl, IWindowBaseImpl, IWindowIconImpl, IWindowImpl,
    PlatformAllowedWindowActions, PlatformRequestedDrawnDecoration, PlatformThemeVariant,
};
use crate::primitives::popup_positioning::{
    IPopupPositioner, ManagedPopupPositioner, ManagedPopupPositionerPopupImplHelper,
};
use crate::{
    AcrylicPlatformCompensationLevels, WindowCloseReason, WindowDecorations, WindowEdge, WindowResizeReason,
    WindowState, WindowTransparencyLevel,
};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::{IInputRoot, PointerPressedEventArgs};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{PixelPoint, PixelRect, Point, Rect, Size, Thickness};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// The size of the screen of the mock windowing platform.
pub const SCREEN_SIZE: Size = Size::new(1280.0, 1024.0);

/// A call made on a [`MockWindowImpl`] by the code under test.
#[derive(Clone, Debug, PartialEq)]
pub enum MockCall {
    Show { activate: bool, is_dialog: bool },
    Hide,
    Activate,
    Dispose,
    Resize(Size, WindowResizeReason),
    Move(PixelPoint),
    SetTitle(Option<String>),
    /// `true` when a parent was set, `false` when it was cleared.
    SetParent(bool),
    SetEnabled(bool),
    SetTopmost(bool),
    SetWindowState(WindowState),
    SetMinMaxSize(Size, Size),
    /// `true` when an icon was set, `false` when it was cleared.
    SetIcon(bool),
    SetWindowDecorations(WindowDecorations),
    ShowTaskbarIcon(bool),
    CanResize(bool),
    SetCanMinimize(bool),
    SetCanMaximize(bool),
    /// `true` when a cursor was set, `false` for the default cursor.
    SetCursor(bool),
    SetTransparencyLevelHint(Vec<WindowTransparencyLevel>),
    SetFrameThemeVariant(Option<PlatformThemeVariant>),
    SetExtendClientAreaToDecorationsHint(bool),
    SetExtendClientAreaTitleBarHeightHint(f64),
    SetShadowExtents(Thickness),
    BeginMoveDrag,
    BeginResizeDrag(WindowEdge),
    SetWindowManagerAddShadowHint(bool),
    TakeFocus,
    SetHitTestVisible(bool),
    CreatePopup,
}

/// What a [`MockWindowImpl`] stands for: decides the answers of the
/// `as_*_impl` hooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MockImplKind {
    TopLevel,
    Window,
    Popup,
}

type Hook<F> = RefCell<Option<Rc<F>>>;

/// A hand-written double of the platform implementations of top-levels,
/// windows and popups.
///
/// Every property is a plain stored value (set it with the `setup_*`
/// methods), every call is recorded (see [`calls`](Self::calls)) and the
/// behaviour of the calls that the windowing platform mock reacts to (show,
/// resize, move, dispose, popup creation) is a replaceable hook.
pub struct MockWindowImpl {
    this: Weak<MockWindowImpl>,
    kind: MockImplKind,
    calls: RefCell<Vec<MockCall>>,
    features: RefCell<HashMap<TypeId, Rc<dyn Any>>>,

    // ITopLevelImpl state
    pub desktop_scaling: Cell<f64>,
    pub render_scaling: Cell<f64>,
    pub client_size: Cell<Size>,
    pub transparency_level: Cell<WindowTransparencyLevel>,
    pub acrylic_compensation_levels: Cell<AcrylicPlatformCompensationLevels>,
    handle: RefCell<Option<Rc<dyn IPlatformHandle>>>,
    compositor: RefCell<Option<Rc<Compositor>>>,
    surfaces: RefCell<Vec<Rc<dyn IPlatformRenderSurface>>>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    cursor: RefCell<Option<Rc<dyn ICursorImpl>>>,
    platform_specific_scene_info: RefCell<Option<Rc<dyn Any>>>,
    input: Hook<dyn Fn(Rc<dyn IRawInputEventArgs>)>,
    paint: Hook<dyn Fn(Rect)>,
    resized: Hook<dyn Fn(Size, WindowResizeReason)>,
    scaling_changed: Hook<dyn Fn(f64)>,
    transparency_level_changed: Hook<dyn Fn(WindowTransparencyLevel)>,
    platform_specific_scene_info_changed: Hook<dyn Fn(Option<Rc<dyn Any>>)>,
    closed: Hook<dyn Fn()>,
    lost_focus: Hook<dyn Fn()>,

    // IWindowBaseImpl state
    pub frame_size: Cell<Option<Size>>,
    pub position: Cell<PixelPoint>,
    pub max_auto_size_hint: Cell<Size>,
    position_changed: Hook<dyn Fn(PixelPoint)>,
    deactivated: Hook<dyn Fn()>,
    activated: Hook<dyn Fn()>,

    // IWindowImpl state
    pub window_state: Cell<WindowState>,
    /// The number of times the window state was read.
    pub window_state_get_count: Cell<usize>,
    pub window_state_getter_is_usable: Cell<bool>,
    pub is_client_area_extended_to_decorations: Cell<bool>,
    pub needs_managed_decorations: Cell<bool>,
    pub requested_drawn_decorations: Cell<PlatformRequestedDrawnDecoration>,
    pub extended_margins: Cell<Thickness>,
    pub off_screen_margin: Cell<Thickness>,
    pub allowed_window_actions: Cell<PlatformAllowedWindowActions>,
    parent: RefCell<Option<Rc<dyn IWindowImpl>>>,
    window_state_changed: Hook<dyn Fn(WindowState)>,
    got_input_when_disabled: Hook<dyn Fn()>,
    closing: Hook<dyn Fn(WindowCloseReason) -> bool>,
    extend_client_area_to_decorations_changed: Hook<dyn Fn(bool)>,
    drawn_decorations_request_changed: Hook<dyn Fn()>,
    allowed_window_actions_changed: Hook<dyn Fn(PlatformAllowedWindowActions)>,

    // IPopupImpl state
    popup_positioner: RefCell<Option<Rc<dyn IPopupPositioner>>>,

    // Behaviour
    on_show: Hook<dyn Fn(&MockWindowImpl, bool, bool)>,
    on_hide: Hook<dyn Fn(&MockWindowImpl)>,
    on_resize: Hook<dyn Fn(&MockWindowImpl, Size, WindowResizeReason)>,
    on_move: Hook<dyn Fn(&MockWindowImpl, PixelPoint)>,
    on_dispose: Hook<dyn Fn(&MockWindowImpl)>,
    on_create_popup: Hook<dyn Fn(&MockWindowImpl) -> Option<Rc<dyn IPopupImpl>>>,
    on_set_window_state: Hook<dyn Fn(&MockWindowImpl, WindowState)>,
    on_set_min_max_size: Hook<dyn Fn(&MockWindowImpl, Size, Size)>,
}

impl MockWindowImpl {
    /// A double without any behaviour: calls are recorded, properties keep
    /// what is stored in them (the counterpart of a mock with all
    /// properties set up).
    pub fn bare(kind: MockImplKind) -> Rc<MockWindowImpl> {
        Rc::new_cyclic(|this| MockWindowImpl {
            this: this.clone(),
            kind,
            calls: RefCell::new(Vec::new()),
            features: RefCell::new(HashMap::new()),
            desktop_scaling: Cell::new(1.0),
            render_scaling: Cell::new(1.0),
            client_size: Cell::new(Size::default()),
            transparency_level: Cell::new(WindowTransparencyLevel::none()),
            acrylic_compensation_levels: Cell::new(AcrylicPlatformCompensationLevels::default()),
            handle: RefCell::new(None),
            compositor: RefCell::new(None),
            surfaces: RefCell::new(Vec::new()),
            input_root: RefCell::new(None),
            cursor: RefCell::new(None),
            platform_specific_scene_info: RefCell::new(None),
            input: RefCell::new(None),
            paint: RefCell::new(None),
            resized: RefCell::new(None),
            scaling_changed: RefCell::new(None),
            transparency_level_changed: RefCell::new(None),
            platform_specific_scene_info_changed: RefCell::new(None),
            closed: RefCell::new(None),
            lost_focus: RefCell::new(None),
            frame_size: Cell::new(None),
            position: Cell::new(PixelPoint::default()),
            max_auto_size_hint: Cell::new(Size::default()),
            position_changed: RefCell::new(None),
            deactivated: RefCell::new(None),
            activated: RefCell::new(None),
            window_state: Cell::new(WindowState::Normal),
            window_state_get_count: Cell::new(0),
            window_state_getter_is_usable: Cell::new(false),
            is_client_area_extended_to_decorations: Cell::new(false),
            needs_managed_decorations: Cell::new(false),
            requested_drawn_decorations: Cell::new(PlatformRequestedDrawnDecoration::empty()),
            extended_margins: Cell::new(Thickness::default()),
            off_screen_margin: Cell::new(Thickness::default()),
            allowed_window_actions: Cell::new(PlatformAllowedWindowActions::ALL),
            parent: RefCell::new(None),
            window_state_changed: RefCell::new(None),
            got_input_when_disabled: RefCell::new(None),
            closing: RefCell::new(None),
            extend_client_area_to_decorations_changed: RefCell::new(None),
            drawn_decorations_request_changed: RefCell::new(None),
            allowed_window_actions_changed: RefCell::new(None),
            popup_positioner: RefCell::new(None),
            on_show: RefCell::new(None),
            on_hide: RefCell::new(None),
            on_resize: RefCell::new(None),
            on_move: RefCell::new(None),
            on_dispose: RefCell::new(None),
            on_create_popup: RefCell::new(None),
            on_set_window_state: RefCell::new(None),
            on_set_min_max_size: RefCell::new(None),
        })
    }

    /// A window implementation with the behaviour of the mock windowing
    /// platform: an 800x600 (by default) client area on a 1280x1024 screen;
    /// showing activates and reports the size, resizing constrains to the
    /// screen and reports the size, moving reports the position, disposing
    /// reports lost focus and closed, and popups are positioned by the
    /// managed positioner.
    pub fn window(initial_width: f64, initial_height: f64) -> Rc<MockWindowImpl> {
        let window_impl = Self::bare(MockImplKind::Window);
        window_impl.client_size.set(Size::new(initial_width, initial_height));
        window_impl.max_auto_size_hint.set(SCREEN_SIZE);
        window_impl.setup_feature::<dyn IScreenImpl>(Self::create_screen_mock());
        window_impl.setup_create_popup(|window_impl| {
            let parent: Rc<dyn ITopLevelImpl> = window_impl.rc();
            let popup: Rc<dyn IPopupImpl> = Self::popup(parent);
            Some(popup)
        });
        window_impl.setup_dispose(|window_impl| {
            if let Some(lost_focus) = ITopLevelImpl::lost_focus(window_impl) {
                lost_focus();
            }
            if let Some(closed) = ITopLevelImpl::closed(window_impl) {
                closed();
            }
        });
        window_impl.setup_move(|window_impl, point| {
            window_impl.position.set(point);
            if let Some(position_changed) = IWindowBaseImpl::position_changed(window_impl) {
                position_changed(point);
            }
        });
        window_impl.setup_resize(|window_impl, size, reason| {
            let constrained_size = size.constrain(SCREEN_SIZE);

            if constrained_size != window_impl.client_size.get() {
                window_impl.client_size.set(constrained_size);
                if let Some(resized) = ITopLevelImpl::resized(window_impl) {
                    resized(constrained_size, reason);
                }
            }
        });
        window_impl.setup_show(|window_impl, activate, _is_dialog| {
            if activate {
                if let Some(resized) = ITopLevelImpl::resized(window_impl) {
                    resized(window_impl.client_size.get(), WindowResizeReason::Unspecified);
                }
                if let Some(activated) = IWindowBaseImpl::activated(window_impl) {
                    activated();
                }
            }
        });
        window_impl
    }

    /// A popup implementation with the behaviour of the mock windowing
    /// platform: positioned and sized by the managed positioner relative to
    /// `parent`; disposing reports closed.
    pub fn popup(parent: Rc<dyn ITopLevelImpl>) -> Rc<MockWindowImpl> {
        let popup_impl = Self::bare(MockImplKind::Popup);
        popup_impl.max_auto_size_hint.set(SCREEN_SIZE);

        let weak = Rc::downgrade(&popup_impl);
        let positioner_helper = ManagedPopupPositionerPopupImplHelper::new(
            parent,
            Rc::new(move |pos, size: Size, _scale| {
                let Some(popup_impl) = weak.upgrade() else { return };
                let client_size = size.constrain(SCREEN_SIZE);
                popup_impl.client_size.set(client_size);
                popup_impl.position.set(pos);
                if let Some(position_changed) = IWindowBaseImpl::position_changed(&*popup_impl) {
                    position_changed(pos);
                }
                if let Some(resized) = ITopLevelImpl::resized(&*popup_impl) {
                    resized(client_size, WindowResizeReason::Unspecified);
                }
            }),
        );
        let positioner: Rc<dyn IPopupPositioner> = Rc::new(ManagedPopupPositioner::new(Rc::new(positioner_helper)));
        *popup_impl.popup_positioner.borrow_mut() = Some(positioner);

        popup_impl.setup_dispose(|popup_impl| {
            if let Some(closed) = ITopLevelImpl::closed(popup_impl) {
                closed();
            }
        });
        popup_impl
    }

    /// The screens of the mock windowing platform: one primary 1280x1024
    /// screen.
    pub fn create_screen_mock() -> Rc<dyn IScreenImpl> {
        let bounds = PixelRect::new(0, 0, SCREEN_SIZE.width as i32, SCREEN_SIZE.height as i32);
        MockScreenImpl::new(vec![mock_screen(96.0, bounds, bounds, true)])
    }

    fn rc(&self) -> Rc<MockWindowImpl> {
        self.this.upgrade().expect("the mock is alive while it is used")
    }

    fn record(&self, call: MockCall) {
        self.calls.borrow_mut().push(call);
    }

    /// The calls made so far, in order.
    pub fn calls(&self) -> Vec<MockCall> {
        self.calls.borrow().clone()
    }

    /// The number of recorded calls that satisfy `predicate`.
    pub fn count(&self, predicate: impl Fn(&MockCall) -> bool) -> usize {
        self.calls.borrow().iter().filter(|call| predicate(call)).count()
    }

    /// The number of recorded calls equal to `call`.
    pub fn count_of(&self, call: &MockCall) -> usize {
        self.count(|c| c == call)
    }

    /// Forgets the recorded calls.
    pub fn clear_calls(&self) {
        self.calls.borrow_mut().clear();
    }

    /// Registers an optional feature, queried as `Rc<T>`.
    pub fn setup_feature<T: ?Sized + 'static>(&self, feature: Rc<T>) {
        self.features.borrow_mut().insert(TypeId::of::<T>(), Rc::new(feature));
    }

    /// Removes an optional feature.
    pub fn remove_feature<T: ?Sized + 'static>(&self) {
        self.features.borrow_mut().remove(&TypeId::of::<T>());
    }

    pub fn setup_handle(&self, handle: Option<Rc<dyn IPlatformHandle>>) {
        *self.handle.borrow_mut() = handle;
    }

    pub fn setup_compositor(&self, compositor: Option<Rc<Compositor>>) {
        *self.compositor.borrow_mut() = compositor;
    }

    pub fn setup_surfaces(&self, surfaces: Vec<Rc<dyn IPlatformRenderSurface>>) {
        *self.surfaces.borrow_mut() = surfaces;
    }

    pub fn setup_platform_specific_scene_info(&self, info: Option<Rc<dyn Any>>) {
        *self.platform_specific_scene_info.borrow_mut() = info;
    }

    pub fn setup_popup_positioner(&self, positioner: Option<Rc<dyn IPopupPositioner>>) {
        *self.popup_positioner.borrow_mut() = positioner;
    }

    /// Replaces what `show` does.
    pub fn setup_show(&self, hook: impl Fn(&MockWindowImpl, bool, bool) + 'static) {
        *self.on_show.borrow_mut() = Some(Rc::new(hook));
    }

    /// Replaces what `hide` does.
    pub fn setup_hide(&self, hook: impl Fn(&MockWindowImpl) + 'static) {
        *self.on_hide.borrow_mut() = Some(Rc::new(hook));
    }

    /// Replaces what `resize` does.
    pub fn setup_resize(&self, hook: impl Fn(&MockWindowImpl, Size, WindowResizeReason) + 'static) {
        *self.on_resize.borrow_mut() = Some(Rc::new(hook));
    }

    /// Replaces what `move_` does.
    pub fn setup_move(&self, hook: impl Fn(&MockWindowImpl, PixelPoint) + 'static) {
        *self.on_move.borrow_mut() = Some(Rc::new(hook));
    }

    /// Replaces what `dispose` does.
    pub fn setup_dispose(&self, hook: impl Fn(&MockWindowImpl) + 'static) {
        *self.on_dispose.borrow_mut() = Some(Rc::new(hook));
    }

    /// Replaces what `create_popup` returns.
    pub fn setup_create_popup(&self, hook: impl Fn(&MockWindowImpl) -> Option<Rc<dyn IPopupImpl>> + 'static) {
        *self.on_create_popup.borrow_mut() = Some(Rc::new(hook));
    }

    /// Adds behaviour to `set_window_state` (the state is stored in any
    /// case).
    pub fn setup_set_window_state(&self, hook: impl Fn(&MockWindowImpl, WindowState) + 'static) {
        *self.on_set_window_state.borrow_mut() = Some(Rc::new(hook));
    }

    /// Adds behaviour to `set_min_max_size`.
    pub fn setup_set_min_max_size(&self, hook: impl Fn(&MockWindowImpl, Size, Size) + 'static) {
        *self.on_set_min_max_size.borrow_mut() = Some(Rc::new(hook));
    }

    /// The input root set by the presentation source.
    pub fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    /// The cursor set last.
    pub fn cursor(&self) -> Option<Rc<dyn ICursorImpl>> {
        self.cursor.borrow().clone()
    }

    /// The parent set last.
    pub fn parent(&self) -> Option<Rc<dyn IWindowImpl>> {
        self.parent.borrow().clone()
    }
}

fn get<F: ?Sized>(hook: &Hook<F>) -> Option<Rc<F>> {
    hook.borrow().clone()
}

fn set<F: ?Sized>(hook: &Hook<F>, value: Option<Rc<F>>) {
    let old = hook.replace(value);
    drop(old);
}

impl IOptionalFeatureProvider for MockWindowImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.features.borrow().get(&feature_type).cloned()
    }
}

impl IDisposable for MockWindowImpl {
    fn dispose(&self) {
        self.record(MockCall::Dispose);
        if let Some(hook) = get(&self.on_dispose) {
            hook(self);
        }
    }
}

impl ITopLevelImpl for MockWindowImpl {
    fn desktop_scaling(&self) -> f64 {
        self.desktop_scaling.get()
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        self.handle.borrow().clone()
    }

    fn client_size(&self) -> Size {
        self.client_size.get()
    }

    fn render_scaling(&self) -> f64 {
        self.render_scaling.get()
    }

    fn surfaces(&self) -> Vec<Rc<dyn IPlatformRenderSurface>> {
        self.surfaces.borrow().clone()
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        self.compositor.borrow().clone()
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

    fn platform_specific_scene_info(&self) -> Option<Rc<dyn Any>> {
        self.platform_specific_scene_info.borrow().clone()
    }

    fn platform_specific_scene_info_changed(&self) -> Option<Rc<dyn Fn(Option<Rc<dyn Any>>)>> {
        get(&self.platform_specific_scene_info_changed)
    }

    fn set_platform_specific_scene_info_changed(&self, value: Option<Rc<dyn Fn(Option<Rc<dyn Any>>)>>) {
        set(&self.platform_specific_scene_info_changed, value)
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        *self.input_root.borrow_mut() = Some(input_root);
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        (point - self.position.get()).to_point(1.0)
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::from_point(point, 1.0) + self.position.get()
    }

    fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
        self.record(MockCall::SetCursor(cursor.is_some()));
        *self.cursor.borrow_mut() = cursor;
    }

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
        self.record(MockCall::CreatePopup);
        get(&self.on_create_popup).and_then(|hook| hook(self))
    }

    fn set_transparency_level_hint(&self, transparency_levels: &[WindowTransparencyLevel]) {
        self.record(MockCall::SetTransparencyLevelHint(transparency_levels.to_vec()));
    }

    fn transparency_level(&self) -> WindowTransparencyLevel {
        self.transparency_level.get()
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        self.acrylic_compensation_levels.get()
    }

    fn set_frame_theme_variant(&self, theme_variant: Option<PlatformThemeVariant>) {
        self.record(MockCall::SetFrameThemeVariant(theme_variant));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_window_base_impl(&self) -> Option<&dyn IWindowBaseImpl> {
        (self.kind != MockImplKind::TopLevel).then_some(self as &dyn IWindowBaseImpl)
    }

    fn as_window_impl(&self) -> Option<&dyn IWindowImpl> {
        (self.kind == MockImplKind::Window).then_some(self as &dyn IWindowImpl)
    }

    fn as_popup_impl(&self) -> Option<&dyn IPopupImpl> {
        (self.kind == MockImplKind::Popup).then_some(self as &dyn IPopupImpl)
    }
}

impl IWindowBaseImpl for MockWindowImpl {
    fn frame_size(&self) -> Option<Size> {
        self.frame_size.get()
    }

    fn show(&self, activate: bool, is_dialog: bool) {
        self.record(MockCall::Show { activate, is_dialog });
        if let Some(hook) = get(&self.on_show) {
            hook(self, activate, is_dialog);
        }
    }

    fn hide(&self) {
        self.record(MockCall::Hide);
        if let Some(hook) = get(&self.on_hide) {
            hook(self);
        }
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
        self.record(MockCall::Activate);
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
        self.max_auto_size_hint.get()
    }

    fn set_topmost(&self, value: bool) {
        self.record(MockCall::SetTopmost(value));
    }
}

impl IWindowImpl for MockWindowImpl {
    fn window_state(&self) -> WindowState {
        self.window_state_get_count.set(self.window_state_get_count.get() + 1);
        self.window_state.get()
    }

    fn set_window_state(&self, value: WindowState) {
        self.record(MockCall::SetWindowState(value));
        self.window_state.set(value);
        if let Some(hook) = get(&self.on_set_window_state) {
            hook(self, value);
        }
    }

    fn window_state_getter_is_usable(&self) -> bool {
        self.window_state_getter_is_usable.get()
    }

    fn window_state_changed(&self) -> Option<Rc<dyn Fn(WindowState)>> {
        get(&self.window_state_changed)
    }

    fn set_window_state_changed(&self, value: Option<Rc<dyn Fn(WindowState)>>) {
        set(&self.window_state_changed, value)
    }

    fn set_title(&self, title: Option<&str>) {
        self.record(MockCall::SetTitle(title.map(str::to_string)));
    }

    fn set_parent(&self, parent: Option<Rc<dyn IWindowImpl>>) {
        self.record(MockCall::SetParent(parent.is_some()));
        *self.parent.borrow_mut() = parent;
    }

    fn set_enabled(&self, enable: bool) {
        self.record(MockCall::SetEnabled(enable));
    }

    fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.got_input_when_disabled)
    }

    fn set_got_input_when_disabled(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.got_input_when_disabled, value)
    }

    fn set_window_decorations(&self, enabled: WindowDecorations) {
        self.record(MockCall::SetWindowDecorations(enabled));
    }

    fn set_icon(&self, icon: Option<Rc<dyn IWindowIconImpl>>) {
        self.record(MockCall::SetIcon(icon.is_some()));
    }

    fn show_taskbar_icon(&self, value: bool) {
        self.record(MockCall::ShowTaskbarIcon(value));
    }

    fn can_resize(&self, value: bool) {
        self.record(MockCall::CanResize(value));
    }

    fn set_can_minimize(&self, value: bool) {
        self.record(MockCall::SetCanMinimize(value));
    }

    fn set_can_maximize(&self, value: bool) {
        self.record(MockCall::SetCanMaximize(value));
    }

    fn closing(&self) -> Option<Rc<dyn Fn(WindowCloseReason) -> bool>> {
        get(&self.closing)
    }

    fn set_closing(&self, value: Option<Rc<dyn Fn(WindowCloseReason) -> bool>>) {
        set(&self.closing, value)
    }

    fn is_client_area_extended_to_decorations(&self) -> bool {
        self.is_client_area_extended_to_decorations.get()
    }

    fn extend_client_area_to_decorations_changed(&self) -> Option<Rc<dyn Fn(bool)>> {
        get(&self.extend_client_area_to_decorations_changed)
    }

    fn set_extend_client_area_to_decorations_changed(&self, value: Option<Rc<dyn Fn(bool)>>) {
        set(&self.extend_client_area_to_decorations_changed, value)
    }

    fn needs_managed_decorations(&self) -> bool {
        self.needs_managed_decorations.get()
    }

    fn requested_drawn_decorations(&self) -> PlatformRequestedDrawnDecoration {
        self.requested_drawn_decorations.get()
    }

    fn drawn_decorations_request_changed(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.drawn_decorations_request_changed)
    }

    fn set_drawn_decorations_request_changed(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.drawn_decorations_request_changed, value)
    }

    fn extended_margins(&self) -> Thickness {
        self.extended_margins.get()
    }

    fn off_screen_margin(&self) -> Thickness {
        self.off_screen_margin.get()
    }

    fn begin_move_drag(&self, _e: &PointerPressedEventArgs) {
        self.record(MockCall::BeginMoveDrag);
    }

    fn begin_resize_drag(&self, edge: WindowEdge, _e: &PointerPressedEventArgs) {
        self.record(MockCall::BeginResizeDrag(edge));
    }

    fn resize(&self, client_size: Size, reason: WindowResizeReason) {
        self.record(MockCall::Resize(client_size, reason));
        if let Some(hook) = get(&self.on_resize) {
            hook(self, client_size, reason);
        }
    }

    fn move_(&self, point: PixelPoint) {
        self.record(MockCall::Move(point));
        if let Some(hook) = get(&self.on_move) {
            hook(self, point);
        }
    }

    fn set_min_max_size(&self, min_size: Size, max_size: Size) {
        self.record(MockCall::SetMinMaxSize(min_size, max_size));
        if let Some(hook) = get(&self.on_set_min_max_size) {
            hook(self, min_size, max_size);
        }
    }

    fn set_extend_client_area_to_decorations_hint(&self, extend_into_client_area_hint: bool) {
        self.record(MockCall::SetExtendClientAreaToDecorationsHint(extend_into_client_area_hint));
    }

    fn set_extend_client_area_title_bar_height_hint(&self, title_bar_height: f64) {
        self.record(MockCall::SetExtendClientAreaTitleBarHeightHint(title_bar_height));
    }

    fn allowed_window_actions(&self) -> PlatformAllowedWindowActions {
        self.allowed_window_actions.get()
    }

    fn allowed_window_actions_changed(&self) -> Option<Rc<dyn Fn(PlatformAllowedWindowActions)>> {
        get(&self.allowed_window_actions_changed)
    }

    fn set_allowed_window_actions_changed(&self, value: Option<Rc<dyn Fn(PlatformAllowedWindowActions)>>) {
        set(&self.allowed_window_actions_changed, value)
    }

    fn set_shadow_extents(&self, extents: Thickness) {
        self.record(MockCall::SetShadowExtents(extents));
    }
}

impl IPopupImpl for MockWindowImpl {
    fn popup_positioner(&self) -> Option<Rc<dyn IPopupPositioner>> {
        self.popup_positioner.borrow().clone()
    }

    fn set_window_manager_add_shadow_hint(&self, enabled: bool) {
        self.record(MockCall::SetWindowManagerAddShadowHint(enabled));
    }

    fn take_focus(&self) {
        self.record(MockCall::TakeFocus);
    }

    fn set_hit_test_visible(&self, is_hit_test_visible: bool) {
        self.record(MockCall::SetHitTestVisible(is_hit_test_visible));
    }
}
