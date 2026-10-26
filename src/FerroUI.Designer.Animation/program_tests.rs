use super::*;
use ferroui_designer_core::{Document, Edit};
use std::collections::BTreeMap;

fn programs(source: &str) -> Document {
    Document::new(source).unwrap()
}
#[test]
fn independent_programs_keep_source_identity_and_shared_styles_are_isolated() {
    let source = "<Canvas><Canvas.Styles><Style Selector='Button'><Setter Property='Opacity' Value='.7'/><Style.Animations><Animation Duration='0:0:1'><KeyFrame Cue='100%'><Setter Property='Width' Value='80'/></KeyFrame></Animation><Animation Duration='0:0:4' IterationCount='Infinite'><KeyFrame Cue='0%'><Setter Property='Height' Value='20'/></KeyFrame></Animation></Style.Animations></Style></Canvas.Styles><Button Name='a'/><Button Name='b'/></Canvas>";
    let mut document = programs(source);
    let initial = AnimationPrograms::scan(&document);
    assert_eq!(initial.len(), 2);
    let id = initial[0].id;
    let other = initial[1].id;
    let rename = AnimationPrograms::rename(&document, id, "Fade -- Café").unwrap();
    document.apply("Rename", rename).unwrap();
    assert_eq!(AnimationPrograms::scan(&document)[0].name, "Fade -- Café");
    let edits = AnimationPrograms::set_selector(&document, id, "Button#a").unwrap();
    document.apply("Retarget", edits).unwrap();
    let current = AnimationPrograms::scan(&document);
    let changed = current.iter().find(|program| program.id == id).unwrap();
    assert_eq!(changed.selector, "Button#a");
    assert_eq!(
        current
            .iter()
            .find(|program| program.id == other)
            .unwrap()
            .selector,
        "Button"
    );
    assert!(document
        .source()
        .contains("<Setter Property='Opacity' Value='.7'/>"));
    document.undo().unwrap();
    assert_eq!(AnimationPrograms::scan(&document)[0].id, id);
    let edits = AnimationPrograms::duplicate(&document, id, "Copy").unwrap();
    document.apply("Copy", edits).unwrap();
    let all = AnimationPrograms::scan(&document);
    assert_eq!(all.len(), 3);
    assert!(all.iter().any(|program| program.id == id));
    let copy = all
        .iter()
        .find(|program| program.name == "Copy")
        .unwrap()
        .id;
    assert_ne!(copy, id);
    document
        .apply(
            "Delete",
            AnimationPrograms::remove(&document, copy).unwrap(),
        )
        .unwrap();
    assert_eq!(AnimationPrograms::scan(&document).len(), 2);
}
#[test]
fn every_program_is_listed_even_outside_local_named_control_styles() {
    let source = "<Grid><Grid.Resources><Animation Duration='0:0:2'/></Grid.Resources><Grid.Styles><Style Selector='Button'><Style Selector='^:pointerover'><Style.Animations><Animation Duration='0:0:3'/><Animation Duration='0:0:5' CustomAnimator='keep-me'/></Style.Animations></Style></Style></Grid.Styles></Grid>";
    let document = programs(source);
    let mut all = AnimationPrograms::scan(&document);
    assert_eq!(all.len(), 3);
    let registry = AnimationCodecRegistry::default();
    for program in &mut all {
        registry.decode(
            program,
            &animation_fragment(&document, program.id).unwrap(),
            &ImportOptions::default(),
        );
    }
    assert_eq!(all[0].timeline.as_ref().unwrap().duration, 2.0);
    assert_eq!(all[1].timeline.as_ref().unwrap().duration, 3.0);
    assert!(all[2].timeline.is_none());
    assert!(!all[2].diagnostics.is_empty());
    assert_eq!(document.source(), source);
}
#[test]
fn extension_codec_preserves_custom_animation_payloads() {
    struct Custom;
    impl AnimationCodec for Custom {
        fn id(&self) -> &str {
            "custom.animator"
        }
        fn can_decode(&self, source: &str) -> bool {
            source.contains("CustomAnimator=")
        }
        fn decode(
            &self,
            source: &str,
            target: u64,
            options: &ImportOptions,
        ) -> Result<AnimationImport> {
            let mut document = Document::new(source).unwrap();
            document
                .apply(
                    "Decode",
                    vec![Edit::RemoveAttribute {
                        node: document.ast().root(),
                        name: "CustomAnimator".into(),
                    }],
                )
                .unwrap();
            Timeline::import_xaml(document.source(), target, options)
        }
        fn encode(&self, original: &str, timeline: &Timeline, target: u64) -> Result<String> {
            let source = Document::new(original).unwrap();
            let payload = source
                .ast()
                .node(source.ast().root())
                .unwrap()
                .attribute("CustomAnimator")
                .unwrap();
            let mut output = Document::new(timeline.to_xaml(target)?).unwrap();
            output
                .apply(
                    "Payload",
                    vec![Edit::SetAttribute {
                        node: output.ast().root(),
                        name: "CustomAnimator".into(),
                        value: payload.into(),
                    }],
                )
                .unwrap();
            Ok(output.source().into())
        }
    }
    let document = programs(
        "<Grid><Grid.Resources><Animation Duration='0:0:2' CustomAnimator='vendor:Keep &amp; preserve'/></Grid.Resources></Grid>",
    );
    let mut program = AnimationPrograms::scan(&document).remove(0);
    let source = animation_fragment(&document, program.id).unwrap();
    let mut registry = AnimationCodecRegistry::default();
    registry.decode(&mut program, &source, &ImportOptions::default());
    assert!(program.timeline.is_none());
    registry.register(Box::new(Custom));
    program.diagnostics.clear();
    registry.decode(&mut program, &source, &ImportOptions::default());
    let mut timeline = program.timeline.clone().unwrap();
    timeline.duration = 7.0;
    let exported = registry.encode(&program, &source, &timeline).unwrap();
    assert!(exported.contains("vendor:Keep &amp; preserve"));
    assert_eq!(
        Custom
            .decode(
                &exported,
                program.primary_target(),
                &ImportOptions::default()
            )
            .unwrap()
            .timeline
            .duration,
        7.0
    );
}
#[test]
fn create_empty_program_and_remove_last_track_are_real_source_edits() {
    let mut document = programs(
        "<ui:Canvas xmlns:ui='https://github.com/ferroui'><ui:Button Name='a'/></ui:Canvas>",
    );
    let root = document.ast().root();
    let timeline = Timeline::new(2.5).unwrap();
    document
        .apply(
            "Create",
            AnimationPrograms::create(&document, root, "Empty", "ui|Button", &timeline).unwrap(),
        )
        .unwrap();
    assert!(document.source().contains("<ui:Animation"));
    let mut program = AnimationPrograms::scan(&document).remove(0);
    let registry = AnimationCodecRegistry::default();
    let source = animation_fragment(&document, program.id).unwrap();
    registry.decode(&mut program, &source, &ImportOptions::default());
    assert_eq!(program.timeline.unwrap().duration, 2.5);
}
fn group(owner: u64) -> VisualStateGroup {
    VisualStateGroup {
        name: "Interaction".into(),
        owner,
        states: vec![
            VisualState {
                name: "Normal".into(),
                setters: vec![StateSetter {
                    target: "Target".into(),
                    property: "Opacity".into(),
                    value: "1".into(),
                }],
            },
            VisualState {
                name: "Disabled".into(),
                setters: vec![StateSetter {
                    target: "Target".into(),
                    property: "Opacity".into(),
                    value: "0.3".into(),
                }],
            },
        ],
        transitions: vec![StateTransition {
            from: Some("Normal".into()),
            to: "Disabled".into(),
            duration: 0.5,
            easing: Easing::Linear,
        }],
    }
}
#[test]
fn state_groups_emit_real_styles_transitions_and_roundtrip_source_edits() {
    let mut document = programs(
        "<Grid xmlns='https://github.com/ferroui'><Button Name='Target' Opacity='1'/></Grid>",
    );
    let root = document.ast().root();
    let model = group(root);
    document
        .apply("States", model.plan(&document).unwrap())
        .unwrap();
    assert!(document.source().contains("Style.Animations"));
    assert!(document.source().contains("FillMode=\"Forward\""));
    let read = VisualStateGroup::read(&document);
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].states[1].setters[0].value, "0.3");
    let actions = read[0]
        .actions("Owner", "Disabled", Some("Normal"))
        .unwrap();
    assert!(actions.iter().any(
        |action| matches!(action,BehaviorAction::AddClass{class,..}if class.starts_with("dst_"))
    ));
    let animation = document
        .ast()
        .elements()
        .find(|node| node.local_name() == "Animation")
        .unwrap()
        .id;
    let setter = document
        .ast()
        .elements()
        .find(|node| {
            node.local_name() == "Setter" && document.ast().is_descendant_of(node.id, animation)
        })
        .unwrap()
        .id;
    document
        .apply(
            "Direct source keyframe",
            vec![Edit::SetAttribute {
                node: setter,
                name: "Value".into(),
                value: "0.9".into(),
            }],
        )
        .unwrap();
    assert_eq!(
        VisualStateGroup::read(&document)[0].states[0].setters[0].value,
        "0.9"
    );
    let remove = VisualStateGroup::remove(&document, root, "Interaction").unwrap();
    document.apply("Delete states", remove).unwrap();
    assert!(VisualStateGroup::read(&document).is_empty());
}
#[test]
fn generated_styles_and_behaviors_follow_owner_resources() {
    let mut document = programs(
        "<Grid xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Grid.Resources><x:Double x:Key='MutedOpacity'>0.25</x:Double></Grid.Resources><Button Name='Target' Opacity='1'/></Grid>",
    );
    let owner = document.ast().root();
    let mut state = group(owner);
    state.states[1].setters[0].value = "{StaticResource MutedOpacity}".into();
    document
        .apply("Resource state", state.plan(&document).unwrap())
        .unwrap();
    let behaviors = BehaviorModel {
        id: None,
        owner,
        trigger: BehaviorTrigger::Event {
            event: "PointerPressed".into(),
            source: None,
        },
        actions: vec![BehaviorAction::ChangeProperty {
            target: Some("Target".into()),
            property: "Opacity".into(),
            value: "{StaticResource MutedOpacity}".into(),
        }],
    };
    document
        .apply("Resource action", behaviors.plan(&document).unwrap())
        .unwrap();
    let children = document.ast().element_children(owner);
    let names = children
        .iter()
        .map(|id| document.ast().node(*id).unwrap().local_name())
        .collect::<Vec<_>>();
    assert_eq!(names[0], "Grid.Resources");
    assert!(
        names
            .iter()
            .position(|name| *name == "Grid.Styles")
            .unwrap()
            > 0
    );
    assert!(
        names
            .iter()
            .position(|name| *name == "Interaction.Behaviors")
            .unwrap()
            > 0
    );
    assert_eq!(document.source().matches("x:Key='MutedOpacity'").count(), 1);
}

#[test]
fn native_spring_and_property_element_easings_roundtrip_with_state_edits() {
    for source in [
        "<Animation Duration='0:0:1'><Animation.Easing><SpringEasing Mass='1' Stiffness='100' Damping='10' InitialVelocity='0'/></Animation.Easing><KeyFrame Cue='0%'><Setter Property='Width' Value='0'/></KeyFrame><KeyFrame Cue='100%'><Setter Property='Width' Value='100'/></KeyFrame></Animation>",
        "<Animation Duration='0:0:1'><Animation.Easing><SplineEasing X1='.2' Y1='0' X2='.8' Y2='1'/></Animation.Easing><KeyFrame Cue='0%'><Setter Property='Width' Value='0'/></KeyFrame><KeyFrame Cue='100%'><Setter Property='Width' Value='100'/></KeyFrame></Animation>",
        "<Animation Duration='0:0:1' Easing='SplineEasing'><KeyFrame Cue='100%'><Setter Property='Width' Value='100'/></KeyFrame></Animation>",
    ] {
        let mut options = ImportOptions::default();
        options.base_values.insert("Width".into(), AnimationValue::Number(0.0));
        let imported = Timeline::import_xaml(source, 1, &options).unwrap().timeline;
        let output = imported.to_xaml(1).unwrap();
        let restored = Timeline::import_xaml(&output, 1, &options).unwrap().timeline;
        assert_eq!(restored, imported);
        assert_eq!(restored.sample(0.45).unwrap(), imported.sample(0.45).unwrap());
    }
    let spring = Easing::parse("Spring(1,100,10,0)").unwrap();
    assert!(spring.sample(0.5).unwrap() > 1.0);
    assert_ne!(spring.sample(1.0).unwrap(), 1.0);
    assert!(Easing::parse("Spring(0,100,10,0)").is_err());
    let mut document =
        programs("<Grid xmlns='https://github.com/ferroui'><Button Name='Target'/></Grid>");
    let mut states = group(document.ast().root());
    states.transitions[0].easing = spring.clone();
    document
        .apply("Spring state", states.plan(&document).unwrap())
        .unwrap();
    let mut states = VisualStateGroup::read(&document).remove(0);
    assert_eq!(states.transitions[0].easing, spring);
    states.transitions[0].easing = Easing::Linear;
    document
        .apply("Linear state", states.plan(&document).unwrap())
        .unwrap();
    assert!(!document.source().contains("SpringEasing"));
    states.transitions[0].easing = spring;
    document
        .apply("Spring again", states.plan(&document).unwrap())
        .unwrap();
    assert_eq!(
        VisualStateGroup::read(&document)[0].transitions[0].easing,
        states.transitions[0].easing
    );
}

#[test]
fn geometry_values_preserve_units_corner_shorthand_and_native_interpolation() {
    for (kind, before, after, midpoint) in [
        (ValueKind::Vector, "-2,4", "6,12", "2,8"),
        (ValueKind::Size, "10,20", "30,60", "20,40"),
        (ValueKind::Rect, "1,2,10,20", "3,4,30,60", "2,3,20,40"),
        (ValueKind::CornerRadius, "1,2", "5,6,7,8", "3,3.5,4.5,5"),
        (ValueKind::RelativePoint, "0%,20%", "100%,60%", "50%,40%"),
        (ValueKind::RelativeScalar, "10%", "90%", "50%"),
    ] {
        let before = AnimationValue::parse(before, kind).unwrap();
        let after = AnimationValue::parse(after, kind).unwrap();
        let expected = AnimationValue::parse(midpoint, kind).unwrap();
        assert_eq!(before.interpolate(&after, 0.5).unwrap(), expected);
        assert_eq!(
            AnimationValue::parse(&expected.to_xaml(), kind).unwrap(),
            expected
        );
        let mut timeline = Timeline::new(1.0).unwrap();
        timeline.record(1, "CustomGeometry", 0.0, before).unwrap();
        timeline.record(1, "CustomGeometry", 1.0, after).unwrap();
        let mut options = ImportOptions::default();
        options.property_kinds.insert("CustomGeometry".into(), kind);
        let restored = Timeline::import_xaml(&timeline.to_xaml(1).unwrap(), 1, &options)
            .unwrap()
            .timeline;
        assert_eq!(restored.sample(0.5).unwrap()[0].value, expected);
    }
    for kind in [ValueKind::RelativePoint, ValueKind::RelativeScalar] {
        let (absolute, relative) = if kind == ValueKind::RelativePoint {
            ("10,20", "10%,20%")
        } else {
            ("10", "10%")
        };
        let absolute = AnimationValue::parse(absolute, kind).unwrap();
        let relative = AnimationValue::parse(relative, kind).unwrap();
        assert_eq!(absolute.interpolate(&relative, 0.49).unwrap(), absolute);
        assert_eq!(absolute.interpolate(&relative, 0.5).unwrap(), relative);
    }
    assert!(AnimationValue::parse("10%,20", ValueKind::RelativePoint).is_err());
    assert!(AnimationValue::parse("NaN,2", ValueKind::Size).is_err());
}

#[test]
fn host_native_interpolation_requirements_never_fall_back_to_discrete_values() {
    let source = "<Animation Duration='0:0:1'><KeyFrame Cue='0%'><Setter Property='BoxShadow' Value='0 0 0 0 Red'/></KeyFrame><KeyFrame Cue='100%'><Setter Property='BoxShadow' Value='20 30 10 4 Blue'/></KeyFrame></Animation>";
    let mut options = ImportOptions::default();
    options.interpolation_errors.insert(
        "BoxShadow".into(),
        "Use the native BoxShadows animator".into(),
    );
    assert!(Timeline::import_xaml(source, 1, &options)
        .unwrap_err()
        .to_string()
        .contains("native BoxShadows"));
    let document = programs(source);
    let mut program = AnimationPrograms::scan(&document).remove(0);
    AnimationCodecRegistry::default().decode(&mut program, source, &options);
    assert!(program.timeline.is_none());
    assert_eq!(program.source, source);
}
#[test]
fn state_validation_rejects_invalid_edges_and_nonfinite_durations() {
    let mut model = group(1);
    model.transitions[0].to = "Missing".into();
    assert!(model.validate().is_err());
    model.transitions[0].to = "Disabled".into();
    model.transitions[0].duration = f64::NAN;
    assert!(model.validate().is_err());
    model.transitions[0].duration = 1.0;
    model.states[1].name = "Normal".into();
    assert!(model.validate().is_err());
}
#[test]
fn behaviors_preserve_unknown_payload_and_use_existing_runtime_markup() {
    let mut document = programs(
        "<Grid xmlns='https://github.com/ferroui' xmlns:plugin='using:Custom'><Button Name='Target'/></Grid>",
    );
    let owner = document.ast().root();
    let behavior=BehaviorModel{id:None,owner,trigger:BehaviorTrigger::Event{event:"PointerPressed".into(),source:None},actions:vec![BehaviorAction::ChangeProperty{target:Some("Target".into()),property:"Opacity".into(),value:"0.4".into()},BehaviorAction::Custom{type_name:"plugin:Action".into(),properties:BTreeMap::from([("xmlns:plugin".into(),"using:Custom".into()),("Payload".into(),"Keep & <value>".into())]),children:"<plugin:Action.Options><!--exact--><plugin:Option Value='x'/></plugin:Action.Options>".into()}]};
    document
        .apply("Add", behavior.plan(&document).unwrap())
        .unwrap();
    let mut read = BehaviorModel::read(&document, owner).unwrap().remove(0);
    assert_eq!(read.actions.len(), 2);
    read.trigger = BehaviorTrigger::Event {
        event: "PointerReleased".into(),
        source: None,
    };
    document
        .apply("Edit", read.plan(&document).unwrap())
        .unwrap();
    assert!(
        matches!(&BehaviorModel::read(&document,owner).unwrap()[0].actions[1],BehaviorAction::Custom{properties,..} if properties.get("Payload").map(String::as_str)==Some("Keep & <value>"))
    );
    assert!(document
        .source()
        .contains("<!--exact--><plugin:Option Value='x'/>"));
    let id = BehaviorModel::read(&document, owner).unwrap()[0]
        .id
        .unwrap();
    document
        .apply("Remove", BehaviorModel::remove(&document, id).unwrap())
        .unwrap();
    assert!(BehaviorModel::read(&document, owner).unwrap().is_empty());
}
#[test]
fn behavior_bound_collection_and_invalid_messages_are_rejected() {
    let document = programs("<Grid Interaction.Behaviors='{Binding Behaviors}'/>");
    let owner = document.ast().root();
    let behavior = BehaviorModel {
        id: None,
        owner,
        trigger: BehaviorTrigger::Data {
            binding: "{Binding IsReady}".into(),
            comparison: "Equal".into(),
            value: "True".into(),
            revert_on_false: true,
        },
        actions: vec![BehaviorAction::AddClass {
            target: None,
            class: "ready".into(),
        }],
    };
    assert!(behavior.plan(&document).is_err());
    let mut invalid = behavior.clone();
    invalid.trigger = BehaviorTrigger::Data {
        binding: "x".into(),
        comparison: "unknown".into(),
        value: "True".into(),
        revert_on_false: false,
    };
    assert!(invalid.to_xaml().is_err());
}

#[test]
fn iteration_delay_and_visibility_policy_roundtrip_and_resume_without_time_jump() {
    let mut timeline = Timeline::new(2.0).unwrap();
    timeline.delay = 1.0;
    timeline.delay_between_iterations = 0.5;
    timeline.repeat = Repeat::Count(3);
    timeline.playback_direction = PlaybackDirection::Alternate;
    timeline.playback_behavior = PlaybackBehavior::OnlyIfVisible;
    timeline
        .record(1, "Width", 0.0, AnimationValue::Number(10.0))
        .unwrap();
    timeline
        .record(1, "Width", 2.0, AnimationValue::Number(30.0))
        .unwrap();
    assert_eq!(timeline.total_duration(), Some(8.0));
    assert_eq!(
        timeline.sample(3.25).unwrap()[0].value,
        AnimationValue::Number(30.0)
    );
    assert_eq!(
        timeline.sample(4.5).unwrap()[0].value,
        AnimationValue::Number(20.0)
    );
    assert_eq!(
        timeline.sample(5.75).unwrap()[0].value,
        AnimationValue::Number(10.0)
    );
    let imported = Timeline::from_xaml(&timeline.to_xaml(1).unwrap(), 1).unwrap();
    assert_eq!(timeline, imported);
    let mut player = Player::new();
    player.play();
    player
        .advance_with_visibility(2.0, &timeline, true, false)
        .unwrap();
    player
        .advance_with_visibility(100.0, &timeline, false, false)
        .unwrap();
    assert_eq!(player.time(), 2.0);
    player
        .advance_with_visibility(0.5, &timeline, true, false)
        .unwrap();
    assert_eq!(player.time(), 2.5);
    timeline.delay_between_iterations = f64::INFINITY;
    assert!(timeline.validate().is_err());
}

#[test]
fn prefixed_behavior_composition_reuses_inherited_namespaces_without_losing_payloads() {
    let mut document =
        Document::new("<ui:Grid xmlns:ui='https://github.com/ferroui' xmlns:p='using:Vendor'/>")
            .unwrap();
    let owner = document.ast().root();
    let model = BehaviorModel {
        id: None,
        owner,
        trigger: BehaviorTrigger::Event {
            event: "PointerPressed".into(),
            source: None,
        },
        actions: vec![BehaviorAction::Custom {
            type_name: "p:Action".into(),
            properties: BTreeMap::new(),
            children: "<p:Action.Child><p:Nested/></p:Action.Child>".into(),
        }],
    };
    document
        .apply("Behavior", model.plan(&document).unwrap())
        .unwrap();
    assert!(document.source().contains("<ui:Interaction.Behaviors>"));
    assert!(document.source().contains("<ui:EventTriggerBehavior"));
    let parsed = BehaviorModel::read(&document, owner).unwrap();
    assert_eq!(parsed.len(), 1);
    document
        .apply("Roundtrip", parsed[0].plan(&document).unwrap())
        .unwrap();
    assert!(document
        .source()
        .contains("<p:Action.Child><p:Nested/></p:Action.Child>"));
}

#[test]
fn copied_state_metadata_and_directly_renamed_selectors_keep_target_references_current() {
    let mut document = Document::new(
        "<Grid xmlns='https://github.com/ferroui' Name='Owner'><Button Name='Target'/></Grid>",
    )
    .unwrap();
    let model = group(document.ast().root());
    document
        .apply("States", model.plan(&document).unwrap())
        .unwrap();
    let renamed = document
        .source()
        .replace("Name='Target'", "Name='Copied'")
        .replace("Button#Target", "Button#Copied");
    let direct = Document::new(renamed.clone()).unwrap();
    assert_eq!(
        VisualStateGroup::read(&direct)[0].states[0].setters[0].target,
        "Copied"
    );
    let rewritten = rewrite_design_metadata(
        &renamed,
        &BTreeMap::from([("Target".into(), "Copied".into())]),
    )
    .unwrap();
    let rewritten = Document::new(rewritten).unwrap();
    let groups = VisualStateGroup::read(&rewritten);
    assert_eq!(groups[0].owner, rewritten.ast().root());
    assert!(groups[0]
        .states
        .iter()
        .all(|state| state.setters.iter().all(|setter| setter.target == "Copied")));
}
#[test]
fn custom_behavior_text_comments_and_property_payloads_survive_edits() {
    let source = "<Grid xmlns='https://github.com/ferroui' xmlns:p='using:Vendor'><Interaction.Behaviors><p:Behavior Flag='x'>lead<!--keep--><![CDATA[a<b]]><p:Behavior.Value><p:Data/></p:Behavior.Value>tail</p:Behavior></Interaction.Behaviors></Grid>";
    let mut document = Document::new(source).unwrap();
    let owner = document.ast().root();
    let mut model = BehaviorModel::read(&document, owner).unwrap().remove(0);
    if let BehaviorTrigger::Custom { properties, .. } = &mut model.trigger {
        properties.insert("Flag".into(), "changed".into());
    }
    document
        .apply("Edit", model.plan(&document).unwrap())
        .unwrap();
    assert!(document.source().contains(
        "lead<!--keep--><![CDATA[a<b]]><p:Behavior.Value><p:Data/></p:Behavior.Value>tail"
    ));
}

#[test]
fn native_resource_identity_survives_edits_and_custom_animation_names_are_not_misclassified() {
    let source="<Grid xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' xmlns:vendor='using:Vendor'><Grid.Resources><Animation x:Key='Fade' Duration='0:0:2'/><vendor:Animation Duration='0:0:3'/></Grid.Resources></Grid>";
    let document = Document::new(source).unwrap();
    let mut programs = AnimationPrograms::scan(&document);
    let registry = AnimationCodecRegistry::default();
    assert_eq!(programs[0].name, "Fade");
    for program in &mut programs {
        registry.decode(
            program,
            &animation_fragment(&document, program.id).unwrap(),
            &ImportOptions::default(),
        );
    }
    assert!(programs[0].timeline.is_some());
    assert!(programs[1].timeline.is_none());
    let mut timeline = programs[0].timeline.clone().unwrap();
    timeline.duration = 5.0;
    let encoded = registry
        .encode(
            &programs[0],
            &animation_fragment(&document, programs[0].id).unwrap(),
            &timeline,
        )
        .unwrap();
    let decoded = Document::new(encoded).unwrap();
    assert_eq!(
        decoded
            .ast()
            .node(decoded.ast().root())
            .unwrap()
            .attribute("x:Key"),
        Some("Fade")
    );
}

#[test]
fn program_uuid_survives_reopening_and_edits_but_duplicate_allocates_a_new_identity() {
    let mut document = Document::new("<Grid/>").unwrap();
    let owner = document.ast().root();
    document
        .apply(
            "Create",
            AnimationPrograms::create(
                &document,
                owner,
                "First",
                "Button",
                &Timeline::new(2.0).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    let program = AnimationPrograms::scan(&document).remove(0);
    let key = program.source_key.clone().unwrap();
    document
        .apply(
            "Rename",
            AnimationPrograms::rename(&document, program.id, "Rename").unwrap(),
        )
        .unwrap();
    assert_eq!(
        AnimationPrograms::scan(&document)[0].source_key.as_ref(),
        Some(&key)
    );
    let reopened = Document::new(document.source()).unwrap();
    assert_eq!(
        AnimationPrograms::scan(&reopened)[0].source_key.as_ref(),
        Some(&key)
    );
    document
        .apply(
            "Duplicate",
            AnimationPrograms::duplicate(&document, program.id, "Copy").unwrap(),
        )
        .unwrap();
    let all = AnimationPrograms::scan(&document);
    assert_ne!(all[0].source_key, all[1].source_key);
    let rewritten = Document::new(rekey_copied_programs(document.source()).unwrap()).unwrap();
    assert_ne!(AnimationPrograms::scan(&rewritten)[0].source_key, Some(key));
}

#[test]
fn duplicated_resource_program_gets_a_distinct_runtime_key_and_source_uuid() {
    let mut document=Document::new("<Grid xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Grid.Resources><Animation x:Key='Fade' x:SetterTargetType='Button' Duration='0:0:1'/></Grid.Resources></Grid>").unwrap();
    let program = AnimationPrograms::scan(&document).remove(0);
    document
        .apply(
            "Copy",
            AnimationPrograms::duplicate(&document, program.id, "Fade").unwrap(),
        )
        .unwrap();
    let programs = AnimationPrograms::scan(&document);
    let copied = document.ast().node(programs[1].id).unwrap();
    assert_eq!(copied.attribute("x:Key"), Some("Fade1"));
    assert_eq!(copied.attribute("x:SetterTargetType"), Some("Button"));
}

#[test]
fn host_literal_resolver_supports_native_named_colors_without_changing_source_on_import() {
    let source="<Animation Duration='0:0:2'><KeyFrame Cue='0%'><Setter Property='Background' Value='Red'/></KeyFrame><KeyFrame Cue='100%'><Setter Property='Background' Value='Blue'/></KeyFrame></Animation>";
    let mut options = ImportOptions::default();
    options.literal_values.insert(
        ("Background".into(), "Red".into()),
        AnimationValue::parse("#FF0000", ValueKind::Color).unwrap(),
    );
    options.literal_values.insert(
        ("Background".into(), "Blue".into()),
        AnimationValue::parse("#0000FF", ValueKind::Color).unwrap(),
    );
    let timeline = Timeline::import_xaml(source, 1, &options).unwrap().timeline;
    assert_eq!(timeline.scrub(1.0).unwrap()[0].value.to_xaml(), "#FFBC00BC");
    options
        .property_kinds
        .insert("Background".into(), ValueKind::Number);
    assert!(Timeline::import_xaml(source, 1, &options).is_err());
}

#[test]
fn resource_programs_are_not_mistaken_for_running_animations_in_the_enclosing_style() {
    let document=Document::new("<Grid><Button Name='Owner'><Button.Resources><Animation Duration='0:0:1'/></Button.Resources><Button.Styles><Style Selector='Button'><Style.Resources><Animation Duration='0:0:2'/></Style.Resources><Style.Animations><Animation Duration='0:0:3'/></Style.Animations></Style></Button.Styles></Button></Grid>").unwrap();
    let programs = AnimationPrograms::scan(&document);
    let owner = document.ast().find_name("Owner").unwrap();
    assert_eq!(programs.len(), 3);
    assert_eq!(programs[0].owner, owner);
    assert!(programs[0].style.is_none());
    assert!(programs[1].style.is_none());
    assert!(programs[2].style.is_some());
}

#[test]
fn state_endpoint_edits_preserve_authored_program_identity_keys_payloads_and_timing() {
    let mut document = Document::new(
        "<Grid xmlns='https://github.com/ferroui' Name='Owner'><Button Name='Target'/></Grid>",
    )
    .unwrap();
    let owner = document.ast().root();
    document
        .apply("States", group(owner).plan(&document).unwrap())
        .unwrap();
    let transition = document
        .ast()
        .elements()
        .find(|node| {
            node.local_name() == "Animation"
                && node
                    .attribute("Duration")
                    .and_then(|value| crate::xaml::parse_time(value).ok())
                    == Some(0.5)
        })
        .unwrap()
        .id;
    document
        .apply(
            "Name transition",
            AnimationPrograms::rename(&document, transition, "Authored transition").unwrap(),
        )
        .unwrap();
    let identity = AnimationPrograms::scan(&document)
        .into_iter()
        .find(|program| program.id == transition)
        .unwrap()
        .source_key;
    let collection = document.ast().node(transition).unwrap().parent.unwrap();
    let final_frame = document
        .ast()
        .element_children(transition)
        .into_iter()
        .rfind(|id| document.ast().node(*id).unwrap().local_name() == "KeyFrame")
        .unwrap();
    document.apply("Authored animation", vec![
        Edit::SetAttribute { node: transition, name: "Duration".into(), value: "0:0:2".into() },
        Edit::SetAttribute { node: final_frame, name: "KeyTime".into(), value: "0:0:2".into() },
        Edit::SetAttribute { node: transition, name: "Delay".into(), value: "0:0:0.25".into() },
        Edit::SetAttribute { node: transition, name: "xmlns:vendor".into(), value: "using:Vendor".into() },
        Edit::InsertChild { parent: transition, index: 1, source: "<KeyFrame KeyTime='0:0:1' KeySpline='0.2,0.3,0.6,0.7'><Setter Property='Opacity' Value='0.6'/></KeyFrame>".into() },
        Edit::InsertChild { parent: transition, index: 3, source: "<vendor:CustomAnimator Payload='keep &amp; preserve'/>".into() },
        Edit::InsertChild { parent: collection, index: 1, source: "<Animation Duration='0:0:9' PlaybackBehavior='Always'><!--independent--><KeyFrame Cue='100%'><Setter Property='Height' Value='42'/></KeyFrame></Animation>".into() },
    ]).unwrap();
    let mut changed = VisualStateGroup::read(&document).remove(0);
    assert_eq!(
        changed
            .transitions
            .iter()
            .find(|transition| transition.from.as_deref() == Some("Normal"))
            .unwrap()
            .duration,
        2.0
    );
    changed
        .states
        .iter_mut()
        .find(|state| state.name == "Disabled")
        .unwrap()
        .setters
        .iter_mut()
        .find(|setter| setter.property == "Opacity")
        .unwrap()
        .value = "0.2".into();
    document
        .apply("Change state endpoint", changed.plan(&document).unwrap())
        .unwrap();
    let all = AnimationPrograms::scan(&document);
    let program = all
        .iter()
        .find(|program| program.name == "Authored transition")
        .unwrap();
    assert_eq!(program.source_key, identity);
    let source = animation_fragment(&document, program.id).unwrap();
    assert!(source.contains("KeySpline='0.2,0.3,0.6,0.7'"));
    assert!(source.contains("KeyTime='0:0:1'"));
    assert!(source.contains("Value='0.6'"));
    assert!(source.contains("vendor:CustomAnimator Payload='keep &amp; preserve'"));
    assert!(source.contains("Delay=\"0:0:0.25\""));
    let parsed = Document::new(&source).unwrap();
    assert_eq!(
        parsed
            .ast()
            .node(parsed.ast().root())
            .unwrap()
            .attribute("Duration"),
        Some("0:0:2")
    );
    assert!(parsed
        .ast()
        .elements()
        .any(|node| node.local_name() == "Setter" && node.attribute("Value") == Some("0.2")));
    assert!(document.source().contains("<!--independent-->"));
    assert!(all.iter().any(|program| document
        .ast()
        .node(program.id)
        .unwrap()
        .attribute("Duration")
        == Some("0:0:9")));
    changed
        .transitions
        .iter_mut()
        .find(|transition| transition.from.as_deref() == Some("Normal"))
        .unwrap()
        .duration = 4.0;
    document
        .apply("Retime transition", changed.plan(&document).unwrap())
        .unwrap();
    let program = AnimationPrograms::scan(&document)
        .into_iter()
        .find(|program| program.name == "Authored transition")
        .unwrap();
    let parsed = Document::new(animation_fragment(&document, program.id).unwrap()).unwrap();
    assert_eq!(
        parsed
            .ast()
            .node(parsed.ast().root())
            .unwrap()
            .attribute("Duration"),
        Some("0:0:4")
    );
    assert!(parsed
        .ast()
        .elements()
        .any(|node| node.local_name() == "KeyFrame" && node.attribute("KeyTime") == Some("0:0:2")));
    assert_eq!(program.source_key, identity);
}

#[test]
fn state_targets_respect_template_name_scopes_and_reject_ambiguous_names() {
    let document = Document::new("<Grid><Grid.Resources><ControlTemplate><Button Name='Target'/></ControlTemplate></Grid.Resources><Button Name='Target'/></Grid>").unwrap();
    let owner = document.ast().root();
    let outside = document
        .ast()
        .element_children(owner)
        .into_iter()
        .find(|id| document.ast().node(*id).unwrap().local_name() == "Button")
        .unwrap();
    assert_eq!(
        resolve_state_target(&document, owner, "Target").unwrap(),
        outside
    );
    let mut only_template = document.clone();
    only_template
        .apply("Remove outside", vec![Edit::RemoveNode { node: outside }])
        .unwrap();
    assert!(resolve_state_target(&only_template, owner, "Target").is_err());
    let ambiguous =
        Document::new("<Grid><Button Name='Target'/><Button Name='Target'/></Grid>").unwrap();
    assert!(resolve_state_target(&ambiguous, ambiguous.ast().root(), "Target").is_err());
}

#[test]
fn state_edits_keep_collection_order_complex_setters_namespaces_and_independent_duplicates() {
    let mut document = Document::new(
        "<Grid xmlns='https://github.com/ferroui' Name='Owner'><Button Name='Target'/></Grid>",
    )
    .unwrap();
    let owner = document.ast().root();
    document
        .apply("States", group(owner).plan(&document).unwrap())
        .unwrap();
    let owned = AnimationPrograms::scan(&document).remove(0);
    let collection = document.ast().node(owned.id).unwrap().parent.unwrap();
    let state_style = document.ast().node(collection).unwrap().parent.unwrap();
    let group_style = document.ast().node(state_style).unwrap().parent.unwrap();
    document.apply("Extension payloads", vec![
        Edit::SetAttribute { node: group_style, name: "xmlns:p".into(), value: "using:Vendor".into() },
        Edit::SetAttribute { node: group_style, name: "p:Metadata".into(), value: "keep".into() },
        Edit::InsertChild { parent: state_style, index: 0, source: "<Setter Property='Background'><Setter.Value><p:Brush Payload='exact'/></Setter.Value></Setter>".into() },
        Edit::InsertChild { parent: collection, index: 0, source: "<!--before-all-->".into() },
    ]).unwrap();
    document
        .apply(
            "Independent copy",
            AnimationPrograms::duplicate(&document, owned.id, "Independent").unwrap(),
        )
        .unwrap();
    let independent = AnimationPrograms::scan(&document)
        .into_iter()
        .find(|program| program.name == "Independent")
        .unwrap();
    assert!(crate::states::state_program_role(&document, independent.id).is_none());
    let order = AnimationPrograms::scan(&document)
        .into_iter()
        .filter(|program| program.style == Some(state_style))
        .map(|program| program.source_key)
        .collect::<Vec<_>>();
    let mut model = VisualStateGroup::read(&document).remove(0);
    model.states[1].setters[0].value = "0.2".into();
    document
        .apply("Unrelated state edit", model.plan(&document).unwrap())
        .unwrap();
    assert!(
        document.source().contains("<p:Brush Payload='exact'/>")
            && document.source().contains("p:Metadata=\"keep\"")
    );
    let all = AnimationPrograms::scan(&document);
    let current = all
        .iter()
        .find(|program| program.source_key == owned.source_key)
        .unwrap();
    assert_eq!(
        all.iter()
            .filter(|program| program.style == current.style)
            .map(|program| program.source_key.clone())
            .collect::<Vec<_>>(),
        order
    );
    assert!(document.source().contains("<!--before-all-->"));
    let mut model = VisualStateGroup::read(&document).remove(0);
    model.states[0].setters.clear();
    document
        .apply("Clear owned properties", model.plan(&document).unwrap())
        .unwrap();
    let all = AnimationPrograms::scan(&document);
    let empty_owned = all
        .iter()
        .find(|program| program.source_key == owned.source_key)
        .unwrap();
    let parsed = Document::new(animation_fragment(&document, empty_owned.id).unwrap()).unwrap();
    assert!(
        !parsed
            .ast()
            .elements()
            .any(|node| node.local_name() == "Setter"
                && node.attribute("Property") == Some("Opacity"))
    );
    let independent_after = all
        .iter()
        .find(|program| program.source_key == independent.source_key)
        .unwrap();
    let parsed =
        Document::new(animation_fragment(&document, independent_after.id).unwrap()).unwrap();
    assert!(
        parsed
            .ast()
            .elements()
            .any(|node| node.local_name() == "Setter"
                && node.attribute("Property") == Some("Opacity"))
    );
    assert!(VisualStateGroup::read(&document)[0].states[0]
        .setters
        .is_empty());
    assert!(
        document.source().contains("<p:Brush Payload='exact'/>")
            && document.source().contains("xmlns:p=")
    );
}

#[test]
fn changing_state_value_kind_or_property_prunes_obsolete_owned_tracks() {
    let mut document =
        Document::new("<Grid xmlns='https://github.com/ferroui'><Button Name='Target'/></Grid>")
            .unwrap();
    let owner = document.ast().root();
    document
        .apply("States", group(owner).plan(&document).unwrap())
        .unwrap();
    let original = AnimationPrograms::scan(&document).remove(0);
    let mut model = VisualStateGroup::read(&document).remove(0);
    model.states[0].setters[0].value = "{Binding Level}".into();
    document
        .apply("Bind state", model.plan(&document).unwrap())
        .unwrap();
    assert_eq!(
        VisualStateGroup::read(&document)[0].states[0].setters[0].value,
        "{Binding Level}"
    );
    let current = AnimationPrograms::scan(&document)
        .into_iter()
        .find(|program| program.source_key == original.source_key)
        .unwrap();
    let literal = Document::new(animation_fragment(&document, current.id).unwrap()).unwrap();
    assert!(!literal
        .ast()
        .elements()
        .any(|node| node.local_name() == "Setter"));
    let mut model = VisualStateGroup::read(&document).remove(0);
    model.states[0].setters[0].property = "Width".into();
    model.states[0].setters[0].value = "250".into();
    document
        .apply("Change state property", model.plan(&document).unwrap())
        .unwrap();
    let all = AnimationPrograms::scan(&document);
    let current = all
        .iter()
        .find(|program| program.source_key == original.source_key)
        .unwrap();
    for program in all.iter().filter(|program| program.style == current.style) {
        let parsed = Document::new(animation_fragment(&document, program.id).unwrap()).unwrap();
        assert!(!parsed
            .ast()
            .elements()
            .any(|node| node.local_name() == "Setter"
                && node.attribute("Property") == Some("Opacity")));
    }
    let literal = Document::new(animation_fragment(&document, current.id).unwrap()).unwrap();
    assert!(literal
        .ast()
        .elements()
        .any(|node| node.local_name() == "Setter"
            && node.attribute("Property") == Some("Width")
            && node.attribute("Value") == Some("250")));
}

#[test]
fn directional_transitions_drop_obsolete_owned_start_values() {
    for binding in [false, true] {
        let mut document = Document::new(
            "<Grid xmlns='https://github.com/ferroui'><Button Name='Target'/></Grid>",
        )
        .unwrap();
        let owner = document.ast().root();
        document
            .apply("States", group(owner).plan(&document).unwrap())
            .unwrap();
        let program = AnimationPrograms::scan(&document)
            .into_iter()
            .find(|program| {
                document
                    .ast()
                    .node(program.id)
                    .unwrap()
                    .attribute("Duration")
                    .and_then(|value| crate::xaml::parse_time(value).ok())
                    == Some(0.5)
            })
            .unwrap();
        let mut model = VisualStateGroup::read(&document).remove(0);
        if binding {
            model.states[0].setters[0].value = "{Binding Level}".into();
        } else {
            model.states[0].setters.clear();
        }
        document
            .apply("Remove literal start", model.plan(&document).unwrap())
            .unwrap();
        let changed = AnimationPrograms::scan(&document)
            .into_iter()
            .find(|candidate| candidate.source_key == program.source_key)
            .unwrap();
        let source = Document::new(animation_fragment(&document, changed.id).unwrap()).unwrap();
        for frame in source.ast().elements().filter(|node| {
            node.local_name() == "KeyFrame"
                && node
                    .attribute("KeyTime")
                    .and_then(|value| crate::xaml::parse_time(value).ok())
                    == Some(0.0)
        }) {
            assert!(!source
                .ast()
                .element_children(frame.id)
                .into_iter()
                .any(|id| source.ast().node(id).unwrap().attribute("Property") == Some("Opacity")));
        }
    }
}

#[test]
fn unchanged_complex_state_setters_survive_but_explicit_changes_replace_them() {
    let mut document = Document::new("<Grid xmlns='https://github.com/ferroui' xmlns:p='using:Vendor'><Button Name='Target'/></Grid>").unwrap();
    let owner = document.ast().root();
    document
        .apply("States", group(owner).plan(&document).unwrap())
        .unwrap();
    let original = AnimationPrograms::scan(&document).remove(0);
    let style = original.style.unwrap();
    let setter = document
        .ast()
        .element_children(style)
        .into_iter()
        .find(|id| document.ast().node(*id).unwrap().local_name() == "Setter")
        .unwrap();
    document
        .apply(
            "Custom object value",
            vec![
                Edit::RemoveAttribute {
                    node: setter,
                    name: "Value".into(),
                },
                Edit::InsertChild {
                    parent: setter,
                    index: 0,
                    source: "<Setter.Value><p:Scalar Value='1'/></Setter.Value>".into(),
                },
            ],
        )
        .unwrap();
    let mut model = VisualStateGroup::read(&document).remove(0);
    model.states[1].setters[0].value = "0.2".into();
    document
        .apply("Other state", model.plan(&document).unwrap())
        .unwrap();
    assert!(document.source().contains("<p:Scalar Value='1'/>"));
    let program = AnimationPrograms::scan(&document)
        .into_iter()
        .find(|candidate| candidate.source_key == original.source_key)
        .unwrap();
    assert_eq!(
        document
            .ast()
            .element_children(program.style.unwrap())
            .into_iter()
            .filter(|id| document.ast().node(*id).unwrap().attribute("Property") == Some("Opacity"))
            .count(),
        1
    );
    let mut model = VisualStateGroup::read(&document).remove(0);
    model.states[0].setters[0].value = "0.8".into();
    document
        .apply("Explicit endpoint", model.plan(&document).unwrap())
        .unwrap();
    assert!(!document.source().contains("p:Scalar"));
}

#[test]
fn retargeting_state_preserves_existing_namespace_aliases_without_rebinding_new_selectors() {
    let mut document = Document::new("<Grid xmlns='https://github.com/ferroui'><a:Control xmlns:a='using:First' Name='First'/><b:Control xmlns:b='using:Second' Name='Second'/></Grid>").unwrap();
    let owner = document.ast().root();
    let mut model = group(owner);
    for setter in model.states.iter_mut().flat_map(|state| &mut state.setters) {
        setter.target = "First".into();
    }
    document
        .apply("First namespace", model.plan(&document).unwrap())
        .unwrap();
    let mut model = VisualStateGroup::read(&document).remove(0);
    for setter in model.states.iter_mut().flat_map(|state| &mut state.setters) {
        setter.target = "Second".into();
    }
    document
        .apply("Other namespace", model.plan(&document).unwrap())
        .unwrap();
    for style in document.ast().elements().filter(|node| {
        node.local_name() == "Style"
            && node
                .attribute("Selector")
                .is_some_and(|selector| selector.ends_with("#Second"))
    }) {
        let target = style
            .attribute("Selector")
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap();
        let prefix = target.split('|').next().unwrap();
        assert_eq!(
            document
                .ast()
                .namespaces(style.id)
                .get(prefix)
                .map(String::as_str),
            Some("using:Second")
        );
    }
    assert!(
        document.source().contains("using:First") && document.source().contains("using:Second")
    );
}
