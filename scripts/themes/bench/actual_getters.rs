//! Exercises the actual metadata function pointers exported by the linked Base archive.
use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::{Color, HslColor, HsvColor};
use ferroui_base::metadata::{
    MarkupInvoke, MarkupInvokeError, MarkupType, MarkupTyped, MarkupValue,
};
use ferroui_base::{
    AnyValue, BoxedValue, CornerRadius, Matrix, PixelPoint, PixelRect, PixelSize, PixelVector,
    Point, Rect, RelativePoint, RelativeRect, RelativeScalar, RelativeUnit, Size, Thickness,
    Vector, Vector3D,
};
use std::{any::type_name, hint::black_box, rc::Rc, time::Instant};

struct Sample {
    metadata: &'static MarkupType,
    type_name: &'static str,
    args: [MarkupValue; 1],
    getters: Vec<MarkupInvoke>,
}

fn sample<T: MarkupTyped + PartialEq + 'static>(value: T) -> Sample {
    let metadata = T::MARKUP;
    let getters = metadata
        .properties
        .iter()
        .map(|member| member.get.expect("readable member"))
        .collect();
    Sample {
        metadata,
        type_name: type_name::<T>(),
        args: [Some(Rc::new(value))],
        getters,
    }
}

fn samples() -> Vec<Sample> {
    vec![
        sample(Thickness::new(2., 3., 5., 7.)),
        sample(Point::new(3., -4.)),
        sample(Vector::new(6., 8.)),
        sample(Size::new(11., 13.)),
        sample(Rect::new(2., 3., 11., 13.)),
        sample(Matrix::new_3x3(2., 7., 1., 5., 11., 3., 17., 19., 4.)),
        sample(CornerRadius::new(2., 3., 5., 7.)),
        sample(RelativePoint::new(0.25, 0.875, RelativeUnit::Relative)),
        sample(RelativeRect::new(2., 3., 5., 7., RelativeUnit::Absolute)),
        sample(RelativeScalar::new(0.375, RelativeUnit::Relative)),
        sample(PixelPoint::new(-2, 5)),
        sample(PixelSize::new(11, 13)),
        sample(PixelRect::new(2, -3, 5, 7)),
        sample(PixelVector::new(-6, 8)),
        sample(Vector3D::new(2., -3., 5.)),
        sample(Color::new(211, 151, 83, 29)),
        sample(HslColor::new(0.875, 60., 0.75, 0.375)),
        sample(HsvColor::new(0.625, 210., 0.5, 0.875)),
    ]
}

fn display(value: &dyn AnyValue) -> String {
    macro_rules! try_types {
        ($($ty:ty),*) => {
            $(if let Some(typed) = value.downcast_ref::<$ty>() {
                return format!("{typed:?}");
            })*
        };
    }
    try_types!(
        f64,
        i32,
        u8,
        bool,
        Point,
        Size,
        Rect,
        PixelPoint,
        PixelSize,
        RelativeUnit
    );
    panic!("unexpected result type {}", value.type_name());
}

fn check(samples: &[Sample], wrong: &[MarkupValue; 1]) {
    let mut total = 0;
    for sample in samples {
        for (member, &get) in sample.metadata.properties.iter().zip(&sample.getters) {
            let result = get(&sample.args)
                .expect("get")
                .expect("non-null scalar or struct");
            assert_eq!(result.value_type_id(), (member.type_)().id());
            println!(
                "{}.{} = {}",
                sample.metadata.name,
                member.name,
                display(&*result)
            );
            for args in [&[][..], &[None, None][..]] {
                assert_eq!(
                    get(args),
                    Err(MarkupInvokeError::ArgumentCount {
                        expected: 1,
                        actual: args.len()
                    })
                );
            }
            for (args, actual) in [
                (wrong.as_slice(), type_name::<String>()),
                (&[None][..], "null"),
            ] {
                assert_eq!(
                    get(args),
                    Err(MarkupInvokeError::Argument {
                        index: 0,
                        expected: sample.type_name,
                        actual: actual.to_owned(),
                    })
                );
            }
            total += 1;
        }
    }
    assert_eq!((samples.len(), total), (18, 88));
    #[derive(PartialEq)]
    struct ExternalPoint(Point);
    ValueTypes::register_cast::<ExternalPoint, Point>(|foreign| foreign.0);
    let point: BoxedValue = Rc::new(ExternalPoint(Point::new(23., 41.)));
    let get = Point::MARKUP.find_property("Y").unwrap().get.unwrap();
    assert_eq!(
        *get(&[Some(point)])
            .unwrap()
            .unwrap()
            .downcast_ref::<f64>()
            .unwrap(),
        41.
    );
}

#[repr(C)]
struct Timespec {
    sec: i64,
    nsec: i64,
}
unsafe extern "C" {
    fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
}
fn cpu_nanos() -> u64 {
    let mut time = Timespec { sec: 0, nsec: 0 };
    // Linux CLOCK_THREAD_CPUTIME_ID measures the timed thread only.
    assert_eq!(unsafe { clock_gettime(3, &mut time) }, 0);
    (time.sec as u64) * 1_000_000_000 + time.nsec as u64
}

fn main() {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "check".to_owned());
    let samples = samples();
    let wrong: [MarkupValue; 1] = [Some(Rc::new(String::from("unconvertible")))];
    if mode == "check" {
        check(&samples, &wrong);
        return;
    }
    let reps = match mode.as_str() {
        "success" => 5000,
        "wrong-type" => 1500,
        "wrong-arity" => 35000,
        _ => panic!("unknown mode"),
    };
    let cases: Vec<_> = samples
        .iter()
        .map(|sample| {
            let args = match mode.as_str() {
                "success" => sample.args.as_slice(),
                "wrong-type" => wrong.as_slice(),
                _ => &[],
            };
            (args, sample.getters.as_slice())
        })
        .collect();
    let start = Instant::now();
    let cpu = cpu_nanos();
    for _ in 0..reps {
        for &(args, getters) in &cases {
            for &get in getters {
                drop(black_box(get(black_box(args))));
            }
        }
    }
    let elapsed = cpu_nanos() - cpu;
    println!(
        "{mode} {} {elapsed} {}",
        88 * reps,
        start.elapsed().as_nanos()
    );
}
