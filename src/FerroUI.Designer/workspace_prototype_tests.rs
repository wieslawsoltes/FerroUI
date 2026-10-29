use super::*;
use crate::{
    PreviewEnvironment, PreviewPrototypeAction, PreviewPrototypeFlow, PreviewPrototypeLink,
    PreviewStageConfiguration, PreviewStageMode, PreviewStageState, PreviewStageStatus,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{Ref, Size};
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::testing::{
    MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
};
use ferroui_controls::{Button, CheckBox, ComboBox, Control, TextBox, Window};
use std::sync::Arc;

const SOURCE: &str = include_str!("../../docs/designer/samples/PrototypeFlow.xaml");
const URI: &str = "memory://prototype/PrototypeFlow.xaml";

fn services() -> TestServices {
    let mut services = TestServices::styled_window();
    let focus = TestServices::real_focus();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let native = MockWindowImpl::window(1400.0, 1000.0);
        native.max_auto_size_hint.set(Size::new(2000.0, 1600.0));
        native.setup_create_popup(|_| None);
        native.setup_resize(|window, size, reason| {
            if window.client_size.get() != size {
                window.client_size.set(size);
                if let Some(resized) = ITopLevelImpl::resized(window) {
                    resized(size, reason);
                }
            }
        });
        native
    }));
    services
}

fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing prototype control {name}"))
}

fn native<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing native prototype element {name}"))
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

fn attach(workspace: &Rc<DesignerWorkspace>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1400.0);
    window.set_height(1000.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    window
}

struct Fixture {
    workspace: Rc<DesignerWorkspace>,
    window: Ref<Window>,
    home: PreviewStageId,
    details: PreviewStageId,
    settings: PreviewStageId,
}

impl Fixture {
    fn new() -> Self {
        let workspace = DesignerWorkspace::with_uri(SOURCE, URI.into()).unwrap();
        workspace.set_timeline_visible(false);
        let window = attach(&workspace);
        workspace.set_preview_stages_visible(true).unwrap();
        let document = workspace
            .session
            .borrow()
            .document()
            .snapshot_without_history();
        let owner = PreviewSourceAnchor::capture(
            &document,
            document.ast().find_name("PrototypeScene").unwrap(),
        )
        .unwrap();
        let home = workspace.preview_stages()[0].id;
        let mut configuration = workspace.preview_stage(home).unwrap().configuration;
        configuration.name = "Home".into();
        configuration.mode = PreviewStageMode::Live;
        configuration.environment.width = 390.0;
        configuration.environment.height = 700.0;
        configuration.states = vec![PreviewStageState {
            owner,
            group: "Screen".into(),
            state: "Home".into(),
        }];
        workspace
            .update_preview_stage(home, configuration.clone())
            .unwrap();
        configuration.name = "Details".into();
        configuration.states[0].state = "Details".into();
        let details = workspace.add_preview_stage(configuration.clone()).unwrap();
        configuration.name = "Settings".into();
        configuration.states[0].state = "Settings".into();
        let settings = workspace.add_preview_stage(configuration).unwrap();
        let link = |name: &str, source, control, action| PreviewPrototypeLink {
            name: name.into(),
            source,
            trigger: PreviewSourceAnchor::capture(
                &document,
                document.ast().find_name(control).unwrap(),
            )
            .unwrap(),
            action,
        };
        workspace
            .set_preview_prototype(PreviewPrototype {
                flows: vec![
                    PreviewPrototypeFlow {
                        name: "Explore".into(),
                        start: home,
                    },
                    PreviewPrototypeFlow {
                        name: "Settings review".into(),
                        start: settings,
                    },
                ],
                links: vec![
                    link(
                        "Open project",
                        home,
                        "HomeOpenDetails",
                        PreviewPrototypeAction::Navigate { stage: details },
                    ),
                    link(
                        "Workspace settings",
                        home,
                        "HomeOpenSettings",
                        PreviewPrototypeAction::Navigate { stage: settings },
                    ),
                    link(
                        "Project settings",
                        details,
                        "DetailsOpenSettings",
                        PreviewPrototypeAction::Navigate { stage: settings },
                    ),
                    link(
                        "Back from details",
                        details,
                        "DetailsBack",
                        PreviewPrototypeAction::Back,
                    ),
                    link(
                        "Back from settings",
                        settings,
                        "SettingsBack",
                        PreviewPrototypeAction::Back,
                    ),
                ],
            })
            .unwrap();
        let fixture = Self {
            workspace,
            window,
            home,
            details,
            settings,
        };
        fixture.layout();
        for stage in fixture.workspace.preview_stages() {
            assert!(
                matches!(stage.status, PreviewStageStatus::Ready { .. }),
                "{}: {:?}",
                stage.configuration.name,
                stage.status
            );
        }
        fixture
    }

    fn layout(&self) {
        self.window.update_layout();
        self.workspace.refresh_preview_stages();
        self.window.update_layout();
    }

    fn start(&self, flow: &str) {
        self.workspace.start_preview_prototype(flow).unwrap();
        self.layout();
        assert!(
            self.workspace
                .preview_prototype_playback()
                .unwrap()
                .current_source
        );
    }

    fn hotspot(&self, index: usize) -> Ref<Button> {
        named(
            &self
                .workspace
                .preview_prototype_player_view()
                .unwrap()
                .control(),
            &format!("PrototypeHotspot{index}"),
        )
    }

    fn activate(&self, index: usize, expected: PreviewStageId) {
        self.layout();
        click(&self.hotspot(index));
        self.layout();
        let playback = self.workspace.preview_prototype_playback().unwrap();
        assert_eq!(playback.current, expected, "{:?}", playback.diagnostics);
    }

    fn input<T: ferroui_base::ObjectType>(&self, stage: PreviewStageId, name: &str) -> Ref<T> {
        native(
            &self.workspace.preview_prototype_control(stage).unwrap(),
            name,
        )
    }

    fn overview_anchor(&self) -> Ref<Button> {
        self.workspace.set_preview_prototype_visible(true).unwrap();
        self.layout();
        named(
            &self.workspace.preview_prototype_view().unwrap().control(),
            "PrototypeNewFlow",
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.workspace.stop_preview_prototype();
        self.window.close();
    }
}

#[test]
fn native_journey_follows_hotspots_and_dynamic_back_without_changing_board_or_source() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let board_host = f.workspace.preview_stage_control(f.home).unwrap();
    let board_note = native::<TextBox>(&board_host, "HomeNote");
    board_note.set_text(Some("Board-only note"));
    let original = f
        .workspace
        .session
        .borrow()
        .document()
        .snapshot_without_history();
    let node = original.ast().find_name("HomeNote").unwrap();
    f.workspace.session.borrow_mut().select([node]);
    f.workspace.session.borrow_mut().set_read_only(true);
    let manifest = f.workspace.preview_stage_manifest();
    f.start("Explore");
    let run_host = f.workspace.preview_prototype_control(f.home).unwrap();
    let note = f.input::<TextBox>(f.home, "HomeNote");
    assert_ne!(run_host, board_host);
    assert_ne!(note, board_note);
    assert_eq!(note.text().as_deref(), Some("Review the desktop layout"));
    note.set_text(Some("Keep this note through the journey"));

    f.activate(0, f.details);
    let completed = f.input::<CheckBox>(f.details, "DetailsChecklist");
    completed.set_is_checked(Some(true));
    f.activate(2, f.settings);
    assert_eq!(
        f.workspace.preview_prototype_playback().unwrap().history,
        vec![f.home, f.details]
    );
    f.activate(4, f.details);
    assert_eq!(
        f.input::<CheckBox>(f.details, "DetailsChecklist"),
        completed
    );
    assert_eq!(completed.is_checked(), Some(true));
    f.activate(3, f.home);
    assert_eq!(f.input::<TextBox>(f.home, "HomeNote"), note);
    assert_eq!(
        note.text().as_deref(),
        Some("Keep this note through the journey")
    );
    assert_eq!(
        f.workspace.preview_stage_control(f.home).unwrap(),
        board_host
    );
    assert_eq!(board_note.text().as_deref(), Some("Board-only note"));
    assert_eq!(f.workspace.preview_stage_manifest(), manifest);
    assert!(f
        .workspace
        .preview_prototype_playback()
        .unwrap()
        .history
        .is_empty());
    assert!(f.workspace.back_preview_prototype().is_err());
    let session = f.workspace.session.borrow();
    assert_eq!(session.document().source(), SOURCE);
    assert_eq!(session.document().revision(), original.revision());
    assert!(Arc::ptr_eq(
        &session.document().ast_snapshot(),
        &original.ast_snapshot()
    ));
    assert_eq!(session.selection(), &[node]);
    assert!(!session.can_undo());
    drop(session);
    assert!(!f.workspace.is_dirty());
}

#[test]
fn restart_retains_this_runs_native_state_and_fresh_play_resets_it_and_old_callbacks() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    f.start("Explore");
    let note = f.input::<TextBox>(f.home, "HomeNote");
    note.set_text(Some("Keep until a new run"));
    f.activate(1, f.settings);
    let settings = f.input::<TextBox>(f.settings, "SettingsName");
    settings.set_text(Some("Edited during Explore"));
    f.workspace.restart_preview_prototype().unwrap();
    f.layout();
    assert_eq!(
        f.workspace.preview_prototype_playback().unwrap().current,
        f.home
    );
    assert!(f
        .workspace
        .preview_prototype_playback()
        .unwrap()
        .history
        .is_empty());
    assert_eq!(f.input::<TextBox>(f.home, "HomeNote"), note);
    assert_eq!(note.text().as_deref(), Some("Keep until a new run"));
    f.activate(1, f.settings);
    assert_eq!(f.input::<TextBox>(f.settings, "SettingsName"), settings);
    assert_eq!(settings.text().as_deref(), Some("Edited during Explore"));
    let retired_back = f.hotspot(4);

    f.workspace.stop_preview_prototype();
    assert!(f.workspace.preview_prototype_playback().is_none());
    f.start("Settings review");
    let current = f.workspace.preview_prototype_playback().unwrap();
    assert_eq!(current.current, f.settings);
    assert!(current.history.is_empty());
    assert_ne!(f.input::<TextBox>(f.settings, "SettingsName"), settings);
    assert_eq!(
        f.input::<TextBox>(f.settings, "SettingsName")
            .text()
            .as_deref(),
        Some("Personal workspace")
    );
    assert!(
        f.workspace.preview_prototype_control(f.home).is_none(),
        "Only this flow's reachable native screens are instantiated"
    );
    click(&retired_back);
    assert_eq!(f.workspace.preview_prototype_playback().unwrap(), current);
    assert!(f.workspace.back_preview_prototype().is_err());

    f.start("Explore");
    assert_ne!(f.input::<TextBox>(f.home, "HomeNote"), note);
    assert_eq!(
        f.input::<TextBox>(f.home, "HomeNote").text().as_deref(),
        Some("Review the desktop layout")
    );
}

#[test]
fn graph_source_and_pending_edits_revoke_playback_before_a_retained_hotspot_can_navigate() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    f.start("Explore");
    let retired = f.hotspot(0);
    let mut graph = f.workspace.preview_prototype();
    graph.links[0].name = "Renamed interaction".into();
    f.workspace.set_preview_prototype(graph).unwrap();
    assert!(f.workspace.preview_prototype_playback().is_none());
    f.start("Explore");
    let current = f.workspace.preview_prototype_playback().unwrap();
    click(&retired);
    assert_eq!(f.workspace.preview_prototype_playback().unwrap(), current);

    let retained = f.hotspot(0);
    let edited = SOURCE.replace(
        "Your projects, notes and next steps.",
        "A new accepted source revision.",
    );
    f.workspace
        .session
        .borrow_mut()
        .update_source(edited.clone());
    let revision = f.workspace.session.borrow().document().revision();
    assert!(
        !f.workspace
            .preview_prototype_playback()
            .unwrap()
            .current_source
    );
    assert!(f.workspace.activate_preview_prototype_link(0).is_err());
    click(&retained);
    assert_eq!(
        f.workspace.preview_prototype_playback().unwrap().current,
        f.home
    );
    assert_eq!(f.workspace.session.borrow().document().source(), edited);
    assert_eq!(f.workspace.session.borrow().document().revision(), revision);
    f.layout();
    assert!(f.workspace.preview_prototype_playback().is_none());

    f.start("Explore");
    let pending_button = f.hotspot(0);
    f.workspace.source_editor().set_text(Some("<UserControl"));
    assert!(f.workspace.pending_source.get());
    let revision = f.workspace.session.borrow().document().revision();
    assert!(f.workspace.activate_preview_prototype_link(0).is_err());
    click(&pending_button);
    assert!(f.workspace.start_preview_prototype("Explore").is_err());
    assert_eq!(
        f.workspace.source_editor().text().as_deref(),
        Some("<UserControl")
    );
    assert_eq!(f.workspace.session.borrow().document().revision(), revision);
    assert!(f.workspace.pending_source.get());
}

#[test]
fn a_retained_include_buffer_edit_invalidates_playback_without_switching_the_active_document() {
    let _app = UnitTestApplication::start(services());
    const VALUES_URI: &str = "ferres://FerroUI.Designer.Tests/PrototypeValues.xaml";
    const VIEW_URI: &str = "ferres://FerroUI.Designer.Tests/PrototypeIncluded.xaml";
    let values = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">123</x:Double></ResourceDictionary>"#;
    let source = r#"<StackPanel xmlns="https://github.com/ferroui"><StackPanel.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source="PrototypeValues.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary></StackPanel.Resources><Button Name="Next" Width="{StaticResource width}" Height="50" Content="Next"/></StackPanel>"#;
    let workspace = DesignerWorkspace::with_uri(values, VALUES_URI.into()).unwrap();
    workspace
        .open_document(source.into(), VIEW_URI.into())
        .unwrap();
    workspace.set_timeline_visible(false);
    let window = attach(&workspace);
    workspace.set_preview_stages_visible(true).unwrap();
    let first = workspace.preview_stages()[0].id;
    let second = workspace.duplicate_preview_stage(first).unwrap();
    let trigger = {
        let session = workspace.session.borrow();
        PreviewSourceAnchor::capture(
            session.document(),
            session.document().ast().find_name("Next").unwrap(),
        )
        .unwrap()
    };
    workspace
        .set_preview_prototype(PreviewPrototype {
            flows: vec![PreviewPrototypeFlow {
                name: "Included".into(),
                start: first,
            }],
            links: vec![PreviewPrototypeLink {
                name: "Next".into(),
                source: first,
                trigger,
                action: PreviewPrototypeAction::Navigate { stage: second },
            }],
        })
        .unwrap();
    workspace.start_preview_prototype("Included").unwrap();
    window.update_layout();
    workspace.refresh_preview_prototype();
    let host = workspace.preview_prototype_control(first).unwrap();
    assert_eq!(native::<Button>(&host, "Next").width(), 123.0);
    let before = workspace.preview_prototype_playback().unwrap();
    let board_epoch = workspace.preview_stage_epoch();
    let document = workspace
        .session
        .borrow()
        .document()
        .snapshot_without_history();
    workspace
        .project
        .borrow_mut()
        .tabs
        .get_mut(VALUES_URI)
        .unwrap()
        .document
        .update_source("Change included resource", values.replace(">123<", ">231<"));
    assert_eq!(workspace.session.borrow().document_uri(), VIEW_URI);
    assert!(
        !workspace
            .preview_prototype_playback()
            .unwrap()
            .current_source
    );
    assert!(workspace.activate_preview_prototype_link(0).is_err());
    assert_eq!(
        workspace.preview_prototype_playback().unwrap().current,
        before.current
    );
    assert_eq!(
        workspace.preview_stage_epoch(),
        board_epoch,
        "A rejected activation cannot synchronize native views"
    );
    assert_eq!(
        workspace.session.borrow().document().source(),
        document.source()
    );
    assert_eq!(
        workspace.session.borrow().document().revision(),
        document.revision()
    );
    assert_eq!(native::<Button>(&host, "Next").width(), 123.0);
    workspace.refresh_preview_stages();
    assert!(workspace.preview_prototype_playback().is_none());
    workspace.start_preview_prototype("Included").unwrap();
    window.update_layout();
    workspace.refresh_preview_prototype();
    assert_eq!(
        native::<Button>(&workspace.preview_prototype_control(first).unwrap(), "Next").width(),
        231.0
    );
    workspace.stop_preview_prototype();
    window.close();
}

#[test]
fn cross_document_stage_search_and_scope_preserve_controls_input_zoom_and_authoring() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let view = f.workspace.preview_stage_view().unwrap();
    let host = f.workspace.preview_stage_control(f.home).unwrap();
    let note = native::<TextBox>(&host, "HomeNote");
    note.set_text(Some("Retain while hidden"));
    let zoom = named::<ComboBox>(&view.control(), &format!("Stage{}Zoom", f.home));
    zoom.set_selected_index(4);
    const OTHER_URI: &str = "memory://prototype/Other.xaml";
    const OTHER: &str =
        r#"<Border xmlns="https://github.com/ferroui" Name="OtherRoot" Background="Green"/>"#;
    f.workspace
        .open_document(OTHER.into(), OTHER_URI.into())
        .unwrap();
    let other = f
        .workspace
        .add_preview_stage(PreviewStageConfiguration::new(
            "Other project",
            OTHER_URI,
            PreviewEnvironment::default(),
        ))
        .unwrap();
    f.layout();
    let source = f
        .workspace
        .session
        .borrow()
        .document()
        .snapshot_without_history();
    let graph = f.workspace.preview_prototype();
    let board = f.workspace.preview_stage_manifest();
    view.set_all_views(false);
    assert_eq!(view.visible_stage_ids(), vec![other]);
    view.set_all_views(true);
    assert_eq!(view.visible_stage_ids().len(), 4);
    view.set_query("HOME prototypeflow");
    assert_eq!(view.visible_stage_ids(), vec![f.home]);
    view.set_query("No such screen");
    assert!(view.visible_stage_ids().is_empty());
    view.set_query("");
    view.set_all_views(false);
    view.set_all_views(true);
    f.layout();
    assert_eq!(f.workspace.preview_stage_control(f.home).unwrap(), host);
    assert_eq!(native::<TextBox>(&host, "HomeNote"), note);
    assert_eq!(note.text().as_deref(), Some("Retain while hidden"));
    assert_eq!(
        named::<ComboBox>(&view.control(), &format!("Stage{}Zoom", f.home)),
        zoom
    );
    assert_eq!(zoom.selected_index(), 4);
    assert_eq!(f.workspace.preview_stage_manifest(), board);
    assert_eq!(f.workspace.preview_prototype(), graph);
    assert_eq!(
        f.workspace.session.borrow().document().source(),
        source.source()
    );
    assert_eq!(
        f.workspace.session.borrow().document().revision(),
        source.revision()
    );
    assert!(!f.workspace.session.borrow().can_undo());
}

#[test]
fn workspace_flow_and_link_editors_apply_detached_drafts_and_reject_stale_models() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    f.workspace.session.borrow_mut().set_read_only(true);
    let anchor = f.overview_anchor();
    let before = f.workspace.preview_prototype();
    f.workspace
        .show_preview_prototype_flow_editor(None, &anchor)
        .unwrap();
    let editor = f.workspace.preview_prototype_flow_editor().unwrap();
    named::<TextBox>(&editor.control(), "PrototypeFlowName").set_text(Some("Discard this flow"));
    editor.cancel();
    assert_eq!(f.workspace.preview_prototype(), before);
    assert!(editor.apply().is_err());

    f.workspace
        .show_preview_prototype_flow_editor(None, &anchor)
        .unwrap();
    let dismissed = f.workspace.preview_prototype_flow_editor().unwrap();
    named::<TextBox>(&dismissed.control(), "PrototypeFlowName")
        .set_text(Some("Dismiss this native flyout"));
    let flyout = f.workspace.prototype.flyout.borrow().clone().unwrap();
    flyout.hide();
    assert!(dismissed.apply().is_err());
    assert_eq!(f.workspace.preview_prototype(), before);

    f.workspace
        .show_preview_prototype_flow_editor(None, &anchor)
        .unwrap();
    let editor = f.workspace.preview_prototype_flow_editor().unwrap();
    named::<TextBox>(&editor.control(), "PrototypeFlowName").set_text(Some("Project review"));
    named::<ComboBox>(&editor.control(), "PrototypeFlowStart").set_selected_index(1);
    assert_eq!(f.workspace.preview_prototype(), before);
    editor.apply().unwrap();
    assert_eq!(
        f.workspace.preview_prototype().flows.last().unwrap(),
        &PreviewPrototypeFlow {
            name: "Project review".into(),
            start: f.details
        }
    );
    assert!(editor.apply().is_err());

    f.workspace
        .show_preview_prototype_link_editor(Some(0), None, &anchor)
        .unwrap();
    let editor = f.workspace.preview_prototype_link_editor().unwrap();
    let accepted = f.workspace.preview_prototype();
    named::<TextBox>(&editor.control(), "PrototypeLinkName").set_text(Some("Inspect settings"));
    named::<ComboBox>(&editor.control(), "PrototypeLinkDestination").set_selected_index(2);
    assert_eq!(f.workspace.preview_prototype(), accepted);
    editor.apply().unwrap();
    let changed = f.workspace.preview_prototype();
    assert_eq!(changed.links[0].name, "Inspect settings");
    assert_eq!(
        changed.links[0].action,
        PreviewPrototypeAction::Navigate { stage: f.settings }
    );
    assert_eq!(changed.links[0].trigger, before.links[0].trigger);

    f.workspace
        .show_preview_prototype_link_editor(Some(0), None, &anchor)
        .unwrap();
    let stale = f.workspace.preview_prototype_link_editor().unwrap();
    named::<TextBox>(&stale.control(), "PrototypeLinkName").set_text(Some("Discard stale change"));
    let mut config = f.workspace.preview_stage(f.home).unwrap().configuration;
    config.name = "Renamed after opening the interaction editor".into();
    f.workspace.update_preview_stage(f.home, config).unwrap();
    assert!(stale.apply().is_err());
    assert_eq!(f.workspace.preview_prototype(), changed);
    assert_eq!(f.workspace.session.borrow().document().source(), SOURCE);
    assert_eq!(f.workspace.session.borrow().document().revision(), 0);
    assert!(!f.workspace.session.borrow().can_undo());
}

#[test]
fn restored_workspace_set_remaps_flow_and_hotspot_endpoints_to_fresh_native_stages() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let manifest = f.workspace.preview_stage_manifest();
    assert_eq!(manifest.version, 2);
    let restored = crate::PreviewStageManifest::from_json(&manifest.to_json().unwrap()).unwrap();
    f.start("Explore");
    let stale = f.hotspot(0);
    f.workspace
        .restore_preview_stage_manifest(restored)
        .unwrap();
    f.layout();
    assert!(f.workspace.preview_prototype_playback().is_none());
    let stages = f.workspace.preview_stages();
    let home = stages
        .iter()
        .find(|stage| stage.configuration.name == "Home")
        .unwrap()
        .id;
    let details = stages
        .iter()
        .find(|stage| stage.configuration.name == "Details")
        .unwrap()
        .id;
    assert_ne!(home, f.home);
    assert_ne!(details, f.details);
    assert_eq!(f.workspace.preview_stage_manifest(), manifest);
    assert_eq!(f.workspace.preview_prototype().flows[0].start, home);
    assert_eq!(f.workspace.preview_prototype().links[0].source, home);
    assert_eq!(
        f.workspace.preview_prototype().links[0].action,
        PreviewPrototypeAction::Navigate { stage: details }
    );
    f.start("Explore");
    let current = f.workspace.preview_prototype_playback().unwrap();
    click(&stale);
    assert_eq!(f.workspace.preview_prototype_playback().unwrap(), current);
    f.activate(0, details);
    assert_eq!(f.workspace.session.borrow().document().source(), SOURCE);
    assert!(!f.workspace.session.borrow().can_undo());
}
