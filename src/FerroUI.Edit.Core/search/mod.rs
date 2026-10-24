mod regex_search_strategy;
mod search_result;
mod search_strategy_factory;

pub use regex_search_strategy::{ISearchStrategy, RegexSearchStrategy, SearchError};
pub use search_result::SearchResult;
pub use search_strategy_factory::{SearchMode, SearchOptions, SearchStrategyFactory};
