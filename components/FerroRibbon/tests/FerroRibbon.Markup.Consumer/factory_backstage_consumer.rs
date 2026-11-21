//! Downstream plain selector and native factory virtuals, outside the library.
use ferroribbon::{
    compat::{DataTemplateSelector, DataTemplateSelectorImpl},
    controls::{RibbonBackstage, RibbonBackstageItem},
    mvvm::{
        RibbonElementFactory, RibbonElementFactoryImpl, RibbonElementFactoryImplExt,
        RibbonModelBindings,
    },
};
use ferroribbon_core::model::{RibbonBackstageItemModelImpl, RibbonBackstageModelImpl};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref,
    data::core::ValueTypes,
    ferro_class, ferro_class_info, instantiate,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::templates::IDataTemplate;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub struct ConsumerTemplateSelector {
    template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    last: RefCell<Option<BoxedValue>>,
    calls: Cell<i32>,
}
impl ConsumerTemplateSelector {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            template: RefCell::default(),
            last: RefCell::default(),
            calls: Cell::new(0),
        })
    }
    pub fn template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.template.borrow().clone()
    }
    pub fn set_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.template.replace(value);
    }
    pub fn last(&self) -> Option<BoxedValue> {
        self.last.borrow().clone()
    }
    pub fn clear_last(&self) {
        self.last.take();
    }
    pub fn calls(&self) -> i32 {
        self.calls.get()
    }
}
impl PartialEq for ConsumerTemplateSelector {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DataTemplateSelectorImpl for ConsumerTemplateSelector {
    fn select_template_core(&self, item: Option<BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        self.calls.set(self.calls.get() + 1);
        self.last.replace(item);
        self.template()
    }
}
ferroui_base::ferro_markup_type!(class ConsumerTemplateSelector{namespace:"FerroRibbon.Consumer",this:Rc<ConsumerTemplateSelector>,handles:[ConsumerTemplateSelector,Rc<ConsumerTemplateSelector>,Option<Rc<ConsumerTemplateSelector>>],base:Rc<dyn DataTemplateSelectorImpl>,constructors:[()=>ConsumerTemplateSelector::new],properties:[Template:Option<Rc<dyn IDataTemplate>>{get:|s:&Rc<ConsumerTemplateSelector>|s.template(),set:|s:&Rc<ConsumerTemplateSelector>,t|s.set_template(t)},Calls:i32{get:|s:&Rc<ConsumerTemplateSelector>|s.calls()}]});
pub fn register() {
    ValueTypes::register_reference::<ConsumerTemplateSelector>();
    ValueTypes::register_cast::<Rc<ConsumerTemplateSelector>, Rc<dyn DataTemplateSelectorImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_cast::<
        Rc<ConsumerTemplateSelector>,
        Option<Rc<dyn DataTemplateSelectorImpl>>,
    >(|v| Some(v.clone()));
    MarkupType::register_all(&[
        ConsumerTemplateSelector::MARKUP,
        DataTemplateSelector::MARKUP,
    ]);
}
#[repr(C)]
pub struct ConsumerBackstageFactory {
    base: RibbonElementFactory,
    backstages: Cell<i32>,
    items: Cell<i32>,
}
ferro_class!(ConsumerBackstageFactory:RibbonElementFactory);
ferro_class_info!(ConsumerBackstageFactory{new:ConsumerBackstageFactory::new,markup:{namespace:"FerroRibbon.Consumer"}});
impl FerroObjectImpl for ConsumerBackstageFactory {}
impl ConsumerBackstageFactory {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonElementFactory::construct(),
            backstages: Cell::new(0),
            items: Cell::new(0),
        })
    }
    pub fn backstages(&self) -> i32 {
        self.backstages.get()
    }
    pub fn items(&self) -> i32 {
        self.items.get()
    }
}
impl RibbonElementFactoryImpl for ConsumerBackstageFactory {
    fn create_backstage(
        this: &Self,
        m: Option<Rc<dyn RibbonBackstageModelImpl>>,
        b: Option<Rc<RibbonModelBindings>>,
    ) -> Ref<RibbonBackstage> {
        this.backstages.set(this.backstages.get() + 1);
        Self::parent_create_backstage(this, m, b)
    }
    fn create_backstage_item(
        this: &Self,
        m: Option<Rc<dyn RibbonBackstageItemModelImpl>>,
        b: Option<Rc<RibbonModelBindings>>,
    ) -> Ref<RibbonBackstageItem> {
        this.items.set(this.items.get() + 1);
        Self::parent_create_backstage_item(this, m, b)
    }
}
