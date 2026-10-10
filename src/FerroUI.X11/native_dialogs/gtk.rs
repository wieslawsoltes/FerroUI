//! The functions of GTK 3 and GDK 3 the platform calls, and how GTK is
//! started (the port of `NativeDialogs/Gtk.cs`).
//!
//! `libgtk-3.so.0` and `libgdk-3.so.0` are opened when GTK is first asked
//! for; without them, or without GLib, [`start_gtk`] answers `false` and a
//! window goes on to its next storage provider.
//!
//! GTK belongs to one thread: the one that initialized it. The functions
//! are methods of [`Gtk`], a value that exists only on that thread
//! ([`Gtk::current`]), and they take widgets as [`GtkWidget`] values, which
//! only these methods make. That is what lets them be safe functions.

use crate::interop::glib::{GSList, Glib};
use crate::interop::native_library::{native_functions, NativeLibrary, NativeLibraryError};
use crate::x11_platform::X11PlatformOptions;
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::ffi::{c_char, c_int, c_ulong, c_void, CStr, CString};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::task::{Context, Poll, Waker};
use std::thread::{self, ThreadId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum GtkFileChooserAction {
    Open,
    Save,
    SelectFolder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum GtkResponseType {
    Help = -11,
    Apply = -10,
    No = -9,
    Yes = -8,
    Close = -7,
    Cancel = -6,
    Ok = -5,
    DeleteEvent = -4,
    Accept = -3,
    Reject = -2,
    None = -1,
}

const GDK_NAME: &CStr = c"libgdk-3.so.0";
const GTK_NAME: &CStr = c"libgtk-3.so.0";

native_functions! {
    /// The functions, as C declares them (a `gboolean` is an `int`).
    pub(crate) struct GtkApi(gtk, gdk) {
        gtk::gtk_main_iteration: unsafe extern "C" fn() -> c_int;
        gtk::gtk_window_set_modal: unsafe extern "C" fn(*mut c_void, c_int);
        gtk::gtk_window_present: unsafe extern "C" fn(*mut c_void);
        // A function with a variable argument list: the first button text, then pairs
        // until a null. The reference passes the one null.
        gtk::gtk_file_chooser_dialog_new:
            unsafe extern "C" fn(*const c_char, *mut c_void, c_int, *const c_char, ...) -> *mut c_void;
        gtk::gtk_file_chooser_set_select_multiple: unsafe extern "C" fn(*mut c_void, c_int);
        gtk::gtk_file_chooser_set_local_only: unsafe extern "C" fn(*mut c_void, c_int);
        gtk::gtk_file_chooser_set_do_overwrite_confirmation: unsafe extern "C" fn(*mut c_void, c_int);
        gtk::gtk_dialog_add_button: unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> *mut c_void;
        gtk::gtk_file_chooser_get_filenames: unsafe extern "C" fn(*mut c_void) -> *mut GSList;
        gtk::gtk_file_chooser_set_filename: unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;
        gtk::gtk_file_chooser_set_current_name: unsafe extern "C" fn(*mut c_void, *const c_char);
        gtk::gtk_file_chooser_set_current_folder: unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;
        gtk::gtk_file_filter_new: unsafe extern "C" fn() -> *mut c_void;
        gtk::gtk_file_filter_set_name: unsafe extern "C" fn(*mut c_void, *const c_char);
        gtk::gtk_file_filter_add_pattern: unsafe extern "C" fn(*mut c_void, *const c_char);
        gtk::gtk_file_filter_add_mime_type: unsafe extern "C" fn(*mut c_void, *const c_char);
        gtk::gtk_file_chooser_add_filter: unsafe extern "C" fn(*mut c_void, *mut c_void);
        gtk::gtk_file_chooser_get_filter: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        gtk::gtk_file_chooser_set_filter: unsafe extern "C" fn(*mut c_void, *mut c_void);
        gtk::gtk_widget_realize: unsafe extern "C" fn(*mut c_void);
        gtk::gtk_widget_destroy: unsafe extern "C" fn(*mut c_void);
        gtk::gtk_widget_get_window: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        gtk::gtk_widget_hide: unsafe extern "C" fn(*mut c_void);
        gtk::gtk_init_check: unsafe extern "C" fn(*mut c_int, *mut c_void) -> c_int;
        gdk::gdk_x11_window_foreign_new_for_display: unsafe extern "C" fn(*mut c_void, c_ulong) -> *mut c_void;
        gdk::gdk_x11_window_get_xid: unsafe extern "C" fn(*mut c_void) -> c_ulong;
        gtk::gtk_container_add: unsafe extern "C" fn(*mut c_void, *mut c_void);
        gdk::gdk_set_allowed_backends: unsafe extern "C" fn(*const c_char);
        gdk::gdk_display_get_default: unsafe extern "C" fn() -> *mut c_void;
        gdk::gdk_x11_display_get_xdisplay: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        gtk::gtk_application_new: unsafe extern "C" fn(*const c_char, c_int) -> *mut c_void;
        gdk::gdk_window_set_transient_for: unsafe extern "C" fn(*mut c_void, *mut c_void);
    }
}

/// The libraries, opened at the first call.
fn api() -> Result<&'static GtkApi, NativeLibraryError> {
    static API: OnceLock<Result<GtkApi, NativeLibraryError>> = OnceLock::new();
    API.get_or_init(|| {
        let gtk = NativeLibrary::open(GTK_NAME)?;
        let gdk = NativeLibrary::open(GDK_NAME)?;
        GtkApi::load(&gtk, &gdk)
    })
    .as_ref()
    .map_err(Clone::clone)
}

/// The display of GDK (`s_display`), once GTK is initialized.
static DISPLAY: AtomicUsize = AtomicUsize::new(0);
/// The thread that initialized GTK.
static GTK_THREAD: OnceLock<ThreadId> = OnceLock::new();

/// A widget (or another GObject of GTK: a file filter) that one of the
/// functions of [`Gtk`] made. The dialogs of the platform are hidden when
/// they are done and never destroyed, as in the reference, so such a value
/// stays valid unless it is given to [`Gtk::gtk_widget_destroy`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GtkWidget(usize);

impl GtkWidget {
    /// The address of the object.
    pub fn as_ptr(self) -> *mut c_void {
        self.0 as *mut c_void
    }

    /// A widget from its address.
    ///
    /// # Safety
    /// `ptr` is a live object of GTK of the kind the functions it is given
    /// to expect, and stays alive while the value is used.
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        Self(ptr as usize)
    }
}

/// A window of GDK.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GdkWindow(usize);

fn c_string(text: &str) -> CString {
    // A text with a zero in it ends there, as it would for the C library.
    let end = text.find('\0').unwrap_or(text.len());
    CString::new(&text[..end]).unwrap_or_default()
}

/// GTK on its thread.
#[derive(Clone, Copy)]
pub struct Gtk {
    api: &'static GtkApi,
    glib: Glib,
    /// Not `Send`, not `Sync`: the value stays on the thread of GTK.
    thread: PhantomData<*const ()>,
}

impl Gtk {
    /// GTK, when the calling thread is the one that initialized it.
    pub fn current() -> Option<Gtk> {
        if GTK_THREAD.get() != Some(&thread::current().id()) {
            return None;
        }
        Some(Gtk { api: api().ok()?, glib: Glib::try_get().ok()?, thread: PhantomData })
    }

    /// GLib.
    pub fn glib(&self) -> Glib {
        self.glib
    }

    // SAFETY of the methods below: the value exists only on the thread that initialized GTK
    // (`current`), so GTK is initialized and called on its thread; every widget is one of
    // these methods' own results and alive (`GtkWidget`); texts are terminated strings that
    // live for the call, which copies them.

    pub fn gtk_window_set_modal(&self, window: GtkWidget, modal: bool) {
        unsafe { (self.api.gtk_window_set_modal)(window.as_ptr(), modal as c_int) }
    }

    pub fn gtk_window_present(&self, gtk_window: GtkWidget) {
        unsafe { (self.api.gtk_window_present)(gtk_window.as_ptr()) }
    }

    /// A file chooser dialog without a parent and without buttons.
    pub fn gtk_file_chooser_dialog_new(&self, title: Option<&str>, action: GtkFileChooserAction) -> GtkWidget {
        let title = title.map(c_string);
        let title_ptr = title.as_ref().map_or(std::ptr::null(), |title| title.as_ptr());
        // SAFETY: as above; the argument list ends with the null first button text.
        let dialog = unsafe {
            (self.api.gtk_file_chooser_dialog_new)(
                title_ptr,
                std::ptr::null_mut(),
                action as c_int,
                std::ptr::null::<c_char>(),
            )
        };
        GtkWidget(dialog as usize)
    }

    pub fn gtk_file_chooser_set_select_multiple(&self, chooser: GtkWidget, allow: bool) {
        unsafe { (self.api.gtk_file_chooser_set_select_multiple)(chooser.as_ptr(), allow as c_int) }
    }

    pub fn gtk_file_chooser_set_local_only(&self, chooser: GtkWidget, local_only: bool) {
        unsafe { (self.api.gtk_file_chooser_set_local_only)(chooser.as_ptr(), local_only as c_int) }
    }

    pub fn gtk_file_chooser_set_do_overwrite_confirmation(&self, chooser: GtkWidget, do_overwrite_confirmation: bool) {
        unsafe {
            (self.api.gtk_file_chooser_set_do_overwrite_confirmation)(chooser.as_ptr(), do_overwrite_confirmation as c_int)
        }
    }

    pub fn gtk_dialog_add_button(&self, raw: GtkWidget, button_text: &str, response_id: GtkResponseType) {
        let button_text = c_string(button_text);
        unsafe { (self.api.gtk_dialog_add_button)(raw.as_ptr(), button_text.as_ptr(), response_id as c_int) };
    }

    /// The names of the chosen files, copied out of the list GTK returns,
    /// which is freed here as the reference frees it.
    pub fn gtk_file_chooser_get_filenames(&self, chooser: GtkWidget) -> Vec<String> {
        let mut result_list = Vec::new();
        // SAFETY: as above. The list and its nodes are GTK's until the list is freed below;
        // each node's data is null or a terminated string.
        unsafe {
            let gs = (self.api.gtk_file_chooser_get_filenames)(chooser.as_ptr());
            let mut cgs = gs;
            while !cgs.is_null() {
                let data = (*cgs).data;
                if !data.is_null() {
                    result_list.push(CStr::from_ptr(data.cast::<c_char>()).to_string_lossy().into_owned());
                }
                cgs = (*cgs).next;
            }
            self.glib.g_slist_free(gs);
        }
        result_list
    }

    pub fn gtk_file_chooser_set_filename(&self, chooser: GtkWidget, file: &str) {
        let file = c_string(file);
        unsafe { (self.api.gtk_file_chooser_set_filename)(chooser.as_ptr(), file.as_ptr()) };
    }

    pub fn gtk_file_chooser_set_current_name(&self, chooser: GtkWidget, file: &str) {
        let file = c_string(file);
        unsafe { (self.api.gtk_file_chooser_set_current_name)(chooser.as_ptr(), file.as_ptr()) }
    }

    pub fn gtk_file_chooser_set_current_folder(&self, chooser: GtkWidget, file: &str) {
        let file = c_string(file);
        unsafe { (self.api.gtk_file_chooser_set_current_folder)(chooser.as_ptr(), file.as_ptr()) };
    }

    pub fn gtk_file_filter_new(&self) -> GtkWidget {
        GtkWidget(unsafe { (self.api.gtk_file_filter_new)() } as usize)
    }

    pub fn gtk_file_filter_set_name(&self, filter: GtkWidget, name: &str) {
        let name = c_string(name);
        unsafe { (self.api.gtk_file_filter_set_name)(filter.as_ptr(), name.as_ptr()) }
    }

    pub fn gtk_file_filter_add_pattern(&self, filter: GtkWidget, pattern: &str) {
        let pattern = c_string(pattern);
        unsafe { (self.api.gtk_file_filter_add_pattern)(filter.as_ptr(), pattern.as_ptr()) }
    }

    pub fn gtk_file_filter_add_mime_type(&self, filter: GtkWidget, mime_type: &str) {
        let mime_type = c_string(mime_type);
        unsafe { (self.api.gtk_file_filter_add_mime_type)(filter.as_ptr(), mime_type.as_ptr()) }
    }

    /// Adds a filter to a chooser, which keeps it alive from then on.
    pub fn gtk_file_chooser_add_filter(&self, chooser: GtkWidget, filter: GtkWidget) {
        unsafe { (self.api.gtk_file_chooser_add_filter)(chooser.as_ptr(), filter.as_ptr()) }
    }

    /// The current filter of a chooser, for comparing with the filters
    /// that were added; `None` without one.
    pub fn gtk_file_chooser_get_filter(&self, chooser: GtkWidget) -> Option<GtkWidget> {
        let filter = unsafe { (self.api.gtk_file_chooser_get_filter)(chooser.as_ptr()) };
        (!filter.is_null()).then_some(GtkWidget(filter as usize))
    }

    pub fn gtk_file_chooser_set_filter(&self, chooser: GtkWidget, filter: GtkWidget) {
        unsafe { (self.api.gtk_file_chooser_set_filter)(chooser.as_ptr(), filter.as_ptr()) }
    }

    pub fn gtk_widget_realize(&self, gtk_widget: GtkWidget) {
        unsafe { (self.api.gtk_widget_realize)(gtk_widget.as_ptr()) }
    }

    /// Destroys a widget.
    ///
    /// # Safety
    /// No copy of `gtk_widget`, nor of a widget it contains, is used
    /// afterwards.
    pub unsafe fn gtk_widget_destroy(&self, gtk_widget: GtkWidget) {
        // SAFETY: as above, and the contract of this function.
        unsafe { (self.api.gtk_widget_destroy)(gtk_widget.as_ptr()) }
    }

    /// The window of a realized widget.
    pub fn gtk_widget_get_window(&self, gtk_widget: GtkWidget) -> Option<GdkWindow> {
        let window = unsafe { (self.api.gtk_widget_get_window)(gtk_widget.as_ptr()) };
        (!window.is_null()).then_some(GdkWindow(window as usize))
    }

    pub fn gtk_widget_hide(&self, gtk_widget: GtkWidget) {
        unsafe { (self.api.gtk_widget_hide)(gtk_widget.as_ptr()) }
    }

    /// The identifier of a window of GDK on the X server.
    pub fn gdk_x11_window_get_xid(&self, window: GdkWindow) -> usize {
        // SAFETY: as above; the window is that of a widget of this thread, which owns it.
        unsafe { (self.api.gdk_x11_window_get_xid)(window.0 as *mut c_void) as usize }
    }

    pub fn gtk_container_add(&self, container: GtkWidget, widget: GtkWidget) {
        unsafe { (self.api.gtk_container_add)(container.as_ptr(), widget.as_ptr()) }
    }

    pub fn gdk_window_set_transient_for(&self, window: GdkWindow, parent: GdkWindow) {
        // SAFETY: as above; both are windows of GDK this thread got from it.
        unsafe { (self.api.gdk_window_set_transient_for)(window.0 as *mut c_void, parent.0 as *mut c_void) }
    }

    /// A window of GDK for a window of another client on the display of
    /// GTK (`GetForeignWindow`); `None` when the server has no such
    /// window.
    pub fn get_foreign_window(&self, xid: usize) -> Option<GdkWindow> {
        let display = DISPLAY.load(Ordering::SeqCst) as *mut c_void;
        // SAFETY: as above; the display is the default display of GDK, which is never
        // closed. An identifier that names no window gives null.
        let window = unsafe { (self.api.gdk_x11_window_foreign_new_for_display)(display, xid as c_ulong) };
        (!window.is_null()).then_some(GdkWindow(window as usize))
    }
}

/// Whether GTK could be started: a result several tasks, on any thread,
/// wait for (the `Task<bool>` of `StartGtk`).
#[derive(Clone)]
pub struct StartGtkTask(Arc<Mutex<StartState>>);

#[derive(Default)]
struct StartState {
    result: Option<bool>,
    wakers: Vec<Waker>,
}

impl StartGtkTask {
    fn completed(result: bool) -> Self {
        Self(Arc::new(Mutex::new(StartState { result: Some(result), wakers: Vec::new() })))
    }

    fn pending() -> Self {
        Self(Arc::new(Mutex::new(StartState::default())))
    }

    fn try_set_result(&self, result: bool) {
        let wakers = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if state.result.is_some() {
                return;
            }
            state.result = Some(result);
            std::mem::take(&mut state.wakers)
        };
        for waker in wakers {
            waker.wake();
        }
    }

    /// The result, when it is there.
    pub fn result(&self) -> Option<bool> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).result
    }
}

impl Future for StartGtkTask {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<bool> {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match state.result {
            Some(result) => Poll::Ready(result),
            None => {
                state.wakers.push(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

static START_GTK_TASK: Mutex<Option<StartGtkTask>> = Mutex::new(None);

/// Starts GTK once for the process (`StartGtk`).
pub fn start_gtk() -> StartGtkTask {
    let mut task = START_GTK_TASK.lock().unwrap_or_else(PoisonError::into_inner);
    task.get_or_insert_with(start_gtk_core).clone()
}

/// The identifier of the application object of GTK: a prefix and 32
/// hexadecimal digits that differ between processes and calls. (The
/// reference has the name of its project as the prefix and a random
/// identifier; DEVIATIONS.md.)
pub(crate) fn application_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or_default();
    let unique = ((std::process::id() as u128) << 96)
        | ((COUNTER.fetch_add(1, Ordering::Relaxed) as u128 & 0xffff_ffff) << 64)
        | nanos as u128;
    format!("ferroui.app.a{unique:032x}")
}

/// `InitializeGtk`, on the thread that is to be the thread of GTK.
fn initialize_gtk() -> bool {
    let (Ok(api), Ok(_glib)) = (api(), Glib::try_get()) else {
        return false;
    };
    // SAFETY: the calls below are the initialization sequence of GTK, made once (the start
    // task is made once) on the thread that calls GTK from then on. Each takes terminated
    // strings or nothing, and reports failure by its result.
    unsafe {
        // Check if GTK was already initialized
        let existing_display = (api.gdk_display_get_default)();
        if !existing_display.is_null() {
            if (api.gdk_x11_display_get_xdisplay)(existing_display).is_null() {
                return false;
            }
            DISPLAY.store(existing_display as usize, Ordering::SeqCst);
            let _ = GTK_THREAD.set(thread::current().id());
            return true;
        }

        (api.gdk_set_allowed_backends)(c"x11".as_ptr());

        // The reference also sets `WAYLAND_DISPLAY` to a path that cannot be opened, through
        // its runtime, whose environment is a copy the C library does not see: GTK never
        // reads that value, and what keeps it from Wayland is the call above. Nothing is set
        // here (DEVIATIONS.md).

        if (api.gtk_init_check)(std::ptr::null_mut(), std::ptr::null_mut()) == 0 {
            return false;
        }

        let app_id = c_string(&application_id());
        let app = (api.gtk_application_new)(app_id.as_ptr(), 0);
        if app.is_null() {
            return false;
        }

        DISPLAY.store((api.gdk_display_get_default)() as usize, Ordering::SeqCst);
    }
    let _ = GTK_THREAD.set(thread::current().id());
    true
}

/// `GtkThread`: initializes GTK and then runs its main loop for ever.
fn gtk_thread(tcs: StartGtkTask) {
    if !initialize_gtk() {
        tcs.try_set_result(false);
        return;
    }
    let Ok(api) = api() else {
        tcs.try_set_result(false);
        return;
    };

    tcs.try_set_result(true);
    loop {
        // SAFETY: GTK was initialized on this thread, which is its thread from now on.
        unsafe { (api.gtk_main_iteration)() };
    }
}

fn start_gtk_core() -> StartGtkTask {
    let use_g_lib_main_loop = FerroLocator::current()
        .get_service::<X11PlatformOptions>()
        .is_some_and(|options| options.use_g_lib_main_loop);
    if use_g_lib_main_loop {
        StartGtkTask::completed(initialize_gtk())
    } else {
        let tcs = StartGtkTask::pending();
        let thread_tcs = tcs.clone();
        // A thread that cannot be started is GTK that cannot be started.
        if thread::Builder::new().name("GTK3THREAD".to_string()).spawn(move || gtk_thread(thread_tcs)).is_err() {
            tcs.try_set_result(false);
        }
        tcs
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;

    #[test]
    fn the_application_identifier_is_a_valid_one_and_differs_per_call() {
        let a = application_id();
        let b = application_id();
        assert_ne!(a, b);
        assert!(a.starts_with("ferroui.app.a"));
        assert_eq!(a.len(), "ferroui.app.a".len() + 32);
        // The rules of an application identifier: elements of letters, digits, '_' and '-'
        // separated by dots, none starting with a digit.
        for element in a.split('.') {
            assert!(!element.is_empty() && !element.starts_with(|c: char| c.is_ascii_digit()));
            assert!(element.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        }
    }

    #[test]
    fn gtk_is_not_reachable_from_a_thread_that_did_not_start_it() {
        // Whether or not the libraries exist, this thread never initialized GTK.
        assert!(Gtk::current().is_none());
    }

    #[test]
    fn a_start_result_is_seen_by_every_waiter() {
        let task = StartGtkTask::pending();
        assert_eq!(task.result(), None);
        let other = task.clone();
        task.try_set_result(false);
        task.try_set_result(true);
        assert_eq!(other.result(), Some(false));
        assert_eq!(StartGtkTask::completed(true).result(), Some(true));
    }

    #[test]
    fn a_text_ends_at_its_first_zero() {
        assert_eq!(c_string("ab\0cd").as_bytes(), b"ab");
        assert_eq!(c_string("name").as_bytes(), b"name");
    }
}
