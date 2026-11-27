//! WinUI's `TabView`, `TabViewItem`, `TabViewListView` and their arguments
//! (`controls/dev/TabView/` of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md). Part of the WinUI Gallery port (#418).

mod tab_view;
mod tab_view_item;
mod tab_view_list_view;

#[cfg(test)]
mod tab_view_tests;

pub use tab_view::{
    TabView, TabViewCloseButtonOverlayMode, TabViewTabCloseRequestedEventArgs, TabViewWidthMode,
};
pub use tab_view_item::{TabViewItem, TabViewItemTemplateSettings};
pub use tab_view_list_view::TabViewListView;
