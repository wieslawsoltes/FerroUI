use crate::project_rust;
use ferroui_designer_animation::{AnimationPrograms, Easing, VisualStateGroup};
use ferroui_designer_core::{Document, TextPatch};

#[test]
fn visual_states_project_native_routes_and_retain_original_rust_sources() {
    let source = r#"
        let other = rsx! { TextBlock { text: "Résumé" } };
        let view = rsx! { StackPanel {
            visual_states: [
                procedural_states(),
                ferroui_view::visual_state_group("Review")
                    .state(ferroui_view::visual_state("Idle")
                        .set_on("Card", ferroui_base::layout::Layoutable::width_property(), 120.0))
                    .state(ferroui_view::visual_state("Ready")
                        .set_on("Card", ferroui_base::layout::Layoutable::width_property(), 260.0))
                    .transition(None, "Ready", ferroui_view::ms(350), "LinearEasing")
                    .transition(Some("Idle"), "Ready", ferroui_view::ms(/* keep */ 640u64), "CubicEaseIn")
                    .transition(Some("Ready"), "Idle", ferroui_view::ms(370), "LinearEasing"),
            ],
            Border { name: "Card", width: 100.0 }
        }};
    "#.replace('\n', "\r\n");
    let offset = source.find("visual_states:").unwrap();
    let projection = project_rust(&source, offset).unwrap();
    assert!(projection.has_projected_states());
    assert!(!projection.can_replace_structure());
    assert_eq!(projection.animation_issues.len(), 1);
    let (xaml, map) = projection.mapped_markup();
    assert_eq!(xaml, project_rust(&source, offset).unwrap().xaml());
    assert!(!xaml.contains("ConditionalAction"));
    let document = Document::new(xaml).unwrap();
    assert!(!AnimationPrograms::scan(&document).is_empty());
    assert!(AnimationPrograms::scan_hosted(&document).is_empty());
    let [state] = VisualStateGroup::read(&document).try_into().unwrap();
    let map = map
        .state_group(
            document.ast().node(state.owner).unwrap().span.start,
            "Review",
        )
        .unwrap();
    assert_eq!(
        state.transition(Some("Idle"), "Ready").unwrap().duration,
        0.64
    );
    assert_eq!(
        state.transition(Some("Ready"), "Idle").unwrap().duration,
        0.37
    );
    assert_eq!(state.transition(None, "Ready").unwrap().duration, 0.35);
    let route = map.transition_span(Some("Idle"), "Ready").unwrap();
    assert!(source[route.start..route.end].starts_with("transition(Some(\"Idle\")"));
    let mut edit = state;
    let route = edit
        .transitions
        .iter_mut()
        .find(|route| route.from.as_deref() == Some("Idle"))
        .unwrap();
    route.duration = 0.825;
    route.easing = Easing::Named("QuadraticEaseOut".into());
    edit.states[1].setters[0].value = "280".into();
    let edited = TextPatch::apply_all(&source, map.patch_model(&edit).unwrap()).unwrap();
    assert_eq!(
        edited,
        source
            .replace("260.0", "280.0")
            .replace("640u64", "825u64")
            .replace("CubicEaseIn", "QuadraticEaseOut")
    );
    assert!(project_rust(&edited, offset)
        .unwrap()
        .has_projected_states());
    edit.transitions[0].duration = 0.333_333;
    assert!(map.patch_model(&edit).is_err());
    edit.transitions.pop();
    assert!(map.patch_model(&edit).is_err());
}

#[test]
fn an_instant_rust_route_retains_its_authored_duration() {
    let source = r#"rsx! { Border {
        visual_states: [ferroui_view::visual_state_group("Review")
            .state(ferroui_view::visual_state("Ready").set(ferroui_base::Visual::opacity_property(), 0.5))
            .transition(None, "Ready", ferroui_view::ms(0), "LinearEasing")]
    } }"#;
    let projection = project_rust(source, 0).unwrap();
    let markup = projection.preview_markup();
    let document = Document::new(markup.source).unwrap();
    let [state] = VisualStateGroup::read(&document).try_into().unwrap();
    assert_eq!(state.transitions[0].duration, 0.0);
    assert!(markup.state_groups[0]
        .1
        .patch_model(&state)
        .unwrap()
        .is_empty());
}

#[test]
fn state_structure_targets_one_invocation_and_preserves_retired_method_comments() {
    use ferroui_designer_animation::{StateSetter, StateTransition, VisualState};

    let source = r#"fn report() {
    let title = rsx! { TextBlock { text: "Résumé 🪴" } };
    let card = rsx! { Border {
        child: make_content(), // unchanged Rust expression
    } };
}
"#
    .replace('\n', "\r\n");
    let caret = source.find("child:").unwrap();
    let mut group = VisualStateGroup {
        owner: 1,
        name: "Review".into(),
        states: vec![VisualState {
            name: "Idle".into(),
            setters: vec![],
        }],
        transitions: Vec::new(),
    };
    let original = project_rust(&source, caret).unwrap();
    assert!(original.can_create_state_group(0));
    let mut added =
        TextPatch::apply_all(&source, original.upsert_state_group(0, &group).unwrap()).unwrap();
    added = added.replace(
        ".state(::ferroui_view::visual_state(",
        ".state(/* preserve 🪴 */ ::ferroui_view::visual_state(",
    );
    let caret = added.find("child:").unwrap();
    let projection = project_rust(&added, caret).unwrap();

    let new_state = VisualState {
        name: "Ready".into(),
        setters: vec![StateSetter {
            target: "".into(),
            property: "Opacity".into(),
            value: "0.42".into(),
        }],
    };
    let route = StateTransition {
        from: Some("Idle".into()),
        to: "Ready".into(),
        duration: 0.23,
        easing: Easing::Linear,
    };
    group.states.push(new_state);
    group.transitions.push(route);
    let edited =
        TextPatch::apply_all(&added, projection.upsert_state_group(0, &group).unwrap()).unwrap();
    assert!(edited.contains(".set(::ferroui_base::Visual::opacity_property(), 0.42)"));
    assert!(edited.contains(
        ".transition(Some(\"Idle\"), \"Ready\", ::ferroui_view::ms(230), \"LinearEasing\")"
    ));
    assert_eq!(edited.lines().nth(1), source.lines().nth(1));
    assert!(edited.contains("child: make_content(), // unchanged Rust expression\r\n"));
    assert!(!edited.replace("\r\n", "").contains('\n'));

    let caret = edited.find("child:").unwrap();
    let projection = project_rust(&edited, caret).unwrap();
    group.transitions[0].duration = 0.2301;
    assert!(projection.upsert_state_group(0, &group).is_err());
    group.transitions.clear();
    group.states.remove(0);
    let edited =
        TextPatch::apply_all(&edited, projection.upsert_state_group(0, &group).unwrap()).unwrap();
    assert!(edited.contains("/* preserve 🪴 */"));
    assert!(!edited.contains("visual_state(\"Idle\")"));
    assert!(!edited.contains(".transition("));
    let projection = project_rust(&edited, edited.find("child:").unwrap()).unwrap();
    let removed =
        TextPatch::apply_all(&edited, projection.remove_state_group(0, "Review").unwrap()).unwrap();
    let projection = project_rust(&removed, removed.find("child:").unwrap()).unwrap();
    assert!(!projection.has_projected_states());
    assert!(projection.can_create_state_group(0));
    assert!(removed.contains("/* preserve 🪴 */"));
    assert!(removed.contains("child: make_content(), // unchanged Rust expression\r\n"));
}

#[test]
fn mapped_group_removal_keeps_an_opaque_array_member_and_its_comment() {
    let source = r#"rsx! { Border {
        visual_states: [
            procedural_group() /* owned by app */,
            ::ferroui_view::visual_state_group("Review")
                .state(::ferroui_view::visual_state("Normal"))
        ],
    } }"#;
    let project = project_rust(source, 0).unwrap();
    assert!(!project.can_create_state_group(0));
    let mut group = VisualStateGroup::read(&Document::new(project.xaml()).unwrap()).remove(0);
    group.name = "Second".into();
    assert!(project.upsert_state_group(0, &group).is_err());
    let edited =
        TextPatch::apply_all(source, project.remove_state_group(0, "Review").unwrap()).unwrap();
    assert!(edited.contains("procedural_group() /* owned by app */\n"));
    let result = project_rust(&edited, 0).unwrap();
    assert!(!result.has_projected_states());
    assert_eq!(result.animation_issues.len(), 1);
}
#[test]
fn a_routed_state_trigger_is_a_checked_rust_edit_and_keeps_existing_handlers() {
    use ferroui_designer_core::TextPatch;

    let source = r#"
fn unrelated() { rsx! { TextBlock { text: "Other" } }; }
fn review() {
    let handler = move |_| update_review();
    rsx! {
        StackPanel {
            visual_states: [ferroui_view::visual_state_group("Review")
                .state(ferroui_view::visual_state("Ready"))
                .state(ferroui_view::visual_state("Queued"))],
            Button {
                // Retain this action 🚀
                on_click: handler,
                "Approve"
            }
        }
    };
}
"#.replace('\n', "\r\n");
    let caret = source.rfind("rsx!").unwrap();
    let projection = crate::project_rust(&source, caret).unwrap();
    let report = projection.state_trigger_report(1).unwrap();
    assert!(report.events.iter().any(|name| name == "Click"));
    assert!(!report.events.iter().any(|name| name == "SelectionChanged"));
    assert_eq!(report.groups[0].group, "Review");
    assert!(projection.plan_state_trigger(1, "Click", "Review", Some("Missing")).is_err());
    assert!(projection.plan_state_trigger(1, "SelectionChanged", "Review", Some("Ready")).is_err());

    let patches = projection.plan_state_trigger(1, "Click", "Review", Some("Ready")).unwrap();
    let draft = TextPatch::apply_all(&source, patches).unwrap();
    assert!(draft.contains("state_trigger(\"Click\", \"Review\", \"Ready\"),\r\n"));
    assert!(draft.contains("// Retain this action 🚀\r\n                on_click: handler,"));
    assert_eq!(&draft[..caret], &source[..caret]);
    let projection = crate::project_rust(&draft, caret).unwrap();
    let markup = projection.preview_markup();
    let projected = Document::new(markup.source).unwrap();
    let triggers = ferroui_designer_animation::ProjectedStateTrigger::scan(&projected);
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].event, "Click");
    assert_eq!(projected.ast().node(triggers[0].source).unwrap().local_name(), "Button");
    assert_eq!(markup.controls.len(), 2);
    assert_eq!(projection.state_trigger_report(1).unwrap().triggers[0].state, "Ready");

    let drafts = projection.plan_state_trigger(1, "Click", "Review", Some("Queued")).unwrap();
    let edited = TextPatch::apply_all(&draft, drafts).unwrap();
    let projection = crate::project_rust(&edited, caret).unwrap();
    let remove = projection.plan_state_trigger(1, "Click", "Review", None).unwrap();
    assert_eq!(TextPatch::apply_all(&edited, remove).unwrap(), source);
    let remove = projection.remove_state_group(0, "Review").unwrap();
    assert!(!TextPatch::apply_all(&edited, remove).unwrap().contains("state_trigger("));
}
