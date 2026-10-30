//! Port of `FileSystem/WriteTextToFileAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that writes text to a file.
#[repr(C)]
pub struct WriteTextToFileAction {
    base: StyledElementAction,
}

ferro_class!(WriteTextToFileAction: StyledElementAction);
ferro_class_info!(WriteTextToFileAction { new: WriteTextToFileAction::new });
ferroui_base::ferro_impl_classes!(WriteTextToFileAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl WriteTextToFileAction {
        /// Identifies the `Path` property.
        pub fn path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<WriteTextToFileAction, _>("Path", None)
        }

        /// Identifies the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<WriteTextToFileAction, _>("Text", None)
        }

        /// Identifies the `Append` property.
        pub fn append_property() -> StyledProperty<bool> {
            FerroProperty::register::<WriteTextToFileAction, _>("Append", false)
        }
    }
}

impl StyledElementActionImpl for WriteTextToFileAction {
    /// Writes the text (creating the file, or replacing its content unless
    /// [`append`](WriteTextToFileAction::append)). Returns true if it was written; else false.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let (Some(path), Some(text)) = (this.path().filter(|path| !path.is_empty()), this.text()) else {
            return Some(Rc::new(false));
        };

        let result = if this.append() {
            use std::io::Write;
            std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut file| file.write_all(text.as_bytes()))
        } else {
            std::fs::write(&path, text)
        };
        Some(Rc::new(result.is_ok()))
    }
}

impl WriteTextToFileAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the path of the file.
    pub fn path(&self) -> Option<String> {
        self.get_value(Self::path_property())
    }

    pub fn set_path(&self, value: Option<String>) {
        self.set_value(Self::path_property(), value)
    }

    /// Gets or sets the text to write.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<String>) {
        self.set_value(Self::text_property(), value)
    }

    /// Gets or sets a value indicating whether to append text to the file.
    pub fn append(&self) -> bool {
        self.get_value(Self::append_property())
    }

    pub fn set_append(&self, value: bool) {
        self.set_value(Self::append_property(), value)
    }
}
