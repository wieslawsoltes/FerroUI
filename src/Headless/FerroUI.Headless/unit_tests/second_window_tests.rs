//! Not from upstream: the difference of the port that the tests of the
//! shared application (`per_assembly`) are ignored for, on its own.
//!
//! An application that closes a window and shows another one renders the
//! first and not the second. When the batch after the close of the first
//! window is applied, the server compositor inserts a server object under
//! an id whose object is still alive (the debug assertion `a server object
//! id was reused while still alive` of `server_compositor.rs`); the panic
//! ends the tick of the render loop, which logs it, and no batch of the
//! compositor is processed from then on: the second window has no frame
//! and no hit test, so no input reaches its controls.

use super::test_application::TestApplication;
use crate::{FerroTestIsolationLevel, HeadlessUnitTestSession, HeadlessWindowExtensions};
use ferroui_base::media::Brushes;
use ferroui_base::threading::CancellationToken;
use ferroui_base::Point;
use ferroui_controls::{Border, Control, Window};

/// Shows a window, captures its frame, hit-tests its centre and closes it.
fn frame_and_hit_test() -> (bool, bool) {
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    let window = Window::new();
    window.set_width(100.0);
    window.set_height(100.0);
    window.set_content(Some(Control::boxed(border)));
    window.show();

    let frame = window.capture_rendered_frame();
    let hit = window.input_hit_test(Point::new(50.0, 50.0));
    window.close();

    (frame.is_some(), hit.is_some())
}

#[test]
#[ignore = "the second top-level of a compositor is not rendered: the render loop panics with `a server object id was reused while still alive` (server_compositor.rs) once a top-level was closed"]
fn a_window_shown_after_another_one_was_closed_is_rendered() {
    let session = HeadlessUnitTestSession::start_new_with_isolation(
        TestApplication::build_ferro_app,
        FerroTestIsolationLevel::PerAssembly,
    );

    let first = session.dispatch_result(frame_and_hit_test, CancellationToken::none()).wait();
    let second = session.dispatch_result(frame_and_hit_test, CancellationToken::none()).wait();
    session.dispose();

    assert_eq!(Ok((true, true)), first);
    assert_eq!(Ok((true, true)), second);
}
