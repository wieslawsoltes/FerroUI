use super::*;
use crate::PreviewStageLayout;
use ferroui_base::input::{
    IPointer, KeyEventArgs, MouseButton, Pointer, PointerEventArgs, PointerPointProperties,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind,
    RawInputModifiers,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{Point, Size, Vector};
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::testing::{
    MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
};
use ferroui_controls::{CheckBox, ComboBox, Window};

const URI: &str = "memory://stage-first.xaml";
const SOURCE: &str = r#"<Canvas xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Canvas.Resources><ControlTemplate x:Key='BadgeTemplate' TargetType='Button'><Border Name='TemplatePart' Background='Orange'/></ControlTemplate></Canvas.Resources>
  <Button Name='Card' Canvas.Left='20' Canvas.Top='20' Width='140' Height='70' Content='Open'/>
  <TextBox Name='Input' Canvas.Left='20' Canvas.Top='120' Width='180' Text='Edit me'/>
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

struct Fixture {
    workspace: Rc<DesignerWorkspace>,
    window: Ref<Window>,
}
impl Fixture {
    fn new() -> Self {
        let workspace = DesignerWorkspace::with_uri(SOURCE, URI.into()).unwrap();
        workspace.set_timeline_visible(false);
        let window = Window::new();
        window.set_width(1400.0);
        window.set_height(1000.0);
        window.set_content(Some(Control::boxed(workspace.control())));
        window.show();
        window.update_layout();
        workspace.set_preview_stages_visible(true).unwrap();
        let fixture = Self { workspace, window };
        fixture.layout();
        fixture
    }
    fn layout(&self) {
        self.window.update_layout();
        self.workspace.refresh_preview_stages();
        self.window.update_layout();
    }
    fn first(&self) -> PreviewStageId {
        self.workspace.preview_stages()[0].id
    }
    fn anchor(&self) -> Ref<Button> {
        self.workspace.control().get_control("StagesToggle")
    }
    fn card_node(&self) -> NodeId {
        self.workspace
            .session
            .borrow()
            .document()
            .ast()
            .find_name("Card")
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing stage UI control {name}"))
}
fn has_named(root: &Control, name: &str) -> bool {
    root.get_logical_descendants()
        .any(|control| control.name().as_deref() == Some(name))
}
fn native<T: ferroui_base::ObjectType>(host: &Control, name: &str) -> Ref<T> {
    host.get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing stage application control {name}"))
}

#[test]
fn stage_editor_applies_complete_draft_for_viewer_and_cancel_preserves_source_and_board() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let workspace = &f.workspace;
    let id = f.first();
    workspace.session.borrow_mut().set_read_only(true);
    let before = workspace.preview_stage_manifest();
    workspace
        .show_preview_stage_editor(Some(id), &f.anchor())
        .unwrap();
    let editor = workspace.preview_stage_editor().unwrap();
    named::<TextBox>(&editor.control(), "StageName").set_text(Some("Discarded"));
    named::<TextBox>(&editor.control(), "StageWidth").set_text(Some("390"));
    editor.cancel();
    assert_eq!(workspace.preview_stage_manifest(), before);
    assert!(editor.apply().is_err());

    workspace
        .show_preview_stage_editor(Some(id), &f.anchor())
        .unwrap();
    let editor = workspace.preview_stage_editor().unwrap();
    named::<TextBox>(&editor.control(), "StageName").set_text(Some("Phone dark RTL"));
    named::<TextBox>(&editor.control(), "StageWidth").set_text(Some("390"));
    named::<TextBox>(&editor.control(), "StageHeight").set_text(Some("844"));
    named::<ComboBox>(&editor.control(), "StageTheme").set_selected_index(1);
    named::<ComboBox>(&editor.control(), "StageDirection").set_selected_index(2);
    named::<CheckBox>(&editor.control(), "StagePinned").set_is_checked(Some(true));
    named::<CheckBox>(&editor.control(), "StageAutoUpdate").set_is_checked(Some(false));
    named::<CheckBox>(&editor.control(), "StageDesignData").set_is_checked(Some(false));
    editor.apply().unwrap();
    let configuration = workspace.preview_stage(id).unwrap().configuration;
    assert_eq!(configuration.name, "Phone dark RTL");
    assert_eq!(
        configuration.environment,
        PreviewEnvironment {
            width: 390.0,
            height: 844.0,
            theme: PreviewTheme::Dark,
            flow_direction: PreviewFlowDirection::RightToLeft
        }
    );
    assert!(configuration.pinned);
    assert!(!configuration.auto_update);
    assert!(!configuration.design_time_data);
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    assert_eq!(workspace.session.borrow().document().revision(), 0);
    assert!(!workspace.session.borrow().can_undo());
}

#[test]
fn source_draft_and_retired_card_callbacks_cannot_apply_staged_configuration() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let workspace = &f.workspace;
    let id = f.first();
    workspace
        .show_preview_stage_editor(Some(id), &f.anchor())
        .unwrap();
    let editor = workspace.preview_stage_editor().unwrap();
    let before = workspace.preview_stage_manifest();
    let draft = SOURCE.replace("Content='Open'", "Content='Draft'");
    workspace.source_editor().set_text(Some(&draft));
    let staged_revision = workspace.session.borrow().document().revision();
    workspace.refresh(false);
    assert!(editor.apply().is_err());
    assert_eq!(workspace.preview_stage_manifest(), before);
    assert_eq!(workspace.session.borrow().document().source(), draft);
    assert_eq!(
        workspace.session.borrow().document().revision(),
        staged_revision
    );
    assert!(workspace.pending_source.get());
    assert!(workspace.source_editor().text().unwrap().contains("Draft"));
    let info = workspace.preview_stage(id).unwrap();
    let mut changed = info.configuration.clone();
    changed.name = "Renamed".into();
    workspace.update_preview_stage(id, changed).unwrap();
    assert!(workspace
        .handle_preview_stage_action(
            PreviewStageAction::Remove {
                stage: id,
                epoch: info.epoch
            },
            &f.anchor()
        )
        .is_err());
    assert_eq!(workspace.preview_stages().len(), 1);
}

#[test]
fn stage_visibility_modes_pinning_and_live_input_preserve_independent_application_state() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let workspace = &f.workspace;
    let first = f.first();
    let mut configuration = workspace.preview_stage(first).unwrap().configuration;
    configuration.mode = PreviewStageMode::Live;
    configuration.pinned = true;
    workspace
        .update_preview_stage(first, configuration)
        .unwrap();
    let second = workspace.duplicate_preview_stage(first).unwrap();
    let mut second_config = workspace.preview_stage(second).unwrap().configuration;
    second_config.pinned = false;
    workspace
        .update_preview_stage(second, second_config)
        .unwrap();
    f.layout();
    let first_host = workspace.preview_stage_control(first).unwrap();
    let first_input = native::<TextBox>(&first_host, "Input");
    let second_input =
        native::<TextBox>(&workspace.preview_stage_control(second).unwrap(), "Input");
    first_input.set_text(Some("Only the first native instance changed"));
    assert_eq!(second_input.text().as_deref(), Some("Edit me"));
    let card_node = f.card_node();
    workspace.session.borrow_mut().select([card_node]);
    let application_button = native::<Button>(&first_host, "Card");
    let mut key = KeyEventArgs::new();
    key.key = Key::Delete;
    key.set_routed_event(Some(InputElement::key_down_event()));
    key.set_source(application_button.to_ref());
    application_button.raise_event(&key);
    assert!(key.handled());
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    workspace.set_preview_stages_visible(false).unwrap();
    workspace.set_preview_stages_visible(true).unwrap();
    workspace.set_view_mode(super::super::shell::WorkspaceViewMode::Xaml);
    assert!(!workspace
        .root
        .get_control::<Control>("PreviewStagesHost")
        .is_visible());
    workspace.set_view_mode(super::super::shell::WorkspaceViewMode::Split);
    assert!(workspace
        .root
        .get_control::<Control>("PreviewStagesHost")
        .is_visible());
    assert!(!workspace
        .root
        .get_control::<Control>("ArtboardScroll")
        .is_visible());
    assert!(!workspace
        .root
        .get_control::<Control>("CanvasToolShelf")
        .is_visible());
    assert_eq!(
        first_input.text().as_deref(),
        Some("Only the first native instance changed")
    );
    workspace
        .open_document(
            "<Border xmlns='https://github.com/ferroui' Background='Green'/>".into(),
            "memory://stage-second.xaml".into(),
        )
        .unwrap();
    f.layout();
    let view = workspace.preview_stage_view().unwrap();
    assert!(has_named(&view.control(), &format!("PreviewStage{first}")));
    assert!(!has_named(
        &view.control(),
        &format!("PreviewStage{second}")
    ));
    workspace.activate_document(URI).unwrap();
    f.layout();
    assert!(has_named(&view.control(), &format!("PreviewStage{second}")));
    assert_eq!(
        first_input.text().as_deref(),
        Some("Only the first native instance changed")
    );
    assert_eq!(second_input.text().as_deref(), Some("Edit me"));
}

#[test]
fn scaled_stage_pointer_selects_its_authored_control_before_inspector_handoff() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let workspace = &f.workspace;
    let id = f.first();
    let mut configuration = workspace.preview_stage(id).unwrap().configuration;
    configuration.environment = PreviewEnvironment {
        width: 390.0,
        height: 844.0,
        theme: PreviewTheme::Dark,
        flow_direction: PreviewFlowDirection::LeftToRight,
    };
    workspace
        .update_preview_stage(id, configuration.clone())
        .unwrap();
    f.layout();
    let host = workspace.preview_stage_control(id).unwrap();
    let button = native::<Button>(&host, "Card");
    let position = button
        .translate_point(Point::new(20.0, 20.0), &f.window)
        .unwrap();
    let surface = named::<Control>(
        &workspace.preview_stage_view().unwrap().control(),
        &format!("Stage{id}SelectionSurface"),
    );
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let event = PointerPressedEventArgs::new(
        surface.to_ref(),
        pointer,
        &f.window,
        position,
        1,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::NONE,
        1,
    );
    surface.raise_event(&event);
    assert!(event.handled());
    assert_eq!(workspace.session.borrow().selection(), [f.card_node()]);
    assert_eq!(workspace.preview_environment(), configuration.environment);
    assert!(workspace.preview_stages_visible());
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    let info = workspace.preview_stage(id).unwrap();
    workspace
        .handle_preview_stage_action(
            PreviewStageAction::Focus {
                stage: id,
                epoch: info.epoch,
            },
            &f.anchor(),
        )
        .unwrap();
    assert!(!workspace.preview_stages_visible());
    assert!(workspace
        .root
        .get_control::<Control>("ArtboardScroll")
        .is_visible());
}

#[test]
fn responsive_cards_and_staged_editor_keep_commands_inside_native_viewports() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let workspace = &f.workspace;
    workspace
        .add_preview_stage_comparison(PreviewStageComparison::Sizes)
        .unwrap();
    for width in [1400.0, 980.0] {
        f.window.set_width(width);
        f.layout();
        let view = workspace.preview_stage_view().unwrap();
        let scroll = named::<Control>(&view.control(), "PreviewStageScroll");
        for info in workspace.preview_stages() {
            let card = named::<Control>(&view.control(), &format!("PreviewStage{}", info.id));
            let position = card.translate_point(Point::default(), &scroll).unwrap();
            assert!(
                position.x >= 0.0
                    && position.x + card.bounds().width <= scroll.bounds().width + 0.5,
                "Stage {} exceeds the {width}px host: {position:?}, {:?}, {:?}",
                info.id,
                card.bounds(),
                scroll.bounds()
            );
        }
    }
    workspace
        .show_preview_stage_editor(Some(f.first()), &f.anchor())
        .unwrap();
    f.layout();
    let editor = workspace.preview_stage_editor().unwrap();
    let apply = named::<Button>(&editor.control(), "StageEditorApply");
    let presenter = workspace
        .preview_stages
        .flyout
        .borrow()
        .as_ref()
        .unwrap()
        .popup()
        .child()
        .unwrap();
    let position = apply.translate_point(Point::default(), &presenter).unwrap();
    assert!(
        position.x >= 12.0 && position.x + apply.bounds().width <= presenter.bounds().width - 11.5
    );
    assert!(
        position.y >= 0.0 && position.y + apply.bounds().height <= presenter.bounds().height + 0.5
    );
    named::<Button>(&editor.control(), "StageEditorCancel")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
}

struct StagePointer {
    pointer: Rc<Pointer>,
    source: Ref<Control>,
    root: Ref<Window>,
}
impl StagePointer {
    fn new(f: &Fixture, id: PreviewStageId) -> Self {
        Self {
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
            source: named(
                &f.workspace.preview_stage_view().unwrap().control(),
                &format!("Stage{id}ResizeGrip"),
            ),
            root: f.window.clone(),
        }
    }
    fn start(&self) -> Point {
        self.source
            .translate_point(Point::new(12.0, 12.0), &self.root)
            .unwrap()
    }
    fn press(&self, point: Point) {
        let event = PointerPressedEventArgs::new(
            self.source.to_ref(),
            self.pointer.clone(),
            &self.root,
            point,
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        );
        self.source.raise_event(&event);
        assert!(event.handled());
    }
    fn motion(&self, point: Point) {
        self.source.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            self.source.to_ref(),
            self.pointer.clone(),
            Some(&self.root),
            point,
            2,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            KeyModifiers::NONE,
        ));
    }
    fn release(&self, point: Point) {
        self.source.raise_event(&PointerReleasedEventArgs::new(
            self.source.to_ref(),
            self.pointer.clone(),
            &self.root,
            point,
            3,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            KeyModifiers::NONE,
            MouseButton::Left,
        ));
    }
    fn key(&self, key: Key, modifiers: KeyModifiers) -> bool {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.key_modifiers = modifiers;
        event.set_routed_event(Some(InputElement::key_down_event()));
        event.set_source(self.source.to_ref());
        self.source.raise_event(&event);
        event.handled()
    }
}
impl Drop for StagePointer {
    fn drop(&mut self) {
        self.pointer.capture(None);
    }
}

#[test]
fn stage_layout_editor_stages_fitted_axes_and_rotates_the_sizing_contract() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let id = f.first();
    let before = f.workspace.preview_stage_manifest();
    f.workspace
        .show_preview_stage_editor(Some(id), &f.anchor())
        .unwrap();
    let editor = f.workspace.preview_stage_editor().unwrap();
    named::<ComboBox>(&editor.control(), "StageLayout").set_selected_index(2);
    named::<TextBox>(&editor.control(), "StageWidth").set_text(Some("390"));
    named::<TextBox>(&editor.control(), "StageHeight").set_text(Some("844"));
    assert_eq!(
        named::<TextBlock>(&editor.control(), "StageWidthCaption")
            .text()
            .as_deref(),
        Some("Width")
    );
    assert_eq!(
        named::<TextBlock>(&editor.control(), "StageHeightCaption")
            .text()
            .as_deref(),
        Some("Maximum height")
    );
    assert_eq!(f.workspace.preview_stage_manifest(), before);
    named::<Button>(&editor.control(), "StageRotate")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let draft = editor.value().unwrap();
    assert_eq!(draft.layout, PreviewStageLayout::FixedHeight);
    assert_eq!(
        (draft.environment.width, draft.environment.height),
        (844.0, 390.0)
    );
    editor.apply().unwrap();
    assert_eq!(f.workspace.preview_stage(id).unwrap().configuration, draft);
    assert_eq!(f.workspace.session.borrow().document().source(), SOURCE);
    assert!(!f.workspace.session.borrow().can_undo());
}

#[test]
fn scaled_native_resize_drafts_then_commits_release_position_once_preserving_live_input() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let id = f.first();
    let mut configuration = f.workspace.preview_stage(id).unwrap().configuration;
    configuration.environment.width = 390.0;
    configuration.environment.height = 844.0;
    configuration.mode = PreviewStageMode::Live;
    f.workspace.update_preview_stage(id, configuration).unwrap();
    f.layout();
    let before = f.workspace.preview_stage_manifest();
    let host = f.workspace.preview_stage_control(id).unwrap();
    let input = native::<TextBox>(&host, "Input");
    input.set_text(Some("Retained while resizing"));
    let delta = |width, height| {
        host.translate_point(Point::new(width, height), &f.window)
            .unwrap()
            - host.translate_point(Point::default(), &f.window).unwrap()
    };
    let pointer = StagePointer::new(&f, id);
    let start = pointer.start();
    pointer.press(start);
    assert!(pointer.pointer.captured().is_some());
    pointer.motion(start + delta(10.0, 20.0));
    assert_eq!(f.workspace.preview_stage_manifest(), before);
    let view = f.workspace.preview_stage_view().unwrap();
    assert!(named::<Control>(&view.control(), &format!("Stage{id}ResizeDraft")).is_visible());
    pointer.release(start + delta(32.0, 40.0));
    assert!(pointer.pointer.captured().is_none());
    let current = f.workspace.preview_stage(id).unwrap();
    assert_eq!(current.configuration.layout, PreviewStageLayout::Fixed);
    assert_eq!(
        (
            current.configuration.environment.width,
            current.configuration.environment.height
        ),
        (422.0, 884.0)
    );
    assert_eq!(input.text().as_deref(), Some("Retained while resizing"));
    assert!(std::ptr::eq(&*input, &*native::<TextBox>(&host, "Input")));
    let accepted = f.workspace.preview_stage_manifest();
    pointer.release(start + Vector::new(100.0, 100.0));
    assert_eq!(f.workspace.preview_stage_manifest(), accepted);
    assert_eq!(f.workspace.session.borrow().document().source(), SOURCE);
    assert!(!f.workspace.session.borrow().can_undo());
    f.layout();
    let pointer = StagePointer::new(&f, id);
    assert!(
        pointer.source.is_focused(),
        "Keyboard resizing retains focus when the card reconciles"
    );
    assert!(pointer.key(Key::Right, KeyModifiers::SHIFT));
    assert_eq!(
        f.workspace
            .preview_stage(id)
            .unwrap()
            .configuration
            .environment
            .width,
        432.0
    );
}

#[test]
fn native_resize_cancels_on_escape_capture_loss_zoom_reflow_and_card_retirement() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let id = f.first();
    let before = f
        .workspace
        .preview_stage(id)
        .unwrap()
        .configuration
        .environment;
    for kind in 0..5 {
        f.layout();
        let pointer = StagePointer::new(&f, id);
        let start = pointer.start();
        pointer.press(start);
        pointer.motion(start + Vector::new(25.0, 20.0));
        match kind {
            0 => assert!(pointer.key(Key::Escape, KeyModifiers::NONE)),
            1 => pointer.pointer.capture(None),
            2 => named::<ComboBox>(
                &f.workspace.preview_stage_view().unwrap().control(),
                &format!("Stage{id}Zoom"),
            )
            .set_selected_index(4),
            3 => {
                f.window.set_width(1100.0);
                f.window.update_layout();
            }
            _ => {
                let mut config = f.workspace.preview_stage(id).unwrap().configuration;
                config.name = "Retired resize card".into();
                f.workspace.update_preview_stage(id, config).unwrap();
            }
        }
        assert!(
            pointer.pointer.captured().is_none(),
            "Cancellation kind {kind}"
        );
        pointer.release(start + Vector::new(80.0, 60.0));
        assert_eq!(
            f.workspace
                .preview_stage(id)
                .unwrap()
                .configuration
                .environment,
            before
        );
    }
    assert_eq!(f.workspace.session.borrow().document().source(), SOURCE);
    assert!(!f.workspace.session.borrow().can_undo());
}

#[test]
#[cfg(feature = "render-smoke")]
fn native_card_zoom_scrolls_at_actual_size_and_preserves_source_model_and_live_graph() {
    use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
    use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver};
    struct ThemeClock;
    impl IObservable<TimeSpan> for ThemeClock {
        fn subscribe(&self, _: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
            Disposable::empty()
        }
    }
    impl IClock for ThemeClock {
        fn play_state(&self) -> PlayState {
            PlayState::Pause
        }
        fn set_play_state(&self, _: PlayState) {}
    }
    impl IGlobalClock for ThemeClock {}
    // The compact test theme supplies only a ContentControl template to ordinary
    // ScrollViewers. Exercise real extents, offsets and scrollbars with WinUI here.
    let mut native_services = services();
    native_services.global_clock = Some(Rc::new(ThemeClock));
    native_services.theme = Some(Rc::new(|| {
        ferroui_themes_winui::WinUITheme::new().as_style()
    }));
    let _app = UnitTestApplication::start(native_services);
    ferroui_themes_winui::register_types();
    let f = Fixture::new();
    let id = f.first();
    let mut configuration = f.workspace.preview_stage(id).unwrap().configuration;
    configuration.mode = PreviewStageMode::Live;
    f.workspace.update_preview_stage(id, configuration).unwrap();
    f.layout();
    let host = f.workspace.preview_stage_control(id).unwrap();
    let input = native::<TextBox>(&host, "Input");
    input.set_text(Some("Camera state is independent"));
    let before = f.workspace.preview_stage_manifest();
    let epoch = f.workspace.preview_stage_epoch();
    let view = f.workspace.preview_stage_view().unwrap();
    let zoom = named::<ComboBox>(&view.control(), &format!("Stage{id}Zoom"));
    for (index, expected) in [(4, 1.0), (2, 0.5), (6, 2.0)] {
        zoom.set_selected_index(index);
        f.layout();
        let origin = host.translate_point(Point::default(), &f.window).unwrap();
        let point = host
            .translate_point(Point::new(100.0, 0.0), &f.window)
            .unwrap();
        assert!(
            (point.x - origin.x - 100.0 * expected).abs() < 0.001,
            "Actual native scale differs from {expected}"
        );
        assert_eq!(f.workspace.preview_stage_manifest(), before);
        assert_eq!(f.workspace.preview_stage_epoch(), epoch);
        assert_eq!(input.text().as_deref(), Some("Camera state is independent"));
    }
    zoom.set_selected_index(4);
    f.layout();
    let viewport =
        named::<ferroui_controls::ScrollViewer>(&view.control(), &format!("Stage{id}Viewport"));
    viewport.set_offset(Vector::new(50.0, 20.0));
    f.layout();
    assert!(viewport.offset().x > 0.0);
    let mut configuration = f.workspace.preview_stage(id).unwrap().configuration;
    configuration.name = "Same camera after card update".into();
    f.workspace.update_preview_stage(id, configuration).unwrap();
    f.layout();
    assert_eq!(
        named::<ComboBox>(&view.control(), &format!("Stage{id}Zoom")).selected_index(),
        4
    );
    assert_eq!(input.text().as_deref(), Some("Camera state is independent"));
    assert_eq!(f.workspace.session.borrow().document().source(), SOURCE);
}

#[test]
fn content_fit_card_tracks_native_desired_size_and_retires_an_incompatible_resize_draft() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let source = "<StackPanel xmlns='https://github.com/ferroui'><Border Name='Fill' Width='120' Height='40'/></StackPanel>";
    f.workspace
        .open_document(source.into(), "memory://fit-card.xaml".into())
        .unwrap();
    let mut config = PreviewStageConfiguration::new(
        "Fitted component",
        "memory://fit-card.xaml",
        PreviewEnvironment {
            width: 320.0,
            height: 240.0,
            ..Default::default()
        },
    );
    config.layout = PreviewStageLayout::SizeThatFits;
    config.mode = PreviewStageMode::Live;
    let id = f.workspace.add_preview_stage(config).unwrap();
    f.layout();
    let host = f.workspace.preview_stage_control(id).unwrap();
    let fill = native::<ferroui_controls::Border>(&host, "Fill");
    let pointer = StagePointer::new(&f, id);
    let start = pointer.start();
    pointer.press(start);
    pointer.motion(start + Vector::new(20.0, 20.0));
    fill.set_width(250.0);
    f.layout();
    assert!(pointer.pointer.captured().is_none());
    pointer.release(start + Vector::new(80.0, 60.0));
    let info = f.workspace.preview_stage(id).unwrap();
    assert_eq!(
        (info.layout_size.width, info.layout_size.height),
        (250.0, 40.0)
    );
    assert_eq!(info.configuration.layout, PreviewStageLayout::SizeThatFits);
    assert!(named::<TextBlock>(
        &f.workspace.preview_stage_view().unwrap().control(),
        &format!("Stage{id}Dimensions")
    )
    .text()
    .unwrap()
    .contains("250 × 40"));
    assert_eq!(f.workspace.session.borrow().document().source(), source);
    assert!(!f.workspace.session.borrow().can_undo());
}
