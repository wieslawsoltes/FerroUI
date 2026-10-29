//! The triggers of the input events of an input element and the actions on its pointer
//! (`InputElement/`), with the markup metadata of their enumerations.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_enum;
use ferroui_base::input::PointerEventArgs;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::BoxedValue;
use std::rc::Rc;

mod actions;
mod triggers;

pub use self::actions::{CapturePointerAction, ReleasePointerCaptureAction};
pub use self::triggers::{
    ClickEventTrigger, DoubleTappedTrigger, FiredOn, GotFocusTrigger, HoldingTrigger, KeyDownTrigger,
    KeyGestureTrigger, KeyGestureTriggerFiredOn, KeyTrigger, KeyUpTrigger, LostFocusTrigger, PointerCaptureLostTrigger,
    PointerEnteredTrigger, PointerExitedTrigger, PointerMovedTrigger, PointerPressedTrigger, PointerReleasedTrigger,
    PointerWheelChangedTrigger, TappedTrigger, TextInputMethodClientRequestedTrigger, TextInputTrigger,
};

ferro_markup_enum!(KeyGestureTriggerFiredOn { KeyDown, KeyUp });
ferro_markup_enum!(FiredOn { KeyDown, KeyUp });

/// The enumerations of the folder, declared for markup.
pub(super) const INPUT_ELEMENT_MARKUP_TYPES: &[&MarkupType] =
    &[<KeyGestureTriggerFiredOn as MarkupTyped>::MARKUP, <FiredOn as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of the folder: the nullable forms
/// of its enumerations.
pub(super) fn register_input_element_value_types() {
    ValueTypes::register_nullable::<KeyGestureTriggerFiredOn>();
    ValueTypes::register_nullable::<FiredOn>();
}

/// The pointer event args an untyped parameter holds (`parameter is PointerEventArgs e`): a shared
/// handle to live args of a pointer event (or of an event whose args derive from them), or the
/// args themselves.
pub(crate) fn pointer_event_args_of(value: Option<&BoxedValue>) -> Option<PointerEventArgs> {
    let value = value?;
    if let Some(args) = value.downcast_ref::<Rc<dyn IRoutedEventArgs>>() {
        return args.downcast_ref::<PointerEventArgs>().cloned();
    }
    value.downcast_ref::<PointerEventArgs>().cloned()
}
