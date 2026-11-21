use super::{
    MarkupDelegate, MarkupDelegateType, MarkupInvokeError, MarkupTyped, SYSTEM_ACTION_DELEGATE,
};
use crate::BoxedValue;
use crate::data::core::{ValueType, ValueTypes};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

static CUSTOM: MarkupDelegateType = MarkupDelegateType::new("CustomVoid", &[], None);
static SAME_NAME: MarkupDelegateType = MarkupDelegateType::new("System.Action", &[], None);
static FUNC: MarkupDelegateType = MarkupDelegateType::new(
    "System.Func`1[System.Boolean]",
    &[],
    Some(ValueType::of::<bool>),
);
static ARG: MarkupDelegateType = MarkupDelegateType::new(
    "System.Action`1[System.Int32]",
    &[ValueType::of::<i32>],
    None,
);
struct Target(Cell<i32>);
impl PartialEq for Target {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
crate::ferro_markup_type!(class Target {this:Rc<Target>,handles:[Rc<Target>],methods:[
 fn Run()=>|v:&Rc<Target>|v.0.set(v.0.get()+1),
 fn Result()->bool =>|v:&Rc<Target>|{v.0.set(v.0.get()+1);true},
 fn Arg(i32)=>|v:&Rc<Target>,n|v.0.set(v.0.get()+n),
 static try fn Fail()=>||Err::<(),_>("typed-failed"),
 static fn Static()=>||{},
 fn Unit()->() =>|_:&Rc<Target>|(),
]});
fn bound(kind: &'static MarkupDelegateType, t: &Rc<Target>, name: &str) -> MarkupDelegate {
    ValueTypes::register_reference::<Target>();
    MarkupDelegate::for_typed_method(
        kind,
        Some(t.clone()),
        Target::MARKUP,
        Target::MARKUP.find_methods(name).next().unwrap(),
    )
    .unwrap()
}
#[test]
fn nominal_identity_is_not_inferred_from_signature_or_display_name() {
    let t = Rc::new(Target(Cell::new(0)));
    let a = bound(&SYSTEM_ACTION_DELEGATE, &t, "Run");
    let custom = bound(&CUSTOM, &t, "Run");
    let same_name = bound(&SAME_NAME, &t, "Run");
    let untyped = MarkupDelegate::for_method(Some(t.clone()), Target::MARKUP, a.method().unwrap());
    assert!(a.is_of_type(&SYSTEM_ACTION_DELEGATE));
    assert!(!custom.is_of_type(&SYSTEM_ACTION_DELEGATE));
    assert!(!same_name.is_of_type(&SYSTEM_ACTION_DELEGATE));
    assert!(untyped.delegate_type().is_none());
    assert_ne!(a, custom);
    assert_ne!(a, same_name);
    assert_ne!(a, untyped);
    assert_eq!(a, bound(&SYSTEM_ACTION_DELEGATE, &t, "Run"));
    assert!(!a.ptr_eq(&bound(&SYSTEM_ACTION_DELEGATE, &t, "Run")));
    assert!(a.ptr_eq(&a.clone()));
    assert_eq!(custom.to_string(), "CustomVoid");
    assert_eq!(same_name.to_string(), "System.Action");
    assert_eq!(t.0.get(), 0);
}
#[test]
fn typed_binding_rejects_incompatible_signatures_and_targets_without_invocation() {
    ValueTypes::register_reference::<Target>();
    let t = Rc::new(Target(Cell::new(0)));
    for name in ["Result", "Arg"] {
        assert!(
            MarkupDelegate::for_action_method(
                Some(t.clone()),
                Target::MARKUP,
                Target::MARKUP.find_methods(name).next().unwrap()
            )
            .is_err()
        );
    }
    let unit = Target::MARKUP.find_methods("Unit").next().unwrap();
    MarkupDelegate::for_action_method(Some(t.clone()), Target::MARKUP, unit)
        .unwrap()
        .try_invoke(&[])
        .unwrap();
    let run = Target::MARKUP.find_methods("Run").next().unwrap();
    assert!(MarkupDelegate::for_action_method(None, Target::MARKUP, run).is_err());
    assert!(MarkupDelegate::for_action_method(Some(Rc::new(7i32)), Target::MARKUP, run).is_err());
    assert_eq!(t.0.get(), 0);
    assert_eq!(
        bound(&FUNC, &t, "Result")
            .try_invoke(&[])
            .unwrap()
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&true)
    );
    let arg = bound(&ARG, &t, "Arg");
    assert!(matches!(
        arg.try_invoke(&[]),
        Err(MarkupInvokeError::ArgumentCount {
            expected: 1,
            actual: 0
        })
    ));
    assert!(matches!(
        arg.try_invoke(&[Some(Rc::new("wrong".to_string()))]),
        Err(MarkupInvokeError::Argument { index: 0, .. })
    ));
    assert_eq!(t.0.get(), 1);
    arg.try_invoke(&[Some(Rc::new(3i32))]).unwrap();
    assert_eq!(t.0.get(), 4);
}
#[test]
fn multicast_uses_nominal_compatibility_last_subsequence_and_error_order() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let make = |n| {
        let trace = trace.clone();
        MarkupDelegate::action(move || trace.borrow_mut().push(n))
    };
    let a = make("a");
    let b = make("b");
    let pair = MarkupDelegate::combine(Some(&a), Some(&b)).unwrap();
    let twice = MarkupDelegate::combine(Some(&pair), Some(&pair)).unwrap();
    assert!(twice.is_of_type(&SYSTEM_ACTION_DELEGATE));
    assert_eq!(
        MarkupDelegate::remove(Some(&twice), Some(&pair)).unwrap(),
        pair
    );
    assert!(MarkupDelegate::remove(Some(&pair), Some(&pair)).is_none());
    let custom = MarkupDelegate::new_typed(&CUSTOM, |_| None);
    for op in [0, 1] {
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| if op == 0 {
                MarkupDelegate::combine(Some(&a), Some(&custom))
            } else {
                MarkupDelegate::remove(Some(&a), Some(&custom))
            }))
            .is_err()
        );
    }
    assert!(
        MarkupDelegate::combine(None, Some(&custom))
            .unwrap()
            .ptr_eq(&custom)
    );
    assert!(MarkupDelegate::remove(None, Some(&custom)).is_none());
    let fail = MarkupDelegate::for_action_method(
        None,
        Target::MARKUP,
        Target::MARKUP.find_methods("Fail").next().unwrap(),
    )
    .unwrap();
    let before = MarkupDelegate::combine(Some(&a), Some(&fail)).unwrap();
    let all = MarkupDelegate::combine(Some(&before), Some(&b)).unwrap();
    assert_eq!(
        all.try_invoke(&[]),
        Err(MarkupInvokeError::Failed("typed-failed".into()))
    );
    assert_eq!(&*trace.borrow(), &["a"]);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| all.invoke(&[]))).is_err());
    assert_eq!(&*trace.borrow(), &["a", "a"]);
}
#[test]
fn typed_methods_and_multicast_retain_targets_until_last_handle_drops() {
    let t = Rc::new(Target(Cell::new(0)));
    let weak = Rc::downgrade(&t);
    let a = bound(&SYSTEM_ACTION_DELEGATE, &t, "Run");
    let b = bound(&SYSTEM_ACTION_DELEGATE, &t, "Run");
    let list = MarkupDelegate::combine(Some(&a), Some(&b)).unwrap();
    drop(t);
    drop(a);
    drop(b);
    assert!(weak.upgrade().is_some());
    assert!(MarkupDelegate::remove(Some(&list), Some(&list)).is_none());
    list.try_invoke(&[]).unwrap();
    assert_eq!(weak.upgrade().unwrap().0.get(), 2);
    drop(list);
    assert!(weak.upgrade().is_none());
}
#[test]
fn callback_contract_failure_is_explicit_and_arguments_checked_before_call() {
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let action = MarkupDelegate::action(move || seen.set(seen.get() + 1));
    assert!(action.try_invoke(&[None]).is_err());
    assert_eq!(count.get(), 0);
    action.try_invoke(&[]).unwrap();
    assert_eq!(count.get(), 1);
    let invalid = MarkupDelegate::new_typed(&SYSTEM_ACTION_DELEGATE, |_| {
        Some(Rc::new(true) as BoxedValue)
    });
    assert!(matches!(
        invalid.try_invoke(&[]),
        Err(MarkupInvokeError::Failed(_))
    ));
}
