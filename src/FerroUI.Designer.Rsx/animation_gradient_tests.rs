use crate::RsxCatalog;
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::TextPatch;

mod shared;

const SOURCE: &str = r###"fn header() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#Card").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.0, |key| key.set(ferroui_controls::Border::background_property(), "#223344"))
                .key_frame(1.0, |key| key.set(ferroui_controls::Border::background_property(),
                    Some(std::rc::Rc::new(ferroui_base::media::immutable::ImmutableLinearGradientBrush::from_stops(&[
                        ferroui_base::media::immutable::ImmutableGradientStop::new(/* 🧭 offset */ 0.25f64, ferroui_base::media::Color::from_uint32(0xFF223344)),
                        /* Keep stop */ ferroui_base::media::immutable::ImmutableGradientStop::new(1.0, ferroui_base::media::Color::from_uint32(0xFF56BBAA)),
                    ])) as std::rc::Rc<dyn ferroui_base::media::IBrush>)))
        )],
        Border { name: "Card", background: "#223344" }
        TextBlock { text: dynamic_caption() }
    }
} }
"###;

#[test]
fn gradient_key_edits_preserve_rust_source_and_round_trip_new_stops() {
    let source = SOURCE.replace('\n', "\r\n");
    let catalog = RsxCatalog::built_in();
    let projection = catalog
        .project(&source, source.rfind("rsx!").unwrap())
        .unwrap();
    let markup = projection.preview_markup();
    let clip = markup.animations[0].1;
    let edit = |time, previous, value| KeyframeEdit::Upsert {
        previous,
        time,
        value,
        easing: "LinearEasing",
    };
    let exact = r##"<LinearGradientBrush TransformOrigin="0%,0%">
        <GradientStop Offset="0.42" Color="#FF223344"/><GradientStop Offset="1" Color="#FF56BBAA"/>
    </LinearGradientBrush>"##;
    let patches = clip
        .patch_keyframe("Background", edit(2.0, Some(2.0), exact))
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patches).unwrap(),
        source.replace("0.25f64", "0.42f64")
    );

    let draft = exact
        .replace("TransformOrigin", "StartPoint=\"10%,20%\" TransformOrigin")
        .replace(
            "</LinearGradientBrush>",
            r##"<GradientStop Offset="0.8" Color="#FF335577"/></LinearGradientBrush>"##,
        );
    let patched = clip
        .patch_keyframe("Background", edit(2.0, Some(2.0), &draft))
        .unwrap();
    let updated = TextPatch::apply_all(&source, patched).unwrap();
    assert!(updated.contains("ImmutableLinearGradientBrush::new(&["));
    assert!(updated.contains("/* 🧭 offset */ 0.42f64"));
    assert!(updated.contains("/* Keep stop */"));
    assert!(updated.ends_with("    }\r\n} }\r\n"));
    let rebuilt = catalog
        .project(&updated, updated.rfind("rsx!").unwrap())
        .unwrap();
    assert_eq!(rebuilt.preview_markup().animations.len(), 1);

    let inserted = clip
        .author_keyframe(&source, catalog, "Background", edit(1.0, None, exact))
        .unwrap();
    let with_key = TextPatch::apply_all(&source, inserted).unwrap();
    assert!(with_key.contains("ImmutableLinearGradientBrush::new("));
    assert!(with_key.contains("0.25f64"));
}

#[test]
fn circular_radial_key_retains_shared_radius_and_expands_for_independent_radii() {
    let source = SOURCE
        .replace(
            "ImmutableLinearGradientBrush::from_stops",
            "ImmutableRadialGradientBrush::with_radius",
        )
        .replace(
            "])) as std::rc::Rc",
            concat!(
                "], 1.0, None, None, ferroui_base::media::GradientSpreadMethod::Pad, None, None, ",
                "/* 🧭 radius */ 0.35f64,)) as std::rc::Rc",
            ),
        )
        .replace('\n', "\r\n");
    let catalog = RsxCatalog::built_in();
    let project = |source: &str| {
        catalog
            .project(source, source.rfind("rsx!").unwrap())
            .unwrap()
            .preview_markup()
            .animations[0]
            .1
            .clone()
    };
    let clip = project(&source);
    assert!(clip.xaml.contains("RadiusX=\"35%\" RadiusY=\"35%\""));
    let circular = r##"<RadialGradientBrush RadiusX="42%" RadiusY="42%" TransformOrigin="0%,0%">
        <GradientStop Offset="0.25" Color="#FF223344"/><GradientStop Offset="1" Color="#FF56BBAA"/>
    </RadialGradientBrush>"##;
    let patch = |clip: &crate::RsxAnimation, source: &str, value: &str| {
        let patches = clip
            .patch_keyframe(
                "Background",
                KeyframeEdit::Upsert {
                    previous: Some(2.0),
                    time: 2.0,
                    value,
                    easing: "LinearEasing",
                },
            )
            .unwrap();
        TextPatch::apply_all(source, patches).unwrap()
    };
    let shared = patch(&clip, &source, circular);
    assert_eq!(shared, source.replace("0.35f64", "0.42f64"));
    let elliptical = circular.replace("RadiusX=\"42%\"", "RadiusX=\"18\"");
    let expanded = patch(&project(&shared), &shared, &elliptical);
    assert!(expanded.contains("ImmutableRadialGradientBrush::new("));
    assert!(expanded.contains("/* 🧭 radius */ Some("));
    assert!(expanded.contains("None,)) as std::rc::Rc"));
    assert!(expanded.contains("/* Keep stop */"));
    assert!(expanded.starts_with(shared.split("fn view()").next().unwrap()));
    assert_eq!(
        expanded.matches('\n').count(),
        expanded.matches("\r\n").count()
    );
    assert!(project(&expanded).xaml.contains("RadiusX=\"18\" RadiusY=\"42%\""));
}
