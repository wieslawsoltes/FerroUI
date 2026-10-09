//! Not from upstream: windows of one application that follow one another,
//! as the tests of the shared application (`per_assembly`) show them.
//!
//! A window that was closed before the compositor committed once (shown and
//! closed within one job of the dispatcher) used to leave the compositor
//! unable to render the windows after it: the batch of the out-of-band
//! disposal of its composition target overtook the batch that created the
//! server target, the id of the target was handed out again while the
//! server target, created after its disposal, was alive, and the batch that
//! reused the id was lost (`Compositor::oob_dispose`).

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

/// Shows a window and closes it within the same job: no frame is rendered
/// and nothing is committed in between.
fn show_and_close() {
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    let window = Window::new();
    window.set_width(100.0);
    window.set_height(100.0);
    window.set_content(Some(Control::boxed(border)));
    window.show();
    window.close();
}

fn session() -> std::sync::Arc<HeadlessUnitTestSession> {
    HeadlessUnitTestSession::start_new_with_isolation(TestApplication::build_ferro_app, FerroTestIsolationLevel::PerAssembly)
}

#[test]
fn a_window_shown_after_another_one_was_closed_is_rendered() {
    let session = session();

    let first = session.dispatch_result(frame_and_hit_test, CancellationToken::none()).wait();
    let second = session.dispatch_result(frame_and_hit_test, CancellationToken::none()).wait();
    session.dispose();

    assert_eq!(Ok((true, true)), first);
    assert_eq!(Ok((true, true)), second);
}

#[test]
fn a_window_shown_after_another_one_was_closed_before_its_first_frame_is_rendered() {
    let session = session();

    let closed = session.dispatch_result(show_and_close, CancellationToken::none()).wait();
    let second = session.dispatch_result(frame_and_hit_test, CancellationToken::none()).wait();
    let third = session.dispatch_result(frame_and_hit_test, CancellationToken::none()).wait();
    session.dispose();

    assert_eq!(Ok(()), closed);
    assert_eq!(Ok((true, true)), second);
    assert_eq!(Ok((true, true)), third);
}
