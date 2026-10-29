//! Native overlay evidence: retained document/template roots, stacked dialogs,
//! swapping, light dismissal and the real placement editor.
use super::*;
use crate::{
    DefinitionPreviewOptions, PreviewEnvironment, PreviewPrototype, PreviewPrototypeAction,
    PreviewPrototypeFlow, PreviewPrototypeLink, PreviewPrototypeOverlayOptions,
    PreviewPrototypeOverlayPlacement, PreviewSourceAnchor, PreviewStageConfiguration,
    PreviewStageStatus, PreviewStageTarget, PreviewTheme,
};
use ferroui_base::input::{
    KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType,
    PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::{Point, Ref};
use ferroui_controls::{CheckBox, ComboBox, FlyoutPresenter, ScrollViewer, TextBox};
use ferroui_designer_tools::ComponentCatalog;

const SOURCE: &str = include_str!("../../docs/designer/samples/PrototypeOverlays.xaml");

fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_logical_descendants()
        .filter_map(|element| element.cast::<Control>())
        .chain(
            root.get_visual_descendants()
                .filter_map(|visual| visual.cast::<Control>()),
        )
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing prototype overlay control {name}"))
}

fn settle(window: &Window, clock: &RenderClock) {
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
}

fn click(workspace: &Rc<DesignerWorkspace>, window: &Window, clock: &RenderClock, link: usize) {
    let player = workspace.preview_prototype_player_view().unwrap();
    named::<Button>(&player.control(), &format!("PrototypeHotspot{link}"))
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    settle(window, clock);
    workspace.refresh(false);
    settle(window, clock);
}

#[test]
fn native_prototype_overlays_render_stack_swap_dismiss_and_preserve_underlying_input() {
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
    let focus = TestServices::real_focus();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
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
        DesignerWorkspace::with_uri(SOURCE, "file:///PrototypeOverlays.xaml".into()).unwrap();
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
    let config = |name, width, height| {
        PreviewStageConfiguration::new(
            name,
            "file:///PrototypeOverlays.xaml",
            PreviewEnvironment {
                width,
                height,
                theme: PreviewTheme::Light,
                ..Default::default()
            },
        )
    };
    let mut stages = vec![workspace
        .add_preview_stage(config("Workspace", 680.0, 660.0))
        .unwrap()];
    let catalog = ComponentCatalog::from_document(&document).unwrap();
    for (name, key, width, height) in [
        ("Editor", "ProjectEditorOverlay", 440.0, 350.0),
        ("Confirm", "ConfirmationOverlay", 320.0, 220.0),
        ("Help", "ProjectHelpOverlay", 380.0, 280.0),
    ] {
        let definition = catalog
            .entries
            .iter()
            .find(|entry| entry.key.as_deref() == Some(key))
            .unwrap();
        let mut configuration = config(name, width, height);
        configuration.target = PreviewStageTarget::Definition {
            anchor: PreviewSourceAnchor::capture(&document, definition.node).unwrap(),
            options: DefinitionPreviewOptions {
                target_type: Some("ContentControl".into()),
                sample_content: String::new(),
                width,
                height,
            },
        };
        stages.push(workspace.add_preview_stage(configuration).unwrap());
    }
    workspace.set_preview_stages_visible(true).unwrap();
    settle(&window, &clock);
    workspace.refresh(true);
    settle(&window, &clock);
    for stage in workspace.preview_stages() {
        assert!(
            matches!(stage.status, PreviewStageStatus::Ready { .. }),
            "{}: {:?}",
            stage.configuration.name,
            stage.status
        );
    }
    let board_note = named::<TextBox>(
        &workspace.preview_stage_control(stages[0]).unwrap(),
        "WorkspaceNote",
    );
    let board_text = board_note.text();
    let link = |name: &str, source: usize, control: &str, action| PreviewPrototypeLink {
        name: name.into(),
        source: stages[source],
        trigger: anchor(control),
        action,
    };
    let centered = PreviewPrototypeOverlayOptions::default();
    let help = PreviewPrototypeOverlayOptions {
        placement: PreviewPrototypeOverlayPlacement::TopRight,
        dismiss_on_outside: true,
        dim_background: false,
    };
    workspace
        .set_preview_prototype(PreviewPrototype {
            flows: vec![PreviewPrototypeFlow {
                name: "Overlay review".into(),
                start: stages[0],
            }],
            links: vec![
                link(
                    "Project settings",
                    0,
                    "WorkspaceOpenEditor",
                    PreviewPrototypeAction::OpenOverlay {
                        stage: stages[1],
                        options: centered,
                    },
                ),
                link(
                    "Quick help",
                    0,
                    "WorkspaceOpenHelp",
                    PreviewPrototypeAction::OpenOverlay {
                        stage: stages[3],
                        options: help,
                    },
                ),
                link(
                    "Review changes",
                    1,
                    "EditorConfirm",
                    PreviewPrototypeAction::OpenOverlay {
                        stage: stages[2],
                        options: PreviewPrototypeOverlayOptions {
                            placement: PreviewPrototypeOverlayPlacement::BottomCenter,
                            ..centered
                        },
                    },
                ),
                link(
                    "Help with settings",
                    1,
                    "EditorHelp",
                    PreviewPrototypeAction::SwapOverlay {
                        stage: stages[3],
                        options: centered,
                    },
                ),
                link(
                    "Close settings",
                    1,
                    "EditorClose",
                    PreviewPrototypeAction::CloseOverlay,
                ),
                link(
                    "Continue editing",
                    2,
                    "ConfirmClose",
                    PreviewPrototypeAction::CloseOverlay,
                ),
                link(
                    "Return to settings",
                    3,
                    "HelpReturn",
                    PreviewPrototypeAction::SwapOverlay {
                        stage: stages[1],
                        options: centered,
                    },
                ),
                link(
                    "Close help",
                    3,
                    "HelpClose",
                    PreviewPrototypeAction::CloseOverlay,
                ),
            ],
        })
        .unwrap();
    workspace.set_preview_prototype_visible(true).unwrap();
    workspace.start_preview_prototype("Overlay review").unwrap();
    settle(&window, &clock);
    workspace.refresh(true);
    settle(&window, &clock);
    let note = named::<TextBox>(
        &workspace.preview_prototype_control(stages[0]).unwrap(),
        "WorkspaceNote",
    );
    note.set_text(Some("This note survives nested overlays"));
    settle(&window, &clock);
    workspace.refresh(false);
    settle(&window, &clock);
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();

    click(&workspace, &window, &clock, 0);
    let editor_name = named::<TextBox>(
        &workspace.preview_prototype_control(stages[1]).unwrap(),
        "EditorName",
    );
    editor_name.set_text(Some("Retained overlay draft"));
    let playback = workspace.preview_prototype_playback().unwrap();
    assert_eq!(playback.screen, stages[0]);
    assert_eq!(playback.current, stages[1]);
    assert_eq!(playback.overlays.len(), 1);
    let overlay_pixels = capture(
        &workspace,
        &clock,
        &directory.join("designer-prototype-overlay.png"),
    );
    let player = workspace.preview_prototype_player_view().unwrap();
    let editor_plane = named::<Control>(
        &player.control(),
        &format!("PrototypePlayerPlane{}", stages[1]),
    );
    let pixel = |point: Point| {
        let point = editor_plane
            .translate_point(point, &workspace.control())
            .unwrap();
        let offset = (point.y.round() as usize * 1600 + point.x.round() as usize) * 4;
        &overlay_pixels[offset..offset + 4]
    };
    assert_eq!(
        pixel(Point::new(2.0, 2.0)),
        pixel(Point::new(-3.0, 2.0)),
        "The rounded native overlay corner must reveal the dimmed retained view, without an opaque stage backing"
    );

    click(&workspace, &window, &clock, 2);
    let playback = workspace.preview_prototype_playback().unwrap();
    assert_eq!(playback.current, stages[2]);
    assert_eq!(playback.overlays.len(), 2);
    assert!(playback.history.is_empty());
    capture(
        &workspace,
        &clock,
        &directory.join("designer-prototype-overlay-stack.png"),
    );
    click(&workspace, &window, &clock, 5);
    assert_eq!(
        workspace.preview_prototype_playback().unwrap().current,
        stages[1]
    );
    assert_eq!(
        editor_name.text().as_deref(),
        Some("Retained overlay draft")
    );

    click(&workspace, &window, &clock, 3);
    assert_eq!(
        workspace.preview_prototype_playback().unwrap().current,
        stages[3]
    );
    click(&workspace, &window, &clock, 6);
    assert_eq!(
        workspace.preview_prototype_playback().unwrap().current,
        stages[1]
    );
    assert_eq!(
        editor_name.text().as_deref(),
        Some("Retained overlay draft")
    );
    click(&workspace, &window, &clock, 4);
    assert_eq!(
        workspace.preview_prototype_playback().unwrap().current,
        stages[0]
    );
    assert_eq!(
        note.text().as_deref(),
        Some("This note survives nested overlays")
    );
    assert_eq!(board_note.text(), board_text);

    click(&workspace, &window, &clock, 1);
    let player = workspace.preview_prototype_player_view().unwrap();
    let plane = named::<Control>(
        &player.control(),
        &format!("PrototypePlayerPlane{}", stages[3]),
    );
    assert_eq!(plane.bounds(), Rect::new(300.0, 0.0, 380.0, 280.0));
    let scrim = named::<Control>(
        &player.control(),
        &format!("PrototypeOverlayScrim{}", stages[3]),
    );
    let outside = scrim
        .translate_point(Point::new(24.0, 620.0), &window)
        .unwrap();
    let pressed = PointerPressedEventArgs::new(
        scrim.clone(),
        Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        &window,
        outside,
        1,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::NONE,
        1,
    );
    scrim.raise_event(&pressed);
    assert!(
        pressed.handled(),
        "The dismissing press must never activate the retained base"
    );
    settle(&window, &clock);
    assert!(workspace
        .preview_prototype_playback()
        .unwrap()
        .overlays
        .is_empty());
    assert_eq!(
        note.text().as_deref(),
        Some("This note survives nested overlays")
    );
    workspace.stop_preview_prototype();
    settle(&window, &clock);

    let overview = workspace.preview_prototype_view().unwrap();
    let anchor = named::<Control>(&overview.control(), "PrototypeAddLink");
    workspace
        .show_preview_prototype_link_editor(Some(1), None, &anchor)
        .unwrap();
    settle(&window, &clock);
    let editor = workspace.preview_prototype_link_editor().unwrap();
    assert!(window
        .get_visual_descendants()
        .any(|visual| visual.cast::<FlyoutPresenter>().is_some()));
    assert_eq!(
        named::<CheckBox>(&editor.control(), "PrototypeLinkOverlayDismiss").is_checked(),
        Some(true)
    );
    assert_eq!(
        named::<CheckBox>(&editor.control(), "PrototypeLinkOverlayDim").is_checked(),
        Some(false)
    );
    named::<ComboBox>(&editor.control(), "PrototypeLinkOverlayPlacement").set_selected_index(9);
    named::<TextBox>(&editor.control(), "PrototypeLinkOverlayX").set_text(Some("88"));
    named::<TextBox>(&editor.control(), "PrototypeLinkOverlayY").set_text(Some("64"));
    settle(&window, &clock);
    named::<ScrollViewer>(&editor.control(), "PrototypeLinkScroll").scroll_to_end();
    settle(&window, &clock);
    let apply = named::<Button>(&editor.control(), "PrototypeLinkApply");
    let position = apply.translate_point(Point::default(), &window).unwrap();
    assert!(position.y >= 0.0 && position.y + apply.bounds().height <= window.bounds().height);
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1600, 1040));
    bitmap.render(&window.clone().upcast());
    bitmap
        .save_to_file(
            directory
                .join("designer-prototype-overlay-editor.png")
                .to_str()
                .unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
    editor.cancel();
    assert_eq!(workspace.session().borrow().document().source(), SOURCE);
    assert!(!workspace.session().borrow().can_undo());
    window.close();
    drop(workspace);
    drop(app);
    drop(locator);
}
