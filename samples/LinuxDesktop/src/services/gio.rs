//! The Unix implementation of GAppInfo reads and resolves XDG desktop
//! entries. Load the system ABI with the same loader as FerroUI's X11 and
//! Wayland backends, keeping GTK development packages optional.

use super::DesktopApplication;
use ferroui_base::platform::native_library::{NativeLibrary, NativeLibraryError};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::{io, ptr, sync::OnceLock};

#[repr(C)]
struct GList {
    data: *mut c_void,
    next: *mut GList,
    prev: *mut GList,
}

#[repr(C)]
struct GError {
    domain: u32,
    code: c_int,
    message: *mut c_char,
}

type DestroyNotify = unsafe extern "C" fn(*mut c_void);

ferroui_base::native_functions! {
    struct Gio(gio, object, glib) {
        gio::g_app_info_get_all: unsafe extern "C" fn() -> *mut GList;
        gio::g_app_info_should_show: unsafe extern "C" fn(*mut c_void) -> c_int;
        gio::g_app_info_get_id: unsafe extern "C" fn(*mut c_void) -> *const c_char;
        gio::g_app_info_get_display_name: unsafe extern "C" fn(*mut c_void) -> *const c_char;
        gio::g_app_info_get_description: unsafe extern "C" fn(*mut c_void) -> *const c_char;
        gio::g_app_info_get_icon: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        gio::g_icon_to_string: unsafe extern "C" fn(*mut c_void) -> *mut c_char;
        gio::g_desktop_app_info_new: unsafe extern "C" fn(*const c_char) -> *mut c_void;
        gio::g_app_launch_context_new: unsafe extern "C" fn() -> *mut c_void;
        gio::g_app_launch_context_setenv: unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char);
        gio::g_app_launch_context_unsetenv: unsafe extern "C" fn(*mut c_void, *const c_char);
        gio::g_app_info_launch: unsafe extern "C" fn(*mut c_void, *mut GList, *mut c_void, *mut *mut GError) -> c_int;
        object::g_object_unref: DestroyNotify;
        glib::g_list_free_full: unsafe extern "C" fn(*mut GList, Option<DestroyNotify>);
        glib::g_error_free: unsafe extern "C" fn(*mut GError);
        glib::g_free: unsafe extern "C" fn(*mut c_void);
    }
}

fn gio() -> io::Result<&'static Gio> {
    static GIO: OnceLock<Result<Gio, NativeLibraryError>> = OnceLock::new();
    GIO.get_or_init(|| {
        Gio::load(
            &NativeLibrary::open(c"libgio-2.0.so.0")?,
            &NativeLibrary::open(c"libgobject-2.0.so.0")?,
            &NativeLibrary::open(c"libglib-2.0.so.0")?,
        )
    })
    .as_ref()
    .map_err(|error| io::Error::new(io::ErrorKind::NotFound, error.clone()))
}

/// Copies a nullable, NUL-terminated string borrowed from a live GLib object.
unsafe fn text(value: *const c_char) -> String {
    if value.is_null() {
        return String::new();
    }
    // SAFETY: guaranteed by the caller for the duration of this copy.
    unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned()
}

pub(super) fn applications() -> io::Result<Vec<DesktopApplication>> {
    let api = gio()?;
    let mut result = Vec::new();
    // SAFETY: g_app_info_get_all transfers the list and a reference to each
    // GAppInfo. Every accessor is called while its GAppInfo is alive. None
    // of the borrowed pointers escape this traversal.
    unsafe {
        let list = (api.g_app_info_get_all)();
        let mut node = list;
        while !node.is_null() {
            let info = (*node).data;
            node = (*node).next;
            if (api.g_app_info_should_show)(info) == 0 {
                continue;
            }
            let id = text((api.g_app_info_get_id)(info));
            let name = text((api.g_app_info_get_display_name)(info));
            if id.is_empty() || name.is_empty() {
                continue;
            }
            let description = text((api.g_app_info_get_description)(info));
            let icon = (api.g_app_info_get_icon)(info);
            let icon_name = if icon.is_null() {
                String::new()
            } else {
                let serialized = (api.g_icon_to_string)(icon);
                let name = text(serialized);
                (api.g_free)(serialized.cast());
                name
            };
            result.push(DesktopApplication::new(id, name, description, icon_name));
        }
        (api.g_list_free_full)(list, Some(api.g_object_unref));
    }
    Ok(result)
}

pub(super) fn launch(id: &str, display: Option<&std::ffi::OsStr>) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let display = display.map(|d| CString::new(d.as_bytes())).transpose()?;
    let api = gio()?;
    let id = CString::new(id)?;
    // SAFETY: the ID is a terminated string. new returns a reference or
    // NULL. Empty file list and default launch context are allowed. A
    // failure transfers a GError to the caller.
    unsafe {
        let info = (api.g_desktop_app_info_new)(id.as_ptr());
        if info.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Application is no longer installed",
            ));
        }
        if (api.g_app_info_should_show)(info) == 0 {
            (api.g_object_unref)(info);
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Application is no longer available in this session",
            ));
        }
        let mut error = ptr::null_mut();
        let context = if let Some(display) = display {
            // Keep the shell's presentation connection on the outer display.
            // GIO applies this environment only to the process it launches.
            let context = (api.g_app_launch_context_new)();
            (api.g_app_launch_context_setenv)(
                context,
                c"WAYLAND_DISPLAY".as_ptr(),
                display.as_ptr(),
            );
            (api.g_app_launch_context_setenv)(
                context,
                c"XDG_SESSION_TYPE".as_ptr(),
                c"wayland".as_ptr(),
            );
            for key in [
                c"WAYLAND_SOCKET",
                c"DISPLAY",
                c"SWAYSOCK",
                c"XDG_ACTIVATION_TOKEN",
                c"DESKTOP_STARTUP_ID",
            ] {
                (api.g_app_launch_context_unsetenv)(context, key.as_ptr());
            }
            context
        } else {
            ptr::null_mut()
        };
        let launched = (api.g_app_info_launch)(info, ptr::null_mut(), context, &mut error);
        if !context.is_null() {
            (api.g_object_unref)(context);
        }
        (api.g_object_unref)(info);
        if launched != 0 {
            return Ok(());
        }
        let message = if error.is_null() {
            "Application activation failed".to_owned()
        } else {
            let message = text((*error).message);
            (api.g_error_free)(error);
            message
        };
        Err(io::Error::other(message))
    }
}
