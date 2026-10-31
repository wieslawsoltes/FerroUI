//! Actual property-inspector popup, native binding evaluation and transaction ownership.
use super::*;
use ferroui_base::Ref;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{ComboBox, Expander, Flyout, TextBlock, TextBox};

fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing native binding control {name}"))
}
fn click(root: &Control, title: &str) {
    root.get_logical_descendants()
        .filter_map(|control| control.cast::<Button>())
        .find(|button| {
            button
                .content()
                .as_ref()
                .and_then(|value| value.downcast_ref::<String>())
                .is_some_and(|value| value == title)
        })
        .unwrap()
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn native_binding_workbench_renders_and_commits_one_guarded_transaction() {
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
    let source = r##"<Grid xmlns="https://github.com/ferroui" Background="#FFF1F4F9"><Border Width="420" Padding="32" Background="White" CornerRadius="16" HorizontalAlignment="Center" VerticalAlignment="Center"><StackPanel Spacing="16"><TextBlock Text="CONNECTED PROPERTIES" Foreground="#FF64748B" FontSize="12"/><TextBlock Name="PreviewTitle" Tag="Structured bindings" Text="{Binding Text, ElementName=WorkspaceName, FallbackValue='New workspace'}" FontSize="28" FontWeight="SemiBold"/><TextBlock Text="Edit the source field to update the heading through a native XAML binding." TextWrapping="Wrap"/><TextBox Name="WorkspaceName" Text="FerroUI Studio"/><Button Content="Continue" HorizontalAlignment="Stretch"/></StackPanel></Border></Grid>"##;
    let workspace = DesignerWorkspace::new(source).unwrap();
    workspace.set_timeline_visible(false);
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("PreviewTitle")
        .unwrap();
    session.borrow_mut().select([target]);
    workspace
        .control()
        .get_control::<TextBox>("PropertySearch")
        .set_text(Some("Text"));
    workspace.refresh(true);
    window.update_layout();
    let value = workspace
        .property_grid()
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find(|control| AutomationProperties::get_name(control).as_deref() == Some("Text value"))
        .unwrap()
        .cast::<Grid>()
        .unwrap();
    let more = value.children().get(1).cast::<Button>().unwrap();
    let flyout = more.flyout().unwrap().cast::<Flyout>().unwrap();
    flyout.popup().set_should_use_overlay_layer(true);
    flyout.show_at(&more);
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
    assert!(flyout.is_open());
    let body = Control::from_boxed(&flyout.content().unwrap()).unwrap();
    assert_eq!(
        named::<ComboBox>(&body, "PropertyBindingSource").selected_index(),
        2
    );
    assert_eq!(
        named::<TextBox>(&body, "PropertyBindingElement")
            .text()
            .as_deref(),
        Some("WorkspaceName")
    );
    named::<ComboBox>(&body, "PropertyBindingMode").set_selected_index(1);
    named::<Expander>(&body, "PropertyBindingAdvanced").set_is_expanded(true);
    assert_eq!(
        session.borrow().document().source(),
        source,
        "Staging a binding must not mutate the document"
    );
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
    let apply = named::<Button>(&body, "PropertyValueApply");
    let position = apply
        .translate_point(ferroui_base::Point::default(), &body)
        .unwrap();
    assert!(
        position.y >= 0.0 && position.y + apply.bounds().height <= body.bounds().height + 0.5,
        "Apply must remain visible when advanced binding fields require scrolling"
    );
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1600, 1040));
    bitmap.render(&window.clone().upcast());
    bitmap
        .save_to_file(
            directory
                .join("designer-binding-editor.png")
                .to_str()
                .unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
    // Use the real row's guarded commit, then replay the retained popup after undo.
    named::<ComboBox>(&body, "PropertyBindingSource").set_selected_index(3);
    named::<TextBox>(&body, "PropertyBindingPath").set_text(Some("Tag"));
    let revision = session.borrow().document().revision();
    click(&body, "Apply value");
    assert_eq!(session.borrow().document().revision(), revision + 1);
    assert_eq!(
        session
            .borrow()
            .preview
            .visual(target)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Structured bindings")
    );
    capture(
        &workspace,
        &clock,
        &directory.join("designer-binding-preview.png"),
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), source);
    named::<TextBox>(&body, "PropertyBindingPath").set_text(Some("Width"));
    click(&body, "Apply value");
    assert_eq!(
        session.borrow().document().source(),
        source,
        "A retained binding editor cannot commit after undo"
    );
    window.close();
    drop(app);
    drop(locator);
}
