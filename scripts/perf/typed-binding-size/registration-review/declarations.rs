// Check the declaration parser and the exact public metadata invokers from a built Catalog Base.
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::metadata::{MarkupInvokeError, MarkupMethod, MarkupValue};
use ferroui_base::{BoxedValue, FerroObject, Ref, StyledElement};
use std::{cell::Cell, rc::Rc};

type Sender = Option<BoxedValue>;
type Event = Rc<dyn IRoutedEventArgs>;
thread_local! { static VALUE: Cell<i32> = const { Cell::new(0) }; }

fn mark(_this: &Ref<FerroObject>, sender: Sender, event: Event) {
    VALUE.with(|value| value.set(1 + i32::from(sender.is_some()) + i32::from(event.handled())));
}

const METHODS: &[MarkupMethod] = ferroui_base::__ferro_markup_methods!([A B C D E F G] Ref<FerroObject>; []
    fn Eligible(Sender, Event) => |this: &Ref<FerroObject>, sender: Sender, event: Event| mark(this, sender, event),
    fn Different(i32, i32) => |_this: &Ref<FerroObject>, a: i32, b: i32| VALUE.with(|v| v.set(a + b)),
    try fn Checked(Sender, Event) => |_this: &Ref<FerroObject>, _a: Sender, _b: Event| Err::<(), _>("error"),
    fn Returns(Sender, Event) -> i32 => |_this: &Ref<FerroObject>, _a: Sender, _b: Event| 18,
    static fn Static(Sender, Event) -> i32 => |_a: Sender, _b: Event| 19,
    fn Attributed(Sender, Event) => mark [DependsOn("Field")],
    fn CallableExpression(Sender, Event) => { VALUE.with(|v| v.set(45)); mark }
);

#[derive(PartialEq)]
struct Owned;
impl Clone for Owned {
    fn clone(&self) -> Self {
        VALUE.with(|v| v.set(v.get() + 10));
        Self
    }
}
const OWNED: &[MarkupMethod] = ferroui_base::__ferro_markup_methods!([A] Owned; []
    fn OwnedReceiver(Sender, Event) => |_this: &Owned, _a: Sender, _b: Event| VALUE.with(|v| v.set(v.get()+1))
);

fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    Some(Rc::new(value))
}

fn main() {
    let obj = boxed(FerroObject::new());
    let event: Event = Rc::new(RoutedEventArgs::new());
    let event_arg = boxed(event.clone());
    let ok = [obj.clone(), None, event_arg.clone()];
    assert!(METHODS.iter().map(|m| m.name).eq([
        "Eligible",
        "Different",
        "Checked",
        "Returns",
        "Static",
        "Attributed",
        "CallableExpression"
    ]));
    let invoke = METHODS[0].invoke;
    assert!(invoke(&ok).unwrap().is_none());
    assert_eq!(VALUE.with(Cell::get), 1);
    let wrong_count = invoke(&[]).unwrap_err();
    assert_eq!(
        wrong_count,
        MarkupInvokeError::ArgumentCount {
            expected: 3,
            actual: 0
        }
    );
    let invalid = [obj.clone(), None, boxed(1_i32)];
    assert_eq!(
        invoke(&invalid).unwrap_err(),
        MarkupInvokeError::Argument {
            index: 2,
            expected: std::any::type_name::<Event>(),
            actual: "i32".into()
        }
    );
    assert_eq!(
        invoke(&[boxed(1_i32), None, boxed(1_i32)]).unwrap_err(),
        MarkupInvokeError::Argument {
            index: 0,
            expected: std::any::type_name::<Ref<FerroObject>>(),
            actual: "i32".into()
        }
    );
    assert_eq!(VALUE.with(Cell::get), 1);
    ValueTypes::register_upcast::<StyledElement, FerroObject>();
    ValueTypes::register_cast::<RoutedEventArgs, Event>(|e| Rc::new(e.clone()));
    assert!(invoke(&[
        boxed(StyledElement::new()),
        boxed(4_i32),
        boxed(RoutedEventArgs::new())
    ])
    .unwrap()
    .is_none());
    assert_eq!(VALUE.with(Cell::get), 2);
    assert!((METHODS[1].invoke)(&[obj.clone(), boxed(12), boxed(3)])
        .unwrap()
        .is_none());
    assert_eq!(VALUE.with(Cell::get), 15);
    assert_eq!(
        (METHODS[2].invoke)(&ok).unwrap_err(),
        MarkupInvokeError::Failed("error".into())
    );
    assert_eq!((METHODS[3].invoke)(&ok).unwrap(), boxed(18_i32));
    assert_eq!((METHODS[4].invoke)(&ok[1..]).unwrap(), boxed(19_i32));
    assert!(METHODS[4].is_static);
    assert_eq!(METHODS[5].attributes.len(), 1);
    assert!((METHODS[5].invoke)(&ok).unwrap().is_none());
    // The original callable expression runs before decoding the bad event argument.
    assert!((METHODS[6].invoke)(&invalid).is_err());
    assert_eq!(VALUE.with(Cell::get), 45);
    assert!((OWNED[0].invoke)(&[boxed(Owned), None, event_arg.clone()])
        .unwrap()
        .is_none());
    assert_eq!(VALUE.with(Cell::get), 56);
    drop(event_arg);
    drop(ok);
    assert_eq!(Rc::strong_count(&event), 1);
}
