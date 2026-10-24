mod folding_manager;
mod folding_events;
mod folding_section;
mod new_folding;
mod xml_folding_strategy;

pub use folding_manager::{FoldingError, FoldingManager};
pub use folding_events::FoldingSubscription;
pub use folding_section::FoldingSection;
pub use new_folding::NewFolding;
pub use xml_folding_strategy::XmlFoldingStrategy;
