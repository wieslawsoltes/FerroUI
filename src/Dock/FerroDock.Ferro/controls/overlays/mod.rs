// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source overlay layers, providers and visual composition host.
mod control_list;
mod i_overlay_content_host;
mod i_overlay_layer;
mod overlay_host;
mod overlay_layer;
mod overlay_layer_collection;
mod overlay_layer_factory;
mod overlay_layer_registry;
pub use control_list::OverlayControlList;
pub use i_overlay_layer::IOverlayLayer;
pub use overlay_host::OverlayHost;
pub use overlay_layer::OverlayLayer;
pub use overlay_layer_collection::{
    OrderedOverlayLayerIterator, OrderedOverlayLayers, OverlayLayerCollection,
};
pub use overlay_layer_factory::{IOverlayLayerFactory, OverlayLayerCreation, OverlayLayerFactory};
pub use overlay_layer_registry::{
    OverlayFactoryProvider, OverlayFactorySource, OverlayLayerProvider, OverlayLayerRegistry,
    OverlayLayerSource,
};

/// Source theme keys for the default overlay layers.
pub struct OverlayLayerStyleKeys;
impl OverlayLayerStyleKeys {
    /// Theme key for the busy overlay layer.
    pub const BUSY_LAYER_THEME_KEY: &'static str = "DockOverlayBusyLayerTheme";
    /// Theme key for the dialog overlay layer.
    pub const DIALOG_LAYER_THEME_KEY: &'static str = "DockOverlayDialogLayerTheme";
    /// Theme key for the confirmation overlay layer.
    pub const CONFIRMATION_LAYER_THEME_KEY: &'static str = "DockOverlayConfirmationLayerTheme";
}

#[cfg(test)]
mod tests;

pub use overlay_layer_registry::{
    OverlayFactoryEnumeration, OverlayFactoryEnumerator, OverlayLayerEnumeration,
    OverlayLayerEnumerator,
};

ferroui_base::ferro_static_type!(OverlayLayerStyleKeys);
ferroui_base::ferro_markup_type!(static OverlayLayerStyleKeys {
    type_info:OverlayLayerStyleKeys,
    fields:[
        BusyLayerThemeKey:String=>||OverlayLayerStyleKeys::BUSY_LAYER_THEME_KEY.to_owned(),
        DialogLayerThemeKey:String=>||OverlayLayerStyleKeys::DIALOG_LAYER_THEME_KEY.to_owned(),
        ConfirmationLayerThemeKey:String=>||OverlayLayerStyleKeys::CONFIRMATION_LAYER_THEME_KEY.to_owned()
    ]
});

mod busy_overlay_control;
mod confirmation_overlay_control;
mod dialog_overlay_control;
mod service_attachment;
pub use busy_overlay_control::BusyOverlayControl;
pub use confirmation_overlay_control::ConfirmationOverlayControl;
pub use dialog_overlay_control::DialogOverlayControl;

#[cfg(test)]
mod service_control_tests;

mod dialog_shell_control;
pub use dialog_shell_control::DialogShellControl;

mod confirmation_dialog_control;
pub use confirmation_dialog_control::ConfirmationDialogControl;
