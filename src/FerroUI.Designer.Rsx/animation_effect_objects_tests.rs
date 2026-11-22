use crate::{project_rust, RsxCatalog};
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::TextPatch;

const SOURCE: &str = r#"fn summary() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() {
    rsx! {
        StackPanel {
            styles: [ferroui_view::style("Border#Card").animation(
                ferroui_view::animation(ferroui_view::ms(2000))
                    .key_frame(0.0, |key| key.set(ferroui_base::Visual::effect_property(),
                        ::std::rc::Rc::new(ferroui_base::media::ImmutableDropShadowDirectionEffect::new(
                            /* angle 🧭 */ 30.0f64, 4.0, 2.0, ferroui_base::media::Color::from_argb(128, 35, 69, 103), 0.25
                        )) as ::std::rc::Rc<dyn ::ferroui_base::media::IEffect>))
                    .key_frame(1.0, |key| key.set(ferroui_base::Visual::effect_property(),
                        Some(std::rc::Rc::new(ferroui_base::media::ImmutableDropShadowDirectionEffect::new(
                            150.0, 20.0, 10.0, ferroui_base::media::Color::from_argb(128, 35, 69, 103), 0.75
                        )) as std::rc::Rc<dyn ferroui_base::media::IEffect>)))
                    .fill_mode(ferroui_base::animation::FillMode::Both)
            )],
            Border { name: "Card", width: 180.0, height: 100.0 }
            TextBlock { text: dynamic_caption() }
        }
    }
}
"#;

#[test]
fn directional_constructor_edits_and_records_keep_the_surrounding_rust() {
    let source = SOURCE.replace('\n', "\r\n");
    let caret = source.rfind("rsx!").unwrap();
    let catalog = RsxCatalog::built_in();
    let projected = catalog.project(&source, caret).unwrap();
    assert!(projected.xaml().contains("<DropShadowDirectionEffect "));
    let clip = projected.preview_markup().animations[0].1;
    let edit = KeyframeEdit::Upsert {
        previous: Some(0.0),
        time: 0.0,
        easing: "LinearEasing",
        value: r##"<DropShadowDirectionEffect Direction="52" ShadowDepth="4" BlurRadius="2" Opacity="0.25" Color="#80234567"/>"##,
    };
    let edited =
        TextPatch::apply_all(&source, clip.patch_keyframe("Effect", edit).unwrap()).unwrap();
    assert_eq!(
        edited,
        source.replace("/* angle 🧭 */ 30.0f64", "/* angle 🧭 */ 52.0f64")
    );
    let incompatible = KeyframeEdit::Upsert {
        previous: Some(0.0),
        time: 0.0,
        easing: "LinearEasing",
        value: "blur(8)",
    };
    assert!(clip.patch_keyframe("Effect", incompatible).is_err());
    let insert = KeyframeEdit::Upsert {
        previous: None,
        time: 1.0,
        easing: "LinearEasing",
        value: r##"<DropShadowDirectionEffect Direction="90" ShadowDepth="12" BlurRadius="6" Opacity="0.5" Color="#80234567"/>"##,
    };
    let patched = TextPatch::apply_all(
        &source,
        clip.author_keyframe(&source, catalog, "Effect", insert)
            .unwrap(),
    )
    .unwrap();
    assert!(patched.contains("ImmutableDropShadowDirectionEffect::new(90.0, 12.0, 6.0"));
    assert_eq!(
        patched
            .lines()
            .filter(|line| !line.contains(".key_frame(0.5,"))
            .collect::<Vec<_>>(),
        source.lines().collect::<Vec<_>>()
    );
    let restored = project_rust(&patched, caret).unwrap();
    assert!(restored
        .xaml()
        .contains("Direction=\"90\" ShadowDepth=\"12\""));
}
