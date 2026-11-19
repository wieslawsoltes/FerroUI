use crate::{RsxAnimationEdit, RsxAnimationPlan, project_rust};
use ferroui_designer_animation::{AnimationProgram, AnimationPrograms};
use ferroui_designer_core::{Document, TextPatch};

fn clips(source: &str) -> Vec<AnimationProgram> {
    let projection = project_rust(source, source.rfind("rsx!").unwrap()).unwrap();
    AnimationPrograms::scan(&Document::new(projection.xaml()).unwrap())
}

#[test]
fn places_two_clips_without_changing_comments_typed_literals_or_opaque_siblings() {
    let source = r##"fn other() { rsx! { Border { name: "Other 🌿" } } }
fn view() { rsx! { Border {
    name: "Card",
    styles: [ferroui_view::style("#Card")
        .animation(ferroui_view::animation(ferroui_view::ms(750)).delay(ferroui_view::ms(/* 🌿 */ 250u64)))
        .animation(ferroui_view::animation(ferroui_view::ms(1000)))
        .animation(compute_motion())],
} } }
"##.replace('\n', "\r\n");
    let caret = source.rfind("rsx!").unwrap();
    let map = project_rust(&source, caret).unwrap();
    let preview = map.preview_markup();
    let first = preview.animations[0].1.declaration;
    let second = preview.animations[1].1.declaration;
    let patches = map.place_animation_clips(&[(first, 0.5), (second, 1.4)]).unwrap();
    assert_eq!(patches.len(), 2);
    let expected = source.replace("250u64", "500u64")
        .replace("ms(1000)))", "ms(1000)).delay(ferroui_view::ms(1400)))");
    let edited = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(edited, expected);
    let programs = clips(&edited);
    assert!(programs[0].source.contains("Delay=\"0:0:0.5000000\""));
    assert!(programs[1].source.contains("Delay=\"0:0:1.4000000\""));
    assert!(map.place_animation_clips(&[(first, 0.0001)]).is_err());
}

#[test]
fn creates_a_named_clip_after_an_opaque_style_without_replacing_the_view() {
    let source = r#"fn other() { rsx! { TextBlock { text: "Other 🌱" } } }
fn dashboard() {
    rsx! {
        StackPanel {
            styles: [
                dynamic_style(model) // keep the application's style
            ],
            TextBlock { text: summary(model) }
            Border { name: "Card" }
        }
    }
}
"#
    .replace('\n', "\r\n");
    let caret = source.rfind("rsx!").unwrap();
    let projection = project_rust(&source, caret).unwrap();
    let label = "Fade /* review */ — \"Revenue\"";
    assert!(projection.can_create_animation(0));
    assert!(
        projection
            .create_animation(0, label, "Border#Card", 0.0001)
            .is_err()
    );
    let RsxAnimationPlan {
        patches,
        selected_key,
    } = projection
        .create_animation(0, label, "Border#Card", 1.25)
        .unwrap();
    assert_eq!(patches.len(), 2);
    assert!(patches.iter().all(|patch| patch.span.is_empty()));
    let edited = TextPatch::apply_all(&source, patches.clone()).unwrap();
    assert!(edited.contains("dynamic_style(model), // keep the application's style\r\n"));
    assert!(edited.contains("ferroui_view::animation(ferroui_view::ms(1250))"));
    assert!(edited.contains("TextBlock { text: summary(model) }"));
    assert_eq!(&edited[..caret], &source[..caret]);
    assert!(!edited.replace("\r\n", "").contains('\n'));
    let programs = clips(&edited);
    assert_eq!(programs.len(), 1);
    assert_eq!(programs[0].name, label);
    assert_eq!(programs[0].source_key, selected_key);
    assert_eq!(programs[0].selector, "Border#Card");
    assert_eq!(
        project_rust(&edited, caret).unwrap().omitted_count,
        projection.omitted_count
    );
}

#[test]
fn duplicates_and_removes_one_clip_with_a_computed_sibling() {
    let source = r##"rsx! { StackPanel {
        styles: [ferroui_view::style("#Card")
            .animation(ferroui_view::animation(ferroui_view::ms(1200))
                .key_frame(0.5, |key| key.set(ferroui_base::Visual::opacity_property(), /* 🪴 hold */ 0.4)))
            .animation(compute_motion(model))],
        Border { name: "Card" }
    }}"##;
    let projection = project_rust(source, 0).unwrap();
    let clip = projection.preview_markup().animations[0].1.declaration;
    let RsxAnimationPlan {
        patches,
        selected_key,
    } = projection
        .edit_animation(clip, RsxAnimationEdit::Rename("Reveal"))
        .unwrap();
    let named = TextPatch::apply_all(source, patches).unwrap();
    assert_eq!(clips(&named)[0].source_key, selected_key);
    let projection = project_rust(&named, 0).unwrap();
    let authored = projection.preview_markup().animations[0].1.declaration;
    let pasted = projection
        .edit_animation(authored, RsxAnimationEdit::Duplicate("Reveal copy"))
        .unwrap();
    assert!(pasted.patches.iter().all(|patch| patch.span.is_empty()));
    assert_ne!(pasted.selected_key, selected_key);
    let duplicated = TextPatch::apply_all(&named, pasted.patches).unwrap();
    let programs = clips(&duplicated);
    assert_eq!(
        programs
            .iter()
            .map(|program| program.name.as_str())
            .collect::<Vec<_>>(),
        ["Reveal", "Reveal copy"]
    );
    assert_eq!(duplicated.matches("/* 🪴 hold */").count(), 2);
    assert_eq!(
        duplicated
            .matches(".animation(compute_motion(model))")
            .count(),
        1
    );

    let projection = project_rust(&duplicated, 0).unwrap();
    let copy = projection.preview_markup().animations[1].1.declaration;
    let plan = projection
        .edit_animation(copy, RsxAnimationEdit::Remove)
        .unwrap();
    let removed = TextPatch::apply_all(&duplicated, plan.patches).unwrap();
    assert_eq!(removed.matches("/* 🪴 hold */").count(), 2);
    assert_eq!(removed.matches("ferroui-designer-animation:").count(), 1);
    assert!(removed.contains(".animation(compute_motion(model))"));
    assert_eq!(clips(&removed)[0].source_key, selected_key);
    assert_eq!(project_rust(&removed, 0).unwrap().animation_issues.len(), 1);

    let projection = project_rust(&removed, 0).unwrap();
    let authored = projection.preview_markup().animations[0].1.declaration;
    let renamed = projection
        .edit_animation(authored, RsxAnimationEdit::Rename("Forecast"))
        .unwrap();
    assert_eq!(renamed.selected_key, selected_key);
}

#[test]
fn removes_the_only_style_entry_and_keeps_comments_around_its_comma() {
    let source = r##"rsx! { Border {
        name: "Card",
        styles: [
            ferroui_view::style("#Card").animation(ferroui_view::animation(/* duration */ ferroui_view::ms(1000)))
                /* after the old style */,
        ],
    }}"##;
    let projection = project_rust(source, 0).unwrap();
    let clip = projection.preview_markup().animations[0].1.declaration;
    let plan = projection
        .edit_animation(clip, RsxAnimationEdit::Remove)
        .unwrap();
    let removed = TextPatch::apply_all(source, plan.patches).unwrap();
    assert!(removed.contains("/* duration */") && removed.contains("/* after the old style */"));
    assert!(!removed.contains("ferroui_view::style"));
    let projection = project_rust(&removed, 0).unwrap();
    assert!(projection.preview_markup().animations.is_empty());
    assert_eq!(projection.omitted_count, 0);
    assert!(projection.can_create_animation(0));
}

#[test]
fn creates_the_first_style_field_and_detects_computed_or_repeated_fields() {
    let source = "rsx! { Border { name: \"Card\" } }";
    let projection = project_rust(source, 0).unwrap();
    let plan = projection
        .create_animation(0, "First", "#Card", 1.0)
        .unwrap();
    let edited = TextPatch::apply_all(source, plan.patches).unwrap();
    assert!(edited.contains("name: \"Card\""));
    assert_eq!(clips(&edited)[0].name, "First");
    for source in [
        "rsx! { Border { styles: live_styles(model), name: \"Card\" } }",
        "rsx! { Border { styles: [], styles: [], name: \"Card\" } }",
    ] {
        let projection = project_rust(source, 0).unwrap();
        assert!(!projection.can_create_animation(0));
        assert!(
            projection
                .create_animation(0, "Blocked", "#Card", 1.0)
                .is_err()
        );
    }
}
