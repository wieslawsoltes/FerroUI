use super::*;
use crate::{
    PreviewEnvironment, PreviewPrototypeAction, PreviewPrototypeFlow, PreviewPrototypeLink,
    PreviewPrototypeOverlayOptions, PreviewPrototypeOverlayPlacement, PreviewStageConfiguration,
    PreviewStageMode,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Size;
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::testing::{
    MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
};
use ferroui_controls::{Button, Canvas, Control, TextBox, Window};

const URI: &str = "memory://prototype-overlays.xaml";
const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui" Name="Scene">
  <TextBox Name="Note" Canvas.Left="16" Canvas.Top="16" Width="150" Height="32" Text="Authored"/>
  <Button Name="Open" Canvas.Left="16" Canvas.Top="58" Width="120" Height="32" Content="Open"/>
  <Button Name="Next" Canvas.Left="16" Canvas.Top="98" Width="120" Height="32" Content="Next"/>
  <Button Name="Close" Canvas.Left="16" Canvas.Top="138" Width="120" Height="32" Content="Close"/>
</Canvas>"#;

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
        .filter_map(|element| element.cast::<Control>())
        .chain(
            root.get_visual_descendants()
                .filter_map(|visual| visual.cast::<Control>()),
        )
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing native overlay control {name}"))
}

struct Fixture {
    workspace: Rc<DesignerWorkspace>,
    window: Ref<Window>,
    stages: Vec<PreviewStageId>,
}
impl Fixture {
    fn new(options: PreviewPrototypeOverlayOptions) -> Self {
        let workspace = DesignerWorkspace::with_uri(SOURCE, URI.into()).unwrap();
        workspace.set_timeline_visible(false);
        let window = Window::new();
        window.set_width(1400.0);
        window.set_height(1000.0);
        window.set_content(Some(Control::boxed(workspace.control())));
        window.show();
        workspace.set_preview_stages_visible(true).unwrap();
        let first = workspace.preview_stages()[0].id;
        let mut stages = Vec::new();
        for (index, (name, width, height)) in [
            ("Home", 400.0, 320.0),
            ("Editor", 240.0, 210.0),
            ("Confirm", 180.0, 180.0),
            ("Help", 220.0, 190.0),
            ("Details", 400.0, 320.0),
        ]
        .into_iter()
        .enumerate()
        {
            let mut configuration = PreviewStageConfiguration::new(
                name,
                URI,
                PreviewEnvironment {
                    width,
                    height,
                    ..Default::default()
                },
            );
            configuration.mode = PreviewStageMode::Live;
            stages.push(if index == 0 {
                workspace
                    .update_preview_stage(first, configuration)
                    .unwrap();
                first
            } else {
                workspace.add_preview_stage(configuration).unwrap()
            });
        }
        let document = workspace
            .session()
            .borrow()
            .document()
            .snapshot_without_history();
        let link = |name: &str, source: usize, control: &str, action| PreviewPrototypeLink {
            name: name.into(),
            source: stages[source],
            action,
            trigger: PreviewSourceAnchor::capture(
                &document,
                document.ast().find_name(control).unwrap(),
            )
            .unwrap(),
        };
        workspace
            .set_preview_prototype(PreviewPrototype {
                flows: vec![PreviewPrototypeFlow {
                    name: "Overlay journey".into(),
                    start: stages[0],
                }],
                links: vec![
                    link(
                        "Open editor",
                        0,
                        "Open",
                        PreviewPrototypeAction::OpenOverlay {
                            stage: stages[1],
                            options,
                        },
                    ),
                    link(
                        "Confirm changes",
                        1,
                        "Open",
                        PreviewPrototypeAction::OpenOverlay {
                            stage: stages[2],
                            options: Default::default(),
                        },
                    ),
                    link(
                        "Replace with help",
                        1,
                        "Next",
                        PreviewPrototypeAction::SwapOverlay {
                            stage: stages[3],
                            options: Default::default(),
                        },
                    ),
                    link(
                        "Close editor",
                        1,
                        "Close",
                        PreviewPrototypeAction::CloseOverlay,
                    ),
                    link(
                        "Close confirmation",
                        2,
                        "Close",
                        PreviewPrototypeAction::CloseOverlay,
                    ),
                    link(
                        "Open details",
                        2,
                        "Next",
                        PreviewPrototypeAction::Navigate { stage: stages[4] },
                    ),
                    link(
                        "Close help",
                        3,
                        "Close",
                        PreviewPrototypeAction::CloseOverlay,
                    ),
                    link("Back", 4, "Close", PreviewPrototypeAction::Back),
                ],
            })
            .unwrap();
        let fixture = Self {
            workspace,
            window,
            stages,
        };
        fixture.layout();
        fixture
            .workspace
            .start_preview_prototype("Overlay journey")
            .unwrap();
        fixture.layout();
        fixture
    }
    fn layout(&self) {
        self.window.update_layout();
        self.workspace.refresh_preview_stages();
        self.window.update_layout();
    }
    fn activate(&self, link: usize) {
        self.layout();
        self.workspace
            .activate_preview_prototype_link(link)
            .unwrap();
        self.layout();
    }
    fn control(&self, index: usize) -> Ref<Control> {
        self.workspace
            .preview_prototype_control(self.stages[index])
            .unwrap()
    }
    fn note(&self, index: usize) -> Ref<TextBox> {
        named(&self.control(index), "Note")
    }
    fn info(&self) -> PreviewPrototypePlaybackInfo {
        self.workspace.preview_prototype_playback().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn native_overlay_stack_retains_covered_views_and_restores_focus_without_source_history() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new(Default::default());
    let workspace = &fixture.workspace;
    let session = workspace.session();
    let source = session.borrow().document().source().to_owned();
    let history = {
        let session = session.borrow();
        (
            session.document().revision(),
            session.document().can_undo(),
            session.document().can_redo(),
        )
    };
    let base = fixture.control(0);
    let parent = base.visual_parent();
    let note = fixture.note(0);
    note.set_text(Some("Retained underneath both overlays"));
    note.focus();
    assert!(note.is_focused());
    fixture.activate(0);
    assert_eq!(fixture.info().screen, fixture.stages[0]);
    assert_eq!(fixture.info().current, fixture.stages[1]);
    assert!(fixture.info().history.is_empty());
    assert_eq!(base.visual_parent(), parent);
    assert!(base.is_attached_to_visual_tree());
    assert!(
        note.is_effectively_enabled(),
        "Modal input blocking must not disable application appearance"
    );
    assert!(!note.is_focused());
    let editor = fixture.control(1);
    let editor_parent = editor.visual_parent();
    let editor_note = fixture.note(1);
    editor_note.set_text(Some("Retained editor draft"));
    editor_note.focus();
    fixture.activate(1);
    assert_eq!(fixture.info().overlays.len(), 2);
    assert_eq!(editor.visual_parent(), editor_parent);
    assert!(editor.is_attached_to_visual_tree());
    assert!(base.is_attached_to_visual_tree());
    workspace.close_preview_prototype_overlay().unwrap();
    fixture.layout();
    assert_eq!(fixture.info().current, fixture.stages[1]);
    assert_eq!(editor_note.text().as_deref(), Some("Retained editor draft"));
    assert!(editor_note.is_focused());
    workspace.back_preview_prototype().unwrap();
    fixture.layout();
    assert_eq!(fixture.info().current, fixture.stages[0]);
    assert!(fixture.info().overlays.is_empty());
    assert!(fixture.info().history.is_empty());
    assert!(note.is_focused());
    assert_eq!(
        note.text().as_deref(),
        Some("Retained underneath both overlays")
    );
    assert_eq!(session.borrow().document().source(), source);
    let session = session.borrow();
    assert_eq!(
        (
            session.document().revision(),
            session.document().can_undo(),
            session.document().can_redo()
        ),
        history
    );
}

#[test]
fn swapping_and_forward_navigation_preserve_base_history_and_retire_old_overlay_callbacks() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new(Default::default());
    fixture.activate(0);
    let draft = fixture.note(1);
    draft.set_text(Some("Editor instance survives replacement"));
    let old_close = named::<Button>(
        &fixture
            .workspace
            .preview_prototype_player_view()
            .unwrap()
            .control(),
        "PrototypeHotspot3",
    );
    fixture.activate(2);
    assert_eq!(fixture.info().current, fixture.stages[3]);
    assert_eq!(fixture.info().overlays.len(), 1);
    assert!(fixture.info().history.is_empty());
    old_close.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(fixture.info().current, fixture.stages[3]);
    fixture.activate(6);
    fixture.activate(0);
    assert_eq!(fixture.note(1), draft);
    assert_eq!(
        draft.text().as_deref(),
        Some("Editor instance survives replacement")
    );
    old_close.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(fixture.info().current, fixture.stages[1]);
    fixture.activate(1);
    fixture.activate(5);
    assert_eq!(fixture.info().screen, fixture.stages[4]);
    assert_eq!(fixture.info().current, fixture.stages[4]);
    assert!(fixture.info().overlays.is_empty());
    assert_eq!(fixture.info().history, [fixture.stages[0]]);
    fixture.activate(7);
    assert_eq!(fixture.info().current, fixture.stages[0]);
    assert!(fixture.info().history.is_empty());
}

#[test]
fn manual_overlay_hotspots_are_clipped_to_base_frame_and_offscreen_open_is_atomic() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new(PreviewPrototypeOverlayOptions {
        placement: PreviewPrototypeOverlayPlacement::Manual { x: -100, y: 0 },
        ..Default::default()
    });
    fixture.activate(0);
    let hotspot = named::<Button>(
        &fixture
            .workspace
            .preview_prototype_player_view()
            .unwrap()
            .control(),
        "PrototypeHotspot1",
    );
    assert_eq!(Canvas::get_left(&hotspot), 100.0);
    assert_eq!(hotspot.width(), 36.0);
    fixture.activate(1);
    assert_eq!(fixture.info().overlays.len(), 2);
    fixture.workspace.stop_preview_prototype();
    let mut graph = fixture.workspace.preview_prototype();
    graph.links[0].action = PreviewPrototypeAction::OpenOverlay {
        stage: fixture.stages[1],
        options: PreviewPrototypeOverlayOptions {
            placement: PreviewPrototypeOverlayPlacement::Manual { x: 500, y: 0 },
            ..Default::default()
        },
    };
    fixture.workspace.set_preview_prototype(graph).unwrap();
    fixture
        .workspace
        .start_preview_prototype("Overlay journey")
        .unwrap();
    fixture.layout();
    let before = fixture.info();
    let base = fixture.control(0);
    assert!(fixture
        .workspace
        .activate_preview_prototype_link(0)
        .unwrap_err()
        .contains("outside"));
    assert_eq!(fixture.info(), before);
    assert_eq!(fixture.control(0), base);
    assert!(base.is_attached_to_visual_tree());
}

#[test]
fn changed_frame_geometry_and_source_revoke_overlay_input_before_native_callbacks_run() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new(Default::default());
    fixture.activate(0);
    let base = fixture.control(0);
    base.set_width(360.0);
    fixture.window.update_layout();
    let before = fixture.info();
    assert!(fixture
        .workspace
        .activate_preview_prototype_link(1)
        .unwrap_err()
        .contains("moved"));
    assert_eq!(fixture.info(), before);
    fixture.layout();
    fixture.activate(1);
    let retained_top = fixture.control(2);
    let retained_editor = fixture.control(1);
    let old_close = named::<Button>(
        &fixture
            .workspace
            .preview_prototype_player_view()
            .unwrap()
            .control(),
        "PrototypeHotspot4",
    );
    let mut graph = fixture.workspace.preview_prototype();
    graph.flows[0].name = "Updated journey".into();
    fixture.workspace.set_preview_prototype(graph).unwrap();
    assert!(fixture.workspace.preview_prototype_playback().is_none());
    assert!(!base.is_attached_to_visual_tree());
    assert!(!retained_top.is_attached_to_visual_tree());
    assert!(!retained_editor.is_attached_to_visual_tree());
    old_close.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert!(fixture.workspace.preview_prototype_playback().is_none());
}

#[test]
fn fitted_manual_overlay_rejection_preserves_the_current_journey_after_native_measurement() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new(PreviewPrototypeOverlayOptions {
        placement: PreviewPrototypeOverlayPlacement::Manual { x: -100, y: 0 },
        ..Default::default()
    });
    fixture.workspace.stop_preview_prototype();
    let mut configuration = fixture
        .workspace
        .preview_stage(fixture.stages[1])
        .unwrap()
        .configuration;
    configuration.layout = crate::PreviewStageLayout::SizeThatFits;
    fixture
        .workspace
        .update_preview_stage(fixture.stages[1], configuration)
        .unwrap();
    fixture
        .workspace
        .start_preview_prototype("Overlay journey")
        .unwrap();
    fixture.layout();

    // The destination has never been mounted in this run. Its initial layout
    // proposal is 240px wide, which overlaps the frame at x=-100, while the real
    // fitted content is 80px wide and therefore ends 20px outside the left edge.
    let overlay = fixture.control(1);
    assert!(!overlay.is_attached_to_visual_tree());
    let content = overlay
        .get_logical_descendants()
        .find(|control| control.name().as_deref() == Some("Scene"))
        .and_then(|control| control.cast::<Canvas>())
        .expect("The detached destination retains its authored content");
    content.set_width(80.0);
    content.set_height(180.0);
    let base = fixture.control(0);
    let note = fixture.note(0);
    note.set_text(Some("Keep this live draft after rejected placement"));
    note.focus();
    fixture.layout();
    let before = fixture.info();
    let source = fixture
        .workspace
        .session()
        .borrow()
        .document()
        .source()
        .to_owned();
    let error = fixture
        .workspace
        .activate_preview_prototype_link(0)
        .unwrap_err();
    assert!(
        error.contains("outside"),
        "Unexpected fitted overlay failure: {error}"
    );
    fixture.layout();

    let after = fixture.info();
    assert_eq!(after.screen, before.screen);
    assert_eq!(after.current, before.current);
    assert_eq!(after.history, before.history);
    assert!(after.overlays.is_empty());
    assert!(after.current_source);
    assert!(after.epoch >= before.epoch);
    assert_eq!(fixture.control(0), base);
    assert!(base.is_attached_to_visual_tree());
    assert!(!overlay.is_attached_to_visual_tree());
    assert!(
        note.is_focused(),
        "Rejected placement restores the accepted focus"
    );
    assert_eq!(
        note.text().as_deref(),
        Some("Keep this live draft after rejected placement")
    );
    assert_eq!(
        fixture.workspace.session().borrow().document().source(),
        source
    );
}

#[test]
fn focus_driven_fitted_overlay_layout_settles_or_rolls_back_the_accepted_journey() {
    let _app = UnitTestApplication::start(services());
    for (focused_width, accepted) in [(180.0, true), (80.0, false)] {
        let fixture = Fixture::new(PreviewPrototypeOverlayOptions {
            placement: PreviewPrototypeOverlayPlacement::Manual { x: -100, y: 0 },
            ..Default::default()
        });
        fixture.workspace.stop_preview_prototype();
        let mut configuration = fixture
            .workspace
            .preview_stage(fixture.stages[1])
            .unwrap()
            .configuration;
        configuration.layout = crate::PreviewStageLayout::SizeThatFits;
        fixture
            .workspace
            .update_preview_stage(fixture.stages[1], configuration)
            .unwrap();
        fixture
            .workspace
            .start_preview_prototype("Overlay journey")
            .unwrap();
        fixture.layout();

        let overlay = fixture.control(1);
        let content = overlay
            .get_logical_descendants()
            .find(|control| control.name().as_deref() == Some("Scene"))
            .and_then(|control| control.cast::<Canvas>())
            .unwrap();
        content.set_width(240.0);
        content.set_height(180.0);
        let content_weak = content.downgrade();
        let changed = Rc::new(Cell::new(false));
        let changed_in_focus = changed.clone();
        let destination_note = fixture.note(1);
        destination_note.got_focus(move |_, _| {
            if !changed_in_focus.replace(true) {
                if let Some(content) = content_weak.upgrade() {
                    content.set_width(focused_width);
                }
            }
        });

        let base = fixture.control(0);
        let base_note = fixture.note(0);
        base_note.set_text(Some("Keep the underlying screen and focus"));
        base_note.focus();
        let before = fixture.info();
        let result = fixture.workspace.activate_preview_prototype_link(0);
        assert!(
            changed.get(),
            "The regression must run the native focus callback"
        );
        if accepted {
            result.unwrap();
            assert_eq!(fixture.info().current, fixture.stages[1]);
            assert_eq!(fixture.info().overlays.len(), 1);
            let player = fixture.workspace.preview_prototype_player_view().unwrap();
            let plane = named::<ferroui_controls::Grid>(
                &player.control(),
                &format!("PrototypePlayerPlane{}", fixture.stages[1]),
            );
            assert_eq!(plane.width(), focused_width);
            assert_eq!(Canvas::get_left(&plane), -100.0);
            assert!(destination_note.is_focused());
        } else {
            assert!(result.unwrap_err().contains("outside"));
            assert_eq!(fixture.info().current, before.current);
            assert_eq!(fixture.info().screen, before.screen);
            assert_eq!(fixture.info().history, before.history);
            assert!(fixture.info().overlays.is_empty());
            assert!(!overlay.is_attached_to_visual_tree());
            assert!(base_note.is_focused());
        }
        assert_eq!(fixture.control(0), base);
        assert!(base.is_attached_to_visual_tree());
        assert_eq!(
            base_note.text().as_deref(),
            Some("Keep the underlying screen and focus")
        );
    }
}

#[test]
fn focus_driven_hotspot_changes_settle_before_an_overlay_accepts_its_first_interaction() {
    let _app = UnitTestApplication::start(services());
    for hide in [false, true] {
        let fixture = Fixture::new(Default::default());
        let destination = fixture.control(1);
        let opener = named::<Button>(&destination, "Open");
        let opener_weak = opener.downgrade();
        let focused = Rc::new(Cell::new(false));
        let focused_in_handler = focused.clone();
        fixture.note(1).got_focus(move |_, _| {
            if !focused_in_handler.replace(true) {
                if let Some(opener) = opener_weak.upgrade() {
                    if hide {
                        opener.set_is_visible(false);
                    } else {
                        Canvas::set_left(&opener, 80.0);
                    }
                }
            }
        });

        fixture
            .workspace
            .activate_preview_prototype_link(0)
            .unwrap();
        assert!(
            focused.get(),
            "The regression must change the native hotspot on focus"
        );
        let player = fixture.workspace.preview_prototype_player_view().unwrap();
        if hide {
            assert!(player.control().get_visual_descendants().all(|visual| {
                visual
                    .cast::<Control>()
                    .is_none_or(|control| control.name().as_deref() != Some("PrototypeHotspot1"))
            }));
            assert!(fixture.info().diagnostics.iter().any(|message| {
                message.contains("Confirm changes") && message.contains("hidden")
            }));
        } else {
            let hotspot = named::<Button>(&player.control(), "PrototypeHotspot1");
            assert_eq!(Canvas::get_left(&hotspot), 80.0);
            // No workspace refresh is needed between accepting the overlay and
            // using its first interaction after a native focus/layout callback.
            fixture
                .workspace
                .activate_preview_prototype_link(1)
                .unwrap();
            assert_eq!(fixture.info().current, fixture.stages[2]);
        }
    }
}
