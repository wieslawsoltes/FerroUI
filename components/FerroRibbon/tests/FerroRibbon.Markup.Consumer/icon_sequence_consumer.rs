//! Independent view model: stores a real Core Parse result, without rebuilding its elements.
use ferroribbon::metadata::model_icon::NativeIconLayers;
use ferroribbon_core::model::RibbonIconLayer;
use ferroui_base::{
    data::{core::ValueTypes, model::ModelTypes},
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::RefCell, rc::Rc};
pub struct IconSequenceConsumer {
    layers: RefCell<NativeIconLayers>,
    changed: Event<str>,
}
impl PartialEq for IconSequenceConsumer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl INotifyPropertyChanged for IconSequenceConsumer {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}
impl IconSequenceConsumer {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            layers: RefCell::new(RibbonIconLayer::parse(Some(
                &"M0,0 L1,1|[stroke=2]M2,2 L3,3".into(),
            ))),
            changed: Event::new(),
        })
    }
    pub fn layers(&self) -> NativeIconLayers {
        self.layers.borrow().clone()
    }
    pub fn set_layers(&self, value: NativeIconLayers) {
        if Rc::ptr_eq(&self.layers.borrow(), &value) {
            return;
        }
        self.layers.replace(value);
        self.changed.raise("Layers");
    }
}
ferroui_base::ferro_markup_type!(class IconSequenceConsumer as "IconSequenceConsumer" {
 namespace:"FerroRibbon.Consumer",this:Rc<IconSequenceConsumer>,handles:[Rc<IconSequenceConsumer>,IconSequenceConsumer],
 constructors:[()=>IconSequenceConsumer::new],notify_property_changed:IconSequenceConsumer,
 properties:[Layers:NativeIconLayers {get:|v:&Rc<IconSequenceConsumer>|v.layers(),set:|v:&Rc<IconSequenceConsumer>,x|v.set_layers(x)}]
});
pub fn register() {
    ValueTypes::register_reference::<IconSequenceConsumer>();
    ModelTypes::register::<IconSequenceConsumer>(|b| b.notify_property_changed());
    MarkupType::register_all(&[IconSequenceConsumer::MARKUP]);
}
