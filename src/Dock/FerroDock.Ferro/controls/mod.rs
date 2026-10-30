// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native docking controls and overlay composition.
pub mod overlays;

mod root_dock_control;
pub use root_dock_control::RootDockControl;

mod dock_target_base;
pub use dock_target_base::{
    DockTargetBase, DockTargetBaseImpl, DockTargetBaseImplExt, DockTargetOperations,
};
mod dock_target;
pub use dock_target::DockTarget;
mod global_dock_target;
pub use global_dock_target::GlobalDockTarget;
mod i_dock_target;
pub use i_dock_target::IDockTarget;
mod i_dock_adorner_host_provider;
pub use i_dock_adorner_host_provider::IDockAdornerHostProvider;

mod dock_adorner_window;
pub use dock_adorner_window::DockAdornerWindow;
mod i_dock_adorner_window_factory;
pub use i_dock_adorner_window_factory::IDockAdornerWindowFactory;

mod mdi_layout_panel;
pub use mdi_layout_panel::MdiLayoutPanel;

mod managed_dockable_base;
#[doc(hidden)]
pub use managed_dockable_base::ManagedDockableBaseCallbacks;
pub use managed_dockable_base::{
    ManagedDockableBase, ManagedDockableBaseImpl, ManagedDockableBaseReference,
};

mod document_dock_control;
pub use document_dock_control::DocumentDockControl;

mod dock_dock_control;
pub use dock_dock_control::DockDockControl;

mod stack_dock_control;
pub use stack_dock_control::StackDockControl;

mod wrap_dock_control;
pub use wrap_dock_control::WrapDockControl;

mod uniform_grid_dock_control;
pub use uniform_grid_dock_control::UniformGridDockControl;

mod proportional_dock_control;
pub use proportional_dock_control::ProportionalDockControl;

mod document_content_control;
pub use document_content_control::DocumentContentControl;

mod tool_content_control;
pub use tool_content_control::ToolContentControl;

mod generated_theme;

mod split_view_dock_control;
pub use split_view_dock_control::SplitViewDockControl;
mod grid_dock_control;
pub use grid_dock_control::GridDockControl;

mod tool_dock_control;
pub use tool_dock_control::ToolDockControl;

mod drag_preview_control;
pub use drag_preview_control::DragPreviewControl;
mod drag_preview_window;
pub use drag_preview_window::DragPreviewWindow;
mod dock_selector_overlay;
pub use dock_selector_overlay::DockSelectorOverlay;
mod dock_command_bar_host;
pub use dock_command_bar_host::DockCommandBarHost;

mod pinned_dock_window;
pub use pinned_dock_window::PinnedDockWindow;

mod managed_dock_window_document;
pub use managed_dock_window_document::{ManagedDockWindowDocument, ManagedTemplateContentFactory};

mod managed_window_dock;
pub use managed_window_dock::ManagedWindowDock;

mod managed_model_registration;
pub(crate) use managed_model_registration::register as register_managed_models;

#[doc(hidden)]
pub use ferrodock_model_ferro::IMdiDocumentMetadata;

mod mdi_document_window;
pub use mdi_document_window::MdiDocumentWindow;

mod mdi_document_control;
pub use mdi_document_control::MdiDocumentControl;

mod managed_window_layer;
pub use managed_window_layer::ManagedWindowLayer;

mod dock_control;
pub use dock_control::{DockControl, HostWindowFactoryValue};

mod host_window;
pub use host_window::HostWindow;

mod managed_host_window;
pub use managed_host_window::ManagedHostWindow;

mod tool_chrome_control;
pub use tool_chrome_control::ToolChromeControl;

mod tool_pin_item_control;
pub use tool_pin_item_control::ToolPinItemControl;

mod tool_pinned_control;
pub use tool_pinned_control::ToolPinnedControl;

mod document_tab_strip;
pub use document_tab_strip::DocumentTabStrip;

mod document_tab_strip_item;
pub use document_tab_strip_item::DocumentTabStripItem;

mod tool_tab_strip;
pub use tool_tab_strip::ToolTabStrip;

mod tool_tab_strip_item;
pub use tool_tab_strip_item::ToolTabStripItem;

mod dockable_control;
pub use dockable_control::DockableControl;

mod document_control;
pub use document_control::DocumentControl;

mod tool_control;
pub use tool_control::ToolControl;

mod pinned_dock_control;
pub use pinned_dock_control::PinnedDockControl;

mod pinned_dock_host_panel;
pub use pinned_dock_host_panel::PinnedDockHostPanel;

mod host_window_title_bar;
pub use host_window_title_bar::HostWindowTitleBar;

mod delayed_uniform_tab_panel;
pub use delayed_uniform_tab_panel::DelayedUniformTabPanel;
