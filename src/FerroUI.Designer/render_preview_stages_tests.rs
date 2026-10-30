//! Native stage-board evidence: independent document/state/template roots, theme
//! inheritance, responsive card layout and the real staged editor popup.
use super::*;
use crate::{
    DefinitionPreviewOptions, PreviewEnvironment, PreviewFlowDirection, PreviewSourceAnchor,
    PreviewStageConfiguration, PreviewStageExportOptions, PreviewStageLayout, PreviewStageMode,
    PreviewStageState, PreviewStageStatus, PreviewStageTarget, PreviewTheme,
};
use ferroui_base::{Point, Ref};
use ferroui_controls::{Border, CheckBox, ComboBox, ScrollViewer, TextBlock, TextBox};
use ferroui_designer_tools::ComponentCatalog;

const SOURCE: &str = include_str!("../../docs/designer/samples/PreviewStages.xaml");

fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing stage control {name}"))
}

fn settle(window: &Window, clock: &RenderClock) {
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
}

fn capture_window(window: &Window, clock: &RenderClock, path: &Path) {
    settle(window, clock);
    let bitmap = RenderTargetBitmap::new(PixelSize::new(
        window.bounds().width as i32,
        window.bounds().height as i32,
    ));
    bitmap.render(&window.to_ref().upcast());
    bitmap
        .save_to_file(
            path.to_str().unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
}

#[test]
fn native_preview_stages_render_desktop_phone_state_template_and_staged_editor() {
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
        DesignerWorkspace::with_uri(SOURCE, "file:///PreviewStages.xaml".into()).unwrap();
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
    let uri = workspace.session().borrow().document_uri().to_owned();
    let owner =
        PreviewSourceAnchor::capture(&document, document.ast().find_name("PreviewScene").unwrap())
            .unwrap();
    let mut stages = Vec::new();
    for (name, width, height, theme, flow, state) in [
        (
            "Desktop · content",
            1280.0,
            800.0,
            PreviewTheme::Light,
            PreviewFlowDirection::LeftToRight,
            "Content",
        ),
        (
            "Phone · loading",
            390.0,
            760.0,
            PreviewTheme::Dark,
            PreviewFlowDirection::LeftToRight,
            "Loading",
        ),
        (
            "Tablet · error / RTL",
            768.0,
            1024.0,
            PreviewTheme::Light,
            PreviewFlowDirection::RightToLeft,
            "Error",
        ),
    ] {
        let mut configuration = PreviewStageConfiguration::new(
            name,
            &uri,
            PreviewEnvironment {
                width,
                height,
                theme,
                flow_direction: flow,
            },
        );
        configuration.states = vec![PreviewStageState {
            owner: owner.clone(),
            group: "Screen".into(),
            state: state.into(),
        }];
        stages.push(workspace.add_preview_stage(configuration).unwrap());
    }
    let definition = ComponentCatalog::from_document(&document)
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.key.as_deref() == Some("ActionPreviewTemplate"))
        .unwrap();
    let mut template = PreviewStageConfiguration::new(
        "Action · template",
        &uri,
        PreviewEnvironment {
            width: 640.0,
            height: 420.0,
            ..Default::default()
        },
    );
    template.target = PreviewStageTarget::Definition {
        anchor: PreviewSourceAnchor::capture(&document, definition.node).unwrap(),
        options: DefinitionPreviewOptions {
            target_type: Some("Button".into()),
            sample_content: "Continue to workspace".into(),
            width: 320.0,
            height: 72.0,
        },
    };
    stages.push(workspace.add_preview_stage(template).unwrap());
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
    let state_visible = |id, name: &str| {
        let host = workspace.preview_stage_control(id).unwrap();
        host.get_visual_descendants()
            .filter_map(|visual| visual.cast::<Border>())
            .find(|control| control.name().as_deref() == Some(name))
            .unwrap()
            .is_visible()
    };
    assert!(state_visible(stages[0], "ContentPanel"));
    assert!(!state_visible(stages[0], "LoadingPanel"));
    assert!(state_visible(stages[1], "LoadingPanel"));
    assert!(!state_visible(stages[1], "ContentPanel"));
    assert!(state_visible(stages[2], "ErrorPanel"));
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    capture(
        &workspace,
        &clock,
        &directory.join("designer-preview-stages.png"),
    );

    workspace.set_canvas_focused(false);
    window.set_width(980.0);
    window.set_height(900.0);
    settle(&window, &clock);
    capture_at(
        &workspace,
        &clock,
        &directory.join("designer-preview-stages-narrow.png"),
        Size::new(980.0, 900.0),
    );
    let view = workspace.preview_stage_view().unwrap();
    let scroll = named::<Control>(&view.control(), "PreviewStageScroll");
    for id in &stages {
        let card = named::<Control>(&view.control(), &format!("PreviewStage{id}"));
        let point = card.translate_point(Point::default(), &scroll).unwrap();
        assert!(
            point.x >= 0.0 && point.x + card.bounds().width <= scroll.bounds().width + 0.5,
            "A stage card must wrap inside the narrow native viewport"
        );
    }

    window.set_width(1600.0);
    window.set_height(1040.0);
    settle(&window, &clock);
    let anchor = named::<Button>(&view.control(), &format!("Stage{}Options", stages[1]));
    workspace
        .show_preview_stage_editor(Some(stages[1]), &anchor)
        .unwrap();
    settle(&window, &clock);
    let editor = workspace.preview_stage_editor().unwrap();
    named::<TextBox>(&editor.control(), "StageName").set_text(Some("Phone · loading / dark"));
    let apply = named::<Button>(&editor.control(), "StageEditorApply");
    let point = apply.translate_point(Point::default(), &window).unwrap();
    assert!(point.x >= 0.0 && point.x + apply.bounds().width <= window.bounds().width + 0.5);
    assert!(point.y >= 0.0 && point.y + apply.bounds().height <= window.bounds().height + 0.5);
    capture_window(
        &window,
        &clock,
        &directory.join("designer-preview-stage-editor.png"),
    );
    editor.cancel();

    // The next capture exercises the new fitting policy and display-only zoom on
    // the same accepted native graphs, with live text surviving logical resizing.
    workspace.set_canvas_focused(true);
    workspace.remove_preview_stage(stages[2]).unwrap();
    let mut desktop = workspace.preview_stage(stages[0]).unwrap().configuration;
    desktop.name = "Desktop · resized / Live".into();
    desktop.mode = PreviewStageMode::Live;
    workspace
        .update_preview_stage(stages[0], desktop.clone())
        .unwrap();
    settle(&window, &clock);
    let input = named::<TextBox>(
        &workspace.preview_stage_control(stages[0]).unwrap(),
        "SearchInput",
    );
    input.set_text(Some("Live text survives layout changes"));
    desktop.environment.width = 720.0;
    desktop.environment.height = 500.0;
    workspace.update_preview_stage(stages[0], desktop).unwrap();
    let mut phone = workspace.preview_stage(stages[1]).unwrap().configuration;
    phone.name = "Phone · fit height / Loading".into();
    phone.layout = PreviewStageLayout::FixedWidth;
    workspace.update_preview_stage(stages[1], phone).unwrap();
    let mut component = workspace.preview_stage(stages[3]).unwrap().configuration;
    component.name = "Action · fit content / 100%".into();
    component.layout = PreviewStageLayout::SizeThatFits;
    workspace
        .update_preview_stage(stages[3], component)
        .unwrap();
    settle(&window, &clock);
    named::<ComboBox>(&view.control(), &format!("Stage{}Zoom", stages[0])).set_selected_index(2);
    named::<ComboBox>(&view.control(), &format!("Stage{}Zoom", stages[3])).set_selected_index(4);
    settle(&window, &clock);
    assert_eq!(
        named::<TextBox>(
            &workspace.preview_stage_control(stages[0]).unwrap(),
            "SearchInput"
        ),
        input,
        "Changing logical constraints must retain the original native input"
    );
    assert_eq!(
        input.text().as_deref(),
        Some("Live text survives layout changes")
    );
    let fitted = workspace.preview_stage(stages[3]).unwrap();
    assert_eq!(
        fitted.layout_size,
        crate::PreviewStageSize {
            width: 368.0,
            height: 120.0
        },
        "Native definition fitting includes the projection's existing 24-unit padding"
    );
    capture(
        &workspace,
        &clock,
        &directory.join("designer-preview-fitting.png"),
    );
    let exported = workspace
        .preview_stage_image(stages[1], PreviewStageExportOptions::default())
        .unwrap();
    let phone = workspace.preview_stage(stages[1]).unwrap();
    assert_eq!(exported.pixel_width, phone.layout_size.width.ceil() as u32);
    assert_eq!(
        exported.pixel_height,
        phone.layout_size.height.ceil() as u32
    );
    #[cfg(feature = "mcp")]
    {
        use base64::Engine;
        use ferroui_mcp::{Permissions, UiToolProvider};
        use serde_json::json;

        let provider = crate::mcp::DesignerMcp::with_workspace(&workspace);
        let permissions = Permissions {
            allow_property_writes: false,
            allow_input: false,
        };
        let before = provider
            .call("designer_preview_stages", &json!({}), permissions)
            .unwrap();
        let image = provider
            .call(
                "designer_preview_image",
                &json!({"stage":stages[1],"expected_model":before["model"],"scale":1.0}),
                permissions,
            )
            .unwrap();
        assert_eq!(image["mimeType"], "image/png");
        assert_eq!(image["stage"], stages[1]);
        assert_eq!(image["source_uri"], uri);
        assert_eq!(image["rendered_revision"], 0);
        assert_eq!(image["width"], exported.pixel_width);
        assert_eq!(image["height"], exported.pixel_height);
        assert_eq!(image["logical_width"], phone.layout_size.width);
        assert_eq!(image["logical_height"], phone.layout_size.height);
        assert_eq!(image["model"], before["model"]);
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(image["data"].as_str().unwrap())
                .unwrap(),
            exported.png
        );
        assert_eq!(
            provider
                .call("designer_preview_stages", &json!({}), permissions)
                .unwrap(),
            before,
            "Reading a preview image cannot rebuild, mutate or synchronize the board"
        );
    }
    std::fs::write(directory.join("designer-preview-export.png"), exported.png).unwrap();

    let anchor = named::<Button>(&view.control(), &format!("Stage{}Options", stages[0]));
    workspace
        .show_preview_stage_matrix_editor(stages[0], &anchor)
        .unwrap();
    let matrix = workspace.preview_stage_matrix_editor().unwrap();
    for name in [
        "MatrixSize0",
        "MatrixSize2",
        "MatrixTheme0",
        "MatrixTheme1",
        "MatrixState0_1",
        "MatrixState0_2",
    ] {
        named::<CheckBox>(&matrix.control(), name).set_is_checked(Some(true));
    }
    settle(&window, &clock);
    assert_eq!(matrix.draft().count().unwrap(), 8);
    assert!(named::<TextBlock>(&matrix.control(), "MatrixSummary")
        .text()
        .unwrap_or_default()
        .starts_with("8 new stages"));
    let matrix_apply = named::<Button>(&matrix.control(), "MatrixApply");
    assert!(matrix_apply.is_enabled());
    let point = matrix_apply
        .translate_point(Point::default(), &window)
        .unwrap();
    assert!(point.x >= 0.0 && point.x + matrix_apply.bounds().width <= window.bounds().width + 0.5);
    assert!(
        point.y >= 0.0 && point.y + matrix_apply.bounds().height <= window.bounds().height + 0.5
    );
    let matrix_scroll = named::<ScrollViewer>(&matrix.control(), "MatrixScroll");
    let data_heading = matrix
        .control()
        .get_logical_descendants()
        .filter_map(|control| control.cast::<TextBlock>())
        .find(|control| control.text().as_deref() == Some("Data projection"))
        .unwrap();
    let offset = data_heading
        .translate_point(Point::default(), &matrix_scroll)
        .unwrap()
        .y
        + matrix_scroll.offset().y
        - 4.0;
    matrix_scroll.set_offset(ferroui_base::Vector::new(0.0, offset.max(0.0)));
    capture_window(
        &window,
        &clock,
        &directory.join("designer-preview-matrix.png"),
    );
    matrix.cancel();
    assert_eq!(workspace.session().borrow().document().source(), SOURCE);
    assert_eq!(workspace.session().borrow().document().revision(), 0);
    assert!(!workspace.session().borrow().can_undo());
    window.close();
    drop(apply);
    drop(editor);
    drop(matrix);
    drop(view);
    drop(window);
    drop(workspace);
    drop(app);
    locator.dispose();
}
