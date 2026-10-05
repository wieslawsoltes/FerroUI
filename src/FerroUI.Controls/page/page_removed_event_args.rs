use super::Page;
use ferroui_base::Ref;

/// Provides data for the `PageRemoved` event of a navigation page.
#[derive(Clone, PartialEq)]
pub struct PageRemovedEventArgs {
    page: Ref<Page>,
}

impl PageRemovedEventArgs {
    /// Creates the args for the page that was removed.
    pub fn new(page: Ref<Page>) -> Self {
        Self { page }
    }

    /// The page that was removed.
    pub fn page(&self) -> Ref<Page> {
        self.page.clone()
    }
}
