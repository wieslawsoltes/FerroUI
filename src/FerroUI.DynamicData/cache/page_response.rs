//! `IPageResponse` and `PageResponse` (`Cache/IPageResponse.cs`, `Cache/PageResponse.cs`).

use std::fmt;

/// The page that a paged change set holds (`IPageResponse`).
pub trait IPageResponse {
    /// The page, from 1 (`Page`).
    fn page(&self) -> usize;
    /// The number of pages (`Pages`).
    fn pages(&self) -> usize;
    /// The number of items of a page (`PageSize`).
    fn page_size(&self) -> usize;
    /// The number of items of all the pages (`TotalSize`).
    fn total_size(&self) -> usize;
}

/// The page that a paged change set holds (`PageResponse`). Two responses are equal when their
/// parts are (`PageResponse.DefaultComparer`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PageResponse {
    page_size: usize,
    total_size: usize,
    page: usize,
    pages: usize,
}

impl PageResponse {
    /// The page `page` of `pages`, of `page_size` items, of `total_size` items in all
    /// (`PageResponse(int pageSize, int totalSize, int page, int pages)`).
    pub const fn new(page_size: usize, total_size: usize, page: usize, pages: usize) -> Self {
        Self { page_size, total_size, page, pages }
    }

    /// The page, from 1 (`Page`).
    pub const fn page(&self) -> usize {
        self.page
    }

    /// The number of pages (`Pages`).
    pub const fn pages(&self) -> usize {
        self.pages
    }

    /// The number of items of a page (`PageSize`).
    pub const fn page_size(&self) -> usize {
        self.page_size
    }

    /// The number of items of all the pages (`TotalSize`).
    pub const fn total_size(&self) -> usize {
        self.total_size
    }
}

impl IPageResponse for PageResponse {
    fn page(&self) -> usize {
        self.page
    }

    fn pages(&self) -> usize {
        self.pages
    }

    fn page_size(&self) -> usize {
        self.page_size
    }

    fn total_size(&self) -> usize {
        self.total_size
    }
}

impl fmt::Display for PageResponse {
    /// `ToString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page: {}, PageSize: {}, Pages: {}, TotalSize: {}", self.page, self.page_size, self.pages, self.total_size)
    }
}
