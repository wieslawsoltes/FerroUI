//! The AppWindow, AppWindowTitleBar and Windowing pages: the windows their buttons open, with the
//! options of the pages, and the handlers of the source's sample windows. The pages live in a
//! window of the mock windowing platform (an 800 by 600 client area on a 1280 by 1024 screen); the
//! platform's own window chrome is not tested here. The pages are laid out in their window, so
//! the tests use the WinUI theme (the controls of WinUI have their templates there).

use crate::app_window_samples::{
    self, centered, compact_overlay_size, decorations, initial_size_description, title_bar_height,
    title_bar_theme, validate_dimensions, COMPACT_OVERLAY_SIZES, NO_WINDOWS, TITLE_BAR_COLORS,
    TITLE_BAR_HEIGHT_OPTIONS, TITLE_BAR_THEMES,
};
use crate::{examples, tests::start_winui as start, window_samples};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::Color;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelPoint, PixelRect, Ref};
use ferroui_controls::media::MicaBackdrop;
use ferroui_controls::platform::{ITopLevelImpl, IWindowBaseImpl};
use ferroui_controls::testing::MockWindowImpl;
use ferroui_controls::{
    Border, Button, CheckBox, ComboBox, Control, InfoBar, NumberBox, TextBlock, TextBox, TitleBar,
    ToggleSwitch, Window, WindowDecorations, WindowState,
};
use ferroui_controls_color_picker::ColorView;
use std::rc::Rc;

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap()
}

/// The page in a shown window of the mock platform with `scaling`, as the gallery's main window.
fn host(root: &Ref<Control>, scaling: f64) -> (Ref<Window>, Rc<MockWindowImpl>) {
    let platform = MockWindowImpl::window(800.0, 600.0);
    platform.render_scaling.set(scaling);
    let window = Window::with_impl(platform.clone());
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    (window, platform)
}

fn click(root: &Ref<Control>, name: &str) {
    root.get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

fn content(window: &Ref<Window>) -> Ref<Control> {
    window
        .content()
        .as_ref()
        .and_then(Control::from_boxed)
        .expect("the sample window has its content")
}

/// The one sample window the last click opened.
fn opened() -> Ref<Window> {
    let windows = window_samples::opened_windows();
    assert_eq!(windows.len(), 1, "one sample window is open");
    windows[0].clone()
}

fn close_all() {
    for window in window_samples::opened_windows() {
        window.close();
    }
    assert!(window_samples::opened_windows().is_empty());
}

/// Fires the timers of the source's `Task.Delay(3000)`.
fn fire_timers() {
    for timer in Dispatcher::timers_for_unit_tests() {
        Dispatcher::force_fire_timer_for_unit_tests(&timer);
    }
}

fn error_text(root: &Ref<Control>) -> Option<String> {
    let error = root.get_control::<TextBlock>("WindowSampleError");
    error.is_visible().then(|| error.text().unwrap_or_default())
}

fn solid_color(brush: Option<Rc<dyn ferroui_base::media::IBrush>>) -> Option<Color> {
    brush.and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
}

#[test]
fn app_window_page_reports_a_platform_without_windows() {
    let _application = start();
    // Not in a window: the page of a single-view platform.
    for (id, buttons) in [
        (
            "AppWindow",
            &[
                "ShowSampleWindow1",
                "ShowSampleWindow5",
                "ShowSampleWindow7",
            ][..],
        ),
        (
            "AppWindowTitleBar",
            &[
                "ShowWindowButton",
                "ShowExtendButton",
                "ShowThemeHeightButton",
            ][..],
        ),
        (
            "Windowing",
            &["CreateWindowButton", "OpenConfiguredWindowButton"][..],
        ),
    ] {
        let root = page(id);
        assert_eq!(error_text(&root), None);
        for button in buttons {
            click(&root, button);
            assert_eq!(
                error_text(&root).as_deref(),
                Some(NO_WINDOWS),
                "{id}: {button}"
            );
            assert!(window_samples::opened_windows().is_empty());
        }
    }
}

#[test]
fn app_window_creates_the_customized_window_and_hides_and_shows_it() {
    let _application = start();
    let root = page("AppWindow");
    let (_host, _) = host(&root, 2.0);
    root.get_control::<TextBox>("WindowTitle")
        .set_text(Some("Sample"));
    root.get_control::<NumberBox>("WindowWidth")
        .set_numeric_value(600.0);
    root.get_control::<NumberBox>("WindowHeight")
        .set_numeric_value(400.0);
    root.get_control::<NumberBox>("XPoint")
        .set_numeric_value(70.0);
    root.get_control::<NumberBox>("YPoint")
        .set_numeric_value(30.0);
    click(&root, "ShowSampleWindow1");
    assert_eq!(error_text(&root), None);
    let window = opened();
    assert_eq!(window.title().as_deref(), Some("Sample"));
    // Resize takes physical pixels: the client area at a scaling of 2.
    assert_eq!((window.width(), window.height()), (300.0, 200.0));
    assert_eq!(window.position(), PixelPoint::new(70, 30));
    assert!(window
        .system_backdrop()
        .and_then(|backdrop| backdrop.cast::<MicaBackdrop>())
        .is_some());
    assert!(window.is_visible());
    let content = content(&window);
    // Hide_Click.
    click(&content, "Hide");
    assert!(!window.is_visible());
    // Show_Click: hidden at once, shown again after the delay.
    window.show();
    click(&content, "Show");
    assert!(!window.is_visible());
    fire_timers();
    assert!(window.is_visible());
    // Close_Click.
    click(&content, "Close");
    assert!(window_samples::opened_windows().is_empty());
}

#[test]
fn app_window_centers_the_window_in_the_work_area() {
    let _application = start();
    let area = PixelRect::new(100, 50, 1000, 800);
    assert_eq!(centered(area, (400, 300)), PixelPoint::new(400, 300));
    let root = page("AppWindow");
    let (_host, _) = host(&root, 1.0);
    click(&root, "ShowSampleWindow2");
    let window = opened();
    // The mock screen's working area is the whole 1280 by 1024 screen.
    let size = window.client_size();
    let expected = centered(
        PixelRect::new(0, 0, 1280, 1024),
        (size.width.round() as i32, size.height.round() as i32),
    );
    assert_eq!(window.position(), expected);
    close_all();
}

#[test]
fn app_window_overlapped_presenter_applies_the_options_and_drives_the_window_state() {
    let _application = start();
    let root = page("AppWindow");
    let (_host, _) = host(&root, 1.0);
    let toggle = |name: &str| root.get_control::<ToggleSwitch>(name);
    // HasBorder_Toggled and HasTitleBar_Toggled.
    toggle("HasBorder").set_is_checked(Some(false));
    assert_eq!(toggle("HasTitleBar").is_checked(), Some(false));
    toggle("HasTitleBar").set_is_checked(Some(true));
    assert_eq!(toggle("HasBorder").is_checked(), Some(true));
    toggle("HasTitleBar").set_is_checked(Some(false));
    toggle("IsAlwaysOnTop").set_is_checked(Some(true));
    toggle("IsMinimizable").set_is_checked(Some(false));
    click(&root, "ShowSampleWindow3");
    let window = opened();
    assert!(window.topmost());
    assert!(window.can_maximize());
    assert!(!window.can_minimize());
    assert!(window.can_resize());
    assert_eq!(window.window_decorations(), WindowDecorations::BorderOnly);
    let content = content(&window);
    let maximize = content.get_control::<Button>("MaximizeRestoreBtn");
    let label = || {
        maximize
            .content()
            .and_then(|content| content.downcast_ref::<String>().cloned())
    };
    // MaximizeRestoreBtn_Click and SampleWindow3_SizeChanged.
    click(&content, "MaximizeRestoreBtn");
    assert_eq!(window.window_state(), WindowState::Maximized);
    assert_eq!(label().as_deref(), Some("Restore"));
    click(&content, "MaximizeRestoreBtn");
    assert_eq!(window.window_state(), WindowState::Normal);
    assert_eq!(label().as_deref(), Some("Maximize"));
    // MinimizeBtn_Click, and RestoreBtn_Click: restored after the delay.
    click(&content, "MinimizeBtn");
    assert_eq!(window.window_state(), WindowState::Minimized);
    window.set_window_state(WindowState::Normal);
    click(&content, "RestoreBtn");
    assert_eq!(window.window_state(), WindowState::Minimized);
    fire_timers();
    assert_eq!(window.window_state(), WindowState::Normal);
    click(&content, "CloseBtn");
    assert!(window_samples::opened_windows().is_empty());
    // SetBorderAndTitleBar.
    assert_eq!(decorations(true, true), WindowDecorations::Full);
    assert_eq!(decorations(true, false), WindowDecorations::BorderOnly);
    assert_eq!(decorations(false, false), WindowDecorations::None);
}

#[test]
fn app_window_sets_the_minimum_and_maximum_size_without_maximize() {
    let _application = start();
    let root = page("AppWindow");
    let (_host, _) = host(&root, 2.0);
    root.get_control::<NumberBox>("MinWidthBox")
        .set_numeric_value(300.0);
    root.get_control::<NumberBox>("MaxHeightBox")
        .set_numeric_value(1200.0);
    click(&root, "ShowSampleWindow4");
    let window = opened();
    // Physical pixels at a scaling of 2.
    assert_eq!((window.width(), window.height()), (400.0, 250.0));
    assert_eq!((window.min_width(), window.min_height()), (150.0, 200.0));
    assert_eq!((window.max_width(), window.max_height()), (500.0, 600.0));
    assert!(!window.can_maximize());
    let content = content(&window);
    assert!(!content
        .get_control::<Button>("MaximizeRestoreBtn")
        .is_visible());
    click(&content, "RestoreBtn");
    assert_eq!(window.window_state(), WindowState::Minimized);
    fire_timers();
    assert_eq!(window.window_state(), WindowState::Normal);
    close_all();
}

#[test]
fn app_window_opens_the_modal_window_owned_by_the_gallery_window() {
    let _application = start();
    let root = page("AppWindow");
    let (host, platform) = host(&root, 1.0);
    click(&root, "ShowSampleWindow5");
    let window = opened();
    assert!(window.is_dialog());
    assert_eq!(
        window.owner().map(|owner| owner.upcast::<Control>()),
        Some(host.clone().upcast::<Control>())
    );
    assert_eq!((window.width(), window.height()), (400.0, 300.0));
    assert!(!window.can_resize());
    assert!(!window.can_minimize());
    assert!(!window.can_maximize());
    platform.clear_calls();
    // OKButton_Click closes it; ModalWindow_Closed activates the owner.
    click(&content(&window), "OKButton");
    assert!(window_samples::opened_windows().is_empty());
    assert!(
        platform.count(|call| matches!(call, ferroui_controls::testing::MockCall::Activate)) >= 1
    );
    click(&root, "ShowSampleWindow5");
    click(&content(&opened()), "CancelButton");
    assert!(window_samples::opened_windows().is_empty());
}

#[test]
fn app_window_opens_the_full_screen_and_compact_overlay_windows() {
    let _application = start();
    let root = page("AppWindow");
    let (_host, _) = host(&root, 1.0);
    click(&root, "ShowSampleWindow6");
    let window = opened();
    assert_eq!(window.window_state(), WindowState::FullScreen);
    click(&content(&window), "Close");
    assert!(window_samples::opened_windows().is_empty());

    // InitialSize_SelectionChanged.
    let sizes = root.get_control::<ComboBox>("InitialSize");
    let description = root.get_control::<TextBlock>("InitialSizeDescription");
    assert_eq!(
        description.text().as_deref(),
        Some("Small: Window size is approximately 5% of the display's work area.")
    );
    sizes.set_selected_index(1);
    assert_eq!(
        description.text().as_deref(),
        Some("Medium: Window size is approximately 15% of the display's work area.")
    );
    assert_eq!(
        initial_size_description(2),
        "Large: Window size is approximately 25% of the display's work area."
    );
    assert_eq!(
        initial_size_description(-1),
        "Unknown: Window size is approximately Unknown of the display's work area."
    );
    // The compact overlay window: topmost, no minimize or maximize, 15% of the work area.
    click(&root, "ShowSampleWindow7");
    let window = opened();
    assert!(window.topmost());
    assert!(!window.can_minimize());
    assert!(!window.can_maximize());
    let (width, height) =
        compact_overlay_size(PixelRect::new(0, 0, 1280, 1024), COMPACT_OVERLAY_SIZES[1].2);
    assert_eq!((window.width(), window.height()), (width, height));
    assert!((width * height / (1280.0 * 1024.0) - 0.15).abs() < 1e-9);
    close_all();
}

#[test]
fn app_window_title_bar_colors_follow_the_options_and_the_activation() {
    let _application = start();
    let root = page("AppWindowTitleBar");
    let (_host, _) = host(&root, 1.0);
    assert_eq!(
        examples::GALLERY_LINKS
            .iter()
            .filter(|link| link.0 == "AppWindowTitleBar")
            .count(),
        1
    );
    assert!(root.find_control::<Control>("TitleBarPageLink").is_some());
    let state = app_window_samples::title_bar_page(&root).expect("the page's state");
    for (name, color) in TITLE_BAR_COLORS {
        assert_eq!(state.color(name), Color::from_uint32(color), "{name}");
        assert_eq!(
            root.get_control::<ColorView>(&format!("{name}Picker"))
                .color(),
            Color::from_uint32(color)
        );
    }
    // ShowWindowButton_Click: the button is disabled while the window is open.
    click(&root, "ShowWindowButton");
    assert!(!root.get_control::<Button>("ShowWindowButton").is_enabled());
    let window = opened();
    assert!(window.topmost());
    assert!(!window.can_resize());
    assert_eq!((window.width(), window.height()), (600.0, 400.0));
    assert!(window.extend_client_area_to_decorations_hint());
    let title_bar = content(&window).get_control::<TitleBar>("titleBar");
    // Shown and activated: the active colours.
    assert!(window.is_active());
    assert_eq!(
        solid_color(title_bar.background()),
        Some(Color::from_uint32(0xFFF2F6FA))
    );
    assert_eq!(
        solid_color(title_bar.foreground()),
        Some(Color::from_uint32(0xFF1E2933))
    );
    // Background_ColorChanged sets the colour of the open window.
    let red = Color::from_rgb(0xFF, 0, 0);
    root.get_control::<ColorView>("BackgroundPicker")
        .set_color(red);
    assert_eq!(state.color("Background"), red);
    assert_eq!(solid_color(title_bar.background()), Some(red));
    assert_eq!(
        solid_color(root.get_control::<Border>("BackgroundPreview").background()),
        Some(red)
    );
    // Deactivated: the inactive colours.
    let platform = window.platform_impl().unwrap();
    IWindowBaseImpl::deactivated(&*platform).expect("the window hears its deactivation")();
    assert!(!window.is_active());
    assert_eq!(
        solid_color(title_bar.background()),
        Some(Color::from_uint32(0xFFE5EAF0))
    );
    assert_eq!(
        solid_color(title_bar.foreground()),
        Some(Color::from_uint32(0xFF6B7280))
    );
    IWindowBaseImpl::activated(&*platform).expect("the window hears its activation")();
    assert_eq!(solid_color(title_bar.background()), Some(red));
    // Window_Closed enables the button again.
    window.close();
    assert!(root.get_control::<Button>("ShowWindowButton").is_enabled());
    assert!(state.window().is_none() || window_samples::opened_windows().is_empty());
}

#[test]
fn app_window_title_bar_extends_the_content_and_sets_the_height_and_theme() {
    let _application = start();
    let root = page("AppWindowTitleBar");
    let (_host, _) = host(&root, 1.0);
    let heights = root.get_control::<ComboBox>("HeightComboBox");
    assert_eq!(heights.items().count(), TITLE_BAR_HEIGHT_OPTIONS.len());
    assert_eq!(heights.selected_index(), 0);
    heights.set_selected_index(1);
    // ShowExtendButton_Click with the Tall option.
    click(&root, "ShowExtendButton");
    let window = opened();
    assert!(!root.get_control::<Button>("ShowExtendButton").is_enabled());
    assert!(window.extend_client_area_to_decorations_hint());
    assert_eq!(window.extend_client_area_title_bar_height_hint(), 48.0);
    // HeightComboBox_SelectionChanged while the content extends into the title bar.
    heights.set_selected_index(2);
    assert_eq!(window.extend_client_area_title_bar_height_hint(), 0.0);
    // ExtendContentCheckBox_Unchecked; the height no longer changes.
    let check = root.get_control::<CheckBox>("ExtendContentCheckBox");
    check.set_is_checked(Some(false));
    assert!(!window.extend_client_area_to_decorations_hint());
    heights.set_selected_index(0);
    assert_eq!(window.extend_client_area_title_bar_height_hint(), 0.0);
    check.set_is_checked(Some(true));
    assert!(window.extend_client_area_to_decorations_hint());
    window.close();
    assert!(root.get_control::<Button>("ShowExtendButton").is_enabled());
    assert_eq!(title_bar_height(0), -1.0);
    assert_eq!(title_bar_height(1), 48.0);

    // ShowThemeButton_Click and ThemeComboBox_SelectionChanged.
    let themes = root.get_control::<ComboBox>("ThemeComboBox");
    assert_eq!(themes.items().count(), TITLE_BAR_THEMES.len());
    assert_eq!(themes.selected_index(), 1);
    click(&root, "ShowThemeHeightButton");
    let window = opened();
    assert!(window.topmost());
    assert_eq!(window.requested_theme_variant(), None);
    themes.set_selected_index(3);
    assert_eq!(window.requested_theme_variant(), Some(ThemeVariant::dark()));
    themes.set_selected_index(0);
    assert_eq!(
        window.requested_theme_variant(),
        Some(ThemeVariant::light())
    );
    assert_eq!(title_bar_theme(2), Some(ThemeVariant::light()));
    window.close();
    assert!(root
        .get_control::<Button>("ShowThemeHeightButton")
        .is_enabled());
    assert!(window_samples::opened_windows().is_empty());
}

#[test]
fn windowing_creates_a_child_window_with_a_physical_client_size() {
    let _application = start();
    let root = page("Windowing");
    let (_host, platform) = host(&root, 1.0);
    ITopLevelImpl::scaling_changed(&*platform).expect("the window hears its scaling")(1.25);
    click(&root, "CreateWindowButton");
    let window = opened();
    assert!(window.extend_client_area_to_decorations_hint());
    assert!(window
        .system_backdrop()
        .and_then(|backdrop| backdrop.cast::<MicaBackdrop>())
        .is_some());
    assert_eq!((window.width(), window.height()), (400.0, 400.0));
    assert_eq!(
        window.requested_theme_variant(),
        root.actual_theme_variant()
    );
    let text = content(&window).cast::<TextBlock>().expect("a text");
    assert_eq!(text.text().as_deref(), Some("New child window!"));
    close_all();
}

#[test]
fn windowing_validates_the_dimensions_and_opens_the_configured_window() {
    let _application = start();
    assert_eq!(
        validate_dimensions((f64::NAN, 480.0), (320.0, 240.0), (960.0, 720.0)),
        Some("Enter a value for every dimension.")
    );
    assert_eq!(
        validate_dimensions((1000.0, 480.0), (320.0, 240.0), (960.0, 720.0)),
        Some("Width and Height must be within their minimum and maximum limits.")
    );
    assert_eq!(
        validate_dimensions((640.0, 480.0), (320.0, 240.0), (960.0, 720.0)),
        None
    );
    let root = page("Windowing");
    let (_host, _) = host(&root, 2.0);
    let info_bar = root.get_control::<InfoBar>("SizeValidationInfoBar");
    assert!(!info_bar.is_open());
    root.get_control::<NumberBox>("ClientWidthInput")
        .set_numeric_value(100.0);
    click(&root, "OpenConfiguredWindowButton");
    assert!(info_bar.is_open());
    assert_eq!(
        info_bar.message(),
        "Width and Height must be within their minimum and maximum limits."
    );
    assert!(window_samples::opened_windows().is_empty());
    root.get_control::<NumberBox>("ClientWidthInput")
        .set_numeric_value(640.0);
    click(&root, "OpenConfiguredWindowButton");
    assert!(!info_bar.is_open());
    let window = opened();
    assert_eq!(
        window.title().as_deref(),
        Some("Window client size and constraints (experimental)")
    );
    // Device-independent pixels, whatever the scaling.
    assert_eq!((window.width(), window.height()), (640.0, 480.0));
    assert_eq!((window.min_width(), window.min_height()), (320.0, 240.0));
    assert_eq!((window.max_width(), window.max_height()), (960.0, 720.0));
    assert!(!window.can_maximize());
    let text = content(&window).cast::<TextBlock>().expect("a text");
    assert_eq!(
        text.text().as_deref(),
        Some(
            "Initial client area: 640 by 480 DIPs. Width is constrained to 320 to 960 DIPs and \
             height to 240 to 720 DIPs. Try resizing this window."
        )
    );
    close_all();
}
