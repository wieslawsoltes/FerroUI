use super::*;
use crate::DesignerSession;
use ferroui_base::layout::Orientation;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, Grid, StackPanel, TextBlock, TextBox, Window};
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

#[test]
fn fitting_policies_use_native_desired_size_with_finite_logical_proposals() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document =
        Document::new(r#"<Border xmlns="https://github.com/ferroui" Width="120" Height="70"/>"#)
            .unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let mut ids = Vec::new();
    for layout in [
        PreviewStageLayout::Fixed,
        PreviewStageLayout::SizeThatFits,
        PreviewStageLayout::FixedWidth,
        PreviewStageLayout::FixedHeight,
    ] {
        let mut config = config(&format!("{layout:?}"));
        config.layout = layout;
        config.environment.width = 300.0;
        config.environment.height = 200.0;
        ids.push(board.add(config).unwrap());
    }
    let window = attach(&board);
    for (id, (width, height)) in
        ids.into_iter()
            .zip([(300.0, 200.0), (120.0, 70.0), (300.0, 70.0), (120.0, 200.0)])
    {
        assert_eq!(
            board.stage(id).unwrap().layout_size,
            PreviewStageSize { width, height }
        );
        let control = board.control(id).unwrap();
        assert_eq!(control.bounds().width, width);
        assert_eq!(control.bounds().height, height);
        let root = visual(&board, id, document.ast().root())
            .cast::<Border>()
            .unwrap();
        assert_eq!(root.width(), 120.0);
        assert_eq!(root.height(), 70.0);
        assert_eq!(
            root.previous_measure(),
            Some(ferroui_base::Size::new(300.0, 200.0))
        );
    }
    assert_eq!(document.revision(), 0);
    window.close();
}

#[test]
fn fit_clamps_empty_and_oversized_content_without_rewriting_authored_constraints() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document =
        Document::new(r#"<Border xmlns="https://github.com/ferroui" Width="900" Height="300"/>"#)
            .unwrap();
    let empty = Document::new(r#"<Border xmlns="https://github.com/ferroui"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync_documents(
        &mut board,
        &[(URI, &document, false), (OTHER, &empty, false)],
    );
    let mut large = config("Overflow");
    large.layout = PreviewStageLayout::SizeThatFits;
    large.environment.width = 160.0;
    large.environment.height = 200.0;
    let large_id = board.add(large).unwrap();
    let mut small = config("Empty");
    small.source_uri = OTHER.into();
    small.layout = PreviewStageLayout::SizeThatFits;
    let small_id = board.add(small).unwrap();
    let window = attach(&board);
    assert_eq!(
        board.stage(large_id).unwrap().layout_size,
        PreviewStageSize {
            width: 160.0,
            height: 200.0
        }
    );
    assert_eq!(
        board.stage(small_id).unwrap().layout_size,
        PreviewStageSize {
            width: 1.0,
            height: 1.0
        }
    );
    let root = visual(&board, large_id, document.ast().root())
        .cast::<Border>()
        .unwrap();
    assert_eq!(root.width(), 900.0);
    assert_eq!(root.height(), 300.0);
    assert!(board.control(large_id).unwrap().clip_to_bounds());
    window.close();
}

#[test]
fn fixed_width_fits_wrapped_text_height_using_the_real_width_proposal() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(
        r#"<TextBlock xmlns="https://github.com/ferroui" FontSize="16" TextWrapping="Wrap" Text="Preview layouts need enough width to place a paragraph. Narrow previews wrap the same native text into more lines."/>"#,
    )
    .unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let mut narrow = config("Narrow paragraph");
    narrow.layout = PreviewStageLayout::FixedWidth;
    narrow.environment.width = 100.0;
    narrow.environment.height = 800.0;
    let narrow_id = board.add(narrow.clone()).unwrap();
    narrow.name = "Wide paragraph".into();
    narrow.environment.width = 300.0;
    let wide_id = board.add(narrow).unwrap();
    let window = attach(&board);
    let narrow_size = board.stage(narrow_id).unwrap().layout_size;
    let wide_size = board.stage(wide_id).unwrap().layout_size;
    assert_eq!(narrow_size.width, 100.0);
    assert_eq!(wide_size.width, 300.0);
    assert!(
        narrow_size.height > wide_size.height,
        "narrow={narrow_size:?}, wide={wide_size:?}"
    );
    assert!(wide_size.height > 1.0);
    assert!(narrow_size.height < 800.0);
    window.close();
}

#[test]
fn fitting_tracks_native_content_invalidation_without_rebuilding_graph_or_epoch() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(
        r#"<StackPanel xmlns="https://github.com/ferroui"><Border Width="100" Height="20"/></StackPanel>"#,
    )
    .unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let mut configuration = config("Live fitting content");
    configuration.layout = PreviewStageLayout::SizeThatFits;
    configuration.mode = PreviewStageMode::Live;
    let id = board.add(configuration).unwrap();
    let window = attach(&board);
    let original = board.stage(id).unwrap();
    assert_eq!(
        original.layout_size,
        PreviewStageSize {
            width: 100.0,
            height: 20.0
        }
    );
    let root = visual(&board, id, document.ast().root())
        .cast::<StackPanel>()
        .unwrap();
    let added = Border::new();
    added.set_width(160.0);
    added.set_height(40.0);
    root.children().add(added.upcast::<Control>());
    window.update_layout();
    let changed = board.stage(id).unwrap();
    assert_eq!(
        changed.layout_size,
        PreviewStageSize {
            width: 160.0,
            height: 60.0
        }
    );
    assert_eq!(changed.epoch, original.epoch);
    assert_eq!(changed.configuration, original.configuration);
    assert_eq!(
        root,
        visual(&board, id, document.ast().root())
            .cast::<StackPanel>()
            .unwrap()
    );
    assert_eq!(document.revision(), 0);
    window.close();
}

#[test]
fn logical_resize_and_layout_policy_preserve_live_input_and_retire_old_tokens() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document =
        Document::new(r#"<TextBox xmlns="https://github.com/ferroui" Text="Initial"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let mut configuration = config("Resizable live input");
    configuration.mode = PreviewStageMode::Live;
    let id = board.add(configuration.clone()).unwrap();
    let window = attach(&board);
    let root = visual(&board, id, document.ast().root())
        .cast::<TextBox>()
        .unwrap();
    root.set_text(Some("Keep this typed text"));
    let token = board
        .selection_token(id, runtime_id(&board, id, document.ast().root()))
        .unwrap();
    configuration.environment.width = 320.0;
    configuration.environment.height = 240.0;
    board.update(id, configuration.clone()).unwrap();
    assert_eq!(
        root,
        visual(&board, id, document.ast().root())
            .cast::<TextBox>()
            .unwrap()
    );
    assert_eq!(root.text().as_deref(), Some("Keep this typed text"));
    assert!(board.resolve_selection(&token).is_err());
    assert_eq!(
        board.stage(id).unwrap().layout_size,
        PreviewStageSize {
            width: 320.0,
            height: 240.0
        }
    );
    configuration.layout = PreviewStageLayout::FixedWidth;
    board.update(id, configuration).unwrap();
    assert_eq!(root.text().as_deref(), Some("Keep this typed text"));
    assert_eq!(board.stage(id).unwrap().layout_size.width, 320.0);
    assert!(board.stage(id).unwrap().layout_size.height < 240.0);
    let fresh = board
        .selection_token(id, runtime_id(&board, id, document.ast().root()))
        .unwrap();
    assert_eq!(
        board.resolve_selection(&fresh).unwrap().node,
        document.ast().root()
    );
    assert_eq!(document.revision(), 0);
    window.close();
}

#[test]
fn a_panicking_native_resize_hook_keeps_its_graph_without_claiming_the_new_layout() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(r#"<Border xmlns="https://github.com/ferroui"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let mut configuration = config("Application layout callback");
    configuration.mode = PreviewStageMode::Live;
    let stage = board.add(configuration.clone()).unwrap();
    let window = attach(&board);
    let root = visual(&board, stage, document.ast().root());
    let subscription = root.property_changed(|change| {
        if change.is_effective_value_change()
            && change.property() == Visual::bounds_property().as_property()
        {
            panic!("Application resize hook failed");
        }
    });
    configuration.environment.width = 220.0;
    configuration.environment.height = 120.0;
    board.update(stage, configuration).unwrap();
    subscription.dispose();
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Failed { ref message, .. }
            if message.contains("Application resize hook failed")
    ));
    assert_eq!(visual(&board, stage, document.ast().root()), root);
    assert!(!board.source_is_current(stage, URI, &document, false, 1));
    assert!(!board.frame_is_current(stage, URI, &document, false, 1));
    assert!(!board.stages[0]
        .rendered
        .as_ref()
        .unwrap()
        .scope
        .is_hit_test_visible());
    board.set_paused(stage, true).unwrap();
    assert!(!board.frame_is_current(stage, URI, &document, false, 1));
    board.set_paused(stage, false).unwrap();
    assert!(board.frame_is_current(stage, URI, &document, false, 1));
    assert!(board.source_is_current(stage, URI, &document, false, 1));
    assert_ne!(visual(&board, stage, document.ast().root()), root);
    assert_eq!(document.revision(), 0);
    window.close();
}

#[test]
fn layout_changes_keep_pending_and_paused_source_authority_revoked() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut document =
        Document::new(r#"<Border xmlns="https://github.com/ferroui" Width="120" Height="70"/>"#)
            .unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let id = board.add(config("Retained result")).unwrap();
    let window = attach(&board);
    let original = visual(&board, id, document.ast().root());
    document.update_source("Draft", document.source().replace("120", "150"));
    sync(&mut board, &document, true);
    let mut configuration = board.stage(id).unwrap().configuration;
    configuration.layout = PreviewStageLayout::SizeThatFits;
    board.update(id, configuration).unwrap();
    assert!(matches!(
        board.stage(id).unwrap().status,
        PreviewStageStatus::PendingSource { .. }
    ));
    assert_eq!(
        visual(
            &board,
            id,
            board.stages[0]
                .rendered
                .as_ref()
                .unwrap()
                .document
                .ast()
                .root()
        ),
        original
    );
    assert_eq!(
        board.stage(id).unwrap().layout_size,
        PreviewStageSize {
            width: 120.0,
            height: 70.0
        }
    );
    assert!(board
        .selection_token(id, Inspector::shared().borrow_mut().node_id(&original))
        .is_err());
    board.set_paused(id, true).unwrap();
    let mut configuration = board.stage(id).unwrap().configuration;
    configuration.environment.width = 100.0;
    board.update(id, configuration).unwrap();
    assert!(matches!(
        board.stage(id).unwrap().status,
        PreviewStageStatus::Paused { .. }
    ));
    assert_eq!(board.stage(id).unwrap().layout_size.width, 100.0);
    assert!(board
        .selection_token(id, Inspector::shared().borrow_mut().node_id(&original))
        .is_err());
    window.close();
}

#[test]
fn old_manifests_default_to_fixed_layout_and_new_policies_round_trip() {
    let mut value = serde_json::to_value(config("Legacy")).unwrap();
    value.as_object_mut().unwrap().remove("layout");
    let legacy: PreviewStageConfiguration = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(legacy.layout, PreviewStageLayout::Fixed);
    for layout in [
        PreviewStageLayout::SizeThatFits,
        PreviewStageLayout::FixedWidth,
        PreviewStageLayout::FixedHeight,
    ] {
        let mut configuration = legacy.clone();
        configuration.layout = layout;
        let manifest = PreviewStageManifest {
            version: 1,
            stages: vec![configuration],
            active: Some(0),
            prototype: Default::default(),
        };
        assert_eq!(
            PreviewStageManifest::from_json(&manifest.to_json().unwrap()).unwrap(),
            manifest
        );
    }
    value["layout"] = serde_json::json!("fit_using_global_font_scale");
    assert!(serde_json::from_value::<PreviewStageConfiguration>(value).is_err());
}

#[test]
fn paused_current_frames_can_be_observed_without_restoring_selection_or_input_authority() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut document =
        Document::new(r#"<TextBox xmlns="https://github.com/ferroui" Text="Initial"/>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let mut configuration = config("Paused image");
    configuration.mode = PreviewStageMode::Live;
    let stage = board.add(configuration).unwrap();
    let window = attach(&board);
    assert!(board.source_is_current(stage, URI, &document, false, 1));
    assert!(board.frame_is_current(stage, URI, &document, false, 1));
    board.set_paused(stage, true).unwrap();
    assert!(!board.source_is_current(stage, URI, &document, false, 1));
    assert!(board.frame_is_current(stage, URI, &document, false, 1));
    assert!(!board.stages[0]
        .rendered
        .as_ref()
        .unwrap()
        .scope
        .is_hit_test_visible());
    let node = document.ast().root();
    assert!(board
        .selection_token(stage, runtime_id(&board, stage, node))
        .is_err());
    assert!(!board.frame_is_current(stage, URI, &document, true, 1));
    assert!(!board.frame_is_current(stage, OTHER, &document, false, 1));
    assert!(!board.frame_is_current(stage, URI, &document, false, 2));
    let replacement = Document::new(document.source()).unwrap();
    assert!(!board.frame_is_current(stage, URI, &replacement, false, 1));
    document.update_source(
        "Change text",
        document.source().replace("Initial", "Accepted later"),
    );
    assert!(!board.frame_is_current(stage, URI, &document, false, 1));
    sync(&mut board, &document, false);
    assert!(!board.frame_is_current(stage, URI, &document, false, 1));
    board.refresh(stage).unwrap();
    assert!(board.frame_is_current(stage, URI, &document, false, 1));
    assert!(!board.source_is_current(stage, URI, &document, false, 1));
    assert!(!board.stages[0]
        .rendered
        .as_ref()
        .unwrap()
        .scope
        .is_hit_test_visible());
    let current = board.stage(stage).unwrap();
    let mut invalid_target = current.configuration;
    invalid_target.target = PreviewStageTarget::Definition {
        anchor: PreviewSourceAnchor::capture(&document, document.ast().root()).unwrap(),
        options: DefinitionPreviewOptions::default(),
    };
    board.update(stage, invalid_target).unwrap();
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    assert!(!board.frame_is_current(stage, URI, &document, false, 1));
    window.close();
}

#[test]
fn comparison_batch_validates_every_descriptor_before_calling_native_factories() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = Document::new(r#"<Border xmlns="https://github.com/ferroui"/>"#).unwrap();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let mut options = PreviewHostOptions::default();
    options.register_root(
        "Border",
        Rc::new(move |request| {
            observed.set(observed.get() + 1);
            request.populate(None)
        }),
    );
    let mut board = PreviewStageBoard::new();
    board
        .sync_sources(URI, vec![source(&document, URI, false)], options, 1)
        .unwrap();
    let mut invalid = config("Invalid last item");
    invalid.environment.width = f64::NAN;
    assert!(board
        .add_configurations(vec![config("First"), invalid])
        .is_err());
    assert_eq!(calls.get(), 0);
    assert!(board.stages().is_empty());
    assert!(board
        .add_configurations(vec![config("Too many"); MAX_PREVIEW_STAGES + 1])
        .is_err());
    assert!(board.add_configurations(Vec::new()).is_err());
    assert_eq!(calls.get(), 0);
    let ids = board
        .add_configurations(vec![config("One"), config("Two")])
        .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(ids.len(), 2);
    assert_ne!(
        visual(&board, ids[0], document.ast().root()),
        visual(&board, ids[1], document.ast().root())
    );
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

fn sync_documents(board: &mut PreviewStageBoard, documents: &[(&str, &Document, bool)]) {
    board
        .sync_sources(
            URI,
            documents
                .iter()
                .map(|(uri, document, pending)| source(document, uri, *pending))
                .collect(),
            PreviewHostOptions::default(),
            1,
        )
        .unwrap();
}

#[test]
fn transitive_resource_and_style_dependencies_refresh_only_their_native_stages() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    const OUTER: &str = "ferres://FerroUI.Designer.Tests/Outer.xaml";
    const VALUES: &str = "ferres://FerroUI.Designer.Tests/Values.xaml";
    const STYLES: &str = "ferres://FerroUI.Designer.Tests/Styles.xaml";
    let document = Document::new(r#"<StackPanel xmlns="https://github.com/ferroui"><StackPanel.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source="Outer.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary></StackPanel.Resources><StackPanel.Styles><StyleInclude Source="Styles.xaml"/></StackPanel.Styles><TextBox Name="Input" Width="{StaticResource width}" Text="Initial"/></StackPanel>"#).unwrap();
    let outer = Document::new(r#"<ResourceDictionary xmlns="https://github.com/ferroui"><ResourceDictionary.MergedDictionaries><ResourceInclude Source="Values.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"#).unwrap();
    let mut values = Document::new(r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">70</x:Double></ResourceDictionary>"#).unwrap();
    let mut styles = Document::new(r#"<Styles xmlns="https://github.com/ferroui"><Style Selector="TextBox"><Setter Property="Height" Value="32"/></Style></Styles>"#).unwrap();
    let unrelated =
        Document::new(r#"<TextBox xmlns="https://github.com/ferroui" Text="Other initial"/>"#)
            .unwrap();
    let input = document.ast().find_name("Input").unwrap();
    let mut board = PreviewStageBoard::new();
    sync_documents(
        &mut board,
        &[
            (URI, &document, false),
            (OUTER, &outer, false),
            (VALUES, &values, false),
            (STYLES, &styles, false),
            (OTHER, &unrelated, false),
        ],
    );
    let dependent = board.add(config("Included resources")).unwrap();
    let mut other_config = PreviewStageConfiguration::new(
        "Independent live input",
        OTHER,
        PreviewEnvironment::default(),
    );
    other_config.mode = PreviewStageMode::Live;
    let independent = board.add(other_config).unwrap();
    let window = attach(&board);
    assert!(board.refresh_all().is_empty(), "{:?}", board.stages());
    let old_input = visual(&board, dependent, input).cast::<TextBox>().unwrap();
    let other_input = visual(&board, independent, unrelated.ast().root())
        .cast::<TextBox>()
        .unwrap();
    other_input.set_text(Some("Keep this live state"));
    assert_eq!(old_input.width(), 70.0);
    assert_eq!(old_input.height(), 32.0);
    let token = board
        .selection_token(dependent, runtime_id(&board, dependent, input))
        .unwrap();
    values.update_source(
        "Change included width",
        values.source().replace(">70<", ">92<"),
    );
    let fresh = vec![
        source(&document, URI, false),
        source(&outer, OUTER, false),
        source(&values, VALUES, false),
        source(&styles, STYLES, false),
        source(&unrelated, OTHER, false),
    ];
    assert!(
        !board.dependencies_are_current(dependent, &fresh),
        "read-only validation must reject a pre-sync dependency edit"
    );
    assert!(board.dependencies_are_current(independent, &fresh));
    sync_documents(
        &mut board,
        &[
            (URI, &document, false),
            (OUTER, &outer, false),
            (VALUES, &values, false),
            (STYLES, &styles, false),
            (OTHER, &unrelated, false),
        ],
    );
    let changed = visual(&board, dependent, input).cast::<TextBox>().unwrap();
    assert_ne!(old_input, changed);
    assert_eq!(changed.width(), 92.0);
    assert!(board.resolve_selection(&token).is_err());
    assert_eq!(
        visual(&board, independent, unrelated.ast().root())
            .cast::<TextBox>()
            .unwrap(),
        other_input
    );
    assert_eq!(other_input.text().as_deref(), Some("Keep this live state"));
    styles.update_source(
        "Change included style",
        styles.source().replace("Value=\"32\"", "Value=\"48\""),
    );
    sync_documents(
        &mut board,
        &[
            (URI, &document, false),
            (OUTER, &outer, false),
            (VALUES, &values, false),
            (STYLES, &styles, false),
            (OTHER, &unrelated, false),
        ],
    );
    assert_eq!(
        visual(&board, dependent, input)
            .cast::<TextBox>()
            .unwrap()
            .height(),
        48.0
    );
    assert_eq!(
        visual(&board, independent, unrelated.ast().root())
            .cast::<TextBox>()
            .unwrap(),
        other_input
    );
    assert_eq!(other_input.text().as_deref(), Some("Keep this live state"));
    assert_eq!(document.revision(), 0);
    assert!(!document.can_undo());
    window.close();
}

#[test]
fn included_drafts_errors_and_late_open_documents_keep_the_last_graph_and_retire_source_tokens() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    const VALUES: &str = "ferres://FerroUI.Designer.Tests/MissingValues.xaml";
    let document = Document::new(r#"<StackPanel xmlns="https://github.com/ferroui"><StackPanel.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source="MissingValues.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary></StackPanel.Resources><TextBox Name="Input" Width="{StaticResource width}"/></StackPanel>"#).unwrap();
    let node = document.ast().find_name("Input").unwrap();
    let valid = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">61</x:Double></ResourceDictionary>"#;
    let mut values = Document::new(valid).unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, false);
    let stage = board.add(config("Late include")).unwrap();
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    sync_documents(
        &mut board,
        &[(URI, &document, false), (VALUES, &values, false)],
    );
    assert!(
        matches!(
            board.stage(stage).unwrap().status,
            PreviewStageStatus::Ready { .. }
        ),
        "{:?}",
        board.stage(stage)
    );
    let accepted = visual(&board, stage, node).cast::<TextBox>().unwrap();
    assert_eq!(accepted.width(), 61.0);
    let token = board
        .selection_token(stage, runtime_id(&board, stage, node))
        .unwrap();
    values.update_source("Pending width", valid.replace(">61<", ">82<"));
    sync_documents(
        &mut board,
        &[(URI, &document, false), (VALUES, &values, true)],
    );
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::PendingSource { .. }
    ));
    assert_eq!(
        visual(&board, stage, node).cast::<TextBox>().unwrap(),
        accepted
    );
    assert_eq!(accepted.width(), 61.0);
    assert!(board.resolve_selection(&token).is_err());
    let epoch = board.epoch();
    sync_documents(
        &mut board,
        &[(URI, &document, false), (VALUES, &values, true)],
    );
    assert_eq!(
        board.epoch(),
        epoch,
        "an unchanged unavailable include must not retry every refresh"
    );
    values.update_source("Invalid included source", "<ResourceDictionary");
    sync_documents(
        &mut board,
        &[(URI, &document, false), (VALUES, &values, false)],
    );
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::InvalidSource { .. }
    ));
    assert_eq!(
        visual(&board, stage, node).cast::<TextBox>().unwrap(),
        accepted
    );
    values.update_source(
        "Invalid native resource",
        "<DoesNotExist xmlns='https://github.com/ferroui'/>",
    );
    sync_documents(
        &mut board,
        &[(URI, &document, false), (VALUES, &values, false)],
    );
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    assert_eq!(
        visual(&board, stage, node).cast::<TextBox>().unwrap(),
        accepted
    );
    values.update_source("Repair resource", valid.replace(">61<", ">94<"));
    sync_documents(
        &mut board,
        &[(URI, &document, false), (VALUES, &values, false)],
    );
    assert_eq!(
        visual(&board, stage, node)
            .cast::<TextBox>()
            .unwrap()
            .width(),
        94.0
    );
    assert!(matches!(
        board.stage(stage).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    assert_eq!(document.revision(), 0);
}

#[test]
fn deferred_include_discovery_uses_the_graphs_accepted_source_stamps_and_snapshot() {
    use ferroui_base::metadata::IServiceProvider;
    use ferroui_markup_xaml::xaml_il::runtime::{
        DeferredContentBuilder, FrameworkContextServices, XamlIlContext, XamlIlContextDefinition,
        XamlIlRuntimeHelpers,
    };
    use ferroui_markup_xaml::{styling::ResourceInclude, templates::DataTemplate};

    // A host factory may supply deferred native content whose include is created
    // only when the template is invoked. Interpreted XAML eagerly discovers its
    // static include paths during compilation, so use the same native builder API
    // as compiled/custom templates to exercise genuinely late discovery.
    fn template(
        board: &PreviewStageBoard,
        stage: PreviewStageId,
        root: &Ref<StackPanel>,
    ) -> Rc<DataTemplate> {
        let snapshot = board.stages[board.index(stage).unwrap()]
            .rendered
            .as_ref()
            .unwrap()
            .source_snapshot
            .clone();
        let context = XamlIlContext::new(
            XamlIlContextDefinition {
                root_object_provider: true,
                parent_stack_provider: true,
                ..Default::default()
            },
            Rc::new(FrameworkContextServices),
            None,
            Rc::from([]),
            Some(Uri::absolute(URI).unwrap()),
        );
        context.set_root_object(Some(Control::boxed(root.clone())));
        context.push_parent(Some(Control::boxed(root.clone())));
        let provider: Rc<dyn IServiceProvider> = context;
        snapshot.run(|| {
            let builder = DeferredContentBuilder::try_new(|_| {
                let include = ResourceInclude::new(Some(Uri::absolute(URI).unwrap()));
                include.set_source(Uri::try_create(
                    "DeferredValues.xaml",
                    UriKind::RelativeOrAbsolute,
                ));
                let dictionary = include.try_loaded()?;
                let width = dictionary.try_get_value(&"width".into()).flatten().unwrap();
                let child = TextBox::new();
                child.set_width(*width.downcast_ref::<f64>().unwrap());
                let border = Border::new();
                border.set_child(Some(child.upcast()));
                Ok(Some(Control::boxed(border)))
            });
            let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(
                builder, &provider,
            );
            let template = DataTemplate::new();
            template.set_content(Some(content));
            template
        })
    }
    let _app = UnitTestApplication::start(TestServices::styled_window());
    const VALUES: &str = "ferres://FerroUI.Designer.Tests/DeferredValues.xaml";
    let document = Document::new(r#"<StackPanel xmlns="https://github.com/ferroui"><TextBox Name="Input" Text="Retain until dependency is discovered"/></StackPanel>"#).unwrap();
    let mut values = Document::new(r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">51</x:Double></ResourceDictionary>"#).unwrap();
    let mut board = PreviewStageBoard::new();
    sync_documents(
        &mut board,
        &[(URI, &document, false), (VALUES, &values, false)],
    );
    let stage = board.add(config("Deferred sources")).unwrap();
    assert!(
        matches!(
            board.stage(stage).unwrap().status,
            PreviewStageStatus::Ready { .. }
        ),
        "{:?}",
        board.stage(stage)
    );
    let accepted = visual(&board, stage, document.ast().root())
        .cast::<StackPanel>()
        .unwrap();
    let deferred = template(&board, stage, &accepted);
    assert!(!board.stages[board.index(stage).unwrap()]
        .rendered
        .as_ref()
        .unwrap()
        .source_snapshot
        .requested_uris()
        .contains(&native_source_uri(VALUES).unwrap()));
    values.update_source(
        "Edit before first template realization",
        values.source().replace(">51<", ">89<"),
    );
    sync_documents(
        &mut board,
        &[(URI, &document, false), (VALUES, &values, false)],
    );
    assert_eq!(
        visual(&board, stage, document.ast().root())
            .cast::<StackPanel>()
            .unwrap(),
        accepted,
        "an unconsumed resource cannot restart unrelated native content"
    );
    let built = deferred.build(None).unwrap().cast::<Border>().unwrap();
    assert_eq!(
        built.child().unwrap().width(),
        51.0,
        "a retained deferred template must use its accepted source snapshot"
    );
    assert!(!board.source_is_current(stage, URI, &document, false, 1));
    sync_documents(
        &mut board,
        &[(URI, &document, false), (VALUES, &values, false)],
    );
    let refreshed = visual(&board, stage, document.ast().root())
        .cast::<StackPanel>()
        .unwrap();
    assert_ne!(
        refreshed, accepted,
        "late discovery must compare with construction-time stamps"
    );
    let deferred = template(&board, stage, &refreshed);
    let built = deferred.build(None).unwrap().cast::<Border>().unwrap();
    assert_eq!(built.child().unwrap().width(), 89.0);
    assert_eq!(document.revision(), 0);
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
    invalid.version = 4;
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
