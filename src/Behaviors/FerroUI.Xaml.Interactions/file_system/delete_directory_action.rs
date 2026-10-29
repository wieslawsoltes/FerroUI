//! Port of `FileSystem/DeleteDirectoryAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that deletes a directory at the specified path.
#[repr(C)]
pub struct DeleteDirectoryAction {
    base: StyledElementAction,
}

ferro_class!(DeleteDirectoryAction: StyledElementAction);
ferro_class_info!(DeleteDirectoryAction { new: DeleteDirectoryAction::new });
ferroui_base::ferro_impl_classes!(DeleteDirectoryAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl DeleteDirectoryAction {
        /// Identifies the `Path` property.
        pub fn path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<DeleteDirectoryAction, _>("Path", None)
        }

        /// Identifies the `Recursive` property.
        pub fn recursive_property() -> StyledProperty<bool> {
            FerroProperty::register::<DeleteDirectoryAction, _>("Recursive", true)
        }
    }
}

impl StyledElementActionImpl for DeleteDirectoryAction {
    /// Deletes the directory. Returns true if it was deleted; else false.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(path) = this.path().filter(|path| !path.is_empty() && std::path::Path::new(path).is_dir()) else {
            return Some(Rc::new(false));
        };

        let result = if this.recursive() { std::fs::remove_dir_all(&path) } else { std::fs::remove_dir(&path) };
        Some(Rc::new(result.is_ok()))
    }
}

impl DeleteDirectoryAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the path of the directory to delete.
    pub fn path(&self) -> Option<String> {
        self.get_value(Self::path_property())
    }

    pub fn set_path(&self, value: Option<String>) {
        self.set_value(Self::path_property(), value)
    }

    /// Gets or sets a value indicating whether to delete subdirectories and files.
    pub fn recursive(&self) -> bool {
        self.get_value(Self::recursive_property())
    }

    pub fn set_recursive(&self, value: bool) {
        self.set_value(Self::recursive_property(), value)
    }
}
