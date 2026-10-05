//! Port of `CompilerExtensions/Transformers`. Append one `mod`/`pub use` pair per ported file.

mod add_name_scope_registration;
mod ferro_xaml_il_control_template_target_type_metadata_transformer;
mod ferro_xaml_il_well_known_types;
mod xaml_property_path_exception;

pub use add_name_scope_registration::*;
pub use ferro_xaml_il_control_template_target_type_metadata_transformer::*;
pub use ferro_xaml_il_well_known_types::*;
pub use xaml_property_path_exception::*;

// Objects, properties, resources.
mod ferro_x_aml_il_classes_transformer;
mod ferro_xaml_il_add_source_info_transformer;
mod ferro_xaml_il_classes_property_resolver;
mod ferro_xaml_il_constructor_service_provider_transformer;
mod ferro_xaml_il_design_properties_transformer;
mod ferro_xaml_il_ensure_resource_dictionary_capacity_transformer;
mod ferro_xaml_il_ferro_property_resolver;
mod ferro_xaml_il_metadata_remover;
mod ferro_xaml_il_option_markup_extension_transformer;
mod ferro_xaml_il_reorder_classes_properties_transformer;
mod ferro_xaml_il_resolve_by_name_markup_extension_replacer;
mod ferro_xaml_il_resource_transformer;
mod ferro_xaml_il_root_object_scope_transformer;
mod ferro_xaml_il_theme_variant_provider_transformer;
mod ferro_xaml_il_transform_instance_attached_properties;
mod ferro_xaml_il_transform_routed_event;
mod ferro_xaml_il_transitions_type_metadata_transformer;
mod ignored_directives_transformer;
mod x_name_transformer;

pub use ferro_x_aml_il_classes_transformer::*;
pub use ferro_xaml_il_add_source_info_transformer::*;
pub use ferro_xaml_il_classes_property_resolver::*;
pub use ferro_xaml_il_constructor_service_provider_transformer::*;
pub use ferro_xaml_il_design_properties_transformer::*;
pub use ferro_xaml_il_ensure_resource_dictionary_capacity_transformer::*;
pub use ferro_xaml_il_ferro_property_resolver::*;
pub use ferro_xaml_il_metadata_remover::*;
pub use ferro_xaml_il_option_markup_extension_transformer::*;
pub use ferro_xaml_il_reorder_classes_properties_transformer::*;
pub use ferro_xaml_il_resolve_by_name_markup_extension_replacer::*;
pub use ferro_xaml_il_resource_transformer::*;
pub use ferro_xaml_il_root_object_scope_transformer::*;
pub use ferro_xaml_il_theme_variant_provider_transformer::*;
pub use ferro_xaml_il_transform_instance_attached_properties::*;
pub use ferro_xaml_il_transform_routed_event::*;
pub use ferro_xaml_il_transitions_type_metadata_transformer::*;
pub use ignored_directives_transformer::*;
pub use x_name_transformer::*;

mod ferro_xaml_il_control_template_parts_checker;
mod ferro_xaml_il_control_template_priority_transformer;
mod ferro_xaml_il_control_theme_transformer;
mod ferro_xaml_il_data_template_warnings_transformer;
mod ferro_xaml_il_duplicate_setters_checker;
mod ferro_xaml_il_query_transformer;
mod ferro_xaml_il_selector_transformer;
mod ferro_xaml_il_setter_target_type_metadata_transformer;
mod ferro_xaml_il_setter_transformer;
mod ferro_xaml_il_style_validator_transformer;

pub use ferro_xaml_il_control_template_parts_checker::*;
pub use ferro_xaml_il_control_template_priority_transformer::*;
pub use ferro_xaml_il_control_theme_transformer::*;
pub use ferro_xaml_il_data_template_warnings_transformer::*;
pub use ferro_xaml_il_duplicate_setters_checker::*;
pub use ferro_xaml_il_query_transformer::*;
pub use ferro_xaml_il_selector_transformer::*;
pub use ferro_xaml_il_setter_target_type_metadata_transformer::*;
pub use ferro_xaml_il_setter_transformer::*;
pub use ferro_xaml_il_style_validator_transformer::*;

// Compiled bindings.
mod ferro_binding_extension_transformer;
mod ferro_xaml_il_binding_path_parser;
mod ferro_xaml_il_binding_path_transformer;
mod ferro_xaml_il_compiled_bindings_metadata_remover;
mod ferro_xaml_il_data_context_type_transformer;
mod ferro_xaml_il_transform_synthetic_compiled_binding_members;
mod x_data_type_transformer;

pub use ferro_binding_extension_transformer::*;
pub use ferro_xaml_il_binding_path_parser::*;
pub use ferro_xaml_il_binding_path_transformer::*;
pub use ferro_xaml_il_compiled_bindings_metadata_remover::*;
pub use ferro_xaml_il_data_context_type_transformer::*;
pub use ferro_xaml_il_transform_synthetic_compiled_binding_members::*;
pub use x_data_type_transformer::*;

#[cfg(test)]
mod ferro_xaml_il_selector_transformer_tests;
#[cfg(test)]
mod ferro_xaml_il_query_transformer_tests;
#[cfg(test)]
mod ferro_xaml_il_setter_transformer_tests;

#[cfg(test)]
mod objects_transformers_tests;
#[cfg(test)]
mod ferro_xaml_il_control_template_parts_checker_tests;
#[cfg(test)]
mod ferro_xaml_il_control_template_target_type_metadata_transformer_tests;
#[cfg(test)]
mod ferro_xaml_il_control_theme_transformer_tests;

#[cfg(test)]
mod compiled_bindings_tests;
