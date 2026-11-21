//! Framework-only model view/weak binding probe. No FerroRibbon imports.
#![cfg(test)]
use ferroui_base::{
    AnyValue, BoxedValue,
    data::core::{
        ValueTypes, WeakValue,
        plugins::{IPropertyAccessorPlugin, InpcPropertyAccessorPlugin},
    },
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};

trait RowView: INotifyPropertyChanged + Any {
    fn number(&self) -> i32;
    fn set_number(&self, number: i32);
}
impl PartialEq for dyn RowView {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
struct Row {
    number: Cell<i32>,
    changed: Event<str>,
}
impl Row {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            number: Cell::new(1),
            changed: Event::new(),
        })
    }
}
impl PartialEq for Row {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl INotifyPropertyChanged for Row {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}
impl RowView for Row {
    fn number(&self) -> i32 {
        self.number.get()
    }
    fn set_number(&self, number: i32) {
        if self.number.replace(number) != number {
            self.changed.raise("Number");
        }
    }
}
struct RowMetadata;
ferroui_base::ferro_markup_type!(class RowMetadata as "Row" {
    namespace:"ModelViewProbe",this:Rc<dyn RowView>,
    handles:[Row,Rc<Row>,Rc<dyn RowView>,Option<Rc<dyn RowView>>],
    properties:[Number:i32 {get:|v:&Rc<dyn RowView>|v.number(),set:|v:&Rc<dyn RowView>,n|v.set_number(n)}]
});
// Current public descriptor fields suffice for a notifier over an Rc<dyn Trait>.
// Do not invent a macro/framework limitation before exercising this supported API.
static ROW_MARKUP: MarkupType = {
    let mut m = *RowMetadata::MARKUP;
    m.notify_property_changed = Some(|value: &dyn AnyValue| {
        if let Some(v) = value.downcast_ref::<Row>() {
            Some(v)
        } else if let Some(v) = value.downcast_ref::<Rc<Row>>() {
            Some(v.as_ref())
        } else {
            value
                .downcast_ref::<Rc<dyn RowView>>()
                .map(|v| v.as_ref() as &dyn INotifyPropertyChanged)
        }
    });
    m
};
fn register() {
    ValueTypes::register_reference::<Row>();
    ValueTypes::register_nullable::<Rc<dyn RowView>>();
    ValueTypes::register_cast::<Rc<Row>, Rc<dyn RowView>>(|v| v.clone());
    ValueTypes::register_boxed_cast::<Row, Rc<dyn RowView>>(|v| {
        let v: Rc<dyn Any> = v.clone();
        Some(v.downcast::<Row>().ok()? as Rc<dyn RowView>)
    });
    MarkupType::register(&ROW_MARKUP);
    ValueTypes::register_reference_view::<Rc<dyn RowView>>(|v| v.clone() as Rc<dyn Any>);
}
#[test]
fn custom_notifier_reads_and_observes_the_actual_trait_view_without_a_proxy() {
    register();
    let row = Row::new();
    let view: Rc<dyn RowView> = row.clone();
    let boxed: BoxedValue = Rc::new(view);
    let reference = WeakValue::new(&boxed);
    let accessor = InpcPropertyAccessorPlugin
        .start(&reference, "Number")
        .unwrap();
    let values = Rc::new(RefCell::new(Vec::new()));
    let out = values.clone();
    accessor.subscribe(Rc::new(move |v| {
        out.borrow_mut()
            .push(*v.unwrap().downcast_ref::<i32>().unwrap())
    }));
    row.set_number(2);
    assert_eq!(*values.borrow(), vec![1, 2]);
    accessor.unsubscribe();
    row.set_number(3);
    assert_eq!(*values.borrow(), vec![1, 2]);
}
#[test]
fn concrete_reference_binding_does_not_retain_its_source() {
    register();
    let row = Row::new();
    let weak = Rc::downgrade(&row);
    let boxed: BoxedValue = row.clone();
    let reference = WeakValue::new(&boxed);
    let accessor = InpcPropertyAccessorPlugin
        .start(&reference, "Number")
        .unwrap();
    accessor.subscribe(Rc::new(|_| {}));
    drop(reference);
    drop(boxed);
    drop(row);
    assert!(weak.upgrade().is_none());
    assert!(accessor.value().is_none());
    accessor.unsubscribe();
}
#[test]
fn trait_view_normalizes_to_the_same_registered_concrete_allocation() {
    register();
    let row = Row::new();
    let view: Rc<dyn RowView> = row.clone();
    let boxed: BoxedValue = Rc::new(view);
    let canonical = ValueTypes::reference_object(&boxed)
        .expect("registered model view must expose its object allocation");
    let canonical: Rc<dyn Any> = canonical;
    let original: Rc<dyn Any> = row;
    assert!(Rc::ptr_eq(&canonical, &original));
}
#[test]
fn trait_reference_binding_does_not_retain_its_source() {
    register();
    let row = Row::new();
    let weak = Rc::downgrade(&row);
    let view: Rc<dyn RowView> = row.clone();
    let boxed: BoxedValue = Rc::new(view);
    let reference = WeakValue::new(&boxed);
    let accessor = InpcPropertyAccessorPlugin
        .start(&reference, "Number")
        .unwrap();
    accessor.subscribe(Rc::new(|_| {}));
    drop(reference);
    drop(boxed);
    drop(row);
    assert!(
        weak.upgrade().is_none(),
        "trait view made the binding retain the model"
    );
    assert!(accessor.value().is_none());
    accessor.unsubscribe();
}
