//! WinUI's `BreadcrumbBarItemClickedEventArgs` (`controls/dev/Breadcrumb/BreadcrumbBar.idl` and
//! `BreadcrumbBarItemClickedEventArgs.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use ferroui_base::{ferro_markup_type, BoxedValue};

/// The arguments of the `ItemClicked` event of a [`BreadcrumbBar`](super::BreadcrumbBar): the
/// item that was clicked and its index in the items source.
#[derive(Clone, Debug)]
pub struct BreadcrumbBarItemClickedEventArgs {
    index: i32,
    item: Option<BoxedValue>,
}

impl BreadcrumbBarItemClickedEventArgs {
    /// Creates the arguments of a click of `item`, the item at `index` of the items source.
    pub fn new(index: i32, item: Option<BoxedValue>) -> Self {
        Self { index, item }
    }

    /// The index of the clicked item in the items source, from 0.
    pub fn index(&self) -> i32 {
        self.index
    }

    /// The clicked item: the item of the items source (the content of its breadcrumb bar item).
    pub fn item(&self) -> Option<BoxedValue> {
        self.item.clone()
    }
}

/// Two arguments are equal when they have the same index and equal items.
impl PartialEq for BreadcrumbBarItemClickedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && crate::items_source::items_equal(&self.item, &other.item)
    }
}

ferro_markup_type!(class BreadcrumbBarItemClickedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [BreadcrumbBarItemClickedEventArgs],
    properties: [
        Index: i32 { get: BreadcrumbBarItemClickedEventArgs::index },
        Item: Option<BoxedValue> { get: |args: &BreadcrumbBarItemClickedEventArgs| args.item() },
    ],
});
