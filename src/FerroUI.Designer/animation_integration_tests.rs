use crate::{DesignerSession, PreviewStatus};
use ferroui_base::StyledElement;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, ContentControl};
use ferroui_designer_animation::{
    AnimationValue, BehaviorAction, BehaviorModel, BehaviorTrigger, Easing, StateSetter,
    StateTransition, VisualState, VisualStateGroup,
};
use ferroui_designer_core::Edit;

fn source() -> &'static str {
    r#"<Canvas xmlns="https://github.com/ferroui" Name="Owner" Width="400" Height="240"><Button Name="A" Width="100" Height="40" Opacity="1"/><Button Name="B" Width="200" Height="40" Opacity="1" Canvas.Top="60"/></Canvas>"#
}
fn attach_preview(session: &DesignerSession) -> ferroui_base::Ref<ferroui_controls::Window> {
    let window = ferroui_controls::Window::new();
    window.set_width(400.0);
    window.set_height(240.0);
    window.set_content(Some(ferroui_controls::Control::boxed(
        &session.preview.host,
    )));
    window.show();
    window.update_layout();
    window
}
fn state_group(owner: u64) -> VisualStateGroup {
    VisualStateGroup {
        name: "Interaction".into(),
        owner,
        states: vec![
            VisualState {
                name: "Normal".into(),
                setters: vec![StateSetter {
                    target: "A".into(),
                    property: "Opacity".into(),
                    value: "1".into(),
                }],
            },
            VisualState {
                name: "Muted".into(),
                setters: vec![StateSetter {
                    target: "A".into(),
                    property: "Opacity".into(),
                    value: "0.25".into(),
                }],
            },
        ],
        transitions: vec![StateTransition {
            from: Some("Normal".into()),
            to: "Muted".into(),
            duration: 0.4,
            easing: Easing::Linear,
        }],
    }
}
#[test]
fn independent_programs_select_edit_and_preview_all_matching_targets() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(source(), ContentControl::new()).unwrap();
    let owner = session.document().ast().root();
    let a = session.document().ast().find_name("A").unwrap();
    let b = session.document().ast().find_name("B").unwrap();
    let id = session
        .create_animation_program("Width", owner, "Button", 2.0)
        .unwrap();
    session
        .record_keyframe(a, "Width", AnimationValue::Number(100.0))
        .unwrap();
    session.seek(2.0).unwrap();
    session
        .record_keyframe(a, "Width", AnimationValue::Number(300.0))
        .unwrap();
    assert_eq!(session.animation_programs().len(), 1);
    assert_eq!(session.animation_programs()[0].targets.len(), 2);
    assert_eq!(
        session
            .preview
            .visual(a)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        300.0,
        "editing a key at the scrub position must update the artboard immediately"
    );
    let second = session
        .create_animation_program("Opacity", owner, "Button#A", 4.0)
        .unwrap();
    session
        .record_keyframe(a, "Opacity", AnimationValue::Number(1.0))
        .unwrap();
    session.seek(4.0).unwrap();
    session
        .record_keyframe(a, "Opacity", AnimationValue::Number(0.0))
        .unwrap();
    session
        .preview_animation_programs(&[id, second], 1.0)
        .unwrap();
    assert_eq!(
        session
            .preview
            .visual(a)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        200.0
    );
    assert_eq!(
        session
            .preview
            .visual(b)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        200.0
    );
    assert!((session.preview.visual(a).unwrap().opacity() - 0.75).abs() < 1e-9);
    session.select_animation_program(id).unwrap();
    session.set_duration(5.0).unwrap();
    assert_eq!(
        session
            .animation_programs()
            .iter()
            .find(|program| program.id == second)
            .unwrap()
            .timeline
            .as_ref()
            .unwrap()
            .duration,
        4.0
    );
    session
        .rename_animation_program(id, "Width -- renamed")
        .unwrap();
    assert_eq!(session.active_animation_program(), Some(id));
    let duplicate = session.duplicate_animation_program(id, "Copy").unwrap();
    assert_ne!(duplicate, id);
    session.remove_animation_program(duplicate).unwrap();
    session.stop();
    assert_eq!(
        session
            .preview
            .visual(b)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        200.0
    );
}
#[test]
fn shared_selector_implicit_endpoints_use_each_targets_own_base_value() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui"><Canvas.Styles><Style Selector="Button"><Style.Animations><Animation Duration="0:0:2"><KeyFrame Cue="50%"><Setter Property="Width" Value="300"/></KeyFrame></Animation></Style.Animations></Style></Canvas.Styles><Button Name="A" Width="100"/><Button Name="B" Width="200"/></Canvas>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(
        session.animation_warnings().is_empty(),
        "{:?}",
        session.animation_warnings()
    );
    let a = session.document().ast().find_name("A").unwrap();
    let b = session.document().ast().find_name("B").unwrap();
    session.seek(0.5).unwrap();
    assert_eq!(
        session
            .preview
            .visual(a)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        200.0
    );
    assert_eq!(
        session
            .preview
            .visual(b)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        250.0
    );
}
#[test]
fn state_groups_and_existing_behavior_actions_load_preview_and_restore() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(source(), ContentControl::new()).unwrap();
    let window = attach_preview(&session);
    let owner = session.document().ast().root();
    let a = session.document().ast().find_name("A").unwrap();
    let group = state_group(owner);
    session.upsert_state_group(group.clone()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    session
        .preview_state(owner, "Interaction", Some("Muted"))
        .unwrap();
    assert!((session.preview.visual(a).unwrap().opacity() - 0.25).abs() < 1e-9);
    assert_eq!(
        session
            .document()
            .ast()
            .node(a)
            .unwrap()
            .attribute("Opacity"),
        Some("1")
    );
    session.stop();
    assert_eq!(session.preview.visual(a).unwrap().opacity(), 1.0);
    let behavior = BehaviorModel {
        id: None,
        owner,
        trigger: BehaviorTrigger::Event {
            event: "PointerPressed".into(),
            source: None,
        },
        actions: vec![BehaviorAction::ChangeProperty {
            target: Some("A".into()),
            property: "Width".into(),
            value: "275".into(),
        }],
    };
    let id = session.upsert_behavior(behavior).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    session.preview_behavior(id).unwrap();
    assert_eq!(
        session
            .preview
            .visual(a)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        275.0
    );
    session.stop();
    assert_eq!(
        session
            .preview
            .visual(a)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        100.0
    );
    session.remove_behavior(id).unwrap();
    assert!(session.behaviors(owner).is_empty());
    window.close();
}
#[test]
fn state_event_actions_route_the_current_from_state_through_existing_conditional_actions() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(source(), ContentControl::new()).unwrap();
    let owner = session.document().ast().root();
    let group = state_group(owner);
    session.upsert_state_group(group.clone()).unwrap();
    let actions = session
        .state_group_actions(owner, "Interaction", "Muted")
        .unwrap();
    let id = session
        .upsert_behavior(BehaviorModel {
            id: None,
            owner,
            trigger: BehaviorTrigger::Event {
                event: "PointerPressed".into(),
                source: None,
            },
            actions,
        })
        .unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    session
        .preview_state(owner, "Interaction", Some("Normal"))
        .unwrap();
    assert!(session
        .preview_state_at(owner, "Interaction", Some("Muted"), Some(0.2))
        .unwrap_err()
        .contains("rooted logical tree"));
    assert!(session
        .preview_behavior(id)
        .unwrap_err()
        .contains("rooted logical tree"));
    let window = attach_preview(&session);
    session.preview_behavior(id).unwrap();
    let classes = session
        .preview
        .visual(owner)
        .unwrap()
        .cast::<StyledElement>()
        .unwrap()
        .classes();
    assert!(classes.contains(&group.class_name("Muted")));
    assert!(!classes.contains(&group.class_name("Normal")));
    assert!(
        classes.contains(&group.transition_class("Normal", "Muted")),
        "directional transition was not selected"
    );
    window.close();
}
#[test]
fn viewer_sessions_reject_all_authored_edits_and_leave_restores_editing() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(source(), ContentControl::new()).unwrap();
    let a = session.document().ast().find_name("A").unwrap();
    session.set_property(&[a], "Width", "110").unwrap();
    let source = session.document().source().to_owned();
    let revision = session.document().revision();
    session.set_read_only(true);
    session.select([a]);
    session.seek(0.0).unwrap();
    assert!(session
        .apply(
            "Forbidden",
            vec![Edit::SetAttribute {
                node: a,
                name: "Width".into(),
                value: "200".into()
            }]
        )
        .is_err());
    assert!(session.apply_at(revision, "Forbidden", vec![]).is_err());
    assert!(session.undo().is_err());
    assert!(session.redo().is_err());
    assert!(!session.update_source("<Grid/>").valid);
    assert!(!session.stage_source("<Border/>").valid);
    assert!(session
        .record_keyframe(a, "Width", AnimationValue::Number(200.0))
        .is_err());
    assert!(session.commit_animation().is_err());
    session.set_recording(true);
    assert!(!session.recording());
    assert_eq!(session.document().source(), source);
    assert_eq!(session.document().revision(), revision);
    session.leave_collaboration();
    assert!(!session.is_read_only());
    session.undo().unwrap();
}

#[test]
fn resource_animation_program_uses_explicit_preview_targets_and_preserves_resource_identity() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Canvas.Resources><Animation x:Key="Fade" x:SetterTargetType="Button" Duration="0:0:2"><KeyFrame Cue="0%"><Setter Property="Opacity" Value="0"/></KeyFrame><KeyFrame Cue="100%"><Setter Property="Opacity" Value="1"/></KeyFrame></Animation></Canvas.Resources><Button Name="A" Opacity="1"/></Canvas>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let id = session.animation_programs()[0].id;
    let target = session.document().ast().find_name("A").unwrap();
    session
        .set_animation_program_preview_targets(id, Some(vec![target]))
        .unwrap();
    session.seek(1.0).unwrap();
    assert!((session.preview.visual(target).unwrap().opacity() - 0.5).abs() < 1e-9);
    session.set_duration(4.0).unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("x:Key"),
        Some("Fade")
    );
    session.duplicate_animation_program(id, "Copy").unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
}

#[test]
fn native_named_brush_colors_are_interpolated_as_colors_and_preview_stays_source_free() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Button xmlns="https://github.com/ferroui" Name="A" Background="Red"><Button.Styles><Style Selector="Button#A"><Style.Animations><Animation Duration="0:0:2"><KeyFrame Cue="0%"><Setter Property="Background" Value="Red"/></KeyFrame><KeyFrame Cue="100%"><Setter Property="Background" Value="Blue"/></KeyFrame></Animation></Style.Animations></Style></Button.Styles></Button>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(
        session.animation_warnings().is_empty(),
        "{:?}",
        session.animation_warnings()
    );
    assert!(matches!(
        session.timeline().tracks[0].keyframes[0].value,
        AnimationValue::Color { .. }
    ));
    session.seek(1.0).unwrap();
    assert_eq!(session.document().source(), source);
}

#[test]
fn stopping_behavior_preview_does_not_restore_mutated_unbound_text_as_hot_reload_state() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(
        "<Canvas xmlns='https://github.com/ferroui'><TextBox Name='Input'/></Canvas>",
        ContentControl::new(),
    )
    .unwrap();
    let window = attach_preview(&session);
    let owner = session.document().ast().root();
    let input = session.document().ast().find_name("Input").unwrap();
    let behavior = session
        .upsert_behavior(BehaviorModel {
            id: None,
            owner,
            trigger: BehaviorTrigger::Event {
                event: "PointerPressed".into(),
                source: None,
            },
            actions: vec![BehaviorAction::ChangeProperty {
                target: Some("Input".into()),
                property: "Text".into(),
                value: "preview only".into(),
            }],
        })
        .unwrap();
    session.preview_behavior(behavior).unwrap();
    assert_eq!(
        session
            .preview
            .visual(input)
            .unwrap()
            .cast::<ferroui_controls::TextBox>()
            .unwrap()
            .text()
            .as_deref(),
        Some("preview only")
    );
    session.stop();
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert_ne!(
        session
            .preview
            .visual(input)
            .unwrap()
            .cast::<ferroui_controls::TextBox>()
            .unwrap()
            .text()
            .as_deref(),
        Some("preview only")
    );
    window.close();
}

#[test]
fn attached_state_clock_samples_literal_and_resource_transitions_through_native_animation() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for resource in [false, true] {
        let host = ContentControl::new();
        let mut session = DesignerSession::new("<Canvas xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Name='Owner' Width='400' Height='240'><Canvas.Resources><x:Double x:Key='MutedOpacity'>0.25</x:Double></Canvas.Resources><Button Name='A' Opacity='1' Width='100' Height='40'/></Canvas>", host.clone()).unwrap();
        let window = ferroui_controls::Window::new();
        window.set_width(400.0);
        window.set_height(240.0);
        window.set_content(Some(ferroui_controls::Control::boxed(&host)));
        window.show();
        window.update_layout();
        let owner = session.document().ast().root();
        let a = session.document().ast().find_name("A").unwrap();
        let mut group = state_group(owner);
        if resource {
            group.states[1].setters[0].value = "{StaticResource MutedOpacity}".into();
        }
        session.upsert_state_group(group).unwrap();
        assert!(
            matches!(session.preview_status(), PreviewStatus::Ready { .. }),
            "{:?}",
            session.preview_status()
        );
        let authored = session.document().source().to_owned();
        session
            .preview_state(owner, "Interaction", Some("Normal"))
            .unwrap();
        session
            .preview_state_at(owner, "Interaction", Some("Muted"), Some(0.2))
            .unwrap();
        assert!(
            (session.preview.visual(a).unwrap().opacity() - 0.625).abs() < 1e-6,
            "native midpoint with resource={resource}: {}",
            session.preview.visual(a).unwrap().opacity()
        );
        session.advance_state_preview(0.2).unwrap();
        assert!((session.preview.visual(a).unwrap().opacity() - 0.25).abs() < 1e-6);
        assert_eq!(session.document().source(), authored);
        session.stop();
        assert_eq!(session.preview.visual(a).unwrap().opacity(), 1.0);
        window.close();
    }
}

#[test]
fn animation_setter_target_type_uses_a_fresh_alias_when_source_prefixes_are_shadowed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new("<Canvas xmlns='https://github.com/ferroui'><Canvas.Styles><Style Selector='Button'><Style.Animations><Animation xmlns:p='using:FerroUI.Animation' Duration='0:0:2'><KeyFrame Cue='0%'><Setter Property='Width' Value='10'/></KeyFrame></Animation></Style.Animations></Style></Canvas.Styles><p:Button xmlns:p='using:FerroUI.Controls' Name='A' Width='100'/></Canvas>", ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    assert!(
        session.animation_programs()[0].timeline.is_some(),
        "{:?}",
        session.animation_warnings()
    );
    let a = session.document().ast().find_name("A").unwrap();
    session
        .record_keyframe(a, "Width", AnimationValue::Number(20.0))
        .unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let program = &session.animation_programs()[0];
    let node = session.document().ast().node(program.id).unwrap();
    let namespaces = session.document().ast().namespaces(program.id);
    assert_eq!(
        namespaces.get("p").map(String::as_str),
        Some("using:FerroUI.Animation")
    );
    let target_type = node
        .attributes
        .iter()
        .find(|attribute| attribute.name.ends_with(":SetterTargetType"))
        .unwrap();
    let (prefix, local) = target_type.value.split_once(':').unwrap();
    assert_eq!(local, "Button");
    assert_eq!(
        namespaces.get(prefix).map(String::as_str),
        Some("using:FerroUI.Controls")
    );
}

#[test]
fn built_in_geometry_and_spring_samples_match_the_existing_native_animators() {
    use ferroui_base::animation::animators::{
        CornerRadiusAnimator, RectAnimator, RelativePointAnimator, RelativeScalarAnimator,
        SizeAnimator, VectorAnimator,
    };
    use ferroui_base::animation::easings::{IEasing, SpringEasing};
    use ferroui_base::{CornerRadius, Rect, RelativePoint, RelativeScalar, Size, Vector};
    use ferroui_designer_animation::ValueKind;
    for progress in [0.0, 0.25, 0.5, 1.0, 1.25] {
        let cases = [
            (
                ValueKind::Size,
                "10,20",
                "30,60",
                SizeAnimator::interpolate_core(
                    progress,
                    &Size::new(10.0, 20.0),
                    &Size::new(30.0, 60.0),
                )
                .to_string(),
            ),
            (
                ValueKind::Vector,
                "-2,4",
                "6,12",
                VectorAnimator::interpolate_core(
                    progress,
                    &Vector::new(-2.0, 4.0),
                    &Vector::new(6.0, 12.0),
                )
                .to_string(),
            ),
            (
                ValueKind::Rect,
                "1,2,10,20",
                "3,4,30,60",
                RectAnimator::interpolate_core(
                    progress,
                    &Rect::new(1.0, 2.0, 10.0, 20.0),
                    &Rect::new(3.0, 4.0, 30.0, 60.0),
                )
                .to_string(),
            ),
            (
                ValueKind::CornerRadius,
                "1,2",
                "5,6,7,8",
                CornerRadiusAnimator::interpolate_core(
                    progress,
                    &CornerRadius::top_bottom(1.0, 2.0),
                    &CornerRadius::new(5.0, 6.0, 7.0, 8.0),
                )
                .to_string(),
            ),
            (
                ValueKind::RelativePoint,
                "0%,20%",
                "100%,60%",
                RelativePointAnimator::interpolate_core(
                    progress,
                    &RelativePoint::parse("0%,20%").unwrap(),
                    &RelativePoint::parse("100%,60%").unwrap(),
                )
                .to_string(),
            ),
            (
                ValueKind::RelativeScalar,
                "10%",
                "90%",
                RelativeScalarAnimator::interpolate_core(
                    progress,
                    &RelativeScalar::parse("10%").unwrap(),
                    &RelativeScalar::parse("90%").unwrap(),
                )
                .to_string(),
            ),
        ];
        for (kind, from, to, native) in cases {
            let actual = AnimationValue::parse(from, kind)
                .unwrap()
                .interpolate(&AnimationValue::parse(to, kind).unwrap(), progress)
                .unwrap();
            let expected = AnimationValue::parse(&native, kind).unwrap();
            let compare = |left: &serde_json::Value, right: &serde_json::Value| {
                if let (Some(left), Some(right)) = (left.as_f64(), right.as_f64()) {
                    assert!((left - right).abs() < 1e-12, "{kind:?}: {left} != {right}");
                } else {
                    assert_eq!(left, right);
                }
            };
            let actual = serde_json::to_value(actual).unwrap();
            let expected = serde_json::to_value(expected).unwrap();
            for (name, value) in actual["value"].as_object().unwrap() {
                compare(value, &expected["value"][name]);
            }
        }
    }
    for (mass, stiffness, damping, velocity) in [
        (1.0, 100.0, 10.0, 0.0),
        (2.0, 50.0, 20.0, 3.0),
        (1.0, 100.0, 35.0, -2.0),
    ] {
        let native = SpringEasing::with_values(mass, stiffness, damping, velocity);
        let portable = Easing::Spring {
            mass,
            stiffness,
            damping,
            initial_velocity: velocity,
        };
        for progress in [0.0, 0.1, 0.25, 0.5, 0.9, 1.0] {
            assert!((portable.sample(progress).unwrap() - native.ease(progress)).abs() < 1e-12);
        }
    }
}

#[test]
fn geometry_metadata_and_spring_property_elements_drive_live_preview() {
    use ferroui_designer_animation::ValueKind;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new("<Border xmlns='https://github.com/ferroui' Name='A' Width='100' CornerRadius='1,2' RenderTransformOrigin='0%,20%'><Border.Styles><Style Selector='Border'><Style.Animations><Animation Duration='0:0:1'><Animation.Easing><SpringEasing Mass='1' Stiffness='100' Damping='10' InitialVelocity='0'/></Animation.Easing><KeyFrame Cue='0%'><Setter Property='CornerRadius' Value='1,2'/><Setter Property='RenderTransformOrigin' Value='0%,20%'/><Setter Property='Width' Value='100'/></KeyFrame><KeyFrame Cue='100%'><Setter Property='CornerRadius' Value='5,6,7,8'/><Setter Property='RenderTransformOrigin' Value='100%,60%'/><Setter Property='Width' Value='200'/></KeyFrame></Animation></Style.Animations></Style></Border.Styles></Border>",ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    assert!(
        session.animation_warnings().is_empty(),
        "{:?}",
        session.animation_warnings()
    );
    let target = session.document().ast().root();
    assert_eq!(
        session
            .timeline()
            .track(target, "CornerRadius")
            .unwrap()
            .keyframes[0]
            .value
            .kind(),
        ValueKind::CornerRadius
    );
    assert_eq!(
        session
            .timeline()
            .track(target, "RenderTransformOrigin")
            .unwrap()
            .keyframes[0]
            .value
            .kind(),
        ValueKind::RelativePoint
    );
    session.seek(0.5).unwrap();
    let control = session
        .preview
        .visual(target)
        .unwrap()
        .cast::<ferroui_controls::Border>()
        .unwrap();
    assert_eq!(
        control.corner_radius(),
        ferroui_base::CornerRadius::new(3.0, 3.5, 4.5, 5.0)
    );
    assert_eq!(
        control.render_transform_origin(),
        ferroui_base::RelativePoint::parse("50%,40%").unwrap()
    );
    let id = session.active_animation_program().unwrap();
    session.preview_animation_programs(&[id], 0.5).unwrap();
    let progress = session.timeline().easing.sample(0.5).unwrap();
    assert!((control.width() - (100.0 + 100.0 * progress)).abs() < 1e-9);
    session.set_duration(2.0).unwrap();
    assert!(matches!(session.timeline().easing, Easing::Spring { .. }));
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
}

#[test]
fn complex_native_programs_seek_shadows_gradients_effects_and_transform_components() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Border xmlns="https://github.com/ferroui" Name="A" Width="100" Height="60" Background="White">
<Border.RenderTransform><RotateTransform Angle="10"/></Border.RenderTransform>
<Border.Styles><Style Selector="Border#A"><Style.Animations><Animation Duration="0:0:2" FillMode="Forward">
<KeyFrame Cue="0%"><Setter Property="BoxShadow" Value="0 0 0 0 Red"/><Setter Property="RotateTransform.Angle" Value="0"/>
<Setter Property="Background"><LinearGradientBrush StartPoint="0%,0%" EndPoint="0%,100%"><GradientStop Offset="0" Color="Red"/><GradientStop Offset="1" Color="Blue"/></LinearGradientBrush></Setter>
<Setter Property="Effect"><BlurEffect Radius="0"/></Setter></KeyFrame>
<KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="20 30 10 4 Blue"/><Setter Property="RotateTransform.Angle" Value="90"/>
<Setter Property="Background"><LinearGradientBrush StartPoint="100%,0%" EndPoint="100%,100%"><GradientStop Offset="0" Color="Blue"/><GradientStop Offset="1" Color="Red"/></LinearGradientBrush></Setter>
<Setter Property="Effect"><BlurEffect Radius="10"/></Setter></KeyFrame>
</Animation></Style.Animations></Style></Border.Styles></Border>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let id = session.animation_programs()[0].id;
    assert!(session.animation_programs()[0].timeline.is_none());
    let target = session.document().ast().root();
    session.seek(1.0).unwrap();
    let border = session
        .preview
        .visual(target)
        .unwrap()
        .cast::<ferroui_controls::Border>()
        .unwrap();
    let shadow = border.box_shadow().get(0);
    assert!((shadow.offset_x - 10.0).abs() < 1e-6);
    assert!((shadow.blur - 5.0).abs() < 1e-6);
    assert!((border.effect().unwrap().as_blur_effect().unwrap().radius() - 5.0).abs() < 1e-6);
    assert_eq!(
        border
            .background()
            .unwrap()
            .as_linear_gradient_brush()
            .unwrap()
            .start_point(),
        ferroui_base::RelativePoint::parse("50%,0%").unwrap()
    );
    let angle = border
        .render_transform()
        .unwrap()
        .as_object()
        .unwrap()
        .downcast_ref::<ferroui_base::media::RotateTransform>()
        .unwrap()
        .angle();
    assert!((angle - 45.0).abs() < 1e-6);
    assert_eq!(session.document().source(), source);
    session.seek(0.5).unwrap();
    assert!((border.box_shadow().get(0).offset_x - 5.0).abs() < 1e-6);
    session.play();
    session.advance(0.5).unwrap();
    assert!((border.box_shadow().get(0).offset_x - 10.0).abs() < 1e-6);
    session.pause();
    session.advance(0.5).unwrap();
    assert!((border.box_shadow().get(0).offset_x - 10.0).abs() < 1e-6);
    session.select_animation_program(id).unwrap();
    assert_eq!(border.box_shadow().count(), 0);
    assert!(border.effect().is_none());
    assert_eq!(
        border
            .render_transform()
            .unwrap()
            .as_object()
            .unwrap()
            .downcast_ref::<ferroui_base::media::RotateTransform>()
            .unwrap()
            .angle(),
        10.0
    );
    session.seek(1.0).unwrap();
    session.stop();
    assert_eq!(border.box_shadow().count(), 0);
    assert_eq!(
        border
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        ferroui_base::media::Color::parse("White").unwrap()
    );
    session.seek(1.0).unwrap();
    session.update_source(
        source
            .replace("<Border.RenderTransform>", "<Border.BadProperty>")
            .replace("</Border.RenderTransform>", "</Border.BadProperty>"),
    );
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Failed { .. }
    ));
    assert_eq!(border.box_shadow().count(), 0);
    assert!(
        session.seek(1.0).is_err(),
        "Never sample a retired compiled source graph"
    );
}

#[test]
fn native_resource_animation_uses_its_source_identity_and_explicit_targets() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Canvas.Resources>
<Animation x:Key="Shadow" x:SetterTargetType="Border" Duration="0:0:2" FillMode="Forward"><KeyFrame Cue="0%"><Setter Property="BoxShadow" Value="0 0 0 0 Red"/></KeyFrame><KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="20 30 10 4 Blue"/></KeyFrame></Animation>
</Canvas.Resources><Border Name="A" Width="100" Height="60"/><Border Name="B"/></Canvas>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let id = session.animation_programs()[0].id;
    let a = session.document().ast().find_name("A").unwrap();
    let b = session.document().ast().find_name("B").unwrap();
    session
        .set_animation_program_preview_targets(id, Some(vec![a]))
        .unwrap();
    assert!(session.animation_programs()[0].timeline.is_none());
    session.preview_animation_programs(&[id], 1.0).unwrap();
    let a = session
        .preview
        .visual(a)
        .unwrap()
        .cast::<ferroui_controls::Border>()
        .unwrap();
    let b = session
        .preview
        .visual(b)
        .unwrap()
        .cast::<ferroui_controls::Border>()
        .unwrap();
    assert!((a.box_shadow().get(0).offset_x - 10.0).abs() < 1e-6);
    assert_eq!(b.box_shadow().count(), 0);
    assert_eq!(session.document().source(), source);
    session.stop();
    assert_eq!(a.box_shadow().count(), 0);
}

#[test]
fn native_resource_program_keeps_provenance_when_factory_root_is_created_in_rust() {
    use ferroui_markup_xaml::FromXamlObject;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Border xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Border.Resources>
<Animation x:Key="Shadow" x:SetterTargetType="Border" Duration="0:0:2" FillMode="Forward"><KeyFrame Cue="0%"><Setter Property="BoxShadow" Value="0 0 0 0 Red"/></KeyFrame><KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="20 30 10 4 Blue"/></KeyFrame></Animation>
</Border.Resources></Border>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let mut options = crate::PreviewHostOptions::default();
    options.register_root(
        "Border",
        std::rc::Rc::new(|request| {
            let loaded = ferroui_base::Ref::<ferroui_controls::Border>::from_xaml_object(
                &request.populate(None)?,
            )
            .ok_or("Expected a loaded Border")?;
            let root = ferroui_controls::Border::new();
            let resources = loaded.resources();
            loaded.set_resources(ferroui_base::controls::ResourceDictionary::new());
            root.resources().merged_dictionaries().add(resources.into());
            Ok(std::rc::Rc::new(root) as ferroui_base::BoxedValue)
        }),
    );
    session.configure_preview(options).unwrap();
    let id = session.animation_programs()[0].id;
    let root = session.document().ast().root();
    session
        .set_animation_program_preview_targets(id, Some(vec![root]))
        .unwrap();
    session.preview_animation_programs(&[id], 1.0).unwrap();
    let border = session
        .preview
        .visual(root)
        .unwrap()
        .cast::<ferroui_controls::Border>()
        .unwrap();
    assert!((border.box_shadow().get(0).offset_x - 10.0).abs() < 1e-6);
    session.preview_animation_programs(&[id], 2.0).unwrap();
    assert_eq!(border.box_shadow().get(0).offset_x, 20.0);
    session.stop();
    assert_eq!(border.box_shadow().count(), 0);
    session.preview_animation_programs(&[id], 1.0).unwrap();
    let mut rejected = crate::PreviewHostOptions::default();
    rejected.register_root(
        "Border",
        std::rc::Rc::new(|_| Err("Rejected preview factory".into())),
    );
    assert!(session.configure_preview(rejected).is_err());
    assert_eq!(
        border.box_shadow().count(),
        0,
        "Failed factory changes dispose native preview handles"
    );
    assert_eq!(session.document().source(), source);
}

#[test]
fn standalone_resource_animation_targets_factory_root_without_authored_control_identity() {
    use ferroui_markup_xaml::FromXamlObject;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
<Animation x:Key="Shadow" x:SetterTargetType="Border" Duration="0:0:2" FillMode="Forward"><KeyFrame Cue="0%"><Setter Property="BoxShadow" Value="0 0 0 0 Red"/></KeyFrame><KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="20 30 10 4 Blue"/></KeyFrame></Animation>
</ResourceDictionary>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let mut options = crate::PreviewHostOptions::default();
    options.resource_factory = Some(std::rc::Rc::new(|loaded| {
        let dictionary =
            ferroui_base::Ref::<ferroui_base::controls::ResourceDictionary>::from_xaml_object(
                &loaded,
            )
            .ok_or("Expected resource dictionary")?;
        let root = ferroui_controls::Border::new();
        root.set_resources(dictionary);
        Ok(root.upcast())
    }));
    session.configure_preview(options).unwrap();
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Ready { .. }
    ));
    assert!(session
        .preview
        .visual(session.document().ast().root())
        .is_none());
    assert!(session.animation_programs()[0].targets.is_empty());
    session.seek(1.0).unwrap();
    let border = session
        .preview
        .root()
        .unwrap()
        .cast::<ferroui_controls::Border>()
        .unwrap();
    assert!((border.box_shadow().get(0).offset_x - 10.0).abs() < 1e-6);
    session.stop();
    assert_eq!(border.box_shadow().count(), 0);
    assert_eq!(session.document().source(), source);
    let mut incompatible = crate::PreviewHostOptions::default();
    incompatible.resource_factory = Some(std::rc::Rc::new(|loaded| {
        let dictionary =
            ferroui_base::Ref::<ferroui_base::controls::ResourceDictionary>::from_xaml_object(
                &loaded,
            )
            .ok_or("Expected resource dictionary")?;
        let root = ferroui_controls::Canvas::new();
        root.set_resources(dictionary);
        Ok(root.upcast())
    }));
    session.configure_preview(incompatible).unwrap();
    assert!(session
        .seek(1.0)
        .unwrap_err()
        .contains("incompatible with factory preview root"));
}

#[test]
fn native_component_preview_reports_unsupported_css_transform_target() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Border xmlns="https://github.com/ferroui" Name="A" RenderTransform="rotate(10deg)"><Border.Styles><Style Selector="Border#A"><Style.Animations><Animation Duration="0:0:2"><KeyFrame Cue="0%"><Setter Property="RotateTransform.Angle" Value="0"/></KeyFrame><KeyFrame Cue="100%"><Setter Property="RotateTransform.Angle" Value="90"/></KeyFrame></Animation></Style.Animations></Style></Border.Styles></Border>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Ready { .. }
    ));
    assert!(session
        .seek(1.0)
        .unwrap_err()
        .contains("CSS TransformOperations"));
    assert_eq!(session.document().source(), source);
}

#[test]
fn native_play_all_skips_deferred_programs_without_live_targets() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Border xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Name="A">
<Border.Resources><DataTemplate x:Key="Unused"><Border><Border.Styles><Style Selector="Border"><Style.Animations><Animation Duration="0:0:2"><KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="40 30 10 4 Blue"/></KeyFrame></Animation></Style.Animations></Style></Border.Styles></Border></DataTemplate></Border.Resources>
<Border.Styles><Style Selector="Border#A"><Style.Animations><Animation Duration="0:0:2" FillMode="Forward"><KeyFrame Cue="0%"><Setter Property="BoxShadow" Value="0 0 0 0 Red"/></KeyFrame><KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="20 30 10 4 Blue"/></KeyFrame></Animation></Style.Animations></Style></Border.Styles></Border>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    assert_eq!(session.animation_programs().len(), 2);
    assert!(session
        .animation_programs()
        .iter()
        .any(|program| program.targets.is_empty()));
    session.play_all_animation_programs();
    session.advance(1.0).unwrap();
    let border = session
        .preview
        .root()
        .unwrap()
        .cast::<ferroui_controls::Border>()
        .unwrap();
    assert!((border.box_shadow().get(0).offset_x - 10.0).abs() < 1e-6);
    session.stop();
    assert_eq!(border.box_shadow().count(), 0);
}
