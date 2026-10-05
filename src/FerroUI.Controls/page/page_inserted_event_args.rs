use super::Page;
use ferroui_base::Ref;

/// Provides data for the `PageInserted` event of a navigation page.
#[derive(Clone, PartialEq)]
pub struct PageInsertedEventArgs {
    page: Ref<Page>,
    before: Ref<Page>,
}

impl PageInsertedEventArgs {
    /// Creates the args for `page`, inserted before `before`.
    pub fn new(page: Ref<Page>, before: Ref<Page>) -> Self {
        Self { page, before }
    }

    /// The page that was inserted.
    pub fn page(&self) -> Ref<Page> {
        self.page.clone()
    }

    /// The page before which the new page was inserted.
    pub fn before(&self) -> Ref<Page> {
        self.before.clone()
    }
}
