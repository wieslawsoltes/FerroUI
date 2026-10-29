//! Port of `Clipboard/GetClipboardFormatsAction.cs`.

use super::{resolve_clipboard, visual_of};
use crate::action_task::ActionTask;
use crate::ClipboardRef;
use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::input::DataFormat;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::{InvokeCommandActionBase, StyledElementActionImpl};
use std::rc::Rc;

const TEXT_FORMAT: &str = "Text";
const FILES_FORMAT: &str = "Files";

/// An action that will get the clipboard formats and pass them to its command (a list of the
/// identifiers of the formats, `Vec<String>`).
#[repr(C)]
pub struct GetClipboardFormatsAction {
    base: InvokeCommandActionBase,
}

ferro_class!(GetClipboardFormatsAction: InvokeCommandActionBase);
ferro_class_info!(GetClipboardFormatsAction { new: GetClipboardFormatsAction::new });
ferroui_base::ferro_impl_classes!(GetClipboardFormatsAction: StyledElementImpl);

ferro_properties! {
    impl GetClipboardFormatsAction {
        /// Identifies the `Clipboard` property.
        pub fn clipboard_property() -> StyledProperty<Option<ClipboardRef>> {
            FerroProperty::register::<GetClipboardFormatsAction, _>("Clipboard", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for GetClipboardFormatsAction {
        /// The constructor body: the formats are passed to the command.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_pass_event_args_to_command(true);
        }
    }
}

impl StyledElementActionImpl for GetClipboardFormatsAction {
    /// Executes the action: reads the formats of the clipboard in a job of the dispatcher and
    /// executes the command with them.
    ///
    /// Returns true if the operation is started (the sender is a visual); else false.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = visual_of(sender) else {
            return Some(Rc::new(false));
        };

        let action: Ref<GetClipboardFormatsAction> = this.to_ref();
        drop(ActionTask::spawn(async move { action.get_clipboard_formats_async(&visual).await }));

        Some(Rc::new(true))
    }
}

impl GetClipboardFormatsAction {
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

    async fn get_clipboard_formats_async(&self, visual: &Visual) {
        let Some(command) = self.command().filter(|_| self.is_enabled()) else {
            return;
        };

        let mut formats: Option<Vec<String>> = None;

        if let Some(clipboard) = resolve_clipboard(self.clipboard(), visual) {
            // A failure of the clipboard gives no formats.
            if let Ok(data_formats) = clipboard.get_data_formats_async().await {
                formats = Some(data_formats.iter().map(Self::convert_data_format_identifier).collect());
            }
        } else {
            return;
        }

        let resolved_parameter = self.resolve_parameter(formats.map(|formats| Rc::new(formats) as BoxedValue).as_ref());

        if !command.can_execute(resolved_parameter.as_ref()) {
            return;
        }

        command.execute(resolved_parameter.as_ref());
    }

    fn convert_data_format_identifier(format: &DataFormat) -> String {
        if DataFormat::text().as_data_format() == format {
            return TEXT_FORMAT.to_string();
        }

        if DataFormat::file().as_data_format() == format {
            return FILES_FORMAT.to_string();
        }

        format.identifier().to_string()
    }
}
