use super::*;
use crate::{
    Document, PreviewEnvironment, PreviewHostOptions, PreviewSourceSegment, PreviewStageBoard,
    PreviewStageConfiguration, PreviewStageManifest, PreviewStageMode, PreviewStageSource,
};
use ferroui_base::{Rect, Ref, Visual};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, Control, Grid, TextBox, Window};
use ferroui_inspect::{Inspector, TreeKind};

const URI: &str = "ferres://FerroUI.Designer.Tests/Prototype.xaml";

fn anchor(name: &str) -> PreviewSourceAnchor {
    PreviewSourceAnchor {
        path: vec![PreviewSourceSegment {
            namespace: "https://github.com/ferroui".into(),
            type_name: "Button".into(),
            name: Some(name.into()),
            key: None,
        }],
    }
}

fn link(
    source: PreviewStageId,
    trigger: &str,
    action: PreviewPrototypeAction,
) -> PreviewPrototypeLink {
    PreviewPrototypeLink {
        name: "Continue".into(),
        source,
        trigger: anchor(trigger),
        action,
    }
}

fn graph(first: PreviewStageId, second: PreviewStageId) -> PreviewPrototype {
    PreviewPrototype {
        flows: vec![PreviewPrototypeFlow {
            name: "Checkout".into(),
            start: first,
        }],
        links: vec![
            link(
                first,
                "Next",
                PreviewPrototypeAction::Navigate { stage: second },
            ),
            link(second, "Return", PreviewPrototypeAction::Back),
        ],
    }
}

#[test]
fn descriptor_validation_rejects_ambiguous_triggers_names_and_missing_stages() {
    let valid = graph(11, 23);
    valid.validate(&[11, 23]).unwrap();
    let mut invalid = valid.clone();
    invalid.flows.push(PreviewPrototypeFlow {
        name: "Checkout".into(),
        start: 23,
    });
    assert!(invalid.validate(&[11, 23]).unwrap_err().contains("unique"));
    let mut invalid = valid.clone();
    let mut duplicate = invalid.links[0].clone();
    duplicate.name = "Different label".into();
    duplicate.action = PreviewPrototypeAction::Back;
    invalid.links.push(duplicate);
    assert!(invalid
        .validate(&[11, 23])
        .unwrap_err()
        .contains("one prototype interaction"));
    let mutations: [fn(&mut PreviewPrototype); 3] = [
        |graph: &mut PreviewPrototype| graph.flows[0].start = 99,
        |graph: &mut PreviewPrototype| graph.links[0].source = 99,
        |graph: &mut PreviewPrototype| {
            graph.links[0].action = PreviewPrototypeAction::Navigate { stage: 99 }
        },
    ];
    for mutate in mutations {
        let mut invalid = valid.clone();
        mutate(&mut invalid);
        assert!(invalid
            .validate(&[11, 23])
            .unwrap_err()
            .contains("no longer exists"));
    }
    let mut invalid = valid.clone();
    invalid.links[0].trigger.path.clear();
    assert!(invalid.validate(&[11, 23]).is_err());
    let mut invalid = valid.clone();
    invalid.flows[0].name = " ".into();
    assert!(invalid.validate(&[11, 23]).is_err());
    let mut invalid = valid.clone();
    invalid.links[0].name = "é".repeat(65);
    assert!(invalid.validate(&[11, 23]).is_err());
    let mut invalid = valid.clone();
    invalid.flows = (0..=MAX_PREVIEW_PROTOTYPE_FLOWS)
        .map(|i| PreviewPrototypeFlow {
            name: format!("Flow {i}"),
            start: 11,
        })
        .collect();
    assert!(invalid.validate(&[11, 23]).is_err());
    let mut invalid = valid;
    invalid.links = (0..=MAX_PREVIEW_PROTOTYPE_LINKS)
        .map(|i| link(11, &format!("Trigger{i}"), PreviewPrototypeAction::Back))
        .collect();
    assert!(invalid.validate(&[11, 23]).is_err());
}

#[test]
fn reachable_screens_follow_navigation_edges_and_terminate_on_cycles() {
    let mut graph = graph(11, 23);
    graph.links.push(link(
        23,
        "Cycle",
        PreviewPrototypeAction::Navigate { stage: 11 },
    ));
    graph.links.push(link(
        23,
        "Branch",
        PreviewPrototypeAction::Navigate { stage: 45 },
    ));
    graph.links.push(link(
        90,
        "Unrelated",
        PreviewPrototypeAction::Navigate { stage: 91 },
    ));
    graph.validate(&[11, 23, 45, 90, 91]).unwrap();
    assert_eq!(graph.reachable(11), BTreeSet::from([11, 23, 45]));
    assert_eq!(graph.reachable(45), BTreeSet::from([45]));
    assert_eq!(graph.reachable(90), BTreeSet::from([90, 91]));
}

#[test]
fn playback_keeps_an_accepted_graph_and_rejects_wrong_screen_or_forged_interactions() {
    let mut graph = graph(11, 23);
    let accepted = graph.clone();
    let mut session = PreviewPrototypeSession::start(&graph, "Checkout").unwrap();
    assert_eq!(session.flow(), "Checkout");
    assert_eq!(session.current(), 11);
    assert!(!session.can_go_back());
    assert!(session.back().is_err());
    assert!(session.activate_link(1).is_err());
    assert!(session.activate_link(2).is_err());
    let mut forged = accepted.links[0].clone();
    forged.action = PreviewPrototypeAction::Navigate { stage: 90 };
    assert!(session.activate(&forged).is_err());
    assert_eq!(session.current(), 11);
    assert!(session.history().is_empty());
    graph.links.clear();
    assert_eq!(session.activate_link(0).unwrap(), 23);
    assert_eq!(session.history(), &[11]);
    assert_eq!(session.activate(&accepted.links[1]).unwrap(), 11);
    assert!(session.history().is_empty());
    assert!(PreviewPrototypeSession::start(&graph, "Removed").is_err());
    graph.flows.push(graph.flows[0].clone());
    assert!(PreviewPrototypeSession::start(&graph, "Checkout").is_err());
}

#[test]
fn navigation_history_is_bounded_transactionally_and_restart_preserves_the_graph() {
    let mut graph = graph(11, 23);
    graph.links[1].action = PreviewPrototypeAction::Navigate { stage: 11 };
    graph.links.push(link(
        11,
        "Stay",
        PreviewPrototypeAction::Navigate { stage: 11 },
    ));
    let mut session = PreviewPrototypeSession::start(&graph, "Checkout").unwrap();
    assert_eq!(session.activate_link(2).unwrap(), 11);
    assert!(session.history().is_empty());
    for index in 0..MAX_PREVIEW_PROTOTYPE_HISTORY {
        session.activate_link(index % 2).unwrap();
    }
    let accepted = session.clone();
    assert!(session
        .activate_link(0)
        .unwrap_err()
        .contains("history is full"));
    assert_eq!(session, accepted);
    assert_eq!(session.back().unwrap(), 23);
    assert_eq!(session.history().len(), MAX_PREVIEW_PROTOTYPE_HISTORY - 1);
    assert_eq!(session.restart(), 11);
    assert!(session.history().is_empty());
    assert_eq!(session.activate_link(0).unwrap(), 23);
}

#[test]
fn portable_graph_maps_positions_to_fresh_ids_and_rejects_invalid_indices() {
    let graph = graph(11001, 23001);
    let portable = PreviewPrototypeManifest::from_prototype(&graph, &[23001, 11001]).unwrap();
    assert_eq!(portable.flows[0].start, 1);
    assert_eq!(portable.links[0].source, 1);
    assert_eq!(
        portable.links[0].action,
        PreviewPrototypeActionManifest::Navigate { stage: 0 }
    );
    let json = serde_json::to_string(&portable).unwrap();
    assert!(!json.contains("11001"));
    assert!(!json.contains("23001"));
    let decoded: PreviewPrototypeManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(
        decoded.restore(&[501, 502]).unwrap(),
        graph_with_ids(&graph, 502, 501)
    );
    let mut invalid = decoded.clone();
    invalid.links[0].action = PreviewPrototypeActionManifest::Navigate { stage: usize::MAX };
    assert!(invalid.validate(2).is_err());
    assert!(decoded.restore(&[7, 7]).is_err());
    assert!(PreviewPrototypeManifest::from_prototype(&graph, &[11001, 11001, 23001]).is_err());
    let mut value = serde_json::to_value(&decoded).unwrap();
    value["links"][0]["inspector_id"] = serde_json::json!(99);
    assert!(serde_json::from_value::<PreviewPrototypeManifest>(value).is_err());
}

fn graph_with_ids(
    original: &PreviewPrototype,
    first: PreviewStageId,
    second: PreviewStageId,
) -> PreviewPrototype {
    let mut graph = graph(first, second);
    graph.links[0].trigger = original.links[0].trigger.clone();
    graph.links[1].trigger = original.links[1].trigger.clone();
    graph
}

fn board(document: &Document) -> (PreviewStageBoard, PreviewStageId, PreviewStageId) {
    let mut board = PreviewStageBoard::new();
    sync(&mut board, document, false);
    let mut first = PreviewStageConfiguration::new("First", URI, PreviewEnvironment::default());
    first.mode = PreviewStageMode::Live;
    let first = board.add(first).unwrap();
    let second = board
        .add(PreviewStageConfiguration::new(
            "Second",
            URI,
            PreviewEnvironment::default(),
        ))
        .unwrap();
    (board, first, second)
}

fn sync(board: &mut PreviewStageBoard, document: &Document, pending: bool) {
    board
        .sync_sources(
            URI,
            vec![PreviewStageSource {
                uri: URI.into(),
                document: document.clone(),
                pending,
            }],
            PreviewHostOptions::default(),
            1,
        )
        .unwrap();
}

fn attach(board: &mut PreviewStageBoard, id: PreviewStageId) -> Ref<Window> {
    let window = Window::new();
    window.set_width(800.0);
    window.set_height(600.0);
    let grid = Grid::new();
    grid.children().add(board.control(id).unwrap());
    window.set_content(Some(Control::boxed(grid)));
    window.show();
    window.update_layout();
    board.refresh_runtime_mapping(id).unwrap();
    window
}

fn visual(
    board: &PreviewStageBoard,
    stage: PreviewStageId,
    document: &Document,
    name: &str,
) -> Ref<Visual> {
    let id = document.ast().find_name(name).unwrap();
    let node = board
        .runtime_tree(stage, TreeKind::Visual)
        .unwrap()
        .into_iter()
        .find(|row| row.source_id == Some(id))
        .unwrap();
    Inspector::shared()
        .borrow()
        .resolve(node.inspector_id)
        .unwrap()
        .cast::<Visual>()
        .unwrap()
}

#[test]
fn board_graph_changes_preserve_native_state_and_manifest_restore_remaps_every_reference() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(
        r#"<TextBox xmlns="https://github.com/ferroui" Name="Input" Text="Authored"/>"#,
    )
    .unwrap();
    let source = document.source().to_owned();
    let ast = document.ast_snapshot();
    let (mut board, first, second) = board(&document);
    let window = attach(&mut board, first);
    let input = visual(&board, first, &document, "Input")
        .cast::<TextBox>()
        .unwrap();
    input.set_text(Some("Typed in preview"));
    let stage_epoch = board.stage(first).unwrap().epoch;
    let board_epoch = board.epoch();
    let graph = graph(first, second);
    board.set_prototype(graph.clone()).unwrap();
    assert!(board.epoch() > board_epoch);
    assert_eq!(board.stage(first).unwrap().epoch, stage_epoch);
    assert_eq!(input.text().as_deref(), Some("Typed in preview"));
    assert_eq!(
        visual(&board, first, &document, "Input"),
        input.clone().upcast::<Visual>()
    );
    let accepted_epoch = board.epoch();
    board.set_prototype(graph.clone()).unwrap();
    assert_eq!(board.epoch(), accepted_epoch);
    let mut rejected = graph.clone();
    rejected.links[0].action = PreviewPrototypeAction::Navigate { stage: u64::MAX };
    assert!(board.set_prototype(rejected).is_err());
    assert_eq!(board.prototype(), graph);
    assert_eq!(board.epoch(), accepted_epoch);
    board.move_stage(first, 1).unwrap();
    assert_eq!(board.prototype(), graph);
    let manifest = board.manifest();
    assert_eq!(manifest.version, 2);
    assert_eq!(manifest.prototype.flows[0].start, 1);
    assert_eq!(
        manifest.prototype.links[0].action,
        PreviewPrototypeActionManifest::Navigate { stage: 0 }
    );
    let json = manifest.to_json().unwrap();
    let manifest = PreviewStageManifest::from_json(&json).unwrap();
    let mut restored = PreviewStageBoard::new();
    sync(&mut restored, &document, false);
    restored.restore_manifest(manifest).unwrap();
    let ids = restored
        .stages()
        .iter()
        .map(|stage| stage.id)
        .collect::<Vec<_>>();
    assert!(ids.iter().all(|id| *id != first && *id != second));
    assert_eq!(restored.prototype(), graph_with_ids(&graph, ids[1], ids[0]));
    board.remove(second).unwrap();
    assert_eq!(board.prototype().flows.len(), 1);
    assert!(board.prototype().links.is_empty());
    board.remove(first).unwrap();
    assert!(board.prototype().is_empty());
    assert_eq!(board.manifest().version, 1);
    assert_eq!(document.source(), source);
    assert!(std::sync::Arc::ptr_eq(&ast, &document.ast_snapshot()));
    assert_eq!(document.revision(), 0);
    window.close();
}

#[test]
fn legacy_manifest_omits_prototype_and_invalid_import_preserves_accepted_board() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document =
        Document::new(r#"<Button xmlns="https://github.com/ferroui" Name="Next"/>"#).unwrap();
    let (mut board, first, second) = board(&document);
    let legacy = board.manifest().to_json().unwrap();
    assert!(!legacy.contains("prototype"));
    assert!(PreviewStageManifest::from_json(&legacy)
        .unwrap()
        .prototype
        .is_empty());
    board.set_prototype(graph(first, second)).unwrap();
    let accepted = board.manifest();
    let epoch = board.epoch();
    let mut invalid = accepted.clone();
    invalid.prototype.flows[0].start = 17;
    assert!(board.restore_manifest(invalid).is_err());
    assert_eq!(board.manifest(), accepted);
    assert_eq!(board.epoch(), epoch);
    let mut invalid = accepted.clone();
    invalid.version = 1;
    assert!(invalid.to_json().is_err());
    invalid.version = 3;
    assert!(invalid.validate().is_err());
}

#[test]
fn native_hotspots_follow_clipped_layout_and_reject_stale_hidden_disabled_or_detached_views() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(
        r#"<Canvas xmlns="https://github.com/ferroui" Width="300" Height="200" HorizontalAlignment="Left" VerticalAlignment="Top">
  <Border Name="Clip" Width="100" Height="100" ClipToBounds="True">
    <Canvas><Button Name="Target" Width="80" Height="30" Canvas.Left="70" Canvas.Top="20"/></Canvas>
  </Border>
</Canvas>"#,
    )
    .unwrap();
    let (mut board, first, _) = board(&document);
    let anchor =
        PreviewSourceAnchor::capture(&document, document.ast().find_name("Target").unwrap())
            .unwrap();
    assert!(board.prototype_hotspot_bounds(first, &anchor).is_err());
    let window = attach(&mut board, first);
    assert_eq!(
        board.prototype_hotspot_bounds(first, &anchor).unwrap(),
        Rect::new(70.0, 20.0, 30.0, 30.0)
    );
    let target = visual(&board, first, &document, "Target")
        .cast::<Button>()
        .unwrap();
    let epoch = board.epoch();
    let transform = ferroui_base::media::TranslateTransform::with_offset(10.0, 0.0);
    target.set_render_transform(Some((&transform).into()));
    assert_eq!(
        board.prototype_hotspot_bounds(first, &anchor).unwrap(),
        Rect::new(80.0, 20.0, 20.0, 30.0)
    );
    assert_eq!(board.epoch(), epoch);
    transform.set_x(100.0);
    assert!(board.prototype_hotspot_bounds(first, &anchor).is_err());
    target.set_render_transform(None);
    target.set_is_enabled(false);
    assert!(board
        .prototype_hotspot_bounds(first, &anchor)
        .unwrap_err()
        .contains("disabled"));
    target.set_is_enabled(true);
    target.set_is_visible(false);
    assert!(board.prototype_hotspot_bounds(first, &anchor).is_err());
    target.set_is_visible(true);
    window.update_layout();
    target.invalidate_measure();
    assert!(board
        .prototype_hotspot_bounds(first, &anchor)
        .unwrap_err()
        .contains("layout"));
    window.update_layout();
    assert!(board.prototype_hotspot_bounds(first, &anchor).is_ok());
    sync(&mut board, &document, true);
    assert!(board.prototype_hotspot_bounds(first, &anchor).is_err());
    sync(&mut board, &document, false);
    window.update_layout();
    board.refresh_runtime_mappings();
    assert!(board.prototype_hotspot_bounds(first, &anchor).is_ok());
    board.set_paused(first, true).unwrap();
    assert!(board.prototype_hotspot_bounds(first, &anchor).is_err());
    window.close();
}

#[test]
fn repeated_native_hotspot_instances_require_an_isolated_definition_stage() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <StackPanel.Resources>
    <ControlTemplate x:Key="Repeated" TargetType="Button">
      <Border Name="Target" Width="100" Height="40" Background="Red"/>
    </ControlTemplate>
  </StackPanel.Resources>
  <Button Name="First" Template="{StaticResource Repeated}"/>
  <Button Name="Second" Template="{StaticResource Repeated}"/>
</StackPanel>"#).unwrap();
    let (mut board, first, _) = board(&document);
    let anchor =
        PreviewSourceAnchor::capture(&document, document.ast().find_name("Target").unwrap())
            .unwrap();
    let window = attach(&mut board, first);
    assert!(board
        .prototype_hotspot_bounds(first, &anchor)
        .unwrap_err()
        .contains("multiple native instances"));
    window.close();
    let template = document
        .ast()
        .elements()
        .find(|node| node.local_name() == "ControlTemplate")
        .unwrap()
        .id;
    let mut configuration =
        PreviewStageConfiguration::new("Definition", URI, PreviewEnvironment::default());
    configuration.target = crate::PreviewStageTarget::Definition {
        anchor: PreviewSourceAnchor::capture(&document, template).unwrap(),
        options: crate::DefinitionPreviewOptions::default(),
    };
    let definition = board.add(configuration).unwrap();
    let window = attach(&mut board, definition);
    let bounds = board.prototype_hotspot_bounds(definition, &anchor).unwrap();
    assert_eq!(bounds.width, 100.0);
    assert_eq!(bounds.height, 40.0);
    assert_eq!(document.revision(), 0);
    window.close();
}
