//! The auto-complete box and the types of its events and filters.

#[allow(clippy::module_inception)]
mod auto_complete_box;
mod auto_complete_filter_mode;
mod populated_event_args;
mod populating_event_args;

pub use auto_complete_box::{
    AutoCompleteAsyncPopulator, AutoCompleteBox, AutoCompleteBoxImpl, AutoCompleteBoxImplExt, AutoCompleteBoxVTable,
    AutoCompleteFilterPredicate, AutoCompletePopulation, AutoCompleteSelector,
};
pub use auto_complete_filter_mode::AutoCompleteFilterMode;
pub use populated_event_args::PopulatedEventArgs;
pub use populating_event_args::PopulatingEventArgs;

#[cfg(test)]
mod auto_complete_box_tests;
