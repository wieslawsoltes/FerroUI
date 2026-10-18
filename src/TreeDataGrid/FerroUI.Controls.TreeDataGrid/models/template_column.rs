//! `Models/TreeDataGrid/TemplateColumn.cs`: a column whose cells a data template shows, given as
//! a template or as the key of a template resource.

use super::column_base::impl_view_column_base;
use super::view_column::{boxed_model, row_model, unwrap_boxed_model};
use super::{CellRef, CellTemplateGetter, ColumnBase, ICellColumn, ITemplateCellOptions, TemplateCell, TemplateColumnOptions};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::BoxedValue;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{Control, GridLength};
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{ColumnKind, Comparison, IColumnOf, IValueColumn, ListSortDirection, RowRef};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

// A template of the column and the key of the resource it is looked up by.
struct TemplateSlot {
    template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    resource_key: Option<ResourceKey>,
}

impl TemplateSlot {
    // `GetCellTemplate` and `GetCellEditingTemplate`: the template, looked up from the resources
    // of `anchor` the first time.
    fn get(&self, anchor: &Control, header: impl FnOnce() -> String) -> Rc<dyn IDataTemplate> {
        if let Some(template) = self.template.borrow().clone() {
            return template;
        }

        let template = self.resource_key.as_ref().and_then(|key| {
            anchor.try_find_resource(key, None).flatten().and_then(|value| data_template_of(&value))
        });

        match template {
            Some(template) => {
                *self.template.borrow_mut() = Some(template.clone());
                template
            }
            None => {
                let key = self.resource_key.as_ref().map(|key| key.to_string()).unwrap_or_default();
                panic!(
                    "No data template resource with the key of '{key}' could be found for the template column '{}'.",
                    header()
                );
            }
        }
    }
}

// A resource value as a data template (`as IDataTemplate`).
fn data_template_of(value: &BoxedValue) -> Option<Rc<dyn IDataTemplate>> {
    if let Some(template) = value.downcast_ref::<Rc<dyn IDataTemplate>>() {
        return Some(template.clone());
    }
    let converted = ValueTypes::try_cast(value, ValueType::of::<Rc<dyn IDataTemplate>>())?;
    converted.downcast_ref::<Rc<dyn IDataTemplate>>().cloned()
}

/// `TemplateColumn<TModel>`.
pub struct TemplateColumn<M: TreeModel> {
    weak_self: Weak<Self>,
    base: ColumnBase<M>,
    options: Rc<TemplateColumnOptions<M>>,
    cell_template: Rc<TemplateSlot>,
    cell_editing_template: Option<Rc<TemplateSlot>>,
    get_cell_template: CellTemplateGetter,
    get_editing_cell_template: Option<CellTemplateGetter>,
}

impl<M: TreeModel> TemplateColumn<M> {
    /// A column whose cells `cell_template` shows and `cell_editing_template` edits.
    pub fn new(
        header: Option<BoxedValue>,
        cell_template: Rc<dyn IDataTemplate>,
        cell_editing_template: Option<Rc<dyn IDataTemplate>>,
        width: Option<GridLength>,
        options: Option<Rc<TemplateColumnOptions<M>>>,
    ) -> Rc<Self> {
        let cell_template = Rc::new(TemplateSlot { template: RefCell::new(Some(cell_template)), resource_key: None });
        let cell_editing_template = cell_editing_template
            .map(|template| Rc::new(TemplateSlot { template: RefCell::new(Some(template)), resource_key: None }));
        Self::create(header, cell_template, cell_editing_template, width, options)
    }

    /// A column whose cells the data template resource `cell_template_resource_key` shows and the
    /// resource `cell_editing_template_resource_key` edits, looked up from the resources of the
    /// cell the first time a cell is shown.
    pub fn from_resource_keys(
        header: Option<BoxedValue>,
        cell_template_resource_key: ResourceKey,
        cell_editing_template_resource_key: Option<ResourceKey>,
        width: Option<GridLength>,
        options: Option<Rc<TemplateColumnOptions<M>>>,
    ) -> Rc<Self> {
        let cell_template =
            Rc::new(TemplateSlot { template: RefCell::new(None), resource_key: Some(cell_template_resource_key) });
        let cell_editing_template = cell_editing_template_resource_key
            .map(|key| Rc::new(TemplateSlot { template: RefCell::new(None), resource_key: Some(key) }));
        Self::create(header, cell_template, cell_editing_template, width, options)
    }

    fn create(
        header: Option<BoxedValue>,
        cell_template: Rc<TemplateSlot>,
        cell_editing_template: Option<Rc<TemplateSlot>>,
        width: Option<GridLength>,
        options: Option<Rc<TemplateColumnOptions<M>>>,
    ) -> Rc<Self> {
        let options = options.unwrap_or_else(|| Rc::new(TemplateColumnOptions::default()));
        Rc::new_cyclic(|weak_self: &Weak<Self>| {
            let get_cell_template = {
                let slot = cell_template.clone();
                let column = weak_self.clone();
                Rc::new(move |anchor: &Control| slot.get(anchor, || header_text(&column))) as CellTemplateGetter
            };
            let get_editing_cell_template = cell_editing_template.as_ref().map(|slot| {
                let slot = slot.clone();
                let column = weak_self.clone();
                Rc::new(move |anchor: &Control| slot.get(anchor, || header_text(&column))) as CellTemplateGetter
            });
            Self {
                weak_self: weak_self.clone(),
                base: ColumnBase::new(header, width, options.clone()),
                options,
                cell_template,
                cell_editing_template,
                get_cell_template,
                get_editing_cell_template,
            }
        })
    }

    /// The options of the column.
    pub fn options(&self) -> &Rc<TemplateColumnOptions<M>> {
        &self.options
    }

    /// Sets the column header.
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.base.set_header(value);
    }

    /// The template of the cells that are not editing.
    ///
    /// # Panics
    /// When the column's template resource cannot be found from `anchor` (`KeyNotFoundException`).
    pub fn get_cell_template(&self, anchor: &Control) -> Rc<dyn IDataTemplate> {
        self.cell_template.get(anchor, || header_text(&self.weak_self))
    }

    /// The template of the cells that are editing.
    ///
    /// # Panics
    /// When the column has no editing template, or its resource cannot be found from `anchor`.
    pub fn get_cell_editing_template(&self, anchor: &Control) -> Rc<dyn IDataTemplate> {
        match &self.cell_editing_template {
            Some(slot) => slot.get(anchor, || header_text(&self.weak_self)),
            None => panic!("The template column '{}' has no cell editing template.", header_text(&self.weak_self)),
        }
    }

    /// The cell of `model`.
    pub fn create_cell_of(&self, model: &Rc<M>) -> CellRef {
        let options: Rc<dyn ITemplateCellOptions> = self.options.clone();
        TemplateCell::new(
            Some(boxed_model(model)),
            self.get_cell_template.clone(),
            self.get_editing_cell_template.clone(),
            Some(options),
        )
    }
}

fn header_text<M: TreeModel>(column: &Weak<TemplateColumn<M>>) -> String {
    let header = column.upgrade().and_then(|column| column.base.header());
    ValueTypes::to_display_string(header.as_ref())
}

impl_view_column_base!(impl[M: TreeModel] TemplateColumn<M>, model M);

impl<M: TreeModel> ICellColumn for TemplateColumn<M> {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        self.create_cell_of(&row_model::<M>(row))
    }

    fn try_reuse_cell(&self, cell: &CellRef, row: &RowRef) -> bool {
        match cell.as_any().downcast_ref::<TemplateCell>() {
            Some(template_cell) => {
                template_cell.set_value(row.model().map(unwrap_boxed_model));
                true
            }
            None => false,
        }
    }

    fn select_value(&self, model: &BoxedValue) -> Option<String> {
        let any: Rc<dyn Any> = model.clone();
        let model = any.downcast::<M>().ok()?;
        self.options.text_search_value_selector().and_then(|selector| selector(&model))
    }

    fn is_text_search_enabled(&self) -> bool {
        self.options.is_text_search_enabled()
    }
}

impl<M: TreeModel> IColumnOf<M> for TemplateColumn<M> {
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>> {
        match direction {
            ListSortDirection::Ascending => self.options.compare_ascending(),
            ListSortDirection::Descending => self.options.compare_descending(),
        }
    }

    fn kind(&self) -> ColumnKind<'_, M> {
        ColumnKind::Value(self)
    }
}

// The value of a template column is the model, as the core's template column's.
impl<M: TreeModel> IValueColumn<M> for TemplateColumn<M> {
    fn get_boxed_value(&self, model: &Rc<M>) -> Option<BoxedValue> {
        Some(boxed_model(model))
    }

    fn set_boxed_value(&self, _model: &Rc<M>, _value: Option<&BoxedValue>) -> bool {
        false
    }

    fn can_set(&self) -> bool {
        false
    }

    fn property_name(&self) -> Option<&'static str> {
        None
    }

    fn value_type_name(&self) -> &'static str {
        std::any::type_name::<Rc<M>>()
    }

    fn as_value_any(&self) -> &dyn Any {
        self
    }
}
