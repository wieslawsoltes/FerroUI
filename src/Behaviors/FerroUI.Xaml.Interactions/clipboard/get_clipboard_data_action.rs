//! Port of `Clipboard/GetClipboardDataAction.cs`.

use super::{resolve_clipboard, visual_of};
use crate::action_task::ActionTask;
use crate::{ClipboardRef, StorageItemRef};
use ferroui_base::input::platform::{ClipboardExtensions, IClipboard};
use ferroui_base::input::{AsyncDataTransferExtensions, DataFormat, IAsyncDataTransfer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::{InvokeCommandActionBase, StyledElementActionImpl};
use std::rc::Rc;

const TEXT_FORMAT: &str = "Text";
const FILES_FORMAT: &str = "Files";
const FILE_NAMES_FORMAT: &str = "FileNames";

/// An action that will get the data of a format from the clipboard and pass it to its command.
///
/// The format `Text` gives the text (`String`), `Files` the files and folders
/// (`Vec<StorageItemRef>`), `FileNames` their local paths (`Vec<String>`); any other format is read
/// as the text of the application format of that name, else as the bytes of the platform format of
/// that name (`Rc<[u8]>`).
///
/// Deviation (DEVIATIONS.md, Behaviors): the `BinaryFormatter` payloads the reference deserializes
/// on one of its targets are not deserialized (the reference disables that path on its other
/// targets too); their bytes are passed.
#[repr(C)]
pub struct GetClipboardDataAction {
    base: InvokeCommandActionBase,
}

ferro_class!(GetClipboardDataAction: InvokeCommandActionBase);
ferro_class_info!(GetClipboardDataAction { new: GetClipboardDataAction::new });
ferroui_base::ferro_impl_classes!(GetClipboardDataAction: StyledElementImpl);

ferro_properties! {
    impl GetClipboardDataAction {
        /// Identifies the `Clipboard` property.
        pub fn clipboard_property() -> StyledProperty<Option<ClipboardRef>> {
            FerroProperty::register::<GetClipboardDataAction, _>("Clipboard", None)
        }

        /// Identifies the `Format` property.
        pub fn format_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<GetClipboardDataAction, _>("Format", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for GetClipboardDataAction {
        /// The constructor body: the data is passed to the command.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_pass_event_args_to_command(true);
        }
    }
}

impl StyledElementActionImpl for GetClipboardDataAction {
    /// Executes the action: reads the data of the clipboard in a job of the dispatcher and
    /// executes the command with it.
    ///
    /// Returns true if the operation is started (the sender is a visual); else false.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = visual_of(sender) else {
            return Some(Rc::new(false));
        };

        let action: Ref<GetClipboardDataAction> = this.to_ref();
        drop(ActionTask::spawn(async move { action.get_clipboard_data_async(&visual).await }));

        Some(Rc::new(true))
    }
}

/// Whether `format` names `expected`, ignoring case.
fn is_format(format: &str, expected: &str) -> bool {
    format.eq_ignore_ascii_case(expected)
}

/// Whether `identifier` is a valid identifier of an application format (only ASCII letters,
/// digits, the dot and the hyphen, and not empty); the reference catches the argument exception
/// of the creation of the format instead.
fn is_valid_application_format_identifier(identifier: &str) -> bool {
    !identifier.is_empty() && identifier.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

impl GetClipboardDataAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InvokeCommandActionBase::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the clipboard to use. This is a property of the object model.
    pub fn clipboard(&self) -> Option<ClipboardRef> {
        self.get_value(Self::clipboard_property())
    }

    pub fn set_clipboard(&self, value: Option<ClipboardRef>) {
        self.set_value(Self::clipboard_property(), value)
    }

    /// Gets or sets the format to get from the clipboard. This is a property of the object model.
    pub fn format(&self) -> Option<String> {
        self.get_value(Self::format_property())
    }

    pub fn set_format(&self, value: Option<String>) {
        self.set_value(Self::format_property(), value)
    }

    async fn get_clipboard_data_async(&self, visual: &Visual) {
        let Some(command) = self.command().filter(|_| self.is_enabled() && self.format().is_some()) else {
            return;
        };

        let Some(clipboard) = resolve_clipboard(self.clipboard(), visual) else {
            return;
        };

        // A failure of the clipboard ends the operation (the exception the reference ignores).
        let Some(data) = self.read_clipboard_data_async(&clipboard).await else {
            return;
        };
        let resolved_parameter = self.resolve_parameter(data.as_ref());

        if !command.can_execute(resolved_parameter.as_ref()) {
            return;
        }

        command.execute(resolved_parameter.as_ref());
    }

    /// The data of the format on the clipboard (`GetClipboardDataAsync(clipboard)`): `None` when
    /// the clipboard failed, `Some(None)` when it has no data of the format.
    async fn read_clipboard_data_async(&self, clipboard: &Rc<dyn IClipboard>) -> Option<Option<BoxedValue>> {
        let Some(format) = self.format() else {
            return Some(None);
        };

        if is_format(&format, TEXT_FORMAT) {
            let text = clipboard.try_get_text_async().await.ok()?;
            return Some(text.map(|text| Rc::new(text) as BoxedValue));
        }

        if is_format(&format, FILES_FORMAT) {
            let files = clipboard.try_get_files_async().await.ok()?;
            return Some(files.map(|files| {
                Rc::new(files.into_iter().map(StorageItemRef::new).collect::<Vec<_>>()) as BoxedValue
            }));
        }

        if is_format(&format, FILE_NAMES_FORMAT) {
            let files = clipboard.try_get_files_async().await.ok()?;
            return Some(files.map(|files| {
                Rc::new(files.iter().filter_map(|file| file.try_get_local_path()).collect::<Vec<String>>()) as BoxedValue
            }));
        }

        let data_transfer = clipboard.try_get_data_async().await.ok()?;
        let Some(data_transfer) = data_transfer else {
            return Some(None);
        };

        // `using (dataTransfer)`.
        let result = Self::read_data_transfer_async(&data_transfer, &format).await;
        data_transfer.dispose();
        result
    }

    async fn read_data_transfer_async(data_transfer: &Rc<dyn IAsyncDataTransfer>, format: &str) -> Option<Option<BoxedValue>> {
        // Try to treat the format as an application string identifier first.
        if let Some(string_value) = Self::try_get_application_string_async(data_transfer, format).await? {
            return Some(Some(Rc::new(string_value)));
        }

        // The creation of a platform format rejects an empty identifier (an argument exception of
        // the reference, which ends the operation).
        if format.is_empty() {
            return None;
        }

        let bytes = data_transfer.try_get_value_async(&DataFormat::create_bytes_platform_format(format)).await.ok()?;
        Some(bytes.map(|bytes| Rc::new(bytes) as BoxedValue))
    }

    async fn try_get_application_string_async(
        data_transfer: &Rc<dyn IAsyncDataTransfer>,
        format: &str,
    ) -> Option<Option<String>> {
        if !is_valid_application_format_identifier(format) {
            return Some(None);
        }

        let string_format = DataFormat::create_string_application_format(format);
        data_transfer.try_get_value_async(&string_format).await.ok()
    }
}
