//! Search engine exports and native culture service bridge.
//! Native controls are available from crate::controls.
mod culture_bridge;
pub(crate) use culture_bridge::{current_compare_rules, install_culture_provider};

pub use ferroribbon_core::search::{
    RibbonSearchEngine, RibbonSearchEntry, RibbonSearchEntryOptions, RibbonSearchResult,
    RibbonSearchResults, RibbonSearchStrings,
};
