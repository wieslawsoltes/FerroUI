//! Independent model subclass: composed Core storage, public metadata, virtual dispatch.
use ferroribbon_core::{
    ObservableObjectImpl, RibbonItemSize, RibbonString,
    model::{
        RibbonButtonModel, RibbonButtonModelImpl, RibbonItemModel, RibbonItemModelImpl,
        RibbonNodeModel, RibbonNodeModelExt, RibbonNodeModelImpl,
    },
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::Cell, rc::Rc};

pub struct ConsumerButtonModel {
    base: RibbonButtonModel,
    notifications: Cell<usize>,
}
impl ConsumerButtonModel {
    pub fn new() -> Rc<Self> {
        let value = Rc::new(Self {
            base: RibbonButtonModel::construct(None, None),
            notifications: Cell::new(0),
        });
        RibbonButtonModel::initialize(value.as_ref(), None, None, RibbonItemSize::Medium);
        value
    }
    pub fn notifications(&self) -> usize {
        self.notifications.get()
    }
}
impl PartialEq for ConsumerButtonModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl INotifyPropertyChanged for ConsumerButtonModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for ConsumerButtonModel {
    fn on_property_changed(&self, name: &str) {
        self.notifications.set(self.notifications.get() + 1);
        self.property_changed().raise(name);
    }
}
impl RibbonNodeModelImpl for ConsumerButtonModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
    fn to_ribbon_string(&self) -> RibbonString {
        format!(
            "consumer:{}",
            self.label()
                .and_then(|s| s.to_utf8().ok())
                .unwrap_or_default()
        )
        .into()
    }
}
impl RibbonItemModelImpl for ConsumerButtonModel {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonModelImpl for ConsumerButtonModel {
    fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
ferroui_base::ferro_markup_type!(class ConsumerButtonModel {
    namespace:"FerroRibbon.Consumer",this:Rc<ConsumerButtonModel>,
    handles:[Rc<ConsumerButtonModel>,Option<Rc<ConsumerButtonModel>>,ConsumerButtonModel],
    base:Rc<dyn RibbonButtonModelImpl>,constructors:[()=>ConsumerButtonModel::new],
    notify_property_changed:ConsumerButtonModel,
});
pub fn register() {
    ferroribbon_core::model::register_ribbon_button_model::<ConsumerButtonModel>().unwrap();
    ferroribbon::metadata::models::register_model_notifications::<ConsumerButtonModel>();
    ValueTypes::register_reference::<ConsumerButtonModel>();
    ValueTypes::register_cast::<Rc<ConsumerButtonModel>, Rc<dyn RibbonButtonModelImpl>>(|v| {
        v.clone()
    });
    ValueTypes::register_cast::<Rc<ConsumerButtonModel>, Rc<dyn RibbonItemModelImpl>>(|v| {
        v.clone()
    });
    ValueTypes::register_cast::<Rc<ConsumerButtonModel>, Rc<dyn RibbonNodeModelImpl>>(|v| {
        v.clone()
    });
    MarkupType::register_all(&[ConsumerButtonModel::MARKUP]);
}
