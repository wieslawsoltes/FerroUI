//! The real native color flyout must paint under the workspace's WinUI host theme.
use super::*;
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelRect, PixelSize, Size};
use ferroui_controls::Window;
use ferroui_controls::platform::{IScreenImpl, ITopLevelImpl};
use ferroui_controls::testing::{
    MockScreenImpl, MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
    mock_screen,
};
use ferroui_controls_color_picker::primitives::ColorSpectrum;
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};

struct ColorRenderClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl IObservable<TimeSpan> for ColorRenderClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for ColorRenderClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, state: PlayState) {
        self.state.set(state);
    }
}
impl IGlobalClock for ColorRenderClock {}

#[test]
fn native_color_flyout_renders_spectrum_and_alpha_under_winui_chrome() {
    let locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()));
    let clock = Rc::new(ColorRenderClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
    });
    let mut services = TestServices::styled_window();
    services.global_clock = Some(clock.clone());
    services.render_interface =
        FerroLocator::current().get_service::<dyn IPlatformRenderInterface>();
    services.font_manager_impl = FerroLocator::current().get_service::<dyn IFontManagerImpl>();
    services.text_shaper_impl = FerroLocator::current().get_service::<dyn ITextShaperImpl>();
    services.theme = Some(Rc::new(|| {
        ferroui_themes_winui::WinUITheme::new().as_style()
    }));
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let window = MockWindowImpl::window(640.0, 760.0);
        window.max_auto_size_hint.set(Size::new(640.0, 760.0));
        let screen = PixelRect::new(0, 0, 1280, 900);
        window.setup_feature::<dyn IScreenImpl>(MockScreenImpl::new(vec![mock_screen(
            1.0, screen, screen, true,
        )]));
        window.setup_resize(|window, size, reason| {
            if window.client_size.get() != size {
                window.client_size.set(size);
                if let Some(resized) = ITopLevelImpl::resized(window) {
                    resized(size, reason);
                }
            }
        });
        window
    }));
    let app = UnitTestApplication::start(services);
    ferroui_themes_winui::register_types();
    if let Some(application) = ferroui_controls::Application::current() {
        application.set_requested_theme_variant(Some(ferroui_base::styling::ThemeVariant::dark()));
        FerroLocator::current_mutable()
            .bind::<dyn ferroui_base::styling::IThemeVariantHost>()
            .to_constant(application.as_theme_variant_host());
    }

    let panel = Grid::new();
    panel.classes().add("designer-chrome");
    panel.styles().add(crate::designer_chrome_style().unwrap());
    panel.set_background(Some(SolidColorBrush::from_uint32(0xff25262b).into()));
    panel.set_row_definitions(RowDefinitions::parse("56,28,*").unwrap());
    let heading = TextBlock::new();
    heading.set_text(Some("Appearance"));
    heading.set_font_size(12.0);
    heading.set_font_weight(FontWeight::SemiBold);
    heading.set_margin(Thickness::new(24.0, 20.0, 0.0, 0.0));
    panel.children().add(heading.upcast::<Control>());
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("96,240").unwrap());
    row.set_margin(Thickness::new(24.0, 0.0, 0.0, 0.0));
    let label = TextBlock::new();
    label.set_text(Some("Background"));
    label.set_font_size(11.0);
    label.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    row.children().add(label.upcast::<Control>());
    let editor = PropertyColorEditor::new().unwrap();
    editor.configure(Some("#B3668DFF"), false).unwrap();
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    editor.on_commit(move |_| observed.set(observed.get() + 1));
    Grid::set_column(&editor.control(), 1);
    row.children().add(editor.control());
    Grid::set_row(&row, 1);
    panel.children().add(row.upcast::<Control>());
    let window = Window::new();
    window.set_width(640.0);
    window.set_height(760.0);
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    window.update_layout();
    editor.flyout().popup().set_should_use_overlay_layer(true);
    editor.flyout().show_at(&editor.swatch_button());
    for tick in 0..6 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.subject.on_next(TimeSpan::from_seconds(tick as f64));
        window.update_layout();
    }
    assert!(editor.flyout().is_open());
    let spectrum = editor
        .color_view()
        .get_visual_descendants()
        .find_map(|control| control.cast::<ColorSpectrum>())
        .expect("the reusable native color spectrum must be realized");
    assert!(spectrum.bounds().width > 100.0 && spectrum.bounds().height > 100.0);
    for tab in editor
        .color_view()
        .get_visual_descendants()
        .filter_map(|control| control.cast::<ferroui_controls::TabItem>())
    {
        let right = tab
            .translate_point(
                ferroui_base::Point::new(tab.bounds().width, 0.0),
                &editor.color_view(),
            )
            .unwrap()
            .x;
        assert!(
            right <= editor.color_view().bounds().width,
            "color tabs must fit the native popup without clipping: tab {:?}, width {}, min {}, right {}, view {:?}, parent {:?}",
            tab.bounds(),
            tab.width(),
            tab.min_width(),
            right,
            editor.color_view().bounds(),
            tab.visual_parent().map(|parent| parent.bounds())
        );
    }
    assert_eq!(
        changes.get(),
        0,
        "rendering and the popup seed are not authored edits"
    );
    assert_eq!(
        editor.color_view().color(),
        Color::parse("#B3668DFF").unwrap()
    );

    let bitmap = RenderTargetBitmap::new(PixelSize::new(640, 760));
    bitmap.render(&window.clone().upcast());
    let mut pixels = vec![0; 640 * 760 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 640, 760), &mut pixels, 640 * 4);
    let distinct = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| (pixel[0], pixel[1], pixel[2]))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        distinct.len() > 1000,
        "the native spectrum should paint a continuous color field"
    );
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-redesign-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    bitmap
        .save_to_file(
            directory
                .join("designer-color-editor.png")
                .to_str()
                .unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    editor.flyout().hide();
    window.close();
    drop(app);
    drop(locator);
}
