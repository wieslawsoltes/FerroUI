//! Port of `Pages/WindowPage.xaml.cs`: the class of the document `Pages/WindowPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::{ShowWindowTest, TopmostWindowTest};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, CornerRadius, Ref, Size, Visual};
use ferroui_controls::automation::{AccessibilityView, AutomationProperties};
use ferroui_controls::primitives::popup_positioning::{PopupAnchor, PopupGravity};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{
    Application, Border, CheckBox, ComboBox, Control, PlacementMode, TextBox, TopLevel, UserControl, Window,
    WindowDecorations, WindowStartupLocation, WindowState, WindowTransparencyLevel, WindowTransparencyLevelCollection,
};
use std::rc::Rc;

#[repr(C)]
pub struct WindowPage {
    base: UserControl,
}

user_control_class!(WindowPage);
ferro_class_info!(WindowPage {
    new: WindowPage::new,
    markup: {
        methods: [
            fn ShowWindow_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.show_window_click(&sender, e.as_routed_event_args())
                },
            fn SendToBack_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.send_to_back_click(&sender, e.as_routed_event_args())
                },
            fn EnterFullscreen_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.enter_fullscreen_click(&sender, e.as_routed_event_args())
                },
            fn ExitFullscreen_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.exit_fullscreen_click(&sender, e.as_routed_event_args())
                },
            fn RestoreAll_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.restore_all_click(&sender, e.as_routed_event_args())
                },
            fn ShowTopmostWindow_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.show_topmost_window_click(&sender, e.as_routed_event_args())
                },
            fn ShowTransparentWindow_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.show_transparent_window_click(&sender, e.as_routed_event_args())
                },
            fn ShowTransparentPopup_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.show_transparent_popup_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(WindowPage, "/Pages/WindowPage.xaml");

/// `(WindowStartupLocation)index`. An index that is no member (no item of the combo box is
/// selected) names no location: the property keeps its value.
fn window_startup_location(index: i32) -> Option<WindowStartupLocation> {
    match index {
        0 => Some(WindowStartupLocation::Manual),
        1 => Some(WindowStartupLocation::CenterScreen),
        2 => Some(WindowStartupLocation::CenterOwner),
        _ => None,
    }
}

/// `(WindowDecorations)index`; see [`window_startup_location`].
fn window_decorations(index: i32) -> Option<WindowDecorations> {
    match index {
        0 => Some(WindowDecorations::None),
        1 => Some(WindowDecorations::BorderOnly),
        2 => Some(WindowDecorations::Full),
        _ => None,
    }
}

/// `(WindowState)index`; see [`window_startup_location`].
fn window_state(index: i32) -> Option<WindowState> {
    match index {
        0 => Some(WindowState::Normal),
        1 => Some(WindowState::Minimized),
        2 => Some(WindowState::Maximized),
        3 => Some(WindowState::FullScreen),
        _ => None,
    }
}

/// The windows of the classic desktop lifetime of the application; none under another
/// lifetime.
fn lifetime_windows() -> Option<Vec<Ref<Window>>> {
    let lifetime = Application::current().and_then(|application| application.application_lifetime());
    lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()).map(|l| l.windows())
}

/// A red circle: the content of the transparent window and of the transparent popup.
fn red_circle() -> Ref<Border> {
    let border = Border::new();
    let red: Rc<dyn IBrush> = Brushes::red();
    border.set_background(Some(red));
    border.set_corner_radius(CornerRadius::uniform(100.0));
    border
}

impl WindowPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// `Window`: the window the page is attached to.
    ///
    /// # Panics
    /// Panics if the page is not attached to a window (the internal exception of the managed
    /// original).
    fn window(&self) -> Ref<Window> {
        let visual: &Visual = self;
        TopLevel::get_top_level(Some(visual))
            .and_then(|top_level| top_level.cast::<Window>())
            .unwrap_or_else(|| panic!("WindowPage is not attached to a Window."))
    }

    fn check_box(&self, name: &str) -> Ref<CheckBox> {
        self.get_control::<CheckBox>(name)
    }

    fn combo_box(&self, name: &str) -> Ref<ComboBox> {
        self.get_control::<ComboBox>(name)
    }

    fn show_window_size(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("ShowWindowSize")
    }

    /// # Panics
    /// Panics if the text of the size is not a size (a format exception in the managed
    /// original).
    fn show_window_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let size_text = self.show_window_size().text().unwrap_or_default();
        let size: Option<Size> = if !size_text.trim().is_empty() {
            Some(Size::parse(&size_text).unwrap_or_else(|error| panic!("{error:?}")))
        } else {
            None
        };
        let can_resize = self.check_box("ShowWindowCanResize").is_checked().unwrap_or(false);
        let window = ShowWindowTest::new();
        if let Some(location) = window_startup_location(self.combo_box("ShowWindowLocation").selected_index()) {
            window.set_window_startup_location(location);
        }
        window.set_can_resize(can_resize);
        window.set_can_minimize(self.check_box("ShowWindowCanMinimize").is_checked().unwrap_or(false));
        window.set_can_maximize(can_resize && self.check_box("ShowWindowCanMaximize").is_checked().unwrap_or(false));

        if let Some(windows) = lifetime_windows() {
            // Make sure the windows have unique names and AutomationIds.
            let existing = windows.iter().filter(|window| window.is::<ShowWindowTest>()).count();
            if existing > 0 {
                AutomationProperties::set_automation_id(
                    &window,
                    Some(&format!("{}{}", window.name().unwrap_or_default(), existing + 1)),
                );
                window.set_title(Some(format!("{} {}", window.title().unwrap_or_default(), existing + 1)));
            }
        }

        if let Some(size) = size {
            window.set_width(size.width);
            window.set_height(size.height);
        }

        self.show_window_size().set_text(Some(""));
        window.set_extend_client_area_to_decorations_hint(
            self.check_box("ShowWindowExtendClientAreaToDecorationsHint").is_checked().unwrap_or(false),
        );
        if let Some(decorations) = window_decorations(self.combo_box("ShowWindowSystemDecorations").selected_index()) {
            window.set_window_decorations(decorations);
        }
        if let Some(state) = window_state(self.combo_box("ShowWindowState").selected_index()) {
            window.set_window_state(state);
        }

        match self.combo_box("ShowWindowMode").selected_index() {
            0 => window.show(),
            1 => window.show_with_owner(&self.window()),
            2 => drop(window.show_dialog(&self.window())),
            _ => {}
        }
    }

    fn show_transparent_window_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        // Show a background window to make sure the color behind the transparent window is
        // a known color (green).
        let background_window = Window::new();
        background_window.set_title(Some("Transparent Window Background".to_string()));
        background_window.set_name(Some("TransparentWindowBackground".to_string()));
        background_window.set_width(300.0);
        background_window.set_height(300.0);
        let green: Rc<dyn IBrush> = Brushes::green();
        background_window.set_background(Some(green));
        background_window.set_window_startup_location(WindowStartupLocation::CenterOwner);

        // This is the transparent window with a red circle.
        let window = Window::new();
        window.set_title(Some("Transparent Window".to_string()));
        window.set_name(Some("TransparentWindow".to_string()));
        window.set_window_decorations(WindowDecorations::None);
        let transparent: Rc<dyn IBrush> = Brushes::transparent();
        window.set_background(Some(transparent));
        window.set_transparency_level_hint(WindowTransparencyLevelCollection::new([WindowTransparencyLevel::transparent()]));
        window.set_window_startup_location(WindowStartupLocation::CenterOwner);
        window.set_width(200.0);
        window.set_height(200.0);
        window.set_content(Some(Control::boxed(&red_circle())));

        {
            // The window holds the handlers of its events: the handler holds it weakly.
            let weak_window = window.downgrade();
            let background_window = background_window.clone();
            window.add_handler(InputElement::pointer_pressed_event(), move |_, _| {
                if let Some(window) = weak_window.upgrade() {
                    window.close();
                }
                background_window.close();
            });
        }

        background_window.show_with_owner(&self.window());
        window.show_with_owner(&background_window);
    }

    fn show_transparent_popup_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let popup = Popup::new();
        popup.set_window_manager_add_shadow_hint(false);
        popup.set_placement(PlacementMode::AnchorAndGravity);
        popup.set_placement_anchor(PopupAnchor::TOP);
        popup.set_placement_gravity(PopupGravity::BOTTOM);
        popup.set_width(200.0);
        popup.set_height(200.0);
        popup.set_child(&red_circle());

        // Show a background window to make sure the color behind the transparent window is
        // a known color (green).
        let background_window = Window::new();
        background_window.set_title(Some("Transparent Popup Background".to_string()));
        background_window.set_name(Some("TransparentPopupBackground".to_string()));
        background_window.set_width(200.0);
        background_window.set_height(200.0);
        let green: Rc<dyn IBrush> = Brushes::green();
        background_window.set_background(Some(green));
        background_window.set_window_decorations(WindowDecorations::None);
        background_window.set_window_startup_location(WindowStartupLocation::CenterOwner);
        let popup_container = Border::new();
        popup_container.set_name(Some("PopupContainer".to_string()));
        popup_container.set_child(&popup);
        AutomationProperties::set_accessibility_view(&popup_container, AccessibilityView::Content);
        background_window.set_content(Some(Control::boxed(&popup_container)));

        {
            // The popup is in the window: its handler holds the window weakly.
            let weak_background_window = background_window.downgrade();
            popup.add_handler(InputElement::pointer_pressed_event(), move |_, _| {
                if let Some(background_window) = weak_background_window.upgrade() {
                    background_window.close();
                }
            });
        }
        background_window.show_with_owner(&self.window());

        popup.open();
    }

    /// # Panics
    /// Panics if the application does not run under the classic desktop lifetime (a null
    /// reference or an invalid cast in the managed original).
    fn send_to_back_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let windows = lifetime_windows().expect("the application runs under the classic desktop lifetime");

        for window in windows {
            window.activate();
        }
    }

    fn enter_fullscreen_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.window().set_window_state(WindowState::FullScreen);
    }

    fn exit_fullscreen_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.window().set_window_state(WindowState::Normal);
    }

    /// # Panics
    /// Panics if the application does not run under the classic desktop lifetime (a null
    /// reference or an invalid cast in the managed original).
    fn restore_all_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let windows = lifetime_windows().expect("the application runs under the classic desktop lifetime");

        for window in windows {
            window.show();
            if window.window_state() == WindowState::Minimized {
                window.set_window_state(WindowState::Normal);
            }
        }
    }

    fn show_topmost_window_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let main_window = TopmostWindowTest::new("OwnerWindow");
        main_window.set_topmost(true);
        main_window.set_title(Some("Owner Window".to_string()));
        let owned_window = TopmostWindowTest::new("OwnedWindow");
        owned_window.set_window_startup_location(WindowStartupLocation::CenterOwner);
        owned_window.set_title(Some("Owned Window".to_string()));

        main_window.show();
        owned_window.show_with_owner(&main_window);
    }
}
