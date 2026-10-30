//! Port of `Clipboard/ClearClipboardAction.cs`.

use super::{resolve_clipboard, visual_of};
use crate::action_task::ActionTask;
use crate::ClipboardRef;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that will clear the clipboard.
#[repr(C)]
pub struct ClearClipboardAction {
    base: StyledElementAction,
}

ferro_class!(ClearClipboardAction: StyledElementAction);
ferro_class_info!(ClearClipboardAction { new: ClearClipboardAction::new });
ferroui_base::ferro_impl_classes!(ClearClipboardAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ClearClipboardAction {
        /// Identifies the `Clipboard` property.
        pub fn clipboard_property() -> StyledProperty<Option<ClipboardRef>> {
            FerroProperty::register::<ClearClipboardAction, _>("Clipboard", None)
        }
    }
}

impl StyledElementActionImpl for ClearClipboardAction {
    /// Executes the action: clears the clipboard in a job of the dispatcher.
    ///
    /// Returns true if the clearing is started (the sender is a visual); else false.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = visual_of(sender) else {
            return Some(Rc::new(false));
        };

        let action: Ref<ClearClipboardAction> = this.to_ref();
        drop(ActionTask::spawn(async move { action.clear_clipboard_async(&visual).await }));

        Some(Rc::new(true))
    }
}

impl ClearClipboardAction {
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

    async fn clear_clipboard_async(&self, visual: &Visual) {
        if !self.is_enabled() {
            return;
        }

        let Some(clipboard) = resolve_clipboard(self.clipboard(), visual) else {
            return;
        };

        // A failure of the clipboard is ignored.
        let _ = clipboard.clear_async().await;
    }
}
