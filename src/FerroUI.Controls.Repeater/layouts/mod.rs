//! WinUI layout objects and their host-independent layout contexts.

mod element_manager;
mod flow_layout;
mod flow_layout_algorithm;
mod flow_layout_state;
mod layout;
mod layout_context;
pub(crate) mod layouts_markup;
// The test hooks are public only with the feature `testing`; the layouts notify them always.
#[cfg_attr(not(any(test, feature = "testing")), allow(dead_code))]
mod layouts_test_hooks;
mod lined_flow_layout;
mod lined_flow_layout_algorithm;
mod lined_flow_layout_item_aspect_ratios;
mod lined_flow_layout_items_info_requested_event_args;
mod orientation_based_measures;
mod stack_layout;
mod stack_layout_state;
mod uniform_grid_layout;
mod uniform_grid_layout_state;

pub use element_manager::ElementManager;
pub use flow_layout::{FlowLayout, FlowLayoutImpl, FlowLayoutImplExt, FlowLayoutVTable};
pub use flow_layout_algorithm::{
    FlowLayoutAlgorithm, FlowLayoutAnchorInfo, FlowLayoutLineAlignment, FlowLayoutMeasureOptions,
    FlowLayoutRealizedElement, IFlowLayoutAlgorithmDelegates,
};
pub use flow_layout_state::FlowLayoutState;
pub use layout::{
    IndexBasedLayoutOrientation, Layout, LayoutImpl, LayoutImplExt, LayoutVTable,
    NonVirtualizingLayout, NonVirtualizingLayoutImpl, NonVirtualizingLayoutImplExt,
    NonVirtualizingLayoutVTable, VirtualizingLayout, VirtualizingLayoutImpl,
    VirtualizingLayoutImplExt, VirtualizingLayoutVTable,
};
pub use layout_context::{
    ElementRealizationOptions, LayoutContext, LayoutContextHost, LayoutContextImpl,
    LayoutContextImplExt, LayoutContextVTable, NonVirtualizingLayoutContext,
    NonVirtualizingLayoutContextImpl, NonVirtualizingLayoutContextImplExt,
    NonVirtualizingLayoutContextVTable, VirtualizingLayoutContext, VirtualizingLayoutContextImpl,
    VirtualizingLayoutContextImplExt, VirtualizingLayoutContextVTable,
};
pub use layouts_test_hooks::LinedFlowLayoutInvalidationTrigger;
#[cfg(any(test, feature = "testing"))]
pub use layouts_test_hooks::{
    LayoutsTestHooks, LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs,
    LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs,
};
#[cfg(not(any(test, feature = "testing")))]
pub(crate) use layouts_test_hooks::LayoutsTestHooks;
pub use lined_flow_layout::{
    LinedFlowLayout, LinedFlowLayoutItemsJustification, LinedFlowLayoutItemsStretch,
};
pub use lined_flow_layout_item_aspect_ratios::{ItemAspectRatio, LinedFlowLayoutItemAspectRatios};
pub use lined_flow_layout_items_info_requested_event_args::{
    LinedFlowLayoutError, LinedFlowLayoutItemsInfoRequestedEventArgs,
};
pub use orientation_based_measures::{OrientationBasedMeasures, ScrollOrientation};
pub use stack_layout::StackLayout;
pub use stack_layout_state::StackLayoutState;
pub use uniform_grid_layout::{
    UniformGridLayout, UniformGridLayoutItemsJustification, UniformGridLayoutItemsStretch,
};
pub use uniform_grid_layout_state::UniformGridLayoutState;

#[cfg(test)]
mod layout_collection_tests;
#[cfg(test)]
mod flow_layout_upstream_tests;
#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod lined_flow_layout_tests;
#[cfg(test)]
mod lined_flow_upstream_tests;
