//! Real native evidence for the graph, detached interaction editor and live player.
use super::*;
use crate::{
    PreviewEnvironment, PreviewPrototype, PreviewPrototypeAction, PreviewPrototypeFlow,
    PreviewPrototypeLink, PreviewSourceAnchor, PreviewStageConfiguration, PreviewStageState,
    PreviewTheme,
};
use ferroui_base::Ref;
use ferroui_controls::{FlyoutPresenter, TextBox};

const SOURCE: &str = include_str!("../../docs/designer/samples/PrototypeFlow.xaml");
fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing prototype control {name}"))
}
fn settle(window: &Window, clock: &RenderClock) {
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
}

#[test]
fn native_prototype_overview_editor_and_player_render_and_navigate_retained_views() {
    let locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()));
    let clock = Rc::new(RenderClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
        now: Cell::new(0.0),
    });
    let mut services = TestServices::styled_window();
    services.global_clock = Some(clock.clone());
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let native = MockWindowImpl::window(1600.0, 1040.0);
        native.max_auto_size_hint.set(Size::new(1600.0, 1040.0));
        native.setup_create_popup(|_| None);
        let screen = PixelRect::new(0, 0, 1600, 1040);
        native.setup_feature::<dyn IScreenImpl>(MockScreenImpl::new(vec![mock_screen(
            1.0, screen, screen, true,
        )]));
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
    services.render_interface =
        FerroLocator::current().get_service::<dyn IPlatformRenderInterface>();
    services.font_manager_impl = FerroLocator::current().get_service::<dyn IFontManagerImpl>();
    services.text_shaper_impl = FerroLocator::current().get_service::<dyn ITextShaperImpl>();
    services.theme = Some(Rc::new(|| {
        ferroui_themes_winui::WinUITheme::new().as_style()
    }));
    let app = UnitTestApplication::start(services);
    ferroui_themes_winui::register_types();
    if let Some(application) = ferroui_controls::Application::current() {
        application.set_requested_theme_variant(Some(ferroui_base::styling::ThemeVariant::dark()));
        FerroLocator::current_mutable()
            .bind::<dyn ferroui_base::styling::IThemeVariantHost>()
            .to_constant(application.as_theme_variant_host());
    }
    let workspace =
        DesignerWorkspace::with_uri(SOURCE, "file:///PrototypeFlow.xaml".into()).unwrap();
    workspace.set_timeline_visible(false);
    workspace.set_canvas_focused(true);
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let document = workspace
        .session()
        .borrow()
        .document()
        .snapshot_without_history();
    let anchor = |name: &str| {
        PreviewSourceAnchor::capture(&document, document.ast().find_name(name).unwrap()).unwrap()
    };
    let mut stages = Vec::new();
    for name in ["Home", "Details", "Settings"] {
        let mut config = PreviewStageConfiguration::new(
            name,
            "file:///PrototypeFlow.xaml",
            PreviewEnvironment {
                width: 520.0,
                height: 740.0,
                theme: PreviewTheme::Light,
                ..Default::default()
            },
        );
        config.states.push(PreviewStageState {
            owner: anchor("PrototypeScene"),
            group: "Screen".into(),
            state: name.into(),
        });
        stages.push(workspace.add_preview_stage(config).unwrap());
    }
    workspace.set_preview_stages_visible(true).unwrap();
    settle(&window, &clock);
    workspace.refresh(true);
    settle(&window, &clock);
    let original_note = named::<TextBox>(
        &workspace.preview_stage_control(stages[0]).unwrap(),
        "HomeNote",
    );
    let original_text = original_note.text();
    let link = |name: &str, source: usize, control: &str, action| PreviewPrototypeLink {
        name: name.into(),
        source: stages[source],
        trigger: anchor(control),
        action,
    };
    workspace
        .set_preview_prototype(PreviewPrototype {
            flows: vec![
                PreviewPrototypeFlow {
                    name: "Explore workspace".into(),
                    start: stages[0],
                },
                PreviewPrototypeFlow {
                    name: "Settings review".into(),
                    start: stages[2],
                },
            ],
            links: vec![
                link(
                    "Open project",
                    0,
                    "HomeOpenDetails",
                    PreviewPrototypeAction::Navigate { stage: stages[1] },
                ),
                link(
                    "Workspace settings",
                    0,
                    "HomeOpenSettings",
                    PreviewPrototypeAction::Navigate { stage: stages[2] },
                ),
                link(
                    "Project settings",
                    1,
                    "DetailsOpenSettings",
                    PreviewPrototypeAction::Navigate { stage: stages[2] },
                ),
                link(
                    "Back to previous screen",
                    1,
                    "DetailsBack",
                    PreviewPrototypeAction::Back,
                ),
                link(
                    "Return from settings",
                    2,
                    "SettingsBack",
                    PreviewPrototypeAction::Back,
                ),
            ],
        })
        .unwrap();
    workspace.set_preview_prototype_visible(true).unwrap();
    settle(&window, &clock);
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    capture(
        &workspace,
        &clock,
        &directory.join("designer-prototype-overview.png"),
    );
    let overview = workspace.preview_prototype_view().unwrap();
    for stage in &stages {
        assert!(
            named::<Control>(&overview.control(), &format!("PrototypeScreen{stage}")).is_visible()
        );
    }
    let editor_anchor = named::<Control>(&overview.control(), "PrototypeAddLink");
    workspace
        .show_preview_prototype_link_editor(Some(0), None, &editor_anchor)
        .unwrap();
    settle(&window, &clock);
    let editor = workspace.preview_prototype_link_editor().unwrap();
    assert!(window
        .get_visual_descendants()
        .any(|visual| visual.cast::<FlyoutPresenter>().is_some()));
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1600, 1040));
    bitmap.render(&window.clone().upcast());
    bitmap
        .save_to_file(
            directory
                .join("designer-prototype-editor.png")
                .to_str()
                .unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
    editor.cancel();
    settle(&window, &clock);
    workspace
        .start_preview_prototype("Explore workspace")
        .unwrap();
    settle(&window, &clock);
    workspace.refresh(true);
    settle(&window, &clock);
    let player_home = workspace.preview_prototype_control(stages[0]).unwrap();
    let note = named::<TextBox>(&player_home, "HomeNote");
    note.set_text(Some("This note survives the complete journey"));
    settle(&window, &clock);
    workspace.refresh(false);
    settle(&window, &clock);
    let player = workspace.preview_prototype_player_view().unwrap();
    named::<Button>(&player.control(), "PrototypeHotspot0")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    settle(&window, &clock);
    assert_eq!(
        workspace.preview_prototype_playback().unwrap().current,
        stages[1]
    );
    named::<Button>(&player.control(), "PrototypeActivate2")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    settle(&window, &clock);
    assert_eq!(
        workspace.preview_prototype_playback().unwrap().current,
        stages[2]
    );
    workspace.back_preview_prototype().unwrap();
    settle(&window, &clock);
    workspace.back_preview_prototype().unwrap();
    settle(&window, &clock);
    assert_eq!(
        workspace.preview_prototype_playback().unwrap().current,
        stages[0]
    );
    assert_eq!(
        note.text().as_deref(),
        Some("This note survives the complete journey")
    );
    assert_eq!(original_note.text(), original_text);
    capture(
        &workspace,
        &clock,
        &directory.join("designer-prototype-player.png"),
    );
    assert_eq!(workspace.session().borrow().document().source(), SOURCE);
    assert!(!workspace.session().borrow().can_undo());
    workspace.stop_preview_prototype();
    window.close();
    drop(workspace);
    drop(app);
    drop(locator);
}
