//! Generated theme selector candidate probes, also buildable against main.
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::diagnostics::perf_counters::{self, PerfCounter};
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::{ControlTheme, Style};
use ferroui_base::{FerroProperty, Ref};
use ferroui_controls::{Border, Button};
use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;
use serde_json::{json, Value};
use std::hint::black_box;
use std::rc::Rc;
use std::time::Instant;

fn width() -> &'static FerroProperty {
    Layoutable::width_property()
}
static DOCUMENT: rt::DocumentInfo = rt::DocumentInfo {
    base_uri: Some("ferres://SelectorBenchmarks/Theme.xaml"),
    namespaces: &[],
};

fn measure<T>(phase: &str, output: &mut Vec<Value>, work: impl FnOnce() -> T) -> T {
    let before = allocations::snapshot();
    let counters = perf_counters::snapshot();
    let start = Instant::now();
    let value = black_box(work());
    let elapsed = start.elapsed().as_secs_f64() * 1e6;
    let counted = perf_counters::snapshot().since(&counters);
    let after = allocations::snapshot();
    output.push(json!({"phase":phase,"elapsed_us":elapsed,
        "styles_evaluated": if perf_counters::ENABLED { Some(counted.get(PerfCounter::StylesEvaluated)) } else { None },
        "allocations": if cfg!(feature="count-allocations") { Some(after.0-before.0) } else { None },
        "allocated_bytes": if cfg!(feature="count-allocations") { Some(after.1-before.1) } else { None }}));
    value
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let count: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(1000);
    let matching_percent: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(10);
    let mode = args.get(3).map(String::as_str).unwrap_or("typed");
    let elements: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(1000);
    assert!(matching_percent <= 100 && elements > 0);
    assert!(["typed", "dynamic", "fallback"].contains(&mode));
    let mut expected = None;
    let mut members = vec![rt::StyleMember::TargetType(Button::TYPE)];
    for i in 0..count {
        let matches = i * 100 / count < matching_percent;
        let mut steps = vec![rt::SelectorStep::Nesting, rt::SelectorStep::Template];
        if mode != "fallback" {
            steps.push(rt::SelectorStep::OfType(if matches {
                Border::TYPE
            } else {
                Button::TYPE
            }));
        }
        if mode != "typed" {
            steps.push(rt::SelectorStep::Class(":active"));
        }
        let value = i as f64 + 1.0;
        if matches || mode == "fallback" {
            expected = Some(value);
        }
        members.push(rt::StyleMember::Style {
            parent: false,
            selector: Some(Box::leak(steps.into_boxed_slice())),
            members: Box::leak(
                vec![rt::StyleMember::Setter(rt::ThemeSetter {
                    property: width,
                    form: rt::SetterForm::Plain,
                    value: rt::ThemeValue::Double(value),
                })]
                .into_boxed_slice(),
            ),
            line: 1,
            position: 1,
        });
    }
    let table = rt::ControlThemeTable {
        parent: false,
        members: Box::leak(members.into_boxed_slice()),
    };
    let dictionary = ResourceDictionary::new();
    let context = rt::populate_context(None, &DOCUMENT, rt::to_value(dictionary));
    let mut samples = Vec::new();
    black_box(allocations::snapshot());
    let theme = measure("build_theme", &mut samples, || {
        let value = rt::build_control_theme(&context, &table).unwrap().unwrap();
        ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap()
    });
    let parent = Button::new();
    parent.set_theme(theme.clone());
    let make = || {
        let target = Border::new();
        target.set_templated_parent(parent.clone());
        target
    };
    let verify = |target: &Ref<Border>, active: bool, expected: Option<f64>| {
        let expected = if mode == "typed" || active {
            expected
        } else {
            None
        };
        match expected {
            Some(value) => assert_eq!(target.width(), value),
            None => assert!(target.width().is_nan()),
        }
    };
    let first = make();
    measure("first_attach", &mut samples, || {
        first.apply_styling();
    });
    verify(&first, false, expected);
    let targets: Vec<_> = (0..elements).map(|_| make()).collect();
    measure("attach_many", &mut samples, || {
        for target in &targets {
            black_box(target.apply_styling());
        }
    });
    for target in &targets {
        verify(target, false, expected);
    }
    if mode != "typed" {
        measure("activate_deactivate_100", &mut samples, || {
            for _ in 0..100 {
                first.pseudo_classes().add_pseudo(":active");
                verify(&first, true, expected);
                first.pseudo_classes().remove_pseudo(":active");
                verify(&first, false, expected);
            }
        });
    }
    // Simulate the selector replacement used by hot reload after the cache is warm.
    if count > 0 {
        let entry = theme.children().get(count - 1);
        let style = entry.as_object().unwrap().to_ref().cast::<Style>().unwrap();
        Style::__markup_set_Selector(
            &style,
            rt::build_selector(&[
                rt::SelectorStep::Nesting,
                rt::SelectorStep::Template,
                rt::SelectorStep::OfType(Border::TYPE),
            ]),
        );
        let changed = make();
        measure("attach_after_selector_replacement", &mut samples, || {
            changed.apply_styling();
        });
        assert_eq!(changed.width(), count as f64);
    }
    println!(
        "{}",
        json!({"rules":count,"matching_percent":matching_percent,"mode":mode,"elements":elements,
        "counted":cfg!(feature="count-allocations"),"samples":samples})
    );
    black_box(Rc::strong_count(&context));
}

mod allocations {
    #[cfg(not(feature = "count-allocations"))]
    pub fn snapshot() -> (u64, u64) {
        (0, 0)
    }
    #[cfg(feature = "count-allocations")]
    mod counted {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::cell::Cell;
        thread_local! { static COUNTS: Cell<(u64,u64)> = const { Cell::new((0,0)) }; }
        pub fn snapshot() -> (u64, u64) {
            COUNTS.try_with(Cell::get).unwrap_or((0, 0))
        }
        fn add(bytes: usize) {
            let _ = COUNTS.try_with(|counts| {
                let (n, b) = counts.get();
                counts.set((n + 1, b + bytes as u64));
            });
        }
        struct Allocator;
        // SAFETY: arguments are forwarded unchanged to System. TLS counters do
        // not allocate and tolerate thread teardown.
        unsafe impl GlobalAlloc for Allocator {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                add(layout.size());
                unsafe { System.alloc(layout) }
            }
            unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
                add(layout.size());
                unsafe { System.alloc_zeroed(layout) }
            }
            unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
                unsafe { System.dealloc(p, layout) }
            }
            unsafe fn realloc(&self, p: *mut u8, layout: Layout, n: usize) -> *mut u8 {
                add(n.saturating_sub(layout.size()));
                unsafe { System.realloc(p, layout, n) }
            }
        }
        #[global_allocator]
        static ALLOCATOR: Allocator = Allocator;
    }
    #[cfg(feature = "count-allocations")]
    pub use counted::snapshot;
}
