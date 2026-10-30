use super::*;
use crate::{
    PreviewEnvironment, PreviewSourceSegment, PreviewStageBoard, PreviewStageConfiguration,
    PreviewStageManifest,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn link(
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

fn graph(start: PreviewStageId, links: Vec<PreviewPrototypeLink>) -> PreviewPrototype {
    PreviewPrototype {
        flows: vec![PreviewPrototypeFlow {
            name: "Review".into(),
            start,
        }],
        links,
    }
}

fn open(stage: PreviewStageId) -> PreviewPrototypeAction {
    PreviewPrototypeAction::OpenOverlay {
        stage,
        options: PreviewPrototypeOverlayOptions::default(),
    }
}

fn swap(stage: PreviewStageId) -> PreviewPrototypeAction {
    PreviewPrototypeAction::SwapOverlay {
        stage,
        options: PreviewPrototypeOverlayOptions::default(),
    }
}

#[test]
fn overlay_descriptors_validate_bounded_options_and_strict_wire_shapes() {
    let options = PreviewPrototypeOverlayOptions {
        placement: PreviewPrototypeOverlayPlacement::Manual {
            x: -16_384,
            y: 16_384,
        },
        dismiss_on_outside: true,
        dim_background: false,
    };
    let valid = graph(
        10,
        vec![
            link(
                10,
                "Open",
                PreviewPrototypeAction::OpenOverlay { stage: 20, options },
            ),
            link(20, "Swap", swap(30)),
            link(30, "Close", PreviewPrototypeAction::CloseOverlay),
        ],
    );
    valid.validate(&[10, 20, 30]).unwrap();
    assert!(valid.has_overlay_actions());
    let json = serde_json::to_string(&valid).unwrap();
    assert_eq!(
        serde_json::from_str::<PreviewPrototype>(&json).unwrap(),
        valid
    );
    for (x, y) in [(16_385, 0), (0, -16_385), (i32::MIN, i32::MAX)] {
        let mut invalid = valid.clone();
        invalid.links[0].action = PreviewPrototypeAction::OpenOverlay {
            stage: 20,
            options: PreviewPrototypeOverlayOptions {
                placement: PreviewPrototypeOverlayPlacement::Manual { x, y },
                ..options
            },
        };
        assert!(invalid
            .validate(&[10, 20, 30])
            .unwrap_err()
            .contains("coordinates"));
        assert!(PreviewPrototypeSession::start(&invalid, "Review").is_err());
    }
    for action in [open(99), swap(99)] {
        let mut invalid = valid.clone();
        invalid.links[0].action = action;
        assert!(invalid
            .validate(&[10, 20, 30])
            .unwrap_err()
            .contains("destination"));
    }
    for placement in [
        PreviewPrototypeOverlayPlacement::Center,
        PreviewPrototypeOverlayPlacement::TopLeft,
        PreviewPrototypeOverlayPlacement::TopCenter,
        PreviewPrototypeOverlayPlacement::TopRight,
        PreviewPrototypeOverlayPlacement::CenterLeft,
        PreviewPrototypeOverlayPlacement::CenterRight,
        PreviewPrototypeOverlayPlacement::BottomLeft,
        PreviewPrototypeOverlayPlacement::BottomCenter,
        PreviewPrototypeOverlayPlacement::BottomRight,
    ] {
        let json = serde_json::to_string(&placement).unwrap();
        assert_eq!(
            serde_json::from_str::<PreviewPrototypeOverlayPlacement>(&json).unwrap(),
            placement
        );
        let mut invalid = serde_json::to_value(placement).unwrap();
        invalid["x"] = serde_json::json!(0);
        assert!(
            serde_json::from_value::<PreviewPrototypeOverlayPlacement>(invalid).is_err(),
            "Preset placement {placement:?} cannot silently accept manual coordinates"
        );
    }
    for kind in ["open_overlay", "swap_overlay"] {
        let value = serde_json::json!({"kind":kind,"stage":20});
        let action = serde_json::from_value::<PreviewPrototypeAction>(value).unwrap();
        let options = match action {
            PreviewPrototypeAction::OpenOverlay { options, .. }
            | PreviewPrototypeAction::SwapOverlay { options, .. } => options,
            _ => unreachable!(),
        };
        assert_eq!(options, PreviewPrototypeOverlayOptions::default());
        assert!(!options.dismiss_on_outside);
        assert!(options.dim_background);
    }
    for value in [
        serde_json::json!({"kind":"center","x":0}),
        serde_json::json!({"kind":"manual","x":0}),
        serde_json::json!({"kind":"manual","x":0.5,"y":0}),
    ] {
        assert!(serde_json::from_value::<PreviewPrototypeOverlayPlacement>(value).is_err());
    }
    for value in [
        serde_json::json!({"kind":"back","stage":20}),
        serde_json::json!({"kind":"close_overlay","stage":20}),
        serde_json::json!({"kind":"close_overlay","options":{}}),
        serde_json::json!({"kind":"open_overlay","stage":20,"options":{"unknown":true}}),
        serde_json::json!({"kind":"swap_overlay","stage":20,"options":{"dim_background":1}}),
    ] {
        assert!(serde_json::from_value::<PreviewPrototypeAction>(value).is_err());
    }
}

#[test]
fn fieldless_actions_keep_their_wire_format_and_reject_all_additional_fields() {
    for (kind, live, persisted) in [
        (
            "back",
            PreviewPrototypeAction::Back,
            PreviewPrototypeActionManifest::Back,
        ),
        (
            "close_overlay",
            PreviewPrototypeAction::CloseOverlay,
            PreviewPrototypeActionManifest::CloseOverlay,
        ),
    ] {
        let expected = serde_json::json!({"kind":kind});
        assert_eq!(serde_json::to_value(live).unwrap(), expected);
        assert_eq!(serde_json::to_value(persisted).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<PreviewPrototypeAction>(expected.clone()).unwrap(),
            live
        );
        assert_eq!(
            serde_json::from_value::<PreviewPrototypeActionManifest>(expected.clone()).unwrap(),
            persisted
        );
        for (field, value) in [
            ("stage", serde_json::json!(0)),
            ("options", serde_json::json!({})),
            ("unknown", serde_json::json!(true)),
        ] {
            let mut invalid = expected.clone();
            invalid[field] = value;
            assert!(
                serde_json::from_value::<PreviewPrototypeAction>(invalid.clone()).is_err(),
                "Live {kind} must reject {field}"
            );
            assert!(
                serde_json::from_value::<PreviewPrototypeActionManifest>(invalid).is_err(),
                "Persistent {kind} must reject {field}"
            );
        }
    }
}

#[test]
fn manifest_json_rejects_mixed_actions_and_preset_fields_before_restore() {
    let graph = graph(
        10,
        vec![
            link(10, "Open", open(20)),
            link(20, "Close", PreviewPrototypeAction::CloseOverlay),
        ],
    );
    let manifest = PreviewStageManifest {
        version: 3,
        stages: ["Home", "Dialog"]
            .into_iter()
            .map(|name| {
                PreviewStageConfiguration::new(
                    name,
                    "memory://prototype.xaml",
                    PreviewEnvironment::default(),
                )
            })
            .collect(),
        active: Some(0),
        prototype: PreviewPrototypeManifest::from_prototype(&graph, &[10, 20]).unwrap(),
    };
    let source = manifest.to_json().unwrap();
    assert_eq!(PreviewStageManifest::from_json(&source).unwrap(), manifest);
    let mut value: serde_json::Value = serde_json::from_str(&source).unwrap();
    for action in [
        serde_json::json!({"kind":"back","stage":1}),
        serde_json::json!({"kind":"close_overlay","options":{}}),
        serde_json::json!({"kind":"navigate","stage":1,"options":{}}),
        serde_json::json!({"kind":"open_overlay","stage":1,"unknown":true}),
        serde_json::json!({"kind":"open_overlay","stage":1,"options":{"placement":{"kind":"center","x":0}}}),
        serde_json::json!({"kind":"swap_overlay","stage":1,"options":{"placement":{"kind":"top_left","y":0}}}),
        serde_json::json!({"kind":"swap_overlay","stage":1,"options":{"placement":{"kind":"manual","x":0,"y":0,"unknown":true}}}),
    ] {
        value["prototype"]["links"][0]["action"] = action.clone();
        assert!(
            serde_json::from_value::<PreviewPrototypeActionManifest>(action.clone()).is_err(),
            "Persistent action accepted mixed fields: {action}"
        );
        assert!(
            PreviewStageManifest::from_json(&value.to_string()).is_err(),
            "Complete manifest accepted mixed fields: {action}"
        );
    }

    for kind in ["open_overlay", "swap_overlay"] {
        let action = serde_json::json!({"kind":kind,"stage":1});
        value["prototype"]["links"][0]["action"] = action;
        let decoded = PreviewStageManifest::from_json(&value.to_string()).unwrap();
        let options = match decoded.prototype.links[0].action {
            PreviewPrototypeActionManifest::OpenOverlay { options, .. }
            | PreviewPrototypeActionManifest::SwapOverlay { options, .. } => options,
            _ => unreachable!(),
        };
        assert_eq!(options, PreviewPrototypeOverlayOptions::default());
    }
}

#[test]
fn overlay_stack_back_and_close_preserve_screen_history_and_reject_covered_actions() {
    let graph = graph(
        10,
        vec![
            link(10, "Next", PreviewPrototypeAction::Navigate { stage: 40 }),
            link(40, "Open", open(20)),
            link(20, "Nested", open(30)),
            link(20, "Close", PreviewPrototypeAction::CloseOverlay),
            link(30, "Close", PreviewPrototypeAction::CloseOverlay),
            link(30, "Back", PreviewPrototypeAction::Back),
        ],
    );
    graph.validate(&[10, 20, 30, 40]).unwrap();
    let mut session = PreviewPrototypeSession::start(&graph, "Review").unwrap();
    session.activate_link(0).unwrap();
    session.activate_link(1).unwrap();
    assert_eq!(session.screen(), 40);
    assert_eq!(session.current(), 20);
    assert_eq!(session.active_stages(), vec![40, 20]);
    assert_eq!(session.history(), &[10]);
    let accepted = session.clone();
    assert!(session.activate_link(1).is_err());
    assert_eq!(session, accepted);
    session.activate_link(2).unwrap();
    assert_eq!(session.active_stages(), vec![40, 20, 30]);
    assert_eq!(session.activate_link(5).unwrap(), 20);
    assert_eq!(session.history(), &[10]);
    session.activate_link(2).unwrap();
    assert_eq!(session.activate_link(4).unwrap(), 20);
    assert_eq!(session.activate_link(3).unwrap(), 40);
    assert!(session.overlays().is_empty());
    assert_eq!(session.back().unwrap(), 10);
    assert!(session.history().is_empty());
    assert!(!session.can_go_back());
    let accepted = session.clone();
    assert!(session.close_overlay().is_err());
    assert!(session.back().is_err());
    assert_eq!(session, accepted);
}

#[test]
fn swapping_overlays_replaces_only_the_top_and_rejects_already_presented_stages() {
    let alternate = PreviewPrototypeOverlayOptions {
        placement: PreviewPrototypeOverlayPlacement::BottomCenter,
        dismiss_on_outside: true,
        dim_background: false,
    };
    let graph = graph(
        10,
        vec![
            link(10, "Open", open(20)),
            link(20, "Nested", open(30)),
            link(30, "Swap", swap(40)),
            link(
                40,
                "Options",
                PreviewPrototypeAction::SwapOverlay {
                    stage: 40,
                    options: alternate,
                },
            ),
            link(40, "Lower", swap(20)),
            link(40, "Base", swap(10)),
            link(10, "Missing", swap(30)),
        ],
    );
    let mut session = PreviewPrototypeSession::start(&graph, "Review").unwrap();
    let initial = session.clone();
    assert!(session.activate_link(6).is_err());
    assert_eq!(session, initial);
    session.activate_link(0).unwrap();
    session.activate_link(1).unwrap();
    session.activate_link(2).unwrap();
    assert_eq!(session.active_stages(), vec![10, 20, 40]);
    assert_eq!(
        session.overlays()[0].options,
        PreviewPrototypeOverlayOptions::default()
    );
    assert!(session.history().is_empty());
    session.activate_link(3).unwrap();
    assert_eq!(session.overlays()[1].options, alternate);
    let accepted = session.clone();
    for index in [4, 5] {
        assert!(session.activate_link(index).is_err());
        assert_eq!(session, accepted);
    }
    let mut forged = graph.links[3].clone();
    forged.action = PreviewPrototypeAction::CloseOverlay;
    assert!(session.activate(&forged).is_err());
    assert_eq!(session, accepted);
    assert_eq!(session.back().unwrap(), 20);
    assert_eq!(session.close_overlay().unwrap(), 10);
}

#[test]
fn overlay_depth_and_duplicate_open_rejections_are_atomic() {
    let mut links = (1..=9)
        .map(|stage| link(stage, "Next", open(stage + 1)))
        .collect::<Vec<_>>();
    for (name, stage) in [("Base", 1), ("Lower", 2), ("Self", 3)] {
        links.push(link(3, name, open(stage)));
    }
    let graph = graph(1, links);
    graph.validate(&(1..=10).collect::<Vec<_>>()).unwrap();
    let mut session = PreviewPrototypeSession::start(&graph, "Review").unwrap();
    session.activate_link(0).unwrap();
    session.activate_link(1).unwrap();
    let accepted = session.clone();
    for index in 9..12 {
        assert!(session
            .activate_link(index)
            .unwrap_err()
            .contains("already presented"));
        assert_eq!(session, accepted);
    }
    for index in 2..8 {
        session.activate_link(index).unwrap();
    }
    assert_eq!(session.overlays().len(), MAX_PREVIEW_PROTOTYPE_OVERLAYS);
    assert_eq!(session.current(), 9);
    let accepted = session.clone();
    assert!(session.activate_link(8).unwrap_err().contains("at most 8"));
    assert_eq!(session, accepted);
    assert_eq!(session.close_overlay().unwrap(), 8);
    assert_eq!(session.activate_link(7).unwrap(), 9);
    assert_eq!(session.overlays().len(), MAX_PREVIEW_PROTOTYPE_OVERLAYS);
}

#[test]
fn navigation_from_overlays_tracks_the_base_and_preserves_layers_when_history_is_full() {
    let graph = graph(
        10,
        vec![
            link(
                10,
                "Forward",
                PreviewPrototypeAction::Navigate { stage: 20 },
            ),
            link(
                20,
                "Forward",
                PreviewPrototypeAction::Navigate { stage: 10 },
            ),
            link(10, "Open", open(30)),
            link(
                30,
                "Different",
                PreviewPrototypeAction::Navigate { stage: 40 },
            ),
            link(30, "Base", PreviewPrototypeAction::Navigate { stage: 10 }),
            link(
                30,
                "Promote",
                PreviewPrototypeAction::Navigate { stage: 30 },
            ),
        ],
    );
    let mut session = PreviewPrototypeSession::start(&graph, "Review").unwrap();
    session.activate_link(2).unwrap();
    assert!(session.can_go_back());
    session.activate_link(3).unwrap();
    assert_eq!(session.current(), 40);
    assert_eq!(session.screen(), 40);
    assert_eq!(session.history(), &[10]);
    assert!(session.overlays().is_empty());
    session.back().unwrap();
    session.activate_link(2).unwrap();
    session.activate_link(4).unwrap();
    assert_eq!(session.active_stages(), vec![10]);
    assert!(session.history().is_empty());
    session.activate_link(2).unwrap();
    session.activate_link(5).unwrap();
    assert_eq!(session.active_stages(), vec![30]);
    assert_eq!(session.history(), &[10]);
    session.restart();
    for index in 0..MAX_PREVIEW_PROTOTYPE_HISTORY {
        session.activate_link(index % 2).unwrap();
    }
    session.activate_link(2).unwrap();
    let accepted = session.clone();
    assert!(session
        .activate_link(3)
        .unwrap_err()
        .contains("history is full"));
    assert_eq!(session, accepted);
    session.activate_link(4).unwrap();
    assert!(session.overlays().is_empty());
    assert_eq!(session.history().len(), MAX_PREVIEW_PROTOTYPE_HISTORY);
    session.activate_link(2).unwrap();
    assert_eq!(session.restart(), 10);
    assert!(session.overlays().is_empty());
    assert!(session.history().is_empty());
    assert_eq!(session.activate_link(2).unwrap(), 30);
}

#[test]
fn overlay_reachability_and_removal_include_all_target_bearing_actions() {
    let mut graph = graph(
        10,
        vec![
            link(10, "Open", open(20)),
            link(20, "Swap", swap(30)),
            link(20, "Close", PreviewPrototypeAction::CloseOverlay),
            link(
                30,
                "Navigate",
                PreviewPrototypeAction::Navigate { stage: 40 },
            ),
            link(40, "Back", PreviewPrototypeAction::Back),
            link(40, "Cycle", open(10)),
            link(80, "Unrelated", open(81)),
        ],
    );
    graph.validate(&[10, 20, 30, 40, 80, 81]).unwrap();
    assert_eq!(graph.reachable(10), BTreeSet::from([10, 20, 30, 40]));
    assert_eq!(graph.reachable(80), BTreeSet::from([80, 81]));
    assert_eq!(graph.links[0].action.target_stage(), Some(20));
    assert_eq!(graph.links[1].action.target_stage(), Some(30));
    assert_eq!(graph.links[2].action.target_stage(), None);
    graph.remove_stage(30);
    assert!(graph
        .links
        .iter()
        .all(|link| link.source != 30 && link.action.target_stage() != Some(30)));
    assert_eq!(graph.reachable(10), BTreeSet::from([10, 20]));
    assert!(graph
        .links
        .iter()
        .any(|link| link.action == PreviewPrototypeAction::CloseOverlay));
    graph.remove_stage(20);
    assert_eq!(graph.reachable(10), BTreeSet::from([10]));
    graph.remove_stage(10);
    assert!(graph.flows.is_empty());
}

fn missing_source_board() -> (PreviewStageBoard, [PreviewStageId; 3]) {
    let mut board = PreviewStageBoard::new();
    let mut ids = Vec::new();
    for name in ["Home", "Prompt", "Choice"] {
        ids.push(
            board
                .add(PreviewStageConfiguration::new(
                    name,
                    "memory://overlays.xaml",
                    PreviewEnvironment::default(),
                ))
                .unwrap(),
        );
    }
    (board, [ids[0], ids[1], ids[2]])
}

#[test]
fn overlay_manifest_v3_roundtrip_remaps_fresh_ids_and_rejects_legacy_versions_atomically() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (mut board, [home, prompt, choice]) = missing_source_board();
    board
        .set_prototype(graph(
            home,
            vec![
                link(home, "Open", open(prompt)),
                link(prompt, "Swap", swap(choice)),
                link(choice, "Close", PreviewPrototypeAction::CloseOverlay),
            ],
        ))
        .unwrap();
    board.move_stage(home, 2).unwrap();
    let manifest = board.manifest();
    assert_eq!(manifest.version, 3);
    assert_eq!(manifest.prototype.flows[0].start, 2);
    assert!(matches!(
        manifest.prototype.links[0].action,
        PreviewPrototypeActionManifest::OpenOverlay { stage: 0, .. }
    ));
    assert!(matches!(
        manifest.prototype.links[1].action,
        PreviewPrototypeActionManifest::SwapOverlay { stage: 1, .. }
    ));
    assert_eq!(
        manifest.prototype.links[2].action,
        PreviewPrototypeActionManifest::CloseOverlay
    );
    let json = manifest.to_json().unwrap();
    let decoded = PreviewStageManifest::from_json(&json).unwrap();
    assert_eq!(decoded, manifest);
    let mut restored = PreviewStageBoard::new();
    restored.restore_manifest(decoded).unwrap();
    let ids = restored
        .stages()
        .iter()
        .map(|info| info.id)
        .collect::<Vec<_>>();
    assert!(ids.iter().all(|id| ![home, prompt, choice].contains(id)));
    assert_eq!(restored.prototype().flows[0].start, ids[2]);
    assert_eq!(
        restored.prototype().links[0].action.target_stage(),
        Some(ids[0])
    );
    assert_eq!(
        restored.prototype().links[1].action.target_stage(),
        Some(ids[1])
    );
    assert_eq!(restored.manifest().prototype, manifest.prototype);
    let epoch = restored.epoch();
    let accepted = restored.manifest();
    for version in [1, 2] {
        let mut invalid = manifest.clone();
        invalid.version = version;
        assert!(invalid.to_json().unwrap_err().contains("version 3"));
        assert!(restored.restore_manifest(invalid).is_err());
        assert_eq!(restored.epoch(), epoch);
        assert_eq!(restored.manifest(), accepted);
    }
    for action in [
        PreviewPrototypeActionManifest::OpenOverlay {
            stage: usize::MAX,
            options: PreviewPrototypeOverlayOptions::default(),
        },
        PreviewPrototypeActionManifest::SwapOverlay {
            stage: usize::MAX,
            options: PreviewPrototypeOverlayOptions::default(),
        },
    ] {
        let mut invalid = manifest.clone();
        invalid.prototype.links[0].action = action;
        assert!(restored.restore_manifest(invalid).is_err());
        assert_eq!(restored.manifest(), accepted);
        assert_eq!(restored.epoch(), epoch);
    }
    let mut invalid = manifest;
    invalid.version = 4;
    assert!(invalid.validate().is_err());
}

#[test]
fn removing_overlay_stages_preserves_remaining_hosts_and_downgrades_manifest_version() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (mut board, [home, prompt, choice]) = missing_source_board();
    let host = board.control(home).unwrap();
    let epoch = board.stage(home).unwrap().epoch;
    board
        .set_prototype(graph(
            home,
            vec![
                link(home, "Open", open(prompt)),
                link(prompt, "Swap", swap(choice)),
                link(choice, "Close", PreviewPrototypeAction::CloseOverlay),
            ],
        ))
        .unwrap();
    assert_eq!(board.manifest().version, 3);
    board.remove(prompt).unwrap();
    assert_eq!(board.prototype().links.len(), 1);
    assert_eq!(board.manifest().version, 3);
    board.remove(choice).unwrap();
    assert_eq!(board.manifest().version, 2);
    assert!(board.prototype().links.is_empty());
    assert_eq!(board.control(home).unwrap(), host);
    assert_eq!(board.stage(home).unwrap().epoch, epoch);
    board.set_prototype(PreviewPrototype::default()).unwrap();
    assert_eq!(board.manifest().version, 1);
    assert!(!board.manifest().to_json().unwrap().contains("prototype"));
}
