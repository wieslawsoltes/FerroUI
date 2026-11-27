use ferroribbon_core::{ObservableObject, ObservableObjectImpl};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::RefCell, rc::Rc};
pub struct GridViewModel {
    base: ObservableObject,
    columns: RefCell<i32>,
    first_row: RefCell<i32>,
}
impl GridViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ObservableObject::construct(),
            columns: RefCell::new(2),
            first_row: RefCell::new(0),
        })
    }
    pub fn columns(&self) -> i32 {
        *self.columns.borrow()
    }
    pub fn set_columns(&self, value: i32) {
        ObservableObject::set_property(self, &self.columns, value, "Columns");
    }
    pub fn first_row(&self) -> i32 {
        *self.first_row.borrow()
    }
    pub fn set_first_row(&self, value: i32) {
        ObservableObject::set_property(self, &self.first_row, value, "FirstRow");
    }
}
impl INotifyPropertyChanged for GridViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for GridViewModel {}
impl PartialEq for GridViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class GridViewModel {
    namespace:"FerroRibbon.Consumer",this:Rc<GridViewModel>,handles:[GridViewModel,Rc<GridViewModel>,Option<Rc<GridViewModel>>],
    constructors:[()=>GridViewModel::new],notify_property_changed:GridViewModel,
    properties:[
        Columns:i32 {get:|this:&Rc<GridViewModel>|this.columns(),set:|this:&Rc<GridViewModel>,value|this.set_columns(value)},
        FirstRow:i32 {get:|this:&Rc<GridViewModel>|this.first_row(),set:|this:&Rc<GridViewModel>,value|this.set_first_row(value)},
    ],
});
