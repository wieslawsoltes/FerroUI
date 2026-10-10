//! The application of the system: where the framework is set up.
//!
//! The reference is a generic class an application derives its application
//! class from, with two overridable members that make the application
//! builder. Here the application class is a class of the Java layer
//! (`FerroApplication`), which loads the library of the application, and
//! the builder is the function the application gave
//! [`android_application!`](crate::android_application).

use crate::application_lifetime::ApplicationLifetime;
use crate::interop::java::JavaObject;
use ferroui_controls::AppBuilder;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

static APPLICATION: OnceLock<JavaObject> = OnceLock::new();
static BUILD: OnceLock<fn() -> AppBuilder> = OnceLock::new();

thread_local! {
    static LIFETIME: RefCell<Option<Rc<ApplicationLifetime>>> = const { RefCell::new(None) };
}

/// The application of the system.
pub struct FerroAndroidApplication;

impl FerroAndroidApplication {
    /// Keeps the function that makes the application builder: what an
    /// application overrides `CreateAppBuilder` and `CustomizeAppBuilder`
    /// for in the reference.
    pub(crate) fn set_builder(build: fn() -> AppBuilder) {
        let _ = BUILD.set(build);
    }

    /// The application object of the system, which is its application
    /// context (`Application.Context`).
    ///
    /// # Panics
    /// Panics before the application was created.
    pub fn context() -> JavaObject {
        match APPLICATION.get() {
            Some(application) => application.clone(),
            None => panic!("The application of the system was not created yet."),
        }
    }

    /// The application context; `None` before the application was created.
    pub fn try_context() -> Option<JavaObject> {
        APPLICATION.get().cloned()
    }

    /// The lifetime of the application; `None` before it was created.
    pub fn lifetime() -> Option<Rc<ApplicationLifetime>> {
        LIFETIME.with(|lifetime| lifetime.borrow().clone())
    }

    /// `onCreate` of the application class.
    pub(crate) fn on_create(application: JavaObject) {
        let _ = APPLICATION.set(application);
        Self::initialize_app_lifetime();
    }

    fn initialize_app_lifetime() {
        let Some(build) = BUILD.get() else {
            panic!("The library has no application: write `ferroui_android::android_application!(build)`.");
        };
        let builder = build();

        let lifetime = ApplicationLifetime::new();

        LIFETIME.with(|slot| *slot.borrow_mut() = Some(lifetime.clone()));

        builder.setup_with_lifetime(lifetime);
    }
}
