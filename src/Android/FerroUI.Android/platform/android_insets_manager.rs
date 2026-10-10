//! The insets of the window of an activity: the safe area, the system bars
//! and the input pane.
//!
//! The reference calls the window and the compatibility classes of
//! AndroidX inline. Here the calls are behind [`IInsetsWindow`], which the
//! Java layer answers with the classes of the platform
//! (docs/porting/android-platform.md, section 3.3) and the tests with a
//! fake; the logic of the manager is the same on both.

use ferroui_base::media::{Color, Colors};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Size, Thickness};
use ferroui_controls::platform::{
    IInputPane, IInsetsManager, InputPaneBase, InputPaneState, InputPaneStateEventArgs, InsetsManagerBase,
    SafeAreaChangedArgs, SystemBarTheme,
};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The insets of the root window, in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RootInsets {
    /// The status bars, the navigation bars and the display cutout together.
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    /// The bottom inset of the navigation bars.
    pub navigation_bars_bottom: i32,
    /// The bottom inset of the input method.
    pub ime_bottom: i32,
    pub ime_visible: bool,
    /// Whether the status and navigation bars are visible.
    pub system_bars_visible: bool,
}

impl RootInsets {
    /// Reads the values of `PlatformHelper.getRootInsets`.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub fn from_array(values: &[i32]) -> Option<RootInsets> {
        match values {
            [left, top, right, bottom, navigation_bars_bottom, ime_bottom, ime_visible, system_bars_visible] => {
                Some(RootInsets {
                    left: *left,
                    top: *top,
                    right: *right,
                    bottom: *bottom,
                    navigation_bars_bottom: *navigation_bars_bottom,
                    ime_bottom: *ime_bottom,
                    ime_visible: *ime_visible != 0,
                    system_bars_visible: *system_bars_visible != 0,
                })
            }
            _ => None,
        }
    }
}

/// The window of the activity, as the insets manager uses it.
pub(crate) trait IInsetsWindow {
    /// `Build.VERSION.SDK_INT`.
    fn sdk_int(&self) -> i32;

    /// Whether the activity has a window.
    fn has_window(&self) -> bool;

    /// The insets of the root window; `None` before the window has any.
    fn root_insets(&self) -> Option<RootInsets>;

    /// Whether the system displays the application edge to edge whatever it
    /// asks for: it targets API 35 and runs on API 35 or later.
    fn is_display_edge_to_edge_forced(&self) -> bool;

    fn set_layout_in_display_cutout_mode(&self, short_edges: bool);
    fn set_decor_fits_system_windows(&self, decor_fits_system_windows: bool);
    /// Adds the translucent flags of the status and the navigation bar.
    fn add_translucent_bars(&self);
    /// Clears the translucent flags, draws the bar backgrounds and, where
    /// the system still allows it, sets their colour.
    fn set_system_bar_color(&self, color: Color);
    fn set_navigation_bar_contrast_enforced(&self, enforced: bool);
    /// Whether the status bar is drawn for a light background; light when
    /// the window cannot be asked.
    fn appearance_light_status_bars(&self) -> bool;
    fn set_appearance_light_bars(&self, light: bool);
    fn set_system_bars_visible(&self, visible: bool);
}

/// The top-level, as the insets manager uses it.
pub(crate) trait IInsetsTopLevel {
    /// `RenderScaling` of the top-level.
    fn top_level_render_scaling(&self) -> f64;
    /// `ClientSize` of the top-level.
    fn top_level_client_size(&self) -> Size;
}

pub struct AndroidInsetsManager {
    window: Box<dyn IInsetsWindow>,
    top_level: Weak<dyn IInsetsTopLevel>,
    displays_edge_to_edge: Cell<bool>,
    system_ui_visibility: Cell<Option<bool>>,
    status_bar_theme: Cell<Option<SystemBarTheme>>,
    is_default_system_bar_light_theme: Cell<Option<bool>>,
    system_bar_color: Cell<Option<Color>>,
    state: Cell<InputPaneState>,
    previous_rect: Cell<Rect>,
    previous_ime_inset: Cell<Option<i32>>,
    display_edge_to_edge_preference: Cell<bool>,
    is_display_edge_to_edge_forced: bool,
    safe_area_changed: InsetsManagerBase,
    state_changed: InputPaneBase,
}

/// Android 10 (`Build.VERSION_CODES.Q`).
const Q: i32 = 29;
/// Android 11 (`Build.VERSION_CODES.R`).
const R: i32 = 30;

impl AndroidInsetsManager {
    /// Creates the manager of the window. The caller makes the window
    /// report applied insets to
    /// [`on_apply_window_insets`](Self::on_apply_window_insets).
    ///
    /// Below API 30 the reference also follows the global layout of the
    /// decor view to learn the state of the input pane; that path is
    /// stage 2 of docs/porting/android-platform.md, with the input method.
    pub(crate) fn new(
        window: Box<dyn IInsetsWindow>,
        top_level: Weak<dyn IInsetsTopLevel>,
    ) -> Rc<AndroidInsetsManager> {
        // Better detection for target sdk and running api level. Apps can change their target sdk and bypass
        // the fixed target sdk level.
        let is_display_edge_to_edge_forced = window.is_display_edge_to_edge_forced();

        let this = Rc::new(AndroidInsetsManager {
            window,
            top_level,
            displays_edge_to_edge: Cell::new(false),
            system_ui_visibility: Cell::new(None),
            status_bar_theme: Cell::new(None),
            is_default_system_bar_light_theme: Cell::new(None),
            system_bar_color: Cell::new(None),
            state: Cell::new(InputPaneState::Closed),
            previous_rect: Cell::new(Rect::default()),
            previous_ime_inset: Cell::new(None),
            display_edge_to_edge_preference: Cell::new(false),
            is_display_edge_to_edge_forced,
            safe_area_changed: InsetsManagerBase::new(),
            state_changed: InputPaneBase::new(),
        });

        this.set_display_edge_to_edge_preference(false);
        this
    }

    fn render_scaling(&self) -> f64 {
        self.top_level.upgrade().map_or(1.0, |top_level| top_level.top_level_render_scaling())
    }

    fn client_size(&self) -> Size {
        self.top_level.upgrade().map_or(Size::default(), |top_level| top_level.top_level_client_size())
    }

    fn set_state(&self, value: InputPaneState) {
        let old_state = self.state.replace(value);

        if old_state != value && self.window.sdk_int() <= Q {
            let current_rect = self.occluded_rect();
            self.notify_state_changed(value, Some(self.previous_rect.get()), current_rect);
            self.previous_rect.set(current_rect);
        }
    }

    fn update_display_edge_to_egde_state(&self) {
        if self.is_display_edge_to_edge_forced {
            self.displays_edge_to_edge.set(true);
            return;
        }

        let preference = self.display_edge_to_edge_preference.get();
        self.displays_edge_to_edge.set(preference);

        self.window.set_layout_in_display_cutout_mode(preference);
        self.window.set_decor_fits_system_windows(!preference);

        if preference {
            self.window.add_translucent_bars();
        } else {
            self.set_system_bar_color(self.system_bar_color.get());
        }
    }

    /// The window applied insets: `ime_visible` and `ime_bottom` are those
    /// of the insets that were applied.
    pub(crate) fn on_apply_window_insets(&self, has_insets: bool, ime_visible: bool, ime_bottom: i32) {
        self.notify_safe_area_changed(self.safe_area_padding());

        if self.previous_rect.get() == Rect::default() {
            self.previous_rect.set(self.occluded_rect());
        }

        self.set_state(if has_insets && ime_visible { InputPaneState::Open } else { InputPaneState::Closed });

        // Workaround for weird inset values for android 11
        if self.window.sdk_int() == R {
            let ime_inset = has_insets.then_some(ime_bottom);
            if self.previous_ime_inset.get().is_none() {
                self.previous_ime_inset.set(ime_inset);
            }
            if ime_inset.unwrap_or(0) != self.previous_ime_inset.get().unwrap_or(0) {
                self.notify_state_changed(self.state.get(), Some(self.previous_rect.get()), self.occluded_rect());
            }
            self.previous_ime_inset.set(ime_inset);
        }
    }

    pub(crate) fn set_default_system_light_mode(&self, is_light_mode: bool) {
        self.is_default_system_bar_light_theme.set(Some(is_light_mode));
    }

    fn notify_safe_area_changed(&self, safe_area_padding: Thickness) {
        self.safe_area_changed.on_safe_area_changed(SafeAreaChangedArgs::new(safe_area_padding));
    }

    fn notify_state_changed(&self, new_state: InputPaneState, start_rect: Option<Rect>, end_rect: Rect) {
        self.state_changed.on_state_changed(InputPaneStateEventArgs::new(new_state, start_rect, end_rect));
    }

    /// The theme of the system bars.
    pub fn system_bar_theme(&self) -> Option<SystemBarTheme> {
        Some(if self.window.appearance_light_status_bars() { SystemBarTheme::Light } else { SystemBarTheme::Dark })
    }

    /// Sets the theme of the system bars; `None` is the theme of the
    /// system.
    pub fn set_system_bar_theme(&self, value: Option<SystemBarTheme>) {
        self.status_bar_theme.set(value);

        if self.is_default_system_bar_light_theme.get().is_none() {
            self.is_default_system_bar_light_theme.set(Some(self.window.appearance_light_status_bars()));
        }

        let value = value.unwrap_or(if self.is_default_system_bar_light_theme.get() == Some(true) {
            SystemBarTheme::Light
        } else {
            SystemBarTheme::Dark
        });

        self.window.set_appearance_light_bars(value == SystemBarTheme::Light);
    }

    pub(crate) fn apply_status_bar_state(&self) {
        self.set_is_system_bar_visible(self.system_ui_visibility.get());
        self.set_system_bar_theme(self.status_bar_theme.get());
        self.set_system_bar_color(self.system_bar_color.get());
    }
}

/// The safe area of insets at a scaling: the bars and the cutout when the
/// window is displayed edge to edge, nothing otherwise.
pub(crate) fn safe_area_padding_of(insets: &RootInsets, displays_edge_to_edge: bool, render_scaling: f64) -> Thickness {
    if !displays_edge_to_edge {
        return Thickness::default();
    }
    Thickness::new(
        f64::from(insets.left) / render_scaling,
        f64::from(insets.top) / render_scaling,
        f64::from(insets.right) / render_scaling,
        f64::from(insets.bottom) / render_scaling,
    )
}

/// The rectangle of the client area the input method covers.
pub(crate) fn occluded_rect_of(
    insets: &RootInsets,
    render_scaling: f64,
    client_size: Size,
    safe_area_bottom: f64,
) -> Rect {
    let navbar_inset = insets.navigation_bars_bottom;
    let ime_inset = insets.ime_bottom;

    let height = f64::from((((ime_inset - navbar_inset) as f64 / render_scaling) as f32).max(0.0));

    Rect::new(0.0, client_size.height - safe_area_bottom - height, client_size.width, height)
}

impl IInsetsManager for AndroidInsetsManager {
    fn is_system_bar_visible(&self) -> Option<bool> {
        if !self.window.has_window() {
            return Some(true);
        }
        self.window.root_insets().map(|insets| insets.system_bars_visible)
    }

    fn set_is_system_bar_visible(&self, value: Option<bool>) {
        self.system_ui_visibility.set(value);

        self.window.set_system_bars_visible(value.unwrap_or(true));
    }

    fn display_edge_to_edge_preference(&self) -> bool {
        self.display_edge_to_edge_preference.get()
    }

    fn set_display_edge_to_edge_preference(&self, value: bool) {
        self.display_edge_to_edge_preference.set(value);

        self.update_display_edge_to_egde_state();
    }

    fn displays_edge_to_edge(&self) -> bool {
        self.displays_edge_to_edge.get()
    }

    fn safe_area_padding(&self) -> Thickness {
        match self.window.root_insets() {
            Some(insets) => safe_area_padding_of(&insets, self.displays_edge_to_edge(), self.render_scaling()),
            None => Thickness::default(),
        }
    }

    fn system_bar_color(&self) -> Option<Color> {
        self.system_bar_color.get()
    }

    fn set_system_bar_color(&self, value: Option<Color>) {
        self.system_bar_color.set(value);

        if self.is_display_edge_to_edge_forced {
            // Allow having fully transparent navbars when on api level 35
            if self.window.sdk_int() >= 36 {
                self.window.set_navigation_bar_contrast_enforced(false);
            } else if self.window.sdk_int() >= 35 {
                self.window.set_navigation_bar_contrast_enforced(value != Some(Colors::TRANSPARENT));
            }
            return;
        }

        if let Some(color) = value {
            if !self.displays_edge_to_edge.get() && self.window.has_window() {
                self.window.set_system_bar_color(color);
            }
        }
    }

    fn safe_area_changed(&self, handler: Rc<dyn Fn(&SafeAreaChangedArgs)>) -> Rc<dyn IDisposable> {
        self.safe_area_changed.safe_area_changed(handler)
    }
}

impl IInputPane for AndroidInsetsManager {
    fn state(&self) -> InputPaneState {
        self.state.get()
    }

    fn occluded_rect(&self) -> Rect {
        match self.window.root_insets() {
            Some(insets) => {
                occluded_rect_of(&insets, self.render_scaling(), self.client_size(), self.safe_area_padding().bottom)
            }
            None => Rect::default(),
        }
    }

    fn state_changed(&self, handler: Rc<dyn Fn(&InputPaneStateEventArgs)>) -> Rc<dyn IDisposable> {
        self.state_changed.state_changed(handler)
    }
}

#[cfg(target_os = "android")]
pub(crate) use imp::ActivityInsetsWindow;

#[cfg(target_os = "android")]
mod imp {
    use super::{IInsetsWindow, RootInsets};
    use crate::interop::java::{
        call_object, call_static_boolean, call_static_object, call_static_void, int_array_of, JavaClass, JavaObject,
        JavaValue,
    };
    use crate::interop::natives::{sdk_int, PLATFORM_HELPER};
    use ferroui_base::media::Color;

    /// The window of an activity, asked through the Java layer.
    pub(crate) struct ActivityInsetsWindow {
        activity: JavaObject,
    }

    impl ActivityInsetsWindow {
        /// The window of `activity`, which reports the insets it applies to
        /// the insets manager registered under `handle`.
        pub fn new(activity: JavaObject, handle: i64) -> Self {
            call_static_void(
                &JavaClass::find(PLATFORM_HELPER),
                "setInsetsListener",
                "(Landroid/app/Activity;J)V",
                &[JavaValue::Object(Some(&activity)), JavaValue::Long(handle)],
            );
            Self { activity }
        }

        fn call(&self, name: &str, value: bool) {
            call_static_void(
                &JavaClass::find(PLATFORM_HELPER),
                name,
                "(Landroid/app/Activity;Z)V",
                &[JavaValue::Object(Some(&self.activity)), JavaValue::Boolean(value)],
            );
        }
    }

    impl IInsetsWindow for ActivityInsetsWindow {
        fn sdk_int(&self) -> i32 {
            sdk_int()
        }

        fn has_window(&self) -> bool {
            call_object(&self.activity, "getWindow", "()Landroid/view/Window;", &[]).is_some()
        }

        fn root_insets(&self) -> Option<RootInsets> {
            let values = call_static_object(
                &JavaClass::find(PLATFORM_HELPER),
                "getRootInsets",
                "(Landroid/app/Activity;)[I",
                &[JavaValue::Object(Some(&self.activity))],
            )?;
            RootInsets::from_array(&int_array_of(&values))
        }

        fn is_display_edge_to_edge_forced(&self) -> bool {
            call_static_boolean(
                &JavaClass::find(PLATFORM_HELPER),
                "isDisplayEdgeToEdgeForced",
                "(Landroid/app/Activity;)Z",
                &[JavaValue::Object(Some(&self.activity))],
            )
        }

        fn set_layout_in_display_cutout_mode(&self, short_edges: bool) {
            self.call("setLayoutInDisplayCutoutMode", short_edges);
        }

        fn set_decor_fits_system_windows(&self, decor_fits_system_windows: bool) {
            self.call("setDecorFitsSystemWindows", decor_fits_system_windows);
        }

        fn add_translucent_bars(&self) {
            self.call("setTranslucentBars", true);
        }

        fn set_system_bar_color(&self, color: Color) {
            call_static_void(
                &JavaClass::find(PLATFORM_HELPER),
                "setSystemBarColor",
                "(Landroid/app/Activity;I)V",
                &[JavaValue::Object(Some(&self.activity)), JavaValue::Int(color.to_uint32() as i32)],
            );
        }

        fn set_navigation_bar_contrast_enforced(&self, enforced: bool) {
            self.call("setNavigationBarContrastEnforced", enforced);
        }

        fn appearance_light_status_bars(&self) -> bool {
            call_static_boolean(
                &JavaClass::find(PLATFORM_HELPER),
                "getAppearanceLightStatusBars",
                "(Landroid/app/Activity;)Z",
                &[JavaValue::Object(Some(&self.activity))],
            )
        }

        fn set_appearance_light_bars(&self, light: bool) {
            self.call("setAppearanceLightBars", light);
        }

        fn set_system_bars_visible(&self, visible: bool) {
            self.call("setSystemBarsVisible", visible);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the insets manager.
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct WindowState {
        sdk_int: Cell<i32>,
        forced: Cell<bool>,
        insets: Cell<Option<RootInsets>>,
        light_status_bars: Cell<bool>,
        calls: RefCell<Vec<String>>,
    }

    struct Window(Rc<WindowState>);

    impl IInsetsWindow for Window {
        fn sdk_int(&self) -> i32 {
            self.0.sdk_int.get()
        }

        fn has_window(&self) -> bool {
            true
        }

        fn root_insets(&self) -> Option<RootInsets> {
            self.0.insets.get()
        }

        fn is_display_edge_to_edge_forced(&self) -> bool {
            self.0.forced.get()
        }

        fn set_layout_in_display_cutout_mode(&self, short_edges: bool) {
            self.0.calls.borrow_mut().push(format!("cutout {short_edges}"));
        }

        fn set_decor_fits_system_windows(&self, decor_fits_system_windows: bool) {
            self.0.calls.borrow_mut().push(format!("fits {decor_fits_system_windows}"));
        }

        fn add_translucent_bars(&self) {
            self.0.calls.borrow_mut().push("translucent".to_string());
        }

        fn set_system_bar_color(&self, color: Color) {
            self.0.calls.borrow_mut().push(format!("color {:08x}", color.to_uint32()));
        }

        fn set_navigation_bar_contrast_enforced(&self, enforced: bool) {
            self.0.calls.borrow_mut().push(format!("contrast {enforced}"));
        }

        fn appearance_light_status_bars(&self) -> bool {
            self.0.light_status_bars.get()
        }

        fn set_appearance_light_bars(&self, light: bool) {
            self.0.light_status_bars.set(light);
            self.0.calls.borrow_mut().push(format!("light {light}"));
        }

        fn set_system_bars_visible(&self, visible: bool) {
            self.0.calls.borrow_mut().push(format!("visible {visible}"));
        }
    }

    struct TopLevel;

    impl IInsetsTopLevel for TopLevel {
        fn top_level_render_scaling(&self) -> f64 {
            2.0
        }

        fn top_level_client_size(&self) -> Size {
            Size::new(540.0, 1200.0)
        }
    }

    fn manager(sdk_int: i32, forced: bool) -> (Rc<AndroidInsetsManager>, Rc<WindowState>, Rc<dyn IInsetsTopLevel>) {
        let state = Rc::new(WindowState::default());
        state.sdk_int.set(sdk_int);
        state.forced.set(forced);
        let top_level: Rc<dyn IInsetsTopLevel> = Rc::new(TopLevel);
        let manager = AndroidInsetsManager::new(Box::new(Window(state.clone())), Rc::downgrade(&top_level));
        (manager, state, top_level)
    }

    const BARS: RootInsets = RootInsets {
        left: 0,
        top: 100,
        right: 0,
        bottom: 48,
        navigation_bars_bottom: 48,
        ime_bottom: 0,
        ime_visible: false,
        system_bars_visible: true,
    };

    #[test]
    fn the_values_of_the_java_layer_are_read() {
        assert_eq!(RootInsets::from_array(&[0, 100, 0, 48, 48, 0, 0, 1]), Some(BARS));
        assert_eq!(RootInsets::from_array(&[0, 100]), None);
    }

    #[test]
    fn a_window_that_is_not_edge_to_edge_has_no_safe_area() {
        let (manager, state, _top_level) = manager(34, false);
        state.insets.set(Some(BARS));

        assert!(!manager.displays_edge_to_edge());
        assert_eq!(manager.safe_area_padding(), Thickness::default());
        // The constructor applied the preference: no cutout, the decor fits.
        assert_eq!(*state.calls.borrow(), ["cutout false", "fits true"]);
    }

    #[test]
    fn the_preference_displays_edge_to_edge_and_the_safe_area_is_the_bars() {
        let (manager, state, _top_level) = manager(34, false);
        state.insets.set(Some(BARS));
        state.calls.borrow_mut().clear();

        manager.set_display_edge_to_edge_preference(true);

        assert!(manager.displays_edge_to_edge());
        assert_eq!(manager.safe_area_padding(), Thickness::new(0.0, 50.0, 0.0, 24.0));
        assert_eq!(*state.calls.borrow(), ["cutout true", "fits false", "translucent"]);
    }

    #[test]
    fn a_system_that_forces_edge_to_edge_ignores_the_preference() {
        let (manager, state, _top_level) = manager(36, true);
        state.insets.set(Some(BARS));

        assert!(manager.displays_edge_to_edge());
        assert!(!manager.display_edge_to_edge_preference());
        assert_eq!(manager.safe_area_padding(), Thickness::new(0.0, 50.0, 0.0, 24.0));
        assert!(state.calls.borrow().is_empty());

        // The colour of the bars cannot change; the contrast of the navigation bar is not enforced.
        manager.set_system_bar_color(Some(Color::from_argb(255, 1, 2, 3)));
        assert_eq!(manager.system_bar_color(), Some(Color::from_argb(255, 1, 2, 3)));
        assert_eq!(*state.calls.borrow(), ["contrast false"]);
    }

    #[test]
    fn on_api_35_the_contrast_is_enforced_unless_the_colour_is_transparent() {
        let (manager, state, _top_level) = manager(35, true);

        manager.set_system_bar_color(Some(Colors::TRANSPARENT));
        manager.set_system_bar_color(None);

        assert_eq!(*state.calls.borrow(), ["contrast false", "contrast true"]);
    }

    #[test]
    fn a_colour_is_set_on_a_window_that_is_not_edge_to_edge() {
        let (manager, state, _top_level) = manager(34, false);
        state.calls.borrow_mut().clear();

        manager.set_system_bar_color(Some(Color::from_argb(255, 0x10, 0x20, 0x30)));
        assert_eq!(*state.calls.borrow(), ["color ff102030"]);

        manager.set_display_edge_to_edge_preference(true);
        state.calls.borrow_mut().clear();
        manager.set_system_bar_color(Some(Color::from_argb(255, 1, 1, 1)));
        assert!(state.calls.borrow().is_empty());
    }

    #[test]
    fn the_occluded_rectangle_is_the_input_method_above_the_navigation_bar() {
        let (manager, state, _top_level) = manager(36, true);
        let mut insets = BARS;
        insets.ime_bottom = 848;
        insets.ime_visible = true;
        insets.bottom = 48;
        state.insets.set(Some(insets));

        // (848 - 48) / 2 = 400 high, above the bottom safe area of 24.
        assert_eq!(manager.occluded_rect(), Rect::new(0.0, 1200.0 - 24.0 - 400.0, 540.0, 400.0));

        state.insets.set(None);
        assert_eq!(manager.occluded_rect(), Rect::default());
    }

    #[test]
    fn applied_insets_report_the_safe_area_and_the_state_of_the_input_pane() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        // Android 10: the state of the input pane is reported when it changes.
        let (manager, state, _top_level) = manager(29, false);
        manager.set_display_edge_to_edge_preference(true);
        state.insets.set(Some(BARS));
        let safe_areas = Rc::new(RefCell::new(Vec::new()));
        let states = Rc::new(RefCell::new(Vec::new()));
        let _ = manager.safe_area_changed(Rc::new({
            let safe_areas = safe_areas.clone();
            move |e: &SafeAreaChangedArgs| safe_areas.borrow_mut().push(e.safe_area_padding())
        }));
        let _ = manager.state_changed(Rc::new({
            let states = states.clone();
            move |e: &InputPaneStateEventArgs| states.borrow_mut().push((e.new_state(), e.end_rect()))
        }));

        manager.on_apply_window_insets(true, false, 0);
        assert_eq!(*safe_areas.borrow(), [Thickness::new(0.0, 50.0, 0.0, 24.0)]);
        assert_eq!(manager.state(), InputPaneState::Closed);
        assert!(states.borrow().is_empty());

        let mut insets = BARS;
        insets.ime_bottom = 848;
        insets.ime_visible = true;
        state.insets.set(Some(insets));
        manager.on_apply_window_insets(true, true, 848);

        assert_eq!(manager.state(), InputPaneState::Open);
        assert_eq!(*states.borrow(), [(InputPaneState::Open, Rect::new(0.0, 776.0, 540.0, 400.0))]);
        assert_eq!(safe_areas.borrow().len(), 2);
    }

    #[test]
    fn on_android_11_a_change_of_the_inset_of_the_input_method_is_reported() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let (manager, state, _top_level) = manager(30, false);
        state.insets.set(Some(BARS));
        let states = Rc::new(RefCell::new(Vec::new()));
        let _ = manager.state_changed(Rc::new({
            let states = states.clone();
            move |e: &InputPaneStateEventArgs| states.borrow_mut().push(e.new_state())
        }));

        manager.on_apply_window_insets(true, false, 0);
        assert!(states.borrow().is_empty());
        manager.on_apply_window_insets(true, true, 700);
        manager.on_apply_window_insets(true, true, 700);

        assert_eq!(*states.borrow(), [InputPaneState::Open]);
    }

    #[test]
    fn the_theme_of_the_bars_falls_back_to_the_theme_of_the_system() {
        let (manager, state, _top_level) = manager(34, false);
        state.calls.borrow_mut().clear();

        manager.set_default_system_light_mode(true);
        manager.set_system_bar_theme(Some(SystemBarTheme::Dark));
        assert_eq!(manager.system_bar_theme(), Some(SystemBarTheme::Dark));
        manager.set_system_bar_theme(None);
        assert_eq!(manager.system_bar_theme(), Some(SystemBarTheme::Light));
        assert_eq!(*state.calls.borrow(), ["light false", "light true"]);
    }

    #[test]
    fn the_visibility_of_the_bars_is_asked_of_the_insets_and_set_on_the_window() {
        let (manager, state, _top_level) = manager(34, false);
        state.calls.borrow_mut().clear();

        assert_eq!(manager.is_system_bar_visible(), None);
        state.insets.set(Some(BARS));
        assert_eq!(manager.is_system_bar_visible(), Some(true));

        manager.set_is_system_bar_visible(Some(false));
        manager.set_is_system_bar_visible(None);
        assert_eq!(*state.calls.borrow(), ["visible false", "visible true"]);
    }
}
