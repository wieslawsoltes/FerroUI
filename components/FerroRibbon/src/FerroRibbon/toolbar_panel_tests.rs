use super::*;
use crate::tests::run;
use ferroui_base::{input::KeyboardNavigation, layout::LayoutableImplExt};
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use serde_json::{Value, json};
#[repr(C)]
struct Fixed {
    base: TemplatedControl,
    size: Cell<Size>,
    after_arrange: RefCell<Option<Rc<dyn Fn()>>>,
    after_measure: RefCell<Option<Rc<dyn Fn()>>>,
    assigned: Cell<Rect>,
}
ferro_class!(Fixed:TemplatedControl);
ferro_impl_classes!(Fixed:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl LayoutableImpl for Fixed {
    fn measure_override(this: &Self, _: Size) -> Size {
        let callback = this.after_measure.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        this.size.get()
    }
    fn arrange_core(this: &Self, rect: Rect) {
        this.assigned.set(rect);
        Self::parent_arrange_core(this, rect);
        let callback = this.after_arrange.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}
impl Fixed {
    fn new(name: &str, size: Size) -> Ref<Self> {
        let v = instantiate(Self {
            base: TemplatedControl::construct(),
            size: Cell::new(size),
            after_arrange: RefCell::default(),
            after_measure: RefCell::default(),
            assigned: Cell::default(),
        });
        init(&v, name);
        v
    }
}
#[repr(C)]
struct FixedSeparator {
    base: RibbonSeparator,
    size: Size,
    assigned: Cell<Rect>,
}
ferro_class!(FixedSeparator:RibbonSeparator);
ferro_impl_classes!(FixedSeparator:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,crate::controls::RibbonControlBaseImpl);
impl LayoutableImpl for FixedSeparator {
    fn measure_override(this: &Self, _: Size) -> Size {
        this.size
    }
    fn arrange_core(this: &Self, rect: Rect) {
        this.assigned.set(rect);
        Self::parent_arrange_core(this, rect);
    }
}
impl FixedSeparator {
    fn new() -> Ref<Self> {
        let v = instantiate(Self {
            base: RibbonSeparator::construct(),
            size: Size::new(2., 3.),
            assigned: Cell::default(),
        });
        init(&v, "sep");
        v
    }
}
fn init(c: &Control, name: &str) {
    c.set_use_layout_rounding(false);
    c.set_name(Some(name.into()));
    KeyboardNavigation::set_is_tab_stop(c, true);
}
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/toolbar-panel.json")).unwrap()
}
fn assert_dimensions(actual: &[f64], expected: &Value, context: &str) {
    let expected: Vec<f64> = expected
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(actual, expected, "{context}");
}
fn overflow(p: &RibbonToolBarPanel) -> Vec<String> {
    p.overflow_items()
        .iter()
        .map(|c| c.unwrap().name().unwrap())
        .collect()
}
fn assigned(c: &Ref<Control>) -> Rect {
    if let Some(c) = c.cast::<Fixed>() {
        c.assigned.get()
    } else {
        c.cast::<FixedSeparator>().unwrap().assigned.get()
    }
}
#[test]
fn toolbar_panel_original_layout_overflow_and_focus_corpus() {
    run(|| {
        for case in corpus()["corpus"]["cases"].as_array().unwrap() {
            let p = RibbonToolBarPanel::new();
            p.set_orientation(Orientation::from_raw(
                case["orientation"].as_i64().unwrap() as i32
            ));
            p.set_columns(case["columns"].as_i64().unwrap() as i32);
            let a = Fixed::new("a", Size::new(29.25, 17.5));
            let b = Fixed::new("b", Size::new(41., 23.));
            let sep = FixedSeparator::new();
            let c = Fixed::new("c", Size::new(19.75, 14.25));
            let hidden = Fixed::new("hidden", Size::new(400., 400.));
            hidden.set_is_visible(false);
            for c in [
                a.upcast::<Control>(),
                b.upcast(),
                sep.upcast(),
                c.upcast(),
                hidden.upcast(),
            ] {
                p.children().add(c);
            }
            p.set_overflow_button(Some(Fixed::new("more", Size::new(12.5, 11.25)).upcast()));
            let held = p.overflow_items();
            let limit = case["limit"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap_or(f64::INFINITY);
            let available = if p.orientation() == Orientation::Horizontal {
                Size::new(limit, 60.)
            } else {
                Size::new(140., limit)
            };
            let desired = <RibbonToolBarPanel as LayoutableImpl>::measure_override(&p, available);
            <RibbonToolBarPanel as LayoutableImpl>::arrange_override(&p, Size::new(140., 60.));
            assert_dimensions(
                &[desired.width, desired.height],
                &case["narrow"]["desired"],
                &case.to_string(),
            );
            assert_eq!(json!(overflow(&p)), case["narrow"]["overflow"]);
            for (i, child) in p.children().to_vec().into_iter().enumerate() {
                let expected = &case["narrow"]["items"][i];
                let rect = assigned(&child);
                assert_dimensions(
                    &[rect.x, rect.y, rect.width, rect.height],
                    &expected["bounds"],
                    &format!("{case} / {}", child.name().unwrap()),
                );
                assert_eq!(
                    json!(!KeyboardNavigation::get_is_tab_stop(&child)),
                    expected["hidden"]
                );
                assert_eq!(child.previous_measure(), Some(Size::INFINITY));
            }
            p.set_is_overflow_enabled(false);
            let desired = <RibbonToolBarPanel as LayoutableImpl>::measure_override(
                &p,
                Size::new(1000., 1000.),
            );
            <RibbonToolBarPanel as LayoutableImpl>::arrange_override(&p, Size::new(1000., 1000.));
            assert_dimensions(
                &[desired.width, desired.height],
                &case["wide"]["desired"],
                &case.to_string(),
            );
            assert_eq!(json!(overflow(&p)), case["wide"]["overflow"]);
            assert_eq!(json!(held == p.overflow_items()), case["wide"]["sameList"]);
            assert_eq!(
                json!(
                    p.children()
                        .to_vec()
                        .iter()
                        .map(|c| !KeyboardNavigation::get_is_tab_stop(c))
                        .collect::<Vec<_>>()
                ),
                case["wide"]["hidden"]
            );
        }
    });
}
#[test]
fn toolbar_panel_defaults_plain_setters_button_reassignment_and_shared_overflow() {
    run(|| {
        let p = RibbonToolBarPanel::new();
        assert_eq!(p.orientation(), Orientation::Horizontal);
        assert_eq!(p.columns(), 1);
        assert_eq!(p.spacing(), 2.);
        assert!(p.is_overflow_enabled());
        p.measure(Size::new(200., 200.));
        assert!(p.is_measure_valid());
        p.set_columns(-2);
        p.set_orientation(Orientation::from_raw(99));
        p.set_spacing(1.5);
        p.set_is_overflow_enabled(false);
        assert!(
            p.is_measure_valid(),
            "source plain fields do not invalidate layout"
        );
        let x = Fixed::new("x", Size::default());
        p.set_overflow_button(Some(x.clone().upcast()));
        p.children().add(Fixed::new("after", Size::default()));
        p.set_overflow_button(Some(x.upcast()));
        assert_eq!(
            json!(
                p.children()
                    .to_vec()
                    .iter()
                    .map(|c| c.name().unwrap())
                    .collect::<Vec<_>>()
            ),
            corpus()["corpus"]["sameButtonMoves"]
        );
        p.set_overflow_button(None);
        assert_eq!(
            json!(
                p.children()
                    .to_vec()
                    .iter()
                    .map(|c| c.name().unwrap())
                    .collect::<Vec<_>>()
            ),
            corpus()["corpus"]["cleared"]
        );
        let held = p.overflow_items();
        (held.clone() as Rc<dyn std::any::Any>)
            .downcast::<ferroribbon_core::collections::RibbonItems<Option<Ref<Control>>>>()
            .unwrap()
            .add(Some(Fixed::new("external", Size::default()).upcast()));
        assert_eq!(p.overflow_items().count(), 1);
        assert!(held == p.overflow_items());
    });
}
#[test]
fn toolbar_panel_missing_overflow_button_preserves_source_failure() {
    run(|| {
        assert_eq!(
            corpus()["corpus"]["missingButtonError"],
            "NullReferenceException"
        );
        let p = RibbonToolBarPanel::new();
        p.children().add(Fixed::new("a", Size::new(20., 10.)));
        p.children().add(Fixed::new("b", Size::new(20., 10.)));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                <RibbonToolBarPanel as LayoutableImpl>::measure_override(&p, Size::new(20., 20.))
            }))
            .is_err()
        );
    });
}
#[test]
fn toolbar_orientation_uses_source_numeric_values_and_explicit_native_conversion() {
    run(|| {
        let d = corpus();
        assert_eq!(
            i64::from(Orientation::Vertical.raw()),
            d["corpus"]["orientation"]["vertical"]
        );
        assert_eq!(
            i64::from(Orientation::Horizontal.raw()),
            d["corpus"]["orientation"]["horizontal"]
        );
        assert_eq!(
            Orientation::from(ferroui_base::layout::Orientation::Horizontal),
            Orientation::Horizontal
        );
        assert_eq!(
            ferroui_base::layout::Orientation::try_from(Orientation::Vertical),
            Ok(ferroui_base::layout::Orientation::Vertical)
        );
        assert_eq!(
            ferroui_base::layout::Orientation::try_from(Orientation::from_raw(99)),
            Err(99)
        );
    });
}
#[test]
fn toolbar_panel_rows_read_later_desired_sizes_after_reentrant_arrange() {
    run(|| {
        let p = RibbonToolBarPanel::new();
        p.set_orientation(Orientation::Vertical);
        let a = Fixed::new("a", Size::new(10., 10.));
        let b = Fixed::new("b", Size::new(10., 20.));
        let c = Fixed::new("c", Size::new(10., 12.));
        for child in [a.clone(), b.clone(), c.clone()] {
            p.children().add(child);
        }
        <RibbonToolBarPanel as LayoutableImpl>::measure_override(&p, Size::new(1000., 1000.));
        let later = b.clone();
        a.after_arrange.replace(Some(Rc::new(move || {
            later.size.set(Size::new(10., 31.));
            later.invalidate_measure();
            later.measure(Size::INFINITY);
        })));
        <RibbonToolBarPanel as LayoutableImpl>::arrange_override(&p, Size::new(100., 100.));
        assert_dimensions(
            &[a.assigned.get().y, b.assigned.get().y, c.assigned.get().y],
            &corpus()["corpus"]["lazyRows"],
            "source rows resume after child Arrange callback",
        );
    });
}
#[test]
fn toolbar_panel_numeric_edges_empty_and_source_linq_max_bits() {
    run(|| {
        let data = corpus();
        let empty = RibbonToolBarPanel::new();
        let size =
            <RibbonToolBarPanel as LayoutableImpl>::measure_override(&empty, Size::default());
        assert_dimensions(
            &[size.width, size.height],
            &data["corpus"]["empty"],
            "empty",
        );
        for case in data["corpus"]["edges"].as_array().unwrap() {
            let p = RibbonToolBarPanel::new();
            p.set_spacing(case["spacing"].as_str().unwrap().parse().unwrap());
            p.children().add(Fixed::new("a", Size::new(20., 10.)));
            p.children().add(Fixed::new("b", Size::new(30., 12.)));
            p.set_overflow_button(Some(Fixed::new("more", Size::new(5., 5.)).upcast()));
            let size =
                <RibbonToolBarPanel as LayoutableImpl>::measure_override(&p, Size::new(25., 20.));
            for (actual, expected) in [size.width, size.height]
                .into_iter()
                .zip(case["desired"].as_array().unwrap())
            {
                if expected.as_str() == Some("NaN") {
                    assert!(actual.is_nan(), "{case}");
                } else {
                    assert_eq!(actual, expected.as_f64().unwrap(), "{case}");
                }
            }
            assert_eq!(
                i64::from(p.overflow_items().count()),
                case["overflow"].as_i64().unwrap()
            );
        }
        for case in data["corpus"]["maxCases"].as_array().unwrap() {
            let values = case["bits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap()));
            let actual = linq_max(values).unwrap();
            assert_eq!(
                format!("{:016X}", actual.to_bits()),
                case["maxBits"].as_str().unwrap(),
                "{case}"
            );
        }
    });
}
#[test]
fn toolbar_panel_measure_mutation_stops_before_clearing_overflow() {
    run(|| {
        let expected = corpus()["corpus"]["mutation"].clone();
        assert_eq!(expected["error"], "InvalidOperationException");
        let panel = RibbonToolBarPanel::new();
        let a = Fixed::new("a", Size::default());
        let b = Fixed::new("b", Size::default());
        panel.children().add(a.clone());
        panel.children().add(b);
        let held = panel.overflow_items();
        (held.clone() as Rc<dyn std::any::Any>)
            .downcast::<ferroribbon_core::collections::RibbonItems<Option<Ref<Control>>>>()
            .unwrap()
            .add(Some(Fixed::new("retained", Size::default()).upcast()));
        let weak = panel.downgrade();
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        a.after_measure.replace(Some(Rc::new(move || {
            count.set(count.get() + 1);
            let panel = weak.upgrade().unwrap();
            let added = Fixed::new("added", Size::default());
            panel.children().add(added.clone());
            panel.children().remove(added);
        })));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                <RibbonToolBarPanel as LayoutableImpl>::measure_override(
                    &panel,
                    Size::new(100., 100.),
                )
            }))
            .is_err()
        );
        assert_eq!(json!(calls.get()), expected["calls"]);
        assert_eq!(json!(held.count()), expected["overflowCount"]);
        assert_eq!(json!(panel.children().count()), expected["children"]);
    });
}
