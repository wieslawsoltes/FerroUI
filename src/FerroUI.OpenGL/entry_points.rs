//! The machinery behind the entry point tables.
//!
//! An OpenGL implementation is reached through function pointers that the
//! platform resolves by name. [`gl_entry_points!`] declares such a table:
//! the fields holding the addresses, the code that resolves them and one
//! method per entry point on the type that owns the table.

use std::ffi::c_void;
use std::rc::Rc;

/// Resolves an OpenGL entry point by name; null when the platform does not
/// have it.
pub type GetProcAddress = Rc<dyn Fn(&str) -> *const c_void>;

/// `addr` when it is not null, otherwise the result of `next`.
pub(crate) fn or_else(addr: *const c_void, next: impl FnOnce() -> *const c_void) -> *const c_void {
    if addr.is_null() {
        next()
    } else {
        addr
    }
}

/// The failure of calling or resolving an entry point the platform lacks.
#[cold]
pub(crate) fn entry_point_not_found(name: &str) -> ! {
    panic!("Unable to find an entry point named '{name}'.")
}

/// Declares a table of OpenGL entry points.
///
/// ```ignore
/// gl_entry_points! {
///     table Table for Owner.field(get, info: InfoType);
///
///     required safe pub fn clear(bits: i32) = get("glClear");
///     optional unsafe pub fn read_buffer(buffer: i32) = get("glReadBuffer");
/// }
/// ```
///
/// - `required` entries make loading the table panic when they cannot be
///   resolved; `optional` ones panic when called while missing and get an
///   `is_<name>_available` method.
/// - `safe` entries take plain values only. `unsafe` entries take pointers,
///   or make OpenGL read memory described by earlier calls, and become
///   `unsafe fn`.
/// - The expression after `=` resolves the address; `get` is the
///   [`GetProcAddress`] and `info` the value passed to `load`.
macro_rules! gl_entry_points {
    (
        table $Table:ident for $Owner:ident . $field:ident ($get:ident, $info:ident : $Info:ty);

        $(
            $(#[$attr:meta])*
            $kind:ident $safety:tt $vis:vis fn $name:ident ( $($arg:ident : $ty:ty),* $(,)? ) $(-> $ret:ty)? = $loader:expr;
        )*
    ) => {
        pub(crate) struct $Table {
            $( $name: Option<unsafe extern "system" fn($($ty),*) $(-> $ret)?>, )*
        }

        impl $Table {
            #[allow(unused_variables)]
            pub(crate) fn load($get: &$crate::entry_points::GetProcAddress, $info: &$Info) -> Self {
                Self {
                    $(
                        $name: {
                            let addr: *const ::std::ffi::c_void = $loader;
                            gl_entry_points!(@check $kind $name addr);
                            if addr.is_null() {
                                None
                            } else {
                                // SAFETY: whoever constructs the owning interface guarantees
                                // that the loader returns, for a name, either null or the
                                // address of that OpenGL entry point, whose signature under
                                // the platform's calling convention is the one declared here.
                                Some(unsafe {
                                    ::std::mem::transmute::<
                                        *const ::std::ffi::c_void,
                                        unsafe extern "system" fn($($ty),*) $(-> $ret)?,
                                    >(addr)
                                })
                            }
                        },
                    )*
                }
            }
        }

        impl $Owner {
            $(
                gl_entry_points!(@method $kind $safety [$(#[$attr])*] $vis $field $name ($($arg : $ty),*) $(-> $ret)?);
            )*
        }
    };

    (@check required $name:ident $addr:ident) => {
        if $addr.is_null() {
            $crate::entry_points::entry_point_not_found(stringify!($name));
        }
    };
    (@check optional $name:ident $addr:ident) => {};

    (@method required $safety:tt [$(#[$attr:meta])*] $vis:vis $field:ident $name:ident ($($arg:ident : $ty:ty),*) $(-> $ret:ty)?) => {
        gl_entry_points!(@call $safety [$(#[$attr])*] $vis $field $name ($($arg : $ty),*) $(-> $ret)?);
    };
    (@method optional $safety:tt [$(#[$attr:meta])*] $vis:vis $field:ident $name:ident ($($arg:ident : $ty:ty),*) $(-> $ret:ty)?) => {
        gl_entry_points!(@call $safety [$(#[$attr])*] $vis $field $name ($($arg : $ty),*) $(-> $ret)?);

        ::paste::paste! {
            #[doc = concat!("Whether the context has the entry point behind `", stringify!($name), "`.")]
            $vis fn [<is_ $name _available>](&self) -> bool {
                self.$field.$name.is_some()
            }
        }
    };

    (@call safe [$(#[$attr:meta])*] $vis:vis $field:ident $name:ident ($($arg:ident : $ty:ty),*) $(-> $ret:ty)?) => {
        $(#[$attr])*
        $vis fn $name(&self, $($arg: $ty),*) $(-> $ret)? {
            match self.$field.$name {
                // SAFETY: the address is the entry point of this name (see
                // `load`), the arguments are plain values and the call is made on
                // the thread of the context, as every use of the interface is.
                Some(entry) => unsafe { entry($($arg),*) },
                None => $crate::entry_points::entry_point_not_found(stringify!($name)),
            }
        }
    };
    (@call unsafe [$(#[$attr:meta])*] $vis:vis $field:ident $name:ident ($($arg:ident : $ty:ty),*) $(-> $ret:ty)?) => {
        $(#[$attr])*
        ///
        /// # Safety
        /// The pointers passed, and the memory OpenGL reads or writes because
        /// of this call, must satisfy what the OpenGL specification requires
        /// for the entry point.
        $vis unsafe fn $name(&self, $($arg: $ty),*) $(-> $ret)? {
            match self.$field.$name {
                // SAFETY: the address is the entry point of this name (see
                // `load`); the caller upholds the requirements on the arguments.
                Some(entry) => unsafe { entry($($arg),*) },
                None => $crate::entry_points::entry_point_not_found(stringify!($name)),
            }
        }
    };
}

pub(crate) use gl_entry_points;
