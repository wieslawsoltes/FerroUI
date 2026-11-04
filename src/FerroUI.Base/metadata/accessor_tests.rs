//! Public invoker checks for observable callable-expression temporaries.
//!
//! The callable expressions below construct closures with destructors. They
//! must run at the original point in argument evaluation, and getters must
//! retain them through the return value's normalization and unwind cleanup.

use crate::data::core::ValueTypes;
use crate::metadata::{into_markup_value, markup_result, MarkupArguments, MarkupInvoke, MarkupInvokeError, MarkupTyped, MarkupValue};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanicAt {
    Never,
    ArgumentClone,
    SetterExpression,
    SetterCall,
    ResultClone,
    CallableDrop,
    IgnoredResultDrop,
}

crate::ferro_thread_local! {
    static EVENTS: RefCell<Vec<&'static str>> = RefCell::new(Vec::new());
    static LIVE_RECEIVERS: Cell<usize> = Cell::new(0);
    static LIVE_CALLABLES: Cell<usize> = Cell::new(0);
    static EXPECTED_RECEIVERS: Cell<usize> = Cell::new(0);
    static PANIC_AT: Cell<PanicAt> = Cell::new(PanicAt::Never);
    static SETTER_ERROR: Cell<bool> = Cell::new(false);
}

fn event(name: &'static str) {
    EVENTS.with(|events| events.borrow_mut().push(name));
}

fn events() -> Vec<&'static str> {
    EVENTS.with(|events| events.borrow().clone())
}

fn reset(panic_at: PanicAt, expected_receivers: usize) {
    assert_eq!(LIVE_RECEIVERS.with(Cell::get), 0);
    assert_eq!(LIVE_CALLABLES.with(Cell::get), 0);
    EVENTS.with(|events| events.borrow_mut().clear());
    EXPECTED_RECEIVERS.with(|value| value.set(expected_receivers));
    PANIC_AT.with(|value| value.set(panic_at));
    SETTER_ERROR.with(|value| value.set(false));
}

fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    Some(Rc::new(value))
}

#[derive(PartialEq)]
struct InvocationReceiver {
    cloned: bool,
}

impl Clone for InvocationReceiver {
    fn clone(&self) -> Self {
        event("receiver clone");
        LIVE_RECEIVERS.with(|count| count.set(count.get() + 1));
        Self { cloned: true }
    }
}

impl Drop for InvocationReceiver {
    fn drop(&mut self) {
        if self.cloned {
            event("receiver drop");
            LIVE_RECEIVERS.with(|count| count.set(count.get() - 1));
        }
    }
}

#[derive(PartialEq)]
struct SetterArgument {
    cloned: bool,
}

impl Clone for SetterArgument {
    fn clone(&self) -> Self {
        event("argument clone");
        assert!(PANIC_AT.with(Cell::get) != PanicAt::ArgumentClone, "argument clone panic");
        Self { cloned: true }
    }
}

impl Drop for SetterArgument {
    fn drop(&mut self) {
        if self.cloned {
            event("argument drop");
        }
    }
}

struct CallableGuard;

impl CallableGuard {
    fn new() -> Self {
        LIVE_CALLABLES.with(|count| count.set(count.get() + 1));
        Self
    }

    fn check(&self) {
        assert_eq!(LIVE_CALLABLES.with(Cell::get), 1);
        assert_eq!(LIVE_RECEIVERS.with(Cell::get), EXPECTED_RECEIVERS.with(Cell::get));
    }
}

impl Drop for CallableGuard {
    fn drop(&mut self) {
        event("callable drop");
        LIVE_CALLABLES.with(|count| count.set(count.get() - 1));
        assert!(PANIC_AT.with(Cell::get) != PanicAt::CallableDrop, "callable drop panic");
    }
}

#[derive(PartialEq)]
struct Returned {
    cloned: bool,
}

impl Clone for Returned {
    fn clone(&self) -> Self {
        event("result clone");
        // Nullable normalization runs user Clone while both the receiver and
        // the temporary callable expression are still alive.
        assert_eq!(LIVE_CALLABLES.with(Cell::get), 1);
        assert_eq!(LIVE_RECEIVERS.with(Cell::get), EXPECTED_RECEIVERS.with(Cell::get));
        assert!(PANIC_AT.with(Cell::get) != PanicAt::ResultClone, "result clone panic");
        Self { cloned: true }
    }
}

impl Drop for Returned {
    fn drop(&mut self) {
        event(if self.cloned { "result clone drop" } else { "result original drop" });
    }
}

struct IgnoredResult;

impl Drop for IgnoredResult {
    fn drop(&mut self) {
        event("ignored result drop");
        assert!(PANIC_AT.with(Cell::get) != PanicAt::IgnoredResultDrop, "ignored result drop panic");
    }
}

struct InvocationError;

impl fmt::Display for InvocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        event("error display");
        assert_eq!(LIVE_CALLABLES.with(Cell::get), 1);
        assert_eq!(LIVE_RECEIVERS.with(Cell::get), EXPECTED_RECEIVERS.with(Cell::get));
        f.write_str("invocation failed")
    }
}

impl Drop for InvocationError {
    fn drop(&mut self) {
        event("error drop");
    }
}

fn getter_expression() -> impl Fn(&InvocationReceiver) -> Option<Returned> {
    event("callable expression");
    let guard = CallableGuard::new();
    move |this| {
        guard.check();
        assert!(this.cloned);
        event("getter call");
        Some(Returned { cloned: false })
    }
}

fn checked_getter_expression() -> impl Fn(&InvocationReceiver) -> Result<Option<Returned>, InvocationError> {
    let get = getter_expression();
    move |this| Ok(get(this))
}

fn static_getter_expression() -> impl Fn() -> Option<Returned> {
    event("callable expression");
    let guard = CallableGuard::new();
    move || {
        guard.check();
        event("getter call");
        Some(Returned { cloned: false })
    }
}

fn checked_static_getter_expression() -> impl Fn() -> Result<Option<Returned>, InvocationError> {
    let get = static_getter_expression();
    move || Ok(get())
}

fn setter_expression() -> impl Fn(&InvocationReceiver, SetterArgument) -> IgnoredResult {
    event("callable expression");
    assert!(PANIC_AT.with(Cell::get) != PanicAt::SetterExpression, "setter expression panic");
    let guard = CallableGuard::new();
    move |this, value| {
        guard.check();
        assert!(this.cloned && value.cloned);
        event("setter call");
        assert!(PANIC_AT.with(Cell::get) != PanicAt::SetterCall, "setter callback panic");
        IgnoredResult
    }
}

fn checked_setter_expression() -> impl Fn(&InvocationReceiver, SetterArgument) -> Result<IgnoredResult, InvocationError> {
    event("callable expression");
    assert!(PANIC_AT.with(Cell::get) != PanicAt::SetterExpression, "setter expression panic");
    let guard = CallableGuard::new();
    move |this, value| {
        guard.check();
        assert!(this.cloned && value.cloned);
        event("setter call");
        assert!(PANIC_AT.with(Cell::get) != PanicAt::SetterCall, "setter callback panic");
        if SETTER_ERROR.with(Cell::get) { Err(InvocationError) } else { Ok(IgnoredResult) }
    }
}

crate::ferro_markup_type!(struct InvocationReceiver {
    handles: [InvocationReceiver],
    properties: [
        Returned: Option<Returned> { get: getter_expression() },
        CheckedReturned: Option<Returned> { try_get: checked_getter_expression() },
        Value: SetterArgument { set: setter_expression() },
        CheckedValue: SetterArgument { try_set: checked_setter_expression() },
    ],
    static_properties: [
        Returned: Option<Returned> { get: static_getter_expression() },
        CheckedReturned: Option<Returned> { try_get: checked_static_getter_expression() },
    ],
});

// The leaf bodies below are copied from the original emitter macros. Keep
// their expression and local scopes: a handwritten function is a useful
// cross-check, but is not a substitute for the original macro-shaped closure.
macro_rules! original_getter_macro {
    ($this:ty, $type_:ty;) => {
        ::std::option::Option::None
    };
    ($this:ty, $type_:ty; get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 1)?;
            let this = $crate::__ferro_markup_receiver!($this, arguments)?;
            let this: &$this = &this;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(($get)(this)))
        })
    };
    ($this:ty, $type_:ty; try_get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 1)?;
            let this = $crate::__ferro_markup_receiver!($this, arguments)?;
            let this: &$this = &this;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(
                $crate::metadata::markup_result(($get)(this))?,
            ))
        })
    };
    ($this:ty, $type_:ty; set: $set:expr $(, $($rest:tt)*)?) => {
        original_getter_macro!($this, $type_; $($($rest)*)?)
    };
    ($this:ty, $type_:ty; try_set: $set:expr $(, $($rest:tt)*)?) => {
        original_getter_macro!($this, $type_; $($($rest)*)?)
    };
}


macro_rules! original_setter_macro {
    ($this:ty, $type_:ty;) => {
        ::std::option::Option::None
    };
    ($this:ty, $type_:ty; set: $set:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 2)?;
            let this = $crate::__ferro_markup_receiver!($this, arguments)?;
            let this: &$this = &this;
            let value = arguments.next::<$type_>()?;
            let _ = ($set)(this, value);
            ::std::result::Result::Ok(::std::option::Option::None)
        })
    };
    ($this:ty, $type_:ty; try_set: $set:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 2)?;
            let this = $crate::__ferro_markup_receiver!($this, arguments)?;
            let this: &$this = &this;
            let value = arguments.next::<$type_>()?;
            let _ = $crate::metadata::markup_result(($set)(this, value))?;
            ::std::result::Result::Ok(::std::option::Option::None)
        })
    };
    ($this:ty, $type_:ty; get: $get:expr $(, $($rest:tt)*)?) => {
        original_setter_macro!($this, $type_; $($($rest)*)?)
    };
    ($this:ty, $type_:ty; try_get: $get:expr $(, $($rest:tt)*)?) => {
        original_setter_macro!($this, $type_; $($($rest)*)?)
    };
}


macro_rules! original_static_getter_macro {
    ($type_:ty;) => {
        ::std::option::Option::None
    };
    ($type_:ty; get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            $crate::metadata::MarkupArguments::new(arguments, 0)?;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(($get)()))
        })
    };
    ($type_:ty; try_get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            $crate::metadata::MarkupArguments::new(arguments, 0)?;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(
                $crate::metadata::markup_result(($get)())?,
            ))
        })
    };
    ($type_:ty; set: $set:expr $(, $($rest:tt)*)?) => {
        original_static_getter_macro!($type_; $($($rest)*)?)
    };
    ($type_:ty; try_set: $set:expr $(, $($rest:tt)*)?) => {
        original_static_getter_macro!($type_; $($($rest)*)?)
    };
}


fn setters() -> [MarkupInvoke; 2] {
    let markup = <InvocationReceiver as MarkupTyped>::MARKUP;
    [markup.find_property("Value").unwrap().set.unwrap(), markup.find_property("CheckedValue").unwrap().set.unwrap()]
}

// Also compare handwritten functions with the original macro-shaped closures
// in every mode. The candidate must match both before a scope change is accepted.
// These use public APIs and the original owned receiver path, not adapter helpers.
fn original_getter(arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
    let mut arguments = MarkupArguments::new(arguments, 1)?;
    let this = arguments.next::<InvocationReceiver>()?;
    let this: &InvocationReceiver = &this;
    Ok(into_markup_value::<Option<Returned>>((getter_expression())(this)))
}

fn original_checked_getter(arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
    let mut arguments = MarkupArguments::new(arguments, 1)?;
    let this = arguments.next::<InvocationReceiver>()?;
    let this: &InvocationReceiver = &this;
    Ok(into_markup_value::<Option<Returned>>(markup_result((checked_getter_expression())(this))?))
}

fn original_static_getter(arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
    MarkupArguments::new(arguments, 0)?;
    Ok(into_markup_value::<Option<Returned>>((static_getter_expression())()))
}

fn original_checked_static_getter(arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
    MarkupArguments::new(arguments, 0)?;
    Ok(into_markup_value::<Option<Returned>>(markup_result((checked_static_getter_expression())())?))
}

fn original_getters() -> [MarkupInvoke; 4] {
    [
        original_getter_macro!(InvocationReceiver, Option<Returned>; get: getter_expression()).unwrap(),
        original_getter_macro!(InvocationReceiver, Option<Returned>; try_get: checked_getter_expression()).unwrap(),
        original_static_getter_macro!(Option<Returned>; get: static_getter_expression()).unwrap(),
        original_static_getter_macro!(Option<Returned>; try_get: checked_static_getter_expression()).unwrap(),
    ]
}

fn setter_cases() -> [(MarkupInvoke, MarkupInvoke); 2] {
    let current = setters();
    [
        (current[0], original_setter_macro!(InvocationReceiver, SetterArgument; set: setter_expression()).unwrap()),
        (current[1], original_setter_macro!(InvocationReceiver, SetterArgument; try_set: checked_setter_expression()).unwrap()),
    ]
}

fn observe_getter(
    invoke: MarkupInvoke,
    arguments: &[MarkupValue],
    panic_at: PanicAt,
    expected_receivers: usize,
) -> (bool, Vec<&'static str>, Vec<&'static str>) {
    reset(panic_at, expected_receivers);
    let outcome = catch_unwind(AssertUnwindSafe(|| invoke(arguments)));
    let panicked = outcome.is_err();
    let before_result_drop = events();
    assert_eq!(panicked, panic_at != PanicAt::Never);
    if let Ok(result) = outcome {
        let result = result.unwrap();
        assert!(result.as_ref().unwrap().as_any().downcast_ref::<Returned>().unwrap().cloned);
        drop(result);
    }
    assert_eq!(LIVE_RECEIVERS.with(Cell::get), 0);
    assert_eq!(LIVE_CALLABLES.with(Cell::get), 0);
    (panicked, before_result_drop, events())
}

#[derive(Debug, PartialEq)]
enum SetterOutcome {
    Success,
    Error(MarkupInvokeError),
    Panic,
}

fn observe_setter(
    invoke: MarkupInvoke,
    arguments: &[MarkupValue],
    panic_at: PanicAt,
    return_error: bool,
) -> (SetterOutcome, Vec<&'static str>) {
    reset(panic_at, 1);
    SETTER_ERROR.with(|value| value.set(return_error));
    let outcome = match catch_unwind(AssertUnwindSafe(|| invoke(arguments))) {
        Ok(Ok(value)) => {
            assert_eq!(value, None);
            SetterOutcome::Success
        }
        Ok(Err(error)) => SetterOutcome::Error(error),
        Err(_) => SetterOutcome::Panic,
    };
    assert_eq!(LIVE_RECEIVERS.with(Cell::get), 0);
    assert_eq!(LIVE_CALLABLES.with(Cell::get), 0);
    (outcome, events())
}

#[test]
fn getter_callable_lives_through_result_conversion_and_unwind() {
    ValueTypes::register_nullable::<Returned>();
    let markup = <InvocationReceiver as MarkupTyped>::MARKUP;
    let receiver = boxed(InvocationReceiver { cloned: false });
    let current = [
        markup.find_property("Returned").unwrap().get.unwrap(),
        markup.find_property("CheckedReturned").unwrap().get.unwrap(),
        markup.static_properties[0].get.unwrap(),
        markup.static_properties[1].get.unwrap(),
    ];
    let original = original_getters();
    let handwritten: [MarkupInvoke; 4] = [
        original_getter, original_checked_getter,
        original_static_getter, original_checked_static_getter,
    ];
    for index in 0..current.len() {
        let instance = index < 2;
        let arguments = if instance { vec![receiver.clone()] } else { Vec::new() };
        for panic_at in [PanicAt::Never, PanicAt::ResultClone, PanicAt::CallableDrop] {
            let original = observe_getter(original[index], &arguments, panic_at, usize::from(instance));
            let handwritten = observe_getter(handwritten[index], &arguments, panic_at, usize::from(instance));
            assert_eq!(handwritten, original, "original macro/function scope, getter {index}, {panic_at:?}");
            let actual = observe_getter(current[index], &arguments, panic_at, usize::from(instance));
            assert_eq!(actual, original, "generated getter {index}, {panic_at:?}");
        }
    }
}

#[test]
fn accessor_expressions_follow_arity_receiver_and_value_checks() {
    let receiver = boxed(InvocationReceiver { cloned: false });
    let value = boxed(SetterArgument { cloned: false });
    let markup = <InvocationReceiver as MarkupTyped>::MARKUP;
    for get in [markup.find_property("Returned").unwrap().get.unwrap(), markup.find_property("CheckedReturned").unwrap().get.unwrap()] {
        reset(PanicAt::Never, 1);
        assert_eq!(get(&[]), Err(MarkupInvokeError::ArgumentCount { expected: 1, actual: 0 }));
        assert!(matches!(get(&[boxed(5i32)]), Err(MarkupInvokeError::Argument { index: 0, .. })));
        assert!(events().is_empty());
    }
    for property in markup.static_properties {
        reset(PanicAt::Never, 0);
        assert_eq!((property.get.unwrap())(&[None]), Err(MarkupInvokeError::ArgumentCount { expected: 0, actual: 1 }));
        assert!(events().is_empty());
    }
    for (current, original) in setter_cases() {
        for arguments in [vec![], vec![boxed(5i32), value.clone()], vec![receiver.clone(), boxed(5i32)]] {
            let expected = observe_setter(original, &arguments, PanicAt::Never, false);
            let actual = observe_setter(current, &arguments, PanicAt::Never, false);
            assert_eq!(actual, expected);
            if arguments.is_empty() {
                assert_eq!(actual, (
                    SetterOutcome::Error(MarkupInvokeError::ArgumentCount { expected: 2, actual: 0 }),
                    vec![],
                ));
            } else if arguments[0].as_ref().unwrap().as_any().is::<i32>() {
                assert!(matches!(actual.0, SetterOutcome::Error(MarkupInvokeError::Argument { index: 0, .. })));
                assert!(actual.1.is_empty());
            } else {
                assert!(matches!(actual.0, SetterOutcome::Error(MarkupInvokeError::Argument { index: 1, .. })));
                assert_eq!(actual.1, ["receiver clone", "receiver drop"]);
            }
        }
        let arguments = [receiver.clone(), value.clone()];
        let expected = observe_setter(original, &arguments, PanicAt::Never, false);
        let actual = observe_setter(current, &arguments, PanicAt::Never, false);
        assert_eq!(actual, expected);
        assert_eq!(actual.0, SetterOutcome::Success);
        assert_eq!(actual.1, [
            "receiver clone", "argument clone", "callable expression", "setter call",
            "argument drop", "ignored result drop", "callable drop", "receiver drop",
        ]);
    }
}

#[test]
fn setter_callable_errors_and_panics_keep_argument_cleanup_order() {
    let arguments = [boxed(InvocationReceiver { cloned: false }), boxed(SetterArgument { cloned: false })];
    for (current, original) in setter_cases() {
        for panic_at in [
            PanicAt::ArgumentClone, PanicAt::SetterExpression, PanicAt::SetterCall,
            PanicAt::CallableDrop, PanicAt::IgnoredResultDrop,
        ] {
            let expected = observe_setter(original, &arguments, panic_at, false);
            let actual = observe_setter(current, &arguments, panic_at, false);
            assert_eq!(actual, expected, "setter cleanup, {panic_at:?}");
            assert_eq!(actual.0, SetterOutcome::Panic);
        }
    }
    let (current, original) = setter_cases()[1];
    let expected = observe_setter(original, &arguments, PanicAt::Never, true);
    let actual = observe_setter(current, &arguments, PanicAt::Never, true);
    assert_eq!(actual, expected);
    assert_eq!(actual.0, SetterOutcome::Error(MarkupInvokeError::Failed("invocation failed".into())));
    assert_eq!(actual.1, [
        "receiver clone", "argument clone", "callable expression", "setter call", "argument drop",
        "error display", "error drop", "callable drop", "receiver drop",
    ]);
}
