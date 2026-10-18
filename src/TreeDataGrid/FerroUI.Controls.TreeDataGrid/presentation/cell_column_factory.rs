//! `Presentation/CellColumnFactory.cs`: the view column of a core column.
//!
//! The reference visits the typed column (`IColumnVisitor<TModel, ICellColumn<TModel>>`); the
//! port's presentation is untyped, so the factory reads what the column is through the core's
//! erased access (`IColumn::view`, `untyped_expander`, `untyped_value`).

use super::{CoreCheckBoxColumn, CoreTextColumn, ExpanderColumnPresentation, TreeDataGridPresentationOptions};
use crate::models::{view_column_of, ColumnRef};
use ferroui_treedatagrid_core::models::IColumn as CoreColumn;
use std::rc::Rc;

/// The presentation key of the core's check box columns.
const CHECK_BOX_KEY: &str = "CheckBox";

/// Creates the view columns of the columns of a presentation (`CellColumnFactory<TModel>`).
pub struct CellColumnFactory {
    options: Rc<TreeDataGridPresentationOptions>,
}

impl CellColumnFactory {
    pub fn new(options: Rc<TreeDataGridPresentationOptions>) -> Self {
        Self { options }
    }

    /// The options whose factories the factory uses.
    pub fn options(&self) -> &Rc<TreeDataGridPresentationOptions> {
        &self.options
    }

    /// The view column of `column`; an error (the reference's `InvalidOperationException`) when
    /// the column has a presentation key with no registered presentation.
    pub fn create(&self, column: &Rc<dyn CoreColumn>) -> Result<ColumnRef, String> {
        // A column of the control crate presents itself (the reference's `LegacySourcePresentation`
        // of the classic columns).
        if let Some(view) = view_column_of(&**column) {
            return Ok(view);
        }

        // `Visit(HierarchicalExpanderColumn<TModel>)`.
        if let Some(expander) = column.untyped_expander() {
            let inner = self.create(&expander.inner())?;
            return Ok(ExpanderColumnPresentation::new(column.clone(), inner) as ColumnRef);
        }

        // `Visit<TValue>(ValueColumn<TModel, TValue>)`.
        let key = column.presentation_key();
        if let Some(factory) = key.as_deref().and_then(|key| self.options.column(key)) {
            return Ok(factory(column));
        }
        if column.untyped_value().is_some() {
            if key.as_deref() == Some(CHECK_BOX_KEY) && !column.untyped_value().is_some_and(|value| value.is_template()) {
                return Ok(CoreCheckBoxColumn::new(column.clone()) as ColumnRef);
            }
            if let Some(missing) = key {
                return Err(format!("No TreeDataGrid column presentation is registered for '{missing}'."));
            }
            return Ok(CoreTextColumn::new(column.clone()) as ColumnRef);
        }
        match key {
            Some(missing) => Err(format!("No TreeDataGrid column presentation is registered for '{missing}'.")),
            None => Err(format!("The column '{}' cannot be presented: it is neither a value nor an expander column.", column.id())),
        }
    }
}
