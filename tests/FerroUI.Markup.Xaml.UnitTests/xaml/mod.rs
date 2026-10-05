//! Tests of documents loaded at run time (upstream folder `Xaml/`), and the
//! checks of the run-time type system that need the metadata of the
//! controls library.

mod type_system_projection_tests;
mod well_known_types_tests;

pub(crate) mod assign_binding_tests;
mod basic_tests;
pub(crate) mod binding_tests;
pub(crate) mod binding_tests_relative_source;
pub(crate) mod control_binding_tests;
mod control_template_tests;
mod control_theme_tests;
mod data_template_tests;
mod design_mode_tests;
mod event_tests;
mod ferro_intrinsics_tests;
mod generic_template_tests;
mod ignored_directives_tests;
mod items_panel_template_tests;
mod merge_resource_include_tests;
mod parent_stack_provider_tests;
mod provide_value_target_tests;
mod relative_panel_tests;
mod resource_dictionary_tests;
mod shape_tests;
mod style_include_tests;
mod style_tests;
mod theme_dictionaries_tests;
mod tree_data_template_tests;
mod window_tests;
mod x_shared_directive_tests;
mod xaml_il_tests;
mod xaml_source_info_tests;
