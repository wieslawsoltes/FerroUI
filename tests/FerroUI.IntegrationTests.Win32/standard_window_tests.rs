use crate::unmanaged_methods::{get_system_metrics, SM_CMONITORS};
use crate::window_extensions::{get_screen_at_index, get_win32_client_size, get_win32_window_bounds, when_loaded};
use crate::{TestCase, WindowGuard};
use ferroui_base::media::Brushes;
use ferroui_base::{PixelPoint, Ref, Thickness};
use ferroui_controls::{Border, Control, Window, WindowDecorations, WindowStartupLocation, WindowState};

const CLIENT_WIDTH: i32 = 200;
const CLIENT_HEIGHT: i32 = 200;

/// The classes that derive from the test class upstream: the decorations
/// of the window and whether a window with them has a caption.
#[derive(Debug, Clone, Copy)]
struct Variant {
    name: &'static str,
    decorations: WindowDecorations,
    has_caption: bool,
}

const VARIANTS: [Variant; 3] = [
    Variant { name: "decorations_full", decorations: WindowDecorations::Full, has_caption: true },
    Variant { name: "decorations_border_only", decorations: WindowDecorations::BorderOnly, has_caption: false },
    Variant { name: "decorations_none", decorations: WindowDecorations::None, has_caption: false },
];

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
            cases.push(TestCase::new(
                format!("standard_window_tests::{}::maximized_state_fills_screen_working_area{arguments}", variant.name),
                move || maximized_state_fills_screen_working_area(variant, screen_index, state, can_resize),
            ));
            cases.push(TestCase::new(
                format!("standard_window_tests::{}::full_screen_state_fills_screen{arguments}", variant.name),
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
    window.set_window_decorations(variant.decorations);
    window.set_extend_client_area_to_decorations_hint(false);
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

fn maximized_state_fills_screen_working_area(variant: Variant, screen_index: usize, initial_state: WindowState, can_resize: bool) {
    let (window, _guard) = init_window(variant, screen_index, initial_state, can_resize);

    if initial_state != WindowState::Maximized {
        window.set_window_state(WindowState::Maximized);
    }

    // The client size should match the screen working area
    let client_size = get_win32_client_size(&window);
    let screen_working_area = get_screen_at_index(&window, screen_index).working_area();

    if variant.has_caption {
        assert_eq!(screen_working_area.size().width, client_size.width);
        assert!(client_size.height < screen_working_area.size().height);
    } else {
        assert_eq!(screen_working_area.size(), client_size);
    }
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
}
