//! A shared library that is opened at run time and asked for its functions
//! by name (no upstream file: the reference declares the functions of a
//! library as platform invokes, which its runtime resolves this way when a
//! function is first called).
//!
//! Nothing of GLib, GTK, the session management library or the ICE library
//! is linked: the crate builds and starts without them, and a library that
//! is missing shows as an error value where the reference gets an
//! exception.

use std::ffi::{c_void, CStr, CString};

/// A library that was opened. It is never closed, so what it exports stays
/// valid for the life of the process.
#[derive(Clone, Copy, Debug)]
pub struct NativeLibrary {
    /// The handle of the dynamic loader, as a number.
    handle: usize,
    name: &'static CStr,
}

/// The message of a library or of a function that cannot be had.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeLibraryError(pub String);

impl std::fmt::Display for NativeLibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NativeLibraryError {}

impl NativeLibrary {
    /// Opens the library of that name, as the dynamic loader finds it.
    pub fn open(name: &'static CStr) -> Result<Self, NativeLibraryError> {
        // SAFETY: the name is a terminated string. The initializers of the library run, as
        // they do for any library a process loads; the handle is kept for ever.
        let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL) };
        if handle.is_null() {
            return Err(NativeLibraryError(format!(
                "Unable to load shared library '{}' or one of its dependencies.",
                name.to_string_lossy()
            )));
        }
        Ok(Self { handle: handle as usize, name })
    }

    /// The name the library was opened by.
    pub fn name(&self) -> &'static CStr {
        self.name
    }

    /// The address of the function of that name.
    pub fn symbol(&self, symbol: &str) -> Result<*const c_void, NativeLibraryError> {
        let missing = || {
            NativeLibraryError(format!(
                "Unable to find an entry point named '{symbol}' in shared library '{}'.",
                self.name.to_string_lossy()
            ))
        };
        let c_symbol = CString::new(symbol).map_err(|_| missing())?;
        // SAFETY: the handle is the one `dlopen` returned and the library is still loaded;
        // the name is a terminated string that lives for the call.
        let address = unsafe { libc::dlsym(self.handle as *mut c_void, c_symbol.as_ptr()) };
        if address.is_null() {
            return Err(missing());
        }
        Ok(address.cast_const())
    }
}

/// Declares the functions the port calls of one or more libraries as a
/// table of function pointers, with a constructor that resolves every one
/// of them or fails with the name of the first that is missing.
///
/// Each entry names the library (an argument of `load`) the function is
/// in, as the `DllImport` attribute of the reference does.
macro_rules! native_functions {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ($($lib:ident),+ $(,)?) {
            $( $(#[$fmeta:meta])* $flib:ident :: $f:ident : $ty:ty ; )*
        }
    ) => {
        $(#[$meta])*
        #[allow(non_snake_case, dead_code)]
        $vis struct $name {
            $( $(#[$fmeta])* pub(crate) $f: $ty, )*
        }

        impl $name {
            /// Resolves every function of the table.
            #[allow(dead_code, non_snake_case)]
            $vis fn load(
                $($lib: &$crate::interop::native_library::NativeLibrary),+
            ) -> Result<Self, $crate::interop::native_library::NativeLibraryError> {
                Ok(Self {
                    $(
                        // SAFETY: the address is that of the exported function of this name,
                        // whose C declaration is the type of the field; a function pointer
                        // has the size and the validity of a non-null address.
                        $f: unsafe {
                            std::mem::transmute::<*const std::ffi::c_void, $ty>($flib.symbol(stringify!($f))?)
                        },
                    )*
                })
            }
        }
    };
}
pub(crate) use native_functions;

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;

    #[test]
    fn a_library_that_does_not_exist_is_an_error_with_its_name() {
        let error = NativeLibrary::open(c"libferroui-there-is-no-such-library.so.0").unwrap_err();
        assert_eq!(
            error.to_string(),
            "Unable to load shared library 'libferroui-there-is-no-such-library.so.0' or one of its dependencies."
        );
    }

    /// The C library is loaded in every process: a function it has is
    /// found, one it does not have is an error that names both.
    #[test]
    fn a_function_is_found_by_name_and_a_missing_one_is_an_error() {
        #[cfg(target_os = "linux")]
        const LIBC: &CStr = c"libc.so.6";
        #[cfg(target_os = "macos")]
        const LIBC: &CStr = c"libSystem.B.dylib";
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        const LIBC: &CStr = c"libc.so";

        let Ok(library) = NativeLibrary::open(LIBC) else {
            return;
        };
        assert!(library.symbol("getpid").is_ok());
        let error = library.symbol("ferroui_no_such_function").unwrap_err();
        assert!(error.to_string().starts_with("Unable to find an entry point named 'ferroui_no_such_function'"));

        native_functions! {
            struct Table(lib) {
                lib::getpid: unsafe extern "C" fn() -> libc::pid_t;
            }
        }
        let table = Table::load(&library).unwrap();
        // SAFETY: `getpid` takes nothing and always succeeds.
        assert_eq!(unsafe { (table.getpid)() } as u32, std::process::id());

        native_functions! {
            struct Missing(lib) {
                lib::ferroui_no_such_function: unsafe extern "C" fn();
            }
        }
        assert!(Missing::load(&library).is_err());
    }
}
