//! `IPageRequest` and `PageRequest` (`Cache/IPageRequest.cs`, `Cache/PageRequest.cs`).

use std::fmt;

/// A request of a page of sorted items (`IPageRequest`).
pub trait IPageRequest {
    /// The page, from 1 (`Page`).
    fn page(&self) -> usize;
    /// The number of items of a page (`Size`).
    fn size(&self) -> usize;
}

/// A request of a page of sorted items (`PageRequest`): the page, from 1, and its size.
///
/// Deviation (DEVIATIONS.md, DynamicData): the page and the size are `usize`, so upstream's
/// `ArgumentException` for a negative one cannot arise. Two requests are equal when their parts
/// are (`PageRequest.DefaultComparer`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PageRequest {
    page: usize,
    size: usize,
}

impl PageRequest {
    /// The first page of 25 items (`PageRequest.Default`, `new PageRequest()`).
    pub const DEFAULT: Self = Self { page: 1, size: 25 };

    /// The request of nothing (`PageRequest.Empty`).
    pub const EMPTY: Self = Self { page: 0, size: 0 };

    /// The page `page` of `size` items (`PageRequest(int page, int size)`).
    pub const fn new(page: usize, size: usize) -> Self {
        Self { page, size }
    }

    /// The page, from 1 (`Page`).
    pub const fn page(&self) -> usize {
        self.page
    }

    /// The number of items of a page (`Size`).
    pub const fn size(&self) -> usize {
        self.size
    }
}

impl Default for PageRequest {
    /// The first page of 25 items (`new PageRequest()`).
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl IPageRequest for PageRequest {
    fn page(&self) -> usize {
        self.page
    }

    fn size(&self) -> usize {
        self.size
    }
}

impl fmt::Display for PageRequest {
    /// `ToString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page: {}, Size: {}", self.page, self.size)
    }
}
