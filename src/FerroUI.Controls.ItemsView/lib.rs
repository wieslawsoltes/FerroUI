//! ferroui-controls-items-view
//!
//! `ItemsView` and `ItemContainer` of WinUI (`controls/dev/ItemsView`, `controls/dev/ItemContainer`
//! of microsoft-ui-xaml) on the repeater of FerroUI, with their visual states and their own
//! control themes. The themes are not part of the base themes: an application that uses the
//! controls includes them, and one that does not links none of this crate:
//!
//! ```xml
//! <StyleInclude Source="ferres://FerroUI.Controls.ItemsView/Themes/WinUI/WinUI.xaml" />
//! ```

// The compiled markup names the types of the crate by the name of the crate.
extern crate self as ferroui_controls_items_view;

// `compiled_xaml` (the compiled markup of the theme documents) and `compiled_markup` (the loader
// table of the crate and `register()`), which the build script generates.
ferroui_markup_xaml::include_compiled_xaml!();

#[cfg(any(test, feature = "document-assets"))]
mod assets;
mod item_container;
#[allow(clippy::module_inception)]
mod items_view;
mod markup_types;
mod register_types;
mod rust_paths;
mod selector;

pub use item_container::{ItemContainer, ItemContainerInteractionTrigger};
pub use items_view::{
    ItemsView, ItemsViewItemInvokedEventArgs, ItemsViewSelectionChangedEventArgs, ItemsViewSelectionMode,
};
pub use markup_types::ItemsViewThemeKeys;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
