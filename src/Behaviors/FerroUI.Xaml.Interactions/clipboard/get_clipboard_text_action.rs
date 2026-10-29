//! Port of `Clipboard/GetClipboardTextAction.cs`.

use super::{resolve_clipboard, visual_of};
use crate::action_task::ActionTask;
use crate::ClipboardRef;
use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::{InvokeCommandActionBase, StyledElementActionImpl};
use std::rc::Rc;

/// An action that will get the text from the clipboard and pass it to its command.
#[repr(C)]
pub struct GetClipboardTextAction {
    base: InvokeCommandActionBase,
}

ferro_class!(GetClipboardTextAction: InvokeCommandActionBase);
ferro_class_info!(GetClipboardTextAction { new: GetClipboardTextAction::new });
ferroui_base::ferro_impl_classes!(GetClipboardTextAction: StyledElementImpl);

ferro_properties! {
    impl GetClipboardTextAction {
        /// Identifies the `Clipboard` property.
        pub fn clipboard_property() -> StyledProperty<Option<ClipboardRef>> {
            FerroProperty::register::<GetClipboardTextAction, _>("Clipboard", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for GetClipboardTextAction {
        /// The constructor body: the text is passed to the command.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_pass_event_args_to_command(true);
        }
    }
}

impl StyledElementActionImpl for GetClipboardTextAction {
    /// Executes the action: reads the text of the clipboard in a job of the dispatcher and
    /// executes the command with it.
    ///
    /// Returns true if the operation is started (the sender is a visual); else false.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = visual_of(sender) else {
            return Some(Rc::new(false));
        };

        let action: Ref<GetClipboardTextAction> = this.to_ref();
        drop(ActionTask::spawn(async move { action.get_clipboard_text_async(&visual).await }));

        Some(Rc::new(true))
    }
}

impl GetClipboardTextAction {
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

    async fn get_clipboard_text_async(&self, visual: &Visual) {
        let Some(command) = self.command().filter(|_| self.is_enabled()) else {
            return;
        };

        let Some(clipboard) = resolve_clipboard(self.clipboard(), visual) else {
            return;
        };

        // A failure of the clipboard gives no text.
        let text = clipboard.try_get_text_async().await.ok().flatten();

        let resolved_parameter = self.resolve_parameter(text.map(|text| Rc::new(text) as BoxedValue).as_ref());

        if !command.can_execute(resolved_parameter.as_ref()) {
            return;
        }

        command.execute(resolved_parameter.as_ref());
    }
}
