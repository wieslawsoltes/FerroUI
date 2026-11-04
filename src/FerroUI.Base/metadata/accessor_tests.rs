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

#[derive(Clone, Copy, PartialEq, Eq)]
enum PanicAt {
    Never,
    ArgumentClone,
    SetterExpression,
    SetterCall,
    ResultClone,
    CallableDrop,
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

fn setters() -> [MarkupInvoke; 2] {
    let markup = <InvocationReceiver as MarkupTyped>::MARKUP;
    [markup.find_property("Value").unwrap().set.unwrap(), markup.find_property("CheckedValue").unwrap().set.unwrap()]
}

// Keep the original getter expressions as an oracle for the exceptional
// return-place cleanup when the callable's destructor itself panics. These
// use public APIs and the original owned receiver path, not adapter helpers.
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

#[test]
fn getter_callable_lives_through_result_conversion_and_unwind() {
    ValueTypes::register_nullable::<Returned>();
    let markup = <InvocationReceiver as MarkupTyped>::MARKUP;
    let receiver = boxed(InvocationReceiver { cloned: false });
    let getters = [
        (true, markup.find_property("Returned").unwrap().get.unwrap(), original_getter as MarkupInvoke),
        (true, markup.find_property("CheckedReturned").unwrap().get.unwrap(), original_checked_getter as MarkupInvoke),
        (false, markup.static_properties[0].get.unwrap(), original_static_getter as MarkupInvoke),
        (false, markup.static_properties[1].get.unwrap(), original_checked_static_getter as MarkupInvoke),
    ];
    for (instance, get, original) in getters {
        let arguments = if instance { vec![receiver.clone()] } else { Vec::new() };
        for panic_result in [false, true] {
            reset(if panic_result { PanicAt::ResultClone } else { PanicAt::Never }, usize::from(instance));
            let outcome = catch_unwind(AssertUnwindSafe(|| get(&arguments)));
            let mut expected = if instance { vec!["receiver clone"] } else { Vec::new() };
            expected.extend(["callable expression", "getter call", "result clone", "result original drop", "callable drop"]);
            if instance { expected.push("receiver drop"); }
            assert_eq!(events(), expected);
            if panic_result {
                assert!(outcome.is_err());
            } else {
                let result = outcome.unwrap().unwrap();
                assert!(result.as_ref().unwrap().as_any().is::<Returned>());
                drop(result);
                expected.push("result clone drop");
                assert_eq!(events(), expected);
            }
            assert_eq!(LIVE_RECEIVERS.with(Cell::get), 0);
            assert_eq!(LIVE_CALLABLES.with(Cell::get), 0);
        }
        reset(PanicAt::CallableDrop, usize::from(instance));
        assert!(catch_unwind(AssertUnwindSafe(|| original(&arguments))).is_err());
        let original_cleanup = events();
        reset(PanicAt::CallableDrop, usize::from(instance));
        assert!(catch_unwind(AssertUnwindSafe(|| get(&arguments))).is_err());
        assert_eq!(events(), original_cleanup);
        assert_eq!(LIVE_RECEIVERS.with(Cell::get), 0);
        assert_eq!(LIVE_CALLABLES.with(Cell::get), 0);
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
    for set in setters() {
        reset(PanicAt::Never, 1);
        assert_eq!(set(&[]), Err(MarkupInvokeError::ArgumentCount { expected: 2, actual: 0 }));
        assert!(matches!(set(&[boxed(5i32), value.clone()]), Err(MarkupInvokeError::Argument { index: 0, .. })));
        assert!(events().is_empty());
        assert!(matches!(set(&[receiver.clone(), boxed(5i32)]), Err(MarkupInvokeError::Argument { index: 1, .. })));
        assert_eq!(events(), ["receiver clone", "receiver drop"]);

        reset(PanicAt::Never, 1);
        assert_eq!(set(&[receiver.clone(), value.clone()]).unwrap(), None);
        assert_eq!(events(), [
            "receiver clone", "argument clone", "callable expression", "setter call",
            "argument drop", "ignored result drop", "callable drop", "receiver drop",
        ]);
    }
}

#[test]
fn setter_callable_errors_and_panics_keep_argument_cleanup_order() {
    let arguments = [boxed(InvocationReceiver { cloned: false }), boxed(SetterArgument { cloned: false })];
    for set in setters() {
        for (panic_at, expected) in [
            (PanicAt::ArgumentClone, vec!["receiver clone", "argument clone", "receiver drop"]),
            (PanicAt::SetterExpression, vec![
                "receiver clone", "argument clone", "callable expression", "argument drop", "receiver drop",
            ]),
            (PanicAt::SetterCall, vec![
                "receiver clone", "argument clone", "callable expression", "setter call",
                "argument drop", "callable drop", "receiver drop",
            ]),
        ] {
            reset(panic_at, 1);
            assert!(catch_unwind(AssertUnwindSafe(|| set(&arguments))).is_err());
            assert_eq!(events(), expected);
            assert_eq!(LIVE_RECEIVERS.with(Cell::get), 0);
            assert_eq!(LIVE_CALLABLES.with(Cell::get), 0);
        }
    }
    reset(PanicAt::Never, 1);
    SETTER_ERROR.with(|value| value.set(true));
    assert_eq!(setters()[1](&arguments), Err(MarkupInvokeError::Failed("invocation failed".into())));
    assert_eq!(events(), [
        "receiver clone", "argument clone", "callable expression", "setter call", "argument drop",
        "error display", "error drop", "callable drop", "receiver drop",
    ]);
}
