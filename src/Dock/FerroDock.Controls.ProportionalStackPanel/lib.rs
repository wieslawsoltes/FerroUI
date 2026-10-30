//! Native proportional docking layout and splitter controls.
extern crate self as ferrodock_controls_proportional_stack_panel;
mod internal;
mod proportional_stack_panel;
pub use proportional_stack_panel::{
    ProportionalStackPanel, ProportionalStackPanelImpl, ProportionalStackPanelImplExt,
};
mod proportional_stack_panel_splitter;
pub use proportional_stack_panel_splitter::ProportionalStackPanelSplitter;
mod splitter_preview_adorner;

ferroui_markup_xaml::include_compiled_xaml!();
mod register_types;
pub use register_types::{register_types, ASSEMBLY};
#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod items_control_tests;
#[cfg(test)]
mod min_max_tests;
#[cfg(test)]
mod tests;
