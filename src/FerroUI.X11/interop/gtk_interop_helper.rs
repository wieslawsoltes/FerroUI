//! Runs code of an application on the thread of GTK (the port of
//! `Interop/GtkInteropHelper.cs`).

use super::glib::Glib;
use crate::native_dialogs::gtk::start_gtk;

/// The public entry to the thread of GTK.
pub struct GtkInteropHelper;

impl GtkInteropHelper {
    /// Starts GTK when it is not started, runs `cb` on its thread and
    /// resolves to what it returned. Inside `cb`,
    /// [`Gtk::current`](crate::native_dialogs::gtk::Gtk::current) gives the
    /// functions of GTK.
    ///
    /// Fails with a message when GTK cannot be initialized (the reference
    /// throws), or when `cb` panics.
    pub async fn run_on_glib_thread<T: Send + 'static>(cb: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
        if !start_gtk().await {
            return Err("Unable to initialize GTK".to_string());
        }
        let glib = Glib::try_get().map_err(|error| error.to_string())?;
        glib.run_on_glib_thread(cb).await
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    /// Without the libraries of GTK (the development machine, a minimal
    /// image) the start fails at once and the call says so. Where they
    /// exist the test does not start GTK, which needs a display.
    #[test]
    fn without_gtk_the_call_fails_with_the_message_of_the_reference() {
        if crate::interop::native_library::NativeLibrary::open(c"libgtk-3.so.0").is_ok() {
            return;
        }
        let mut call = std::pin::pin!(GtkInteropHelper::run_on_glib_thread(|| 1));
        let mut context = Context::from_waker(Waker::noop());
        // The start runs on a thread of its own, which fails as soon as it looks for the
        // library.
        for _ in 0..500 {
            if let Poll::Ready(result) = call.as_mut().poll(&mut context) {
                assert_eq!(result, Err("Unable to initialize GTK".to_string()));
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the start of GTK did not answer");
    }
}
