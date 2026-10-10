//! The platform graphics of WGL, the OpenGL of the system (the port of
//! `OpenGl/WglPlatformOpenGlInterface.cs`).

use ferroui_opengl::GlVersion;

/// The platform graphics of WGL.
///
/// The reference keeps the primary context, the one it created to learn
/// that the system has an OpenGL of one of the profiles of the options. A
/// context is an object of one thread here (`Rc`), and the platform
/// graphics are shared with the render thread, which creates the contexts:
/// so the graphics hold the version of the primary context, which is all
/// the reference reads of it afterwards, and the primary context itself is
/// kept by the thread that created the graphics
/// ([`WglPlatformOpenGlInterface::primary_context`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WglPlatformOpenGlInterface {
    primary_version: GlVersion,
}

impl WglPlatformOpenGlInterface {
    /// The graphics of a primary context of `primary_version`.
    pub(crate) fn new(primary_version: GlVersion) -> WglPlatformOpenGlInterface {
        WglPlatformOpenGlInterface { primary_version }
    }

    /// The version of the primary context, which every context of the
    /// graphics is created with.
    pub fn primary_version(&self) -> GlVersion {
        self.primary_version
    }
}

#[cfg(windows)]
mod imp {
    use super::super::wgl_context::WglContext;
    use super::super::wgl_display::WglDisplay;
    use super::WglPlatformOpenGlInterface;
    use crate::win32_platform_options::Win32PlatformOptions;
    use ferroui_base::logging::{LogEventLevel, Logger};
    use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
    use ferroui_base::{FerroLocator, LocatorExtensions};
    use ferroui_opengl::{GlVersion, IGlContext, OpenGlException};
    use std::any::Any;
    use std::cell::RefCell;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;

    thread_local! {
        static PRIMARY_CONTEXT: RefCell<Option<Rc<WglContext>>> = const { RefCell::new(None) };
    }

    fn log_failure(error: &dyn std::fmt::Display) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
            let source: &dyn Any = &"WGL";
            logger.log_with_values(Some(source), "Unable to initialize WGL: {0}", &[&error.to_string()]);
        }
    }

    impl WglPlatformOpenGlInterface {
        /// The primary context (`PrimaryContext`), on the thread that
        /// created the graphics; `None` on another thread.
        pub fn primary_context() -> Option<Rc<WglContext>> {
            PRIMARY_CONTEXT.with(|primary| primary.borrow().clone())
        }

        /// A context of the version of the primary context (`CreateContext`).
        pub fn create_gl_context(&self) -> Result<Rc<dyn IGlContext>, OpenGlException> {
            match WglDisplay::create_context(&[self.primary_version], None)? {
                Some(context) => Ok(context),
                None => Err(OpenGlException::new("Unable to create additional WGL context")),
            }
        }

        /// The platform graphics, when the system creates a context of one
        /// of the profiles of the options (`Win32PlatformOptions::wgl_profiles`);
        /// `None`, with the failure logged, when not.
        pub fn try_create() -> Option<WglPlatformOpenGlInterface> {
            let opts = FerroLocator::current()
                .get_service::<Win32PlatformOptions>()
                .map_or_else(Win32PlatformOptions::default, |options| (*options).clone());
            Self::try_create_with_profiles(&opts.wgl_profiles)
        }

        pub(crate) fn try_create_with_profiles(profiles: &[GlVersion]) -> Option<WglPlatformOpenGlInterface> {
            // The reference catches every exception of the creation. The
            // entry points of a context are resolved by the OpenGL crate,
            // which panics when a required one is missing: that is caught
            // here too, and the mode is then one that did not initialize.
            match catch_unwind(AssertUnwindSafe(|| WglDisplay::create_context(profiles, None))) {
                Ok(Ok(Some(primary))) => {
                    let graphics = WglPlatformOpenGlInterface::new(primary.version());
                    PRIMARY_CONTEXT.with(|slot| *slot.borrow_mut() = Some(primary));
                    Some(graphics)
                }
                Ok(Ok(None)) => None,
                Ok(Err(error)) => {
                    log_failure(&error);
                    None
                }
                Err(panic) => {
                    let message = panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|message| (*message).to_owned()))
                        .unwrap_or_else(|| "the creation of the context panicked".to_owned());
                    log_failure(&message);
                    None
                }
            }
        }
    }

    impl IPlatformGraphics for WglPlatformOpenGlInterface {
        fn uses_shared_context(&self) -> bool {
            false
        }

        /// # Panics
        /// Panics when the context cannot be created (the exception of the
        /// reference).
        fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
            match WglDisplay::create_context(&[self.primary_version], None) {
                Ok(Some(context)) => context,
                Ok(None) => panic!("{}", OpenGlException::new("Unable to create additional WGL context")),
                Err(error) => panic!("{error}"),
            }
        }

        /// # Panics
        /// Always: the graphics have no shared context (the
        /// `NotSupportedException` of the reference).
        fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
            panic!("Specified method is not supported.")
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the platform
    // graphics.
    use super::*;
    use ferroui_opengl::GlProfileType;

    #[test]
    fn the_graphics_keep_the_version_of_the_primary_context_and_are_shared_with_the_render_thread() {
        let graphics = WglPlatformOpenGlInterface::new(GlVersion::new(GlProfileType::OpenGL, 4, 0));

        assert_eq!(GlVersion::new(GlProfileType::OpenGL, 4, 0), graphics.primary_version());
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WglPlatformOpenGlInterface>();
    }

    /// What the system of the run offers: with a driver that has the
    /// extensions the graphics are created, a context of them is made
    /// current and names itself; with the generic implementation of the
    /// system (OpenGL 1.1, the hosted runners) the mode does not
    /// initialize. Either way nothing panics, and the outcome is printed.
    #[cfg(windows)]
    #[test]
    fn the_graphics_are_created_where_the_system_has_a_driver_and_not_created_where_it_has_none() {
        use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
        use ferroui_opengl::gl_consts::{GL_RENDERER, GL_VERSION};
        use ferroui_opengl::IGlContext;

        let profiles = [GlVersion::new(GlProfileType::OpenGL, 4, 0), GlVersion::new(GlProfileType::OpenGL, 3, 2)];
        match WglPlatformOpenGlInterface::try_create_with_profiles(&profiles) {
            Some(graphics) => {
                assert!(profiles.contains(&graphics.primary_version()));
                assert!(!graphics.uses_shared_context());
                let primary = WglPlatformOpenGlInterface::primary_context().expect("the primary context of this thread");
                assert_eq!(graphics.primary_version(), primary.version());

                let context = graphics.create_gl_context().expect("a second context");
                assert!(!context.is_shared_with(&*primary));
                let shared = primary.create_shared_context(None).expect("a shared context");
                assert!(shared.is_shared_with(&*primary));
                assert!(primary.is_shared_with(&*shared));
                {
                    let current = context.make_current();
                    let gl = context.gl_interface();
                    println!(
                        "WGL: OpenGL {:?} on {:?}, stencil {}",
                        gl.get_string(GL_VERSION),
                        gl.get_string(GL_RENDERER),
                        context.stencil_size()
                    );
                    // Current already: nothing to restore.
                    context.ensure_current().dispose();
                    current.dispose();
                }
                IPlatformGraphicsContext::dispose(&*shared);
                IPlatformGraphicsContext::dispose(&*context);
                assert!(context.is_lost());
            }
            None => {
                assert!(WglPlatformOpenGlInterface::primary_context().is_none());
                println!("WGL: the system creates no context of OpenGL 4.0 or 3.2 (no driver with the extensions of WGL)");
            }
        }
    }
}
