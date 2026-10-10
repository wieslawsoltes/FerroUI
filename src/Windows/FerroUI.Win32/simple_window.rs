//! A plain window of a class the backend registers once: the hidden parent
//! of windows without a taskbar button, and the window of helpers that only
//! need a window procedure.

use crate::interop::unmanaged_methods::{
    create_window_ex, def_window_proc, destroy_window, get_last_error, hwnd_to_isize, register_class_ex, WindowStyles,
    WindowsMessage, CW_USEDEFAULT,
};
use crate::wnd_proc_guard;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// The window procedure of a [`SimpleWindow`]: window, message, `wParam`,
/// `lParam`, to the result.
pub type SimpleWndProc = Rc<dyn Fn(isize, u32, usize, isize) -> isize>;

thread_local! {
    /// The atom of the class, registered when the first window is created.
    static CLASS_ATOM: Cell<u16> = const { Cell::new(0) };
    /// The window procedures of the windows of this thread, by handle.
    static INSTANCES: RefCell<HashMap<isize, Option<SimpleWndProc>>> = RefCell::new(HashMap::new());
    /// The window procedure of the window that is being created: the
    /// system sends its first messages before the handle is known here.
    static CREATING: RefCell<Option<Option<SimpleWndProc>>> = const { RefCell::new(None) };
}

/// A plain overlapped window that is never shown.
pub struct SimpleWindow {
    handle: Cell<isize>,
}

impl SimpleWindow {
    fn class_atom() -> u16 {
        let atom = CLASS_ATOM.get();
        if atom != 0 {
            return atom;
        }
        // One class a thread: the name carries the thread, where the
        // reference makes the name unique with a new identifier.
        let class_name = format!("FerroSimpleWindow-{:?}-{}", std::thread::current().id(), std::process::id());
        let atom = register_class_ex(&class_name, 0, wnd_proc, 0);
        CLASS_ATOM.set(atom);
        atom
    }

    /// Creates the window; `wnd_proc` handles its messages, or the default
    /// processing of the system when there is none.
    ///
    /// # Panics
    /// Panics when the system cannot create the window.
    pub fn new(wnd_proc: Option<SimpleWndProc>) -> SimpleWindow {
        CREATING.with(|creating| *creating.borrow_mut() = Some(wnd_proc));
        let hwnd = create_window_ex(
            0,
            Self::class_atom(),
            WindowStyles::WS_OVERLAPPEDWINDOW.bits(),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
        );
        CREATING.with(|creating| *creating.borrow_mut() = None);
        wnd_proc_guard::resume_pending();
        if hwnd == 0 {
            panic!("The window could not be created (error code {})", get_last_error());
        }

        SimpleWindow { handle: Cell::new(hwnd) }
    }

    /// The handle of the window; 0 once it is disposed.
    pub fn handle(&self) -> isize {
        self.handle.get()
    }

    /// Destroys the window.
    #[allow(dead_code)] // The helpers of later stages (tray icons, the volume listener) own windows they destroy.
    pub fn dispose(&self) {
        destroy_window(self.handle.get());
        self.handle.set(0);
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    let hwnd = hwnd_to_isize(hwnd);
    let handled = wnd_proc_guard::guard(None, || {
        let known = INSTANCES.with(|instances| instances.borrow().contains_key(&hwnd));
        if !known {
            // The first messages of a window arrive while it is created.
            let creating = CREATING.with(|creating| creating.borrow_mut().take());
            if let Some(window) = creating {
                INSTANCES.with(|instances| instances.borrow_mut().insert(hwnd, window));
            }
        }

        let window = INSTANCES.with(|instances| instances.borrow().get(&hwnd).cloned());

        if msg == WindowsMessage::WM_DESTROY {
            INSTANCES.with(|instances| instances.borrow_mut().remove(&hwnd));
        }

        window.flatten().map(|wnd_proc| wnd_proc(hwnd, msg, w_param, l_param))
    });

    match handled {
        Some(result) => result,
        None => def_window_proc(hwnd, msg, w_param, l_param),
    }
}
