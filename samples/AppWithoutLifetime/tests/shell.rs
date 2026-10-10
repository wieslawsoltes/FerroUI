//! The real application of the sample, headless (`sample_testing::Shell`): the application
//! populates itself from `App.xaml` and has no lifetime; the test does what the entry point
//! does before it runs the main loop (it creates the main window and shows it), presses the
//! button that opens the second window, and renders the frames of both windows through the
//! compositor with Skia.

use crate::{App, MainWindow, Sub, SAMPLE};
use ferroui_base::{AnyValue, ObjectType, Point, Ref, Visual};
use ferroui_controls::{Application, Button, CheckBox, TextBlock, Window};
use sample_testing::{assert_accepted, Shell};

/// The size of the windows of the test.
const SIZE: (f64, f64) = (640.0, 480.0);

/// The binding errors the sample is accepted to report: none. The documents of the sample
/// have no binding.
const ACCEPTED: &[(&str, &str, usize)] = &[];

fn start() -> (Shell, Ref<MainWindow>) {
    let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || App::new().upcast());
    // `AppMain` of the upstream sample: `app.Run(new MainWindow())` shows the window and runs
    // the main loop, which is the frames of the test here.
    let window = MainWindow::new();
    window.show();
    shell.settle();
    (shell, window)
}

fn descendants<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter_map(|visual| visual.cast::<T>()).collect()
}

/// The texts of the text blocks below `root` that are shown.
fn texts(root: &Visual) -> Vec<String> {
    descendants::<TextBlock>(root)
        .into_iter()
        .filter(|text_block| text_block.is_effectively_visible())
        .filter_map(|text_block| text_block.text())
        .map(|text| text.trim().to_string())
        .collect()
}

/// The button of the main window (`Content="Open"`); the template of a window has buttons
/// of its own.
fn open_button(window: &MainWindow) -> Ref<Button> {
    descendants::<Button>(window)
        .into_iter()
        .find(|button| {
            button.content().is_some_and(|content| {
                let content: &dyn AnyValue = &*content;
                content.downcast_ref::<String>().is_some_and(|text| text == "Open")
            })
        })
        .expect("the button of the window")
}

fn reports(shell: &Shell, place: &str) -> Vec<(String, String, usize)> {
    shell.take_binding_reports().into_iter().map(|(report, count)| (place.to_string(), report.line(), count)).collect()
}

/// Whether something is drawn inside the bounds of `visual` in the last frame of the window
/// `window`: more than one colour.
fn draws(shell: &Shell, window: usize, top_level: &Window, visual: &Visual) -> bool {
    shell.last_frame(window).colors(Shell::frame_rect_of(top_level, visual)) > 1
}

#[test]
fn the_application_has_no_lifetime() {
    let (shell, window) = start();

    let application = Application::current().expect("the application of the test");
    assert!(application.cast::<App>().is_some(), "the application is the application of the sample");
    assert!(application.application_lifetime().is_none(), "the application is run without a lifetime");
    assert_eq!(application.styles().count(), 1, "App.xaml declares one style: the Fluent theme");

    window.close();
    drop(shell);
}

#[test]
fn the_main_window_shows_what_its_document_declares() {
    let (shell, window) = start();

    assert_eq!(shell.window_count(), 1, "the entry point created one window");
    assert!(window.is_visible(), "the main window is shown");
    assert_eq!(window.title().as_deref(), Some("AppWithoutLifetime"));

    let shown = texts(&window);
    assert_eq!(
        shown.iter().filter(|text| *text == "Welcome to FerroUI!").count(),
        2,
        "the text block and the check box show the welcome text: {shown:?}"
    );
    assert!(shown.iter().any(|text| text == "Open"), "the button shows its content: {shown:?}");

    assert!(shell.frames(0) > 0, "the compositor drew the main window");
    let text_block = descendants::<TextBlock>(&window).into_iter().next().expect("the text block of the window");
    let check_box = descendants::<CheckBox>(&window).into_iter().next().expect("the check box of the window");
    let button = open_button(&window);
    assert!(draws(&shell, 0, &window, &text_block), "the text block is drawn");
    assert!(draws(&shell, 0, &window, &check_box), "the check box is drawn");
    assert!(draws(&shell, 0, &window, &button), "the button is drawn");

    assert_accepted(&reports(&shell, "MainWindow"), ACCEPTED);

    window.close();
    drop(shell);
}

#[test]
fn the_button_opens_the_second_window_and_both_windows_render() {
    let (shell, window) = start();
    assert!(window.owned_windows().is_empty(), "no window is open before the button is pressed");

    let button = open_button(&window);
    let bounds = Shell::bounds_of(&window, &button);
    shell.click(0, Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0));

    assert_eq!(shell.window_count(), 2, "the click handler created the second window");
    let owned = window.owned_windows();
    assert_eq!(owned.len(), 1, "the second window is owned by the main window");
    let sub = owned[0].cast::<Sub>().expect("the owned window is the second window of the sample");
    assert!(sub.is_visible(), "the second window is shown");
    assert_eq!(sub.title().as_deref(), Some("Window1"));

    let shown = texts(&sub);
    assert!(shown.iter().any(|text| text == "Welcome to FerroUI Sub!"), "the second window shows its text: {shown:?}");

    assert!(shell.frames(0) > 0, "the compositor drew the main window");
    assert!(shell.frames(1) > 0, "the compositor drew the second window");
    assert!(shell.last_frame(0).colors((0.0, 0.0, SIZE.0, SIZE.1)) > 1, "the main window draws its controls");
    assert!(shell.last_frame(1).colors((0.0, 0.0, SIZE.0, SIZE.1)) > 1, "the second window draws its text");

    assert_accepted(&reports(&shell, "MainWindow and Sub"), ACCEPTED);

    // Closing the main window is what ends the main loop of the entry point; an owned window
    // is closed with its owner.
    window.close();
    shell.settle();
    assert!(!sub.is_visible(), "the second window is closed with its owner");
    drop(shell);
}
