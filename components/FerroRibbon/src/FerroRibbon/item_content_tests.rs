use crate::{
    primitives::{
        RibbonIconPresenter, RibbonItemContent, RibbonItemContentPart, RibbonMetrics,
        ribbon_item_content::split_label,
    },
    tests::run,
};
use ferroribbon_core::{RibbonDensity, RibbonItemSize, RibbonString};
use ferroui_base::media::{IBrush, SolidColorBrush, TextLineBounds, TextTrimming, TextWrapping};
use ferroui_base::{
    FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl, ferro_class,
    ferro_impl_classes, instantiate,
};
use ferroui_base::{
    input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{Border, Control, ControlImpl, TextBlock, TextBlockImpl};
use std::cell::Cell;
use std::rc::Rc;

fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!("../../tests/fixtures/item-content.json")).unwrap()
}
fn bits(v: &serde_json::Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn strings(v: &serde_json::Value) -> RibbonString {
    RibbonString::from_utf16(
        v.as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
fn units(v: Option<String>) -> serde_json::Value {
    v.map(|s| serde_json::json!(s.encode_utf16().collect::<Vec<_>>()))
        .unwrap_or(serde_json::Value::Null)
}
fn b(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
fn advance(s: &RibbonString, font: f64) -> f64 {
    let mut sum = 0.;
    for &c in s.as_utf16() {
        sum += 3. + f64::from(c % 7) * 0.25;
    }
    sum * font / 12.
}
fn brush() -> Rc<dyn IBrush> {
    SolidColorBrush::from_uint32(0xff112233).into()
}

// Test-only measurement service. It is intentionally not a font renderer.
#[repr(C)]
struct ContractText {
    base: TextBlock,
}
ferro_class!(ContractText:TextBlock);
ferro_impl_classes!(ContractText:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TextBlockImpl);
impl LayoutableImpl for ContractText {
    fn measure_override(this: &Self, _: Size) -> Size {
        Size::new(
            advance(&this.text().unwrap_or_default().into(), this.font_size()),
            if this.line_height() > 0. {
                this.line_height()
            } else {
                this.font_size() + 2.
            },
        )
    }
}
fn contract_text() -> Ref<TextBlock> {
    let t: Ref<ContractText> = instantiate(ContractText {
        base: TextBlock::construct(),
    });
    t.set_use_layout_rounding(false);
    t.set_text(Some(""));
    t.set_is_hit_test_visible(false);
    t.set_text_wrapping(TextWrapping::NoWrap);
    t.set_text_trimming(<dyn TextTrimming>::none());
    t.upcast()
}
fn content() -> Ref<RibbonItemContent> {
    let p = instantiate(RibbonItemContent::construct_with_text_factory(
        contract_text,
    ));
    p.set_use_layout_rounding(false);
    p
}
fn text(p: &RibbonItemContent, index: usize) -> Ref<TextBlock> {
    p.children().get(index).cast::<TextBlock>().unwrap()
}
fn lines(p: &RibbonItemContent) -> serde_json::Value {
    serde_json::json!([units(text(p, 1).text()), units(text(p, 2).text())])
}
#[test]
fn verbatim_source_split_calls_utf16_whitespace_and_half_pixel_ties() {
    let c = corpus();
    for case in c["splits"].as_array().unwrap() {
        let font = bits(&case["font"]);
        let mut calls = Vec::new();
        let (a, z) = split_label(
            &strings(&case["label"]),
            case["show"].as_bool().unwrap(),
            |s| {
                calls.push(serde_json::json!({"text":s.as_utf16(),"font":b(font)}));
                advance(s, font)
            },
        );
        assert_eq!(
            serde_json::json!([a.as_utf16(), z.as_utf16()]),
            case["lines"],
            "{case}"
        );
        assert_eq!(serde_json::json!(calls), case["calls"], "{case}");
    }
    for case in c["thresholds"].as_array().unwrap() {
        let second = bits(&case["second"]);
        let (a, z) = split_label(&"A B C".into(), false, |s| {
            if s == &RibbonString::from("A") || s == &RibbonString::from("B C") {
                20.
            } else {
                second
            }
        });
        assert_eq!(
            serde_json::json!([a.as_utf16(), z.as_utf16()]),
            case["lines"]
        );
    }
}
#[test]
fn unchanged_source_controlled_measure_and_sequential_arrange_matrix() {
    run(|| {
        let c = corpus();
        for case in c["cases"].as_array().unwrap() {
            let p = content();
            let density = case["density"].as_i64().unwrap() as i32;
            p.set_metrics(
                (density >= 0)
                    .then(|| RibbonMetrics::for_density(RibbonDensity::from_raw(density))),
            );
            p.set_item_size(RibbonItemSize::from_raw(
                case["size"].as_i64().unwrap() as i32
            ));
            p.set_part(RibbonItemContentPart::from_raw(
                case["part"].as_i64().unwrap() as i32,
            ));
            p.set_is_simplified(case["simple"].as_bool().unwrap());
            let flags = case["flags"].as_i64().unwrap();
            p.set_show_label(flags & 1 != 0);
            p.set_show_chevron(flags & 2 != 0);
            let mode = case["iconMode"].as_i64().unwrap();
            if mode & 1 != 0 {
                let icon = Border::new();
                icon.set_width(7.);
                icon.set_height(8.);
                p.set_icon(Some(Rc::new(icon)));
            }
            if mode & 2 != 0 {
                let icon = Border::new();
                icon.set_width(11.);
                icon.set_height(12.);
                p.set_large_icon(Some(Rc::new(icon)));
            }
            p.set_color_bar((flags & 4 != 0).then(brush));
            p.set_label(
                (!case["label"].is_null()).then(|| strings(&case["label"]).to_utf8().unwrap()),
            );
            // Compare original override bodies, not two different framework engines'
            // outer desired-size clamping/rounding or child alignment rectangles.
            let size =
                <RibbonItemContent as LayoutableImpl>::measure_override(&p, Size::new(37.25, 25.5));
            assert_eq!(
                serde_json::json!([b(size.width), b(size.height)]),
                case["desired"],
                "case{}",
                case["id"]
            );
            p.arranged.borrow_mut().clear();
            let final_size = Size::new(bits(&case["final"][0]), bits(&case["final"][1]));
            assert_eq!(
                <RibbonItemContent as LayoutableImpl>::arrange_override(&p, final_size),
                final_size
            );
            let arranged=p.arranged.borrow().iter().map(|(i,r)|serde_json::json!({"index":i,"rect":[b(r.x),b(r.y),b(r.width),b(r.height)]})).collect::<Vec<_>>();
            assert_eq!(
                serde_json::json!(arranged),
                case["arranged"],
                "arrange case{}",
                case["id"]
            );
            let icon = p.children().get(0).cast::<RibbonIconPresenter>().unwrap();
            let mut labels = Vec::new();
            for i in [1, 2] {
                let t = text(&p, i);
                labels.push(serde_json::json!({"text":units(t.text()),"visible":t.is_visible(),"size":[b(t.desired_size().width),b(t.desired_size().height)],"font":b(t.font_size()),"lineHeight":b(t.line_height())}));
            }
            assert_eq!(
                serde_json::json!({"lines":labels,"icon":icon.has_icon(),"iconSize":b(icon.icon_size()),"visible":p.children().snapshot().iter().map(|c|c.is_visible()).collect::<Vec<_>>()}),
                case["output"],
                "state case{}",
                case["id"]
            );
        }
    });
}
#[test]
fn source_cached_lines_survive_chevron_mode_and_subthreshold_font_changes() {
    run(|| {
        let source = corpus();
        let p = content();
        p.set_item_size(RibbonItemSize::Large);
        p.set_label(Some("A B C D".into()));
        let mut trace = Vec::new();
        let capture = |name: &str, trace: &mut Vec<serde_json::Value>| {
            trace.push(serde_json::json!({"name":name,"lines":lines(&p)}))
        };
        capture("large", &mut trace);
        p.set_show_chevron(true);
        capture("chevron", &mut trace);
        p.set_item_size(RibbonItemSize::Medium);
        capture("medium", &mut trace);
        p.set_item_size(RibbonItemSize::Large);
        capture("large-again", &mut trace);
        p.set_metrics(Some(
            RibbonMetrics::comfortable().with_values(|m| m.font_size = 12.005),
        ));
        capture("font-below-threshold", &mut trace);
        p.set_metrics(Some(
            RibbonMetrics::comfortable().with_values(|m| m.font_size = 12.02),
        ));
        capture("font-above-threshold", &mut trace);
        assert_eq!(serde_json::json!(trace), source["cache"]);
    });
}
#[test]
fn native_text_rendering_children_brushes_measure_invalidation_and_icon_identity() {
    run(|| {
        use crate::compat::FontIcon;
        use ferroui_controls::Viewbox;
        use ferroui_controls::shapes::Rectangle;
        let p = RibbonItemContent::new();
        assert_eq!(p.children().snapshot().len(), 5);
        let original = p.children().snapshot();
        TextBlock::set_text_line_bounds_on(&p, TextLineBounds::Tight);
        assert_eq!(text(&p, 1).text_line_bounds(), TextLineBounds::Full);
        assert_eq!(text(&p, 2).text_line_bounds(), TextLineBounds::Full);
        p.set_use_layout_rounding(false);
        p.set_label(Some("Format Painter".into()));
        p.set_item_size(RibbonItemSize::Large);
        p.set_show_chevron(true);
        p.set_foreground(Some(brush()));
        p.set_color_bar(Some(brush()));
        p.measure(Size::INFINITY);
        p.arrange(Rect::from_size(p.desired_size()));
        assert!(p.desired_size().height > 0.);
        assert!(p.is_measure_valid());
        assert_eq!(text(&p, 1).line_height(), 14.);
        assert_eq!(text(&p, 1).foreground(), p.foreground());
        let chevron = p.children().get(3).cast::<FontIcon>().unwrap();
        assert_eq!(chevron.glyph().as_deref(), Some("\u{e70d}"));
        let before = chevron.foreground();
        p.set_foreground(None);
        assert!(text(&p, 1).foreground().is_none());
        assert_eq!(chevron.foreground(), before);
        assert!(!p.is_measure_valid());
        let icon = Border::new();
        icon.set_width(10.);
        icon.set_height(10.);
        p.set_icon(Some(Rc::new(icon.clone())));
        let presenter = p.children().get(0).cast::<RibbonIconPresenter>().unwrap();
        let box1 = presenter.children().get(0).cast::<Viewbox>().unwrap();
        assert!(box1.child().unwrap().ptr_eq(&icon));
        p.set_item_size(RibbonItemSize::Medium);
        assert!(box1.child().is_none());
        assert!(
            presenter
                .children()
                .get(0)
                .cast::<Viewbox>()
                .unwrap()
                .child()
                .unwrap()
                .ptr_eq(&icon)
        );
        p.set_part(RibbonItemContentPart::LabelAndChevron);
        assert!(!presenter.has_icon());
        assert!(icon.parent().is_none());
        assert!(
            !p.children()
                .get(4)
                .cast::<Rectangle>()
                .unwrap()
                .is_visible()
        );
        assert!(
            p.children()
                .snapshot()
                .iter()
                .zip(original.iter())
                .all(|(a, b)| a.ptr_eq(b))
        );
    });
}
#[repr(C)]
struct MeasureAction {
    base: ferroui_controls::Control,
    action: Rc<dyn Fn()>,
}
ferro_class!(MeasureAction:Control);
ferro_impl_classes!(MeasureAction:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl LayoutableImpl for MeasureAction {
    fn measure_override(this: &Self, _: Size) -> Size {
        (this.action)();
        Size::new(1., 1.)
    }
}
#[test]
fn original_reentrant_label_measure_metrics_and_arrange_part_reads() {
    run(|| {
        let c = corpus();
        let p = content();
        p.set_item_size(RibbonItemSize::Large);
        let once = Rc::new(Cell::new(false));
        let weak = p.downgrade();
        text(&p, 1).add_property_changed_handler(Rc::new(move |e| {
            if e.property() == TextBlock::text_property().as_property()
                && e.is_effective_value_change()
                && !once.replace(true)
            {
                weak.upgrade()
                    .unwrap()
                    .set_label(Some("Replacement Words".into()));
            }
        }));
        p.set_label(Some("Alpha Beta Gamma".into()));
        assert_eq!(lines(&p), c["reentry"]["firstLines"]);
        <RibbonItemContent as LayoutableImpl>::measure_override(&p, Size::new(200., 100.));
        assert_eq!(lines(&p), c["reentry"]["measuredLines"]);
        let metrics = content();
        metrics.set_label(Some("A B".into()));
        let weak = metrics.downgrade();
        let action = instantiate(MeasureAction {
            base: ferroui_controls::Control::construct(),
            action: Rc::new(move || {
                weak.upgrade()
                    .unwrap()
                    .set_metrics(Some(RibbonMetrics::touch()))
            }),
        });
        metrics.children().add(action);
        let size = <RibbonItemContent as LayoutableImpl>::measure_override(
            &metrics,
            Size::new(200., 100.),
        );
        assert_eq!(
            serde_json::json!([b(size.width), b(size.height)]),
            c["reentry"]["metricDesired"]
        );
        let p = content();
        p.set_item_size(RibbonItemSize::Large);
        p.set_label(Some("A B C".into()));
        p.set_show_chevron(true);
        <RibbonItemContent as LayoutableImpl>::measure_override(&p, Size::new(200., 100.));
        let weak = p.downgrade();
        let once = Cell::new(false);
        p.arrange_observer.replace(Some(Rc::new(move |i| {
            if i == 0 && !once.replace(true) {
                weak.upgrade()
                    .unwrap()
                    .set_part(RibbonItemContentPart::IconOnly)
            }
        })));
        <RibbonItemContent as LayoutableImpl>::arrange_override(&p, Size::new(80., 68.));
        let arranged=p.arranged.borrow().iter().map(|(index,r)|serde_json::json!({"index":index,"rect":[b(r.x),b(r.y),b(r.width),b(r.height)]})).collect::<Vec<_>>();
        assert_eq!(serde_json::json!(arranged), c["reentry"]["arrangement"]);
    });
}
