//! Original content contracts preserve object identity across the UI/model boundary.
use ferrodock_model::controls::{IDocumentContent, IToolContent};
use ferrodock_model_reactiveui::{
    core::{DockableBase, DockableBaseImpl},
    impl_dockable,
};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::MarkupType,
    BoxedValue,
};
use std::{
    any::TypeId,
    cell::RefCell,
    rc::{Rc, Weak},
};
struct Content {
    base: DockableBase,
    value: RefCell<Option<ferrodock_model::BoxedValue>>,
}
ferroui_reactiveui::reactive_object!(Content, base);
impl DockableBaseImpl for Content {}
impl_dockable!(
    Content,
    fn as_document_content(self: Rc<Self>) -> Option<Rc<dyn IDocumentContent>> {
        Some(self)
    },
    fn as_tool_content(self: Rc<Self>) -> Option<Rc<dyn IToolContent>> {
        Some(self)
    }
);
impl IDocumentContent for Content {
    fn content(&self) -> Option<ferrodock_model::BoxedValue> {
        self.value.borrow().clone()
    }
    fn set_content(&self, v: Option<ferrodock_model::BoxedValue>) {
        self.value.replace(v);
    }
}
impl IToolContent for Content {
    fn content(&self) -> Option<ferrodock_model::BoxedValue> {
        self.value.borrow().clone()
    }
    fn set_content(&self, v: Option<ferrodock_model::BoxedValue>) {
        self.value.replace(v);
    }
}
#[test]
fn original_content_metadata_routes_get_set_null_and_checked_interface_casts() {
    crate::register_types();
    let model = Rc::new_cyclic(|runtime: &Weak<Content>| Content {
        base: DockableBase::construct(runtime.clone()),
        value: RefCell::new(None),
    });
    let erased = crate::model_projection::to_dockable_value(model.clone());
    for handle in [
        TypeId::of::<Rc<dyn IDocumentContent>>(),
        TypeId::of::<Rc<dyn IToolContent>>(),
    ] {
        let meta = MarkupType::find_by_handle(handle).unwrap();
        let property = meta.find_property("Content").unwrap();
        let value: BoxedValue = Rc::new("same original object".to_owned());
        property.set.unwrap()(&[Some(erased.clone()), Some(value.clone())]).unwrap();
        let actual = property.get.unwrap()(&[Some(erased.clone())])
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&actual, &value));
        property.set.unwrap()(&[Some(erased.clone()), None]).unwrap();
        assert!(property.get.unwrap()(&[Some(erased.clone())])
            .unwrap()
            .is_none());
    }
    let typed = ValueTypes::try_cast(&erased, ValueType::of::<Rc<dyn IDocumentContent>>()).unwrap();
    assert!(Rc::ptr_eq(
        typed.downcast_ref::<Rc<dyn IDocumentContent>>().unwrap(),
        &(model as Rc<dyn IDocumentContent>)
    ));
}
