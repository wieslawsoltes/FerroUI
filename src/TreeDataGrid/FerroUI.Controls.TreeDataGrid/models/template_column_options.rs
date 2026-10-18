//! `Models/TreeDataGrid/TemplateColumnOptions.cs`: the options of a template column.

use super::{BeginEditGestures, ColumnOptions, ICellOptions, IColumnOptions, ITemplateCellOptions};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// The text of a model a template column searches (`Func<TModel, string?>`).
pub type TextSearchValueSelector<M> = Rc<dyn Fn(&Rc<M>) -> Option<String>>;

/// `TemplateColumnOptions<TModel>`.
pub struct TemplateColumnOptions<M: 'static> {
    base: ColumnOptions<M>,
    is_text_search_enabled: Cell<bool>,
    text_search_value_selector: RefCell<Option<TextSearchValueSelector<M>>>,
}

impl<M: 'static> Default for TemplateColumnOptions<M> {
    fn default() -> Self {
        Self {
            base: ColumnOptions::default(),
            is_text_search_enabled: Cell::new(false),
            text_search_value_selector: RefCell::new(None),
        }
    }
}

impl<M: 'static> TemplateColumnOptions<M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the column takes part in text searches.
    pub fn is_text_search_enabled(&self) -> bool {
        self.is_text_search_enabled.get()
    }

    pub fn set_is_text_search_enabled(&self, value: bool) {
        self.is_text_search_enabled.set(value);
    }

    /// The function that selects the search text of a model.
    pub fn text_search_value_selector(&self) -> Option<TextSearchValueSelector<M>> {
        self.text_search_value_selector.borrow().clone()
    }

    pub fn set_text_search_value_selector(&self, value: Option<TextSearchValueSelector<M>>) {
        *self.text_search_value_selector.borrow_mut() = value;
    }
}

impl<M: 'static> Deref for TemplateColumnOptions<M> {
    type Target = ColumnOptions<M>;

    fn deref(&self) -> &ColumnOptions<M> {
        &self.base
    }
}

impl<M: 'static> IColumnOptions<M> for TemplateColumnOptions<M> {
    fn column_options(&self) -> &ColumnOptions<M> {
        &self.base
    }
}

impl<M: 'static> ICellOptions for TemplateColumnOptions<M> {
    fn begin_edit_gestures(&self) -> BeginEditGestures {
        self.base.begin_edit_gestures()
    }
}

impl<M: 'static> ITemplateCellOptions for TemplateColumnOptions<M> {}
