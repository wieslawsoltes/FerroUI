//! `Models/TreeDataGrid/TemplateCell.cs`: a cell whose content a data template builds for the
//! cell's value (the row's model).

use super::view_column::unwrap_boxed_model;
use super::{as_editable_object, BeginEditGestures, ICell, IEditableObject, ITemplateCellOptions};
use ferroui_base::BoxedValue;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::Control;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The data template of a cell, looked up from the resources of a control (`Func<Control,
/// IDataTemplate>`).
pub type CellTemplateGetter = Rc<dyn Fn(&Control) -> Rc<dyn IDataTemplate>>;

/// `TemplateCell`.
pub struct TemplateCell {
    value: RefCell<Option<BoxedValue>>,
    get_cell_template: CellTemplateGetter,
    get_cell_editing_template: Option<CellTemplateGetter>,
    options: Option<Rc<dyn ITemplateCellOptions>>,
}

impl TemplateCell {
    /// A cell of `value` shown with the template `get_cell_template` returns and edited with the
    /// one `get_cell_editing_template` returns.
    pub fn new(
        value: Option<BoxedValue>,
        get_cell_template: CellTemplateGetter,
        get_cell_editing_template: Option<CellTemplateGetter>,
        options: Option<Rc<dyn ITemplateCellOptions>>,
    ) -> Rc<Self> {
        Rc::new(Self { value: RefCell::new(value), get_cell_template, get_cell_editing_template, options })
    }

    /// The template of the cell when it is not editing.
    pub fn get_cell_template(&self) -> &CellTemplateGetter {
        &self.get_cell_template
    }

    /// The template of the cell when it is editing, if the cell can be edited.
    pub fn get_cell_editing_template(&self) -> Option<&CellTemplateGetter> {
        self.get_cell_editing_template.as_ref()
    }

    /// The value of the cell.
    pub fn value(&self) -> Option<BoxedValue> {
        self.value.borrow().clone()
    }

    /// Points the cell at another value (a reused cell of another row).
    pub fn set_value(&self, value: Option<BoxedValue>) {
        *self.value.borrow_mut() = value;
    }

    fn with_editable_value(&self, action: impl FnOnce(&dyn IEditableObject)) {
        let value = self.value.borrow().clone();
        if let Some(value) = value.map(unwrap_boxed_model) {
            if let Some(editable) = as_editable_object(&*value) {
                action(editable);
            }
        }
    }
}

impl ICell for TemplateCell {
    fn can_edit(&self) -> bool {
        self.get_cell_editing_template.is_some()
    }

    fn edit_gestures(&self) -> BeginEditGestures {
        self.options.as_ref().map(|o| o.begin_edit_gestures()).unwrap_or(BeginEditGestures::DEFAULT)
    }

    fn value(&self) -> Option<BoxedValue> {
        TemplateCell::value(self)
    }

    fn as_editable(&self) -> Option<&dyn IEditableObject> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IEditableObject for TemplateCell {
    fn begin_edit(&self) {
        self.with_editable_value(|value| value.begin_edit());
    }

    fn cancel_edit(&self) {
        self.with_editable_value(|value| value.cancel_edit());
    }

    fn end_edit(&self) {
        self.with_editable_value(|value| value.end_edit());
    }
}
