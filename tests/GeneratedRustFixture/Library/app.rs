//! The class of the document `App.xaml`: an application class with `new()` only.

use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl};

/// An application whose resources merge a dictionary of the library.
#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_class_info!(App { new: App::new });
ferro_impl_classes!(App: FerroObjectImpl);

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        crate::register_types();
        if let Err(error) = crate::compiled_app::populate(None, &this.to_ref()) {
            panic!("{error}");
        }
    }
}

impl App {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: Application::construct() }
    }

    /// Creates the application; its document populates it when it is initialised.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
