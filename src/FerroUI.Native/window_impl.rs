use crate::double_click_helper::DoubleClickHelper;
use crate::ferro_native_menu_exporter::FerroNativeMenuExporter;
use crate::ferro_native_platform_extensions::FerroNativePlatformOptions;
use crate::frn_string::to_c_string;
use crate::helpers::*;
use crate::interop::*;
use crate::popup_impl::PopupImpl;
use crate::top_level_impl::{
    callback_property, impl_top_level_contract, MacOSTopLevelHandle, TopLevelEvents, TopLevelImpl, TopLevelParent,
};
use crate::window_impl_base::{impl_window_base_contract, WindowBaseImpl, WindowBaseParent, WindowEventsParent};
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{InputElement, PointerPressedEventArgs, WindowDecorationsElementRole};
use ferroui_base::media::Color;
use ferroui_base::{PixelPoint, Ref, Size, Thickness, Visual};
use ferroui_controls::chrome::WindowDecorationProperties;
use ferroui_controls::platform::{
    ITopLevelNativeMenuExporter, IPopupImpl, ITopLevelImpl, IWindowBaseImpl, IWindowIconImpl, IWindowImpl, PlatformRequestedDrawnDecoration,
    PlatformThemeVariant,
};
use ferroui_controls::{WindowCloseReason, WindowDecorations, WindowEdge, WindowResizeReason, WindowState};
use ferroui_microcom::ComPtr;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A macOS window.
pub struct WindowImpl {
    weak_self: Weak<WindowImpl>,
    base: WindowBaseImpl,
    opts: FerroNativePlatformOptions,
    /// The native window; `None` until it is created and once it is
    /// disposed.
    native: RefCell<Option<ComPtr<IFrnWindow>>>,
    extend_title_bar_height: Cell<f64>,
    double_click_helper: DoubleClickHelper,
    can_resize: Cell<bool>,
    can_maximize: Cell<bool>,
    decorations: Cell<WindowDecorations>,
    native_menu_exporter: RefCell<Option<Rc<FerroNativeMenuExporter>>>,
    is_extended: Cell<bool>,
    extended_margins: Cell<Thickness>,

    window_state_changed: RefCell<Option<Rc<dyn Fn(WindowState)>>>,
    extend_client_area_to_decorations_changed: RefCell<Option<Rc<dyn Fn(bool)>>>,
    closing: RefCell<Option<Rc<dyn Fn(WindowCloseReason) -> bool>>>,
    got_input_when_disabled: RefCell<Option<Rc<dyn Fn()>>>,
}

impl WindowImpl {
    pub(crate) fn new(factory: ComPtr<IFerroNativeFactory>, opts: FerroNativePlatformOptions) -> Rc<WindowImpl> {
        let this = Rc::new_cyclic(|weak_self| WindowImpl {
            weak_self: weak_self.clone(),
            base: WindowBaseImpl::new(factory.clone()),
            opts,
            native: RefCell::new(None),
            extend_title_bar_height: Cell::new(-1.0),
            double_click_helper: DoubleClickHelper::new(),
            can_resize: Cell::new(true),
            can_maximize: Cell::new(true),
            decorations: Cell::new(WindowDecorations::Full),
            native_menu_exporter: RefCell::new(None),
            is_extended: Cell::new(false),
            extended_margins: Cell::new(Thickness::default()),
            window_state_changed: RefCell::new(None),
            extend_client_area_to_decorations_changed: RefCell::new(None),
            closing: RefCell::new(None),
            got_input_when_disabled: RefCell::new(None),
        });

        let e = IFrnWindowEvents::from_impl(TopLevelEvents(this.clone()));
        let native = factory.create_window(Some(&e)).check().expect("the native window");
        *this.native.borrow_mut() = Some(native.clone());
        this.base.init(MacOSTopLevelHandle::from_window_base(ComPtr::<IFrnWindowBase>::from_ref(&native)));

        *this.native_menu_exporter.borrow_mut() = Some(FerroNativeMenuExporter::for_window(native, &factory));

        this
    }

    /// The native window; `None` once the window is disposed.
    pub fn native(&self) -> Option<ComPtr<IFrnWindow>> {
        self.native.borrow().clone()
    }

    /// The native window of a live window.
    ///
    /// # Panics
    /// Panics when the window is disposed, like every use of a disposed
    /// native object in the reference implementation.
    #[track_caller]
    fn live_native(&self) -> ComPtr<IFrnWindow> {
        match self.native() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: the native window"),
        }
    }

    pub(crate) fn top_level(&self) -> &Rc<TopLevelImpl> {
        self.base.top_level()
    }

    pub fn set_title_bar_color(&self, color: Color) {
        self.live_native()
            .set_title_bar_color(FrnColor { alpha: color.a, red: color.r, green: color.g, blue: color.b })
            .check();
    }

    pub fn z_order(&self) -> Option<isize> {
        Some(self.live_native().get_window_z_order().check() as isize)
    }

    fn invalidate_extended_margins(&self) {
        let Some(native) = self.native() else {
            return;
        };

        if self.window_state() == WindowState::FullScreen
            || !self.is_extended.get()
            || self.decorations.get() != WindowDecorations::Full
        {
            self.extended_margins.set(Thickness::default());
        } else {
            let top = if self.extend_title_bar_height.get() == -1.0 {
                native.get_extend_title_bar_height().check()
            } else {
                self.extend_title_bar_height.get()
            };
            self.extended_margins.set(Thickness::new(0.0, top, 0.0, 0.0));
        }

        let changed = self.extend_client_area_to_decorations_changed.borrow().clone();
        if let Some(changed) = changed {
            changed(self.is_extended.get());
        }
    }

    callback_property!(window_state_changed_cb, set_window_state_changed_cb, window_state_changed, dyn Fn(WindowState));
    callback_property!(
        extend_client_area_to_decorations_changed_cb,
        set_extend_client_area_to_decorations_changed_cb,
        extend_client_area_to_decorations_changed,
        dyn Fn(bool)
    );
    callback_property!(closing_cb, set_closing_cb, closing, dyn Fn(WindowCloseReason) -> bool);
    callback_property!(got_input_when_disabled_cb, set_got_input_when_disabled_cb, got_input_when_disabled, dyn Fn());
}

impl TopLevelParent for WindowImpl {
    fn top_level(&self) -> &Rc<TopLevelImpl> {
        self.base.top_level()
    }

    fn dispose_top_level(&self) {
        self.base.dispose();
        let exporter = self.native_menu_exporter.borrow_mut().take();
        drop(exporter);
        let native = self.native.borrow_mut().take();
        drop(native);
    }

    fn chrome_hit_test(&self, e: &RawPointerEventArgs) -> bool {
        if self.is_extended.get() && e.type_() == RawPointerEventType::LeftButtonDown {
            // The presentation source behind the input root hit-tests its
            // visual tree.
            let root: Option<Ref<Visual>> = self.top_level().input_root().map(|root| root.root_element().upcast());
            let source = root.as_ref().and_then(|root| root.presentation_source());
            let visual = match (&source, &root) {
                (Some(source), Some(root)) => source.hit_tester().hit_test_first(
                    e.position(),
                    root,
                    Some(&|x: &Visual| {
                        if let Some(ie) = x.to_ref().cast::<InputElement>() {
                            if !ie.is_hit_test_visible() || !ie.is_effectively_visible() {
                                return false;
                            }
                        }
                        true
                    }),
                ),
                _ => None,
            };

            if visual.is_none_or(|visual| {
                WindowDecorationProperties::get_element_role(&visual) == WindowDecorationsElementRole::TitleBar
            }) {
                if self.double_click_helper.is_double_click(e.timestamp(), e.position()) {
                    match self.window_state() {
                        WindowState::Maximized | WindowState::FullScreen if self.can_resize.get() => {
                            self.set_window_state(WindowState::Normal);
                        }
                        WindowState::Normal if self.can_maximize.get() => {
                            self.set_window_state(WindowState::Maximized);
                        }
                        _ => {}
                    }
                } else {
                    self.live_native().begin_move_drag().check();
                }
            }
        }

        false
    }

    fn create_popup_core(&self) -> Option<Rc<dyn IPopupImpl>> {
        if self.opts.overlay_popups {
            return None;
        }
        let this: Rc<dyn ITopLevelImpl> = self.weak_self.upgrade()?;
        Some(PopupImpl::new(self.top_level().factory().clone(), this))
    }

    fn set_frame_theme_variant_core(&self, theme_variant: Option<PlatformThemeVariant>) {
        self.base.set_frame_theme_variant(theme_variant);
    }

    fn try_get_feature_core(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn ITopLevelNativeMenuExporter>() {
            let exporter: Rc<dyn ITopLevelNativeMenuExporter> = self.native_menu_exporter.borrow().clone()?;
            return Some(Rc::new(exporter));
        }

        self.top_level().try_get_feature(feature_type)
    }
}

impl WindowBaseParent for WindowImpl {
    fn window_base(&self) -> &WindowBaseImpl {
        &self.base
    }

    fn show_core(&self, activate: bool, is_dialog: bool) {
        self.base.show(activate, is_dialog);

        self.invalidate_extended_margins();
    }
}

impl WindowEventsParent for WindowImpl {
    fn on_closing(&self) -> bool {
        let closing = self.closing.borrow().clone();
        if let Some(closing) = closing {
            return closing(WindowCloseReason::WindowClosing);
        }

        true
    }

    fn on_window_state_changed(&self, state: FrnWindowState) {
        self.invalidate_extended_margins();

        let changed = self.window_state_changed.borrow().clone();
        if let Some(changed) = changed {
            changed(to_window_state(state));
        }
    }

    fn on_got_input_when_disabled(&self) {
        let callback = self.got_input_when_disabled.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}

impl_top_level_contract!(WindowImpl {
    fn as_window_base_impl(&self) -> Option<&dyn IWindowBaseImpl> {
        Some(self)
    }

    fn as_window_impl(&self) -> Option<&dyn IWindowImpl> {
        Some(self)
    }
});

impl_window_base_contract!(WindowImpl);

impl IWindowImpl for WindowImpl {
    fn window_state(&self) -> WindowState {
        to_window_state(self.live_native().get_window_state().check())
    }

    fn set_window_state(&self, value: WindowState) {
        self.live_native().set_window_state(to_frn_window_state(value)).check();
    }

    fn window_state_getter_is_usable(&self) -> bool {
        false
    }

    fn window_state_changed(&self) -> Option<Rc<dyn Fn(WindowState)>> {
        self.window_state_changed_cb()
    }

    fn set_window_state_changed(&self, value: Option<Rc<dyn Fn(WindowState)>>) {
        self.set_window_state_changed_cb(value)
    }

    fn set_title(&self, title: Option<&str>) {
        self.live_native().set_title(Some(&to_c_string(title.unwrap_or("")))).check();
    }

    fn set_parent(&self, parent: Option<Rc<dyn IWindowImpl>>) {
        let parent_native = parent.as_ref().map(|parent| {
            let Some(parent) = parent.as_any().downcast_ref::<WindowImpl>() else {
                panic!("The parent window belongs to a different windowing platform.");
            };
            parent.live_native()
        });
        let parent_base: Option<&IFrnWindowBase> = parent_native.as_deref().map(|native| &**native);
        self.live_native().set_parent(parent_base).check();
    }

    fn set_enabled(&self, enable: bool) {
        self.live_native().set_enabled(enable).check();

        // Showing a dialog should result in mouse capture being lost. macOS doesn't have the concept of mouse
        // capture, so no we have no OS-level event to hook into. Instead, release the mouse capture when the
        // owner window is disabled. This behavior matches win32, which sends a WM_CANCELMODE message when
        // EnableWindow(hWnd, false) is called from SetEnabled.
        if !enable {
            self.top_level().mouse().platform_capture_lost();
        }
    }

    fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>> {
        self.got_input_when_disabled_cb()
    }

    fn set_got_input_when_disabled(&self, value: Option<Rc<dyn Fn()>>) {
        self.set_got_input_when_disabled_cb(value)
    }

    fn set_window_decorations(&self, enabled: WindowDecorations) {
        self.decorations.set(enabled);
        self.live_native().set_decorations(to_frn_decorations(enabled)).check();
        self.invalidate_extended_margins();
    }

    fn set_icon(&self, _icon: Option<Rc<dyn IWindowIconImpl>>) {
        // NO OP on OSX
    }

    fn show_taskbar_icon(&self, _value: bool) {
        // NO OP On OSX
    }

    fn can_resize(&self, value: bool) {
        self.can_resize.set(value);
        self.live_native().set_can_resize(value).check();
    }

    fn set_can_minimize(&self, value: bool) {
        self.live_native().set_can_minimize(value).check();
    }

    fn set_can_maximize(&self, value: bool) {
        self.can_maximize.set(value);
        self.live_native().set_can_maximize(value).check();
    }

    fn closing(&self) -> Option<Rc<dyn Fn(WindowCloseReason) -> bool>> {
        self.closing_cb()
    }

    fn set_closing(&self, value: Option<Rc<dyn Fn(WindowCloseReason) -> bool>>) {
        self.set_closing_cb(value)
    }

    fn is_client_area_extended_to_decorations(&self) -> bool {
        self.is_extended.get()
    }

    fn extend_client_area_to_decorations_changed(&self) -> Option<Rc<dyn Fn(bool)>> {
        self.extend_client_area_to_decorations_changed_cb()
    }

    fn set_extend_client_area_to_decorations_changed(&self, value: Option<Rc<dyn Fn(bool)>>) {
        self.set_extend_client_area_to_decorations_changed_cb(value)
    }

    fn needs_managed_decorations(&self) -> bool {
        false
    }

    // Extension is handled by native backend
    fn requested_drawn_decorations(&self) -> PlatformRequestedDrawnDecoration {
        PlatformRequestedDrawnDecoration::NONE
    }

    fn extended_margins(&self) -> Thickness {
        self.extended_margins.get()
    }

    fn off_screen_margin(&self) -> Thickness {
        Thickness::default()
    }

    fn begin_move_drag(&self, _e: &PointerPressedEventArgs) {
        self.base.begin_move_drag();
    }

    fn begin_resize_drag(&self, edge: WindowEdge, _e: &PointerPressedEventArgs) {
        self.base.begin_resize_drag(edge);
    }

    fn resize(&self, client_size: Size, reason: WindowResizeReason) {
        self.base.resize(client_size, reason);
    }

    fn move_(&self, point: PixelPoint) {
        self.base.set_position(point);
    }

    fn set_min_max_size(&self, min_size: Size, max_size: Size) {
        self.base.set_min_max_size(min_size, max_size);
    }

    fn set_extend_client_area_to_decorations_hint(&self, extend_into_client_area_hint: bool) {
        self.is_extended.set(extend_into_client_area_hint);

        self.live_native().set_extend_client_area(extend_into_client_area_hint).check();

        self.invalidate_extended_margins();
    }

    fn set_extend_client_area_title_bar_height_hint(&self, title_bar_height: f64) {
        self.extend_title_bar_height.set(title_bar_height);
        self.live_native().set_extend_title_bar_height(title_bar_height).check();

        self.invalidate_extended_margins();
    }
}
