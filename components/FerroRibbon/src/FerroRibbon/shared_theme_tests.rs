use crate::{controls::RibbonKeyTip, tests::run, theming::RibbonThemeResources};
use ferroui_base::{
    Ref, StaticType, StyledElementImpl, VisualImpl, ferro_class, ferro_impl_classes, instantiate,
};
use ferroui_base::{
    controls::{IResourceProvider, ResourceDictionary},
    data::core::ValueTypes,
    input::{InputElementImpl, KeyboardNavigation},
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    styling::ControlTheme,
};
use ferroui_controls::{
    Application, Border, Button, Control, ControlImpl, TextBlock, Window,
    primitives::TemplatedControlImpl,
};
use std::rc::Rc;
fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!("../../tests/fixtures/keytip-badge.json")).unwrap()
}
fn source_styles() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../tests/fixtures/shared-theme-source.json"
    ))
    .unwrap()
}
fn theme(r: &RibbonThemeResources, key: &str) -> Ref<ControlTheme> {
    ValueTypes::as_object(&*r.try_get_resource(&key.into(), None).flatten().unwrap())
        .unwrap()
        .cast()
        .unwrap()
}
fn render(root: Ref<Control>) -> Ref<Window> {
    let w = Window::new();
    w.set_width(600.);
    w.set_height(400.);
    w.set_content(Some(Control::boxed(root)));
    w.show();
    w.measure(ferroui_base::Size::new(600., 400.));
    w.arrange(ferroui_base::Rect::new(0., 0., 600., 400.));
    w
}
fn child<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    let mut queue = vec![root.to_ref()];
    while let Some(c) = queue.pop() {
        if c.name().as_deref() == Some(name) {
            return c.cast().unwrap();
        }
        for v in c.visual_children().snapshot().iter() {
            if let Some(c) = v.cast::<Control>() {
                queue.push(c);
            }
        }
    }
    panic!("missing template part {name}")
}
#[test]
fn source_badge_defaults_valid_text_attached_values_and_target_ownership() {
    run(|| {
        let c = corpus();
        let b = RibbonKeyTip::new();
        assert_eq!(b.text().as_deref(), Some(""));
        assert!(b.target().is_none());
        assert!(!b.is_hit_test_visible());
        assert!(!KeyboardNavigation::get_is_tab_stop(&b));
        assert_eq!(b.style_key(), <RibbonKeyTip as StaticType>::TYPE);
        for row in c["cases"].as_array().unwrap().iter().take(7) {
            let value = if row["value"].is_null() {
                None
            } else {
                Some(
                    String::from_utf16(
                        &row["value"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_u64().unwrap() as u16)
                            .collect::<Vec<_>>(),
                    )
                    .unwrap(),
                )
            };
            b.set_text(value.clone());
            let target = Border::new();
            RibbonKeyTip::set_key_tip(&target, value.clone());
            assert_eq!(b.text(), value);
            assert_eq!(RibbonKeyTip::get_key_tip(&target), value);
        }
        // The eighth source case stores a lone surrogate. Native String cannot do so;
        // this remains an explicit UI string API gap, not a passing source case.
        assert_eq!(c["cases"][7]["value"], serde_json::json!([55296]));
        let target = Button::new();
        let weak = target.downgrade();
        b.target.replace(Some(target.clone().upcast()));
        drop(target);
        assert!(weak.upgrade().is_some());
        b.target.replace(None);
        assert!(weak.upgrade().is_none());
    });
}
#[repr(C)]
struct DerivedBadge {
    base: RibbonKeyTip,
}
ferro_class!(DerivedBadge:RibbonKeyTip);
ferro_impl_classes!(DerivedBadge:ferroui_base::FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
#[test]
fn default_resource_constructor_loads_compiled_shared_and_badge_keeps_default_style_key() {
    run(|| {
        let r = RibbonThemeResources::new();
        assert_eq!(r.merged_dictionaries().count(), 1);
        let source = source_styles();
        assert_eq!(source.as_array().unwrap().len(), 15);
        for s in source.as_array().unwrap() {
            if let Some(key) = s["key"].as_str() {
                let shared = r
                    .merged_dictionaries()
                    .get(0)
                    .as_object()
                    .unwrap()
                    .to_ref()
                    .cast::<ResourceDictionary>()
                    .unwrap();
                assert!(shared.contains_key(&key.into()), "{key}");
            }
        }
        let badge_theme = r
            .try_get_resource(&<RibbonKeyTip as StaticType>::TYPE.into(), None)
            .flatten()
            .unwrap();
        let badge_theme = ValueTypes::as_object(&*badge_theme)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap();
        let badge: Ref<DerivedBadge> = instantiate(DerivedBadge {
            base: RibbonKeyTip::construct(),
        });
        assert_eq!(badge.style_key(), <RibbonKeyTip as StaticType>::TYPE);
        badge.set_theme(badge_theme);
        badge.set_text(Some("AB".into()));
        let w = render(badge.clone().upcast());
        let text = badge
            .get_visual_descendants()
            .into_iter()
            .find_map(|v| v.cast::<TextBlock>())
            .unwrap();
        assert_eq!(text.text().as_deref(), Some("AB"));
        assert_eq!(
            text.text_line_bounds(),
            ferroui_base::media::TextLineBounds::Tight
        );
        assert_eq!(badge.font_size(), 11.);
        w.close();
    });
}
#[test]
fn native_shared_chrome_template_content_ownership_state_precedence_and_live_brushes() {
    run(|| {
        let r = RibbonThemeResources::new();
        let app = Application::current().unwrap();
        let provider: Rc<dyn IResourceProvider> = r.clone().upcast::<ResourceDictionary>().into();
        app.resources().merged_dictionaries().add(provider.clone());
        let b = Button::new();
        b.set_theme(theme(&r, "RibbonChromeButtonStyle"));
        let content = Border::new();
        content.set_width(12.);
        content.set_height(10.);
        b.set_content(Some(Control::boxed(content.clone())));
        let w = render(b.clone().upcast());
        assert!(b.presenter().is_some());
        assert!(content.parent().unwrap().ptr_eq(&b));
        let surface = child::<Border>(&b, "RootGrid");
        b.pseudo_classes().set(":pointerover", true);
        assert_eq!(
            surface.background(),
            Some(r.get_brush("RibbonItemHoverBrush", None).unwrap().into())
        );
        b.pseudo_classes().set(":pressed", true);
        assert_eq!(
            surface.background(),
            Some(r.get_brush("RibbonItemPressedBrush", None).unwrap().into())
        );
        b.set_is_enabled(false);
        assert_eq!(surface.background(), b.background());
        assert_eq!(
            b.presenter().unwrap().foreground(),
            Some(
                r.get_brush("RibbonDisabledForegroundBrush", None)
                    .unwrap()
                    .into()
            )
        );
        b.set_is_enabled(true);
        b.pseudo_classes().set(":pressed", false);
        b.pseudo_classes().set(":pointerover", false);
        r.set_color(
            "RibbonForegroundBrush",
            ferroui_base::media::Color::from_argb(255, 9, 8, 7),
            None,
        );
        assert_eq!(
            b.foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            ferroui_base::media::Color::from_argb(255, 9, 8, 7)
        );
        w.close();
        app.resources().merged_dictionaries().remove(&provider);
    });
}
#[test]
fn inherited_popup_themes_preserve_native_default_templates() {
    run(|| {
        let app = Application::current().unwrap();
        let simple = ferroui_themes_simple::SimpleTheme::new();
        let style = simple.as_style();
        app.styles().add(style.clone());
        let r = RibbonThemeResources::new();
        let provider: Rc<dyn IResourceProvider> = r.clone().upcast::<ResourceDictionary>().into();
        app.resources().merged_dictionaries().add(provider.clone());
        for key in [
            "RibbonToolTipStyle",
            "RibbonFlyoutPresenterStyle",
            "RibbonMenuFlyoutPresenterStyle",
        ] {
            let t = theme(&r, key);
            assert!(t.based_on().is_some(), "{key}");
        }
        app.resources().merged_dictionaries().remove(&provider);
        app.styles().remove(style);
    });
}
