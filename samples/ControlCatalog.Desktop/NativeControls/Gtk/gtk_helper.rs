//! Port of `GtkHelper.cs`: a file chooser of GTK as a native control, made
//! on the thread of GTK and embedded by its window on the X server.

use ferroui_controls::platform::{INativeControlHostDestroyableControlHandle, IPlatformHandle};
use ferroui_x11::interop::GtkInteropHelper;
use ferroui_x11::native_dialogs::gtk::{Gtk, GtkFileChooserAction, GtkWidget};
use std::any::Any;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;

/// Wakes the thread that waits for a result of the thread of GTK.
struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Waits for a result of the thread of GTK on the calling thread (the
/// `.Result` and `.Wait()` of the reference).
fn wait<T>(future: impl Future<Output = T>) -> T {
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

struct FileChooser {
    widget: GtkWidget,
    handle: isize,
}

impl IPlatformHandle for FileChooser {
    fn handle(&self) -> isize {
        self.handle
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some("XID")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_native_control_host_destroyable_control_handle(&self) -> Option<&dyn INativeControlHostDestroyableControlHandle> {
        Some(self)
    }
}

impl INativeControlHostDestroyableControlHandle for FileChooser {
    fn destroy(&self) {
        let widget = self.widget;
        let _ = wait(GtkInteropHelper::run_on_glib_thread(move || {
            if let Some(gtk) = Gtk::current() {
                // SAFETY: the widget is the dialog this handle was made for, destroyed once: the
                // host destroys a control once, and no other copy of the value is used.
                unsafe { gtk.gtk_widget_destroy(widget) };
            }
            0
        }));
    }
}

/// A file chooser of GTK for the native control host; `None` when GTK
/// cannot be started (the reference fails).
pub fn create_gtk_file_chooser(_parent_xid: isize) -> Option<Rc<dyn IPlatformHandle>> {
    let created = wait(GtkInteropHelper::run_on_glib_thread(|| {
        let gtk = Gtk::current()?;
        let widget = gtk.gtk_file_chooser_dialog_new(Some("Embedded"), GtkFileChooserAction::SelectFolder);
        gtk.gtk_widget_realize(widget);
        let xid = gtk.gdk_x11_window_get_xid(gtk.gtk_widget_get_window(widget)?);
        gtk.gtk_window_present(widget);
        Some((widget, xid))
    }));
    let (widget, xid) = created.ok()??;
    Some(Rc::new(FileChooser { widget, handle: xid as isize }))
}
