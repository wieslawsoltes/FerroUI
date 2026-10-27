//! Full native workspace evidence: real pointer capture, semantic placement, immutable
//! drafts and a single source transaction on release. The sample is also runnable with --file.
use super::*;
use crate::DesignSurface;
use ferroui_base::input::{
    IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer,
    PointerEventArgs, PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::{Point, Ref, Visual};
use ferroui_designer_core::NodeId;

const SOURCE: &str = include_str!("../../docs/designer/samples/LayoutHelpers.xaml");
const SIZE: Size = Size::new(1600.0, 1040.0);
const INSERT: &str = r##"<Border Name="InsertedComponent" Width="104" Height="38" Background="#FFCCDBF7" CornerRadius="4" Padding="10,8"><TextBlock Text="New item" Foreground="#FF315D93" /></Border>"##;

struct PointerWorkspace {
    workspace: Rc<DesignerWorkspace>,
    window: Ref<Window>,
    surface: Ref<DesignSurface>,
    pointer: Rc<Pointer>,
}
impl PointerWorkspace {
    fn new() -> Self {
        let workspace = DesignerWorkspace::new(SOURCE).unwrap();
        workspace.set_artboard_size(880.0, 720.0).unwrap();
        workspace.set_timeline_visible(false);
        workspace.set_zoom(1.0);
        let surface = workspace.design_surface();
        surface.set_grid(0.0);
        let window = Window::new();
        window.set_width(SIZE.width);
        window.set_height(SIZE.height);
        window.set_content(Some(Control::boxed(workspace.control())));
        window.show();
        window.update_layout();
        let fixture = Self {
            workspace,
            window,
            surface,
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        };
        fixture.refresh();
        fixture
    }
    fn refresh(&self) {
        self.workspace.refresh(true);
        self.workspace.control().measure(SIZE);
        self.workspace.control().arrange(Rect::from_size(SIZE));
        self.window.update_layout();
        self.workspace
            .session()
            .borrow_mut()
            .refresh_runtime_mapping();
        assert!(self.workspace.session().borrow().can_manipulate());
    }
    fn id(&self, name: &str) -> NodeId {
        self.workspace
            .session()
            .borrow()
            .document()
            .ast()
            .find_name(name)
            .unwrap_or_else(|| panic!("Missing sample element {name}"))
    }
    fn select(&self, name: &str) {
        let id = self.id(name);
        self.workspace.session().borrow_mut().select([id]);
        self.refresh();
    }
    fn visual(&self, name: &str) -> Ref<Visual> {
        self.workspace
            .session()
            .borrow()
            .preview
            .visual(self.id(name))
            .unwrap_or_else(|| panic!("Missing native sample element {name}"))
    }
    fn at(&self, name: &str, x: f64, y: f64) -> Point {
        self.visual(name)
            .translate_point(Point::new(x, y), &self.surface)
            .unwrap()
    }
    fn position(&self, point: Point) -> Point {
        self.surface.translate_point(point, &self.window).unwrap()
    }
    fn press(&self, point: Point) {
        self.surface.raise_event(&PointerPressedEventArgs::new(
            self.surface.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(point),
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        ));
    }
    fn move_to(&self, point: Point) {
        self.surface.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            self.surface.to_ref(),
            self.pointer.clone(),
            Some(&self.window),
            self.position(point),
            2,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            KeyModifiers::NONE,
        ));
    }
    fn release(&self, point: Point) {
        self.surface.raise_event(&PointerReleasedEventArgs::new(
            self.surface.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(point),
            3,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            KeyModifiers::NONE,
            MouseButton::Left,
        ));
    }
    fn escape(&self) {
        let mut event = KeyEventArgs::new();
        event.key = Key::Escape;
        event.set_routed_event(Some(InputElement::key_down_event()));
        self.surface.raise_event(&event);
    }
    fn assert_unchanged(&self) {
        let session = self.workspace.session();
        let session = session.borrow();
        assert_eq!(
            session.document().source(),
            SOURCE,
            "A pointer draft must not author source"
        );
        assert!(
            !session.can_undo(),
            "A pointer draft must not create history"
        );
    }
    fn assert_draft(&self, target: &str, accepted: bool) -> Rc<crate::LayoutEditPreview> {
        let preview = self
            .surface
            .layout_edit_preview()
            .expect("Pointer motion must expose layout feedback");
        assert_eq!(
            preview.target(),
            self.id(target),
            "{}",
            preview.description()
        );
        assert_eq!(preview.is_accepted(), accepted, "{:?}", preview.rejection());
        self.assert_unchanged();
        preview
    }
    fn undo_once(&self) {
        let session = self.workspace.session();
        assert!(
            session.borrow().can_undo(),
            "Release must author one transaction"
        );
        session.borrow_mut().undo().unwrap();
        assert_eq!(session.borrow().document().source(), SOURCE);
        assert!(
            !session.borrow().can_undo(),
            "One undo must completely restore the gesture"
        );
        assert!(self.surface.layout_edit_preview().is_none());
        self.refresh();
    }
    fn capture(&self, clock: &RenderClock, directory: &Path, name: &str) -> Vec<u8> {
        let pixels = capture_current_at(&self.workspace, clock, &directory.join(name), SIZE);
        self.assert_unchanged();
        pixels
    }
    fn begin_insert(&self, target: &str, x: f64, y: f64) -> Point {
        self.select(target);
        let point = self.at(target, x, y);
        self.surface.arm_insert(INSERT.into());
        self.press(point);
        self.move_to(point);
        self.assert_draft(target, true);
        point
    }
    fn assert_inserted_into(&self, target: &str) {
        let session = self.workspace.session();
        let session = session.borrow();
        let inserted = session
            .document()
            .ast()
            .find_name("InsertedComponent")
            .unwrap();
        assert_eq!(
            session.document().ast().node(inserted).unwrap().parent,
            Some(self.id(target))
        );
    }
}
impl Drop for PointerWorkspace {
    fn drop(&mut self) {
        self.pointer.capture(None);
        self.window.close();
    }
}

#[test]
fn native_workspace_renders_layout_helper_interactions() {
    let locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()));
    let mut services = TestServices::styled_window();
    let clock = Rc::new(RenderClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
        now: Cell::new(0.0),
    });
    services.global_clock = Some(clock.clone());
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let window = MockWindowImpl::window(SIZE.width, SIZE.height);
        window.max_auto_size_hint.set(SIZE);
        let screen = PixelRect::new(0, 0, SIZE.width as i32, SIZE.height as i32);
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
    let fixture = PointerWorkspace::new();
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();

    // Cell placement shares the same measured tracks used for the visible Grid guides.
    let point = fixture.begin_insert("GridLayout", 164.0, 108.0);
    let grid = fixture.assert_draft("GridLayout", true);
    assert!(grid.helpers.scene.handles.iter().any(|handle| matches!(
        &handle.action,
        ferroui_designer_tools::LayoutHandleAction::GridTrack { .. }
    )));
    fixture.capture(&clock, &directory, "designer-helper-grid-cell.png");
    fixture.release(point);
    fixture.assert_inserted_into("GridLayout");
    {
        let session = fixture.workspace.session();
        let session = session.borrow();
        let inserted = session
            .document()
            .ast()
            .node(fixture.id("InsertedComponent"))
            .unwrap();
        assert_eq!(inserted.attribute("Grid.Row"), Some("1"));
        assert_eq!(inserted.attribute("Grid.Column"), Some("1"));
    }
    fixture.undo_once();

    // A spacing drag previews a shared layout property, rather than moving one child.
    fixture.select("StackLayout");
    let snapshot = fixture.surface.layout_helper_snapshot().unwrap();
    let handle = snapshot
        .scene
        .handles
        .iter()
        .find(|handle| handle.id == "Spacing")
        .unwrap();
    let start = snapshot.to_artboard.transform(handle.position).unwrap();
    let start = Point::new(start.x, start.y);
    let end = Point::new(start.x, start.y + 12.0);
    fixture.press(start);
    fixture.move_to(end);
    let draft = fixture.assert_draft("StackLayout", true);
    assert_eq!(
        draft
            .helpers
            .scene
            .handles
            .iter()
            .find(|handle| handle.id == "Spacing")
            .unwrap()
            .value,
        24.0
    );
    fixture.capture(&clock, &directory, "designer-helper-stack-gap.png");
    fixture.release(end);
    assert_eq!(
        fixture
            .workspace
            .session()
            .borrow()
            .document()
            .ast()
            .node(fixture.id("StackLayout"))
            .unwrap()
            .attribute("Spacing"),
        Some("24")
    );
    fixture.undo_once();

    // Reparent an actual authored control, retaining the original runtime tree until release.
    fixture.select("MovingCard");
    let original = fixture.visual("MovingCard");
    let original_bounds = original.bounds();
    // Use the authored Border's padding, away from resize/padding handles and its TextBlock.
    let start = fixture.at("MovingCard", 201.0, 36.0);
    let end = fixture.at("WrapLayout", 110.0, 19.0);
    fixture.press(start);
    fixture.move_to(end);
    assert_eq!(
        fixture.workspace.session().borrow().selection(),
        &[fixture.id("MovingCard")]
    );
    let draft = fixture.assert_draft("WrapLayout", true);
    let placement = draft.helpers.scene.placement.as_ref().unwrap();
    assert!(
        placement.bounds.width == 0.0 || placement.bounds.height == 0.0,
        "Wrap placement must show an insertion caret"
    );
    let pixels = fixture.capture(&clock, &directory, "designer-helper-wrap-reparent.png");
    assert_eq!(
        original.bounds(),
        original_bounds,
        "A drag ghost must not move the runtime control"
    );
    // A patch inside the translated coral control proves native pixels are present in the
    // drag image; the source remains in its original bottom-row slot throughout the draft.
    let ghost_patch = fixture
        .surface
        .translate_point(
            Point::new(end.x - 12.0, end.y - 16.0),
            &fixture.workspace.control(),
        )
        .unwrap();
    let coral = (-3..=3)
        .flat_map(|dy| (-3..=3).map(move |dx| (dx, dy)))
        .filter(|(dx, dy)| {
            let x = ghost_patch.x.round() as i32 + dx;
            let y = ghost_patch.y.round() as i32 + dy;
            let index = (y as usize * SIZE.width as usize + x as usize) * 4;
            let p = &pixels[index..index + 4];
            p[1] < 175 && ((p[0] > 200 && p[2] < 160) || (p[2] > 200 && p[0] < 160))
        })
        .count();
    assert!(
        coral > 12,
        "The drag must paint a cached native control, not just an outline ({coral} coral pixels)"
    );
    fixture.release(end);
    assert_eq!(
        fixture
            .workspace
            .session()
            .borrow()
            .document()
            .ast()
            .node(fixture.id("MovingCard"))
            .unwrap()
            .parent,
        Some(fixture.id("WrapLayout"))
    );
    fixture.undo_once();

    // Edge, cell and flow targets each retain their own dedicated placement geometry.
    for (target, x, y, file) in [
        ("DockLayout", 224.0, 95.0, "designer-helper-dock-edge.png"),
        (
            "UniformLayout",
            116.0,
            118.0,
            "designer-helper-uniform-cell.png",
        ),
        (
            "FlexLayout",
            80.0,
            19.0,
            "designer-helper-flex-insertion.png",
        ),
        (
            "EmptyContent",
            132.0,
            29.0,
            "designer-helper-content-slot.png",
        ),
    ] {
        let point = fixture.begin_insert(target, x, y);
        fixture.capture(&clock, &directory, file);
        fixture.release(point);
        fixture.assert_inserted_into(target);
        if target == "DockLayout" {
            assert_eq!(
                fixture
                    .workspace
                    .session()
                    .borrow()
                    .document()
                    .ast()
                    .node(fixture.id("InsertedComponent"))
                    .unwrap()
                    .attribute("DockPanel.Dock"),
                Some("Right")
            );
        }
        fixture.undo_once();
    }

    // An occupied content holder keeps both its content and the source unchanged on release.
    fixture.select("OccupiedContent");
    let point = fixture.at("OccupiedContent", 132.0, 29.0);
    fixture.surface.arm_insert(INSERT.into());
    fixture.press(point);
    fixture.move_to(point);
    fixture.assert_draft("OccupiedContent", false);
    fixture.capture(&clock, &directory, "designer-helper-content-rejected.png");
    fixture.release(point);
    fixture.assert_unchanged();
    assert!(fixture.surface.layout_edit_preview().is_none());

    // Escape restores an active shared-property draft without adding another gallery image.
    fixture.select("StackLayout");
    let snapshot = fixture.surface.layout_helper_snapshot().unwrap();
    let handle = snapshot
        .scene
        .handles
        .iter()
        .find(|handle| handle.id == "Spacing")
        .unwrap();
    let start = snapshot.to_artboard.transform(handle.position).unwrap();
    let start = Point::new(start.x, start.y);
    let end = Point::new(start.x, start.y + 8.0);
    fixture.press(start);
    fixture.move_to(end);
    fixture.assert_draft("StackLayout", true);
    fixture.escape();
    fixture.release(end);
    fixture.assert_unchanged();
    println!(
        "Designer native layout helper evidence: {}",
        directory.display()
    );
    drop(fixture);
    drop(app);
    locator.dispose();
}
