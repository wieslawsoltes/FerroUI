//! Typed column declarations for a TreeDataGrid that displays its ItemsSource.
mod bindings;

use ferroui_controls::{templates::FuncDataTemplate, GridLength};
use ferroui_controls_treedatagrid::{
    TreeDataGridCheckBoxColumn, TreeDataGridColumn, TreeDataGridColumns,
    TreeDataGridHierarchicalExpanderColumn, TreeDataGridRowHeaderColumn,
    TreeDataGridTemplateColumn, TreeDataGridTextColumn,
};
use std::rc::Rc;

/// Builds one native column. Call [`build`](Self::build) to collect columns of
/// different kinds in [`tree_data_grid_columns`].
#[must_use]
pub struct TreeDataGridColumnBuilder<C>(Rc<C>);

/// Collect columns for the `column_definitions` property.
pub fn tree_data_grid_columns(
    columns: impl IntoIterator<Item = Rc<dyn TreeDataGridColumn>>,
) -> TreeDataGridColumns {
    TreeDataGridColumns::from_columns(columns)
}

/// Start a text column that displays the row value by default.
pub fn tree_data_grid_text_column() -> TreeDataGridColumnBuilder<TreeDataGridTextColumn> {
    TreeDataGridColumnBuilder(TreeDataGridTextColumn::new()).binding_path("")
}

/// Start a checkbox column bound to the row's `IsChecked` property by default.
pub fn tree_data_grid_check_box_column() -> TreeDataGridColumnBuilder<TreeDataGridCheckBoxColumn> {
    TreeDataGridColumnBuilder(TreeDataGridCheckBoxColumn::new()).binding_path("IsChecked")
}

/// Start a template column with the framework's default text template.
pub fn tree_data_grid_template_column() -> TreeDataGridColumnBuilder<TreeDataGridTemplateColumn> {
    TreeDataGridColumnBuilder(TreeDataGridTemplateColumn::new())
        .cell_template(FuncDataTemplate::default_template())
}

/// Start a column showing the row numbers.
pub fn tree_data_grid_row_header_column() -> TreeDataGridColumnBuilder<TreeDataGridRowHeaderColumn>
{
    TreeDataGridColumnBuilder(TreeDataGridRowHeaderColumn::new())
        .width(GridLength::from_pixels(44.0))
}

/// Wrap a text, checkbox, or template column in an expander that reads `Children` by default.
pub fn tree_data_grid_expander_column(
    inner: Rc<dyn TreeDataGridColumn>,
) -> TreeDataGridColumnBuilder<TreeDataGridHierarchicalExpanderColumn> {
    let column = TreeDataGridHierarchicalExpanderColumn::new();
    column.set_inner_column(Some(inner));
    TreeDataGridColumnBuilder(column).children_path("Children")
}

impl<C: TreeDataGridColumn + 'static> TreeDataGridColumnBuilder<C> {
    pub fn header(self, text: impl AsRef<str>) -> Self {
        self.0.base().set_header_text(text.as_ref());
        self
    }

    pub fn width(self, width: GridLength) -> Self {
        self.0.base().set_width(width);
        self
    }

    pub fn min_width(self, width: GridLength) -> Self {
        self.0.base().set_min_width(width);
        self
    }

    pub fn max_width(self, width: GridLength) -> Self {
        self.0.base().set_max_width(Some(width));
        self
    }

    pub fn can_user_resize(self, resize: bool) -> Self {
        self.0.base().set_can_user_resize(Some(resize));
        self
    }

    pub fn can_user_sort_column(self, sort: bool) -> Self {
        self.0.base().set_can_user_sort_column(Some(sort));
        self
    }

    pub fn allow_tri_state_sorting(self, enabled: bool) -> Self {
        self.0.base().set_allow_tri_state_sorting(enabled);
        self
    }

    /// Finish the column for a collection or the inner column of an expander.
    pub fn build(self) -> Rc<dyn TreeDataGridColumn> {
        self.0
    }
}
