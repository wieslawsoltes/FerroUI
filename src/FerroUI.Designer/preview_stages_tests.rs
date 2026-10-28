use super::*;
use crate::DesignerSession;
use ferroui_base::layout::Orientation;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, StackPanel, TextBlock, TextBox, Window};
use ferroui_designer_core::Edit;
use std::cell::{Cell, RefCell};

const URI: &str = "ferres://FerroUI.Designer.Tests/Stages.xaml";
const OTHER: &str = "ferres://FerroUI.Designer.Tests/Other.xaml";

fn source(document: &Document, uri: &str, pending: bool) -> PreviewStageSource {
    PreviewStageSource {
        uri: uri.into(),
        document: document.clone(),
        pending,
    }
}

fn sync(board: &mut PreviewStageBoard, document: &Document, pending: bool) {
    board
        .sync_sources(
            URI,
            vec![source(document, URI, pending)],
            PreviewHostOptions::default(),
            1,
        )
        .unwrap();
}

fn config(name: &str) -> PreviewStageConfiguration {
    PreviewStageConfiguration::new(name, URI, PreviewEnvironment::default())
}

fn attach(board: &PreviewStageBoard) -> Ref<Window> {
    let panel = StackPanel::new();
    panel.set_orientation(Orientation::Horizontal);
    for stage in board.stages() {
        panel.children().add(board.control(stage.id).unwrap());
    }
    let window = Window::new();
    window.set_width(1800.0);
    window.set_height(1200.0);
    window.set_content(Some(Control::boxed(panel)));
    window.show();
    window.update_layout();
    window
}

fn visual(board: &PreviewStageBoard, stage: PreviewStageId, node: NodeId) -> Ref<Visual> {
    board.stages[board.index(stage).unwrap()]
        .rendered
        .as_ref()
        .unwrap()
        .preview
        .visual(node)
        .unwrap()
}

fn runtime_id(board: &PreviewStageBoard, stage: PreviewStageId, node: NodeId) -> u64 {
    Inspector::shared()
        .borrow_mut()
        .node_id(&visual(board, stage, node))
}

#[test]
fn same_source_has_multiple_independent_native_environments_without_reload_ownership_conflicts() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let authored = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:d="http://schemas.microsoft.com/expression/blend/2008" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="d"><TextBlock Name="Label" Text="Runtime" d:Text="Design"/><TextBox Name="Input" Text="Initial"/></StackPanel>"#;
    let mut session =
        DesignerSession::with_uri(authored, ContentControl::new(), URI.into()).unwrap();
    session.set_read_only(true);
    let document = session.document().clone();
    let label = document.ast().find_name("Label").unwrap();
    let input = document.ast().find_name("Input").unwrap();
    let mut board = PreviewStageBoard::new();
    board
        .sync_sources(
            URI,
            vec![source(&document, URI, false)],
            session.preview_host_options(),
            session.preview_host_options_epoch(),
        )
        .unwrap();
    let mut phone = config("Phone");
    phone.environment = PreviewEnvironment {
        width: 390.0,
        height: 844.0,
        theme: PreviewTheme::Dark,
        flow_direction: PreviewFlowDirection::RightToLeft,
    };
    let phone = board.add(phone).unwrap();
    let mut desktop = config("Desktop");
    desktop.environment.width = 1280.0;
    desktop.environment.height = 800.0;
    desktop.design_time_data = false;
    let desktop = board.add(desktop).unwrap();
    let window = attach(&board);
    assert!(board.refresh_all().is_empty());
    assert!(matches!(
        board.stage(phone).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    assert!(matches!(
        board.stage(desktop).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    let phone_label = visual(&board, phone, label).cast::<TextBlock>().unwrap();
    let desktop_label = visual(&board, desktop, label).cast::<TextBlock>().unwrap();
    assert_ne!(phone_label, desktop_label);
    assert_ne!(
        phone_label.clone().upcast::<Visual>(),
        session.preview.visual(label).unwrap()
    );
    assert_eq!(phone_label.text().as_deref(), Some("Design"));
    assert_eq!(desktop_label.text().as_deref(), Some("Runtime"));
    assert_eq!(phone_label.flow_direction(), FlowDirection::RightToLeft);
    assert_eq!(desktop_label.flow_direction(), FlowDirection::LeftToRight);
    assert_eq!(
        phone_label.actual_theme_variant(),
        Some(ThemeVariant::dark())
    );
    assert_eq!(
        desktop_label.actual_theme_variant(),
        Some(ThemeVariant::light())
    );
    visual(&board, phone, input)
        .cast::<TextBox>()
        .unwrap()
        .set_text(Some("Phone edit"));
    assert_eq!(
        visual(&board, desktop, input)
            .cast::<TextBox>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Initial")
    );
    assert_eq!(session.document().source(), authored);
    assert_eq!(session.document().revision(), document.revision());
    assert!(!session.document().can_undo());
    window.close();
}

#[test]
fn pause_pending_invalid_missing_and_native_errors_keep_the_last_graph_without_source_authority() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut document =
        Document::new(r#"<TextBlock xmlns="https://github.com/ferroui" Name="Label" Text="One"/>"#)
            .unwrap();
    let node = document.ast().root();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let live = board.add(config("Updating")).unwrap();
    let paused = board.duplicate(live).unwrap();
    board.set_paused(paused, true).unwrap();
    let old = visual(&board, paused, node);
    let token = board
        .selection_token(live, runtime_id(&board, live, node))
        .unwrap();
    document
        .apply(
            "Change text",
            vec![Edit::SetAttribute {
                node,
                name: "Text".into(),
                value: "Two".into(),
            }],
        )
        .unwrap();
    sync(&mut board, &document, true);
    assert!(matches!(
        board.stage(live).unwrap().status,
        PreviewStageStatus::PendingSource { .. }
    ));
    assert!(board.resolve_selection(&token).is_err());
    assert_eq!(
        visual(&board, live, node)
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("One")
    );
    assert!(board.refresh(live).is_err());
    sync(&mut board, &document, false);
    assert_eq!(
        visual(&board, live, node)
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Two")
    );
    assert_eq!(visual(&board, paused, node), old);
    assert!(matches!(
        board.stage(paused).unwrap().status,
        PreviewStageStatus::Paused { revision: Some(0) }
    ));
    board.refresh(paused).unwrap();
    assert_eq!(
        visual(&board, paused, node)
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Two")
    );
    assert!(board
        .selection_token(paused, runtime_id(&board, paused, node))
        .is_err());
    board.set_paused(paused, false).unwrap();
    let accepted = visual(&board, paused, node);
    document.update_source("Bad source", "<TextBlock");
    sync(&mut board, &document, false);
    assert!(matches!(
        board.stage(paused).unwrap().status,
        PreviewStageStatus::InvalidSource { .. }
    ));
    assert_eq!(visual(&board, paused, node), accepted);
    document.update_source(
        "Bad native type",
        r#"<DoesNotExist xmlns="https://github.com/ferroui"/>"#,
    );
    sync(&mut board, &document, false);
    assert!(matches!(
        board.stage(paused).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    assert_eq!(
        board.stages[board.index(paused).unwrap()]
            .rendered
            .as_ref()
            .unwrap()
            .preview
            .root()
            .unwrap()
            .upcast::<Visual>(),
        accepted
    );
    board
        .sync_sources(OTHER, Vec::new(), PreviewHostOptions::default(), 1)
        .unwrap();
    assert_eq!(
        board.stage(paused).unwrap().status,
        PreviewStageStatus::MissingDocument
    );
    assert!(board
        .runtime_tree(paused, TreeKind::Visual)
        .unwrap()
        .iter()
        .all(|node| node.source_id.is_none()));
}

#[test]
fn source_anchor_survives_named_reordering_but_rejects_ambiguity_namespace_and_reused_ids() {
    let original = Document::new(r#"<Grid xmlns="https://github.com/ferroui"><Border Name="A"><Button/></Border><Border Name="B"><Button/></Border></Grid>"#).unwrap();
    let a = original.ast().find_name("A").unwrap();
    let target = original
        .ast()
        .node(a)
        .unwrap()
        .children
        .iter()
        .copied()
        .find(|id| original.ast().node(*id).unwrap().is_element())
        .unwrap();
    let anchor = PreviewSourceAnchor::capture(&original, target).unwrap();
    let reordered = Document::new(r#"<Grid xmlns="https://github.com/ferroui"><Border Name="Other"/><Border Name="B"><Button/></Border><Border Name="A"><Button/></Border></Grid>"#).unwrap();
    let resolved = anchor.resolve(&reordered).unwrap();
    assert_eq!(
        reordered.ast().node(resolved).unwrap().parent,
        reordered.ast().find_name("A")
    );
    let replaced = Document::new(r#"<Grid xmlns="https://github.com/ferroui"><Border Name="Different"><Button/></Border><Border Name="B"><Button/></Border></Grid>"#).unwrap();
    assert!(replaced.ast().node(target).is_some());
    assert!(
        anchor.resolve(&replaced).is_err(),
        "A reused numeric ID is never a persistent anchor"
    );
    let foreign =
        Document::new(r#"<Grid xmlns="urn:foreign"><Border Name="A"><Button/></Border></Grid>"#)
            .unwrap();
    assert!(anchor.resolve(&foreign).is_err());
    let ambiguous = Document::new(
        r#"<StackPanel xmlns="https://github.com/ferroui"><Button/><Button/></StackPanel>"#,
    )
    .unwrap();
    let button = ambiguous
        .ast()
        .elements()
        .find(|node| node.local_name() == "Button")
        .unwrap()
        .id;
    assert!(PreviewSourceAnchor::capture(&ambiguous, button).is_err());
}

#[test]
fn native_selection_tokens_reject_same_source_reconstruction_and_accept_anonymous_controls() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(r#"<StackPanel xmlns="https://github.com/ferroui"><Button Content="First"/><Button Content="Second"/></StackPanel>"#).unwrap();
    let node = document
        .ast()
        .elements()
        .find(|node| node.local_name() == "Button")
        .unwrap()
        .id;
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let stage = board.add(config("Source selection")).unwrap();
    let window = attach(&board);
    board.refresh(stage).unwrap();
    let old_visual = visual(&board, stage, node);
    let token = board
        .selection_token(stage, runtime_id(&board, stage, node))
        .unwrap();
    let selected = board.resolve_selection(&token).unwrap();
    assert_eq!(selected.node, node);
    assert!(selected.anchor.is_none());
    assert!(selected.matches_source(URI, &document));
    let recreated = Document::new(document.source()).unwrap();
    assert!(!selected.matches_source(URI, &recreated));
    let mut rebased = document.clone();
    rebased.advance_revision_to(50).unwrap();
    assert!(selected.matches_source(URI, &rebased));
    board.refresh(stage).unwrap();
    assert_ne!(visual(&board, stage, node), old_visual);
    assert!(board.resolve_selection(&token).is_err());
    let foreign = board.duplicate(stage).unwrap();
    assert!(board
        .selection_token(stage, runtime_id(&board, foreign, node))
        .is_err());
    window.close();
}

#[test]
fn unchanged_selection_and_tab_revisions_do_not_restart_pinned_native_inputs() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut document =
        Document::new(r#"<TextBox xmlns="https://github.com/ferroui" Text="Initial"/>"#).unwrap();
    let other = Document::new(r#"<Button xmlns="https://github.com/ferroui"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let mut configuration = config("Pinned input");
    configuration.pinned = true;
    configuration.mode = PreviewStageMode::Live;
    let stage = board.add(configuration).unwrap();
    let input = visual(&board, stage, document.ast().root())
        .cast::<TextBox>()
        .unwrap();
    input.set_text(Some("Keep my interaction"));
    let epoch = board.epoch();
    sync(&mut board, &document, false);
    assert_eq!(board.epoch(), epoch);
    board
        .sync_sources(
            OTHER,
            vec![source(&document, URI, false), source(&other, OTHER, false)],
            PreviewHostOptions::default(),
            1,
        )
        .unwrap();
    assert_eq!(
        visual(&board, stage, document.ast().root())
            .cast::<TextBox>()
            .unwrap(),
        input
    );
    document.advance_revision_to(50).unwrap();
    board
        .sync_sources(
            URI,
            vec![source(&document, URI, false), source(&other, OTHER, false)],
            PreviewHostOptions::default(),
            1,
        )
        .unwrap();
    assert_eq!(
        visual(&board, stage, document.ast().root())
            .cast::<TextBox>()
            .unwrap(),
        input
    );
    assert_eq!(input.text().as_deref(), Some("Keep my interaction"));
    assert_eq!(board.stage(stage).unwrap().rendered_revision, Some(50));
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Ready { revision: 50 }
    ));
}

#[test]
fn registered_factories_receive_original_source_and_base_uri_for_every_independent_stage() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let authored = r#"<TextBlock xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Example.Stage" Text="Hello"/>"#;
    let mut session =
        DesignerSession::with_uri(authored, ContentControl::new(), URI.into()).unwrap();
    let calls = Rc::new(Cell::new(0));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let mut options = PreviewHostOptions::default();
    let counter = calls.clone();
    let received = seen.clone();
    options.register_root(
        "Example.Stage",
        Rc::new(move |request| {
            counter.set(counter.get() + 1);
            received
                .borrow_mut()
                .push((request.uri.clone(), request.source.clone()));
            request.populate(Some(Control::boxed(TextBlock::new())))
        }),
    );
    session.configure_preview(options).unwrap();
    // The main development reload owner canonicalizes its registration URI. The
    // isolated stage contract is checked separately against the authored base URI.
    assert_eq!(calls.get(), 1);
    seen.borrow_mut().clear();
    let mut board = PreviewStageBoard::new();
    board
        .sync_sources(
            URI,
            vec![source(session.document(), URI, false)],
            session.preview_host_options(),
            session.preview_host_options_epoch(),
        )
        .unwrap();
    let first = board.add(config("First")).unwrap();
    let second = board.duplicate(first).unwrap();
    assert_eq!(calls.get(), 3);
    assert_eq!(seen.borrow().len(), 2);
    assert!(
        seen.borrow()
            .iter()
            .all(|(uri, source)| uri == URI && source == authored),
        "{:?}",
        seen.borrow()
    );
    assert_ne!(
        visual(&board, first, session.document().ast().root()),
        visual(&board, second, session.document().ast().root())
    );
    assert_eq!(session.document().source(), authored);
}

#[test]
fn projected_window_root_does_not_invoke_a_factory_for_its_synthetic_border_type() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(r#"<Window xmlns="https://github.com/ferroui" Title="Native chrome"><TextBlock Name="TitleText" Text="Window content"/></Window>"#).unwrap();
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let mut options = PreviewHostOptions::default();
    options.register_root(
        "Border",
        Rc::new(move |_| {
            counter.set(counter.get() + 1);
            Err("The synthetic Border is not an authored factory target".into())
        }),
    );
    let mut board = PreviewStageBoard::new();
    board
        .sync_sources(URI, vec![source(&document, URI, false)], options, 1)
        .unwrap();
    let stage = board.add(config("Window projection")).unwrap();
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    assert_eq!(calls.get(), 0);
    let title = document.ast().find_name("TitleText").unwrap();
    assert_eq!(
        visual(&board, stage, title)
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Window content")
    );
    assert!(board.stages[board.index(stage).unwrap()]
        .rendered
        .as_ref()
        .unwrap()
        .preview
        .is_window_projection());
    assert!(document.source().starts_with("<Window"));
}

#[test]
fn cached_factory_root_is_rejected_before_it_can_steal_another_preview_graph() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let authored = r#"<Button xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Example.Cached" Content="Initial"/>"#;
    let mut session =
        DesignerSession::with_uri(authored, ContentControl::new(), URI.into()).unwrap();
    let mut board = PreviewStageBoard::new();
    board
        .sync_sources(
            URI,
            vec![source(session.document(), URI, false)],
            session.preview_host_options(),
            session.preview_host_options_epoch(),
        )
        .unwrap();
    let stage = board.add(config("Keep last graph")).unwrap();
    let previous = visual(&board, stage, session.document().ast().root());
    let singleton = Button::new();
    singleton.set_content(Some(Rc::new("Cached".to_owned())));
    let cached = singleton.clone();
    let mut options = PreviewHostOptions::default();
    options.register_root(
        "Example.Cached",
        Rc::new(move |_| Ok(Control::boxed(cached.clone()))),
    );
    session.configure_preview(options).unwrap();
    let parent = singleton.parent();
    assert!(parent.is_some());
    board
        .sync_sources(
            URI,
            vec![source(session.document(), URI, false)],
            session.preview_host_options(),
            session.preview_host_options_epoch(),
        )
        .unwrap();
    assert!(
        matches!(board.stage(stage).unwrap().status, PreviewStageStatus::Failed { message, .. } if message.contains("fresh, unparented"))
    );
    assert_eq!(
        visual(&board, stage, session.document().ast().root()),
        previous
    );
    assert_eq!(singleton.parent(), parent);
    assert_eq!(
        session.preview.root().unwrap(),
        singleton.upcast::<Control>()
    );
}

#[test]
fn a_panicking_factory_keeps_the_accepted_graph_and_releases_its_candidate_host() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(r#"<Button xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Example.Panic" Content="Accepted"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let stage = board.add(config("Keep accepted native view")).unwrap();
    let accepted = visual(&board, stage, document.ast().root());
    let parent = accepted.parent();
    let card = board.control(stage).unwrap().cast::<Grid>().unwrap();
    let design_mode = ferroui_controls::Design::is_design_mode();
    let published = ferroui_markup_xaml::source_overlay::source(URI);
    assert_eq!(card.children().count(), 1);
    let mut options = PreviewHostOptions::default();
    options.register_root(
        "Example.Panic",
        Rc::new(|_| panic!("Factory deliberately panicked")),
    );
    board
        .sync_sources(URI, vec![source(&document, URI, false)], options, 2)
        .unwrap();
    assert!(
        matches!(board.stage(stage).unwrap().status, PreviewStageStatus::Failed { message, .. }
        if message.contains("Factory deliberately panicked"))
    );
    assert_eq!(visual(&board, stage, document.ast().root()), accepted);
    assert_eq!(accepted.parent(), parent);
    assert_eq!(
        card.children().count(),
        1,
        "A failing candidate cannot leave a hidden extra subtree"
    );
    assert_eq!(ferroui_controls::Design::is_design_mode(), design_mode);
    assert_eq!(ferroui_markup_xaml::source_overlay::source(URI), published);
    board
        .sync_sources(
            URI,
            vec![source(&document, URI, false)],
            PreviewHostOptions::default(),
            3,
        )
        .unwrap();
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    assert_eq!(card.children().count(), 1);
    assert_ne!(visual(&board, stage, document.ast().root()), accepted);
}

#[test]
fn failed_configuration_preserves_accepted_scenario_values_and_freezes_accepted_stateless_live_graphs(
) {
    use ferroui_base::animation::PlayState;
    use ferroui_designer_animation::{StateSetter, VisualState, VisualStateGroup};

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut document = Document::new(r#"<Canvas xmlns="https://github.com/ferroui" Name="Owner"><Button Name="Target" Opacity="1"/></Canvas>"#).unwrap();
    let owner = document.ast().root();
    let group = VisualStateGroup {
        name: "Interaction".into(),
        owner,
        states: vec![VisualState {
            name: "Muted".into(),
            setters: vec![StateSetter {
                target: "Target".into(),
                property: "Opacity".into(),
                value: "0.25".into(),
            }],
        }],
        transitions: Vec::new(),
    };
    document
        .apply("Author state", group.plan(&document).unwrap())
        .unwrap();
    let target = document.ast().find_name("Target").unwrap();
    let owner_anchor = PreviewSourceAnchor::capture(&document, owner).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let mut state_config = config("Accepted muted state");
    state_config.states.push(PreviewStageState {
        owner: owner_anchor.clone(),
        group: "Interaction".into(),
        state: "Muted".into(),
    });
    let state_stage = board.add(state_config.clone()).unwrap();
    let accepted_state = visual(&board, state_stage, target);
    assert_eq!(accepted_state.opacity(), 0.25);
    state_config.states.clear();
    state_config.target = PreviewStageTarget::Definition {
        anchor: PreviewSourceAnchor::capture(&document, target).unwrap(),
        options: DefinitionPreviewOptions::default(),
    };
    board.update(state_stage, state_config).unwrap();
    assert!(matches!(
        board.stage(state_stage).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    assert_eq!(visual(&board, state_stage, target), accepted_state);
    assert_eq!(
        accepted_state.opacity(),
        0.25,
        "A rejected desired configuration must preserve the accepted state's animation-value layer"
    );

    let mut live_config = config("Accepted live graph");
    live_config.mode = PreviewStageMode::Live;
    let live_stage = board.add(live_config.clone()).unwrap();
    let accepted_live = visual(&board, live_stage, target);
    assert!(board.stages[board.index(live_stage).unwrap()]
        .rendered
        .as_ref()
        .unwrap()
        .preview
        .host
        .clock()
        .is_none());
    live_config.states.push(PreviewStageState {
        owner: owner_anchor,
        group: "Interaction".into(),
        state: "Missing state".into(),
    });
    board.update(live_stage, live_config).unwrap();
    assert!(matches!(
        board.stage(live_stage).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    assert_eq!(visual(&board, live_stage, target), accepted_live);
    assert_eq!(board.stages[board.index(live_stage).unwrap()].rendered.as_ref().unwrap().preview.host.clock().unwrap().play_state(), PlayState::Pause,
        "The accepted stateless Live graph must stop even when the rejected configuration requested a scenario");
}

#[test]
fn invalid_configuration_and_manifest_are_atomic_and_imported_ids_never_reuse_retired_tokens() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document =
        Document::new(r#"<Button xmlns="https://github.com/ferroui" Name="A"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let id = board.add(config("One")).unwrap();
    let token = board
        .selection_token(id, runtime_id(&board, id, document.ast().root()))
        .unwrap();
    let before = board.manifest();
    let epoch = board.epoch();
    let mut invalid = config("Invalid");
    invalid.environment.width = f64::NAN;
    assert!(board.update(id, invalid).is_err());
    assert_eq!(board.manifest(), before);
    assert_eq!(board.epoch(), epoch);
    let mut invalid = before.clone();
    invalid.version = 2;
    assert!(board.restore_manifest(invalid).is_err());
    assert_eq!(board.manifest(), before);
    let serialized = before.to_json().unwrap();
    assert!(serialized.contains("\"kind\": \"document\""));
    assert!(serialized.contains("\"mode\": \"select\""));
    assert!(serialized.contains("\"theme\": \"light\""));
    let mut unknown: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    unknown["stages"][0]["unexpected"] = serde_json::json!(true);
    assert!(PreviewStageManifest::from_json(&unknown.to_string()).is_err());
    board
        .restore_manifest(PreviewStageManifest::from_json(&serialized).unwrap())
        .unwrap();
    let restored = board.active().unwrap();
    assert_ne!(restored, id);
    assert!(board.resolve_selection(&token).is_err());
    assert_eq!(board.manifest(), before);
    let mut too_many = before;
    too_many.stages = vec![config("Too many"); MAX_PREVIEW_STAGES + 1];
    assert!(board.restore_manifest(too_many).is_err());
    assert_eq!(board.active(), Some(restored));
}

#[test]
fn definition_stages_map_original_parts_and_keep_last_graph_when_the_target_is_removed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut document = Document::new(r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Grid.Resources><ControlTemplate x:Key="Card" TargetType="Button"><Border Name="Part" Background="Red"/></ControlTemplate></Grid.Resources></Grid>"#).unwrap();
    let definition = document
        .ast()
        .elements()
        .find(|node| node.local_name() == "ControlTemplate")
        .unwrap()
        .id;
    let part = document.ast().find_name("Part").unwrap();
    let mut configuration = config("Card definition");
    configuration.target = PreviewStageTarget::Definition {
        anchor: PreviewSourceAnchor::capture(&document, definition).unwrap(),
        options: DefinitionPreviewOptions::default(),
    };
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let stage = board.add(configuration).unwrap();
    let window = attach(&board);
    board.refresh(stage).unwrap();
    let old = visual(&board, stage, part);
    let token = board
        .selection_token(stage, runtime_id(&board, stage, part))
        .unwrap();
    assert_eq!(board.resolve_selection(&token).unwrap().node, part);
    assert!(board
        .runtime_tree(stage, TreeKind::Visual)
        .unwrap()
        .iter()
        .any(|row| row.source_id.is_none()));
    document
        .apply(
            "Remove definition",
            vec![Edit::RemoveNode { node: definition }],
        )
        .unwrap();
    sync(&mut board, &document, false);
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    assert_eq!(visual(&board, stage, part), old);
    assert!(board.resolve_selection(&token).is_err());
    window.close();
}

#[test]
fn failed_target_configuration_cannot_become_ready_by_pausing_and_resuming() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document =
        Document::new(r#"<Button xmlns="https://github.com/ferroui" Name="A"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let stage = board.add(config("Valid root")).unwrap();
    let previous = visual(&board, stage, document.ast().root());
    let mut invalid_target = config("Invalid definition");
    invalid_target.target = PreviewStageTarget::Definition {
        anchor: PreviewSourceAnchor::capture(&document, document.ast().root()).unwrap(),
        options: DefinitionPreviewOptions::default(),
    };
    board.update(stage, invalid_target).unwrap();
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    board.set_paused(stage, true).unwrap();
    board.set_paused(stage, false).unwrap();
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    assert_eq!(visual(&board, stage, document.ast().root()), previous);
}

#[test]
fn stage_removal_and_board_drop_release_native_graphs_and_source_publication_is_unchanged() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::with_uri(
        r#"<Button xmlns="https://github.com/ferroui"/>"#,
        ContentControl::new(),
        URI.into(),
    )
    .unwrap();
    let mut board = PreviewStageBoard::new();
    board
        .sync_sources(
            URI,
            vec![source(session.document(), URI, false)],
            session.preview_host_options(),
            session.preview_host_options_epoch(),
        )
        .unwrap();
    let first = board.add(config("Disposable")).unwrap();
    let second = board.duplicate(first).unwrap();
    let first_weak = visual(&board, first, session.document().ast().root()).downgrade();
    let second_weak = visual(&board, second, session.document().ast().root()).downgrade();
    let retained_card = board.control(first).unwrap();
    board.remove(first).unwrap();
    assert!(first_weak.upgrade().is_none());
    assert!(retained_card.cast::<Grid>().unwrap().children().is_empty());
    drop(board);
    assert!(second_weak.upgrade().is_none());
    let root = session.document().ast().root();
    session
        .apply(
            "Source remains owned",
            vec![Edit::SetAttribute {
                node: root,
                name: "Content".into(),
                value: "Still authoring".into(),
            }],
        )
        .unwrap();
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Ready { .. }
    ));
}

#[test]
fn host_options_epoch_changes_only_for_construction_services_not_focus_or_document_switches() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::with_uri(
        r#"<Button xmlns="https://github.com/ferroui"/>"#,
        ContentControl::new(),
        URI.into(),
    )
    .unwrap();
    let epoch = session.preview_host_options_epoch();
    session.set_design_time_enabled(false).unwrap();
    assert_eq!(session.preview_host_options_epoch(), epoch);
    session.select([session.document().ast().root()]);
    assert_eq!(session.preview_host_options_epoch(), epoch);
    session
        .open_document(
            r#"<TextBlock xmlns="https://github.com/ferroui"/>"#.into(),
            OTHER.into(),
        )
        .unwrap();
    assert_eq!(session.preview_host_options_epoch(), epoch);
    session
        .configure_preview(PreviewHostOptions::default())
        .unwrap();
    assert_ne!(session.preview_host_options_epoch(), epoch);
}

#[test]
fn retargeting_save_as_retires_source_tokens_and_reloads_using_the_new_base_uri() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(r#"<Button xmlns="https://github.com/ferroui"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let stage = board.add(config("Saved view")).unwrap();
    let token = board
        .selection_token(stage, runtime_id(&board, stage, document.ast().root()))
        .unwrap();
    board.retarget_source(URI, OTHER).unwrap();
    assert!(board.resolve_selection(&token).is_err());
    board
        .sync_sources(
            OTHER,
            vec![source(&document, OTHER, false)],
            PreviewHostOptions::default(),
            1,
        )
        .unwrap();
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    let new_token = board
        .selection_token(stage, runtime_id(&board, stage, document.ast().root()))
        .unwrap();
    assert_eq!(
        board.resolve_selection(&new_token).unwrap().source_uri,
        OTHER
    );
    assert_eq!(
        board.stages[board.index(stage).unwrap()]
            .rendered
            .as_ref()
            .unwrap()
            .preview
            .uri(),
        OTHER
    );
}

#[test]
fn readonly_current_source_check_rejects_unsynchronized_drafts_and_new_ast_without_building() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document =
        Document::new(r#"<Button xmlns="https://github.com/ferroui" Content="One"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let stage = board.add(config("Read current source")).unwrap();
    let epoch = board.epoch();
    let native = visual(&board, stage, document.ast().root());
    assert!(board.source_is_current(stage, URI, &document, false, 1));
    assert!(!board.source_is_current(stage, URI, &document, true, 1));
    assert!(!board.source_is_current(stage, URI, &document, false, 2));
    assert!(!board.source_is_current(stage, OTHER, &document, false, 1));
    let unrelated = Document::new(document.source()).unwrap();
    assert!(!board.source_is_current(stage, URI, &unrelated, false, 1));
    let mut draft = document.clone();
    draft.update_source(
        "Unapplied",
        r#"<Button xmlns="https://github.com/ferroui" Content="Two"/>"#,
    );
    assert!(!board.source_is_current(stage, URI, &draft, false, 1));
    assert_eq!(board.epoch(), epoch);
    assert_eq!(visual(&board, stage, document.ast().root()), native);
}

#[test]
fn document_hit_testing_selects_template_owner_and_definition_stage_selects_the_authored_part() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Grid.Resources><ControlTemplate x:Key="Card" TargetType="Button"><Border Name="Part" Background="Red"/></ControlTemplate></Grid.Resources><Button Name="Instance" Template="{StaticResource Card}" Width="160" Height="80"/></Grid>"#).unwrap();
    let part = document.ast().find_name("Part").unwrap();
    let instance = document.ast().find_name("Instance").unwrap();
    let definition = document
        .ast()
        .elements()
        .find(|node| node.local_name() == "ControlTemplate")
        .unwrap()
        .id;
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let ordinary = board.add(config("Whole document")).unwrap();
    let mut definition_config = config("Definition");
    definition_config.target = PreviewStageTarget::Definition {
        anchor: PreviewSourceAnchor::capture(&document, definition).unwrap(),
        options: DefinitionPreviewOptions::default(),
    };
    let definition_stage = board.add(definition_config).unwrap();
    let window = attach(&board);
    board.refresh_runtime_mappings();
    let native_part = visual(&board, ordinary, part);
    assert!(native_part.templated_parent().is_some());
    let point = native_part
        .translate_point(
            Point::new(
                native_part.bounds().width / 2.0,
                native_part.bounds().height / 2.0,
            ),
            &board.control(ordinary).unwrap(),
        )
        .unwrap();
    let token = board.hit_test_selection(ordinary, point).unwrap();
    assert_eq!(board.resolve_selection(&token).unwrap().node, instance);
    assert!(board
        .selection_token(ordinary, runtime_id(&board, ordinary, part))
        .is_err());
    assert!(board
        .runtime_tree(ordinary, TreeKind::Visual)
        .unwrap()
        .iter()
        .all(|row| row.source_id != Some(part)));
    let part_token = board
        .selection_token(definition_stage, runtime_id(&board, definition_stage, part))
        .unwrap();
    assert_eq!(board.resolve_selection(&part_token).unwrap().node, part);
    window.close();
}
