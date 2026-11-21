//! External subclass and ordinary model/command consumer of the native input family.
use ferroribbon::controls::{RibbonControlBaseImpl, RibbonInputBaseImpl, RibbonTextBox};
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
pub struct ConsumerTextBox {
    base: RibbonTextBox,
}
ferro_class!(ConsumerTextBox:RibbonTextBox);
ferro_class_info!(ConsumerTextBox{new:ConsumerTextBox::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerTextBox:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl);
impl ConsumerTextBox {
    /// A downstream numeric control can call the source protected input helper.
    pub fn parse_number(text: Option<&str>, value: &mut f64) -> bool {
        ferroribbon::controls::RibbonInputBase::try_parse_number(text, value)
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonTextBox::construct(),
        })
    }
}
impl RibbonInputBaseImpl for ConsumerTextBox {
    fn shows_label_by_default(_this: &Self) -> bool {
        false
    }
}
pub struct InputViewModel {
    base: ObservableObject,
    text: RefCell<String>,
    calls: Rc<RefCell<Vec<String>>>,
    command: Rc<dyn ICommand>,
}
impl InputViewModel {
    pub fn new() -> Rc<Self> {
        let calls = Rc::new(RefCell::new(vec![]));
        let c = calls.clone();
        Rc::new(Self {
            base: ObservableObject::construct(),
            text: RefCell::new("initial".into()),
            calls,
            command: RibbonRelayCommand::new_with_parameter(
                move |p| {
                    c.borrow_mut()
                        .push(p.unwrap().downcast_ref::<String>().unwrap().clone())
                },
                Some(Rc::new(|p| {
                    p.and_then(|p| p.downcast_ref::<String>())
                        .is_some_and(|s| s != "blocked")
                })),
            ),
        })
    }
    pub fn text(&self) -> String {
        self.text.borrow().clone()
    }
    pub fn set_text(&self, value: String) {
        ObservableObject::set_property(self, &self.text, value, "Text");
    }
    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}
impl PartialEq for InputViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl INotifyPropertyChanged for InputViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for InputViewModel {}
ferroui_base::ferro_markup_type!(class InputViewModel{namespace:"FerroRibbon.Consumer",this:Rc<InputViewModel>,handles:[InputViewModel,Rc<InputViewModel>,Option<Rc<InputViewModel>>],constructors:[()=>InputViewModel::new],notify_property_changed:InputViewModel,properties:[Text:String{get:|v:&Rc<InputViewModel>|v.text(),set:|v:&Rc<InputViewModel>,x|v.set_text(x)},Command:Rc<dyn ferroui_base::input::ICommand>{get:|v:&Rc<InputViewModel>|ferroribbon::commands::adapt_core_command(v.command.clone())}]});
