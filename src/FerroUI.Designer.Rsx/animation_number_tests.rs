use crate::{RsxAnimation, RsxCatalog};
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::TextPatch;

const VIEW: &str = r###"use ferroui_view::prelude::*;
use ferroui_base::Visual;
mod motion {
    pub const CUE: f64 = 0.25;
    pub const INITIAL: f64 = 0.65;
    pub const OVERSHOOT: f64 = 1.0;
}
use self::motion::INITIAL as SHARED;
const HOLD: core::primitive::f64 = SHARED;

fn caption() -> Element { rsx! { TextBlock { text: "Separate invocation 🧭" } } }
fn view() -> Element { rsx! { StackPanel {
    styles: [style("Border#Forecast").animation(animation(ms(2000))
        .key_frame(0.0, |key| key.set(Visual::opacity_property(), HOLD))
        .key_frame(motion::/* time 🧭 */CUE, |key| key.set(Visual::opacity_property(),
            (motion::/* value 🧭 */INITIAL))
            .key_spline(0.25, 0.1, 0.75, motion::OVERSHOOT))
        .key_frame(1.0, |key| key.set(Visual::opacity_property(), 1.0))
    )],
    Border { name: "Forecast", opacity: HOLD, width: 180.0, height: 100.0 }
} } }
"###;

fn projection(source: &str) -> crate::RsxProjection<'_> {
    RsxCatalog::built_in()
        .project(source, source.rfind("rsx!").unwrap())
        .unwrap()
}

fn first_clip(source: &str) -> RsxAnimation {
    projection(source).preview_markup().animations[0].1.clone()
}

#[test]
fn maps_and_edits_only_selected_numeric_uses() {
    let source = VIEW.replace('\n', "\r\n");
    let clip = first_clip(&source);
    assert_eq!(
        clip.xaml
            .matches("Property=\"Opacity\" Value=\"0.65\"")
            .count(),
        2
    );
    let patches = clip
        .patch_keyframe(
            "Opacity",
            KeyframeEdit::Upsert {
                previous: Some(0.5),
                time: 1.4,
                value: "0.82",
                easing: "0.25,0.1,0.75,1.4",
            },
        )
        .unwrap();
    let edited = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(
        edited,
        source
            .replace("motion::/* time 🧭 */CUE", "/* time 🧭 */0.7f64")
            .replace("motion::/* value 🧭 */INITIAL", "/* value 🧭 */0.82f64")
            .replace("motion::OVERSHOOT", "1.4f64")
    );
    let next = first_clip(&edited);
    assert_eq!(
        next.xaml
            .matches("Property=\"Opacity\" Value=\"0.65\"")
            .count(),
        1
    );
    assert!(next.maps_key(
        "Opacity",
        1.4,
        ferroui_designer_animation::ValueKind::Number
    ));
    assert!(next.xaml.contains("KeySpline=\"0.25,0.1,0.75,1.4\""));
}

#[test]
fn structural_edits_keep_unmodified_constant_references() {
    let clip = first_clip(VIEW);
    let catalog = RsxCatalog::built_in();
    let patches = clip
        .author_keyframe(
            VIEW,
            catalog,
            "Opacity",
            KeyframeEdit::Upsert {
                previous: None,
                time: 1.0,
                value: "0.8",
                easing: "LinearEasing",
            },
        )
        .unwrap();
    let with_key = TextPatch::apply_all(VIEW, patches).unwrap();
    assert_eq!(with_key.matches("motion::/*").count(), 2);
    let next = first_clip(&with_key);
    let patches = next
        .author_keyframe(&with_key, catalog, "Opacity", KeyframeEdit::Remove(1.0))
        .unwrap();
    assert_eq!(TextPatch::apply_all(&with_key, patches).unwrap(), VIEW);

    let retimed = clip
        .patch_key_times([("Opacity", 0.5, 0.75), ("Opacity", 2.0, 1.5)])
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(VIEW, retimed).unwrap(),
        VIEW.replace("motion::/* time 🧭 */CUE", "/* time 🧭 */0.375f64")
            .replace(".key_frame(1.0,", ".key_frame(0.75,")
    );
}

#[test]
fn computed_conditional_and_wrong_type_constants_do_not_map() {
    for source in [
        VIEW.replace(
            "const INITIAL: f64 = 0.65",
            "const INITIAL: f64 = 0.5 + 0.15",
        ),
        VIEW.replace("const INITIAL: f64", "const INITIAL: f32"),
        VIEW.replace(
            "pub const INITIAL",
            "#[cfg(feature = \"dark\")] pub const INITIAL",
        ),
        VIEW.replace(
            "pub const INITIAL",
            "#[other::adjust_value] pub const INITIAL",
        ),
        VIEW.replace("const INITIAL: f64 = 0.65", "const INITIAL: f64 = INITIAL"),
        VIEW.replace("const HOLD: core::primitive::f64", "const HOLD: f32"),
    ] {
        let projected = projection(&source);
        let view = projected.preview_markup();
        assert!(view.animations.is_empty(), "{source}");
        assert_eq!(projected.animation_issues.len(), 1, "{source}");
    }
}

#[test]
fn keeps_unchanged_constructor_operands_and_negated_references() {
    let source = VIEW
        .replace(
            "use ferroui_base::Visual;",
            "use ferroui_base::CornerRadius;\nuse ferroui_controls::Border;",
        )
        .replace(
            "pub const OVERSHOOT:",
            "pub const CONTRACT: f64 = -0.65;\n    pub const OVERSHOOT:",
        )
        .replace(
            "Visual::opacity_property()",
            "Border::corner_radius_property()",
        )
        .replace(
            ".set(Border::corner_radius_property(), HOLD)",
            ".set(Border::corner_radius_property(), CornerRadius::uniform(HOLD))",
        )
        .replace(
            "(motion::/* value 🧭 */INITIAL)",
            "CornerRadius::top_bottom(HOLD, -/* lower corners */ (motion::CONTRACT))",
        )
        .replace(
            ".set(Border::corner_radius_property(), 1.0)",
            ".set(Border::corner_radius_property(), CornerRadius::uniform(1.0))",
        );
    let clip = first_clip(&source);
    let patches = clip
        .patch_keyframe(
            "CornerRadius",
            KeyframeEdit::Upsert {
                previous: Some(0.5),
                time: 0.5,
                value: "0.65,0.65,1.25,1.25",
                easing: "0.25,0.1,0.75,1.0",
            },
        )
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patches).unwrap(),
        source.replace(
            "-/* lower corners */ (motion::CONTRACT)",
            "/* lower corners */ (1.25f64)"
        )
    );
}
