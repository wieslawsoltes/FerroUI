//! The templates of markup: objects whose content is built on demand.

mod control_template;
mod data_template;
mod focus_adorner_template;
mod items_panel_template;
mod template;
mod template_content;
mod tree_data_template;
mod window_drawn_decorations_template;

pub use control_template::ControlTemplate;
pub use data_template::DataTemplate;
pub use focus_adorner_template::FocusAdornerTemplate;
pub use items_panel_template::ItemsPanelTemplate;
pub use template::Template;
pub use template_content::TemplateContent;
pub use tree_data_template::TreeDataTemplate;
pub use window_drawn_decorations_template::WindowDrawnDecorationsTemplate;

#[cfg(test)]
mod templates_tests;
