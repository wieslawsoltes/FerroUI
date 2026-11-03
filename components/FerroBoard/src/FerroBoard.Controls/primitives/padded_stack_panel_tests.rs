use super::*;
use ferroui_controls::Control;
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// Override the child boundary, not the parent algorithm: record raw constraints/slots,
// return the same preassigned desired sizes as the C# oracle (including collapsed calls).
#[repr(C)]
struct Probe {
    base: Control,
    requested: Size,
    measured: Cell<Size>,
    slot: Cell<Rect>,
    on_measure: RefCell<Option<Rc<dyn Fn()>>>,
    on_arrange: RefCell<Option<Rc<dyn Fn()>>>,
}
ferro_class!(Probe:Control);
ferro_impl_classes!(Probe:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl LayoutableImpl for Probe {
    fn measure_core(this: &Self, size: Size) -> Size {
        this.measured.set(size);
        let desired = if this.is_visible() {
            this.requested
        } else {
            Size::default()
        };
        let callback = this.on_measure.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        desired
    }
    fn arrange_core(this: &Self, rect: Rect) {
        this.slot.set(rect);
        let callback = this.on_arrange.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}
impl Probe {
    fn new(requested: Size) -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            requested,
            measured: Cell::new(Size::default()),
            slot: Cell::new(Rect::default()),
            on_measure: RefCell::new(None),
            on_arrange: RefCell::new(None),
        })
    }
}
fn f(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}
fn size(value: &Value) -> Size {
    Size::new(f(&value[0]), f(&value[1]))
}
fn bits(values: impl IntoIterator<Item = f64>) -> Vec<String> {
    values
        .into_iter()
        .map(|v| format!("{:016x}", v.to_bits()))
        .collect()
}
fn expected(row: &Value) -> Vec<String> {
    row.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn unchanged_uno_source_padding_spacing_and_slot_corpus() {
    crate::test_support::run(|| {
        for (index, line) in include_str!("../../../tests/fixtures/padded-stack.jsonl")
            .lines()
            .enumerate()
        {
            let row: Value = serde_json::from_str(line).unwrap();
            let panel = PaddedStackPanel::new();
            panel.set_use_layout_rounding(row["rounding"].as_bool().unwrap());
            panel.set_orientation(if row["orientation"] == "Vertical" {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            });
            panel.set_padding(Thickness::new(
                f(&row["padding"][0]),
                f(&row["padding"][1]),
                f(&row["padding"][2]),
                f(&row["padding"][3]),
            ));
            panel.set_spacing(f(&row["spacing"]));
            let children: Vec<_> = row["sizes"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let child = Probe::new(size(s));
                    child.set_is_visible(row["visible"][i].as_bool().unwrap());
                    panel.children().add(child.clone());
                    child
                })
                .collect();
            let measured = <PaddedStackPanel as LayoutableImpl>::measure_override(
                &panel,
                size(&row["available"]),
            );
            assert_eq!(
                bits([measured.width, measured.height]),
                expected(&row["measured"]),
                "measure row{index}"
            );
            let arranged =
                <PaddedStackPanel as LayoutableImpl>::arrange_override(&panel, size(&row["final"]));
            assert_eq!(
                bits([arranged.width, arranged.height]),
                expected(&row["arranged"]),
                "arrange row{index}"
            );
            for (i, child) in children.iter().enumerate() {
                let m = child.measured.get();
                let s = child.slot.get();
                assert_eq!(
                    bits([m.width, m.height]),
                    expected(&row["constraints"][i]),
                    "constraint row{index} child{i}"
                );
                assert_eq!(
                    bits([s.x, s.y, s.width, s.height]),
                    expected(&row["slots"][i]),
                    "slot row{index} child{i}"
                );
            }
        }
    });
}

#[test]
fn native_padding_changes_child_bounds_and_invalidates_measure_without_outer_margin() {
    crate::test_support::run(|| {
        let panel = PaddedStackPanel::new();
        panel.set_use_layout_rounding(false);
        panel.set_padding(Thickness::new(16.0, 12.0, 16.0, 24.0));
        panel.set_spacing(10.0);
        let a = Control::new();
        a.set_use_layout_rounding(false);
        a.set_height(30.0);
        let b = Control::new();
        b.set_use_layout_rounding(false);
        b.set_height(20.0);
        panel.children().add(a.clone());
        panel.children().add(b.clone());
        panel.measure(Size::new(200.0, 200.0));
        assert_eq!(panel.desired_size(), Size::new(32.0, 96.0));
        panel.arrange(Rect::new(0.0, 0.0, 200.0, 96.0));
        assert_eq!(panel.bounds(), Rect::new(0.0, 0.0, 200.0, 96.0));
        assert_eq!(panel.margin(), Thickness::default());
        assert_eq!(a.bounds(), Rect::new(16.0, 12.0, 168.0, 30.0));
        assert_eq!(b.bounds(), Rect::new(16.0, 52.0, 168.0, 20.0));
        panel.set_padding(Thickness::new(4.0, 3.0, 8.0, 9.0));
        assert!(!panel.is_measure_valid());
        panel.measure(Size::new(200.0, 200.0));
        panel.arrange(Rect::new(0.0, 0.0, 200.0, 72.0));
        assert_eq!(a.bounds(), Rect::new(4.0, 3.0, 188.0, 30.0));
        assert_eq!(b.bounds(), Rect::new(4.0, 43.0, 188.0, 20.0));
        assert_eq!(PaddedStackPanel::TYPE.base_type(), Some(StackPanel::TYPE));
    });
}

#[test]
fn source_captured_count_live_index_and_post_callback_state() {
    crate::test_support::run(|| {
        for line in include_str!("../../../tests/fixtures/padded-stack-mutations.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let panel = PaddedStackPanel::new();
            panel.set_use_layout_rounding(false);
            panel.set_padding(Thickness::new(1.0, 2.0, 3.0, 4.0));
            panel.set_spacing(10.0);
            let first = Probe::new(Size::new(5.0, 6.0));
            let second = Probe::new(Size::new(7.0, 8.0));
            panel.children().add(first.clone());
            panel.children().add(second);
            let weak = panel.downgrade();
            let weak_first = first.downgrade();
            let action = row["action"].as_str().unwrap().to_owned();
            *first.on_measure.borrow_mut() = Some(Rc::new(move || {
                let panel = weak.upgrade().unwrap();
                match action.as_str() {
                    "append" => {
                        panel.children().add(Probe::new(Size::new(11.0, 12.0)));
                    }
                    "replace" => {
                        panel.children().set(1, Probe::new(Size::new(11.0, 12.0)));
                    }
                    "mode" => {
                        panel.set_orientation(Orientation::Horizontal);
                        panel.set_spacing(3.0);
                        panel.set_padding(Thickness::new(100.0, 200.0, 300.0, 400.0));
                    }
                    "visibility" => weak_first.upgrade().unwrap().set_is_visible(false),
                    _ => unreachable!(),
                }
            }));
            let measured = <PaddedStackPanel as LayoutableImpl>::measure_override(
                &panel,
                Size::new(100.0, 200.0),
            );
            assert_eq!(
                bits([measured.width, measured.height]),
                expected(&row["measured"]),
                "{}",
                row["action"]
            );
            assert_eq!(
                panel.children().count(),
                row["count"].as_u64().unwrap() as usize
            );
            for (i, child) in panel.children().snapshot().iter().enumerate() {
                let child = child.cast::<Probe>().unwrap();
                let m = child.measured.get();
                let d = child.desired_size();
                assert_eq!(
                    bits([m.width, m.height]),
                    expected(&row["children"][i]["constraint"])
                );
                assert_eq!(
                    bits([d.width, d.height]),
                    expected(&row["children"][i]["desired"])
                );
                assert_eq!(
                    child.is_visible(),
                    row["children"][i]["visible"].as_bool().unwrap()
                );
            }
        }
    });
}
