//! `UpdateRealizedCount` of `MainWindow.xaml.cs`: the text of the element of class
//! `realized-count` of a page counts the rows of the first TreeDataGrid of the page.

use ferroui_base::Ref;
use ferroui_controls::{Control, TextBlock};
use ferroui_controls_treedatagrid::TreeDataGrid;

/// Shows on the `realized-count` text of `content` how many rows its first TreeDataGrid
/// realized, and how many rows of the presenter are not realized.
pub fn update_realized_count(content: &Ref<Control>) {
    let tree_data_grid = content.get_logical_descendants().find_map(|x| x.cast::<TreeDataGrid>());
    let text_block = content
        .get_logical_descendants()
        .filter_map(|x| x.cast::<TextBlock>())
        .find(|x| x.classes().contains("realized-count"));
    let rows = tree_data_grid.and_then(|tree_data_grid| tree_data_grid.rows_presenter());
    let (Some(rows), Some(text_block)) = (rows, text_block) else {
        return;
    };

    let realized_row_count = rows.get_realized_elements().len();
    let unrealized_row_count = rows.get_visual_children().len() - realized_row_count;
    text_block.set_text(Some(&format!("{realized_row_count} rows realized ({unrealized_row_count} unrealized)")));
}
