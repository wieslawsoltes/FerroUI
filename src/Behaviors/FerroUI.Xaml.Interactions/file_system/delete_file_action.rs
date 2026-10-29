//! Port of `FileSystem/DeleteFileAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that deletes a file at the specified path.
#[repr(C)]
pub struct DeleteFileAction {
    base: StyledElementAction,
}

ferro_class!(DeleteFileAction: StyledElementAction);
ferro_class_info!(DeleteFileAction { new: DeleteFileAction::new });
ferroui_base::ferro_impl_classes!(DeleteFileAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl DeleteFileAction {
        /// Identifies the `Path` property.
        pub fn path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<DeleteFileAction, _>("Path", None)
        }
    }
}

impl StyledElementActionImpl for DeleteFileAction {
    /// Deletes the file. Returns true if it was deleted; else false.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(path) = this.path().filter(|path| !path.is_empty() && std::path::Path::new(path).is_file()) else {
            return Some(Rc::new(false));
        };

        Some(Rc::new(std::fs::remove_file(path).is_ok()))
    }
}

impl DeleteFileAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the path of the file to delete.
    pub fn path(&self) -> Option<String> {
        self.get_value(Self::path_property())
    }

    pub fn set_path(&self, value: Option<String>) {
        self.set_value(Self::path_property(), value)
    }
}
