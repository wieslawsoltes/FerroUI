use super::*;
use crate::{
    PreviewEnvironment, PreviewPrototypeAction, PreviewPrototypeFlow, PreviewPrototypeLink,
    PreviewPrototypeSession, PreviewSourceAnchor, PreviewSourceSegment, PreviewStageBoard,
    PreviewStageConfiguration, PreviewStageSet,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn interaction(
    source: PreviewStageId,
    name: &str,
    action: PreviewPrototypeAction,
) -> PreviewPrototypeLink {
    PreviewPrototypeLink {
        name: name.into(),
        source,
        trigger: PreviewSourceAnchor {
            path: vec![PreviewSourceSegment {
                namespace: "https://github.com/ferroui".into(),
                type_name: "Button".into(),
                name: Some(name.into()),
                key: None,
            }],
        },
        action,
    }
}

fn transition(link: usize, effect: PreviewPrototypeEffect) -> PreviewPrototypeLinkTransition {
    PreviewPrototypeLinkTransition {
        link,
        transition: PreviewPrototypeTransition {
            effect,
            duration_ms: 300,
            easing: PreviewPrototypeTransitionEasing::EaseIn,
        },
    }
}

#[test]
fn prototype_transition_stage_set_round_trip_and_rebase_are_atomic() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut board = PreviewStageBoard::new();
    let stages = ["Home", "Details", "Settings"].map(|name| {
        board
            .add(PreviewStageConfiguration::new(
                name,
                "memory://flow.xaml",
                PreviewEnvironment::default(),
            ))
            .unwrap()
    });
    let [home, details, settings] = stages;
    let mut graph = PreviewPrototype {
        flows: vec![PreviewPrototypeFlow {
            name: "Review".into(),
            start: home,
        }],
        links: vec![
            interaction(
                home,
                "Details",
                PreviewPrototypeAction::Navigate { stage: details },
            ),
            interaction(
                home,
                "Settings",
                PreviewPrototypeAction::Navigate { stage: settings },
            ),
        ],
    };
    let motion = transition(1, PreviewPrototypeEffect::SlideLeft);
    board
        .set_prototype_with_transitions(graph.clone(), vec![motion])
        .unwrap();
    let saved = board.stage_set();
    assert_eq!(saved.version, 4);
    assert_eq!(saved.prototype_transitions, vec![motion]);
    let mut restored = PreviewStageBoard::new();
    restored
        .restore_stage_set(
            PreviewStageSet::from_json_with_fixtures(&saved.to_json().unwrap()).unwrap(),
        )
        .unwrap();
    assert_eq!(restored.stage_set(), saved);
    assert_ne!(restored.prototype().flows[0].start, home);

    let epoch = board.epoch();
    assert!(board
        .set_prototype_with_transitions(graph.clone(), vec![motion, motion])
        .is_err());
    let mut invalid = motion;
    invalid.transition.duration_ms = 0;
    assert!(board
        .set_prototype_with_transitions(graph.clone(), vec![invalid])
        .is_err());
    let mut legacy = saved.clone();
    legacy.version = 2;
    assert!(legacy.validate().is_err());
    assert_eq!(board.epoch(), epoch);
    assert_eq!(board.stage_set(), saved);

    graph.links.swap(0, 1);
    graph.links[0].name = "Edit settings".into();
    board.set_prototype(graph).unwrap();
    assert_eq!(
        board.prototype_transitions(),
        &[PreviewPrototypeLinkTransition { link: 0, ..motion }]
    );
    board.remove(settings).unwrap();
    assert!(board.prototype_transitions().is_empty());
    assert_eq!(board.stage_set().version, 2);
}

#[test]
fn prototype_motion_uses_the_current_history_entry_and_explicit_close() {
    let graph = PreviewPrototype {
        flows: vec![PreviewPrototypeFlow {
            name: "Review".into(),
            start: 1,
        }],
        links: vec![
            interaction(1, "Next", PreviewPrototypeAction::Navigate { stage: 2 }),
            interaction(
                2,
                "Open",
                PreviewPrototypeAction::OpenOverlay {
                    stage: 3,
                    options: Default::default(),
                },
            ),
            interaction(
                3,
                "Nested",
                PreviewPrototypeAction::OpenOverlay {
                    stage: 4,
                    options: Default::default(),
                },
            ),
            interaction(4, "Close", PreviewPrototypeAction::CloseOverlay),
        ],
    };
    let assignments = [
        transition(0, PreviewPrototypeEffect::SlideRight),
        transition(1, PreviewPrototypeEffect::SlideUp),
        transition(3, PreviewPrototypeEffect::Dissolve),
    ];
    let mut player =
        PreviewPrototypeSession::start_with_transitions(&graph, "Review", &assignments).unwrap();
    player.activate_link(0).unwrap();
    assert_eq!(player.motion().unwrap().surface, MotionSurface::ScreenIn);
    player.activate_link(1).unwrap();
    assert_eq!(player.motion().unwrap().surface, MotionSurface::OverlayIn);
    player.activate_link(2).unwrap();
    assert!(player.motion().is_none());
    player.activate_link(3).unwrap();
    assert_eq!(
        player.motion().unwrap().transition,
        assignments[2].transition
    );
    assert!(!player.motion().unwrap().reverse);
    player.close_overlay().unwrap();
    let close = player.motion().unwrap();
    assert_eq!(close.surface, MotionSurface::OverlayOut);
    assert!(close.reverse);
    assert_eq!(close.transition, assignments[1].transition);
    assert!(close.transition.progress(Duration::from_millis(150), true) > 0.5);
    player.back().unwrap();
    let back = player.motion().unwrap();
    assert_eq!(back.surface, MotionSurface::ScreenOut);
    assert_eq!(back.transition, assignments[0].transition);
    assert!(back.reverse);
    player.restart();
    assert!(player.motion().is_none());
    assert!(player.back().is_err());
}
