// Each cohort entry carries the public descriptor and calls a distinct typed handler.
use ferroui_base::animation::Animatable;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
#[cfg(shared)]
use ferroui_base::metadata::MarkupArguments;
use ferroui_base::metadata::{MarkupInvokeError, MarkupMethod, MarkupValue};
use ferroui_base::{BoxedValue, FerroObject, ObjectType, Ref, StyledElement};
use std::{cell::Cell, hint::black_box, rc::Rc, time::Instant};

type Event = Rc<dyn IRoutedEventArgs>;
thread_local! { static SUM: Cell<usize> = const { Cell::new(0) }; }

#[cfg(shared)]
#[inline(never)]
fn invoke_handler<T: ObjectType>(
    values: &[MarkupValue],
    handler: fn(&Ref<T>, Option<BoxedValue>, Event),
) -> Result<MarkupValue, MarkupInvokeError> {
    let mut args = MarkupArguments::new(values, 3)?;
    let receiver = ferroui_base::__ferro_markup_receiver!(Ref<T>, args)?;
    handler(&receiver, args.next()?, args.next()?);
    Ok(None)
}

fn handle<T: ObjectType, const ID: usize>(
    receiver: &Ref<T>,
    sender: Option<BoxedValue>,
    event: Event,
) {
    black_box(receiver);
    let increment =
        ID + sender.is_some() as usize + event.as_routed_event_args().handled() as usize;
    SUM.with(|sum| sum.set(sum.get().wrapping_add(increment)));
}

macro_rules! method {
    ($owner:ty, $name:ident, $id:literal) => {{
        let native = ferroui_base::__ferro_markup_method!(
            N Ref<$owner>; [] [] $name (Option<BoxedValue>, Event) () (handle::<$owner, $id>) []
        );
        #[cfg(shared)]
        let native = MarkupMethod {
            invoke: |values| invoke_handler::<$owner>(values, handle::<$owner, $id>),
            ..native
        };
        native
    }}
}
macro_rules! methods {
    ($owner:ty) => {
        [
            method!($owner, OnFirst, 1),
            method!($owner, OnSecond, 2),
            method!($owner, OnThird, 3),
            method!($owner, OnFourth, 4),
            method!($owner, OnFifth, 5),
            method!($owner, OnSixth, 6),
            method!($owner, OnSeventh, 7),
            method!($owner, OnEighth, 8),
        ]
    };
}
const METHODS: [[MarkupMethod; 8]; 4] = [
    methods!(FerroObject),
    methods!(Animatable),
    methods!(StyledElement),
    methods!(Interactive),
];
fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    Some(Rc::new(value))
}
fn main() {
    let owners = [
        boxed(FerroObject::new()),
        boxed(Animatable::new()),
        boxed(StyledElement::new()),
        boxed(Interactive::new()),
    ];
    let owner_names = [
        std::any::type_name::<Ref<FerroObject>>(),
        std::any::type_name::<Ref<Animatable>>(),
        std::any::type_name::<Ref<StyledElement>>(),
        std::any::type_name::<Ref<Interactive>>(),
    ];
    let event: Event = Rc::new(RoutedEventArgs::new());
    let event_value = boxed(event.clone());
    let rows = black_box(&METHODS);
    let mut expected = 0;
    for (index, group) in rows.iter().enumerate() {
        for (i, method) in group.iter().enumerate() {
            let member = method.invoke;
            assert!(
                !method.is_static && method.return_type.is_none() && method.parameters.len() == 2
            );
            assert!(matches!(
                member(&[]),
                Err(MarkupInvokeError::ArgumentCount {
                    expected: 3,
                    actual: 0
                })
            ));
            assert_eq!(
                member(&[boxed(42_i32), None, event_value.clone()]).unwrap_err(),
                MarkupInvokeError::Argument {
                    index: 0,
                    expected: owner_names[index],
                    actual: "i32".to_owned()
                }
            );
            assert_eq!(
                member(&[owners[index].clone(), None, boxed(42_i32)]).unwrap_err(),
                MarkupInvokeError::Argument {
                    index: 2,
                    expected: std::any::type_name::<Event>(),
                    actual: "i32".to_owned()
                }
            );
            for sender in [None, boxed(42_i32)] {
                expected += i + 1 + sender.is_some() as usize;
                assert!(
                    member(&[owners[index].clone(), sender, event_value.clone()])
                        .unwrap()
                        .is_none()
                );
            }
        }
    }
    ValueTypes::register_upcast::<StyledElement, FerroObject>();
    // Use a registered receiver conversion and an event conversion on the same call.
    ValueTypes::register_cast::<RoutedEventArgs, Event>(|event| Rc::new(event.clone()));
    assert!(
        (rows[0][0].invoke)(&[owners[2].clone(), None, boxed(RoutedEventArgs::new())])
            .unwrap()
            .is_none()
    );
    assert_eq!(SUM.with(Cell::get), expected + 1);
    assert_eq!(Rc::strong_count(&event), 2);

    // Alternate the eight public invokers on one real framework receiver.
    let args = [owners[2].clone(), boxed(7_u8), event_value];
    let calls = 1_000_000;
    for i in 0..100_000 {
        black_box((rows[2][i % 8].invoke)(black_box(&args)).unwrap());
    }
    let start = Instant::now();
    for i in 0..calls {
        black_box((rows[2][i % 8].invoke)(black_box(&args)).unwrap());
    }
    let elapsed = start.elapsed().as_nanos();
    assert_eq!(
        SUM.with(Cell::get),
        expected + 1 + (calls + 100_000) / 8 * 44
    );
    println!("{elapsed}");
}
