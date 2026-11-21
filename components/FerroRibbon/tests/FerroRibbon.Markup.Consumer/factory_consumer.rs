//! Independent downstream virtual factory using the public native class hierarchy.
use ferroribbon::mvvm::{
    RibbonElementFactory, RibbonElementFactoryImpl, RibbonElementFactoryImplExt,
    RibbonModelBindings,
};
use ferroribbon_core::model::{
    RibbonButtonModelImpl, RibbonCustomItemModelImpl, RibbonGroupModelImpl,
};
use ferroui_base::{FerroObjectImpl, Ref, ferro_class, ferro_class_info, instantiate};
use ferroui_controls::Control;
use std::{cell::Cell, rc::Rc};
#[repr(C)]
pub struct ConsumerElementFactory {
    base: RibbonElementFactory,
    creates: Cell<usize>,
    customs: Cell<usize>,
    groups: Cell<usize>,
}
ferro_class!(ConsumerElementFactory:RibbonElementFactory);
ferro_class_info!(ConsumerElementFactory{new:ConsumerElementFactory::new,markup:{namespace:"FerroRibbon.Consumer"}});
impl ConsumerElementFactory {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonElementFactory::construct(),
            creates: Cell::new(0),
            customs: Cell::new(0),
            groups: Cell::new(0),
        })
    }
    pub fn created_group_count(&self) -> usize {
        self.groups.get()
    }
    pub fn created_custom_count(&self) -> usize {
        self.customs.get()
    }
    pub fn created_count(&self) -> usize {
        self.creates.get()
    }
}
impl FerroObjectImpl for ConsumerElementFactory {}
impl RibbonElementFactoryImpl for ConsumerElementFactory {
    fn create_custom(
        this: &Self,
        model: Rc<dyn RibbonCustomItemModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        this.customs.set(this.customs.get() + 1);
        Self::parent_create_custom(this, model, bindings)
    }

    fn create_group(
        this: &Self,
        model: Rc<dyn RibbonGroupModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<ferroribbon::controls::RibbonGroup> {
        this.groups.set(this.groups.get() + 1);
        let group = Self::parent_create_group(this, model, bindings);
        group.set_name(Some("factory-group".into()));
        group
    }

    fn create_button(
        this: &Self,
        _model: Rc<dyn RibbonButtonModelImpl>,
        _bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        this.creates.set(this.creates.get() + 1);
        super::button_consumer::ConsumerRibbonButton::new().upcast()
    }
}
