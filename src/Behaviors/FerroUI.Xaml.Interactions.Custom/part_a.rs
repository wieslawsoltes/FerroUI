//! Part A of the crate: the folders of the reference listed below, one module per folder.
//!
//! - `Core/` ([`core`](self::core)): the base classes of the behaviors and triggers of the
//!   crate ([`AttachedToVisualTreeBehavior`], [`AttachedToVisualTreeTriggerBase`],
//!   [`RoutedEventTriggerBase`], [`RoutedEventTriggerBaseOfT`], ..) and the triggers of the
//!   lifecycle of their host;
//! - `ExecuteCommand/` ([`execute_command`](self::execute_command)): the behaviors that execute a
//!   command on an event, with their base classes (`ExecuteCommand/Core/`);
//! - `Actions/` ([`actions`](self::actions)): actions on controls, classes, flyouts, popups,
//!   focus, properties, methods and the launcher;
//! - `Logic/` ([`logic`](self::logic)): the conditional and switch actions and triggers.
//!
//! The part owns its classes ([`TYPES`]), their public Rust paths and its markup types
//! ([`register`]); `lib.rs` re-exports everything public of it.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{StaticType, TypeInfo};

#[path = "core/mod.rs"]
mod core;
#[path = "execute_command/mod.rs"]
mod execute_command;
#[path = "actions/mod.rs"]
mod actions;
#[path = "logic/mod.rs"]
mod logic;
// `Focus/FocusNavigationHelper.cs`, an internal class of the folder `Focus/` of another part, which
// `FocusNextElementAction` and its tests use.
#[path = "focus/focus_navigation_helper.rs"]
mod focus_navigation_helper;

pub use self::actions::*;
pub use self::core::*;
pub use self::execute_command::*;
pub use self::logic::*;
pub(crate) use self::focus_navigation_helper::FocusNavigationHelper;

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

/// The classes of this part, registered by `register_types`.
pub(crate) const TYPES: &[&TypeInfo] = types![
    // Core/
    crate::ActualThemeVariantChangedBehavior,
    crate::ActualThemeVariantChangedTrigger,
    crate::AsyncLoadBehavior,
    crate::AttachedToLogicalTreeBehavior,
    crate::AttachedToLogicalTreeTrigger,
    crate::AttachedToLogicalTreeTriggerBase,
    crate::AttachedToVisualTreeBehavior,
    crate::AttachedToVisualTreeTrigger,
    crate::AttachedToVisualTreeTriggerBase,
    crate::BindingBehavior,
    crate::BindingTriggerBehavior,
    crate::DataContextChangedBehavior,
    crate::DataContextChangedTrigger,
    crate::DelayedLoadTrigger,
    crate::DetachedFromLogicalTreeTrigger,
    crate::DetachedFromVisualTreeTrigger,
    crate::IfElseTrigger,
    crate::InitializedBehavior,
    crate::InitializedTrigger,
    crate::LaunchUriOrFileAction,
    crate::LoadedBehavior,
    crate::LoadedTrigger,
    crate::ObservableTriggerBehavior,
    crate::PropertyChangedTrigger,
    crate::ResourcesChangedBehavior,
    crate::ResourcesChangedTrigger,
    crate::RoutedEventTrigger,
    crate::RoutedEventTriggerBase,
    crate::RoutedEventTriggerBaseOfT,
    crate::RoutedEventTriggerBehavior,
    crate::SizeChangedTrigger,
    crate::UnloadedTrigger,
    crate::ValueChangedTriggerBehavior,
    // ExecuteCommand/Core/
    crate::ExecuteCommandBehaviorBase,
    crate::ExecuteCommandOnKeyBehaviorBase,
    crate::ExecuteCommandRoutedEventBehaviorBase,
    // ExecuteCommand/
    crate::ExecuteCommandOnActivatedBehavior,
    crate::ExecuteCommandOnDoubleTappedBehavior,
    crate::ExecuteCommandOnGotFocusBehavior,
    crate::ExecuteCommandOnHoldingBehavior,
    crate::ExecuteCommandOnKeyDownBehavior,
    crate::ExecuteCommandOnKeyUpBehavior,
    crate::ExecuteCommandOnLostFocusBehavior,
    crate::ExecuteCommandOnPinchBehavior,
    crate::ExecuteCommandOnPinchEndedBehavior,
    crate::ExecuteCommandOnPointerCaptureLostBehavior,
    crate::ExecuteCommandOnPointerEnteredBehavior,
    crate::ExecuteCommandOnPointerExitedBehavior,
    crate::ExecuteCommandOnPointerMovedBehavior,
    crate::ExecuteCommandOnPointerPressedBehavior,
    crate::ExecuteCommandOnPointerReleasedBehavior,
    crate::ExecuteCommandOnPointerTouchPadGestureMagnifyBehavior,
    crate::ExecuteCommandOnPointerTouchPadGestureRotateBehavior,
    crate::ExecuteCommandOnPointerTouchPadGestureSwipeBehavior,
    crate::ExecuteCommandOnPointerWheelChangedBehavior,
    crate::ExecuteCommandOnPullGestureBehavior,
    crate::ExecuteCommandOnPullGestureEndedBehavior,
    crate::ExecuteCommandOnRightTappedBehavior,
    crate::ExecuteCommandOnScrollGestureBehavior,
    crate::ExecuteCommandOnScrollGestureEndedBehavior,
    crate::ExecuteCommandOnScrollGestureInertiaStartingBehavior,
    crate::ExecuteCommandOnTappedBehavior,
    crate::ExecuteCommandOnTextInputBehavior,
    crate::ExecuteCommandOnTextInputMethodClientRequestedBehavior,
    // Actions/
    crate::AddClassAction,
    crate::CallMethodAsyncAction,
    crate::ChangeFerroPropertyAction,
    crate::DelayedShowControlAction,
    crate::FocusControlAction,
    crate::FocusNextElementAction,
    crate::HideControlAction,
    crate::HideFlyoutAction,
    crate::HidePopupAction,
    crate::LaunchUriAction,
    crate::PopupAction,
    crate::RemoveClassAction,
    crate::RemoveElementAction,
    crate::ScreenshotAction,
    crate::ScrollToControlAction,
    crate::SetEnabledAction,
    crate::SetThemeVariantAction,
    crate::ShowContextMenuAction,
    crate::ShowControlAction,
    crate::ShowFlyoutAction,
    crate::ShowPopupAction,
    crate::ToggleClassAction,
    // Logic/
    crate::Case,
    crate::ConditionalAction,
    crate::ConditionalBehavior,
    crate::SwitchCaseAction,
    crate::SwitchCaseBehavior,
];

/// The public Rust paths of the classes of this part, by the shortest path another crate names
/// them by; the emitter of Rust source of the markup compiler reads them.
mod rust_paths {
    ferroui_base::ferro_rust_paths! {
        classes: [
            // Core/
            crate::ActualThemeVariantChangedBehavior,
            crate::ActualThemeVariantChangedTrigger,
            crate::AsyncLoadBehavior,
            crate::AttachedToLogicalTreeBehavior,
            crate::AttachedToLogicalTreeTrigger,
            crate::AttachedToLogicalTreeTriggerBase,
            crate::AttachedToVisualTreeBehavior,
            crate::AttachedToVisualTreeTrigger,
            crate::AttachedToVisualTreeTriggerBase,
            crate::BindingBehavior,
            crate::BindingTriggerBehavior,
            crate::DataContextChangedBehavior,
            crate::DataContextChangedTrigger,
            crate::DelayedLoadTrigger,
            crate::DetachedFromLogicalTreeTrigger,
            crate::DetachedFromVisualTreeTrigger,
            crate::IfElseTrigger,
            crate::InitializedBehavior,
            crate::InitializedTrigger,
            crate::LaunchUriOrFileAction,
            crate::LoadedBehavior,
            crate::LoadedTrigger,
            crate::ObservableTriggerBehavior,
            crate::PropertyChangedTrigger,
            crate::ResourcesChangedBehavior,
            crate::ResourcesChangedTrigger,
            crate::RoutedEventTrigger,
            crate::RoutedEventTriggerBase,
            crate::RoutedEventTriggerBaseOfT,
            crate::RoutedEventTriggerBehavior,
            crate::SizeChangedTrigger,
            crate::UnloadedTrigger,
            crate::ValueChangedTriggerBehavior,
            // ExecuteCommand/Core/
            crate::ExecuteCommandBehaviorBase,
            crate::ExecuteCommandOnKeyBehaviorBase,
            crate::ExecuteCommandRoutedEventBehaviorBase,
            // ExecuteCommand/
            crate::ExecuteCommandOnActivatedBehavior,
            crate::ExecuteCommandOnDoubleTappedBehavior,
            crate::ExecuteCommandOnGotFocusBehavior,
            crate::ExecuteCommandOnHoldingBehavior,
            crate::ExecuteCommandOnKeyDownBehavior,
            crate::ExecuteCommandOnKeyUpBehavior,
            crate::ExecuteCommandOnLostFocusBehavior,
            crate::ExecuteCommandOnPinchBehavior,
            crate::ExecuteCommandOnPinchEndedBehavior,
            crate::ExecuteCommandOnPointerCaptureLostBehavior,
            crate::ExecuteCommandOnPointerEnteredBehavior,
            crate::ExecuteCommandOnPointerExitedBehavior,
            crate::ExecuteCommandOnPointerMovedBehavior,
            crate::ExecuteCommandOnPointerPressedBehavior,
            crate::ExecuteCommandOnPointerReleasedBehavior,
            crate::ExecuteCommandOnPointerTouchPadGestureMagnifyBehavior,
            crate::ExecuteCommandOnPointerTouchPadGestureRotateBehavior,
            crate::ExecuteCommandOnPointerTouchPadGestureSwipeBehavior,
            crate::ExecuteCommandOnPointerWheelChangedBehavior,
            crate::ExecuteCommandOnPullGestureBehavior,
            crate::ExecuteCommandOnPullGestureEndedBehavior,
            crate::ExecuteCommandOnRightTappedBehavior,
            crate::ExecuteCommandOnScrollGestureBehavior,
            crate::ExecuteCommandOnScrollGestureEndedBehavior,
            crate::ExecuteCommandOnScrollGestureInertiaStartingBehavior,
            crate::ExecuteCommandOnTappedBehavior,
            crate::ExecuteCommandOnTextInputBehavior,
            crate::ExecuteCommandOnTextInputMethodClientRequestedBehavior,
            // Actions/
            crate::AddClassAction,
            crate::CallMethodAsyncAction,
            crate::ChangeFerroPropertyAction,
            crate::DelayedShowControlAction,
            crate::FocusControlAction,
            crate::FocusNextElementAction,
            crate::HideControlAction,
            crate::HideFlyoutAction,
            crate::HidePopupAction,
            crate::LaunchUriAction,
            crate::PopupAction,
            crate::RemoveClassAction,
            crate::RemoveElementAction,
            crate::ScreenshotAction,
            crate::ScrollToControlAction,
            crate::SetEnabledAction,
            crate::SetThemeVariantAction,
            crate::ShowContextMenuAction,
            crate::ShowControlAction,
            crate::ShowFlyoutAction,
            crate::ShowPopupAction,
            crate::ToggleClassAction,
            // Logic/
            crate::Case,
            crate::ConditionalAction,
            crate::ConditionalBehavior,
            crate::SwitchCaseAction,
            crate::SwitchCaseBehavior,
        ],
        types: [
            crate::CaseCollection,
        ],
        contracts: [
        ],
        generics: [
        ],
        generic_contracts: [
        ],
    }
}

/// Registers the Rust paths (`ferroui_base::ferro_rust_paths!`) and the markup types
/// (`MarkupType::register_all`) of this part. Called once by `register_types`.
pub(crate) fn register() {
    rust_paths::register_rust_paths();
    MarkupType::register_all(MARKUP_TYPES);
    ValueTypes::register_global(register_value_types);
}

/// The types of this part with markup metadata that are not classes: the list of the cases of
/// the switches.
const MARKUP_TYPES: &[&MarkupType] = &[<crate::CaseCollection as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this part: the nullable form
/// of the list of cases.
fn register_value_types() {
    ValueTypes::register_nullable::<crate::CaseCollection>();
}
