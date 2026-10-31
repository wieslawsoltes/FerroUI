//! Native property-source grouping and options remain usable under the real WinUI theme.
use super::*;
use crate::{PropertyInspectorFilter, PropertyInspectorSort, PropertyInspectorView};
use ferroui_controls::{ComboBox, Flyout, TextBox};

#[test]
fn native_property_inspector_sources_and_view_options_render_without_authoring() {
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
    let source = r##"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Background="#F3F5F9">
  <Grid.Resources><SolidColorBrush x:Key="CardBrush" Color="#E0E7FF" /></Grid.Resources>
  <StackPanel Margin="42" Spacing="20">
    <TextBox x:Name="TitleInput" Text="A connected property inspector" />
    <Border x:Name="Card" Width="480" Padding="24" CornerRadius="12" Background="{StaticResource CardBrush}" Tag="{Binding Text, ElementName=TitleInput}">
      <Border.RenderTransform><TranslateTransform X="0" /></Border.RenderTransform>
      <StackPanel Spacing="12">
        <TextBlock Text="{Binding Text, ElementName=TitleInput}" FontSize="24" />
        <TextBlock Text="Browse local values, bindings, resources and object values. Follow a property back to its XAML definition." TextWrapping="Wrap" />
      </StackPanel>
    </Border>
  </StackPanel>
</Grid>"##;
    let workspace = DesignerWorkspace::new(source).unwrap();
    workspace.set_timeline_visible(false);
    workspace.set_layout_helpers_visible(false);
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let card = session.borrow().document().ast().find_name("Card").unwrap();
    session.borrow_mut().select([card]);
    workspace.set_property_inspector_view(PropertyInspectorView {
        sort: PropertyInspectorSort::ValueSource,
        filter: PropertyInspectorFilter::All,
    });
    workspace.fit_artboard().unwrap();
    let revision = session.borrow().document().revision();
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    capture(
        &workspace,
        &clock,
        &directory.join("designer-property-sources.png"),
    );

    let options = workspace
        .control()
        .get_control::<Button>("PropertyViewOptions");
    let flyout = options.flyout().unwrap().cast::<Flyout>().unwrap();
    flyout.popup().set_should_use_overlay_layer(true);
    flyout.show_at(&options);
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
    assert!(flyout.is_open());
    let sort = workspace.control().get_control::<ComboBox>("PropertySort");
    let filter = workspace
        .control()
        .get_control::<ComboBox>("PropertyFilter");
    assert!(sort.bounds().width > 180.0 && filter.bounds().width > 180.0);
    assert_eq!(sort.selected_index(), 2);
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1600, 1040));
    bitmap.render(&window.to_ref().upcast());
    bitmap
        .save_to_file(
            directory
                .join("designer-property-view-options.png")
                .to_str()
                .unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
    flyout.hide();
    workspace
        .control()
        .get_control::<TextBox>("PropertySearch")
        .set_text(Some("@resource"));
    capture(
        &workspace,
        &clock,
        &directory.join("designer-property-resource-filter.png"),
    );
    workspace.go_to_property_definition("Background").unwrap();
    assert_eq!(session.borrow().document().revision(), revision);
    assert_eq!(session.borrow().document().source(), source);
    assert_eq!(session.borrow().selection(), &[card]);
    assert!(!session.borrow().can_undo());
    window.close();
    drop(workspace);
    drop(app);
    drop(locator);
}
