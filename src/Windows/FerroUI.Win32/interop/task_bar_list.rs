//! Port of `Interop/TaskBarList.cs`: the task bar list of the shell, a COM
//! object the backend tells that a window is full screen (so the task bar
//! goes behind it) and whose overlay icon it resets to make the task bar
//! draw the icon of a window again.
//!
//! The reference keeps one object for the process behind a lock and never
//! releases it. Here the object belongs to the thread that asked for it
//! (an interface pointer is used in the apartment that created it) and is
//! released when the thread ends; a creation that failed is remembered
//! and not tried again, as the reference remembers it.

use std::ffi::c_void;

/// The vtable of `ITaskbarList3`, as the reference declares it: the three
/// methods of `IUnknown`, then the methods of the three versions of the
/// interface in their order. The methods the backend calls have their
/// types; the others are addresses.
#[repr(C)]
pub struct ITaskBarList3VTable {
    pub i_unknown1: *const c_void,
    pub i_unknown2: *const c_void,
    /// `IUnknown::Release`.
    pub i_unknown3: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub hr_init: unsafe extern "system" fn(this: *mut c_void) -> i32,
    pub add_tab: *const c_void,
    pub delete_tab: *const c_void,
    pub activate_tab: *const c_void,
    pub set_active_alt: *const c_void,
    pub mark_fullscreen_window: unsafe extern "system" fn(this: *mut c_void, hwnd: isize, fullscreen: i32) -> i32,
    pub set_progress_value: *const c_void,
    pub set_progress_state: *const c_void,
    pub register_tab: *const c_void,
    pub unregister_tab: *const c_void,
    pub set_tab_order: *const c_void,
    pub set_tab_active: *const c_void,
    pub thumb_bar_add_buttons: *const c_void,
    pub thumb_bar_update_buttons: *const c_void,
    pub thumb_bar_set_image_list: *const c_void,
    pub set_overlay_icon:
        unsafe extern "system" fn(this: *mut c_void, hwnd: isize, h_icon: isize, description: *const u16) -> i32,
    pub set_thumbnail_tooltip: *const c_void,
    pub set_thumbnail_clip: *const c_void,
}

/// The description of an overlay icon as the string the system reads: its
/// UTF-16 code units with a terminator, or nothing (a null pointer) when
/// there is no description.
pub(crate) fn description_buffer(description: Option<&str>) -> Option<Vec<u16>> {
    description.map(|description| description.encode_utf16().chain(std::iter::once(0)).collect())
}

#[cfg(windows)]
pub(crate) use imp::TaskBarList;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::co_create_instance;
    use crate::interop::unmanaged_methods::ShellIds;
    use std::cell::OnceCell;

    /// `CLSCTX_INPROC_SERVER`.
    const CLSCTX_INPROC_SERVER: u32 = 1;

    /// The task bar list of a thread: the interface pointer, released when
    /// the thread ends.
    struct Instance(*mut *const ITaskBarList3VTable);

    impl Instance {
        fn vtable(&self) -> &ITaskBarList3VTable {
            // SAFETY: the pointer is a live COM interface pointer (this
            // value holds a reference): it points at a pointer to the
            // vtable of the object, which lives as long as the object.
            unsafe { &**self.0 }
        }
    }

    impl Drop for Instance {
        fn drop(&mut self) {
            // SAFETY: `Release` of the interface, with the pointer whose
            // reference this value holds and does not use again.
            unsafe { (self.vtable().i_unknown3)(self.0.cast()) };
        }
    }

    thread_local! {
        static TASK_BAR_LIST: OnceCell<Option<Instance>> = const { OnceCell::new() };
    }

    pub(crate) struct TaskBarList;

    impl TaskBarList {
        fn init() -> Option<Instance> {
            let instance = co_create_instance(&ShellIds::TASK_BAR_LIST, CLSCTX_INPROC_SERVER, &ShellIds::I_TASK_BAR_LIST2).ok()?;
            if instance.is_null() {
                return None;
            }
            let instance = Instance(instance.cast());

            // SAFETY: `HrInit` of the interface that was just created, with
            // its own pointer.
            if unsafe { (instance.vtable().hr_init)(instance.0.cast()) } != 0 {
                return None;
            }

            Some(instance)
        }

        fn with(f: impl FnOnce(&Instance)) {
            // A thread that is ending has no task bar list any more.
            let _ = TASK_BAR_LIST.try_with(|cell| {
                if let Some(instance) = cell.get_or_init(Self::init) {
                    f(instance);
                }
            });
        }

        /// Ported from https://github.com/chromium/chromium/blob/master/ui/views/win/fullscreen_handler.cc
        pub(crate) fn mark_fullscreen(hwnd: isize, fullscreen: bool) {
            Self::with(|instance| {
                // SAFETY: the method of the interface with its own pointer;
                // a handle and a number.
                unsafe { (instance.vtable().mark_fullscreen_window)(instance.0.cast(), hwnd, i32::from(fullscreen)) };
            });
        }

        pub(crate) fn set_overlay_icon(hwnd: isize, h_icon: isize, description: Option<&str>) {
            Self::with(|instance| {
                let description = description_buffer(description);
                let pointer = description.as_ref().map_or(std::ptr::null(), |description| description.as_ptr());
                // SAFETY: the method of the interface with its own pointer;
                // the description is null or a string with a terminator
                // that lives until the call returns.
                unsafe { (instance.vtable().set_overlay_icon)(instance.0.cast(), hwnd, h_icon, pointer) };
            });
        }

        /// Whether the thread has a task bar list (the shell created and
        /// initialised one). For the test against the system.
        #[cfg(test)]
        pub(crate) fn is_available() -> bool {
            let mut available = false;
            Self::with(|_| available = true);
            available
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn the_vtable_has_the_methods_of_the_three_versions_in_order() {
        let pointer = size_of::<usize>();
        assert_eq!(size_of::<ITaskBarList3VTable>(), 21 * pointer);
        assert_eq!(offset_of!(ITaskBarList3VTable, i_unknown3), 2 * pointer);
        assert_eq!(offset_of!(ITaskBarList3VTable, hr_init), 3 * pointer);
        // ITaskbarList: HrInit, AddTab, DeleteTab, ActivateTab, SetActiveAlt;
        // ITaskbarList2: MarkFullscreenWindow.
        assert_eq!(offset_of!(ITaskBarList3VTable, mark_fullscreen_window), 8 * pointer);
        // ITaskbarList3: nine methods before SetOverlayIcon.
        assert_eq!(offset_of!(ITaskBarList3VTable, set_overlay_icon), 18 * pointer);
    }

    /// Against the system: the shell creates and initialises the object on
    /// a thread with COM, and the two methods are called through its
    /// vtable (with a handle that is no window, which the shell refuses).
    #[cfg(windows)]
    #[test]
    fn the_shell_creates_the_task_bar_list_and_takes_the_calls() {
        let result = crate::interop::unmanaged_methods::ole_initialize();
        assert!(result == 0 || result == 1, "OleInitialize: {result:#010x}");
        assert!(TaskBarList::is_available(), "the task bar list of the shell could not be created");
        TaskBarList::mark_fullscreen(0, false);
        TaskBarList::set_overlay_icon(0, 0, None);
        TaskBarList::set_overlay_icon(0, 0, Some("none"));
        assert!(TaskBarList::is_available());
    }

    #[test]
    fn a_description_is_a_string_with_a_terminator_or_nothing() {
        assert_eq!(description_buffer(None), None);
        assert_eq!(description_buffer(Some("")), Some(vec![0]));
        assert_eq!(description_buffer(Some("ab")), Some(vec![0x61, 0x62, 0]));
    }
}
