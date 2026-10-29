//! Port of `TreeView/TreeViewFilterHelper.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_controls::{TreeView, TreeViewItem};

/// Filters the items of a [`TreeView`] by their header (an internal class of the reference).
pub(crate) struct TreeViewFilterHelper;

impl TreeViewFilterHelper {
    /// Shows the items whose header contains `query` (ignoring case), the items with such a
    /// descendant (expanded) and every item for an empty query; hides the others. Returns the
    /// number of top-level items shown.
    pub(crate) fn filter(tree_view: &TreeView, query: &str) -> i32 {
        let query = query.to_lowercase();
        let mut count = 0;

        for item in tree_view.get_logical_children().iter().filter_map(|child| child.cast::<TreeViewItem>()) {
            if Self::filter_item(&item, &query) {
                count += 1;
            }
        }

        count
    }

    fn filter_item(item: &TreeViewItem, query: &str) -> bool {
        let is_match = item
            .header()
            .map(|header| ValueTypes::to_display_string(Some(&header)).to_lowercase().contains(query))
            .unwrap_or(false);
        let mut visible_children = false;

        for child in item.get_logical_children().iter().filter_map(|child| child.cast::<TreeViewItem>()) {
            if Self::filter_item(&child, query) {
                visible_children = true;
            }
        }

        let visible = query.is_empty() || is_match || visible_children;
        item.set_is_visible(visible);
        item.set_is_expanded(visible_children && !query.is_empty());

        visible
    }
}
