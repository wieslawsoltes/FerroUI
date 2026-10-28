//! Native end-to-end evidence for property transfer, measured arrangement and overlap
//! selection. Popups use the actual WinUI templates inside the headless overlay host.
use super::*;
use crate::{LayoutMatch, PreviewStatus};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelRect, PixelSize, Point, Size};
use ferroui_controls::platform::{IScreenImpl, ITopLevelImpl};
use ferroui_controls::primitives::Popup;
use ferroui_controls::testing::{
    mock_screen, MockScreenImpl, MockWindowImpl, MockWindowingPlatform, TestServices,
    UnitTestApplication,
};
use ferroui_controls::{Border, MenuFlyout, MenuFlyoutPresenter, Window};
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};
use std::path::Path;

struct RenderClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
    now: Cell<f64>,
}
impl IObservable<TimeSpan> for RenderClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for RenderClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, state: PlayState) {
        self.state.set(state);
    }
}
impl IGlobalClock for RenderClock {}

fn settle(window: &Window, clock: &RenderClock) {
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.now.set(clock.now.get() + 1.0);
        clock
            .subject
            .on_next(TimeSpan::from_seconds(clock.now.get()));
        window.update_layout();
    }
}

fn capture(window: &Ref<Window>, clock: &RenderClock, path: &Path) {
    settle(window, clock);
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1600, 1040));
    bitmap.render(&window.clone().upcast());
    bitmap
        .save_to_file(
            path.to_str().unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
}

fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing selection workflow control {name}"))
}

fn resource_color(control: &Control, key: &str) -> Color {
    let value = control
        .try_find_resource(&key.into(), control.actual_theme_variant().as_ref())
        .unwrap();
    from_markup_value::<Rc<dyn IBrush>>(&value)
        .unwrap()
        .as_solid_color_brush()
        .unwrap()
        .color()
}

fn assert_submenu_background(item: &MenuItem) {
    let popup = item
        .get_template_descendants()
        .into_iter()
        .find_map(|visual| visual.cast::<Popup>())
        .unwrap();
    let backdrop = popup
        .child()
        .unwrap()
        .get_visual_children()
        .iter()
        .find_map(|visual| visual.cast::<Border>())
        .unwrap();
    let brush = backdrop.background().unwrap();
    assert_eq!(brush.opacity(), 1.0);
    assert_eq!(brush.as_solid_color_brush().unwrap().color().a, 255);
}

#[test]
fn native_selection_workflows_render_and_commit_through_their_real_controls() {
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
    let source = include_str!("../../docs/designer/samples/SelectionWorkflows.xaml");
    let workspace = DesignerWorkspace::new(source).unwrap();
    workspace
        .open_document(source.to_owned(), "file:///SelectionWorkflows.xaml".into())
        .unwrap();
    workspace.set_timeline_visible(false);
    workspace.set_layout_helpers_visible(false);
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    workspace.fit_artboard().unwrap();
    let session = workspace.session();
    let id = |name: &str| session.borrow().document().ast().find_name(name).unwrap();
    let original = id("SourceCard");
    let destinations = [id("ResearchCard"), id("PrototypeCard")];
    let back = id("BackCard");
    let front = id("FrontCard");
    let application_menu_color = resource_color(
        &session.borrow().preview.root().unwrap(),
        "MenuFlyoutPresenterBackground",
    );
    assert!(matches!(
        session.borrow().preview_status(),
        PreviewStatus::Ready { .. }
    ));
    workspace.synchronize_language_service().unwrap();
    let uri = session.borrow().document_uri().to_owned();
    let diagnostics = workspace
        .try_language_service()
        .unwrap()
        .diagnostics(&uri)
        .unwrap();
    assert!(
        diagnostics.iter().all(|diagnostic| diagnostic.severity > 2),
        "{diagnostics:?}"
    );

    session.borrow_mut().select([original]);
    workspace.refresh(true);
    workspace
        .control()
        .get_control::<MenuItem>("PropertyCopyAppearance")
        .raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    assert!(workspace.appearance_clipboard().is_some());
    session.borrow_mut().select(destinations);
    workspace.refresh(true);
    workspace
        .control()
        .get_control::<MenuItem>("PropertyPasteAppearance")
        .raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    settle(&window, &clock);
    let flyout = workspace.property_transfer.flyout.borrow().clone().unwrap();
    assert!(flyout.is_open());
    let editor = workspace.property_transfer_editor().unwrap();
    let body = editor.control();
    let apply = named::<Button>(&body, "PropertyTransferApply");
    let position = apply.translate_point(Point::default(), &body).unwrap();
    assert!(
        position.y >= 0.0 && position.y + apply.bounds().height <= body.bounds().height + 0.5,
        "Apply must remain visible below the scrolling compatibility list"
    );
    assert!(
        position.x >= 0.0 && position.x + apply.bounds().width <= body.bounds().width + 0.5,
        "Apply must fit inside the editor's constrained width"
    );
    let presenter = flyout.popup().child().unwrap();
    let position = apply.translate_point(Point::default(), &presenter).unwrap();
    assert!(
        position.x >= 12.0
            && position.x + apply.bounds().width <= presenter.bounds().width - 12.0 + 0.5,
        "Apply must fit inside the flyout's padded viewport: {position:?}, {:?}, {:?}",
        apply.bounds(),
        presenter.bounds(),
    );
    let position = apply.translate_point(Point::default(), &window).unwrap();
    assert!(
        position.x >= 0.0 && position.x + apply.bounds().width <= window.bounds().width + 0.5,
        "Apply must stay onscreen when its anchor is next to the window edge"
    );
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    assert_eq!(
        session.borrow().document().source(),
        source,
        "Opening the staged paste UI must not write XAML"
    );
    capture(
        &window,
        &clock,
        &directory.join("designer-property-transfer.png"),
    );
    let revision = session.borrow().document().revision();
    apply.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(session.borrow().document().revision(), revision + 1);
    assert!(!flyout.is_open());
    for target in destinations {
        assert_eq!(
            session
                .borrow()
                .preview
                .visual(target)
                .unwrap()
                .cast::<Border>()
                .unwrap()
                .corner_radius()
                .top_left,
            18.0
        );
    }

    session
        .borrow_mut()
        .select([original, destinations[0], destinations[1]]);
    workspace.refresh(true);
    window.update_layout();
    let arrange = workspace
        .control()
        .get_control::<Button>("ArrangeSelection");
    let menu = arrange.flyout().unwrap().cast::<MenuFlyout>().unwrap();
    menu.show_at(&arrange);
    settle(&window, &clock);
    let match_menu = workspace
        .control()
        .get_control::<MenuItem>("LayoutMatchMenu");
    match_menu.set_is_sub_menu_open(true);
    settle(&window, &clock);
    let presenter = menu
        .popup()
        .child()
        .unwrap()
        .cast::<MenuFlyoutPresenter>()
        .unwrap();
    let background = presenter.background().unwrap();
    assert_eq!(background.opacity(), 1.0);
    assert_eq!(background.as_solid_color_brush().unwrap().color().a, 255);
    assert_submenu_background(&match_menu);
    capture(
        &window,
        &clock,
        &directory.join("designer-selection-layout.png"),
    );
    menu.hide();
    let revision = session.borrow().document().revision();
    workspace.match_selection_size(LayoutMatch::Size).unwrap();
    assert_eq!(session.borrow().document().revision(), revision + 1);
    window.update_layout();
    for target in destinations {
        let visual = session.borrow().preview.visual(target).unwrap();
        assert_eq!(visual.bounds().size(), Size::new(220.0, 164.0));
    }

    session.borrow_mut().select([front]);
    workspace.refresh(true);
    window.update_layout();
    let accepted_source = session.borrow().document().source().to_owned();
    let snapshot = workspace
        .capture_canvas_selection(Point::new(224.0, 426.0))
        .unwrap();
    assert!(snapshot.choices().iter().any(|choice| choice.node == back));
    assert!(snapshot.choices().iter().any(|choice| choice.node == front));
    let overlap = workspace.create_canvas_selection_menu(&snapshot).unwrap();
    workspace.open_canvas_selection_menu(&overlap, snapshot.position());
    settle(&window, &clock);
    let expected_position = workspace
        .design_surface()
        .translate_point(snapshot.position(), &window)
        .unwrap();
    let menu_position = overlap.translate_point(Point::default(), &window).unwrap();
    assert!(
        (menu_position.x - expected_position.x).abs() <= 32.0
            && (menu_position.y - expected_position.y).abs() <= 32.0,
        "The overlap menu must open at its transformed canvas point: {menu_position:?}, expected {expected_position:?}"
    );
    assert_eq!(
        overlap
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color()
            .a,
        255,
    );
    let layers = Control::from_boxed(&overlap.items().get_at(0).unwrap())
        .unwrap()
        .cast::<MenuItem>()
        .unwrap();
    layers.set_is_sub_menu_open(true);
    settle(&window, &clock);
    assert_submenu_background(&layers);
    assert_eq!(
        resource_color(
            &session.borrow().preview.root().unwrap(),
            "MenuFlyoutPresenterBackground",
        ),
        application_menu_color,
        "Editor popup resources must not replace the application's preview theme"
    );
    capture(
        &window,
        &clock,
        &directory.join("designer-overlap-picker.png"),
    );
    assert_eq!(session.borrow().selection(), [front]);
    assert_eq!(session.borrow().document().source(), accepted_source);
    let index = snapshot
        .choices()
        .iter()
        .position(|choice| choice.node == back)
        .unwrap();
    let item = Control::from_boxed(&layers.items().get_at(index).unwrap())
        .unwrap()
        .cast::<MenuItem>()
        .unwrap();
    item.raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    assert_eq!(session.borrow().selection(), [back]);
    assert_eq!(session.borrow().document().source(), accepted_source);
    overlap.close();
    session.borrow_mut().undo().unwrap();
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), source);
    window.close();
    drop(overlap);
    drop(menu);
    drop(flyout);
    drop(editor);
    drop(body);
    drop(apply);
    drop(session);
    drop(window);
    drop(workspace);
    drop(app);
    locator.dispose();
}
