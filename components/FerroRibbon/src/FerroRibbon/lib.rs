//! Native FerroUI controls. The complete ribbon remains under implementation.
extern crate self as ferroribbon;
pub mod commands;

pub mod compat;
pub mod metadata;
pub mod primitives;
mod register_types;
#[cfg(test)]
mod state_file_future_tests;
#[cfg(test)]
mod test_process;
pub use register_types::{ASSEMBLY, register_types};
#[cfg(test)]
mod tests;

#[cfg(test)]
mod metrics_tests;

pub mod search;

pub mod localization;

#[cfg(test)]
mod scroll_tests;

pub mod controls;
pub mod theming;
// Selected emitter clones Copy literals and emits identical nullable/non-null
// Theme assignment arms and borrows an already borrowed template context.
#[allow(
    clippy::clone_on_copy,
    clippy::if_same_then_else,
    clippy::needless_borrow
)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_shared};

#[cfg(test)]
mod item_link_tests;

#[cfg(test)]
mod command_base_tests;
#[cfg(test)]
mod font_icon_tests;
#[cfg(test)]
mod icon_tests;
#[cfg(test)]
mod image_icon_tests;
#[cfg(test)]
mod item_content_tests;
#[cfg(test)]
mod shared_theme_tests;
#[cfg(test)]
mod theme_resource_tests;

#[cfg(test)]
mod runtime_theme_tests;

#[cfg(test)]
#[path = "../../tests/fixtures/framework/platform_settings.rs"]
mod platform_settings;

#[cfg(test)]
mod theme_metadata_tests;

#[cfg(test)]
mod theme_event_tests;

#[cfg(test)]
mod item_contract_tests;

#[cfg(test)]
mod group_items_panel_tests;

#[cfg(test)]
mod common_item_tests;

#[cfg(test)]
mod simple_items_tests;

#[cfg(test)]
mod button_tests;
#[cfg(test)]
mod click_delivery_tests;

#[cfg(test)]
mod container_tests;
#[cfg(test)]
mod group_tests;
#[cfg(test)]
mod groups_panel_tests;

#[cfg(test)]
mod ribbon_tests;
#[cfg(test)]
mod root_command_tests;
#[cfg(test)]
mod runtime_behavior_tests;
#[cfg(test)]
mod runtime_command_tests;
#[cfg(test)]
mod runtime_feature_tests;
#[cfg(test)]
mod runtime_item_tests;
#[cfg(test)]
mod runtime_layout_tests;
#[cfg(test)]
mod runtime_mvvm_tests;
#[cfg(test)]
mod runtime_sample;

#[cfg(test)]
mod checked_control_tests;

#[cfg(test)]
mod menu_tests;

#[cfg(test)]
mod backstage_tests;
#[cfg(test)]
mod key_tip_peer_tests;
pub mod mvvm;
#[cfg(test)]
mod root_backstage_tests;
#[cfg(test)]
mod root_model_tests;

#[cfg(test)]
mod application_menu_items_tests;

/// Packaging audit hook; absent for ordinary unstamped development builds.
#[doc(hidden)]
pub const fn build_source_fingerprint() -> Option<&'static str> {
    option_env!("FERRORIBBON_BUILD_FINGERPRINT")
}

#[cfg(test)]
mod dialog_button_tests;

#[cfg(test)]
mod command_enabled_tests;
#[cfg(test)]
mod command_reentrancy_tests;
#[cfg(test)]
mod dialog_lifecycle_tests;

#[cfg(test)]
mod control_base_overflow_tests;

#[cfg(test)]
#[path = "../../tests/fixtures/catalog_item_cases.rs"]
mod catalog_item_cases;
#[cfg(test)]
mod catalog_item_tests;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../tests/FerroRibbon.Markup.Consumer/item_contract_consumer.rs"]
mod item_contract_consumer;

#[cfg(test)]
#[path = "../../tests/fixtures/menu_image_uri_cases.rs"]
mod menu_image_uri_cases;

#[cfg(test)]
mod dialog_future_test_support;

#[cfg(test)]
mod state_entry_tests;

#[cfg(test)]
mod runtime_cad_tests;

#[cfg(test)]
mod icon_source_tests;

#[cfg(test)]
mod bitmap_icon_tests;
#[cfg(test)]
mod symbol_icon_tests;
