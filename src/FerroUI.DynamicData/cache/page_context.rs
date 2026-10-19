//! `PageContext<TObject>` and `VirtualContext<TObject>` (`Cache/PageContext.cs`,
//! `Cache/VirtualContext.cs`): the context of the change sets of `SortAndPage` and
//! `SortAndVirtualize`.

use super::{PageResponse, SortAndPageOptions, SortAndVirtualizeOptions, VirtualResponse};
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

/// The page of a change set of `SortAndPage`, with the comparer and the options
/// (`record PageContext<TObject>`).
pub struct PageContext<T> {
    /// The page (`Response`).
    pub response: PageResponse,
    /// The comparer of the sort (`Comparer`).
    pub comparer: Rc<dyn Fn(&T, &T) -> Ordering>,
    /// The options (`Options`).
    pub options: SortAndPageOptions,
}

impl<T> PageContext<T> {
    /// A context of its parts.
    pub fn new(response: PageResponse, comparer: Rc<dyn Fn(&T, &T) -> Ordering>, options: SortAndPageOptions) -> Self {
        Self { response, comparer, options }
    }

    /// The context before any page (`PageContext<TObject>.Empty`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): its comparer finds every pair equal, as there is no
    /// `Comparer<TObject>.Default` for every item.
    pub fn empty() -> Self
    where
        T: 'static,
    {
        Self::new(PageResponse::new(0, 0, 0, 0), Rc::new(|_: &T, _: &T| Ordering::Equal), SortAndPageOptions::default())
    }
}

impl<T> Clone for PageContext<T> {
    fn clone(&self) -> Self {
        Self { response: self.response, comparer: self.comparer.clone(), options: self.options }
    }
}

impl<T> PartialEq for PageContext<T> {
    /// The equality of a record: the parts, the comparer by reference.
    fn eq(&self, other: &Self) -> bool {
        self.response == other.response && Rc::ptr_eq(&self.comparer, &other.comparer) && self.options == other.options
    }
}

impl<T> fmt::Debug for PageContext<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageContext").field("response", &self.response).field("options", &self.options).finish_non_exhaustive()
    }
}

/// The window of a change set of `SortAndVirtualize`, with the comparer and the options
/// (`record VirtualContext<TObject>`).
pub struct VirtualContext<T> {
    /// The window (`Response`).
    pub response: VirtualResponse,
    /// The comparer of the sort (`Comparer`).
    pub comparer: Rc<dyn Fn(&T, &T) -> Ordering>,
    /// The options (`Options`).
    pub options: SortAndVirtualizeOptions,
}

impl<T> VirtualContext<T> {
    /// A context of its parts.
    pub fn new(response: VirtualResponse, comparer: Rc<dyn Fn(&T, &T) -> Ordering>, options: SortAndVirtualizeOptions) -> Self {
        Self { response, comparer, options }
    }

    /// The context before any window (`VirtualContext<TObject>.Empty`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): its comparer finds every pair equal, as there is no
    /// `Comparer<TObject>.Default` for every item.
    pub fn empty() -> Self
    where
        T: 'static,
    {
        Self::new(VirtualResponse::new(0, 0, 0), Rc::new(|_: &T, _: &T| Ordering::Equal), SortAndVirtualizeOptions::default())
    }
}

impl<T> Clone for VirtualContext<T> {
    fn clone(&self) -> Self {
        Self { response: self.response, comparer: self.comparer.clone(), options: self.options }
    }
}

impl<T> PartialEq for VirtualContext<T> {
    /// The equality of a record: the parts, the comparer by reference.
    fn eq(&self, other: &Self) -> bool {
        self.response == other.response && Rc::ptr_eq(&self.comparer, &other.comparer) && self.options == other.options
    }
}

impl<T> fmt::Debug for VirtualContext<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VirtualContext").field("response", &self.response).field("options", &self.options).finish_non_exhaustive()
    }
}
