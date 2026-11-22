use crate::RsxCatalog;
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::TextPatch;

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
