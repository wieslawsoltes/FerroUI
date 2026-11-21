use crate::{
    controls::{RibbonButton, RibbonGroup, RibbonPanelPresentation},
    primitives::{RibbonGroupsPanel, RibbonMetrics},
};
use ferroribbon_core::{
    RibbonGroupState, RibbonSimplifiedVisibility, layout::RibbonReductionStrategy,
};
use ferroui_base::{
    FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl,
    data::core::ValueTypes,
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
    metadata::{MarkupDelegate, MarkupTyped},
    utilities::EventArgs,
};
use ferroui_controls::{
    ControlImpl, Panel,
    primitives::{TemplatedControl, TemplatedControlImpl},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
struct FixedGroup {
    base: RibbonGroup,
    widths: [f64; 4],
    simple: [f64; 2],
    calls: Cell<usize>,
    assigned: Cell<Rect>,
}
ferro_class!(FixedGroup:RibbonGroup);
ferro_class_info!(FixedGroup {});
ferro_impl_classes!(FixedGroup:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl FixedGroup {
    fn new(i: usize) -> Ref<Self> {
        let group = instantiate(Self {
            base: RibbonGroup::construct(),
            widths: [
                90. + i as f64 * 10.,
                60. + i as f64 * 6.,
                40. + i as f64 * 3.,
                20. + i as f64,
            ],
            simple: [16.5 + i as f64 * 2., 24.25 + i as f64 * 3.],
            calls: Cell::new(0),
            assigned: Cell::new(Rect::default()),
        });
        group.set_use_layout_rounding(false);
        for width in group.simple {
            let child = Panel::new();
            child.set_width(width - 1.);
            child.set_height(12.);
            child.set_use_layout_rounding(false);
            group.items().add(Some(child.upcast()));
        }
        group
    }
}
impl LayoutableImpl for FixedGroup {
    fn arrange_core(this: &Self, rect: Rect) {
        this.assigned.set(rect);
        Self::parent_arrange_core(this, rect);
    }
    fn measure_override(this: &Self, available: Size) -> Size {
        this.calls.set(this.calls.get() + 1);
        let width = if this.is_simplified() {
            9. + this
                .simple
                .iter()
                .enumerate()
                .filter(|(i, _)| this.items_panel().is_in_line(*i as i32))
                .map(|(_, v)| *v)
                .sum::<f64>()
        } else {
            this.widths[this.state().raw() as usize]
        };
        Size::new(width, available.height)
    }
}
#[repr(C)]
struct Fixed {
    base: TemplatedControl,
    width: f64,
    height: f64,
    assigned: Cell<Rect>,
}
ferro_class!(Fixed:TemplatedControl);
ferro_class_info!(Fixed {});
ferro_impl_classes!(Fixed:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl Fixed {
    fn new(width: f64, height: f64) -> Ref<Self> {
        let value = instantiate(Self {
            base: TemplatedControl::construct(),
            width,
            height,
            assigned: Cell::new(Rect::default()),
        });
        value.set_use_layout_rounding(false);
        value
    }
}
impl LayoutableImpl for Fixed {
    fn measure_override(this: &Self, _: Size) -> Size {
        Size::new(this.width, this.height)
    }
    fn arrange_core(this: &Self, rect: Rect) {
        this.assigned.set(rect);
        Self::parent_arrange_core(this, rect)
    }
}
fn number(v: &serde_json::Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn rect(v: &serde_json::Value) -> [u64; 4] {
    std::array::from_fn(|i| number(&v[i]).to_bits())
}
fn bits(r: Rect) -> [u64; 4] {
    [r.x, r.y, r.width, r.height].map(f64::to_bits)
}
#[test]
fn unchanged_entire_groups_panel_fixed_measurement_corpus() {
    crate::tests::run(|| {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/groups-panel.json")).unwrap();
        for (case_index, case) in data["cases"].as_array().unwrap().iter().enumerate() {
            let p = RibbonGroupsPanel::new();
            p.set_use_layout_rounding(false);
            p.set_is_simplified(case["simplified"].as_bool().unwrap());
            p.set_is_adaptive(case["adaptive"].as_bool().unwrap());
            p.set_reduction_strategy(RibbonReductionStrategy::from_raw(
                case["strategy"].as_i64().unwrap() as i32,
            ));
            p.set_panel_presentation(RibbonPanelPresentation::from_raw(
                case["presentation"].as_i64().unwrap() as i32,
            ));
            p.set_group_spacing(number(&case["spacing"]));
            for i in 0..case["count"].as_u64().unwrap() as usize {
                let group = FixedGroup::new(i);
                group.set_reduction_order((i % 2) as i32);
                group.set_can_collapse(i != 0);
                group.set_simplified_visibility(if i == 3 {
                    RibbonSimplifiedVisibility::Overflow
                } else {
                    RibbonSimplifiedVisibility::Auto
                });
                if i == 2 {
                    group.items().get(1).unwrap().set_is_visible(false);
                }
                if i == 1 {
                    group.set_is_visible(false);
                }
                p.children().add(group);
            }
            let overflow = Fixed::new(15.5, 12.);
            overflow.set_use_layout_rounding(false);
            p.set_overflow_button(Some(overflow.clone().upcast()));
            let size = p.measure_override(Size::new(number(&case["available"]), 300.));
            assert_eq!(
                [size.width.to_bits(), size.height.to_bits()],
                [
                    number(&case["size"][0]).to_bits(),
                    number(&case["size"][1]).to_bits()
                ],
                "measure case {case_index}"
            );
            assert_eq!(
                p.extent_width().to_bits(),
                number(&case["extent"]).to_bits()
            );
            assert_eq!(p.has_overflow(), case["HasOverflow"].as_bool().unwrap());
            p.arrange_override(Size::new(size.width + 10., size.height + 2.5));
            for (index, group) in p.groups().iter().enumerate() {
                let expected = &case["groups"][index];
                assert_eq!(
                    group.state().raw(),
                    expected["state"].as_i64().unwrap() as i32
                );
                assert_eq!(
                    group.is_simplified(),
                    expected["Simplified"].as_bool().unwrap()
                );
                assert_eq!(
                    group.panel_presentation().raw(),
                    expected["presentation"].as_i64().unwrap() as i32
                );
                let mask = group
                    .items_panel()
                    .in_line_mask()
                    .map(|mask| mask.iter().collect::<Vec<_>>());
                assert_eq!(
                    serde_json::to_value(mask).unwrap(),
                    expected["mask"],
                    "mask case {case_index}"
                );
                assert_eq!(
                    bits(group.cast::<FixedGroup>().unwrap().assigned.get()),
                    rect(&expected["bounds"]),
                    "group {index} case {case_index}"
                );
            }
            assert_eq!(
                bits(overflow.assigned.get()),
                rect(&case["overflow"]),
                "overflow case {case_index}"
            );
        }
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    })
}
#[test]
fn source_classic_cache_reuses_widths_and_invalidates_on_version_metrics_or_reset() {
    crate::tests::run(|| {
        let p = RibbonGroupsPanel::new();
        let group = FixedGroup::new(0);
        p.children().add(group.clone());
        let measure = || p.measure_override(Size::new(300., 300.));
        measure();
        let first = group.calls.get();
        assert!(first >= 4);
        measure();
        assert_eq!(group.calls.get(), first);
        group.set_header(Some("version".into()));
        measure();
        assert!(group.calls.get() >= first + 4);
        let before = group.calls.get();
        p.set_metrics(Some(RibbonMetrics::touch()));
        measure();
        assert!(group.calls.get() >= before + 4);
        let before = group.calls.get();
        p.invalidate_cache();
        measure();
        assert!(group.calls.get() >= before + 4);
        p.set_is_adaptive(false);
        p.measure_override(Size::new(0., 300.));
        assert_eq!(group.state(), RibbonGroupState::Large);
    })
}
#[test]
fn source_simplified_only_cache_retains_removed_group_until_explicit_reset() {
    crate::tests::run(|| {
        let p = RibbonGroupsPanel::new();
        p.set_is_simplified(true);
        let group = FixedGroup::new(0);
        let weak = group.downgrade();
        p.children().add(group.clone());
        p.measure_override(Size::new(200., 300.));
        p.children().remove(group.clone());
        drop(group);
        p.measure_override(Size::new(200., 300.));
        assert!(weak.upgrade().is_some());
        p.invalidate_cache();
        assert!(weak.upgrade().is_none());
    })
}
type EventFn = dyn Fn(Option<ferroui_base::BoxedValue>, EventArgs);
struct Handler(Box<EventFn>);
impl PartialEq for Handler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class Handler{this:Rc<Handler>,handles:[Rc<Handler>],methods:[fn Invoke(Option<ferroui_base::BoxedValue>,EventArgs)=>|h:&Rc<Handler>,sender,args|(h.0)(sender,args)]});
fn handler(f: impl Fn(Option<ferroui_base::BoxedValue>, EventArgs) + 'static) -> MarkupDelegate {
    ValueTypes::register_reference::<Handler>();
    MarkupDelegate::for_method(
        Some(Rc::new(Handler(Box::new(f)))),
        Handler::MARKUP,
        &Handler::MARKUP.methods[0],
    )
}
#[test]
fn source_overflow_event_reads_latest_handlers_and_state_at_deferred_delivery() {
    crate::tests::run(|| {
        let p = RibbonGroupsPanel::new();
        p.set_is_simplified(true);
        p.children().add(FixedGroup::new(0));
        let trace = Rc::new(RefCell::new(vec![]));
        let t = trace.clone();
        let old = handler(move |_, _| t.borrow_mut().push(false));
        p.add_overflow_changed(Some(old.clone()));
        p.measure_override(Size::new(0., 300.));
        assert!(p.has_overflow());
        assert!(trace.borrow().is_empty());
        p.remove_overflow_changed(Some(old));
        let t = trace.clone();
        let weak = p.downgrade();
        let fresh = handler(move |sender, args| {
            let p = weak.upgrade().unwrap();
            assert!(ValueTypes::as_object(&*sender.unwrap()).unwrap().ptr_eq(&p));
            assert_eq!(args, EventArgs::EMPTY);
            t.borrow_mut().push(p.has_overflow());
        });
        p.add_overflow_changed(Some(fresh.clone()));
        p.set_is_simplified(false);
        p.measure_override(Size::new(300., 300.));
        assert!(!p.has_overflow());
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(&*trace.borrow(), &[false, false]);
        p.remove_overflow_changed(Some(fresh));
    })
}
#[test]
fn source_titles_presentation_updates_actual_group_collapsed_button() {
    crate::group_tests::run(|| {
        use ferroui_base::{StaticType, controls::ResourceDictionary, styling::ControlTheme};
        let group = RibbonGroup::new();
        let value =
            crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Groups.xaml")
                .unwrap()
                .unwrap();
        let resources = ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<ResourceDictionary>()
            .unwrap();
        let theme = ValueTypes::as_object(
            &*resources
                .try_get_resource(&<RibbonGroup as StaticType>::TYPE.into(), None)
                .flatten()
                .unwrap(),
        )
        .unwrap()
        .cast::<ControlTheme>()
        .unwrap();
        group.set_resources(resources);
        group.set_theme(theme);
        group.apply_styling();
        group.apply_template();
        group.set_header(Some("Tools".into()));
        group.set_icon(Some(Rc::new("icon".to_string())));
        group.set_panel_presentation(RibbonPanelPresentation::Titles);
        assert!(group.collapsed_button().unwrap().icon().is_none());
        assert_eq!(
            group.collapsed_button().unwrap().current_size(),
            ferroribbon_core::RibbonItemSize::Medium
        );
        assert!(group.collapsed_button().unwrap().actual_show_label());
        group.set_panel_presentation(RibbonPanelPresentation::Buttons);
        assert_eq!(
            group.collapsed_button().unwrap().current_size(),
            ferroribbon_core::RibbonItemSize::Large
        );
        assert!(group.collapsed_button().unwrap().icon().is_some());
        let child = RibbonButton::new();
        group.items().add(Some(child.upcast()));
    })
}
