//! Port of `Clipboard/SetClipboardDataObjectAction.cs`.

use super::{resolve_clipboard, visual_of};
use crate::action_task::ActionTask;
use crate::{AsyncDataTransferRef, ClipboardRef};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that will set the data object to the clipboard.
#[repr(C)]
pub struct SetClipboardDataObjectAction {
    base: StyledElementAction,
}

ferro_class!(SetClipboardDataObjectAction: StyledElementAction);
ferro_class_info!(SetClipboardDataObjectAction { new: SetClipboardDataObjectAction::new });
ferroui_base::ferro_impl_classes!(SetClipboardDataObjectAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetClipboardDataObjectAction {
        /// Identifies the `Clipboard` property.
        pub fn clipboard_property() -> StyledProperty<Option<ClipboardRef>> {
            FerroProperty::register::<SetClipboardDataObjectAction, _>("Clipboard", None)
        }

        /// Identifies the `DataTransfer` property.
        pub fn data_transfer_property() -> StyledProperty<Option<AsyncDataTransferRef>> {
            FerroProperty::register::<SetClipboardDataObjectAction, _>("DataTransfer", None)
        }
    }
}

impl StyledElementActionImpl for SetClipboardDataObjectAction {
    /// Executes the action: places the data transfer on the clipboard in a job of the dispatcher.
    ///
    /// Returns true if the operation is started (the sender is a visual); else false.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = visual_of(sender) else {
            return Some(Rc::new(false));
        };

        let action: Ref<SetClipboardDataObjectAction> = this.to_ref();
        drop(ActionTask::spawn(async move { action.set_clipboard_data_object_async(&visual).await }));

        Some(Rc::new(true))
    }
}

impl SetClipboardDataObjectAction {
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

    /// Gets or sets the data transfer content to place on the clipboard.
    pub fn data_transfer(&self) -> Option<AsyncDataTransferRef> {
        self.get_value(Self::data_transfer_property())
    }

    pub fn set_data_transfer(&self, value: Option<AsyncDataTransferRef>) {
        self.set_value(Self::data_transfer_property(), value)
    }

    async fn set_clipboard_data_object_async(&self, visual: &Visual) {
        let Some(data_transfer) = self.data_transfer().filter(|_| self.is_enabled()) else {
            return;
        };

        let Some(clipboard) = resolve_clipboard(self.clipboard(), visual) else {
            return;
        };

        // A failure of the clipboard is ignored.
        let _ = clipboard.set_data_async(Some(data_transfer.0.clone())).await;
    }
}
