use super::*;
use crate::{
    AnimationCodecRegistry, AnimationPrograms, AnimationValue, Easing, KeyframeGroupEdit, ValueKind,
};

const SOURCE: &str = r#"<f:Border xmlns:f="https://github.com/ferroui" Name="Review">
    <f:Border.Styles><f:Style Selector="Border#Review"><f:Style.Animations>
        <f:Animation FillMode='Both' Duration='0:0:2'>
            <f:KeyFrame Cue='25%'>
                <!-- Keep this shared cue 🌿 -->
                <f:Setter Property='Opacity' Value = '0&#46;25' />
                <f:Setter Property='CornerRadius' Value='3, 6, 9, 12'/>
            </f:KeyFrame>
            <f:KeyFrame Cue='75%'><!-- Keep this cue annotation -->
                <f:Setter Property='Opacity' Value='1'/>
            </f:KeyFrame>
        </f:Animation>
    </f:Style.Animations></f:Style></f:Border.Styles>
</f:Border>"#;

fn load(source: &str, options: &ImportOptions) -> (Document, AnimationProgram) {
    let document = Document::new(source).unwrap();
    let mut program = AnimationPrograms::scan(&document).pop().unwrap();
    let fragment = animation_fragment(&document, program.id).unwrap();
    AnimationCodecRegistry::default().decode(&mut program, &fragment, options);
    assert!(program.timeline.is_some(), "{:?}", program.diagnostics);
    (document, program)
}

fn location(program: &AnimationProgram, property: &str, time: f64) -> KeyframeLocation {
    KeyframeLocation {
        target: program.primary_target(),
        property: property.into(),
        time,
    }
}

#[test]
fn patches_only_the_changed_attribute_bytes_and_rejects_a_retired_model() {
    let source = SOURCE.replace('\n', "\r\n");
    let options = ImportOptions::default();
    let (document, program) = load(&source, &options);
    let mut draft = program.timeline.clone().unwrap();
    draft
        .track_mut(program.primary_target(), "Opacity")
        .unwrap()
        .keyframes[0]
        .value = AnimationValue::Number(0.5);
    let patches = native_keyframe_patches(&document, &program, &draft, &[], || &options)
        .unwrap()
        .unwrap();
    assert_eq!(patches.len(), 1);
    assert_eq!(&source[patches[0].span.range()], "2");
    assert_eq!(
        TextPatch::apply_all(&source, patches).unwrap(),
        source.replace("0&#46;25", "0&#46;5")
    );
    let retired = Document::new(source.replace("0&#46;25", "0.4")).unwrap();
    assert!(native_keyframe_patches(&retired, &program, &draft, &[], || &options).is_err());
}

#[test]
fn retimes_an_existing_cue_and_adds_an_incoming_spline_inside_the_same_frame() {
    let options = ImportOptions::default();
    let (document, program) = load(SOURCE, &options);
    let mut draft = program.timeline.clone().unwrap();
    let opacity = draft
        .track_mut(program.primary_target(), "Opacity")
        .unwrap();
    opacity.move_keyframe(1.5, 1.75).unwrap();
    opacity
        .set_easing(1.75, Easing::parse("0.2,0.3,0.8,1").unwrap())
        .unwrap();
    let retimed = [(
        location(&program, "Opacity", 1.5),
        location(&program, "Opacity", 1.75),
    )];
    let patches = native_keyframe_patches(&document, &program, &draft, &retimed, || &options)
        .unwrap()
        .unwrap();
    let expected = SOURCE.replace("Cue='75%'", "Cue='87.5%' KeySpline='0.2,0.3,0.8,1'");
    assert_eq!(TextPatch::apply_all(SOURCE, patches).unwrap(), expected);
}

#[test]
fn leaves_shared_timing_to_the_codec_when_an_edit_would_split_its_setters() {
    let options = ImportOptions::default();
    let (document, program) = load(SOURCE, &options);
    let mut draft = program.timeline.clone().unwrap();
    let target = program.primary_target();
    draft
        .track_mut(target, "Opacity")
        .unwrap()
        .move_keyframe(0.5, 0.75)
        .unwrap();
    let move_opacity = (
        location(&program, "Opacity", 0.5),
        location(&program, "Opacity", 0.75),
    );
    assert!(native_keyframe_patches(
        &document,
        &program,
        &draft,
        std::slice::from_ref(&move_opacity),
        || &options
    )
    .unwrap()
    .is_none());
    draft
        .track_mut(target, "CornerRadius")
        .unwrap()
        .move_keyframe(0.5, 0.75)
        .unwrap();
    let retimed = [
        move_opacity,
        (
            location(&program, "CornerRadius", 0.5),
            location(&program, "CornerRadius", 0.75),
        ),
    ];
    let patches = native_keyframe_patches(&document, &program, &draft, &retimed, || &options)
        .unwrap()
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(SOURCE, patches).unwrap(),
        SOURCE.replace("Cue='25%'", "Cue='37.5%'")
    );
}

#[test]
fn grouped_retime_follows_the_original_frame_when_a_time_is_reused() {
    let source = SOURCE.replace("Cue='75%'", "KeyTime='0:00:00.75'");
    let options = ImportOptions::default();
    let (document, program) = load(&source, &options);
    let locations = vec![
        location(&program, "Opacity", 0.5),
        location(&program, "CornerRadius", 0.5),
        location(&program, "Opacity", 0.75),
    ];
    let (draft, moved) = program
        .timeline
        .as_ref()
        .unwrap()
        .with_keyframe_edit(&locations, KeyframeGroupEdit::Move(0.25))
        .unwrap();
    let retimed = locations.into_iter().zip(moved).collect::<Vec<_>>();
    let patches = native_keyframe_patches(&document, &program, &draft, &retimed, || &options)
        .unwrap()
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patches).unwrap(),
        source
            .replace("Cue='25%'", "Cue='37.5%'")
            .replace("KeyTime='0:00:00.75'", "KeyTime='0:0:1'")
    );
}

#[test]
fn formats_cues_after_native_tick_rounding() {
    let source = SOURCE.replace("Cue='25%'", "Cue='20%'");
    let options = ImportOptions::default();
    let (document, program) = load(&source, &options);
    let locations = vec![
        location(&program, "Opacity", 0.4),
        location(&program, "CornerRadius", 0.4),
    ];
    let (draft, moved) = program
        .timeline
        .as_ref()
        .unwrap()
        .with_keyframe_edit(&locations, KeyframeGroupEdit::Move(0.2))
        .unwrap();
    let retimed = locations.into_iter().zip(moved).collect::<Vec<_>>();
    let patches = native_keyframe_patches(&document, &program, &draft, &retimed, || &options)
        .unwrap()
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patches).unwrap(),
        source.replace("Cue='20%'", "Cue='30%'")
    );
}

#[test]
fn preserves_a_discrete_markup_escape_and_quotes() {
    let source = SOURCE.replace(
        "Property='Opacity' Value = '0&#46;25'",
        "Property='Tag' Value = '{}{Draft &#x26; review}'",
    );
    let options = ImportOptions {
        property_kinds: [("Tag".into(), ValueKind::Discrete)].into(),
        ..Default::default()
    };
    let (document, program) = load(&source, &options);
    let mut draft = program.timeline.clone().unwrap();
    draft
        .track_mut(program.primary_target(), "Tag")
        .unwrap()
        .keyframes[0]
        .value = AnimationValue::Discrete("{Draft & review's notes}".into());
    let patches = native_keyframe_patches(&document, &program, &draft, &[], || &options)
        .unwrap()
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patches).unwrap(),
        source.replace("review}", "review&apos;s notes}")
    );
}

#[cfg(feature = "native-values")]
mod object {
    use super::*;
    use crate::TrackCurveChannel;

    const BRUSH: &str = r##"<f:Setter Property='Background'><f:Setter.Value>
                <!-- Keep the wrapped brush 🌿 -->
                <m:LinearGradientBrush StartPoint='10%,0%'>
                    <m:GradientBrush.GradientStops>
                        <m:GradientStop Offset='0.25' Color='&#x23;FF254679'/>
                        <!-- Keep this endpoint -->
                        <m:GradientStop Offset='1' Color='#FF62B9A7'/>
                    </m:GradientBrush.GradientStops>
                </m:LinearGradientBrush>
            </f:Setter.Value></f:Setter>"##;

    fn brush_source() -> String {
        SOURCE
            .replace(
                "Name=\"Review\"",
                "Name=\"Review\" xmlns:m='using:FerroUI.Media'",
            )
            .replace("<f:Setter Property='Opacity' Value = '0&#46;25' />", BRUSH)
            .replace(
                "Property='Opacity' Value='1'",
                "Property='Background' Value='#FF254679'",
            )
    }

    #[test]
    fn patches_gradient_fields_in_the_original_wrappers() {
        let source = brush_source().replace('\n', "\r\n");
        let options = ImportOptions::default();
        let (document, program) = load(&source, &options);
        let mut draft = program.timeline.clone().unwrap();
        let key = &mut draft
            .track_mut(program.primary_target(), "Background")
            .unwrap()
            .keyframes[0];
        let stop: TrackCurveChannel = "gradient_stop/1/stop_offset".parse().unwrap();
        let geometry: TrackCurveChannel = "gradient/start_point/percent_x".parse().unwrap();
        key.value = stop.replace_key_component(&key.value, 0.45).unwrap();
        key.value = geometry.replace_key_component(&key.value, 22.0).unwrap();
        let patches = native_keyframe_patches(&document, &program, &draft, &[], || &options)
            .unwrap()
            .unwrap();
        assert_eq!(patches.len(), 2);
        assert_eq!(
            TextPatch::apply_all(&source, patches).unwrap(),
            source
                .replace("Offset='0.25'", "Offset='0.45'")
                .replace("StartPoint='10%,0%'", "StartPoint='22%,0%'")
        );
    }

    #[test]
    fn changes_an_attribute_into_an_object_inside_the_original_setter() {
        let source = brush_source();
        let options = ImportOptions::default();
        let (document, program) = load(&source, &options);
        let mut draft = program.timeline.clone().unwrap();
        let track = draft
            .track_mut(program.primary_target(), "Background")
            .unwrap();
        track.keyframes[1].value = track.keyframes[0].value.clone();
        let patches = native_keyframe_patches(&document, &program, &draft, &[], || &options)
            .unwrap()
            .unwrap();
        let edited = TextPatch::apply_all(&source, patches).unwrap();
        assert_eq!(
            edited.split("Cue='75%'").next(),
            source.split("Cue='75%'").next()
        );
        assert!(edited.contains("<f:Setter Property='Background'><f:Setter.Value>"));
        assert!(edited.contains("<!-- Keep this cue annotation -->"));
    }
}
