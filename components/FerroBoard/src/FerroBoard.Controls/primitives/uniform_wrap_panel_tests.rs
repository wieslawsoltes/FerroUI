use super::*;
use ferroui_controls::Control;
use serde_json::Value;
#[repr(C)]
struct Probe {
    base: Control,
    measured: Cell<Size>,
}
ferro_class!(Probe:Control);
ferro_impl_classes!(Probe:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl LayoutableImpl for Probe {
    fn measure_override(this: &Self, size: Size) -> Size {
        this.measured.set(size);
        Size::new(0.0, 0.0)
    }
}
impl Probe {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            measured: Cell::new(Size::default()),
        })
    }
}
fn f(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn b(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
#[test]
fn unchanged_source_columns_and_layout_corpus() {
    crate::test_support::run(|| {
        for line in include_str!("../../../tests/fixtures/uniform-wrap.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let p = UniformWrapPanel::new();
            p.set_use_layout_rounding(false);
            p.set_max_columns(row["maxColumns"].as_i64().unwrap() as i32);
            if row["op"] == "columns" {
                p.set_item_width(f(&row["itemWidth"]));
                assert_eq!(
                    p.columns(f(&row["width"])),
                    row["columns"].as_i64().unwrap() as i32
                );
                continue;
            }
            p.set_item_height(27.25);
            let mut children = Vec::new();
            for _ in 0..row["count"].as_u64().unwrap() {
                let child = Probe::new();
                child.set_use_layout_rounding(false);
                p.children().add(child.clone());
                children.push(child);
            }
            let size = <UniformWrapPanel as LayoutableImpl>::measure_override(
                &p,
                Size::new(f(&row["width"]), f(&row["height"])),
            );
            assert_eq!(
                [b(size.width), b(size.height)],
                [
                    row["measured"][0].as_str().unwrap(),
                    row["measured"][1].as_str().unwrap()
                ]
            );
            for (i, c) in children.iter().enumerate() {
                assert_eq!(
                    [b(c.measured.get().width), b(c.measured.get().height)],
                    [
                        row["children"][i][0].as_str().unwrap(),
                        row["children"][i][1].as_str().unwrap()
                    ]
                );
            }
            let size = <UniformWrapPanel as LayoutableImpl>::arrange_override(
                &p,
                Size::new(f(&row["finalWidth"]), f(&row["height"])),
            );
            assert_eq!(
                [b(size.width), b(size.height)],
                [
                    row["arranged"][0].as_str().unwrap(),
                    row["arranged"][1].as_str().unwrap()
                ]
            );
            for (i, c) in children.iter().enumerate() {
                let r = c.bounds();
                assert_eq!(
                    [b(r.x), b(r.y), b(r.width), b(r.height)],
                    [
                        row["bounds"][i][0].as_str().unwrap(),
                        row["bounds"][i][1].as_str().unwrap(),
                        row["bounds"][i][2].as_str().unwrap(),
                        row["bounds"][i][3].as_str().unwrap()
                    ]
                );
            }
        }
    });
}
#[test]
fn source_plain_properties_do_not_invalidate_native_layout() {
    crate::test_support::run(|| {
        let p = UniformWrapPanel::new();
        p.measure(Size::new(300.0, 400.0));
        assert!(p.is_measure_valid());
        p.set_item_width(90.0);
        p.set_item_height(70.0);
        p.set_max_columns(2);
        assert!(p.is_measure_valid());
    });
}
