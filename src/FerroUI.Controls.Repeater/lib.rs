//! ferroui-controls-repeater
//!
//! A port of the Repeater of WinUI on FerroUI controls: [`ItemsRepeater`] and
//! [`ItemsRepeaterScrollHost`], [`LayoutPanel`], the stack, uniform grid, flow and lined-flow
//! layouts with their virtualizing and non-virtualizing contexts, element factories and
//! [`RecyclePool`], phased template binding, item collection transitions, the selection model
//! of the repeater (`selection`) and the automation peer. [`register_types`] registers the
//! types of the crate for markup, in the XML namespace of the framework.
//!
//! The port, its sources, tests and deviations are described in
//! `docs/porting/winui-repeater/` in the repository.

// The markup macros and the registered Rust paths name the types of the crate by its name.
extern crate self as ferroui_controls_repeater;

pub(crate) mod animation_manager;
mod build_tree_scheduler;
mod diagnostics;
mod element_factory;
mod item_collection_transition;
mod item_collection_transition_completed_event_args;
mod item_collection_transition_progress;
mod item_collection_transition_provider;
mod item_template_wrapper;
mod items_repeater;
mod items_repeater_event_args;
mod items_repeater_scroll_host;
mod layout_panel;
pub mod layouts;
pub(crate) mod markup;
mod lined_flow_layout_item_collection_transition_provider;
mod phaser;
mod recycle_pool;
mod recycling_element_factory;
mod repeater_automation_peer;
mod repeater_layout_context;
// The test hooks are public only with the feature `testing`; the repeater notifies them always.
#[cfg_attr(not(any(test, feature = "testing")), allow(dead_code))]
mod repeater_test_hooks;
mod register_types;
mod rust_paths;
pub mod selection;
pub(crate) mod services_markup;
mod transition_manager;
mod unique_id_element_pool;
mod view_manager;
mod viewport_manager;
mod virtualization_info;

pub use build_tree_scheduler::BuildTreeScheduler;
pub use element_factory::*;
pub use item_collection_transition::*;
pub use item_collection_transition_completed_event_args::*;
pub use item_collection_transition_progress::*;
pub use item_collection_transition_provider::*;
pub use item_template_wrapper::*;
pub use items_repeater::ItemsRepeater;
pub use items_repeater_event_args::*;
pub use items_repeater_scroll_host::ItemsRepeaterScrollHost;
pub use layout_panel::LayoutPanel;
pub use layouts::*;
pub use lined_flow_layout_item_collection_transition_provider::*;
pub use phaser::*;
pub use recycle_pool::*;
pub use recycling_element_factory::*;
pub use repeater_automation_peer::RepeaterAutomationPeer;
#[cfg(any(test, feature = "testing"))]
pub use repeater_test_hooks::RepeaterTestHooks;
#[cfg(not(any(test, feature = "testing")))]
pub(crate) use repeater_test_hooks::RepeaterTestHooks;
pub use register_types::register_types;
pub use services_markup::ElementFactoryTemplatesMetadata;
pub use transition_manager::TransitionManager;
pub use virtualization_info::*;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod xaml_tests;

#[cfg(test)]
mod services_tests;

#[cfg(test)]
mod diagnostics_tests;

#[cfg(test)]
mod items_repeater_tests;

#[cfg(test)]
mod conformance_tests;

#[cfg(test)]
mod acceptance_tests;

#[cfg(test)]
mod view_manager_acceptance_tests;

#[cfg(test)]
mod viewport_tests;

#[cfg(test)]
mod source_view_tests;

#[cfg(test)]
mod layout_contract_tests;

#[cfg(test)]
mod layout_panel_tests;
