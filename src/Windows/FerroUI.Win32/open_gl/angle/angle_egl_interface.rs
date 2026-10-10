//! The EGL of ANGLE: the entry points of EGL and the two of
//! `EGL_ANGLE_device_creation`.
//!
//! The reference loads its own build of ANGLE from a library next to the
//! application and resolves the entry points through the function that
//! library exports. Here ANGLE is the crate `mozangle`, compiled from source
//! and linked into the application with the feature `angle` of this crate
//! (`docs/porting/win32-platform.md`, section 6.2); its
//! `egl::get_proc_address` is the loader. Without the feature there is no
//! ANGLE and [`Win32AngleEglInterface::new`] says so, which the graphics
//! manager treats like a library that could not be loaded.
//!
//! Everything above the loader is written against a loader function, so the
//! tests run it against one of their own on every host.

use ferroui_opengl::egl::EglInterface;
use ferroui_opengl::{GetProcAddress, OpenGlException};
use std::ffi::c_void;
use std::rc::Rc;

type CreateDeviceAngle = unsafe extern "system" fn(device_type: i32, native_device: isize, attribs: *const isize) -> isize;
type ReleaseDeviceAngle = unsafe extern "system" fn(device: isize) -> u32;

/// The entry points of the EGL of ANGLE on Windows.
///
/// The reference derives the type from the EGL interface; here it holds the
/// interface ([`egl`](Self::egl)) beside the two entry points it adds.
pub struct Win32AngleEglInterface {
    egl: Rc<EglInterface>,
    create_device_angle: Option<CreateDeviceAngle>,
    release_device_angle: Option<ReleaseDeviceAngle>,
}

impl Win32AngleEglInterface {
    /// Loads ANGLE.
    ///
    /// # Panics
    /// Panics when ANGLE lacks an entry point of EGL the framework requires
    /// (the reference throws, and reports ANGLE as not loadable; the ANGLE
    /// of this build is linked in, so its entry points are known when the
    /// application is built).
    pub fn new() -> Result<Win32AngleEglInterface, OpenGlException> {
        let get_proc_address = load_angle()?;
        // SAFETY: the loader of ANGLE returns, for a name, null or the
        // address of the entry point of that name, and the entry points are
        // part of the program.
        Ok(unsafe { Self::from_loader(&get_proc_address) })
    }

    /// Resolves the entry points with `get_proc_address`.
    ///
    /// # Safety
    /// The contract of [`EglInterface::new`]: for a name, the loader returns
    /// null or the address of the entry point of that name, which stays
    /// valid for as long as the interface is used.
    ///
    /// # Panics
    /// Panics when a required entry point of EGL cannot be resolved.
    pub unsafe fn from_loader(get_proc_address: &GetProcAddress) -> Win32AngleEglInterface {
        // SAFETY: the caller upholds the contract of the loader.
        let egl = Rc::new(unsafe { EglInterface::new(get_proc_address) });
        let create_device_angle = get_proc_address("eglCreateDeviceANGLE");
        let release_device_angle = get_proc_address("eglReleaseDeviceANGLE");
        Win32AngleEglInterface {
            egl,
            // SAFETY: an address that is not null is the entry point of that
            // name (the contract of the loader), whose signature under the
            // calling convention of the system is the one declared above.
            create_device_angle: (!create_device_angle.is_null())
                .then(|| unsafe { std::mem::transmute::<*const c_void, CreateDeviceAngle>(create_device_angle) }),
            // SAFETY: as above.
            release_device_angle: (!release_device_angle.is_null())
                .then(|| unsafe { std::mem::transmute::<*const c_void, ReleaseDeviceAngle>(release_device_angle) }),
        }
    }

    /// The entry points of EGL.
    pub fn egl(&self) -> &Rc<EglInterface> {
        &self.egl
    }

    /// Whether ANGLE has `eglCreateDeviceANGLE` (`EGL_ANGLE_device_creation`).
    pub fn is_create_device_angle_available(&self) -> bool {
        self.create_device_angle.is_some()
    }

    /// `eglCreateDeviceANGLE`: an EGL device over a device of Direct3D.
    /// `attribs`, when given, is terminated with `EGL_NONE`.
    ///
    /// # Panics
    /// Panics when ANGLE does not have the entry point.
    pub fn create_device_angle(&self, device_type: i32, native_device: isize, attribs: Option<&[isize]>) -> isize {
        let Some(entry) = self.create_device_angle else {
            panic!("Unable to find an entry point named 'eglCreateDeviceANGLE'.");
        };
        // SAFETY: the address is the entry point of this name; the attribute
        // list is null or a terminated list, and the native device is a
        // handle ANGLE validates against the device type.
        unsafe { entry(device_type, native_device, attribs.map_or(std::ptr::null(), <[isize]>::as_ptr)) }
    }

    /// `eglReleaseDeviceANGLE`.
    ///
    /// # Panics
    /// Panics when ANGLE does not have the entry point.
    pub fn release_device_angle(&self, device: isize) {
        let Some(entry) = self.release_device_angle else {
            panic!("Unable to find an entry point named 'eglReleaseDeviceANGLE'.");
        };
        // SAFETY: the address is the entry point of this name, and the
        // argument is a plain value.
        unsafe {
            entry(device);
        }
    }
}

/// The loader of the ANGLE that is linked into the application.
#[cfg(all(windows, feature = "angle"))]
fn load_angle() -> Result<GetProcAddress, OpenGlException> {
    let disp = mozangle::egl::get_proc_address("eglGetPlatformDisplayEXT");

    if disp.is_null() {
        return Err(OpenGlException::new("libegl.dll doesn't have eglGetPlatformDisplayEXT entry point"));
    }

    Ok(Rc::new(|name: &str| mozangle::egl::get_proc_address(name)))
}

/// Without the feature `angle` the application has no ANGLE.
#[cfg(not(all(windows, feature = "angle")))]
fn load_angle() -> Result<GetProcAddress, OpenGlException> {
    Err(OpenGlException::new(
        "ANGLE is not part of this build: the rendering mode AngleEgl needs the feature `angle` of ferroui-win32 \
         (docs/porting/win32-platform.md, section 6.2)",
    ))
}

/// Whether this build has ANGLE (the feature `angle` on Windows).
pub const fn is_angle_built_in() -> bool {
    cfg!(all(windows, feature = "angle"))
}

#[cfg(test)]
pub(crate) mod tests {
    // Not from upstream: the reference has no tests of the interface.
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        pub(crate) static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    unsafe extern "system" fn create_device(device_type: i32, native_device: isize, attribs: *const isize) -> isize {
        CALLS.with(|calls| {
            calls.borrow_mut().push(format!("eglCreateDeviceANGLE({device_type:#x}, {native_device}, {})", attribs.is_null()))
        });
        if native_device == 0 {
            0
        } else {
            native_device + 1000
        }
    }

    unsafe extern "system" fn release_device(device: isize) -> u32 {
        CALLS.with(|calls| calls.borrow_mut().push(format!("eglReleaseDeviceANGLE({device})")));
        1
    }

    unsafe extern "system" fn other() {}

    /// A loader whose EGL has every required entry point (functions that are
    /// never called) and, when asked, the two of the device creation.
    pub(crate) fn loader(with_device_creation: bool) -> GetProcAddress {
        Rc::new(move |name: &str| match name {
            "eglCreateDeviceANGLE" if with_device_creation => create_device as *const c_void,
            "eglReleaseDeviceANGLE" if with_device_creation => release_device as *const c_void,
            name if name.ends_with("EXT") || name.ends_with("KHR") || name.ends_with("ANGLE") => std::ptr::null(),
            _ => other as *const c_void,
        })
    }

    #[test]
    fn the_device_entry_points_are_resolved_and_called() {
        // SAFETY: the loader returns functions of the declared signatures
        // for the entry points this test calls.
        let egl = unsafe { Win32AngleEglInterface::from_loader(&loader(true)) };
        CALLS.with(|calls| calls.borrow_mut().clear());

        assert!(egl.is_create_device_angle_available());
        assert_eq!(1007, egl.create_device_angle(0x33A1, 7, None));
        assert_eq!(0, egl.create_device_angle(0x33A1, 0, Some(&[0x3038])));
        egl.release_device_angle(1007);

        assert_eq!(
            vec![
                "eglCreateDeviceANGLE(0x33a1, 7, true)".to_string(),
                "eglCreateDeviceANGLE(0x33a1, 0, false)".to_string(),
                "eglReleaseDeviceANGLE(1007)".to_string()
            ],
            CALLS.with(|calls| calls.borrow().clone())
        );
    }

    #[test]
    fn an_egl_without_device_creation_says_so() {
        // SAFETY: as above.
        let egl = unsafe { Win32AngleEglInterface::from_loader(&loader(false)) };

        assert!(!egl.is_create_device_angle_available());
    }

    #[test]
    #[should_panic(expected = "Unable to find an entry point named 'eglCreateDeviceANGLE'.")]
    fn calling_a_missing_device_entry_point_fails() {
        // SAFETY: as above.
        let egl = unsafe { Win32AngleEglInterface::from_loader(&loader(false)) };

        egl.create_device_angle(0x33A1, 7, None);
    }

    #[cfg(not(all(windows, feature = "angle")))]
    #[test]
    fn a_build_without_angle_reports_it_instead_of_loading() {
        let error = Win32AngleEglInterface::new().err().expect("no ANGLE in this build");

        assert!(error.message().contains("feature `angle`"));
        assert!(!is_angle_built_in());
    }
}
