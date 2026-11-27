use crate::{
    controls::{IRibbonItem, RibbonItemLayout, RibbonKeyTipResult, RibbonOverflowMenuItems},
    primitives::{RibbonGroupItemsPanel, RibbonMetrics},
    tests::run,
};
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{
    RibbonGroupItemsLayout, RibbonItemSize, RibbonSimplifiedVisibility, RibbonString,
    collections::RibbonReadOnlyList, layout::RibbonSizeDefinition,
};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::{InputElementImpl, KeyboardNavigation},
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
};
use ferroui_controls::{
    Control, ControlImpl,
    primitives::{TemplatedControl, TemplatedControlImpl},
};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};
#[repr(C)]
struct Fixed {
    base: TemplatedControl,
    size: Size,
    assigned: Cell<Rect>,
}
ferro_class!(Fixed:TemplatedControl);
ferro_impl_classes!(Fixed:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl LayoutableImpl for Fixed {
    fn measure_override(this: &Self, _available: Size) -> Size {
        this.size
    }
    fn arrange_core(this: &Self, rect: Rect) {
        this.assigned.set(rect);
        Self::parent_arrange_core(this, rect)
    }
}
impl Fixed {
    fn construct(size: Size) -> Self {
        Self {
            base: TemplatedControl::construct(),
            size,
            assigned: Cell::new(Rect::default()),
        }
    }
    fn new(size: Size) -> Ref<Self> {
        let value = instantiate(Self::construct(size));
        initialize(&value);
        value
    }
}
fn initialize(value: &Control) {
    value.set_use_layout_rounding(false);
    KeyboardNavigation::set_is_tab_stop(value, true)
}
#[repr(C)]
struct FixedItem {
    base: Fixed,
    size: RibbonItemSize,
}
ferro_class!(FixedItem:Fixed);
ferro_impl_classes!(FixedItem:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_class_info!(FixedItem {interfaces:[Rc<dyn IRibbonItem> => |v|Rc::new(ItemView(v)) as Rc<dyn IRibbonItem>]});
impl FixedItem {
    fn new(size: Size, item_size: RibbonItemSize) -> Ref<Self> {
        let value = instantiate(Self {
            base: Fixed::construct(size),
            size: item_size,
        });
        initialize(&value);
        value
    }
}
struct ItemView(Ref<FixedItem>);
impl IRibbonItem for ItemView {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn current_size(&self) -> RibbonItemSize {
        self.0.size
    }
    fn id(&self) -> Option<RibbonString> {
        panic!("size fixture has no ID")
    }
    fn label(&self) -> Option<RibbonString> {
        panic!("size fixture has no label")
    }
    fn icon(&self) -> Option<BoxedValue> {
        panic!("size fixture has no icon")
    }
    fn key_tip(&self) -> Option<RibbonString> {
        panic!("size fixture has no KeyTip")
    }
    fn command_id(&self) -> Option<RibbonString> {
        panic!("size fixture has no command")
    }
    fn size(&self) -> RibbonItemSize {
        self.0.size
    }
    fn effective_size_definition(&self) -> RibbonSizeDefinition {
        panic!("size fixture has no size definition")
    }
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        panic!("size fixture has no overflow policy")
    }
    fn show_label_in_simplified_mode(&self) -> bool {
        panic!("size fixture has no label policy")
    }
    fn can_add_to_quick_access(&self) -> bool {
        panic!("size fixture has no QAT policy")
    }
    fn apply_layout(&self, _layout: RibbonItemLayout) {
        panic!("size fixture cannot apply layouts")
    }
    fn create_linked_copy(&self) -> Option<Ref<Control>> {
        panic!("size fixture has no copy factory")
    }
    fn create_overflow_menu_items(&self) -> RibbonOverflowMenuItems {
        panic!("size fixture has no menu")
    }
    fn on_key_tip(&self) -> Rc<RibbonKeyTipResult> {
        panic!("size fixture has no KeyTip action")
    }
    fn invoke(&self) -> bool {
        panic!("size fixture has no command")
    }
}
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/group-items-panel.json")).unwrap()
}
fn capture(panel: &RibbonGroupItemsPanel, size: Size) -> Value {
    let children=panel.children().snapshot().iter().map(|c|{
  let fixed=c.downcast_ref::<Fixed>().unwrap();let r=fixed.assigned.get();
  json!({"bounds":[bits(r.x),bits(r.y),bits(r.width),bits(r.height)],"tab":KeyboardNavigation::get_is_tab_stop(c),"full":panel.is_full_height(c)})
 }).collect::<Vec<_>>();
    json!({"size":[bits(size.width),bits(size.height)],"children":children})
}
fn body(panel: &RibbonGroupItemsPanel) -> Size {
    let size =
        <RibbonGroupItemsPanel as LayoutableImpl>::measure_override(panel, Size::new(47., 31.));
    <RibbonGroupItemsPanel as LayoutableImpl>::arrange_override(
        panel,
        Size::new(size.width + 10., size.height + 2.5),
    );
    size
}
#[test]
fn actual_native_group_panel_matches_unchanged_layout_body_across_modes_rows_metrics_and_masks() {
    run(|| {
        for case in corpus()["cases"].as_array().unwrap() {
            let p = RibbonGroupItemsPanel::new();
            p.set_use_layout_rounding(false);
            p.set_metrics(Some(if case["density"] == "compact" {
                RibbonMetrics::compact()
            } else {
                RibbonMetrics::comfortable()
            }));
            p.set_items_layout(RibbonGroupItemsLayout::from_raw(
                case["layout"].as_i64().unwrap() as i32,
            ));
            p.set_row_count(case["rows"].as_i64().unwrap() as i32);
            p.set_is_simplified(case["simplified"].as_bool().unwrap());
            p.set_simplified_spacing(1.25);
            p.set_in_line_mask(case["mask"].as_array().map(|v| {
                RibbonItems::from_values(v.iter().map(|b| b.as_bool().unwrap()).collect())
                    as RibbonReadOnlyList<_>
            }));
            let widths = [12.5, 9., 21.25, 16., 0., 7., 5.];
            let heights = [13., 32., 7.5, 90., 0., 24., 5.];
            for i in 0..case["count"].as_u64().unwrap() as usize {
                let size = Size::new(widths[i], heights[i]);
                let c = if i % 3 == 1 {
                    FixedItem::new(
                        size,
                        if i % 2 == 1 {
                            RibbonItemSize::Large
                        } else {
                            RibbonItemSize::Small
                        },
                    )
                    .upcast::<Control>()
                } else {
                    Fixed::new(size).upcast::<Control>()
                };
                c.set_is_visible(i % 5 != 4);
                p.children().add(c);
            }
            let first = body(&p);
            assert_eq!(capture(&p, first), case["first"], "{case}");
            p.set_is_simplified(!p.is_simplified());
            let second = body(&p);
            assert_eq!(capture(&p, second), case["second"], "{case}");
        }
    });
}
#[test]
fn group_panel_keeps_live_mask_identity_and_plain_properties_require_host_invalidation() {
    run(|| {
        let fixture = corpus();
        let p = RibbonGroupItemsPanel::new();
        p.set_use_layout_rounding(false);
        p.set_is_simplified(true);
        p.children().add(Fixed::new(Size::new(10., 12.)));
        p.children().add(Fixed::new(Size::new(20., 13.)));
        let list = RibbonItems::from_values(vec![true, false]);
        let view = list.clone() as RibbonReadOnlyList<_>;
        p.set_in_line_mask(Some(view.clone()));
        assert!(Rc::ptr_eq(&p.in_line_mask().unwrap(), &view));
        let before =
            <RibbonGroupItemsPanel as LayoutableImpl>::measure_override(&p, Size::INFINITY);
        <RibbonGroupItemsPanel as LayoutableImpl>::arrange_override(&p, before);
        assert_eq!(capture(&p, before), fixture["live"]["before"]);
        list.set(0, false);
        list.set(1, true);
        let after = <RibbonGroupItemsPanel as LayoutableImpl>::measure_override(&p, Size::INFINITY);
        <RibbonGroupItemsPanel as LayoutableImpl>::arrange_override(&p, after);
        assert_eq!(capture(&p, after), fixture["live"]["after"]);
        let p = RibbonGroupItemsPanel::new();
        p.measure(Size::new(100., 100.));
        assert!(p.is_measure_valid());
        p.set_row_count(9);
        p.set_is_simplified(true);
        p.set_simplified_spacing(5.);
        p.set_in_line_mask(Some(RibbonItems::new() as RibbonReadOnlyList<_>));
        p.set_items_layout(RibbonGroupItemsLayout::Rows);
        p.set_metrics(Some(RibbonMetrics::touch()));
        assert!(p.is_measure_valid());
        assert_eq!(fixture["plainSetterInvalidations"], 0);
        p.invalidate_measure();
        assert!(!p.is_measure_valid());
    });
}

fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

struct ChangingMask {
    panel: ferroui_base::WeakRef<RibbonGroupItemsPanel>,
    mode: String,
    trace: Rc<std::cell::RefCell<Vec<String>>>,
}
impl ferroribbon_core::collections::RibbonReadOnlyListImpl<bool> for ChangingMask {
    fn count(&self) -> i32 {
        self.trace.borrow_mut().push("count".into());
        let panel = self.panel.upgrade().unwrap();
        if self.mode == "replace" {
            panel.set_in_line_mask(Some(
                RibbonItems::from_values(vec![false, true]) as RibbonReadOnlyList<_>
            ))
        }
        if self.mode == "clear" {
            panel.set_in_line_mask(None)
        }
        if self.mode == "negative" { -1 } else { 2 }
    }
    fn get(&self, index: i32) -> bool {
        self.trace.borrow_mut().push(format!("old-index:{index}"));
        assert!(self.mode != "throw", "mask index failure");
        true
    }
    fn iter_owned(self: Rc<Self>) -> ferroribbon_core::model::RibbonIterator<bool> {
        panic!("mask must not enumerate")
    }
}
#[test]
fn group_panel_mask_short_circuit_and_reentrant_replacement_match_source() {
    run(|| {
        for case in corpus()["masks"].as_array().unwrap() {
            let panel = RibbonGroupItemsPanel::new();
            let trace = Rc::new(std::cell::RefCell::new(Vec::new()));
            panel.set_in_line_mask(Some(Rc::new(ChangingMask {
                panel: panel.downgrade(),
                mode: case["mode"].as_str().unwrap().into(),
                trace: trace.clone(),
            }) as RibbonReadOnlyList<_>));
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| panel.is_in_line(0)));
            assert_eq!(result.is_err(), case["error"].is_string(), "{case}");
            if let Ok(value) = result {
                assert_eq!(value, case["result"].as_bool().unwrap(), "{case}")
            }
            assert_eq!(json!(&*trace.borrow()), case["trace"], "{case}");
        }
    });
}
