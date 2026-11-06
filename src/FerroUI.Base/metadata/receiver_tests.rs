//! Receiver lifetime and conversion checks through public metadata invokers.
//!
//! These cases also compile as an external test module against the production
//! Base library: they do not replace metadata or call private receiver helpers.

use crate::data::core::ValueTypes;
use crate::metadata::{
    from_markup_value, MarkupArguments, MarkupDelegate, MarkupInvoke, MarkupInvokeError,
    MarkupProperty, MarkupTyped, MarkupValue,
};
use crate::{BoxedValue, FerroObject, FerroObjectImpl, Ref, StaticType};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    Some(Rc::new(value))
}

fn number(value: MarkupValue) -> i32 {
    from_markup_value(&value).expect("an i32 result")
}

struct SharedReceiver {
    value: Cell<i32>,
    drops: Rc<Cell<usize>>,
}

impl SharedReceiver {
    fn new(value: i32, drops: &Rc<Cell<usize>>) -> Rc<Self> {
        Rc::new(Self { value: Cell::new(value), drops: drops.clone() })
    }

    fn strong_count(this: &Rc<Self>) -> usize {
        // An accessor may register more conversions. In particular, a
        // converted receiver must not keep a registry borrow across this call.
        ValueTypes::register_nullable::<i64>();
        Rc::strong_count(this)
    }
}

impl PartialEq for SharedReceiver {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Drop for SharedReceiver {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

crate::ferro_markup_type!(class SharedReceiver {
    handles: [Rc<SharedReceiver>],
    this: Rc<SharedReceiver>,
    properties: [
        StrongCount: usize { get: SharedReceiver::strong_count },
        Value: i32 {
            get: |this: &Rc<SharedReceiver>| this.value.get(),
            set: |this: &Rc<SharedReceiver>, value: i32| this.value.set(value),
        },
        Checked: i32 {
            try_get: |this: &Rc<SharedReceiver>| {
                if this.value.get() < 0 { Err("negative value") } else { Ok(this.value.get()) }
            },
            try_set: |this: &Rc<SharedReceiver>, value: i32| {
                if value < 0 {
                    Err("negative value")
                } else {
                    this.value.set(value);
                    Ok(())
                }
            },
        },
    ],
    methods: [
        fn Add(i32) -> i32 => |this: &Rc<SharedReceiver>, delta: i32| {
            this.value.set(this.value.get() + delta);
            this.value.get()
        },
        try fn CheckedAdd(i32) -> i32 => |this: &Rc<SharedReceiver>, delta: i32| {
            if delta < 0 { Err("negative delta") } else { Ok(this.value.get() + delta) }
        },
        fn Panic() -> i32 => |this: &Rc<SharedReceiver>| {
            assert!(this.value.get() < 0, "receiver panic");
            this.value.get()
        },
    ],
    indexers: [
        (i32) -> i32 {
            get: |this: &Rc<SharedReceiver>, offset: i32| this.value.get() + offset,
            set: |this: &Rc<SharedReceiver>, offset: i32, value: i32| this.value.set(value - offset),
        },
        (String) -> i32 {
            try_get: |this: &Rc<SharedReceiver>, offset: String| {
                offset.parse::<i32>().map(|offset| this.value.get() + offset).map_err(|_| "invalid index")
            },
            try_set: |this: &Rc<SharedReceiver>, offset: String, value: i32| {
                offset.parse::<i32>().map(|offset| this.value.set(value - offset)).map_err(|_| "invalid index")
            },
        },
    ],
    events: [
        Changed(i32) => |this: &Rc<SharedReceiver>, handler: MarkupDelegate| {
            handler.invoke(&[boxed(this.value.get())]);
        },
        try CheckedChanged(i32) => |this: &Rc<SharedReceiver>, handler: MarkupDelegate| {
            if this.value.get() < 0 {
                Err("negative value")
            } else {
                handler.invoke(&[boxed(this.value.get())]);
                Ok(())
            }
        },
    ],
});

fn shared_property(name: &str) -> &'static MarkupProperty {
    <SharedReceiver as MarkupTyped>::MARKUP.find_property(name).unwrap()
}

#[derive(PartialEq)]
struct ConvertedReceiver(Rc<SharedReceiver>);

fn register_receiver_conversion() {
    ValueTypes::register_cast::<ConvertedReceiver, Rc<SharedReceiver>>(|input| input.0.clone());
}

#[test]
fn exact_shared_receiver_is_borrowed_for_the_call() {
    let drops = Rc::new(Cell::new(0));
    let receiver = SharedReceiver::new(7, &drops);
    let weak = Rc::downgrade(&receiver);
    let arguments = [boxed(receiver.clone())];
    let get: MarkupInvoke = shared_property("StrongCount").get.unwrap();

    assert_eq!(from_markup_value::<usize>(&get(&arguments).unwrap()), Some(2));
    assert_eq!(Rc::strong_count(&receiver), 2);
    assert_eq!(drops.get(), 0);
    drop(receiver);
    assert!(weak.upgrade().is_some());
    drop(arguments);
    assert!(weak.upgrade().is_none());
    assert_eq!(drops.get(), 1);
}

#[test]
fn all_instance_member_forms_keep_the_typed_receiver_and_allow_reentry() {
    let drops = Rc::new(Cell::new(0));
    let receiver = SharedReceiver::new(10, &drops);
    let target = boxed(receiver.clone());
    let value = shared_property("Value");
    let checked = shared_property("Checked");
    let markup = <SharedReceiver as MarkupTyped>::MARKUP;

    crate::__ferro_markup_functions! {
        let _: fn(&Rc<SharedReceiver>) -> i32 = SharedReceiver::__markup_get_Value;
        let _: fn(&Rc<SharedReceiver>, i32) = SharedReceiver::__markup_set_Value;
        let _: fn(&Rc<SharedReceiver>) -> Result<i32, MarkupInvokeError> = SharedReceiver::__markup_get_Checked;
        let _: fn(&Rc<SharedReceiver>, i32) -> Result<(), MarkupInvokeError> = SharedReceiver::__markup_set_Checked;
    }

    assert_eq!(number((value.get.unwrap())(&[target.clone()]).unwrap()), 10);
    assert_eq!((value.set.unwrap())(&[target.clone(), boxed(11)]).unwrap(), None);
    assert_eq!(number((checked.get.unwrap())(&[target.clone()]).unwrap()), 11);
    assert_eq!((checked.set.unwrap())(&[target.clone(), boxed(13)]).unwrap(), None);
    let add = markup.find_methods("Add").next().unwrap().invoke;
    assert_eq!(number(add(&[target.clone(), boxed(4)]).unwrap()), 17);
    let checked_add = markup.find_methods("CheckedAdd").next().unwrap().invoke;
    assert_eq!(number(checked_add(&[target.clone(), boxed(2)]).unwrap()), 19);
    assert_eq!(checked_add(&[target.clone(), boxed(-1)]), Err(MarkupInvokeError::Failed("negative delta".into())));

    let indexer = &markup.indexers[0];
    assert_eq!(number((indexer.get.unwrap())(&[target.clone(), boxed(2)]).unwrap()), 19);
    assert_eq!((indexer.set.unwrap())(&[target.clone(), boxed(2), boxed(100)]).unwrap(), None);
    let checked_indexer = &markup.indexers[1];
    assert_eq!(number((checked_indexer.get.unwrap())(&[target.clone(), boxed("2".to_string())]).unwrap()), 100);
    assert_eq!((checked_indexer.set.unwrap())(&[target.clone(), boxed("2".to_string()), boxed(50)]).unwrap(), None);

    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let weak = Rc::downgrade(&receiver);
    let handler = MarkupDelegate::new(move |arguments| {
        let this = weak.upgrade().expect("the invocation keeps its receiver alive");
        assert_eq!(number(arguments[0].clone()), 48);
        let get: MarkupInvoke = shared_property("Value").get.unwrap();
        assert_eq!(number(get(&[boxed(this)]).unwrap()), 48);
        called.set(called.get() + 1);
        None
    });
    for name in ["Changed", "CheckedChanged"] {
        assert_eq!((markup.find_event(name).unwrap().add)(&[target.clone(), boxed(handler.clone())]).unwrap(), None);
    }
    assert_eq!(calls.get(), 2);
    assert_eq!((checked.set.unwrap())(&[target.clone(), boxed(-1)]), Err(MarkupInvokeError::Failed("negative value".into())));
    assert_eq!(receiver.value.get(), 48);
    receiver.value.set(-1);
    assert_eq!((checked.get.unwrap())(&[target.clone()]), Err(MarkupInvokeError::Failed("negative value".into())));
    assert_eq!((markup.find_event("CheckedChanged").unwrap().add)(&[target, boxed(handler)]), Err(MarkupInvokeError::Failed("negative value".into())));
    assert_eq!(calls.get(), 2);
}

#[test]
fn converted_shared_receiver_owns_its_handle_and_releases_registry_borrows() {
    register_receiver_conversion();
    let drops = Rc::new(Cell::new(0));
    let receiver = SharedReceiver::new(7, &drops);
    let arguments = [boxed(ConvertedReceiver(receiver.clone()))];
    let get = shared_property("StrongCount").get.unwrap();

    assert_eq!(from_markup_value::<usize>(&get(&arguments).unwrap()), Some(3));
    assert_eq!(Rc::strong_count(&receiver), 2);
    assert_eq!(number((shared_property("Value").get.unwrap())(&arguments).unwrap()), 7);
    drop(arguments);
    assert_eq!(Rc::strong_count(&receiver), 1);
    drop(receiver);
    assert_eq!(drops.get(), 1);
}

#[test]
fn shared_receiver_cleanup_survives_exact_and_converted_callback_panics() {
    register_receiver_conversion();
    let panic = <SharedReceiver as MarkupTyped>::MARKUP.find_methods("Panic").next().unwrap().invoke;
    for converted in [false, true] {
        let drops = Rc::new(Cell::new(0));
        let receiver = SharedReceiver::new(7, &drops);
        let weak = Rc::downgrade(&receiver);
        let arguments = [if converted { boxed(ConvertedReceiver(receiver.clone())) } else { boxed(receiver.clone()) }];

        assert!(catch_unwind(AssertUnwindSafe(|| panic(&arguments))).is_err());
        assert_eq!(Rc::strong_count(&receiver), 2);
        assert_eq!(drops.get(), 0);
        assert_eq!(number((shared_property("Value").get.unwrap())(&arguments).unwrap()), 7);
        drop(arguments);
        drop(receiver);
        assert!(weak.upgrade().is_none());
        assert_eq!(drops.get(), 1);
    }
}

struct CustomReceiver {
    value: i32,
    events: Rc<RefCell<Vec<&'static str>>>,
    panic_on_clone: Rc<Cell<bool>>,
    cloned: bool,
}

impl Clone for CustomReceiver {
    fn clone(&self) -> Self {
        self.events.borrow_mut().push("clone");
        assert!(!self.panic_on_clone.get(), "custom clone panic");
        Self {
            value: self.value + 100,
            events: self.events.clone(),
            panic_on_clone: self.panic_on_clone.clone(),
            cloned: true,
        }
    }
}

impl PartialEq for CustomReceiver {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Drop for CustomReceiver {
    fn drop(&mut self) {
        self.events.borrow_mut().push(if self.cloned { "drop clone" } else { "drop original" });
    }
}

crate::ferro_markup_type!(struct CustomReceiver {
    handles: [CustomReceiver],
    properties: [
        Value: i32 { get: |this: &CustomReceiver| {
            this.events.borrow_mut().push("get");
            this.value
        } },
    ],
    methods: [
        fn Add(i32) -> i32 => |this: &CustomReceiver, amount: i32| this.value + amount,
        fn Panic() -> i32 => |this: &CustomReceiver| {
            this.events.borrow_mut().push("panic");
            panic!("custom receiver callback panic");
        },
    ],
});

#[test]
fn custom_clone_keeps_its_value_side_effects_order_and_unwind_cleanup() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let panic_on_clone = Rc::new(Cell::new(false));
    let arguments = [boxed(CustomReceiver {
        value: 1, events: events.clone(), panic_on_clone: panic_on_clone.clone(), cloned: false,
    })];
    let markup = <CustomReceiver as MarkupTyped>::MARKUP;
    let get = markup.find_property("Value").unwrap().get.unwrap();
    let add = markup.find_methods("Add").next().unwrap().invoke;
    let panic = markup.find_methods("Panic").next().unwrap().invoke;

    assert_eq!(number(get(&arguments).unwrap()), 101);
    assert_eq!(&*events.borrow(), &["clone", "get", "drop clone"]);
    events.borrow_mut().clear();
    let error = add(&[arguments[0].clone(), boxed("wrong argument")]).unwrap_err();
    assert!(matches!(error, MarkupInvokeError::Argument { index: 1, .. }));
    assert_eq!(&*events.borrow(), &["clone", "drop clone"]);
    events.borrow_mut().clear();
    assert_eq!(add(&arguments), Err(MarkupInvokeError::ArgumentCount { expected: 2, actual: 1 }));
    assert!(events.borrow().is_empty());

    assert!(catch_unwind(AssertUnwindSafe(|| panic(&arguments))).is_err());
    assert_eq!(&*events.borrow(), &["clone", "panic", "drop clone"]);
    events.borrow_mut().clear();
    panic_on_clone.set(true);
    assert!(catch_unwind(AssertUnwindSafe(|| get(&arguments))).is_err());
    assert_eq!(&*events.borrow(), &["clone"]);
    events.borrow_mut().clear();
    panic_on_clone.set(false);
    assert_eq!(number(get(&arguments).unwrap()), 101);
    assert_eq!(&*events.borrow(), &["clone", "get", "drop clone"]);
    events.borrow_mut().clear();
    drop(arguments);
    assert_eq!(&*events.borrow(), &["drop original"]);
}

#[repr(C)]
pub struct ReceiverObject {
    base: FerroObject,
    value: Cell<i32>,
}

crate::ferro_class!(ReceiverObject: FerroObject);
crate::ferro_impl_classes!(ReceiverObject: FerroObjectImpl);

impl ReceiverObject {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), value: Cell::new(7) }
    }

    pub fn new() -> Ref<Self> {
        crate::instantiate(Self::construct())
    }
}

crate::ferro_class_info!(ReceiverObject {
    markup: {
        properties: [
            Value: i32 {
                get: |this: &Ref<ReceiverObject>| this.value.get(),
                set: |this: &Ref<ReceiverObject>, value: i32| this.value.set(value),
            },
        ],
    },
    new: ReceiverObject::new,
});

#[test]
fn object_receivers_accept_exact_and_base_handles_and_reject_wrong_or_null_values() {
    let receiver = ReceiverObject::new();
    let weak = receiver.downgrade();
    let property = ReceiverObject::TYPE.markup().unwrap().find_property("Value").unwrap();
    let get: MarkupInvoke = property.get.unwrap();
    let set: MarkupInvoke = property.set.unwrap();
    let exact = boxed(receiver.clone());
    let base: Ref<FerroObject> = receiver.clone().upcast();
    let base = boxed(base);

    assert_eq!(number(get(&[exact.clone()]).unwrap()), 7);
    set(&[base.clone(), boxed(23)]).unwrap();
    assert_eq!(number(get(&[base.clone()]).unwrap()), 23);
    assert_eq!(receiver.value.get(), 23);
    for wrong in [None, boxed(5i32)] {
        let error = get(&[wrong.clone()]).unwrap_err();
        assert_eq!(error, MarkupInvokeError::Argument {
            index: 0,
            expected: std::any::type_name::<Ref<ReceiverObject>>(),
            actual: if wrong.is_none() { "null".into() } else { std::any::type_name::<i32>().into() },
        });
    }
    drop(receiver);
    drop(exact);
    assert!(weak.upgrade().is_some());
    drop(base);
    assert!(weak.upgrade().is_none());
}

#[test]
fn public_argument_conversion_still_clones_shared_handles() {
    let drops = Rc::new(Cell::new(0));
    let receiver = SharedReceiver::new(7, &drops);
    let arguments = [boxed(receiver.clone())];
    let copied = from_markup_value::<Rc<SharedReceiver>>(&arguments[0]).unwrap();
    assert!(Rc::ptr_eq(&copied, &receiver));
    assert_eq!(Rc::strong_count(&receiver), 3);
    drop(copied);
    let mut reader = MarkupArguments::new(&arguments, 1).unwrap();
    let copied = reader.next::<Rc<SharedReceiver>>().unwrap();
    assert!(Rc::ptr_eq(&copied, &receiver));
    assert_eq!(Rc::strong_count(&receiver), 3);
    drop(copied);
    assert_eq!(Rc::strong_count(&receiver), 2);
}

struct BoxedReceiver;

crate::ferro_markup_type!(class BoxedReceiver {
    this: BoxedValue,
    properties: [Identity: BoxedValue { get: |this: &BoxedValue| this.clone() }],
});

#[test]
fn boxed_receiver_preserves_outer_box_shortcut_and_exact_nested_identity() {
    let get: MarkupInvoke = <BoxedReceiver as MarkupTyped>::MARKUP.find_property("Identity").unwrap().get.unwrap();
    let inner: BoxedValue = Rc::new(23i32);
    let scalar = Some(inner.clone());
    let returned = get(&[scalar.clone()]).unwrap().unwrap();
    assert!(Rc::ptr_eq(&returned, &inner));
    assert_eq!(returned.downcast_ref::<i32>(), Some(&23));

    let nested: MarkupValue = Some(Rc::new(inner.clone()));
    let returned = get(&[nested.clone()]).unwrap().unwrap();
    assert!(Rc::ptr_eq(&returned, &inner));
    assert!(!Rc::ptr_eq(&returned, nested.as_ref().unwrap()));
    assert_eq!(get(&[None]), Err(MarkupInvokeError::Argument {
        index: 0, expected: std::any::type_name::<BoxedValue>(), actual: "null".into(),
    }));
    // MarkupValue itself remains an owned argument with its original
    // special case before the exact-payload check.
    let copied = from_markup_value::<MarkupValue>(&nested).unwrap();
    assert!(Rc::ptr_eq(copied.as_ref().unwrap(), nested.as_ref().unwrap()));
    assert_eq!(from_markup_value::<MarkupValue>(&None), Some(None));
}
