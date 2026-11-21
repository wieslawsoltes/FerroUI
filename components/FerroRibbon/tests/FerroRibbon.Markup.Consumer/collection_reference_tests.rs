//! Original object slots use actual registered collection allocations and virtual equality.
use ferroribbon::metadata::{model_icon::*, models::*};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{RibbonString, collections::RibbonReadOnlyListImpl, model::*};
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn source() -> Value {
    serde_json::from_str(include_str!("../fixtures/collection-reference.json")).unwrap()
}
fn set(
    meta: &ferroui_base::metadata::MarkupType,
    owner: &BoxedValue,
    name: &str,
    value: Option<BoxedValue>,
) {
    let p = meta.properties.iter().find(|p| p.name == name).unwrap();
    (p.set.unwrap())(&[Some(owner.clone()), value]).unwrap();
}
fn get(meta: &ferroui_base::metadata::MarkupType, owner: &BoxedValue, name: &str) -> BoxedValue {
    let p = meta.properties.iter().find(|p| p.name == name).unwrap();
    (p.get.unwrap())(&[Some(owner.clone())]).unwrap().unwrap()
}
#[test]
fn original_four_object_slots_retain_list_array_and_nullable_interface_views_with_exact_notifications()
 {
    crate::tests::run(|| {
        let mut cases = Vec::new();
        for slot in ["Tag", "ItemParameter", "Content", "BackstageParameter"] {
            let item = RibbonButtonModel::new(Some("id".into()), Some("Label".into()));
            let page = RibbonBackstageItemModel::with_options(
                Some("page".into()),
                Some("Page".into()),
                None,
                None,
            );
            let trace = Rc::new(RefCell::new(Vec::<String>::new()));
            let t = trace.clone();
            item.property_changed()
                .add(Rc::new(move |name| t.borrow_mut().push(name.into())));
            let t = trace.clone();
            page.property_changed()
                .add(Rc::new(move |name| t.borrow_mut().push(name.into())));
            let (owner, meta, name): (BoxedValue, _, _) = match slot {
                "Tag" => (
                    Rc::new(item.clone() as Rc<dyn RibbonNodeModelImpl>),
                    RibbonNodeModelMetadata::MARKUP,
                    "Tag",
                ),
                "ItemParameter" => (
                    Rc::new(item.clone() as Rc<dyn RibbonItemModelImpl>),
                    RibbonItemModelMetadata::MARKUP,
                    "CommandParameter",
                ),
                "Content" => (
                    Rc::new(page.clone() as Rc<dyn RibbonBackstageItemModelImpl>),
                    RibbonBackstageItemModelMetadata::MARKUP,
                    "Content",
                ),
                _ => (
                    Rc::new(page.clone() as Rc<dyn RibbonBackstageItemModelImpl>),
                    RibbonBackstageItemModelMetadata::MARKUP,
                    "CommandParameter",
                ),
            };
            let list = NativeIconLayerListItems::from_values(vec![Some(RibbonIconLayer::new(
                Some("x".into()),
            ))]);
            let read = list.clone() as NativeIconLayers;
            set(meta, &owner, name, Some(Rc::new(list.clone())));
            let original = get(meta, &owner, name);
            let actual: BoxedValue = list.clone();
            assert!(Rc::ptr_eq(&original, &actual));
            set(meta, &owner, name, Some(Rc::new(read.clone())));
            let retained_list = Rc::ptr_eq(&original, &get(meta, &owner, name));
            set(meta, &owner, name, Some(Rc::new(Some(read))));
            set(
                meta,
                &owner,
                name,
                Some(Rc::new(NativeIconLayerListItems::from_values(vec![Some(
                    RibbonIconLayer::new(Some("x".into())),
                )]))),
            );
            let array = RibbonFixedItems::new(vec![Some(RibbonIconLayer::new(Some("x".into())))]);
            set(meta, &owner, name, Some(Rc::new(array.clone())));
            let original = get(meta, &owner, name);
            let actual: BoxedValue = array.clone();
            assert!(Rc::ptr_eq(&original, &actual));
            set(meta, &owner, name, Some(Rc::new(array as NativeIconLayers)));
            let retained_array = Rc::ptr_eq(&original, &get(meta, &owner, name));
            set(meta, &owner, name, None);
            set(meta, &owner, name, None);
            cases.push(json!({"slot":slot,"trace":*trace.borrow(),"retainedList":retained_list,"retainedArray":retained_array}));
        }
        assert_eq!(json!(cases), source()["corpus"]["cases"]);
    });
}
struct RegisteredList {
    calls: Cell<i32>,
}
impl PartialEq for RegisteredList {
    fn eq(&self, _: &Self) -> bool {
        self.calls.set(self.calls.get() + 1);
        false
    }
}
impl RibbonReadOnlyListImpl<Option<Rc<RibbonIconLayer>>> for RegisteredList {
    fn count(&self) -> i32 {
        1
    }
    fn get(&self, _: i32) -> Option<Rc<RibbonIconLayer>> {
        Some(RibbonIconLayer::new(Some("custom".into())))
    }
    fn iter_owned(self: Rc<Self>) -> RibbonIterator<Option<Rc<RibbonIconLayer>>> {
        Box::new(std::iter::empty())
    }
}
struct UnregisteredList;
impl RibbonReadOnlyListImpl<Option<Rc<RibbonIconLayer>>> for UnregisteredList {
    fn count(&self) -> i32 {
        1
    }
    fn get(&self, _: i32) -> Option<Rc<RibbonIconLayer>> {
        Some(RibbonIconLayer::new(Some("unregistered".into())))
    }
    fn iter_owned(self: Rc<Self>) -> RibbonIterator<Option<Rc<RibbonIconLayer>>> {
        Box::new(std::iter::empty())
    }
}
#[test]
fn registered_custom_view_dispatches_false_self_equals_and_unregistered_view_stays_usable() {
    crate::tests::run(|| {
        ValueTypes::register_reference::<RegisteredList>();
        ValueTypes::register_cast::<Rc<RegisteredList>, NativeIconLayers>(|v| {
            v.clone() as NativeIconLayers
        });
        let custom = Rc::new(RegisteredList {
            calls: Cell::new(0),
        });
        let weak = Rc::downgrade(&custom);
        let view = custom.clone() as NativeIconLayers;
        let boxed = Rc::new(view) as BoxedValue;
        let canonical = ValueTypes::reference_object(&boxed).unwrap();
        let actual: BoxedValue = custom.clone();
        assert!(Rc::ptr_eq(&canonical, &actual));
        let item = RibbonButtonModel::new(Some("id".into()), Some("Label".into()));
        let changes = Rc::new(Cell::new(0));
        let changed = changes.clone();
        item.property_changed().add(Rc::new(move |name| {
            if name == "Tag" {
                changed.set(changed.get() + 1)
            }
        }));
        let owner = Rc::new(item.clone() as Rc<dyn RibbonNodeModelImpl>) as BoxedValue;
        let meta = RibbonNodeModelMetadata::MARKUP;
        set(meta, &owner, "Tag", Some(actual.clone()));
        set(meta, &owner, "Tag", Some(boxed.clone()));
        let stored = get(meta, &owner, "Tag");
        assert_eq!(
            json!({"calls":custom.calls.get(),"changes":changes.get(),"retained":Rc::ptr_eq(&stored,&actual)}),
            source()["corpus"]["custom"]
        );
        let cast = ValueTypes::try_cast(&stored, ValueType::of::<NativeIconLayers>()).unwrap();
        assert_eq!(
            cast.downcast_ref::<NativeIconLayers>()
                .unwrap()
                .get(0)
                .unwrap()
                .data(),
            Some(RibbonString::from("custom"))
        );
        set(meta, &owner, "Tag", None);
        drop(stored);
        drop(cast);
        drop(canonical);
        drop(actual);
        drop(boxed);
        drop(custom);
        assert!(
            weak.upgrade().is_none(),
            "type registrations must not own instances"
        );
        let view = Rc::new(UnregisteredList) as NativeIconLayers;
        let value = Rc::new(view) as BoxedValue;
        assert!(ValueTypes::reference_object(&value).is_none());
        let output = (RibbonIconLayersMetadata::MARKUP.indexers[0].get.unwrap())(&[
            Some(value),
            Some(Rc::new(0i32)),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(
            ValueTypes::try_cast(&output, ValueType::of::<Rc<RibbonIconLayer>>())
                .unwrap()
                .downcast_ref::<Rc<RibbonIconLayer>>()
                .unwrap()
                .data(),
            Some("unregistered".into())
        );
    });
}
