//! A control that lets the user navigate through a paginated collection
//! using a set of pips.

#[allow(clippy::module_inception)]
mod pips_pager;
mod pips_pager_selected_index_changed_event_args;
mod pips_pager_template_settings;

pub use pips_pager::PipsPager;
pub use pips_pager_selected_index_changed_event_args::PipsPagerSelectedIndexChangedEventArgs;
pub use pips_pager_template_settings::PipsPagerTemplateSettings;

#[cfg(test)]
mod pips_pager_tests;
