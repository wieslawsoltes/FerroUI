//! Independent root Model binding; application graph construction stays outside the library.
use ferroribbon_core::{ObservableObject, ObservableObjectImpl, model::*};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::RefCell, rc::Rc};

pub struct RootModelViewModel {
    base: ObservableObject,
    model: RefCell<Option<Rc<dyn RibbonModelImpl>>>,
}
impl RootModelViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ObservableObject::construct(),
            model: RefCell::new(None),
        })
    }
    pub fn model(&self) -> Option<Rc<dyn RibbonModelImpl>> {
        self.model.borrow().clone()
    }
    pub fn set_model(&self, value: Option<Rc<dyn RibbonModelImpl>>) {
        ObservableObject::set_property(self, &self.model, value, "Model");
    }
}
impl INotifyPropertyChanged for RootModelViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RootModelViewModel {}
impl PartialEq for RootModelViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class RootModelViewModel {
    namespace:"FerroRibbon.Consumer",this:Rc<RootModelViewModel>,
    handles:[RootModelViewModel,Rc<RootModelViewModel>,Option<Rc<RootModelViewModel>>],
    constructors:[()=>RootModelViewModel::new],notify_property_changed:RootModelViewModel,
    properties:[Model:Option<Rc<dyn RibbonModelImpl>> {
        get:|this:&Rc<RootModelViewModel>|this.model(),
        set:|this:&Rc<RootModelViewModel>,value|this.set_model(value)
    }],
});
