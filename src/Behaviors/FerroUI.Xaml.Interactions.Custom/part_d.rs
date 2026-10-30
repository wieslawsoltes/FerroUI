//! Part D of the crate: the folders of the reference listed below, one module per folder.
//!
//! - `FileUpload/` ([`file_upload`](self::file_upload)): uploading a file to a URL;
//! - `Debugging/` ([`debugging`](self::debugging)): breaking, logging and outlining elements;
//! - `Input/` ([`input`](self::input)): hot keys, inactivity, focus and selection on input;
//! - `TreeView/` and `TreeViewItem/` ([`tree_view`](self::tree_view),
//!   [`tree_view_item`](self::tree_view_item)): filtering a tree view, expanding its items;
//! - `Automation/` ([`automation`](self::automation)): automation names and identifiers;
//! - `Show/` ([`show`](self::show)): showing a control on an input event;
//! - `TabControl/`, `Carousel/`, `AutoCompleteBox/`, `ListBox/`, `ListBoxItem/`, `TextBox/`,
//!   `SelectingItemsControl/`, `Popup/` and `Icon/`: behaviors, triggers and actions of these
//!   controls;
//! - `Composition/` ([`composition`](self::composition)): tilt, orbit and parallax effects;
//! - `Screen/`, `Dialog/`, `System/`, `ThemeVariant/` and `Multimedia/`: services of the top
//!   level and of the platform;
//! - `Layout/` ([`layout`](self::layout)): fluid moves and moving elements between panels;
//! - `Behaviors/` ([`behaviors`](self::behaviors)): scrolling to the end of an items control;
//! - `Properties/` ([`properties`](self::properties)): the animation types the crate forwards.
//!
//! The converters (`Converters/`) are the public module `converters`, as their namespace is
//! `FerroUI.Xaml.Interactions.Custom.Converters`.
//!
//! The part owns its classes ([`TYPES`]), their public Rust paths and its markup types
//! ([`register`]); `lib.rs` re-exports everything public of it.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{StaticType, TypeInfo};

#[path = "auto_complete_box/mod.rs"]
mod auto_complete_box;
#[path = "automation/mod.rs"]
mod automation;
#[path = "behaviors/mod.rs"]
mod behaviors;
#[path = "carousel/mod.rs"]
mod carousel;
#[path = "composition/mod.rs"]
mod composition;
#[path = "debugging/mod.rs"]
mod debugging;
#[path = "dialog/mod.rs"]
mod dialog;
#[path = "file_upload/mod.rs"]
mod file_upload;
#[path = "icon/mod.rs"]
mod icon;
#[path = "input/mod.rs"]
mod input;
#[path = "layout/mod.rs"]
mod layout;
#[path = "list_box/mod.rs"]
mod list_box;
#[path = "list_box_item/mod.rs"]
mod list_box_item;
#[path = "multimedia/mod.rs"]
mod multimedia;
#[path = "popup/mod.rs"]
mod popup;
#[path = "properties/mod.rs"]
mod properties;
#[path = "screen/mod.rs"]
mod screen;
#[path = "selecting_items_control/mod.rs"]
mod selecting_items_control;
#[path = "show/mod.rs"]
mod show;
#[path = "system/mod.rs"]
mod system;
#[path = "tab_control/mod.rs"]
mod tab_control;
#[path = "text_box/mod.rs"]
mod text_box;
#[path = "theme_variant/mod.rs"]
mod theme_variant;
#[path = "tree_view/mod.rs"]
mod tree_view;
#[path = "tree_view_item/mod.rs"]
mod tree_view_item;

pub use self::auto_complete_box::*;
pub use self::automation::*;
pub use self::behaviors::*;
pub use self::carousel::*;
pub use self::composition::*;
pub use self::debugging::*;
pub use self::dialog::*;
pub use self::file_upload::*;
pub use self::icon::*;
pub use self::input::*;
pub use self::layout::*;
pub use self::list_box::*;
pub use self::list_box_item::*;
pub use self::multimedia::*;
pub use self::popup::*;
pub use self::properties::*;
pub use self::screen::*;
pub use self::selecting_items_control::*;
pub use self::show::*;
pub use self::system::*;
pub use self::tab_control::*;
pub use self::text_box::*;
pub use self::theme_variant::*;
pub use self::tree_view::*;
pub use self::tree_view_item::*;

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

/// The classes of this part, registered by `register_types`.
pub(crate) const TYPES: &[&TypeInfo] = types![
    // AutoCompleteBox/
    crate::AutoCompleteBoxOpenDropDownOnFocusBehavior,
    crate::AutoCompleteBoxSelectionChangedTrigger,
    crate::ClearAutoCompleteBoxSelectionAction,
    crate::FocusAutoCompleteBoxTextBoxBehavior,
    // Automation/
    crate::ScreenReaderAnnounceAction,
    crate::SetAutomationIdAction,
    crate::AutomationNameBehavior,
    crate::AutomationNameChangedTrigger,
    // Behaviors/
    crate::AutoScrollToBottomBehavior,
    // Carousel/
    crate::CarouselKeyNavigationBehavior,
    crate::CarouselNextAction,
    crate::CarouselPreviousAction,
    crate::CarouselSelectionChangedTrigger,
    // Composition/
    crate::OrbitEffectBehavior,
    crate::ParallaxBehavior,
    crate::TiltEffectBehavior,
    // Debugging/
    crate::BreakAction,
    crate::LogAction,
    crate::VisualDebugBehavior,
    // Dialog/
    crate::DialogClosedTrigger,
    crate::DialogOpenedTrigger,
    crate::ShowDialogAction,
    // FileUpload/
    crate::ButtonUploadFileBehavior,
    crate::UploadCompletedTrigger,
    crate::UploadFileAction,
    crate::UploadFileBehaviorBase,
    // Icon/
    crate::PathIconDataBehavior,
    crate::PathIconDataChangedTrigger,
    crate::SetPathIconDataAction,
    // Input/
    crate::GlobalHotkeyBehavior,
    crate::InactivityTrigger,
    crate::LoseFocusOnEnterBehavior,
    crate::SelectAllOnFocusBehavior,
    // Layout/
    crate::FluidMoveBehavior,
    crate::MoveElementToPanelAction,
    // ListBox/
    crate::ListBoxSelectAllBehavior,
    crate::ListBoxUnselectAllBehavior,
    crate::RemoveItemInListBoxAction,
    // ListBoxItem/
    crate::SelectListBoxItemOnPointerMovedBehavior,
    // Multimedia/
    crate::ConsoleBeepAction,
    // Popup/
    crate::PopupClosedTrigger,
    crate::PopupOpenedTrigger,
    // Screen/
    crate::ActiveScreenBehavior,
    crate::RequestScreenDetailsAction,
    crate::ScreensChangedTrigger,
    // SelectingItemsControl/
    crate::SelectingItemsControlEventsBehavior,
    crate::SelectingItemsControlSearchBehavior,
    // Show/
    crate::ShowBehaviorBase,
    crate::ShowOnDoubleTappedBehavior,
    crate::ShowOnKeyDownBehavior,
    crate::ShowOnTappedBehavior,
    // System/
    crate::ClipboardMonitorBehavior,
    crate::NetworkStatusTrigger,
    // TabControl/
    crate::TabControlKeyNavigationBehavior,
    crate::TabControlNextAction,
    crate::TabControlPreviousAction,
    crate::TabControlSelectionChangedTrigger,
    // TextBox/
    crate::AutoSelectBehavior,
    crate::TextBoxSelectAllOnGotFocusBehavior,
    crate::TextBoxSelectAllTextBehavior,
    // ThemeVariant/
    crate::ThemeVariantBehavior,
    crate::ThemeVariantTrigger,
    // TreeView/
    crate::ApplyTreeViewFilterAction,
    crate::TreeViewFilterBehavior,
    crate::TreeViewFilterTextChangedTrigger,
    // TreeViewItem/
    crate::ToggleIsExpandedOnDoubleTappedBehavior,
];

/// The markup types of this part that are not classes: the enumerations.
const MARKUP_TYPES: &[&MarkupType] = &[
    <crate::FluidMoveScope as MarkupTyped>::MARKUP,
    <crate::LogActionLevel as MarkupTyped>::MARKUP,
    <crate::NetworkStatus as MarkupTyped>::MARKUP,
    <crate::SortDirection as MarkupTyped>::MARKUP,
];

/// The public Rust paths of the classes of this part, by the shortest path another crate names
/// them by; the emitter of Rust source of the markup compiler reads them.
mod rust_paths {
    ferroui_base::ferro_rust_paths! {
        classes: [
            // AutoCompleteBox/
            crate::AutoCompleteBoxOpenDropDownOnFocusBehavior,
            crate::AutoCompleteBoxSelectionChangedTrigger,
            crate::ClearAutoCompleteBoxSelectionAction,
            crate::FocusAutoCompleteBoxTextBoxBehavior,
            // Automation/
            crate::ScreenReaderAnnounceAction,
            crate::SetAutomationIdAction,
            crate::AutomationNameBehavior,
            crate::AutomationNameChangedTrigger,
            // Behaviors/
            crate::AutoScrollToBottomBehavior,
            // Carousel/
            crate::CarouselKeyNavigationBehavior,
            crate::CarouselNextAction,
            crate::CarouselPreviousAction,
            crate::CarouselSelectionChangedTrigger,
            // Composition/
            crate::OrbitEffectBehavior,
            crate::ParallaxBehavior,
            crate::TiltEffectBehavior,
            // Debugging/
            crate::BreakAction,
            crate::LogAction,
            crate::VisualDebugBehavior,
            // Dialog/
            crate::DialogClosedTrigger,
            crate::DialogOpenedTrigger,
            crate::ShowDialogAction,
            // FileUpload/
            crate::ButtonUploadFileBehavior,
            crate::UploadCompletedTrigger,
            crate::UploadFileAction,
            crate::UploadFileBehaviorBase,
            // Icon/
            crate::PathIconDataBehavior,
            crate::PathIconDataChangedTrigger,
            crate::SetPathIconDataAction,
            // Input/
            crate::GlobalHotkeyBehavior,
            crate::InactivityTrigger,
            crate::LoseFocusOnEnterBehavior,
            crate::SelectAllOnFocusBehavior,
            // Layout/
            crate::FluidMoveBehavior,
            crate::MoveElementToPanelAction,
            // ListBox/
            crate::ListBoxSelectAllBehavior,
            crate::ListBoxUnselectAllBehavior,
            crate::RemoveItemInListBoxAction,
            // ListBoxItem/
            crate::SelectListBoxItemOnPointerMovedBehavior,
            // Multimedia/
            crate::ConsoleBeepAction,
            // Popup/
            crate::PopupClosedTrigger,
            crate::PopupOpenedTrigger,
            // Screen/
            crate::ActiveScreenBehavior,
            crate::RequestScreenDetailsAction,
            crate::ScreensChangedTrigger,
            // SelectingItemsControl/
            crate::SelectingItemsControlEventsBehavior,
            crate::SelectingItemsControlSearchBehavior,
            // Show/
            crate::ShowBehaviorBase,
            crate::ShowOnDoubleTappedBehavior,
            crate::ShowOnKeyDownBehavior,
            crate::ShowOnTappedBehavior,
            // System/
            crate::ClipboardMonitorBehavior,
            crate::NetworkStatusTrigger,
            // TabControl/
            crate::TabControlKeyNavigationBehavior,
            crate::TabControlNextAction,
            crate::TabControlPreviousAction,
            crate::TabControlSelectionChangedTrigger,
            // TextBox/
            crate::AutoSelectBehavior,
            crate::TextBoxSelectAllOnGotFocusBehavior,
            crate::TextBoxSelectAllTextBehavior,
            // ThemeVariant/
            crate::ThemeVariantBehavior,
            crate::ThemeVariantTrigger,
            // TreeView/
            crate::ApplyTreeViewFilterAction,
            crate::TreeViewFilterBehavior,
            crate::TreeViewFilterTextChangedTrigger,
            // TreeViewItem/
            crate::ToggleIsExpandedOnDoubleTappedBehavior,
        ],
        types: [
            crate::FluidMoveScope,
            crate::LogActionLevel,
            crate::NetworkStatus,
            crate::SortDirection,
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

/// What the untyped value conversions must know about the types of this part: the nullable forms
/// of the enumerations and of the handles the properties hold.
fn register_value_types() {
    ValueTypes::register_nullable::<crate::FluidMoveScope>();
    ValueTypes::register_nullable::<crate::LogActionLevel>();
    ValueTypes::register_nullable::<crate::NetworkStatus>();
    ValueTypes::register_nullable::<crate::SortDirection>();
    ValueTypes::register_nullable::<crate::ScreensRef>();
}
