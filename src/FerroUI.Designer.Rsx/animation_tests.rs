use crate::{RsxCatalog, project_rust};
use ferroui_designer_animation::{
    AnimationPrograms, ImportOptions, KeyframeEdit, KeyframeGroupEdit, KeyframeLocation, Timeline,
};
use ferroui_designer_core::{Document, TextPatch};

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
fn color_keys_change_alpha_without_reformatting_authored_rust() {
    for (original, expected) in [
        (
            "from_rgb(/* red 🧭 */ 120u8, 32, 16)",
            "from_argb(128, /* red 🧭 */ 224u8, 32, 16)",
        ),
        (
            "from_uint32(/* ARGB 🧭 */ 0x7F_78_20_10u32)",
            "from_uint32(/* ARGB 🧭 */ 0x80_E0_20_10u32)",
        ),
    ] {
        let source = VIEW
            .replace(
                "ferroui_base::Visual::opacity_property()",
                "ferroui_controls::Border::background_property()",
            )
            .replace("0.2))", r##""#FF000000"))"##)
            .replace("0.6f64", &format!("ferroui_base::media::Color::{original}"))
            .replace("1.0))", r##""#20808080"))"##)
            .replace('\n', "\r\n");
        let caret = source.rfind("rsx!").unwrap();
        let projection = project_rust(&source, caret).unwrap();
        let markup = projection.preview_markup();
        let clip = markup.animations[0].1;
        let update = KeyframeEdit::Upsert {
            previous: Some(0.5),
            time: 0.5,
            value: "#80E02010",
            easing: "0.25,-0.5,0.6,1.2",
        };
        let patches = clip.patch_keyframe("Background", update).unwrap();
        let edited = TextPatch::apply_all(&source, patches).unwrap();
        assert_eq!(edited, source.replace(original, expected));
        assert!(project_rust(&edited, caret)
            .unwrap()
            .xaml()
            .contains("#80E02010"));
    }
}

#[test]
fn relative_point_keys_preserve_the_constructor_when_editing_units() {
    let constructor = "ferroui_base::RelativePoint::from_point(ferroui_base::Point::new(/* origin 🧭 */ 0.6f64, 0.25), ::ferroui_base::RelativeUnit::Relative)";
    let source = VIEW.replace("ferroui_base::Visual::opacity_property()", "ferroui_base::Visual::render_transform_origin_property()")
        .replace("0.2))", "ferroui_base::RelativePoint::new(0.2, 0.25, ferroui_base::RelativeUnit::Relative)))")
        .replace("0.6f64", constructor)
        .replace("1.0))", "ferroui_base::RelativePoint::new(1.0, 0.5, ferroui_base::RelativeUnit::Relative)))")
        .replace('\n', "\r\n");
    let caret = source.rfind("rsx!").unwrap();
    let projection = project_rust(&source, caret).unwrap();
    let markup = projection.preview_markup();
    assert!(markup.source.contains("Property=\"RenderTransformOrigin\" Value=\"60%,25%\""));
    let clip = markup.animations[0].1;
    let edit = |value| KeyframeEdit::Upsert { previous: Some(0.5), time: 0.5, value, easing: "0.25,-0.5,0.6,1.2" };
    assert!(clip.patch_keyframe("RenderTransformOrigin", edit("18%,24")).is_err());
    let patches = clip.patch_keyframe("RenderTransformOrigin", edit("18,24")).unwrap();
    let expected = constructor.replace("0.6f64, 0.25", "18.0f64, 24.0").replace("::RelativeUnit::Relative", "::RelativeUnit::Absolute");
    let edited = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(edited, source.replace(constructor, &expected));
    assert!(project_rust(&edited, caret).unwrap().xaml().contains("Value=\"18,24\""));
}

#[test]
fn retiming_shared_cues_requires_every_setter_and_changes_each_cue_once() {
    let source = VIEW.replace(
        "0.2))",
        "0.2).set(ferroui_base::layout::Layoutable::width_property(), 120.0))",
    );
    let caret = source.rfind("rsx!").unwrap();
    let projected = project_rust(&source, caret).unwrap();
    let markup = projected.preview_markup();
    let clip = markup.animations[0].1;
    assert!(clip.patch_key_times([("Opacity", 0.0, 0.5)]).unwrap_err().contains("shares"));
    let patches = clip.patch_key_times([
        ("Opacity", 0.0, 0.5),
        ("Width", 0.0, 0.5),
        ("Opacity", 0.5, 1.0),
    ]).unwrap();
    assert_eq!(patches.len(), 2);
    let updated = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(updated, source.replace(".key_frame(0.25,", ".key_frame(0.5,")
        .replace(".key_frame(0.0,", ".key_frame(0.25,"));
}

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

fn authored_timeline(source: &str) -> Timeline {
    let projection = project_rust(source, source.rfind("rsx!").unwrap()).unwrap();
    let document = Document::new(projection.xaml()).unwrap();
    let programs = AnimationPrograms::scan(&document);
    Timeline::import_xaml(&programs[0].source, 1, &ImportOptions::default()).unwrap().timeline
}

#[test]
fn insert_and_duplicate_only_add_methods_and_reject_native_tick_collisions() {
    let source = VIEW.replace('\n', "\r\n");
    let catalog = RsxCatalog::built_in();
    let projection = catalog.project(&source, source.rfind("rsx!").unwrap()).unwrap();
    let clip = projection.preview_markup().animations[0].1;
    let timeline = authored_timeline(&source);
    let location = KeyframeLocation { target: 1, property: "Opacity".into(), time: 0.5 };
    let duplicate = |offset| {
        clip.author_keyframes(&source, catalog, &timeline, std::slice::from_ref(&location),
            KeyframeGroupEdit::Duplicate(offset))
    };
    assert!(duplicate(1.49999996).unwrap_err().contains("occupies"));
    let (patches, destinations) = duplicate(1.0).unwrap();
    assert_eq!(destinations[0].time, 1.5);
    let updated = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(updated, source.replace(
        "\r\n                        .fill_mode",
        "\r\n                        .key_frame(0.75, |key| key.set(ferroui_base::Visual::opacity_property(), 0.6).key_spline(0.25, -0.5, 0.6, 1.2))\r\n                        .fill_mode",
    ));
    let new = authored_timeline(&updated);
    let mut duplicate = timeline.tracks[0].keyframes[1].clone();
    duplicate.time = 1.5;
    assert_eq!(new.tracks[0].keyframes[2], duplicate);

    let patches = clip.author_keyframe(&source, catalog, "Opacity", KeyframeEdit::Upsert {
        previous: None, time: 0.75, value: "0.72", easing: "LinearEasing",
    }).unwrap();
    let updated = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(updated, source.replace(
        "\r\n                        .fill_mode",
        "\r\n                        .key_frame(0.375, |key| key.set(ferroui_base::Visual::opacity_property(), 0.72))\r\n                        .fill_mode",
    ));
    assert_eq!(authored_timeline(&updated).tracks[0].keyframes[2].time, 0.75);
}

#[test]
fn mirror_retains_comment_trivia_and_rejects_conflicting_shared_cue_easings() {
    let source = VIEW.replace(
        "0.2))",
        "0.2).set(ferroui_base::layout::Layoutable::width_property(), 120.0))",
    ).replace('\n', "\r\n");
    let catalog = RsxCatalog::built_in();
    let projection = catalog.project(&source, source.rfind("rsx!").unwrap()).unwrap();
    let clip = projection.preview_markup().animations[0].1;
    let timeline = authored_timeline(&source);
    let key = |property: &str, time| KeyframeLocation { target: 1, property: property.into(), time };
    let opacity = [key("Opacity", 0.0), key("Opacity", 0.5)];
    assert!(clip.mirror_keyframes(&source, catalog, &timeline, &opacity, 0.5).unwrap_err().contains("shares"));
    let shared = [opacity[0].clone(), opacity[1].clone(), key("Width", 0.0)];
    assert!(clip.mirror_keyframes(&source, catalog, &timeline, &shared, 0.5).unwrap_err().contains("different incoming easings"));

    let keys = [key("Opacity", 0.5), key("Opacity", 2.0)];
    let (patches, accepted) = clip.mirror_keyframes(&source, catalog, &timeline, &keys, 1.25).unwrap();
    assert_eq!(accepted, [key("Opacity", 2.0), key("Opacity", 0.5)]);
    let rewritten = TextPatch::apply_all(&source, patches).unwrap();
    for text in ["/* tangent */", "0.6f64", "Separate 🪴", "compute_label()"] {
        assert!(rewritten.contains(text), "{rewritten}");
    }
    assert!(!rewritten.replace("\r\n", "").contains('\n'));
    let updated = authored_timeline(&rewritten);
    let opacity = updated.track(1, "Opacity").unwrap();
    assert_eq!(opacity.keyframes[1].easing, timeline.track(1, "Opacity").unwrap().keyframes[1].easing);
    assert_eq!(opacity.keyframes[2].value, ferroui_designer_animation::AnimationValue::Number(0.6));
}

#[test]
fn deleting_keys_keeps_comment_trivia_and_leaves_an_editable_empty_clip() {
    let source = VIEW.replace("0.6f64)\n", concat!(
        "0.6f64)\n",
        "                            .set(ferroui_base::layout::Layoutable::width_property(),\n",
        "                                /* width */ 64.0 // keep this note\n",
        "                            )\n",
    )).replace('\n', "\r\n");
    let catalog = RsxCatalog::built_in();
    let projection = catalog.project(&source, source.rfind("rsx!").unwrap()).unwrap();
    let clip = projection.preview_markup().animations[0].1;
    let changes = clip.author_keyframe(&source, catalog, "Width", KeyframeEdit::Remove(0.5)).unwrap();
    let updated = TextPatch::apply_all(&source, changes).unwrap();
    for text in ["/* width */", "// keep this note\r\n", "/* tangent */", "compute_label()", "Separate 🪴"] {
        assert!(updated.contains(text), "{updated}");
    }
    let timeline = authored_timeline(&updated);
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].keyframes, authored_timeline(VIEW).tracks[0].keyframes);
    let projection = catalog.project(&updated, updated.rfind("rsx!").unwrap()).unwrap();
    let clip = projection.preview_markup().animations[0].1;
    let keys = timeline.tracks[0].keyframes.iter().map(|key| KeyframeLocation {
        target: 1, property: "Opacity".into(), time: key.time,
    }).collect::<Vec<_>>();
    let (changes, keys) = clip.author_keyframes(&updated, catalog, &timeline, &keys, KeyframeGroupEdit::Remove).unwrap();
    assert!(keys.is_empty());
    let empty = TextPatch::apply_all(&updated, changes).unwrap();
    assert!(authored_timeline(&empty).tracks.is_empty());
    assert!(empty.contains(".fill_mode(ferroui_base::animation::FillMode::Both)"));
    assert!(empty.contains("// keep this note\r\n"));
    let projection = catalog.project(&empty, empty.rfind("rsx!").unwrap()).unwrap();
    let clip = projection.preview_markup().animations[0].1;
    let changes = clip.author_keyframe(&empty, catalog, "Opacity", KeyframeEdit::Upsert {
        previous: None, time: 1.25, value: "0.6", easing: "LinearEasing",
    }).unwrap();
    let restored = TextPatch::apply_all(&empty, changes).unwrap();
    assert!(restored.contains("::ferroui_base::Visual::opacity_property()"));
    assert_eq!(authored_timeline(&restored).tracks[0].keyframes[0].time, 1.25);
}
