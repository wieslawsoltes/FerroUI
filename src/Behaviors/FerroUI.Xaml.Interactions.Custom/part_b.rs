//! Part B of the crate: the folders of the reference listed below, one module per folder.
//!
//! - `Collections/` ([`collections`](self::collections)): the actions on a collection and the
//!   behavior and trigger of its changes;
//! - `Control/` ([`control`](self::control)): behaviors of a control (pointer over, bounds,
//!   delayed load, dragging, hiding, inline editing, ..);
//! - `Focus/` ([`focus`](self::focus)): the behaviors that move the keyboard focus, and the focus
//!   trap;
//! - `Gestures/` ([`gestures`](self::gestures)): the triggers of the gestures;
//! - `InputElement/` ([`input_element`](self::input_element)): the triggers of the input events
//!   (with the click trigger) and the actions on the pointer;
//! - `ItemsControl/` ([`items_control`](self::items_control)): the actions on the items of an
//!   items control, the triggers of its containers and the behaviors that scroll to an item;
//! - `Validation/` ([`validation`](self::validation)): the validation behaviors and their rules.
//!
//! The part owns its classes ([`TYPES`]), their public Rust paths and its markup types
//! ([`register`]); `lib.rs` re-exports everything public of it.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupType;
use ferroui_base::{StaticType, TypeInfo};

#[path = "collections/mod.rs"]
mod collections;
#[path = "control/mod.rs"]
mod control;
#[path = "focus/mod.rs"]
mod focus;
#[path = "gestures/mod.rs"]
mod gestures;
#[path = "input_element/mod.rs"]
mod input_element;
#[path = "items_control/mod.rs"]
mod items_control;
#[path = "validation/mod.rs"]
mod validation;

pub use self::collections::*;
pub use self::control::*;
pub use self::focus::*;
pub use self::gestures::*;
pub use self::input_element::*;
pub use self::items_control::*;
pub use self::validation::*;

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

/// The classes of this part, registered by `register_types`.
pub(crate) const TYPES: &[&TypeInfo] = types![
    // Collections/
    crate::AddRangeAction,
    crate::ClearCollectionAction,
    crate::CollectionChangedBehavior,
    crate::CollectionChangedTrigger,
    crate::RemoveRangeAction,
    // Control/
    crate::BindPointerOverBehavior,
    crate::BindTagToVisualRootDataContextBehavior,
    crate::BoundsObserverBehavior,
    crate::DelayedLoadBehavior,
    crate::DragControlBehavior,
    crate::HideAttachedFlyoutBehavior,
    crate::HideOnKeyPressedBehavior,
    crate::HideOnLostFocusBehavior,
    crate::InlineEditBehavior,
    crate::ShowPointerPositionBehavior,
    // Focus/
    crate::AutoFocusBehavior,
    crate::FocusBehavior,
    crate::FocusBehaviorBase,
    crate::FocusControlBehavior,
    crate::FocusOnAttachedBehavior,
    crate::FocusOnAttachedToVisualTreeBehavior,
    crate::FocusOnPointerMovedBehavior,
    crate::FocusOnPointerPressedBehavior,
    crate::FocusOnVisibleBehavior,
    crate::FocusSelectedItemBehavior,
    crate::FocusTrapBehavior,
    // Gestures/
    crate::DoubleTappedGestureTrigger,
    crate::HoldingGestureTrigger,
    crate::PinchEndedGestureTrigger,
    crate::PinchGestureTrigger,
    crate::PointerTouchPadGestureMagnifyGestureTrigger,
    crate::PointerTouchPadGestureRotateGestureTrigger,
    crate::PointerTouchPadGestureSwipeGestureTrigger,
    crate::PullGestureEndedGestureTrigger,
    crate::PullGestureGestureTrigger,
    crate::RightTappedGestureTrigger,
    crate::ScrollGestureEndedGestureTrigger,
    crate::ScrollGestureGestureTrigger,
    crate::ScrollGestureInertiaStartingGestureTrigger,
    crate::TappedGestureTrigger,
    // InputElement/
    crate::CapturePointerAction,
    crate::ReleasePointerCaptureAction,
    crate::ClickEventTrigger,
    crate::DoubleTappedTrigger,
    crate::GotFocusTrigger,
    crate::HoldingTrigger,
    crate::KeyDownTrigger,
    crate::KeyGestureTrigger,
    crate::KeyTrigger,
    crate::KeyUpTrigger,
    crate::LostFocusTrigger,
    crate::PointerCaptureLostTrigger,
    crate::PointerEnteredTrigger,
    crate::PointerExitedTrigger,
    crate::PointerMovedTrigger,
    crate::PointerPressedTrigger,
    crate::PointerReleasedTrigger,
    crate::PointerWheelChangedTrigger,
    crate::TappedTrigger,
    crate::TextInputMethodClientRequestedTrigger,
    crate::TextInputTrigger,
    // ItemsControl/
    crate::AddItemToItemsControlAction,
    crate::ClearItemsControlAction,
    crate::InsertItemToItemsControlAction,
    crate::ItemNudgeDropBehavior,
    crate::ItemsControlContainerClearingTrigger,
    crate::ItemsControlContainerEventsBehavior,
    crate::ItemsControlContainerIndexChangedTrigger,
    crate::ItemsControlContainerPreparedTrigger,
    crate::ItemsControlPreparingContainerTrigger,
    crate::MoveItemInItemsControlAction,
    crate::RemoveItemAtAction,
    crate::RemoveItemInItemsControlAction,
    crate::ScrollToItemBehavior,
    crate::ScrollToItemIndexBehavior,
    // Validation/
    crate::ComboBoxValidationBehavior,
    crate::DatePickerValidationBehavior,
    crate::MaxValueValidationRule,
    crate::MinLengthValidationRule,
    crate::MinValueValidationRule,
    crate::NotNullValidationRule,
    crate::NumericUpDownValidationBehavior,
    crate::PropertyValidationBehavior,
    crate::RangeValidationRule,
    crate::RegexValidationRule,
    crate::RequiredDateValidationRule,
    crate::RequiredDecimalValidationRule,
    crate::RequiredTextValidationRule,
    crate::SliderValidationBehavior,
    crate::TextBoxValidationBehavior,
];

/// The public Rust paths of the classes of this part, by the shortest path another crate names
/// them by; the emitter of Rust source of the markup compiler reads them.
mod rust_paths {
    ferroui_base::ferro_rust_paths! {
        classes: [
            // Collections/
            crate::AddRangeAction,
            crate::ClearCollectionAction,
            crate::CollectionChangedBehavior,
            crate::CollectionChangedTrigger,
            crate::RemoveRangeAction,
            // Control/
            crate::BindPointerOverBehavior,
            crate::BindTagToVisualRootDataContextBehavior,
            crate::BoundsObserverBehavior,
            crate::DelayedLoadBehavior,
            crate::DragControlBehavior,
            crate::HideAttachedFlyoutBehavior,
            crate::HideOnKeyPressedBehavior,
            crate::HideOnLostFocusBehavior,
            crate::InlineEditBehavior,
            crate::ShowPointerPositionBehavior,
            // Focus/
            crate::AutoFocusBehavior,
            crate::FocusBehavior,
            crate::FocusBehaviorBase,
            crate::FocusControlBehavior,
            crate::FocusOnAttachedBehavior,
            crate::FocusOnAttachedToVisualTreeBehavior,
            crate::FocusOnPointerMovedBehavior,
            crate::FocusOnPointerPressedBehavior,
            crate::FocusOnVisibleBehavior,
            crate::FocusSelectedItemBehavior,
            crate::FocusTrapBehavior,
            // Gestures/
            crate::DoubleTappedGestureTrigger,
            crate::HoldingGestureTrigger,
            crate::PinchEndedGestureTrigger,
            crate::PinchGestureTrigger,
            crate::PointerTouchPadGestureMagnifyGestureTrigger,
            crate::PointerTouchPadGestureRotateGestureTrigger,
            crate::PointerTouchPadGestureSwipeGestureTrigger,
            crate::PullGestureEndedGestureTrigger,
            crate::PullGestureGestureTrigger,
            crate::RightTappedGestureTrigger,
            crate::ScrollGestureEndedGestureTrigger,
            crate::ScrollGestureGestureTrigger,
            crate::ScrollGestureInertiaStartingGestureTrigger,
            crate::TappedGestureTrigger,
            // InputElement/
            crate::CapturePointerAction,
            crate::ReleasePointerCaptureAction,
            crate::ClickEventTrigger,
            crate::DoubleTappedTrigger,
            crate::GotFocusTrigger,
            crate::HoldingTrigger,
            crate::KeyDownTrigger,
            crate::KeyGestureTrigger,
            crate::KeyTrigger,
            crate::KeyUpTrigger,
            crate::LostFocusTrigger,
            crate::PointerCaptureLostTrigger,
            crate::PointerEnteredTrigger,
            crate::PointerExitedTrigger,
            crate::PointerMovedTrigger,
            crate::PointerPressedTrigger,
            crate::PointerReleasedTrigger,
            crate::PointerWheelChangedTrigger,
            crate::TappedTrigger,
            crate::TextInputMethodClientRequestedTrigger,
            crate::TextInputTrigger,
            // ItemsControl/
            crate::AddItemToItemsControlAction,
            crate::ClearItemsControlAction,
            crate::InsertItemToItemsControlAction,
            crate::ItemNudgeDropBehavior,
            crate::ItemsControlContainerClearingTrigger,
            crate::ItemsControlContainerEventsBehavior,
            crate::ItemsControlContainerIndexChangedTrigger,
            crate::ItemsControlContainerPreparedTrigger,
            crate::ItemsControlPreparingContainerTrigger,
            crate::MoveItemInItemsControlAction,
            crate::RemoveItemAtAction,
            crate::RemoveItemInItemsControlAction,
            crate::ScrollToItemBehavior,
            crate::ScrollToItemIndexBehavior,
            // Validation/
            crate::ComboBoxValidationBehavior,
            crate::DatePickerValidationBehavior,
            crate::MaxValueValidationRule,
            crate::MinLengthValidationRule,
            crate::MinValueValidationRule,
            crate::NotNullValidationRule,
            crate::NumericUpDownValidationBehavior,
            crate::PropertyValidationBehavior,
            crate::RangeValidationRule,
            crate::RegexValidationRule,
            crate::RequiredDateValidationRule,
            crate::RequiredDecimalValidationRule,
            crate::RequiredTextValidationRule,
            crate::SliderValidationBehavior,
            crate::TextBoxValidationBehavior,
        ],
        types: [
            // InputElement/
            crate::FiredOn,
            crate::KeyGestureTriggerFiredOn,
            // Validation/
            crate::ValidationRules,
        ],
        contracts: [
        ],
        generics: [
        ],
        generic_contracts: [
        ],
    }
}

/// The markup types of the folders of this part that are not classes.
const MARKUP_TYPE_LISTS: &[&[&MarkupType]] =
    &[input_element::INPUT_ELEMENT_MARKUP_TYPES, validation::VALIDATION_MARKUP_TYPES];

/// Registers the Rust paths (`ferroui_base::ferro_rust_paths!`) and the markup types
/// (`MarkupType::register_all`) of this part. Called once by `register_types`.
pub(crate) fn register() {
    rust_paths::register_rust_paths();
    for types in MARKUP_TYPE_LISTS {
        MarkupType::register_all(types);
    }
    ValueTypes::register_global(register_value_types);
}

/// What the untyped value conversions must know about the types of this part.
fn register_value_types() {
    input_element::register_input_element_value_types();
    validation::register_validation_value_types();
}
