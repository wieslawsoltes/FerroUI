//! Port of `FileSystem/CreateDirectoryAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that creates a directory at the specified path (and the directories above it).
#[repr(C)]
pub struct CreateDirectoryAction {
    base: StyledElementAction,
}

ferro_class!(CreateDirectoryAction: StyledElementAction);
ferro_class_info!(CreateDirectoryAction { new: CreateDirectoryAction::new });
ferroui_base::ferro_impl_classes!(CreateDirectoryAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl CreateDirectoryAction {
        /// Identifies the `Path` property.
        pub fn path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<CreateDirectoryAction, _>("Path", None)
        }
    }
}

impl StyledElementActionImpl for CreateDirectoryAction {
    /// Creates the directory. Returns true if it exists afterwards; else false.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(path) = this.path().filter(|path| !path.is_empty()) else {
            return Some(Rc::new(false));
        };

        Some(Rc::new(std::fs::create_dir_all(path).is_ok()))
    }
}

impl CreateDirectoryAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the path of the directory to create.
    pub fn path(&self) -> Option<String> {
        self.get_value(Self::path_property())
    }

    pub fn set_path(&self, value: Option<String>) {
        self.set_value(Self::path_property(), value)
    }
}
