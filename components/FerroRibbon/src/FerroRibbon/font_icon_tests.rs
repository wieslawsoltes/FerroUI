use crate::{compat::FontIcon, primitives::RibbonIconPresenter, tests::run};
use ferroribbon_core::{RibbonString, model::RibbonIcon};
use ferroui_base::media::{
    FlowDirection, FontFamily, FontStyle, FontWeight, IBrush, SolidColorBrush,
};
use ferroui_base::{Rect, Size};
use ferroui_controls::{Application, Control, TextBlock, Viewbox};
use std::rc::Rc;

fn brush(argb: u32) -> Rc<dyn IBrush> {
    SolidColorBrush::from_uint32(argb).into()
}
fn units(value: &serde_json::Value) -> RibbonString {
    RibbonString::from_utf16(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|u| u.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
fn snapshot(p: &RibbonIconPresenter, expected: &serde_json::Value) {
    assert_eq!(p.has_icon(), expected["has"].as_bool().unwrap());
    let root = p.children().get(0);
    let (element, root_kind) = if let Some(boxed) = root.cast::<Viewbox>() {
        (boxed.child().unwrap(), "Viewbox")
    } else if root.cast::<FontIcon>().is_some() {
        (root.clone(), "FontIcon")
    } else {
        (root.clone(), "TextBlock")
    };
    assert_eq!(root_kind, expected["root"].as_str().unwrap());
    let (kind, glyph, size, weight, style, foreground) =
        if let Some(font) = element.cast::<FontIcon>() {
            // FontFamily is an explicitly mapped native value type, not CLR Source/ToString.
            assert_eq!(
                font.font_family(),
                FontFamily::new(expected["font"].as_str().unwrap())
            );
            (
                "FontIcon",
                font.glyph(),
                font.font_size(),
                font.font_weight().0,
                font.font_style().to_string(),
                font.foreground(),
            )
        } else {
            let text = element.cast::<TextBlock>().unwrap();
            assert!(expected["font"].is_null());
            (
                "TextBlock",
                text.text(),
                text.font_size(),
                text.font_weight().0,
                "Normal".to_owned(),
                text.foreground(),
            )
        };
    assert_eq!(kind, expected["kind"].as_str().unwrap());
    assert_eq!(
        glyph.map(RibbonString::from),
        (!expected["glyph"].is_null()).then(|| units(&expected["glyph"]))
    );
    assert_eq!(size.to_bits(), expected["size"].as_f64().unwrap().to_bits());
    assert_eq!(
        weight,
        i32::try_from(expected["weight"].as_i64().unwrap()).unwrap()
    );
    // This is a named SDK -> native enum mapping; the WinRT source has
    // Italic=2/Oblique=1 and FerroUI has Italic=1/Oblique=2. No numeric equivalence.
    assert_eq!(style, expected["style"].as_str().unwrap());
    assert_eq!(
        expected["styleValue"].as_i64().unwrap(),
        match style.as_str() {
            "Normal" => 0,
            "Oblique" => 1,
            "Italic" => 2,
            other => panic!("Unexpected source style {other}"),
        }
    );
    let color = foreground.and_then(|b| {
        b.as_solid_color_brush()
            .map(|b| format!("#{:08X}", b.color().to_uint32()))
    });
    assert_eq!(color.as_deref(), expected["color"].as_str());
    assert_eq!(
        element.width().is_nan() && element.height().is_nan(),
        expected["auto"].as_bool().unwrap()
    );
}
#[test]
fn original_whole_presenter_glyph_decisions_and_parented_font_clones() {
    run(|| {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/font-icons.json")).unwrap();
        let app = Application::current().unwrap();
        let key = "SymbolThemeFontFamily".into();
        let resources = app.resources();
        let previous = resources.try_get_value(&key);
        for case in corpus["cases"].as_array().unwrap() {
            resources.remove(&key);
            match case["resource"].as_str().unwrap() {
                "custom" => resources.set(
                    key.clone(),
                    Some(Rc::new(FontFamily::new("Resource Symbols"))),
                ),
                "wrong" => resources.set(key.clone(), Some(Rc::new("Not a FontFamily".to_owned()))),
                "absent" => (),
                other => panic!("Unexpected source resource {other}"),
            }
            let p = RibbonIconPresenter::new();
            p.set_foreground(Some(brush(0xffff0000)));
            p.set_icon_size(23.5);
            p.set_icon(Some(RibbonIcon::glyph_with_options(
                Some(units(&case["glyph"])),
                case["family"].as_str().map(RibbonString::from),
                case["color"].as_str().map(RibbonString::from),
            )));
            snapshot(&p, &case["output"]);
        }
        resources.remove(&key);
        if let Some(previous) = previous {
            resources.set(key, previous);
        }
        for case in corpus["clones"].as_array().unwrap() {
            let source = FontIcon::new();
            source.set_glyph(Some("\u{e000}".into()));
            source.set_font_family(FontFamily::new("Clone Font"));
            source.set_font_size(99.);
            source.set_font_weight(FontWeight::Bold);
            source.set_font_style(FontStyle::Italic);
            source.set_flow_direction(FlowDirection::RightToLeft);
            source.set_mirrored_when_right_to_left(true);
            match case["local"].as_str().unwrap() {
                "null" => source.set_foreground(None),
                "fixed" => source.set_foreground(Some(brush(0xff0000ff))),
                "unset" => (),
                other => panic!("Unknown source local {other}"),
            }
            let parent = Viewbox::new();
            parent.set_child(source.clone());
            let p = RibbonIconPresenter::new();
            p.set_foreground(Some(brush(0xffff0000)));
            p.set_icon_size(24.);
            p.set_icon(Some(Rc::new(source.clone())));
            snapshot(&p, &case["first"]);
            let clone = p.children().get(0).cast::<FontIcon>().unwrap();
            assert!(!clone.mirrored_when_right_to_left());
            assert!(clone.render_transform().is_none());
            assert_eq!(
                p.children().get(0).ptr_eq(&source),
                case["same"].as_bool().unwrap()
            );
            source.set_glyph(Some("Changed".into()));
            snapshot(&p, &case["before"]);
            p.set_icon_size(32.);
            snapshot(&p, &case["after"]);
            assert_eq!(
                parent.child().unwrap().ptr_eq(&source),
                case["parentRetained"].as_bool().unwrap()
            );
        }
    });
}
#[test]
fn native_font_object_inheritance_text_child_invalidation_and_lifetimes() {
    run(|| {
        let font = FontIcon::new();
        assert!(
            font.clone()
                .upcast::<Control>()
                .cast::<FontIcon>()
                .is_some()
        );
        font.set_use_layout_rounding(false);
        font.set_glyph(Some("a".into()));
        font.set_font_size(20.);
        let text = font.visual_children_snapshot().unwrap()[0]
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(text.text().as_deref(), Some("a"));
        assert!(text.parent().unwrap().ptr_eq(&font));
        font.measure(Size::INFINITY);
        font.arrange(Rect::from_size(font.desired_size()));
        let first = font.desired_size();
        assert!(first.width > 0. && first.height > 0.);
        font.set_glyph(Some("Longer".into()));
        assert!(!font.is_measure_valid());
        font.measure(Size::INFINITY);
        assert!(font.desired_size().width > first.width);
        font.set_font_style(FontStyle::Italic);
        font.set_font_weight(FontWeight::Bold);
        font.set_font_size(25.);
        font.set_foreground(Some(brush(0xffff0000)));
        assert_eq!(text.font_style(), FontStyle::Italic);
        assert_eq!(text.font_weight(), FontWeight::Bold);
        assert_eq!(text.font_size(), 25.);
        assert_eq!(text.foreground(), font.foreground());
        font.set_flow_direction(FlowDirection::RightToLeft);
        assert!(!font.has_mirror_transform());
        let weak = font.downgrade();
        let text_weak = text.downgrade();
        drop(text);
        drop(font);
        assert!(weak.upgrade().is_none());
        assert!(text_weak.upgrade().is_none());
    });
}
#[test]
fn font_settings_are_independent_of_ambient_text_but_foreground_inherits() {
    run(|| {
        let parent = ferroui_controls::primitives::TemplatedControl::new();
        parent.set_font_size(80.);
        parent.set_font_weight(FontWeight::Bold);
        parent.set_font_style(FontStyle::Italic);
        parent.set_font_family(FontFamily::new("Ambient Text"));
        parent.set_foreground(Some(brush(0xffff0000)));
        let font = FontIcon::new();
        font.set_glyph(Some("A".into()));
        font.set_parent(parent.clone());
        assert_eq!(font.font_size(), 20.);
        assert_eq!(font.font_weight(), FontWeight::Normal);
        assert_eq!(font.font_style(), FontStyle::Normal);
        assert_ne!(font.font_family(), parent.font_family());
        assert_eq!(font.foreground(), parent.foreground());
        let text = font.visual_children_snapshot().unwrap()[0]
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(text.font_size(), 20.);
        // Uno omits initial FontWeight synchronization: the child can inherit
        // ambient weight even while the icon's separate property is Normal.
        assert_eq!(text.font_weight(), FontWeight::Bold);
        assert_eq!(text.foreground(), parent.foreground());
        parent.set_font_size(100.);
        parent.set_foreground(Some(brush(0xff0000ff)));
        assert_eq!(font.font_size(), 20.);
        assert_eq!(text.foreground(), parent.foreground());
        font.set_parent(None);
    });
}
