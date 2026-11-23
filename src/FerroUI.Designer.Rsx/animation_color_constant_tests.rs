use crate::{RsxAnimation, RsxCatalog};
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::TextPatch;

const SOURCE: &str = r###"use ferroui_view::prelude::*;
use ferroui_controls::Border as Card;
use ferroui_base::media::{Color, IBrush, immutable::ImmutableSolidColorBrush as Solid};
use std::rc::Rc;

mod brand {
    use ferroui_base::media::Color as Paint;
    pub const PACKED: core::primitive::u32 = 0xFF32_4050_u32;
    pub const RED: u8 = 0x32_u8;
    pub const INK: Paint = Paint { a: 255, r: RED, g: 64, b: 80 };
}
use self::brand::PACKED as BASE;
const INK: Color = brand::INK;

fn other_view() -> Element { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() -> Element { rsx! {
    StackPanel {
        styles: [style("Border#Card").animation(animation(ms(2000))
            .key_frame(0.0, |key| key.set(Card::background_property(),
                Some(Rc::new(Solid::from_uint32(BASE)) as Rc<dyn IBrush>)))
            .key_frame(1.0, |key| key.set(Card::background_property(),
                Some(Rc::new(Solid::from_uint32((brand::/* end 🧭 */PACKED))) as Rc<dyn IBrush>)))
            .fill_mode(ferroui_base::animation::FillMode::Both)
        )],
        Border { name: "Card", width: 176.0, height: 94.0 }
    }
} }
"###;

fn project(source: &str) -> crate::RsxProjection<'_> {
    RsxCatalog::built_in()
        .project(source, source.rfind("rsx!").unwrap())
        .unwrap()
}

fn clip(source: &str) -> RsxAnimation {
    project(source).preview_markup().animations[0].1.clone()
}

fn edit(previous: Option<f64>, time: f64, value: &str) -> KeyframeEdit<'_> {
    KeyframeEdit::Upsert {
        previous,
        time,
        value,
        easing: "LinearEasing",
    }
}

#[test]
fn packed_key_edits_only_one_use_and_retains_comments_and_radix() {
    let source = SOURCE.replace('\n', "\r\n");
    let initial = clip(&source);
    assert_eq!(initial.xaml.matches("Color=\"#FF324050\"").count(), 2);
    let color = r##"<SolidColorBrush Color="#B8734050" Opacity="0.75"/>"##;
    let patches = initial
        .patch_keyframe("Background", edit(Some(2.0), 2.0, color))
        .unwrap();
    let result = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(
        result,
        source.replace(
            "Solid::from_uint32((brand::/* end 🧭 */PACKED))",
            "Solid::with_opacity(::ferroui_base::media::Color::from_uint32((/* end 🧭 */0xB873_4050_u32)), 0.75)",
        )
    );
    let next = clip(&result);
    assert_eq!(next.xaml.matches("Color=\"#FF324050\"").count(), 1);
    assert!(next.xaml.contains("Color=\"#B8734050\""));
    assert!(next.xaml.contains("Opacity=\"0.75\""));
}

#[test]
fn color_references_and_channel_references_can_be_edited_independently() {
    let source = SOURCE
        .replace(
            "Solid::from_uint32(BASE)",
            "Solid::new(Color::from_rgb(brand::RED, 64, 80))",
        )
        .replace(
            "Solid::from_uint32((brand::/* end 🧭 */PACKED))",
            "Solid::new((/* end 🧭 */ INK))",
        );
    let initial = clip(&source);
    assert_eq!(initial.xaml.matches("Color=\"#FF324050\"").count(), 2);

    let value = r##"<SolidColorBrush Color="#C0784050"/>"##;
    let patches = initial
        .patch_keyframe("Background", edit(Some(0.0), 0.0, value))
        .unwrap();
    let changed = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(
        changed,
        source.replace(
            "Color::from_rgb(brand::RED, 64, 80)",
            "Color::from_argb(192, 0x78_u8, 64, 80)"
        )
    );

    let patches = initial
        .patch_keyframe("Background", edit(Some(2.0), 2.0, value))
        .unwrap();
    let changed = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(
        changed,
        source.replace(
            "/* end 🧭 */ INK",
            "/* end 🧭 */ ::ferroui_base::media::Color::from_argb(192, 120, 64, 80)"
        )
    );
    assert_eq!(
        clip(&changed).xaml.matches("Color=\"#FF324050\"").count(),
        1
    );

    let insertion = initial
        .author_keyframe(
            &source,
            RsxCatalog::built_in(),
            "Background",
            edit(None, 1.0, value),
        )
        .unwrap();
    let inserted = TextPatch::apply_all(&source, insertion).unwrap();
    let mapped = clip(&inserted);
    let removal = mapped
        .author_keyframe(
            &inserted,
            RsxCatalog::built_in(),
            "Background",
            KeyframeEdit::Remove(1.0),
        )
        .unwrap();
    assert_eq!(TextPatch::apply_all(&inserted, removal).unwrap(), source);
}

#[test]
fn constrains_constant_types_and_rejects_computed_or_conditional_colors() {
    for source in [
        SOURCE.replace("core::primitive::u32", "u64"),
        SOURCE.replace("0xFF32_4050_u32;", "(0xFF00_0000 | 0x0032_4050);"),
        SOURCE.replace(
            "pub const PACKED:",
            "#[cfg(feature = \"brand\")] pub const PACKED:",
        ),
        SOURCE.replace("pub const PACKED:", "#[custom::rewrite] pub const PACKED:"),
        SOURCE.replace("0xFF32_4050_u32;", "PACKED;"),
        SOURCE
            .replace("Solid::from_uint32(BASE)", "Solid::new(INK)")
            .replace("Paint { a: 255, r: RED, g: 64, b: 80 }", "make_color()"),
    ] {
        let projection = project(&source);
        assert!(
            projection.preview_markup().animations.is_empty(),
            "{source}"
        );
        assert_eq!(projection.animation_issues.len(), 1, "{source}");
    }
}
