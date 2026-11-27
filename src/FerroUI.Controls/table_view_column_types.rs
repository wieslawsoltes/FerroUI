//! The column types of WinUI's tabular TableView (`TableViewTextColumn`, `TableViewTemplateColumn`),
//! WinUI-named columns of FerroUI's [`TableView`](crate::TableView): a column that presents the
//! value of its `Binding` as text, and a column that presents each item with its `CellTemplate`
//! (`docs/porting/winui-controls.md`, section 10.11). Both are [`TableViewColumn`]s; the upstream
//! column, which does either, is unchanged.

use crate::TableViewColumn;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl,
};

/// A column that presents the value its `Binding` selects from each item (WinUI's
/// `TableViewTextColumn`). The cells of a text column are the ones in-place editing changes.
#[repr(C)]
pub struct TableViewTextColumn {
    base: TableViewColumn,
}

ferro_class!(TableViewTextColumn: TableViewColumn);
ferro_class_info!(TableViewTextColumn { new: TableViewTextColumn::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(TableViewTextColumn: FerroObjectImpl, StyledElementImpl);

impl TableViewTextColumn {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TableViewColumn::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// A column that presents each item with its `CellTemplate` (WinUI's `TableViewTemplateColumn`).
/// It sorts by the property at `SortMemberPath`.
#[repr(C)]
pub struct TableViewTemplateColumn {
    base: TableViewColumn,
}

ferro_class!(TableViewTemplateColumn: TableViewColumn);
ferro_class_info!(TableViewTemplateColumn { new: TableViewTemplateColumn::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(TableViewTemplateColumn: FerroObjectImpl, StyledElementImpl);

impl TableViewTemplateColumn {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TableViewColumn::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
