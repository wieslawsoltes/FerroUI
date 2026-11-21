//! Local source helper/caller decisions and actual native child/ownership state.
use crate::{
    compat::{BitmapIcon, BitmapIconSource},
    controls::RibbonItemHelper,
    primitives::RibbonIconPresenter,
};
use ferroui_base::{
    Ref,
    media::{Color, LinearGradientBrush, SolidColorBrush},
    styling::ThemeVariant,
    utilities::{Uri, UriKind},
};
use ferroui_controls::{Control, Grid, Image, StackPanel, Viewbox};
use serde_json::{Value, json};
use std::rc::Rc;

fn child(icon: &BitmapIcon) -> Ref<Image> {
    icon.visual_children_snapshot().unwrap()[0]
        .cast::<Grid>()
        .unwrap()
        .children()
        .get(0)
        .cast::<Image>()
        .unwrap()
}
fn uri(value: &str) -> Uri {
    Uri::new(value, UriKind::RelativeOrAbsolute).unwrap()
}
fn snapshot(root: Ref<Control>) -> Value {
    let viewbox = root.cast::<Viewbox>();
    let icon = viewbox
        .as_ref()
        .map(|v| v.child().unwrap())
        .unwrap_or(root.clone())
        .cast::<BitmapIcon>()
        .unwrap();
    let dimension = |value: f64| {
        if value.is_nan() {
            Value::Null
        } else {
            json!(value)
        }
    };
    let local = icon.has_local_value_binding(
        ferroui_controls::primitives::TemplatedControl::foreground_property().as_property(),
    ) || icon
        .get_local_value(ferroui_controls::primitives::TemplatedControl::foreground_property())
        .is_some();
    let color = icon.foreground().and_then(|b| {
        b.as_solid_color_brush().map(|b| {
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                b.color().a,
                b.color().r,
                b.color().g,
                b.color().b
            )
        })
    });
    json!({"root":if viewbox.is_some(){"Viewbox"}else{"BitmapIcon"},"kind":"BitmapIcon","uri":icon.uri_source().map(|u|u.original_string().to_owned()),"mono":icon.show_as_monochrome(),"width":dimension(icon.width()),"height":dimension(icon.height()),"boxWidth":viewbox.as_ref().map(|v|v.width()),"boxHeight":viewbox.as_ref().map(|v|v.height()),"color":color,"local":local})
}
#[test]
fn bitmap_caller_snapshot_copy_sets_parented_identity_and_foreground_match_64_source_cases() {
    crate::tests::run(|| {
        let cases: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/bitmap-snapshots.json"))
                .unwrap();
        for case in cases["corpus"]["cases"].as_array().unwrap() {
            let is_source = case["kind"] == "source";
            let mono = case["mono"].as_bool().unwrap();
            let fg = case["fg"].as_str().unwrap();
            let initial = case["uri"].as_str().map(uri);
            let source = BitmapIconSource::new();
            let icon = BitmapIcon::new();
            let parent = StackPanel::new();
            if is_source {
                source.set_uri_source(initial);
                source.set_show_as_monochrome(mono);
                if fg == "blue" {
                    source.set_foreground(Some(SolidColorBrush::from_uint32(0xff0000ff).into()));
                }
            } else {
                icon.set_uri_source(initial);
                icon.set_show_as_monochrome(mono);
                icon.set_width(99.);
                icon.set_height(88.);
                if fg != "unset" {
                    icon.set_foreground(
                        (fg == "blue").then(|| SolidColorBrush::from_uint32(0xff0000ff).into()),
                    );
                }
                if case["parented"] == true {
                    parent.children().add(icon.clone());
                }
            }
            let input: ferroui_base::BoxedValue = if is_source {
                Rc::new(source.clone())
            } else {
                Rc::new(icon.clone())
            };
            let presenter = RibbonIconPresenter::new();
            presenter.set_foreground(Some(SolidColorBrush::from_uint32(0xffff0000).into()));
            presenter.set_icon_size(23.5);
            presenter.set_icon(Some(input.clone()));
            let menu = RibbonItemHelper::create_menu_icon(Some(input.clone()))
                .unwrap()
                .unwrap();
            let compare = |step: &str, root: Ref<Control>| {
                let actual = snapshot(root);
                let mut expected = case[step].clone();
                for property in ["width", "height", "boxWidth", "boxHeight"] {
                    if let Some(value) = expected[property].as_f64() {
                        expected[property] = json!(value);
                    }
                }
                if expected["color"].is_null() && expected["local"] == false {
                    expected["color"] = json!("#FFFFFFFF");
                }
                assert_eq!(actual, expected, "{case} {step}");
            };
            compare("first", presenter.children().get(0));
            compare("menuFirst", menu.clone().upcast());
            let later = Some(uri("ferres://bitmap_icon_probe/later.png"));
            if is_source {
                source.set_uri_source(later);
                source.set_show_as_monochrome(!mono);
                source.set_foreground(None);
            } else {
                icon.set_uri_source(later);
                icon.set_show_as_monochrome(!mono);
                icon.set_foreground(None);
            }
            compare("unchanged", presenter.children().get(0));
            compare("menuUnchanged", menu.upcast());
            presenter.set_icon_size(32.);
            compare("rebuilt", presenter.children().get(0));
            compare(
                "menuRebuilt",
                RibbonItemHelper::create_menu_icon(Some(input))
                    .unwrap()
                    .unwrap()
                    .upcast(),
            );
            if case["parented"] == true {
                assert!(parent.children().get(0).ptr_eq(&icon));
            }
        }
    });
}

#[test]
fn bitmap_monochrome_flag_and_brush_snapshot_cache_match_whole_source_helper() {
    crate::tests::run(|| {
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/bitmap-icon.json")).unwrap();
        let icon = BitmapIcon::new();
        let image = child(&icon);
        let brush = SolidColorBrush::with_color_and_opacity(Color::from_uint32(0x80ff0000), 0.25);
        // Private cached color is observed by the child module, as the source oracle
        // observes its internal Image.MonochromeColor. Loading is tested separately.
        let read = |step: &str| {
            let expected = fixture["corpus"]["trace"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["step"] == step)
                .unwrap();
            assert_eq!(
                icon.cached_bitmap_tint_for_test().map(|v| v.to_uint32()),
                expected["color"].as_u64().map(|v| v as u32),
                "{step}"
            );
            assert_eq!(
                icon.show_as_monochrome(),
                expected["mono"].as_bool().unwrap()
            );
            assert!(child(&icon).ptr_eq(&image));
        };
        read("default");
        icon.set_foreground(Some(brush.clone().into()));
        read("red-opacity");
        brush.set_color(Color::from_uint32(0xff00ff00));
        read("mutate-brush");
        icon.set_foreground(Some(brush.into()));
        read("same-brush");
        icon.set_value(
            ThemeVariant::actual_theme_variant_property(),
            Some(ThemeVariant::dark()),
        );
        read("theme-refresh");
        icon.set_show_as_monochrome(false);
        read("off");
        icon.set_foreground(Some(SolidColorBrush::from_uint32(0xff0000ff).into()));
        read("blue-off");
        icon.set_show_as_monochrome(true);
        read("on");
        icon.set_foreground(Some(LinearGradientBrush::new().into()));
        read("gradient");
        icon.set_foreground(None);
        read("null");
        icon.set_foreground(Some(SolidColorBrush::from_uint32(0x00ff0000).into()));
        read("transparent-color");
        for value in [
            Some("ferres://FerroRibbon/Assets/test.png"),
            Some("images/a%20b.png"),
            Some("ms-appx:///Assets/icon.png"),
            None,
        ] {
            icon.set_uri_source(value.map(uri));
            assert_eq!(
                icon.uri_source().as_ref().map(|u| u.original_string()),
                value
            );
            assert!(child(&icon).ptr_eq(&image));
        }
        let weak = icon.downgrade();
        let weak_image = image.downgrade();
        drop(image);
        drop(icon);
        assert!(weak.upgrade().is_none() && weak_image.upgrade().is_none());
    });
}

#[test]
fn bitmap_source_assignment_reentry_keeps_latest_request_and_tint_retains_decoded_source() {
    crate::tests::run(|| {
        use std::cell::Cell;
        ferroui_base::platform::register_assets(
            "bitmap_icon_reentry",
            &[(
                "second.png",
                include_bytes!("../../tests/fixtures/images/bitmap-mask-3x2.png"),
            )],
        );
        let icon = BitmapIcon::new();
        let image = child(&icon);
        assert!(image.has_local_value_binding(Image::source_property().as_property()));
        let changes = Rc::new(Cell::new(0));
        let observed = changes.clone();
        let weak = icon.downgrade();
        let fired = Cell::new(false);
        let token = image.add_property_changed_handler(Rc::new(move |change| {
            if change.property() == Image::source_property().as_property() {
                observed.set(observed.get() + 1);
                if !fired.replace(true) {
                    weak.upgrade()
                        .unwrap()
                        .set_uri_source(Some(uri("ferres://bitmap_icon_reentry/second.png")));
                }
            }
        }));
        icon.set_uri_source(Some(uri("ferres://bitmap_icon_reentry/missing-first.png")));
        assert_eq!(
            icon.uri_source().unwrap().original_string(),
            "ferres://bitmap_icon_reentry/second.png"
        );
        assert!(icon.load_error().is_none());
        assert_eq!(changes.get(), 2);
        let source = image.source().unwrap();
        assert!(source.as_bitmap().is_some());
        let bitmap =
            source.as_bitmap().unwrap() as *const dyn ferroui_base::media::imaging::IBitmap;
        icon.set_foreground(Some(SolidColorBrush::from_uint32(0xffff0000).into()));
        icon.set_show_as_monochrome(false);
        icon.set_foreground(Some(SolidColorBrush::from_uint32(0xff0000ff).into()));
        icon.set_show_as_monochrome(true);
        assert_eq!(changes.get(), 2);
        let current = image.source().unwrap();
        assert!(Rc::ptr_eq(&source, &current));
        assert!(std::ptr::addr_eq(bitmap, current.as_bitmap().unwrap()));
        icon.set_uri_source(Some(uri("ferres://bitmap_icon_reentry/missing-last.png")));
        assert!(icon.load_error().is_some());
        assert!(image.source().unwrap().as_bitmap().is_none());
        assert_eq!(changes.get(), 3);
        image.remove_property_changed_handler(token);
        let old = Rc::downgrade(&source);
        drop(current);
        drop(source);
        assert!(old.upgrade().is_none());
        icon.set_uri_source(None);
        assert!(image.source().is_none() && icon.load_error().is_none());
    });
}

#[test]
fn bitmap_load_error_belongs_to_the_displayed_source() {
    use crate::primitives::{RibbonDeferredImage, RibbonImageLoadError};
    use ferroui_base::media::{DrawingImage, IImage};

    crate::tests::run(|| {
        let icon = BitmapIcon::new();
        let image = child(&icon);
        icon.set_uri_source(Some(uri(
            "ferres://FerroRibbon/Assets/missing-test-icon.png",
        )));
        let failure = image.source().unwrap();
        let error = icon.load_error().unwrap();
        assert!(matches!(&*error, RibbonImageLoadError::Load(_)));

        let replacement: Rc<dyn IImage> = DrawingImage::new().into();
        image.set_source(Some(replacement.clone()));
        assert!(Rc::ptr_eq(&image.source().unwrap(), &replacement));
        assert!(
            icon.load_error().is_none(),
            "an old URI failed, not this image"
        );

        image.set_source(Some(failure.clone()));
        assert!(Rc::ptr_eq(&icon.load_error().unwrap(), &error));
        let failed_source = failure
            .as_object()
            .and_then(|object| object.downcast_ref::<RibbonDeferredImage>())
            .unwrap();
        let notified = Rc::new(std::cell::Cell::new(false));
        let observed = notified.clone();
        let _completion = failed_source.completed(move |result| {
            assert!(Rc::ptr_eq(result.as_ref().unwrap_err(), &error));
            assert!(!observed.replace(true));
        });
        assert!(notified.get());

        image.clear_local_value_and_binding(Image::source_property());
        assert!(image.source().is_none());
        assert!(icon.load_error().is_none());
    });
}

#[test]
fn bitmap_native_source_binding_clear_replacement_and_owner_release() {
    crate::tests::run(|| {
        use ferroui_base::media::{DrawingImage, IImage};
        use ferroui_base::{
            data::BindingPriority,
            reactive::{IObserver, LightweightSubject},
        };
        ferroui_base::platform::register_assets(
            "bitmap_binding_lifetime",
            &[
                (
                    "mask.png",
                    include_bytes!("../../tests/fixtures/images/bitmap-mask-3x2.png"),
                ),
                (
                    "second.png",
                    include_bytes!("../../tests/fixtures/images/bitmap-3x2.png"),
                ),
            ],
        );
        let icon = BitmapIcon::new();
        let image = child(&icon);
        let property = Image::source_property();
        assert!(image.has_local_value_binding(property.as_property()));
        icon.set_uri_source(Some(uri("ferres://bitmap_binding_lifetime/mask.png")));
        let manual: Rc<dyn IImage> = DrawingImage::new().into();
        image.set_source(Some(manual.clone()));
        assert!(Rc::ptr_eq(&manual, &image.source().unwrap()));
        assert!(image.has_local_value_binding(property.as_property()));
        icon.set_uri_source(Some(uri("ferres://bitmap_binding_lifetime/second.png")));
        let source = image.source().unwrap();
        assert!(!Rc::ptr_eq(&manual, &source));
        assert!(source.as_bitmap().is_some());
        let retired = Rc::downgrade(&source);
        drop(source);
        image.clear_local_value_and_binding(property);
        assert!(image.source().is_none());
        assert!(!image.has_local_value_binding(property.as_property()));
        assert!(retired.upgrade().is_none());
        icon.set_uri_source(Some(uri("ferres://bitmap_binding_lifetime/missing.png")));
        assert!(image.source().is_none());
        // Detached URI bindings perform no loading or error-state updates.
        assert!(icon.load_error().is_none());

        let replacement = Rc::new(LightweightSubject::<Option<Rc<dyn IImage>>>::new());
        let binding = image.bind(property, replacement.clone(), BindingPriority::LocalValue);
        replacement.on_next(None);
        let weak = icon.downgrade();
        drop(icon);
        assert!(weak.upgrade().is_none());
        // Dropping the owner disposes only its own native subscription. It must
        // not clear an application's newer binding on the retained Image.
        assert!(image.has_local_value_binding(property.as_property()));
        binding.dispose();
        assert!(!image.has_local_value_binding(property.as_property()));

        let owner = BitmapIcon::new();
        let retained_child = child(&owner);
        let weak = owner.downgrade();
        drop(owner);
        assert!(weak.upgrade().is_none());
        assert!(!retained_child.has_local_value_binding(property.as_property()));
    });
}

#[test]
fn bitmap_filter_belongs_to_retained_image_across_source_replacement() {
    crate::tests::run(|| {
        use ferroui_base::media::{DrawingImage, IImage};
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/bitmap-icon.json")).unwrap();
        let icon = BitmapIcon::new();
        let image = child(&icon);
        image.clear_local_value_and_binding(Image::source_property());
        let peer =
            ferroui_controls::automation::peers::ControlAutomationPeer::create_peer_for_element(
                &image,
            );
        assert_eq!(peer.get_class_name(), "Image");
        assert_eq!(
            peer.get_automation_control_type(),
            ferroui_controls::automation::peers::AutomationControlType::Image
        );
        let first: Rc<dyn IImage> = DrawingImage::new().into();
        let second: Rc<dyn IImage> = DrawingImage::new().into();
        let read = |step: &str| {
            let source = image.source();
            let name = source.as_ref().map(|source| {
                if Rc::ptr_eq(source, &first) {
                    "first"
                } else {
                    assert!(Rc::ptr_eq(source, &second));
                    "second"
                }
            });
            let expected = fixture["corpus"]["replacements"]
                .as_array()
                .unwrap()
                .iter()
                .find(|state| state["step"] == step)
                .unwrap();
            assert_eq!(
                json!({"step":step,"mono":icon.show_as_monochrome(),"color":icon.cached_bitmap_tint_for_test().map(|color| color.to_uint32()),"source":name}),
                *expected
            );
            assert!(child(&icon).ptr_eq(&image));
        };
        icon.set_foreground(Some(SolidColorBrush::from_uint32(0xff0000ff).into()));
        image.set_source(Some(first.clone()));
        read("first-blue");
        icon.set_foreground(Some(SolidColorBrush::from_uint32(0xffff0000).into()));
        read("first-red");
        icon.set_show_as_monochrome(false);
        read("off");
        image.set_source(Some(second.clone()));
        read("second-off");
        let brush = SolidColorBrush::from_uint32(0xff00ff00);
        icon.set_foreground(Some(brush.clone().into()));
        read("green-off");
        icon.set_show_as_monochrome(true);
        read("second-green");
        brush.set_color(Color::from_uint32(0xffff00ff));
        read("mutate-brush");
        icon.set_value(
            ThemeVariant::actual_theme_variant_property(),
            Some(ThemeVariant::dark()),
        );
        read("theme-pink");
        image.set_source(None);
        read("empty-pink");
        image.set_source(Some(first.clone()));
        read("first-pink");
        let old_first = Rc::downgrade(&first);
        let old_second = Rc::downgrade(&second);
        image.set_source(None);
        drop(first);
        drop(second);
        assert!(old_first.upgrade().is_none() && old_second.upgrade().is_none());
    });
}
