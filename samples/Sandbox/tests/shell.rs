//! The real application of the sample, headless (`sample_testing::Shell`): the application
//! populates itself from `App.xaml`, the main window is created and shown as the desktop
//! lifetime shows it, and its frames are rendered through the compositor with Skia.

use crate::{App, MainWindow, SAMPLE};
use ferroui_base::Ref;
use ferroui_controls::Application;
use sample_testing::{assert_accepted, Shell};

/// The size of the window of the test.
const SIZE: (f64, f64) = (640.0, 480.0);

/// The binding errors the sample is accepted to report: none. The documents of the sample
/// have no binding.
const ACCEPTED: &[(&str, &str, usize)] = &[];

fn start() -> (Shell, Ref<MainWindow>) {
    let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || App::new().upcast());
    let window = MainWindow::new();
    window.show();
    shell.settle();
    (shell, window)
}

fn reports(shell: &Shell, place: &str) -> Vec<(String, String, usize)> {
    shell.take_binding_reports().into_iter().map(|(report, count)| (place.to_string(), report.line(), count)).collect()
}

#[test]
fn the_application_has_the_theme_its_document_declares() {
    let (shell, window) = start();

    let application = Application::current().expect("the application of the test");
    assert!(application.cast::<App>().is_some(), "the application is the application of the sample");
    assert_eq!(application.styles().count(), 1, "App.xaml declares one style: the Fluent theme");

    window.close();
    drop(shell);
}

#[test]
fn the_main_window_shows_what_its_document_declares() {
    let (shell, window) = start();

    assert_eq!(shell.window_count(), 1, "the application created one window");
    assert!(window.is_visible(), "the main window is shown");
    // `MainWindow.xaml` declares a window without content.
    assert!(window.content().is_none(), "the window of the sandbox has no content");
    assert!(window.visual_children_count() > 0, "the window has the template of the theme");

    assert!(shell.frames(0) > 0, "the compositor drew the window");
    let frame = shell.last_frame(0);
    assert_eq!((frame.width, frame.height), (SIZE.0 as usize, SIZE.1 as usize), "the frame has the size of the window");
    // An empty window is its background.
    assert_eq!(frame.colors((0.0, 0.0, SIZE.0, SIZE.1)), 1, "an empty window draws its background only");
    assert_eq!(frame.pixel(0, 0)[3], 255, "the background of the window is opaque");

    assert_accepted(&reports(&shell, "MainWindow"), ACCEPTED);

    window.close();
    drop(shell);
}
