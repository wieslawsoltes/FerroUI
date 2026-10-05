use super::IApplicationLifetime;
use crate::TopLevel;
use ferroui_base::Ref;

/// Used for the single-view lifetimes of platforms with a single top-level
/// to get the top-level that hosts the main view.
pub trait ISingleTopLevelApplicationLifetime: IApplicationLifetime {
    /// The top-level that hosts the main view of the application.
    fn top_level(&self) -> Option<Ref<TopLevel>>;
}
