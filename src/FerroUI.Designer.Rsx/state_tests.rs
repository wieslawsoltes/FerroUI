use crate::project_rust;
use ferroui_designer_animation::{Easing, VisualStateGroup};
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
    let document = Document::new(xaml).unwrap();
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
