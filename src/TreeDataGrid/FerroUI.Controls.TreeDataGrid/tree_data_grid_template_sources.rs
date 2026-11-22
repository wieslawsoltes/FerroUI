//! The declarative owner of each template column created for a generated source.
use crate::models::{HierarchicalExpanderColumn, IColumn, IViewColumn, TemplateColumn};
use crate::{
    TreeDataGrid, TreeDataGridColumn, TreeDataGridHierarchicalExpanderColumn,
    TreeDataGridTemplateColumn,
};
use ferroui_base::BoxedValue;
use ferroui_controls::diagnostics::{ColumnTemplate, ColumnTemplateMember};
use std::{
    any::Any,
    collections::HashSet,
    rc::{Rc, Weak},
};

const MAX_SOURCE_COLUMNS: usize = 8_192;

pub(super) struct GeneratedTemplateColumn {
    index: usize,
    expander_inner: bool,
    declaration: Weak<TreeDataGridTemplateColumn>,
    view: Weak<dyn IViewColumn<BoxedValue>>,
}

impl GeneratedTemplateColumn {
    pub(super) fn new(
        index: usize,
        declaration: &Rc<dyn TreeDataGridColumn>,
        view: &Rc<dyn IViewColumn<BoxedValue>>,
    ) -> Option<Self> {
        let expander = declaration
            .as_any()
            .downcast_ref::<TreeDataGridHierarchicalExpanderColumn>();
        let declaration = match expander {
            Some(expander) => expander.inner_column()?,
            None => declaration.clone(),
        };
        let declaration = (declaration as Rc<dyn Any>)
            .downcast::<TreeDataGridTemplateColumn>()
            .ok()?;
        Some(Self {
            index,
            expander_inner: expander.is_some(),
            declaration: Rc::downgrade(&declaration),
            view: Rc::downgrade(view),
        })
    }

    fn slots(&self) -> Option<[ColumnTemplate; 2]> {
        let declaration = self.declaration.upgrade()?;
        let view = self.view.upgrade()?;
        let view = if self.expander_inner {
            IColumn::as_any(view.as_ref())
                .downcast_ref::<HierarchicalExpanderColumn<BoxedValue>>()?
                .inner()
                .as_ref()
        } else {
            view.as_ref()
        };
        let native = IColumn::as_any(view).downcast_ref::<TemplateColumn<BoxedValue>>()?;
        let [display, editing] = native.known_templates();
        let header = IColumn::header(native);
        let declaration: BoxedValue = declaration;
        Some(
            [
                (ColumnTemplateMember::Display, display),
                (ColumnTemplateMember::Editing, editing),
            ]
            .map(|(member, template)| ColumnTemplate {
                index: self.index,
                expander_inner: self.expander_inner,
                declaration: declaration.clone(),
                header: header.clone(),
                type_name: "TreeDataGridTemplateColumn",
                member,
                template,
            }),
        )
    }
}

pub(super) fn read(grid: &TreeDataGrid, budget: usize) -> Option<Vec<ColumnTemplate>> {
    let origins = grid.generated_template_columns.borrow();
    if origins.is_empty() {
        return Some(Vec::new());
    }
    let Some(source) = grid
        .source()
        .filter(|active| grid.generated_source().as_ref() == Some(active))
    else {
        return Some(Vec::new());
    };
    let count = source.column_count();
    if count > MAX_SOURCE_COLUMNS {
        return None;
    }
    // The active source tracks replacements and moves even while the presentation is detached.
    let active_columns: HashSet<*const ()> = (0..count)
        .map(|index| Rc::as_ptr(&source.column(index)) as *const ())
        .collect();
    let mut templates = Vec::new();
    for origin in origins
        .iter()
        .filter(|origin| active_columns.contains(&(origin.view.as_ptr() as *const ())))
    {
        if let Some(slots) = origin.slots() {
            if slots.len() > budget.saturating_sub(templates.len()) {
                return None;
            }
            templates.extend(slots);
        }
    }
    Some(templates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::FlatTreeDataGridSource, TreeDataGridColumns, TreeDataGridRowHeaderColumn};
    use ferroui_controls::{
        diagnostics::column_templates,
        templates::{FuncDataTemplate, IDataTemplate},
        ItemsSource,
    };

    #[test]
    fn reads_templates_from_the_presented_column_and_keeps_the_authored_index() {
        let grid = TreeDataGrid::new();
        let data = ItemsSource::from_strs(["Northwind"]);
        let column = TreeDataGridTemplateColumn::new();
        let original: Rc<dyn IDataTemplate> = FuncDataTemplate::default_template();
        column.set_cell_template(Some(original.clone()));
        grid.set_column_definitions(TreeDataGridColumns::from_columns([
            TreeDataGridRowHeaderColumn::new() as Rc<dyn TreeDataGridColumn>,
            column.clone(),
        ]));
        grid.set_items_source(Some(data));
        let source = grid
            .generated_source()
            .unwrap()
            .downcast::<FlatTreeDataGridSource<BoxedValue>>()
            .unwrap();
        column.set_cell_template(None);
        source.columns().move_column(1, 0);

        let slots = column_templates(&grid, 2).unwrap();
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].index, 1);
        assert!(Rc::ptr_eq(slots[0].template.as_ref().unwrap(), &original));
        assert!(slots[1].template.is_none());
        assert!(column_templates(&grid, 1).is_none());

        let explicit = FlatTreeDataGridSource::<BoxedValue>::new(source.items());
        grid.set_source(Some(explicit.into()));
        assert!(column_templates(&grid, 2).unwrap().is_empty());
        grid.set_source(None);
        assert_eq!(column_templates(&grid, 2).unwrap().len(), 2);

        source.columns().remove_at(0);
        assert!(column_templates(&grid, 2).unwrap().is_empty());
    }
}
