use super::*;
use ferroui_base::input::{
    ContextRequestedEventArgs, MouseButton, Pointer, PointerEventArgs, PointerPointProperties,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind,
    RawInputModifiers,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Vector;
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::testing::{
    MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
};
use ferroui_controls::Window;

const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui"
    xmlns:ui="https://github.com/ferroui"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    Width="500" Height="360" Background="White">
  <Canvas.Resources>
    <ControlTemplate x:Key="SampleTemplate" TargetType="Button">
      <StackPanel><Button Name="TemplateFirst" Width="100" Height="35"/><Button Name="TemplateSecond" Width="100" Height="35"/></StackPanel>
    </ControlTemplate>
  </Canvas.Resources>
  <Button Name="Back" Canvas.Left="40" Canvas.Top="40" Width="140" Height="60"/>
  <Button Name="Front" Canvas.Left="40" Canvas.Top="40" Width="140" Height="60"/>
  <ui:Button Name="Aliased" Canvas.Left="220" Canvas.Top="40" Width="100" Height="40"/>
  <ToggleButton Name="Derived" Canvas.Left="220" Canvas.Top="100" Width="100" Height="40"/>
  <Canvas Name="Group" Canvas.Left="40" Canvas.Top="160" Width="200" Height="120">
    <Button Name="GroupFirst" Width="100" Height="40"/>
    <Button Name="GroupSecond" Canvas.Top="50" Width="100" Height="40"/>
  </Canvas>
  <Button Name="Hidden" Canvas.Left="350" Canvas.Top="40" Width="100" Height="40"/>
  <Button Name="Locked" Canvas.Left="350" Canvas.Top="100" Width="100" Height="40"/>
  <Button Name="RuntimeHidden" IsVisible="False" Width="100" Height="40"/>
  <Button Name="Templated" Template="{StaticResource SampleTemplate}" Canvas.Left="280" Canvas.Top="180" Width="150" Height="130"/>
</Canvas>"#;

fn services() -> TestServices {
    let mut services = TestServices::styled_window();
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let native = MockWindowImpl::window(1400.0, 950.0);
        native.max_auto_size_hint.set(Size::new(2048.0, 2048.0));
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
    pointer: Rc<Pointer>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

impl Fixture {
    fn new() -> Self {
        let workspace =
            DesignerWorkspace::with_uri(SOURCE, "memory://selection.xaml".into()).unwrap();
        workspace.set_timeline_visible(false);
        workspace.set_layout_helpers_visible(false);
        let window = Window::new();
        window.set_width(1400.0);
        window.set_height(950.0);
        window.set_content(Some(Control::boxed(workspace.control())));
        window.show();
        window.update_layout();
        workspace.session.borrow_mut().refresh_runtime_mapping();
        Self {
            workspace,
            window,
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        }
    }

    fn id(&self, name: &str) -> NodeId {
        self.workspace
            .session
            .borrow()
            .document()
            .ast()
            .find_name(name)
            .unwrap()
    }

    fn point(&self) -> Point {
        let front = self
            .workspace
            .session
            .borrow()
            .preview
            .visual(self.id("Front"))
            .unwrap();
        front
            .translate_point(
                Point::new(front.bounds().width / 2.0, front.bounds().height / 2.0),
                &self.workspace.surface,
            )
            .unwrap()
    }

    fn capture(&self) -> CanvasSelectionSnapshot {
        self.workspace
            .capture_canvas_selection(self.point())
            .unwrap()
    }

    fn native_position(&self) -> Point {
        self.workspace
            .surface
            .translate_point(self.point(), &self.window)
            .unwrap()
    }

    fn context_request(&self) -> bool {
        let pointer = PointerEventArgs::new(
            Some(InputElement::pointer_released_event()),
            self.workspace.surface.to_ref(),
            self.pointer.clone(),
            Some(&self.window),
            self.native_position(),
            1,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::RightButtonReleased,
            ),
            KeyModifiers::NONE,
        );
        let event = ContextRequestedEventArgs::from_pointer_event_args(&pointer);
        self.workspace.surface.raise_event(&event);
        event.handled()
    }
}

fn item_from(value: Option<ferroui_base::BoxedValue>) -> Ref<MenuItem> {
    Ref::<MenuItem>::from_xaml_object(&value.unwrap()).unwrap()
}

fn layer_menu_choice(
    menu: &ContextMenu,
    snapshot: &CanvasSelectionSnapshot,
    node: NodeId,
) -> Ref<MenuItem> {
    let layers = item_from(menu.items().to_vec()[0].clone());
    let index = snapshot
        .choices()
        .iter()
        .position(|choice| choice.node == node)
        .unwrap();
    item_from(layers.items().to_vec()[index].clone())
}

#[test]
fn overlap_picker_keeps_native_order_and_selection_until_a_guarded_menu_choice() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    let seed = fixture.id("Aliased");
    workspace.session.borrow_mut().select([seed]);
    workspace.session.borrow_mut().set_read_only(true);
    let revision = workspace.session.borrow().document().revision();
    let generation = workspace.session.borrow().generation();
    let snapshot = fixture.capture();
    assert_eq!(snapshot.position(), fixture.point());
    assert_eq!(
        snapshot
            .choices()
            .iter()
            .map(|choice| choice.node)
            .collect::<Vec<_>>(),
        workspace.surface.hit_stack(fixture.point()),
    );
    assert_eq!(snapshot.choices()[0].node, fixture.id("Front"));
    assert!(snapshot
        .choices()
        .iter()
        .any(|choice| choice.node == fixture.id("Back")));
    assert!(snapshot.choices()[0].label.contains("Button"));
    assert!(snapshot.choices()[0].label.contains("Front"));
    assert!(snapshot.choices()[0].path.ends_with("Button#Front"));
    let menu = workspace.create_canvas_selection_menu(&snapshot).unwrap();
    assert_eq!(workspace.session.borrow().selection(), [seed]);
    assert_eq!(workspace.session.borrow().generation(), generation);
    let back = layer_menu_choice(&menu, &snapshot, fixture.id("Back"));
    back.raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    assert_eq!(workspace.session.borrow().selection(), [fixture.id("Back")]);
    assert_eq!(workspace.session.borrow().document().revision(), revision);
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    assert!(!workspace.session.borrow().can_undo());
    assert!(workspace
        .select_canvas_layer(&snapshot, fixture.id("Front"))
        .is_err());
}

#[test]
fn overlap_picker_rejects_non_candidates_source_changes_and_reused_document_ids() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new();
    let snapshot = fixture.capture();
    let back = fixture.id("Back");
    let stale_menu = fixture
        .workspace
        .create_canvas_selection_menu(&snapshot)
        .unwrap();
    assert!(fixture
        .workspace
        .select_canvas_layer(&snapshot, fixture.id("Aliased"))
        .is_err());
    assert!(fixture.workspace.session.borrow().selection().is_empty());
    fixture
        .workspace
        .session
        .borrow_mut()
        .set_property(&[back], "Content", "Changed")
        .unwrap();
    fixture.workspace.session.borrow_mut().undo().unwrap();
    fixture.window.update_layout();
    assert_eq!(
        fixture.workspace.session.borrow().document().source(),
        SOURCE
    );
    assert!(fixture
        .workspace
        .select_canvas_layer(&snapshot, back)
        .is_err());
    layer_menu_choice(&stale_menu, &snapshot, back)
        .raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    assert!(fixture.workspace.session.borrow().selection().is_empty());
    let fresh = fixture.capture();
    fixture
        .workspace
        .session
        .borrow_mut()
        .open_document(SOURCE.into(), "memory://other.xaml".into())
        .unwrap();
    fixture.window.update_layout();
    fixture
        .workspace
        .session
        .borrow_mut()
        .refresh_runtime_mapping();
    assert!(fixture.workspace.select_canvas_layer(&fresh, back).is_err());
}

#[test]
fn overlap_picker_rejects_layer_round_trips_camera_and_runtime_geometry_changes() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new();
    let front = fixture.id("Front");
    let snapshot = fixture.capture();
    fixture
        .workspace
        .session
        .borrow_mut()
        .set_layers_locked(&[front], true)
        .unwrap();
    fixture
        .workspace
        .session
        .borrow_mut()
        .set_layers_locked(&[front], false)
        .unwrap();
    assert!(fixture
        .workspace
        .select_canvas_layer(&snapshot, front)
        .is_err());
    let snapshot = fixture.capture();
    fixture
        .workspace
        .pan_canvas(Vector::new(17.0, 9.0))
        .unwrap();
    assert!(fixture
        .workspace
        .select_canvas_layer(&snapshot, front)
        .is_err());
    let snapshot = fixture.capture();
    let stale_menu = fixture
        .workspace
        .create_canvas_selection_menu(&snapshot)
        .unwrap();
    let generation = fixture.workspace.session.borrow().generation();
    let visual = fixture
        .workspace
        .session
        .borrow()
        .preview
        .visual(front)
        .unwrap();
    visual.cast::<Control>().unwrap().set_width(145.0);
    fixture.window.update_layout();
    assert_eq!(fixture.workspace.session.borrow().generation(), generation);
    assert!(fixture
        .workspace
        .select_canvas_layer(&snapshot, front)
        .is_err());
    layer_menu_choice(&stale_menu, &snapshot, front)
        .raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    assert!(fixture.workspace.session.borrow().selection().is_empty());
    assert_eq!(
        fixture.workspace.session.borrow().document().source(),
        SOURCE
    );
}

#[test]
fn matching_type_uses_exact_native_identity_preserves_seed_and_filters_presentation() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    let seed = fixture.id("Aliased");
    let hidden = fixture.id("Hidden");
    let locked = fixture.id("Locked");
    let group = fixture.id("Group");
    let derived = fixture.id("Derived");
    {
        let mut session = workspace.session.borrow_mut();
        session.select([seed, derived]);
        session.set_layers_hidden(&[hidden], true).unwrap();
        session.set_layers_locked(&[locked, group], true).unwrap();
        session.set_read_only(true);
    }
    let revision = workspace.session.borrow().document().revision();
    workspace.select_matching_native_type().unwrap();
    assert_eq!(
        workspace.session.borrow().selection(),
        [
            seed,
            fixture.id("Back"),
            fixture.id("Front"),
            fixture.id("Templated")
        ]
    );
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    assert_eq!(workspace.session.borrow().document().revision(), revision);
    assert!(!workspace.session.borrow().can_undo());
    assert!(workspace
        .control()
        .get_control::<MenuItem>("LayerSelectMatchingType")
        .is_enabled());
    let generation = workspace.session.borrow().generation();
    workspace.select_matching_native_type().unwrap();
    assert_eq!(
        workspace.session.borrow().generation(),
        generation,
        "an unchanged selection is a no-op"
    );
}

#[test]
fn matching_type_is_confined_to_isolation_and_reusable_definition_scope() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new();
    let group = fixture.id("Group");
    let first = fixture.id("GroupFirst");
    let second = fixture.id("GroupSecond");
    {
        let mut session = fixture.workspace.session.borrow_mut();
        session.isolate_layers(&[group]).unwrap();
        session.select([second]);
        session.select_matching_native_type().unwrap();
        assert_eq!(session.selection(), [second, first]);
        session.clear_layer_isolation();
        let template = session
            .document()
            .ast()
            .elements()
            .find(|node| node.attribute("x:Key") == Some("SampleTemplate"))
            .unwrap()
            .id;
        session.enter_definition_scope(template).unwrap();
    }
    fixture.window.update_layout();
    fixture
        .workspace
        .session
        .borrow_mut()
        .refresh_runtime_mapping();
    let template_first = fixture.id("TemplateFirst");
    let template_second = fixture.id("TemplateSecond");
    let mut session = fixture.workspace.session.borrow_mut();
    session.select([template_second]);
    session.select_matching_native_type().unwrap();
    assert_eq!(session.selection(), [template_second, template_first]);
    assert_eq!(session.document().source(), SOURCE);
    assert!(!session.can_undo());
}

#[test]
fn overlap_picker_selects_template_instance_and_exposes_parts_only_in_definition_scope() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new();
    let instance = fixture.id("Templated");
    let first = fixture.id("TemplateFirst");
    let second = fixture.id("TemplateSecond");
    let native_instance = fixture
        .workspace
        .session
        .borrow()
        .preview
        .visual(instance)
        .unwrap();
    let part = native_instance
        .get_visual_descendants()
        .find(|visual| visual.name().as_deref() == Some("TemplateFirst"))
        .expect("the native template realizes its authored button");
    let center = Point::new(part.bounds().width / 2.0, part.bounds().height / 2.0);
    let point = part
        .translate_point(center, &fixture.workspace.surface)
        .unwrap();
    let snapshot = fixture.workspace.capture_canvas_selection(point).unwrap();
    assert!(snapshot
        .choices()
        .iter()
        .any(|choice| choice.node == instance));
    assert!(snapshot
        .choices()
        .iter()
        .all(|choice| choice.node != first && choice.node != second));
    assert!(fixture
        .workspace
        .select_canvas_layer(&snapshot, first)
        .is_err());
    let template = fixture
        .workspace
        .session
        .borrow()
        .document()
        .ast()
        .elements()
        .find(|node| node.attribute("x:Key") == Some("SampleTemplate"))
        .unwrap()
        .id;
    fixture
        .workspace
        .session
        .borrow_mut()
        .enter_definition_scope(template)
        .unwrap();
    fixture.window.update_layout();
    fixture
        .workspace
        .session
        .borrow_mut()
        .refresh_runtime_mapping();
    let part = fixture
        .workspace
        .session
        .borrow()
        .preview
        .visual(first)
        .unwrap();
    let point = part
        .translate_point(
            Point::new(part.bounds().width / 2.0, part.bounds().height / 2.0),
            &fixture.workspace.surface,
        )
        .unwrap();
    let snapshot = fixture.workspace.capture_canvas_selection(point).unwrap();
    assert!(snapshot.choices().iter().any(|choice| choice.node == first));
    assert!(snapshot
        .choices()
        .iter()
        .all(|choice| choice.node != instance));
    fixture
        .workspace
        .select_canvas_layer(&snapshot, first)
        .unwrap();
    assert_eq!(fixture.workspace.session.borrow().selection(), [first]);
    assert_eq!(
        fixture.workspace.session.borrow().document().source(),
        SOURCE
    );
}

#[test]
fn selection_commands_leave_pending_source_untouched() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new();
    let seed = fixture.id("Front");
    fixture.workspace.session.borrow_mut().select([seed]);
    let snapshot = fixture.capture();
    fixture.workspace.source_editor().set_text(Some("<Canvas>"));
    let revision = fixture.workspace.session.borrow().document().revision();
    let generation = fixture.workspace.session.borrow().generation();
    let syntax = fixture.workspace.session.borrow().document().ast_snapshot();
    assert!(fixture
        .workspace
        .capture_canvas_selection(fixture.point())
        .is_err());
    assert!(fixture.workspace.select_matching_native_type().is_err());
    assert!(fixture
        .workspace
        .select_canvas_layer(&snapshot, fixture.id("Back"))
        .is_err());
    assert_eq!(
        fixture.workspace.source_editor().text().as_deref(),
        Some("<Canvas>")
    );
    assert_eq!(
        fixture.workspace.session.borrow().document().source(),
        "<Canvas>"
    );
    assert_eq!(
        fixture.workspace.session.borrow().document().revision(),
        revision
    );
    assert_eq!(fixture.workspace.session.borrow().generation(), generation);
    assert!(Arc::ptr_eq(
        &syntax,
        &fixture.workspace.session.borrow().document().ast_snapshot()
    ));
    assert_eq!(fixture.workspace.session.borrow().selection(), [seed]);
    assert!(!fixture.context_request());
}

#[test]
fn native_context_requests_preserve_selection_and_respect_pan_interact_and_insert_owners() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new();
    let seed = fixture.id("Aliased");
    fixture.workspace.session.borrow_mut().select([seed]);
    let surface = fixture.workspace.design_surface();
    let press = PointerPressedEventArgs::new(
        surface.to_ref(),
        fixture.pointer.clone(),
        &fixture.window,
        fixture.native_position(),
        1,
        PointerPointProperties::new(
            RawInputModifiers::RIGHT_MOUSE_BUTTON,
            PointerUpdateKind::RightButtonPressed,
        ),
        KeyModifiers::NONE,
        1,
    );
    surface.raise_event(&press);
    assert_eq!(fixture.workspace.session.borrow().selection(), [seed]);
    let release = PointerReleasedEventArgs::new(
        surface.to_ref(),
        fixture.pointer.clone(),
        &fixture.window,
        fixture.native_position(),
        2,
        PointerPointProperties::new(
            RawInputModifiers::NONE,
            PointerUpdateKind::RightButtonReleased,
        ),
        KeyModifiers::NONE,
        MouseButton::Right,
    );
    surface.raise_event(&release);
    assert!(release.handled());
    assert_eq!(fixture.workspace.session.borrow().selection(), [seed]);
    fixture.workspace.set_canvas_pan_tool(true);
    assert!(!fixture.context_request());
    fixture.workspace.set_canvas_pan_tool(false);
    surface.set_interactive(true);
    assert!(!fixture.context_request());
    surface.set_interactive(false);
    surface.arm_insert("<Button/>".into());
    assert!(!fixture.context_request());
    assert_eq!(
        fixture.workspace.session.borrow().document().source(),
        SOURCE
    );
    assert_eq!(fixture.workspace.session.borrow().selection(), [seed]);
}

#[test]
fn matching_type_menu_works_from_source_view_and_keyboard_context_uses_selected_bounds() {
    let _app = UnitTestApplication::start(services());
    let fixture = Fixture::new();
    let seed = fixture.id("Front");
    fixture.workspace.session.borrow_mut().select([seed]);
    fixture
        .workspace
        .set_view_mode(crate::WorkspaceViewMode::Xaml);
    let item = fixture
        .workspace
        .control()
        .get_control::<MenuItem>("LayerSelectMatchingType");
    item.raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    assert_eq!(
        fixture.workspace.session.borrow().selection().first(),
        Some(&seed)
    );
    assert!(fixture
        .workspace
        .session
        .borrow()
        .selection()
        .contains(&fixture.id("Back")));
    fixture
        .workspace
        .set_view_mode(crate::WorkspaceViewMode::Design);
    fixture.window.update_layout();
    let selection = fixture.workspace.session.borrow().selection().to_vec();
    let event = ContextRequestedEventArgs::new();
    fixture.workspace.surface.raise_event(&event);
    assert!(event.handled());
    assert_eq!(fixture.workspace.session.borrow().selection(), selection);
    assert_eq!(
        fixture.workspace.session.borrow().document().source(),
        SOURCE
    );
}
