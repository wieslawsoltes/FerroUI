// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

//! Original command bar composition and native rendering.
mod default_dock_command_bar_adapter;
mod dock_command_bar_manager;
mod i_dock_command_bar_adapter;
pub use default_dock_command_bar_adapter::DefaultDockCommandBarAdapter;
pub use dock_command_bar_manager::DockCommandBarManager;
pub use i_dock_command_bar_adapter::IDockCommandBarAdapter;
mod model_metadata;
#[doc(hidden)]
pub use model_metadata::{
    DockCommandBarDefinitionMetadata, DockCommandBarItemMetadata, IDockCommandBarProviderMetadata,
};
mod register_types;
pub(crate) use register_types::register_types;

#[cfg(test)]
mod adapter_tests;
