//! Port of the data context tests of the reference leak tests: the data
//! context of a window is freed after the window closed.

use crate::leak::Tracked;
use crate::services::{collect_garbage_loaded, start_styled_window};
use ferroui_base::data::model::Model;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_model, BoxedValue};
use ferroui_controls::application_lifetimes::ClassicDesktopStyleApplicationLifetime;
use ferroui_controls::{ShutdownMode, Window};

/// The view model of the tests. (The one of the reference has a finalizer,
/// which keeps an object in the queue of the collector for one more
/// collection; nothing stands for it here.)
struct ViewModelForDisposingTest;

ferro_model!(ViewModelForDisposingTest, |b| b);

#[test]
fn window_data_context_disposed_after_window_close_with_lifetime() {
    // The reference never ends the application of this test, and shuts the
    // lifetime down when the test ends; here the application ends with the
    // test, after the lifetime.
    let _app = start_styled_window();
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();

    let view_model = {
        lifetime.set_shutdown_mode(ShutdownMode::OnExplicitShutdown);
        let view_model = Model::new_model(ViewModelForDisposingTest);
        let window = Window::new();
        window.set_data_context(Some(view_model.clone() as BoxedValue));
        window.show();
        window.close();

        let tracked = Tracked::shared("the data context of the window", &view_model);
        tracked.assert_alive();
        tracked
    };

    collect_garbage_loaded();

    view_model.assert_freed();

    lifetime.shutdown(0);
    lifetime.dispose();
}

#[test]
fn window_data_context_disposed_after_window_close_without_lifetime() {
    // The jobs of the dispatcher run after the application ended, as in the
    // reference: the dispatcher of the test outlives the application.
    let _scope = Dispatcher::unit_test_scope();

    let view_model = {
        let _app = start_styled_window();
        let view_model = Model::new_model(ViewModelForDisposingTest);
        let window = Window::new();
        window.set_data_context(Some(view_model.clone() as BoxedValue));
        window.show();
        window.close();

        let tracked = Tracked::shared("the data context of the window", &view_model);
        tracked.assert_alive();
        tracked
    };

    collect_garbage_loaded();

    view_model.assert_freed();
}
