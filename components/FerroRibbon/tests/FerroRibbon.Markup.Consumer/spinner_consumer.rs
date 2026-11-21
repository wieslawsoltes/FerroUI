//! Independent native subclass and framework-independent presentation model.
use ferroribbon::controls::{RibbonControlBaseImpl, RibbonInputBaseImpl, RibbonSpinner};
use ferroribbon_core::{
    ObservableObject, ObservableObjectImpl,
    commands::{ICommand, RibbonRelayCommand},
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, primitives::TemplatedControlImpl};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::RefCell, rc::Rc};
#[repr(C)]
pub struct ConsumerSpinner {
    base: RibbonSpinner,
}
ferro_class!(ConsumerSpinner:RibbonSpinner);
ferro_class_info!(ConsumerSpinner{new:ConsumerSpinner::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerSpinner:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl);
impl ConsumerSpinner {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonSpinner::construct(),
        })
    }
}
impl RibbonInputBaseImpl for ConsumerSpinner {
    fn shows_label_by_default(_c: &Self) -> bool {
        false
    }
}
pub struct NumberViewModel {
    base: ObservableObject,
    value: RefCell<f64>,
    calls: Rc<RefCell<Vec<f64>>>,
    command: Rc<dyn ICommand>,
}
impl NumberViewModel {
    pub fn new() -> Rc<Self> {
        let calls = Rc::new(RefCell::new(vec![]));
        let out = calls.clone();
        Rc::new(Self {
            base: ObservableObject::construct(),
            value: RefCell::new(4.),
            calls,
            command: RibbonRelayCommand::new_with_parameter(
                move |p| {
                    out.borrow_mut()
                        .push(*p.unwrap().downcast_ref::<f64>().unwrap())
                },
                None,
            ),
        })
    }
    pub fn value(&self) -> f64 {
        *self.value.borrow()
    }
    pub fn set_value(&self, v: f64) {
        ObservableObject::set_property(self, &self.value, v, "Value");
    }
    pub fn calls(&self) -> Vec<f64> {
        self.calls.borrow().clone()
    }
}
impl PartialEq for NumberViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl INotifyPropertyChanged for NumberViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for NumberViewModel {}
ferroui_base::ferro_markup_type!(class NumberViewModel{namespace:"FerroRibbon.Consumer",this:Rc<NumberViewModel>,handles:[NumberViewModel,Rc<NumberViewModel>,Option<Rc<NumberViewModel>>],notify_property_changed:NumberViewModel,constructors:[()=>NumberViewModel::new],properties:[Value:f64{get:|v:&Rc<NumberViewModel>|v.value(),set:|v:&Rc<NumberViewModel>,x|v.set_value(x)},Command:Rc<dyn ferroui_base::input::ICommand>{get:|v:&Rc<NumberViewModel>|ferroribbon::commands::adapt_core_command(v.command.clone())}]});
