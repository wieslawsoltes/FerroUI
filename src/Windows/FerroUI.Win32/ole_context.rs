//! OLE on the UI thread: initialised on first use, and the registration of
//! the drop target of a window.

use crate::interop::unmanaged_methods::{self, co_get_apartment_type, APTTYPE_MAINSTA, APTTYPE_STA, HRESULT};
use crate::win32_com::IDropTarget;
use ferroui_controls::platform::IPlatformHandle;
use ferroui_base::threading::Dispatcher;
use ferroui_microcom::ComPtr;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static CURRENT: RefCell<Option<Rc<OleContext>>> = const { RefCell::new(None) };
}

/// OLE, initialised on the UI thread.
pub(crate) struct OleContext {
    _private: (),
}

impl OleContext {
    /// The context of the UI thread; `None` on another thread, or on a
    /// thread whose apartment OLE cannot use.
    ///
    /// # Panics
    /// Panics when OLE cannot be initialised.
    pub(crate) fn current() -> Option<Rc<OleContext>> {
        if !Self::is_valid_ole_thread() {
            return None;
        }

        if let Some(current) = CURRENT.with(|current| current.borrow().clone()) {
            return Some(current);
        }
        let context = Rc::new(OleContext::new());
        CURRENT.with(|current| *current.borrow_mut() = Some(context.clone()));
        Some(context)
    }

    fn new() -> OleContext {
        let res = unmanaged_methods::ole_initialize();

        if res != HRESULT::S_OK && res != HRESULT::S_FALSE
        /*already initialized*/
        {
            panic!("Failed to initialize OLE (result code {res:#010x})");
        }
        OleContext { _private: () }
    }

    /// The reference asks its runtime whether the thread is a
    /// single-threaded apartment, which the runtime decides when the thread
    /// starts (the attribute of the entry point). A thread of the port has
    /// no apartment until something initialises COM on it, and initialising
    /// OLE is what makes it single-threaded: so a thread without an
    /// apartment is valid, and a thread that is in the multithreaded
    /// apartment is not.
    fn is_valid_ole_thread() -> bool {
        Dispatcher::ui_thread().check_access()
            && matches!(co_get_apartment_type(), None | Some(APTTYPE_STA) | Some(APTTYPE_MAINSTA))
    }

    pub(crate) fn register_drag_drop(&self, hwnd: Option<&dyn IPlatformHandle>, target: Option<&ComPtr<IDropTarget>>) -> bool {
        let (Some(hwnd), Some(target)) = (hwnd, target) else {
            return false;
        };
        if hwnd.handle_descriptor() != Some("HWND") {
            return false;
        }

        let trg_ptr = target.as_ptr();
        // SAFETY: a live drop target, borrowed for the call; the system
        // takes a reference of its own.
        unsafe { unmanaged_methods::register_drag_drop(hwnd.handle(), trg_ptr.cast()) == HRESULT::S_OK }
    }

    pub(crate) fn unregister_drag_drop(&self, hwnd: Option<&dyn IPlatformHandle>) -> bool {
        let Some(hwnd) = hwnd else {
            return false;
        };
        if hwnd.handle_descriptor() != Some("HWND") {
            return false;
        }

        unmanaged_methods::revoke_drag_drop(hwnd.handle()) == HRESULT::S_OK
    }
}
