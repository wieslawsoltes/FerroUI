//! The layout system: measure/arrange and the layout manager.

mod bring_into_view_request;
mod effective_viewport_changed_event_args;
mod i_bring_into_view_layout_manager;
mod i_layout_manager;
mod i_layout_root;
mod layout_extensions;
mod layout_helper;
mod layout_information;
mod layout_manager;
mod layout_queue;
mod layoutable;
mod min_max;
mod orientation;

pub use bring_into_view_request::BringIntoViewRequest;
pub use effective_viewport_changed_event_args::EffectiveViewportChangedEventArgs;
pub use i_bring_into_view_layout_manager::IBringIntoViewLayoutManager;
pub use i_layout_manager::ILayoutManager;
pub use i_layout_root::ILayoutRoot;
pub use layout_extensions::LayoutExtensions;
pub use layout_helper::LayoutHelper;
pub use layout_information::LayoutInformation;
pub use layout_manager::LayoutManager;
pub use layoutable::{
    HorizontalAlignment, Layoutable, LayoutableImpl, LayoutableImplExt, LayoutableVTable, VerticalAlignment,
};
pub use min_max::MinMax;
pub use orientation::Orientation;

#[cfg(test)]
mod layout_helper_tests;
#[cfg(test)]
mod layout_manager_tests_bring_into_view;
#[cfg(test)]
mod layout_queue_tests;
