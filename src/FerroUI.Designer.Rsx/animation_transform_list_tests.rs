use crate::RsxCatalog;
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::TextPatch;

const SOURCE: &str = r###"fn summary() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#Card").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.0, |key| key.set(ferroui_base::Visual::render_transform_property(), None))
                .key_frame(1.0, |key| key.set(ferroui_base::Visual::render_transform_property(),
                    Some(ferroui_base::media::transformation::TransformOperations::parse(
                        /* preserve 🧭 */ r#"translate(28px, 4px) rotate(40deg)"#,
                    ).expect("lift") as std::rc::Rc<dyn ferroui_base::media::ITransform>)))
        )],
        Border { name: "Card", render_transform: r#"translate(4px, 0px) rotate(10deg)"# }
        TextBlock { text: dynamic_caption() }
    }
} }
"###;

#[test]
fn transform_list_keys_retain_constructors_and_surrounding_rust() {
    let source = SOURCE.replace('\n', "\r\n");
    let catalog = RsxCatalog::built_in();
    let projection = catalog
        .project(&source, source.rfind("rsx!").unwrap())
        .unwrap();
    assert!(
        projection
            .xaml()
            .contains(r#"RenderTransform="translate(4px, 0px) rotate(10deg)""#)
    );
    let markup = projection.preview_markup();
    let clip = markup.animations[0].1;
    let edit = |time, value| KeyframeEdit::Upsert {
        previous: Some(time),
        time,
        value,
        easing: "LinearEasing",
    };
    let revised = "matrix(1, 0.3, 0, 1, 30, 4) rotate(20deg)";
    let patch = clip
        .patch_keyframe("RenderTransform", edit(2.0, revised))
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patch).unwrap(),
        source.replace("translate(28px, 4px) rotate(40deg)", revised)
    );
    assert!(
        clip.patch_keyframe("RenderTransform", edit(2.0, "perspective(12px)"))
            .is_err()
    );
    let filled = clip
        .patch_keyframe("RenderTransform", edit(0.0, "rotate(10deg)"))
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, filled).unwrap(),
        source.replace(", None)", ", \"rotate(10deg)\")")
    );
    let inserted = clip
        .author_keyframe(
            &source,
            catalog,
            "RenderTransform",
            KeyframeEdit::Upsert {
                previous: None,
                time: 1.0,
                value: "rotate(30deg)",
                easing: "LinearEasing",
            },
        )
        .unwrap();
    let updated = TextPatch::apply_all(&source, inserted).unwrap();
    assert!(updated.contains("render_transform_property(), \"rotate(30deg)\""));
    assert_eq!(
        updated
            .lines()
            .filter(|line| !line.contains(".key_frame(0.5,"))
            .collect::<Vec<_>>(),
        source.lines().collect::<Vec<_>>()
    );
}

#[test]
fn direct_transform_properties_round_trip_their_literal_operand() {
    let source =
        r###"rsx! { Border { render_transform: /* turn */ r#"rotate(14deg) scale(1.2)"# } }"###;
    let projection = RsxCatalog::built_in().project(source, 0).unwrap();
    let draft = projection.xaml().replace("rotate(14deg)", "rotate(24deg)");
    let patched = TextPatch::apply_all(source, projection.apply_xaml(&draft).unwrap()).unwrap();
    assert_eq!(patched, source.replace("rotate(14deg)", "rotate(24deg)"));
}
