//! Templates: the objects that build controls on demand for templated
//! controls and for data.

mod data_template_extensions;
mod data_templates;
mod func_control_template;
mod func_data_template;
mod func_template;
mod func_template_name_scope_extensions;
mod func_tree_data_template;
mod i_control_template;
mod i_data_template;
mod i_data_template_host;
mod i_recycling_data_template;
mod i_template;
mod i_tree_data_template;
mod i_typed_data_template;
mod template_extensions;
mod template_result;

pub use data_templates::DataTemplates;
pub use func_control_template::FuncControlTemplate;
pub use func_data_template::FuncDataTemplate;
pub use func_template::{FuncTemplate, FuncTemplateWithParam};
pub use func_template_name_scope_extensions::FuncTemplateNameScopeExtensions;
pub use func_tree_data_template::FuncTreeDataTemplate;
pub use i_control_template::IControlTemplate;
pub use i_data_template::IDataTemplate;
pub use i_data_template_host::IDataTemplateHost;
pub use i_recycling_data_template::IRecyclingDataTemplate;
pub use i_template::{ITemplateOf, ITemplateWithParam, TemplateRef};
pub use i_tree_data_template::ITreeDataTemplate;
pub use i_typed_data_template::ITypedDataTemplate;
pub use template_result::TemplateResult;

#[cfg(test)]
mod templates_tests;
