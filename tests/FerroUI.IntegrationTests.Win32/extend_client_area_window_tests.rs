use crate::unmanaged_methods::{get_system_metrics, SM_CMONITORS};
use crate::window_extensions::{get_screen_at_index, get_win32_client_size, get_win32_window_bounds, when_loaded};
use crate::{TestCase, WindowGuard};
use ferroui_base::media::Brushes;
use ferroui_base::{PixelPoint, PixelSize, Ref, Size, Thickness, Visual};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Border, Control, Window, WindowDecorations, WindowStartupLocation, WindowState};

const CLIENT_WIDTH: i32 = 200;
const CLIENT_HEIGHT: i32 = 200;

/// The automation identifier of the title bar of the drawn decorations
/// (upstream's starts with the name of its project).
const TITLE_BAR_AUTOMATION_ID: &str = "FerroTitleBar";

/// The classes that derive from the test class upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    DecorationsFull,
    DecorationsBorderOnly,
    DecorationsNone,
}

impl Variant {
    fn name(self) -> &'static str {
        match self {
            Variant::DecorationsFull => "decorations_full",
            Variant::DecorationsBorderOnly => "decorations_border_only",
            Variant::DecorationsNone => "decorations_none",
        }
    }

    fn decorations(self) -> WindowDecorations {
        match self {
            Variant::DecorationsFull => WindowDecorations::Full,
            Variant::DecorationsBorderOnly => WindowDecorations::BorderOnly,
            Variant::DecorationsNone => WindowDecorations::None,
        }
    }

    fn verify_normal_state(self, window: &Window, _can_resize: bool) {
        match self {
            Variant::DecorationsFull => {
                assert_has_border(window);
                assert_large_title_bar_with_buttons(window);
            }
            Variant::DecorationsBorderOnly => {
                assert_has_border(window);
                assert_no_title_bar(window);
            }
            Variant::DecorationsNone => {
                assert_no_border(window);
                assert_no_title_bar(window);
            }
        }
    }

    fn verify_maximized_state(self, window: &Window) {
        match self {
            Variant::DecorationsFull => assert_large_title_bar_with_buttons(window),
            Variant::DecorationsBorderOnly | Variant::DecorationsNone => assert_no_title_bar(window),
        }
    }
}

const VARIANTS: [Variant; 3] = [Variant::DecorationsFull, Variant::DecorationsBorderOnly, Variant::DecorationsNone];

const WINDOW_STATES: [WindowState; 4] =
    [WindowState::Normal, WindowState::Minimized, WindowState::Maximized, WindowState::FullScreen];

/// The matrix of the theories: every screen, every window state, resizable
/// or not.
fn states() -> Vec<(usize, WindowState, bool)> {
    let mut states = Vec::new();
    for screen_index in 0..get_system_metrics(SM_CMONITORS).max(0) as usize {
        for state in WINDOW_STATES {
            for can_resize in [true, false] {
                states.push((screen_index, state, can_resize));
            }
        }
    }
    states
}

pub fn tests(cases: &mut Vec<TestCase>) {
    for variant in VARIANTS {
        for (screen_index, state, can_resize) in states() {
            let arguments = format!("(screen_index: {screen_index}, initial_state: {state:?}, can_resize: {can_resize})");
            let name = variant.name();
            cases.push(TestCase::new(
                format!("extend_client_area_window_tests::{name}::normal_state_respects_client_size{arguments}"),
                move || normal_state_respects_client_size(variant, screen_index, state, can_resize),
            ));
            cases.push(TestCase::new(
                format!("extend_client_area_window_tests::{name}::maximized_state_fills_screen_working_area{arguments}"),
                move || maximized_state_fills_screen_working_area(variant, screen_index, state, can_resize),
            ));
            cases.push(TestCase::new(
                format!("extend_client_area_window_tests::{name}::full_screen_state_fills_screen{arguments}"),
                move || full_screen_state_fills_screen(variant, screen_index, state, can_resize),
            ));
        }
    }
}

fn init_window(variant: Variant, screen_index: usize, state: WindowState, can_resize: bool) -> (Ref<Window>, WindowGuard) {
    let content = Border::new();
    content.set_background(Some(Brushes::dodger_blue()));
    content.set_border_brush(Some(Brushes::yellow()));
    content.set_border_thickness(Thickness::uniform(1.0));

    let window = Window::new();
    window.set_can_resize(can_resize);
    window.set_window_state(state);
    window.set_window_decorations(variant.decorations());
    window.set_extend_client_area_to_decorations_hint(true);
    window.set_width(f64::from(CLIENT_WIDTH));
    window.set_height(f64::from(CLIENT_HEIGHT));
    window.set_window_startup_location(WindowStartupLocation::Manual);
    window.set_content(Some(Control::boxed(content)));
    let guard = WindowGuard(window.clone());

    let screen_center = window.screens().all()[screen_index].bounds().center();
    window.set_position(PixelPoint::new(screen_center.x - CLIENT_WIDTH / 2, screen_center.y - CLIENT_HEIGHT / 2));

    window.show();

    when_loaded(&window);
    (window, guard)
}

fn normal_state_respects_client_size(variant: Variant, screen_index: usize, initial_state: WindowState, can_resize: bool) {
    let (window, _guard) = init_window(variant, screen_index, initial_state, can_resize);

    if initial_state != WindowState::Normal {
        window.set_window_state(WindowState::Normal);
    }

    // The client size should have been kept
    let expected =
        PixelSize::from_size(Size::new(f64::from(CLIENT_WIDTH), f64::from(CLIENT_HEIGHT)), window.render_scaling());
    let client_size = get_win32_client_size(&window);
    assert_eq!(expected, client_size);

    variant.verify_normal_state(&window, can_resize);
}

fn maximized_state_fills_screen_working_area(variant: Variant, screen_index: usize, initial_state: WindowState, can_resize: bool) {
    let (window, _guard) = init_window(variant, screen_index, initial_state, can_resize);

    if initial_state != WindowState::Maximized {
        window.set_window_state(WindowState::Maximized);
    }

    // The client size should match the screen working area
    let client_size = get_win32_client_size(&window);
    let screen_working_area = get_screen_at_index(&window, screen_index).working_area();
    assert_eq!(screen_working_area.size(), client_size);

    variant.verify_maximized_state(&window);
}

fn full_screen_state_fills_screen(variant: Variant, screen_index: usize, initial_state: WindowState, can_resize: bool) {
    let (window, _guard) = init_window(variant, screen_index, initial_state, can_resize);

    if initial_state != WindowState::FullScreen {
        window.set_window_state(WindowState::FullScreen);
    }

    // The client size should match the screen bounds
    let client_size = get_win32_client_size(&window);
    let screen_bounds = get_screen_at_index(&window, screen_index).bounds();
    assert_eq!(screen_bounds.width, client_size.width);
    assert_eq!(screen_bounds.height, client_size.height);

    // The window size should also match the screen bounds
    let window_bounds = get_win32_window_bounds(&window);
    assert_eq!(screen_bounds, window_bounds);

    // And no visible title bar
    assert_no_title_bar(&window);
}

fn assert_has_border(window: &Window) {
    let client_size = get_win32_client_size(window);
    let window_bounds = get_win32_window_bounds(window);
    assert_ne!(client_size.width, window_bounds.width);
    assert_ne!(client_size.height, window_bounds.height);
}

fn assert_no_border(window: &Window) {
    let client_size = get_win32_client_size(window);
    let window_bounds = get_win32_window_bounds(window);
    assert_eq!(client_size.width, window_bounds.width);
    assert_eq!(client_size.height, window_bounds.height);
}

/// The heights of the title bar and of the caption buttons of the drawn
/// decorations of a window; zero for what is not there or not visible.
fn get_title_bar_info(window: &Window) -> (f64, f64) {
    let host = window.get_visual_parent().expect("the window has a visual parent: the host of its decorations");
    host.get_layout_manager().expect("the host has a layout manager").execute_layout_pass();

    let with_automation_id = |id: &str| -> Option<Ref<Visual>> {
        host.get_visual_descendants().find(|c| AutomationProperties::get_automation_id(c).as_deref() == Some(id))
    };
    let titlebar = with_automation_id(TITLE_BAR_AUTOMATION_ID);
    let close_button = with_automation_id("Close");

    let visible_height = |visual: Option<Ref<Visual>>| match visual {
        Some(visual) if visual.is_effectively_visible() => visual.bounds().height,
        _ => 0.0,
    };

    (visible_height(titlebar), visible_height(close_button))
}

fn assert_no_title_bar(window: &Window) {
    let (title_bar_height, buttons_height) = get_title_bar_info(window);
    assert_eq!(0.0, title_bar_height);
    assert_eq!(0.0, buttons_height);
}

fn assert_large_title_bar_with_buttons(window: &Window) {
    let (title_bar_height, buttons_height) = get_title_bar_info(window);
    assert!(title_bar_height > 20.0, "the title bar is {title_bar_height} high");
    assert!(buttons_height > 20.0, "the caption buttons are {buttons_height} high");
}
