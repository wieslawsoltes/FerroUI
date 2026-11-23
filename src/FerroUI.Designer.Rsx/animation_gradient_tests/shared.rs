use super::*;

const SOURCE: &str = r###"use ferroui_base::media::{IBrush, immutable::ImmutableLinearGradientBrush as Gradient};
use ferroui_controls::Border as Card;
use ferroui_view::{animation as clip, ms, style};
use std::rc::Rc;
mod palette {
    use ferroui_base::media::{Color as Paint, immutable::ImmutableGradientStop as Stop};
    const FIRST_OFFSET: f64 = 0.25;
    pub const APPROVAL: &[Stop] = &[
        Stop::new(FIRST_OFFSET, Paint::from_argb(255, 44, 98, 166)),
        Stop::new(1.0, Paint::from_argb(255, 103, 190, 154)),
    ];
}
use self::palette::APPROVAL as PALETTE;
fn title() { rsx! { TextBlock { text: "Keep this view 🌿" } } }
fn view() { rsx! {
    StackPanel {
        styles: [style("Border#Review").animation(
            clip(ms(2000))
                .key_frame(0.0, |key| key.set(Card::background_property(),
                    Some(Rc::new(Gradient::from_stops(PALETTE)) as Rc<dyn IBrush>)))
                .key_frame(1.0, |key| key.set(Card::background_property(),
                    Some(Rc::new(Gradient::from_stops((/* chosen key 🌿 */ PALETTE))) as Rc<dyn IBrush>)))
        )],
        Border { name: "Review", width: 180.0, height: 80.0 }
    }
} }
"###;

fn project(source: &str) -> crate::RsxProjection<'_> {
    RsxCatalog::built_in()
        .project(source, source.rfind("rsx!").unwrap())
        .unwrap()
}

#[test]
fn edit_one_shared_palette_use_and_preserve_detached_key_operations() {
    let source = SOURCE.replace('\n', "\r\n");
    let initial = project(&source);
    let clip = initial.preview_markup().animations[0].1;
    assert_eq!(clip.xaml.matches("Offset=\"0.25\"").count(), 2);
    let value = r##"<LinearGradientBrush TransformOrigin="0%,0%">
        <GradientStop Offset="0.45" Color="#FF2C62A6"/>
        <GradientStop Offset="1" Color="#FF67BE9A"/>
    </LinearGradientBrush>"##;
    let edit = |time, previous| KeyframeEdit::Upsert {
        time,
        previous,
        value,
        easing: "LinearEasing",
    };
    let patches = clip
        .patch_keyframe("Background", edit(2.0, Some(2.0)))
        .unwrap();
    assert_eq!(patches.len(), 1);
    let span = patches[0].span;
    assert_eq!(&source[span.range()], "(/* chosen key 🌿 */ PALETTE)");
    let patched = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(
        patched.split("fn view").next(),
        source.split("fn view").next()
    );
    assert_eq!(patched.matches("Gradient::from_stops(PALETTE)").count(), 1);
    assert!(patched.contains("/* chosen key 🌿 */"));
    assert_eq!(
        patched.matches("\r\n").count(),
        source.matches("\r\n").count()
    );
    let next = project(&patched);
    let edited = next.preview_markup().animations[0].1;
    assert_eq!(edited.xaml.matches("Offset=\"0.25\"").count(), 1);
    assert_eq!(edited.xaml.matches("Offset=\"0.45\"").count(), 1);
    let extra = clip
        .author_keyframe(
            &source,
            RsxCatalog::built_in(),
            "Background",
            edit(1.0, None),
        )
        .unwrap();
    let added = TextPatch::apply_all(&source, extra).unwrap();
    assert_eq!(added.matches("PALETTE)").count(), 2);
    assert_eq!(
        project(&added).preview_markup().animations[0]
            .1
            .xaml
            .matches("<KeyFrame ")
            .count(),
        3
    );
}

#[test]
fn resolves_a_borrowed_array_and_preserves_it_for_geometry_edits() {
    let source = SOURCE
        .replace("&[Stop] = &[", "[Stop; 2] = [")
        .replace("from_stops(PALETTE)", "from_stops(&PALETTE)")
        .replace("*/ PALETTE", "*/ &PALETTE");
    let initial = project(&source);
    let clip = initial.preview_markup().animations[0].1;
    let value = r##"<LinearGradientBrush StartPoint="20%,0%" TransformOrigin="0%,0%">
        <GradientStop Offset="0.25" Color="#FF2C62A6"/>
        <GradientStop Offset="1" Color="#FF67BE9A"/>
    </LinearGradientBrush>"##;
    let patches = clip
        .patch_keyframe(
            "Background",
            KeyframeEdit::Upsert {
                time: 2.0,
                previous: Some(2.0),
                value,
                easing: "LinearEasing",
            },
        )
        .unwrap();
    let edited = TextPatch::apply_all(&source, patches).unwrap();
    assert!(edited.contains("Gradient::new((/* chosen key 🌿 */ &PALETTE),"));
    assert_eq!(
        project(&edited).preview_markup().animations[0]
            .1
            .xaml
            .matches("Offset=\"0.25\"")
            .count(),
        2
    );
    let conditional = source.replace(
        "pub const APPROVAL",
        "#[cfg(unknown_target)] pub const APPROVAL",
    );
    assert!(project(&conditional).preview_markup().animations.is_empty());
}
