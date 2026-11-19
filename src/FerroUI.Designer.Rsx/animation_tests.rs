use crate::project_rust;
use ferroui_designer_animation::{AnimationPrograms, ImportOptions, KeyframeEdit, Timeline};
use ferroui_designer_core::Document;

const VIEW: &str = r#"fn different() { rsx! { TextBlock { text: "Separate 🪴" } } }

fn preview() {
    rsx! {
        StackPanel {
            styles: [
                ferroui_view::style("Border#Card").animation(
                    ferroui_view::animation(ferroui_view::ms(2000))
                        .key_frame(0.0, |key| key.set(ferroui_base::Visual::opacity_property(), 0.2))
                        .key_frame(0.25, |key| key.set(ferroui_base::Visual::opacity_property(), 0.6f64)
                            .key_spline(0.25, - /* tangent */ 0.5, 0.6, 1.2))
                        .key_frame(1.0, |key| key.set(ferroui_base::Visual::opacity_property(), 1.0))
                        .fill_mode(ferroui_base::animation::FillMode::Both)
                ),
            ],
            Border { name: "Card", width: 120.0 }
            TextBlock { text: compute_label() }
        }
    }
}
"#;

#[test]
fn styled_clip_has_rust_anchors_and_edits_only_its_authored_literals() {
    let source = VIEW.replace('\n', "\r\n");
    let caret = source.find("ferroui_view::animation(").unwrap();
    let projection = project_rust(&source, caret).unwrap();
    assert_eq!(projection.blocks.len(), 2);
    assert_eq!(projection.omitted_count, 1);
    let markup = projection.preview_markup();
    let root = Document::new(&markup.source).unwrap();
    let control_names = root
        .ast()
        .nodes_in_source_order()
        .filter(|node| markup.controls.contains(&node.span.start))
        .map(|node| node.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(control_names, ["StackPanel", "Border", "TextBlock"]);
    let [(offset, animation)] = &markup.animations[..] else {
        panic!("one animated style")
    };
    assert!(markup.source[*offset..].starts_with("<Animation "));
    assert!(source[animation.declaration.range()].starts_with("ferroui_view::animation("));
    let programs = AnimationPrograms::scan(&root);
    assert_eq!(programs.len(), 1);
    let timeline = Timeline::import_xaml(&programs[0].source, 1, &ImportOptions::default())
        .unwrap()
        .timeline;
    assert_eq!(timeline.tracks[0].keyframes[1].time, 0.5);
    assert!(projection.apply_xaml(&markup.source).is_err());

    let edit = KeyframeEdit::Upsert {
        previous: Some(0.5),
        time: 1.4,
        value: "0.72",
        easing: "0.25,0.75,0.6,1.2",
    };
    let mut edited = source.clone();
    for patch in animation
        .patch_keyframe("Opacity", edit)
        .unwrap()
        .into_iter()
        .rev()
    {
        edited.replace_range(patch.span.range(), &patch.replacement);
    }
    assert_eq!(
        edited,
        source
            .replace("0.25, |key|", "0.7, |key|")
            .replace("0.6f64", "0.72f64")
            .replace("- /* tangent */ 0.5", " /* tangent */ 0.75")
    );
    let rewritten = project_rust(&edited, caret).unwrap().xaml();
    let rebuilt = Document::new(&rewritten).unwrap();
    let programs = AnimationPrograms::scan(&rebuilt);
    let timeline = Timeline::import_xaml(&programs[0].source, 1, &ImportOptions::default())
        .unwrap()
        .timeline;
    let keys = &timeline.tracks[0].keyframes;
    assert_eq!(keys[1].time, 1.4);
    assert!(rewritten.contains("Value=\"0.72\""));
}

#[test]
fn animated_source_keeps_dynamic_styles_in_rust_and_rejects_unmapped_keys() {
    let source = VIEW.replace("0.6f64", "calculate_opacity()");
    let caret = source.find("ferroui_view::animation(").unwrap();
    let projection = project_rust(&source, caret).unwrap();
    assert!(projection.preview_markup().animations.is_empty());
    assert!(projection
        .omissions
        .iter()
        .any(|reason| reason.contains("styles expression")));

    let projection = project_rust(VIEW, VIEW.find("ferroui_view::animation(").unwrap()).unwrap();
    let markup = projection.preview_markup();
    let animation = markup.animations[0].1;
    let not_authored = KeyframeEdit::Upsert {
        previous: Some(0.3),
        time: 0.5,
        value: "0.72",
        easing: "LinearEasing",
    };
    assert!(animation.patch_keyframe("Opacity", not_authored).is_err());
    let collision = KeyframeEdit::Upsert {
        previous: Some(0.5),
        time: 1.999999999,
        value: "0.72",
        easing: "LinearEasing",
    };
    assert!(animation.patch_keyframe("Opacity", collision).is_err());
    let insert_spline = KeyframeEdit::Upsert {
        previous: Some(2.0),
        time: 2.0,
        value: "1",
        easing: "0.2,1.4,0.8,1.0",
    };
    let patches = animation.patch_keyframe("Opacity", insert_spline).unwrap();
    assert_eq!(patches.len(), 1);
    assert!(patches[0].span.is_empty());
    assert_eq!(patches[0].replacement, ".key_spline(0.2, 1.4, 0.8, 1.0)");
}

#[test]
fn a_shared_rust_key_can_edit_one_value_and_keeps_its_common_cue_and_easing() {
    let source = VIEW.replace("0.6f64)\n", "0.6f64)\n                            .set(ferroui_base::layout::Layoutable::width_property(), 64.0)\n");
    let projection = project_rust(&source, source.find("ferroui_view::animation(").unwrap()).unwrap();
    let markup = projection.preview_markup();
    let clip = markup.animations[0].1;
    let edit = |time, easing| KeyframeEdit::Upsert { previous: Some(0.5), time, value: "72", easing };
    let changes = clip.patch_keyframe("Width", edit(0.5, "0.25,-0.5,0.6,1.2")).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(&source[changes[0].span.range()], "64.0");
    assert_eq!(changes[0].replacement, "72.0");
    assert!(clip.patch_keyframe("Width", edit(1.0, "0.25,-0.5,0.6,1.2")).is_err());
    assert!(clip.patch_keyframe("Width", edit(0.5, "LinearEasing")).is_err());
}
