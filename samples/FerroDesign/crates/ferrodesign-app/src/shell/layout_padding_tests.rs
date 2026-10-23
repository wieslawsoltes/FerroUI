//! Compiled-XAML entry, native fields, routed keys and the shared save boundary.
use super::*;
use ferroui_base::input::KeyEventArgs;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::FerroLocator;
use ferroui_controls::{AppBuilder, Application, Window};
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use ferroui_themes_fluent::FluentTheme;
use ferroui_vello::{VelloApplicationExtensions, VelloOptions, VelloRenderingMode};

struct Fixture {
    shell: Rc<EditorShell>,
    window: Ref<Window>,
    id: NodeId,
    locator: Rc<dyn IDisposable>,
    _dispatcher: UnitTestDispatcherScope,
}
impl Fixture {
    fn new(initial: core::Padding, grid: bool, empty: bool) -> Self {
        let dispatcher = Dispatcher::unit_test_scope();
        AppBuilder::reset_setup_for_unit_tests();
        let locator = FerroLocator::enter_scope();
        crate::register_types();
        let builder = AppBuilder::configure::<Application>()
            .with_inter_font()
            .use_harfbuzz()
            .with(Rc::new(VelloOptions::with_rendering_mode(
                VelloRenderingMode::Cpu,
            )))
            .use_vello()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
            .setup_without_starting();
        builder
            .instance()
            .unwrap()
            .styles()
            .add(FluentTheme::new().as_style());
        let mut document = core::Document::new("Native padding fields");
        let mut frame = Node::new(NodeKind::Frame, "Layout", 40.0, 40.0, 320.0, 180.0);
        frame.layout = Some(core::AutoLayout {
            axis: if grid {
                LayoutAxis::Grid
            } else {
                LayoutAxis::Row
            },
            padding: initial,
            width: core::SizingMode::Fixed,
            height: core::SizingMode::Fixed,
            ..core::AutoLayout::default()
        });
        let id = document.insert_node(document.pages[0].id, frame).unwrap();
        if !empty {
            document
                .insert_node(
                    id,
                    Node::new(NodeKind::Rectangle, "Child", 0.0, 0.0, 40.0, 30.0),
                )
                .unwrap();
        }
        assert!(document.layout_all().errors.is_empty());
        let mut editor = Editor::new(document);
        editor.selection = vec![id];
        let shell = EditorShell::new();
        *shell.editor.borrow_mut() = editor;
        shell.refresh();
        let window = Window::new();
        window.set_width(1500.0);
        window.set_height(1000.0);
        window.set_content(Some(Control::boxed(shell.root.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let fixture = Self {
            shell,
            window,
            id,
            locator,
            _dispatcher: dispatcher,
        };
        fixture.stop_save_timer();
        fixture.open();
        fixture
    }
    fn open(&self) {
        let button = self.shell.button("EditLayoutButton");
        assert!(button.is_effectively_visible());
        assert!(button.focus());
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        self.stop_save_timer();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(self
            .control::<TextBox>(NAMES[0])
            .is_attached_to_visual_tree());
    }
    fn control<T: ObjectType>(&self, name: &str) -> Ref<T> {
        self.shell
            .root
            .get_visual_descendants()
            .into_iter()
            .find_map(|visual| {
                let control = visual.cast::<Control>()?;
                if control.name().as_deref() == Some(name) {
                    control.cast::<T>()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("Native control {name} is missing"))
    }
    fn field(&self, index: usize) -> Ref<TextBox> {
        self.control(NAMES[index])
    }
    fn key(&self, field: &Ref<TextBox>, key: Key) {
        assert!(field.is_focused());
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.set_routed_event(Some(InputElement::key_down_event()));
        field.raise_event(&event);
        self.stop_save_timer();
        assert!(event.handled());
    }
    fn submit(&self, index: usize, value: &str) {
        let field = self.field(index);
        assert!(field.focus());
        field.set_text(Some(value));
        self.key(&field, Key::Enter);
    }
    fn set_linked(&self, linked: bool) {
        self.control::<CheckBox>("LayoutPaddingLinked")
            .set_is_checked(Some(linked));
        self.stop_save_timer();
    }
    fn json(&self) -> String {
        self.shell.editor.borrow().document.to_json().unwrap()
    }
    fn padding(&self) -> core::Padding {
        self.shell.editor.borrow().document.nodes[&self.id]
            .layout
            .as_ref()
            .unwrap()
            .padding
    }
    fn stop_save_timer(&self) {
        if let Some(timer) = self.shell.pending_save_timer.borrow_mut().take() {
            timer.dispose();
        }
        self.shell.pending_save.set(false);
        platform::set_document_serialization_pending(false);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop_save_timer();
        self.window.close();
        self.stop_save_timer();
        self.locator.dispose();
        AppBuilder::reset_setup_for_unit_tests();
    }
}

#[test]
fn native_padding_diagnostics_publish_real_fields_and_export_controls() {
    let fixture = Fixture::new(core::Padding::all(10.0), false, false);
    let state: serde_json::Value = serde_json::from_str(&fixture.shell.state_json()).unwrap();
    assert_eq!(state["chrome"]["PaddingField"]["text"], "10");
    assert_eq!(state["chrome"]["LayoutPaddingTop"]["text"], "10");
    assert_eq!(state["chrome"]["LayoutPaddingLinked"]["checked"], true);
    assert!(state["chrome"]["ModalTitle"]["text"].as_str().unwrap().contains("Layout padding"));
    for name in ["ExportXamlButton", "VariablesRailButton", "PresentButton", "GapField", "CrossGapField"] {
        assert!(state["chrome"][name].is_object(), "Missing native control diagnostic {name}");
    }
}

#[test]
fn native_variable_binding_controls_reflow_padding_and_unbind_exactly_once() {
    let fixture = Fixture::new(core::Padding::all(10.0), false, false);
    fixture.shell.close_modal();
    let variable = fixture.shell.editor.borrow_mut().edit_variables(core::VariableEdit::Upsert {
        id: None, name: "Responsive spacing".into(), collection: "Spacing".into(), mode: "Default".into(),
        value: core::VariableValue::Number(24.25), replace_all_modes: false,
    }).unwrap().variable.unwrap();
    fixture.shell.refresh();
    let before = fixture.json(); let depth = fixture.shell.editor.borrow().history_len();
    let button = fixture.shell.button("VariablesRailButton");
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    Dispatcher::ui_thread().run_jobs(None);
    let choice = fixture.control::<ComboBox>("VariableBindingProperty");
    assert_eq!(choice.item_count(), 15);
    choice.set_selected_index(12); // Top, Right, Bottom, Left follow the nine original bindings.
    fixture.control::<Button>("VariableBind").raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    fixture.stop_save_timer(); Dispatcher::ui_thread().run_jobs(None);
    let after = fixture.json();
    assert_eq!(fixture.padding().left, 24.25);
    assert_eq!(fixture.shell.editor.borrow().document.nodes[&fixture.id].variable_bindings["layout.padding.left"], variable);
    assert_eq!(fixture.shell.editor.borrow().history_len(), depth + 1);
    fixture.shell.close_modal();
    assert!(fixture.shell.editor.borrow_mut().undo()); assert_eq!(fixture.json(), before);
    assert!(fixture.shell.editor.borrow_mut().redo()); assert_eq!(fixture.json(), after);
    fixture.shell.refresh();
    fixture.shell.button("VariablesRailButton").raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    Dispatcher::ui_thread().run_jobs(None);
    fixture.control::<ComboBox>("VariableBindingProperty").set_selected_index(12);
    fixture.control::<Button>("VariableUnbind").raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    fixture.stop_save_timer(); Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(fixture.padding().left, 24.25);
    assert!(!fixture.shell.editor.borrow().document.nodes[&fixture.id].variable_bindings.contains_key("layout.padding.left"));
    assert_eq!(fixture.shell.editor.borrow().history_len(), depth + 2);
}

#[test]
fn native_fractional_padding_edit_relinquishes_only_the_changed_side_variable() {
    let fixture = Fixture::new(core::Padding::all(10.0), false, false);
    fixture.shell.close_modal();
    let variable = {
        let mut editor = fixture.shell.editor.borrow_mut();
        let variable = editor.edit_variables(core::VariableEdit::Upsert {
            id: None, name: "Spacing".into(), collection: "Spacing".into(), mode: "Default".into(),
            value: core::VariableValue::Number(10.0), replace_all_modes: false,
        }).unwrap().variable.unwrap();
        for property in ["layout.padding.top", "layout.padding.left", "layout.gap"] {
            editor.edit_variables(core::VariableEdit::Bind { nodes: vec![fixture.id], property: property.into(), variable: Some(variable) }).unwrap();
        }
        variable
    };
    fixture.shell.refresh(); fixture.open(); fixture.set_linked(false);
    let before = fixture.json(); let depth = fixture.shell.editor.borrow().history_len();
    fixture.submit(3, "31.25");
    assert_eq!(fixture.padding().left, 31.25);
    {
        let editor = fixture.shell.editor.borrow();
        let node = &editor.document.nodes[&fixture.id];
        assert!(!node.variable_bindings.contains_key("layout.padding.left"));
        assert_eq!(node.variable_bindings["layout.padding.top"], variable);
        assert_eq!(node.variable_bindings["layout.gap"], variable);
        assert_eq!(editor.history_len(), depth + 1);
    }
    let after = fixture.json(); fixture.shell.close_modal();
    assert!(fixture.shell.editor.borrow_mut().undo()); assert_eq!(fixture.json(), before);
    assert!(fixture.shell.editor.borrow_mut().redo()); assert_eq!(fixture.json(), after);
}

#[test]
fn native_empty_layout_exposes_padding_and_link_mode_never_equalizes_on_open_or_toggle() {
    let initial = core::Padding {
        top: 10.5,
        right: 20.25,
        bottom: 30.75,
        left: 40.125,
    };
    let fixture = Fixture::new(initial, false, true);
    let before = fixture.json();
    assert_eq!(
        fixture.shell.field("PaddingField").text().as_deref(),
        Some("")
    );
    assert_eq!(
        fixture.shell.field("PaddingField").placeholder_text().as_deref(),
        Some("Mixed")
    );
    assert_eq!(
        fixture
            .control::<CheckBox>("LayoutPaddingLinked")
            .is_checked(),
        Some(false)
    );
    for index in 0..4 {
        assert_eq!(
            fixture.field(index).text(),
            Some(number(values(initial)[index]))
        );
    }
    fixture.set_linked(true);
    fixture.set_linked(false);
    fixture.set_linked(true);
    assert_eq!(fixture.json(), before);
    assert_eq!(fixture.shell.editor.borrow().history_len(), 0);
    fixture.submit(1, "18.5");
    assert_eq!(fixture.padding(), core::Padding::all(18.5));
    for index in 0..4 {
        assert_eq!(fixture.field(index).text().as_deref(), Some("18.50"));
    }
    assert_eq!(
        fixture.shell.field("PaddingField").text().as_deref(),
        Some("18.50")
    );
    assert_eq!(fixture.shell.editor.borrow().history_len(), 1);
}

#[test]
fn native_unlinked_edit_preserves_unedited_precision_and_grid_tracks_share_the_new_snapshot() {
    let initial = core::Padding {
        top: 10.125,
        right: 20.5625,
        bottom: 30.9375,
        left: 40.375,
    };
    let fixture = Fixture::new(initial, true, false);
    fixture.submit(0, "12.75");
    assert_eq!(
        fixture.padding(),
        core::Padding {
            top: 12.75,
            ..initial
        }
    );
    let columns = fixture.control::<TextBox>("LayoutColumnTracks");
    assert!(columns.focus());
    columns.set_text(Some("80, 1fr"));
    fixture.key(&columns, Key::Enter);
    assert_eq!(
        fixture.shell.editor.borrow().document.nodes[&fixture.id]
            .layout
            .as_ref()
            .unwrap()
            .column_tracks,
        vec![core::GridTrack::Fixed(80.0), core::GridTrack::Fraction(1.0)]
    );
    fixture.submit(3, "44.25");
    assert_eq!(
        fixture.padding(),
        core::Padding {
            top: 12.75,
            left: 44.25,
            ..initial
        }
    );
    assert_eq!(fixture.shell.editor.borrow().history_len(), 3);
    assert!(!fixture.control::<TextBlock>("LayoutError").is_visible());
}

#[test]
fn native_all_padding_drafts_validate_atomically_and_escape_discards_the_group() {
    let fixture = Fixture::new(core::Padding::all(10.0), false, false);
    fixture.set_linked(false);
    let before = fixture.json();
    let top = fixture.field(0);
    let right = fixture.field(1);
    assert!(top.focus());
    top.set_text(Some("NaN"));
    assert!(right.focus());
    right.set_text(Some("28.5"));
    fixture.key(&right, Key::Enter);
    assert!(right.is_focused(), "Rejected Enter retains field ownership");
    assert!(fixture.control::<TextBlock>("LayoutError").is_visible());
    assert_eq!(fixture.json(), before);
    assert_eq!(fixture.shell.editor.borrow().history_len(), 0);
    assert!(!fixture.shell.flush_inspector_drafts());
    fixture.stop_save_timer();
    assert_eq!(fixture.json(), before);
    fixture.set_linked(true);
    assert_eq!(
        fixture
            .control::<CheckBox>("LayoutPaddingLinked")
            .is_checked(),
        Some(false)
    );
    assert_eq!(top.text().as_deref(), Some("NaN"));
    assert_eq!(right.text().as_deref(), Some("28.5"));
    assert!(right.focus());
    fixture.key(&right, Key::Escape);
    for index in 0..4 {
        assert_eq!(fixture.field(index).text().as_deref(), Some("10"));
    }
    assert_eq!(fixture.json(), before);
    assert!(fixture.shell.flush_inspector_drafts());
    fixture.stop_save_timer();
    assert!(!fixture.control::<TextBlock>("LayoutError").is_visible());

    // Several dirty native controls are committed as one complete value.
    assert!(top.focus());
    top.set_text(Some("-1"));
    assert!(right.focus());
    right.set_text(Some("28.5"));
    top.set_text(Some("14.25"));
    fixture.key(&right, Key::Enter);
    assert_eq!(
        fixture.padding(),
        core::Padding {
            top: 14.25,
            right: 28.5,
            bottom: 10.0,
            left: 10.0
        }
    );
    assert_eq!(fixture.shell.editor.borrow().history_len(), 1);
}

#[test]
fn native_padding_flush_uses_the_existing_save_boundary_once_and_equal_submit_preserves_redo() {
    let fixture = Fixture::new(core::Padding::all(10.0), false, false);
    let before = fixture.json();
    let field = fixture.field(2);
    assert!(field.focus());
    field.set_text(Some("24.5"));
    assert_eq!(fixture.json(), before);
    assert!(fixture.shell.flush_inspector_drafts());
    fixture.stop_save_timer();
    assert_eq!(fixture.padding(), core::Padding::all(24.5));
    let after = fixture.json();
    assert!(fixture.shell.flush_inspector_drafts());
    fixture.stop_save_timer();
    assert_eq!(fixture.json(), after);
    assert_eq!(fixture.shell.editor.borrow().history_len(), 1);
    fixture.shell.close_modal();
    fixture.stop_save_timer();
    assert!(fixture.shell.editor.borrow_mut().undo());
    fixture.shell.refresh();
    fixture.open();
    assert_eq!(fixture.json(), before);
    fixture.set_linked(false);
    fixture.set_linked(true);
    fixture.submit(0, "10.00");
    assert_eq!(fixture.json(), before);
    assert_eq!(fixture.shell.editor.borrow().history_len(), 0);
    assert!(fixture.shell.editor.borrow().can_redo());
    fixture.shell.close_modal();
    fixture.stop_save_timer();
    assert!(fixture.shell.editor.borrow_mut().redo());
    assert_eq!(fixture.json(), after);
}

#[test]
fn native_padding_rejects_external_changes_and_editor_replacement_even_with_equal_ids() {
    for replace in [false, true] {
        let fixture = Fixture::new(core::Padding::all(10.0), false, false);
        let field = fixture.field(0);
        assert!(field.focus());
        field.set_text(Some("28.25"));
        if replace {
            let document = fixture.shell.editor.borrow().document.clone();
            let mut editor = Editor::new(document);
            editor.selection = vec![fixture.id];
            *fixture.shell.editor.borrow_mut() = editor;
        } else {
            fixture
                .shell
                .editor
                .borrow_mut()
                .update_node(fixture.id, "External change", |node| node.opacity = 0.75);
        }
        let before = fixture.json();
        let depth = fixture.shell.editor.borrow().history_len();
        fixture.key(&field, Key::Enter);
        assert!(fixture
            .control::<TextBlock>("LayoutError")
            .text()
            .unwrap_or_default()
            .contains("frame changed"));
        assert!(fixture.control::<TextBlock>("LayoutError").is_visible());
        assert!(field.is_focused());
        assert_eq!(field.text().as_deref(), Some("28.25"));
        assert_eq!(fixture.json(), before);
        assert_eq!(fixture.shell.editor.borrow().history_len(), depth);
        fixture.key(&field, Key::Escape);
        assert_eq!(field.text().as_deref(), Some("10"));
        assert_eq!(fixture.json(), before);
    }
}
