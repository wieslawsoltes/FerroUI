use crate::RsxCatalog;
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::TextPatch;

const SOURCE: &str = r###"fn another_view() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() {
    rsx! {
        StackPanel {
            styles: [ferroui_view::style("Border#Card").animation(
                ferroui_view::animation(ferroui_view::ms(2000))
                    .key_frame(0.0, |key| key.set(ferroui_controls::Border::box_shadow_property(),
                        ferroui_base::media::BoxShadows::with_rest(
                            ferroui_base::media::BoxShadow {
                                offset_y: /* vertical 🧭 */ 8.0f64,
                                color: ferroui_base::media::Color::from_uint32(0x8026_3452),
                                ..ferroui_base::media::BoxShadow::default()
                            },
                            &[ferroui_base::media::BoxShadow::parse(r#"inset 4 8 12 2 #80325AC0"#).unwrap()]
                        )))
                    .key_frame(1.0, |key| key.set(ferroui_controls::Border::box_shadow_property(),
                        ferroui_base::media::BoxShadows::parse(r#"0 8 12 0 #80263452"#).expect("shadows")))
            )],
            Border { name: "Card", width: 180.0, height: 100.0 }
            TextBlock { text: dynamic_caption() }
        }
    }
}
"###;

#[test]
fn box_shadow_key_patches_keep_rust_comments_constructor_and_crlf() {
    let source = SOURCE.replace('\n', "\r\n");
    let caret = source.rfind("rsx!").unwrap();
    let catalog = RsxCatalog::built_in();
    let projected = catalog.project(&source, caret).unwrap();
    let markup = projected.preview_markup();
    let clip = markup.animations[0].1;
    let edit = |time, value| KeyframeEdit::Upsert {
        previous: Some(time),
        time,
        value,
        easing: "LinearEasing",
    };
    let structured = "0 -12 0 -3 #80263452, inset 4 8 12 2 #80325AC0";
    let edited = TextPatch::apply_all(
        &source,
        clip.patch_keyframe("BoxShadow", edit(0.0, structured))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        edited,
        source.replace("8.0f64", "-12.0f64").replace(
            "..ferroui_base::media::BoxShadow::default()",
            "spread: -3.0, ..ferroui_base::media::BoxShadow::default()"
        )
    );
    assert!(clip.patch_keyframe("BoxShadow", edit(0.0, "none")).is_err());
    let empty = TextPatch::apply_all(
        &source,
        clip.patch_keyframe("BoxShadow", edit(2.0, "none")).unwrap(),
    )
    .unwrap();
    assert_eq!(empty, source.replace("0 8 12 0 #80263452", "none"));
    let insert = KeyframeEdit::Upsert {
        previous: None,
        time: 1.0,
        value: structured,
        easing: "LinearEasing",
    };
    let inserted = TextPatch::apply_all(
        &source,
        clip.author_keyframe(&source, catalog, "BoxShadow", insert)
            .unwrap(),
    )
    .unwrap();
    assert!(inserted.contains(".key_frame(0.5,"));
    assert!(inserted.contains(&format!(
        "::ferroui_base::media::BoxShadows::parse({structured:?}).unwrap()"
    )));
    assert_eq!(
        inserted
            .lines()
            .filter(|line| !line.contains(".key_frame(0.5,"))
            .collect::<Vec<_>>(),
        source.lines().collect::<Vec<_>>()
    );
}

#[test]
fn box_shadow_base_property_uses_the_same_literal_source_mapping() {
    let source = r###"rsx! { Border { box_shadow: ::ferroui_base::media::BoxShadows::parse(
        /* ambient 🧭 */ r#"0 8 12 0 #80263452"#,
    ).expect("base shadows") } }"###;
    let projected = RsxCatalog::built_in().project(source, 0).unwrap();
    assert!(
        projected
            .xaml()
            .contains("BoxShadow=\"0 8 12 0 #80263452\"")
    );
    let patch = projected.set_property(0, "BoxShadow", "none").unwrap();
    assert_eq!(
        TextPatch::apply_all(source, vec![patch]).unwrap(),
        source.replace("0 8 12 0 #80263452", "none")
    );
}
