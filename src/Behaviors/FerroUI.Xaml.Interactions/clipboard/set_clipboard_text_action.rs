//! Port of `Clipboard/SetClipboardTextAction.cs`.

use super::{resolve_clipboard, visual_of};
use crate::action_task::ActionTask;
use crate::ClipboardRef;
use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that will set the text to the clipboard.
#[repr(C)]
pub struct SetClipboardTextAction {
    base: StyledElementAction,
}

ferro_class!(SetClipboardTextAction: StyledElementAction);
ferro_class_info!(SetClipboardTextAction { new: SetClipboardTextAction::new });
ferroui_base::ferro_impl_classes!(SetClipboardTextAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetClipboardTextAction {
        /// Identifies the `Clipboard` property.
        pub fn clipboard_property() -> StyledProperty<Option<ClipboardRef>> {
            FerroProperty::register::<SetClipboardTextAction, _>("Clipboard", None)
        }

        /// Identifies the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SetClipboardTextAction, _>("Text", None)
        }
    }
}

impl StyledElementActionImpl for SetClipboardTextAction {
    /// Executes the action: places the text on the clipboard in a job of the dispatcher.
    ///
    /// Returns true if the operation is started (the sender is a visual); else false.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = visual_of(sender) else {
            return Some(Rc::new(false));
        };

        let action: Ref<SetClipboardTextAction> = this.to_ref();
        drop(ActionTask::spawn(async move { action.set_clipboard_text_async(&visual).await }));

        Some(Rc::new(true))
    }
}

impl SetClipboardTextAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
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

    /// Gets or sets the text to set to the clipboard. This is a property of the object model.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<String>) {
        self.set_value(Self::text_property(), value)
    }

    async fn set_clipboard_text_async(&self, visual: &Visual) {
        let Some(text) = self.text().filter(|_| self.is_enabled()) else {
            return;
        };

        let Some(clipboard) = resolve_clipboard(self.clipboard(), visual) else {
            return;
        };

        // A failure of the clipboard is ignored.
        let _ = clipboard.set_text_async(Some(&text)).await;
    }
}
