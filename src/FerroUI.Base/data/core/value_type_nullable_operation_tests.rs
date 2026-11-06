use super::{NullableRequest, ValueType, ValueTypes};
use crate::{AnyValue, BoxedValue, PropertyValue};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

// These are the original independently callable operation expressions. Keep
// their owning result and borrowed input boundaries as the differential oracle.
fn original_unwrap<T: PropertyValue>(value: &dyn AnyValue) -> Option<BoxedValue> {
    value.downcast_ref::<Option<T>>()?.as_ref()
        .map(|item| Rc::new(item.clone()) as BoxedValue)
}

fn original_null<T: PropertyValue>() -> BoxedValue {
    Rc::new(None::<T>)
}

fn original_wrap<T: PropertyValue>(value: &dyn Any) -> Option<BoxedValue> {
    Some(Rc::new(Some(value.downcast_ref::<T>()?.clone())))
}

fn original_reference_unwrap<T: PartialEq + 'static>(value: &dyn AnyValue) -> Option<BoxedValue> {
    match value.downcast_ref::<Option<Rc<T>>>() {
        Some(Some(v)) => Some(v.clone() as BoxedValue),
        _ => None,
    }
}

fn original_reference_null<T: PartialEq + 'static>() -> BoxedValue {
    Rc::new(Option::<Rc<T>>::None)
}

// The public normalizer owns the input until its original return expression
// completes. Supplying the known original leaf replaces only registry lookup;
// this preserves the return-place cleanup tested when input Drop panics.
fn original_normalize(
    value: BoxedValue,
    unwrap: fn(&dyn AnyValue) -> Option<BoxedValue>,
) -> Option<BoxedValue> {
    let mut value = value;
    loop {
        let id = super::value_type_id(&*value);
        if id == TypeId::of::<Option<BoxedValue>>() {
            match value.downcast_ref::<Option<BoxedValue>>() {
                Some(Some(inner)) => {
                    let inner = inner.clone();
                    value = inner;
                    continue;
                }
                _ => return None,
            }
        }
        let unwrap = Some(unwrap);
        return match unwrap {
            Some(unwrap) => unwrap(&*value),
            None => Some(value),
        };
    }
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    Unwrap,
    Wrap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanicAt {
    Never,
    Clone,
    SourceDrop,
    ResultDrop,
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Clone { input_strong_count: usize },
    SourceDrop,
    ResultDrop,
    Returned,
}

struct Trace {
    events: RefCell<Vec<Event>>,
    input: RefCell<Option<Weak<dyn AnyValue>>>,
    panic_at: PanicAt,
    armed: Cell<bool>,
}

struct Tracked {
    trace: Rc<Trace>,
    is_result: bool,
}

impl PartialEq for Tracked {
    fn eq(&self, other: &Self) -> bool {
        self.is_result == other.is_result
    }
}

impl Clone for Tracked {
    fn clone(&self) -> Self {
        let input_strong_count = self.trace.input.borrow().as_ref().unwrap().strong_count();
        self.trace.events.borrow_mut().push(Event::Clone { input_strong_count });
        if self.trace.panic_at == PanicAt::Clone && self.trace.armed.replace(false) {
            panic!("nullable clone oracle");
        }
        Self { trace: self.trace.clone(), is_result: true }
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        let panic_at = if self.is_result {
            self.trace.events.borrow_mut().push(Event::ResultDrop);
            PanicAt::ResultDrop
        } else {
            self.trace.events.borrow_mut().push(Event::SourceDrop);
            PanicAt::SourceDrop
        };
        if self.trace.panic_at == panic_at && self.trace.armed.replace(false) {
            panic!("nullable drop oracle");
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Observation {
    panicked: bool,
    events: Vec<Event>,
}

fn observe_cleanup(operation: Operation, panic_at: PanicAt, original: bool) -> Observation {
    let trace = Rc::new(Trace {
        events: RefCell::new(Vec::new()),
        input: RefCell::new(None),
        panic_at,
        armed: Cell::new(true),
    });
    let during = trace.clone();
    let result = catch_unwind(AssertUnwindSafe(move || {
        let payload = Tracked { trace: during.clone(), is_result: false };
        let input: BoxedValue = match operation {
            Operation::Unwrap => Rc::new(Some(payload)),
            Operation::Wrap => Rc::new(payload),
        };
        *during.input.borrow_mut() = Some(Rc::downgrade(&input));
        let output = match (operation, original) {
            (Operation::Unwrap, true) => original_normalize(input, original_unwrap::<Tracked>),
            (Operation::Unwrap, false) => ValueTypes::normalize(input),
            (Operation::Wrap, true) => original_wrap::<Tracked>(input.as_any()),
            (Operation::Wrap, false) => ValueTypes::try_convert_for_property(
                input.as_any(), TypeId::of::<Option<Tracked>>(),
            ),
        };
        during.events.borrow_mut().push(Event::Returned);
        let output = output.expect("the registered operation succeeds");
        assert_eq!(Rc::strong_count(&output), 1);
        match operation {
            Operation::Unwrap => assert!(output.downcast_ref::<Tracked>().unwrap().is_result),
            Operation::Wrap => assert!(output.downcast_ref::<Option<Tracked>>().unwrap().as_ref().unwrap().is_result),
        }
        drop(output);
    }));
    let events = std::mem::take(&mut *trace.events.borrow_mut());
    Observation { panicked: result.is_err(), events }
}

#[test]
fn ordinary_nullable_operations_match_original_cleanup_oracles() {
    ValueTypes::register_nullable::<Tracked>();
    for operation in [Operation::Unwrap, Operation::Wrap] {
        for panic_at in [PanicAt::Never, PanicAt::Clone, PanicAt::SourceDrop, PanicAt::ResultDrop] {
            let expected = observe_cleanup(operation, panic_at, true);
            let actual = observe_cleanup(operation, panic_at, false);
            assert_eq!(expected.panicked, panic_at != PanicAt::Never, "{operation:?}/{panic_at:?}");
            assert_eq!(actual, expected, "{operation:?}/{panic_at:?}");
            assert_eq!(actual.events.first(), Some(&Event::Clone { input_strong_count: 1 }));
        }
    }
}

#[test]
fn nullable_operations_preserve_nested_exact_types_and_nulls() {
    #[derive(Clone, Debug, PartialEq)]
    struct Count(i32);

    ValueTypes::register_nullable::<Count>();
    ValueTypes::register_nullable::<Option<Count>>();
    // Registering again replaces the facts with the same ones.
    ValueTypes::register_nullable::<Count>();

    let source: BoxedValue = Rc::new(Count(7));
    let expected = original_wrap::<Count>(source.as_any()).unwrap();
    let wrapped = ValueTypes::try_cast(&source, ValueType::of::<Option<Count>>()).unwrap();
    assert_eq!(wrapped.downcast_ref::<Option<Count>>(), expected.downcast_ref::<Option<Count>>());
    let unwrapped = ValueTypes::normalize(wrapped).unwrap();
    assert_eq!(unwrapped.downcast_ref::<Count>(), Some(&Count(7)));

    let none: BoxedValue = Rc::new(None::<Count>);
    let nested = ValueTypes::try_convert_for_property(none.as_any(), TypeId::of::<Option<Option<Count>>>()).unwrap();
    let expected_nested = original_wrap::<Option<Count>>(none.as_any()).unwrap();
    assert_eq!(nested.downcast_ref::<Option<Option<Count>>>(), expected_nested.downcast_ref::<Option<Option<Count>>>());
    assert_eq!(nested.downcast_ref::<Option<Option<Count>>>(), Some(&Some(None)));
    let normalized = ValueTypes::normalize(nested).unwrap();
    assert_eq!(normalized.downcast_ref::<Option<Count>>(), Some(&None));
    assert!(ValueTypes::normalize(none).is_none());

    let null = ValueTypes::null_value(ValueType::of::<Option<Count>>()).unwrap();
    assert_eq!(null.downcast_ref::<Option<Count>>(), original_null::<Count>().downcast_ref::<Option<Count>>());
    let nested_null = ValueTypes::null_value(ValueType::of::<Option<Option<Count>>>()).unwrap();
    assert_eq!(nested_null.downcast_ref::<Option<Option<Count>>>(), original_null::<Option<Count>>().downcast_ref::<Option<Option<Count>>>());
    let wrong: BoxedValue = Rc::new(false);
    assert!(original_wrap::<Count>(wrong.as_any()).is_none());
    assert!(ValueTypes::try_convert_for_property(wrong.as_any(), TypeId::of::<Option<Count>>()).is_none());

    // The special untyped wrapper retains the inner allocation rather than
    // dispatching the ordinary typed nullable operation.
    let wrapper: BoxedValue = Rc::new(Some(source.clone()));
    let result = ValueTypes::normalize(wrapper).unwrap();
    assert!(Rc::ptr_eq(&source, &result));
    assert_eq!(Rc::strong_count(&source), 2);
    assert!(ValueTypes::null_value(ValueType::of::<Option<BoxedValue>>()).unwrap()
        .downcast_ref::<Option<BoxedValue>>().unwrap().is_none());
}

#[test]
fn nullable_clone_reentry_preserves_current_snapshot_and_later_overrides() {
    struct Reentrant {
        value: i32,
        replace: Rc<Cell<bool>>,
        events: Rc<RefCell<Vec<&'static str>>>,
    }
    impl PartialEq for Reentrant {
        fn eq(&self, other: &Self) -> bool { self.value == other.value }
    }
    impl Clone for Reentrant {
        fn clone(&self) -> Self {
            self.events.borrow_mut().push("clone");
            if self.replace.replace(false) {
                ValueTypes::register_cast::<Reentrant, Option<Reentrant>>(|_| None);
                self.events.borrow_mut().push("replaced");
                let null = ValueTypes::null_value(ValueType::of::<Option<Reentrant>>()).unwrap();
                assert!(null.downcast_ref::<Option<Reentrant>>().unwrap().is_none());
            }
            Self { value: self.value, replace: self.replace.clone(), events: self.events.clone() }
        }
    }

    for operation in [Operation::Unwrap, Operation::Wrap] {
        ValueTypes::register_nullable::<Reentrant>();
        let events = Rc::new(RefCell::new(Vec::new()));
        let value = Reentrant { value: 23, replace: Rc::new(Cell::new(true)), events: events.clone() };
        let output = match operation {
            Operation::Unwrap => ValueTypes::normalize(Rc::new(Some(value))).unwrap(),
            Operation::Wrap => {
                let input: BoxedValue = Rc::new(value);
                let first = ValueTypes::try_convert_for_property(input.as_any(), TypeId::of::<Option<Reentrant>>()).unwrap();
                assert_eq!(first.downcast_ref::<Option<Reentrant>>().unwrap().as_ref().unwrap().value, 23);
                ValueTypes::normalize(first).unwrap()
            }
        };
        assert_eq!(output.downcast_ref::<Reentrant>().unwrap().value, 23);
        let later = ValueTypes::try_convert_for_property(output.as_any(), TypeId::of::<Option<Reentrant>>()).unwrap();
        assert!(later.downcast_ref::<Option<Reentrant>>().unwrap().is_none());
        let expected: &[&str] = match operation {
            Operation::Unwrap => &["clone", "replaced"],
            Operation::Wrap => &["clone", "replaced", "clone"],
        };
        assert_eq!(events.borrow().as_slice(), expected);
    }
}

#[test]
fn nullable_registration_keeps_borrowed_cast_and_conversion_views_independent() {
    #[derive(Clone, Debug, PartialEq)]
    struct Token(i32);

    let source: BoxedValue = Rc::new(Token(7));
    let target = ValueType::of::<Option<Token>>();
    let check = |expected: [Option<i32>; 3]| {
        let values = [
            ValueTypes::try_convert_registered(&source, target).unwrap(),
            ValueTypes::try_cast(&source, target).unwrap(),
            ValueTypes::try_convert_for_property(source.as_any(), target.id()).unwrap(),
        ];
        for (value, expected) in values.iter().zip(expected) {
            assert_eq!(value.downcast_ref::<Option<Token>>().unwrap().as_ref().map(|v| v.0), expected);
        }
    };

    ValueTypes::register_nullable::<Token>();
    check([Some(7), Some(7), Some(7)]);
    ValueTypes::register_conversion::<Token, Option<Token>>(|_| Some(Some(Token(11))));
    check([Some(11), Some(7), Some(7)]);
    ValueTypes::register_boxed_cast::<Token, Option<Token>>(|_| Some(None));
    check([None, None, Some(7)]);
    ValueTypes::register_boxed_conversion::<Token, Option<Token>>(|_| Some(Some(Token(13))));
    check([Some(13), None, Some(7)]);
    ValueTypes::register_nullable::<Token>();
    check([Some(7), Some(7), Some(7)]);
}

#[test]
fn reference_nullable_operations_keep_non_clone_identity_and_registration_order() {
    // Neither model implements Clone. The reference-specific nullable operation
    // clones its Rc, never the object, and never gains a borrowed object cast.
    #[derive(Debug, PartialEq)]
    struct Model(i32);
    #[derive(Debug, PartialEq)]
    struct NullableFirst(i32);

    ValueTypes::register_reference::<Model>();
    let model = Rc::new(Model(31));
    for original in [true, false] {
        let input: BoxedValue = Rc::new(Some(model.clone()));
        assert_eq!(Rc::strong_count(&model), 2);
        let result = if original {
            original_normalize(input, original_reference_unwrap::<Model>).unwrap()
        } else {
            ValueTypes::normalize(input).unwrap()
        };
        assert!(result.downcast_ref::<Model>().is_some());
        assert_eq!(Rc::strong_count(&model), 2);
        let expected: BoxedValue = model.clone();
        assert!(Rc::ptr_eq(&result, &expected));
        drop(expected);
        drop(result);
        assert_eq!(Rc::strong_count(&model), 1);
    }
    let null = ValueTypes::null_value(ValueType::of::<Option<Rc<Model>>>()).unwrap();
    let expected = original_reference_null::<Model>();
    assert_eq!(null.downcast_ref::<Option<Rc<Model>>>(), expected.downcast_ref::<Option<Rc<Model>>>());
    assert!(ValueTypes::normalize(null).is_none());

    // Exercise the unchanged four owning-Box casts and their strong ownership.
    let object: BoxedValue = model.clone();
    let handle = ValueTypes::try_cast(&object, ValueType::of::<Rc<Model>>()).unwrap();
    let nullable_object = ValueTypes::try_cast(&object, ValueType::of::<Option<Rc<Model>>>()).unwrap();
    let restored = ValueTypes::try_cast(&handle, ValueType::of::<Model>()).unwrap();
    let nullable_handle = ValueTypes::try_cast(&handle, ValueType::of::<Option<Rc<Model>>>()).unwrap();
    assert!(Rc::ptr_eq(handle.downcast_ref::<Rc<Model>>().unwrap(), &model));
    assert!(Rc::ptr_eq(nullable_object.downcast_ref::<Option<Rc<Model>>>().unwrap().as_ref().unwrap(), &model));
    assert!(Rc::ptr_eq(&restored, &object));
    assert!(Rc::ptr_eq(nullable_handle.downcast_ref::<Option<Rc<Model>>>().unwrap().as_ref().unwrap(), &model));
    assert_eq!(Rc::strong_count(&model), 6);
    assert!(ValueTypes::try_convert_for_property(object.as_any(), TypeId::of::<Option<Rc<Model>>>()).is_none());
    assert!(ValueTypes::try_convert_for_property(handle.as_any(), TypeId::of::<Option<Rc<Model>>>()).is_none());
    drop((object, handle, nullable_object, restored, nullable_handle));
    assert_eq!(Rc::strong_count(&model), 1);

    // An ordinary nullable registration after the model declaration replaces
    // nullable facts and the handle-wrap cast. Repeating model registration is
    // still first-registration-only and must not undo this later registration.
    ValueTypes::register_nullable::<Rc<Model>>();
    ValueTypes::register_reference::<Model>();
    assert!(ValueTypes::nullable_inner(ValueType::of::<Option<Rc<Model>>>()).unwrap().is::<Rc<Model>>());
    let result = ValueTypes::normalize(Rc::new(Some(model.clone()))).unwrap();
    assert!(Rc::ptr_eq(result.downcast_ref::<Rc<Model>>().unwrap(), &model));
    assert!(result.downcast_ref::<Model>().is_none());
    drop(result);
    assert_eq!(Rc::strong_count(&model), 1);

    // Reversing registration order restores model normalization, while the
    // pre-existing borrowed handle cast survives the boxed-cast replacement.
    ValueTypes::register_nullable::<Rc<NullableFirst>>();
    ValueTypes::register_reference::<NullableFirst>();
    let other = Rc::new(NullableFirst(41));
    assert!(ValueTypes::nullable_inner(ValueType::of::<Option<Rc<NullableFirst>>>()).unwrap().is::<NullableFirst>());
    let result = ValueTypes::normalize(Rc::new(Some(other.clone()))).unwrap();
    assert!(result.downcast_ref::<NullableFirst>().is_some());
    let expected: BoxedValue = other.clone();
    assert!(Rc::ptr_eq(&result, &expected));
    drop((result, expected));
    let handle: BoxedValue = Rc::new(other.clone());
    let wrapped = ValueTypes::try_convert_for_property(handle.as_any(), TypeId::of::<Option<Rc<NullableFirst>>>()).unwrap();
    assert!(Rc::ptr_eq(wrapped.downcast_ref::<Option<Rc<NullableFirst>>>().unwrap().as_ref().unwrap(), &other));
    drop((handle, wrapped));
    assert_eq!(Rc::strong_count(&other), 1);
}

#[test]
fn nullable_requests_borrow_their_input() {
    // The single dispatch frame carries no owned value: the caller keeps the
    // owning box through conversion and unwinding.
    assert!(!std::mem::needs_drop::<NullableRequest<'static>>());
}
