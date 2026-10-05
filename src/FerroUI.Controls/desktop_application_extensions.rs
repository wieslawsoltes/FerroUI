//! The desktop-style ways of running an [`Application`] without a lifetime.
//!
//! # Run loops
//!
//! All of these end in [`Application::run_with_token`], which runs the main
//! loop of the UI thread dispatcher only where the platform supports run
//! loops ([`Dispatcher::supports_run_loops`]). On the other platforms (the
//! event loop belongs to the host, such as a browser) they do everything
//! else - show the window, hook its closing - and return immediately; the
//! application keeps running, driven by the host, and nothing blocks.

use crate::{Application, Window};
use ferroui_base::input::ICloseable;
use ferroui_base::threading::{CancellationToken, CancellationTokenSource, Dispatcher};
use ferroui_base::{Nullable, ObjectType, Ref, Upcast};
use std::rc::Rc;

impl Application {
    /// On desktop-style platforms runs the application's main loop until
    /// `closable` is closed.
    ///
    /// Consider using a classic desktop lifetime instead. Without run loops
    /// this returns right after subscribing to the closed event.
    pub fn run(&self, closable: &dyn ICloseable) {
        let cts = CancellationTokenSource::new();
        let source = cts.clone();
        closable.closed(Rc::new(move || source.cancel()));

        self.run_with_token(&cts.token());
    }

    /// On desktop-style platforms runs the application's main loop until
    /// the main window is closed. The window is shown if it is not visible.
    ///
    /// Consider using a classic desktop lifetime instead. Without run loops
    /// this returns right after showing the window, which stays alive until
    /// it is closed.
    ///
    /// # Panics
    /// Panics when `main_window` is null.
    pub fn run_window(&self, main_window: impl Into<Nullable<Window>>) {
        let Some(main_window) = main_window.into().0 else {
            panic!("Value cannot be null. (Parameter 'main_window')");
        };
        let cts = CancellationTokenSource::new();
        let source = cts.clone();
        main_window.closed(move || source.cancel());
        if !main_window.is_visible() {
            main_window.show();
        }
        self.run_with_token(&cts.token());
    }

    /// On desktop-style platforms runs the application's main loop with a
    /// custom cancellation token, without setting a lifetime: the loop runs
    /// until `token` is canceled.
    ///
    /// Without run loops this does nothing and returns.
    pub fn run_with_token(&self, token: &CancellationToken) {
        let dispatcher = Dispatcher::ui_thread();
        if dispatcher.supports_run_loops() {
            dispatcher.main_loop(token);
        }
    }

    /// Creates a window with `new_window`, shows it and runs the
    /// application's main loop until it is closed; see
    /// [`run_window`](Self::run_window).
    pub fn run_with_main_window<TWindow: ObjectType + Upcast<Window>>(&self, new_window: impl FnOnce() -> Ref<TWindow>) {
        let window: Ref<Window> = new_window().upcast();
        window.show();
        self.run_window(window);
    }
}
