use super::*;
use ferroui_base::input::{DataTransfer, DragEventArgs, KeyModifiers};
use ferroui_base::interactivity::Interactive;
use ferroui_base::{BoxedValue, Point};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, StackPanel, Window};
use ferroui_controls_treedatagrid::primitives::TreeDataGridRow;
use ferroui_controls_treedatagrid::{TreeDataGrid, TreeDataGridRowDragStartedEventArgs};
use ferroui_treedatagrid_core::IndexPath;

const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui">
  <StackPanel Name="Left" xmlns:p="clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls">
    <!-- authored comment -->
    <p:Button Name="A" Content="A"/>
    <Button Name="B" Content="B"/>
  </StackPanel>
  <StackPanel Name="Right"><StackPanel.Children><Button Name="C" Content="C"/></StackPanel.Children></StackPanel>
  <Border Name="Empty"/>
  <Button Name="Occupied" Content="Existing"/>
  <Button Name="TextOccupied">Existing text</Button>
</StackPanel>"#;

fn new_session(source: &str) -> DesignerSession {
    DesignerSession::new(source, ContentControl::new()).unwrap()
}

fn named(session: &DesignerSession, name: &str) -> NodeId {
    session.document().ast().find_name(name).unwrap()
}

fn content_children(session: &DesignerSession, owner: NodeId) -> Vec<NodeId> {
    let container = ContentHost::of_owner(session, owner).unwrap().container;
    session.document().ast().element_children(container)
}

#[test]
fn outline_reparent_preserves_source_order_namespace_identity_preview_and_single_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = new_session(SOURCE);
    let (a, b, c, left, right) = (
        named(&session, "A"),
        named(&session, "B"),
        named(&session, "C"),
        named(&session, "Left"),
        named(&session, "Right"),
    );
    session.select([b, a]);
    let drag = OutlineDrag::capture(&session, [b, a]).unwrap();
    assert_eq!(drag.nodes, [a, b]);
    let revision = session.document().revision();
    drag.commit(&mut session, right, DropPosition::Inside)
        .unwrap();
    assert_eq!(session.document().revision(), revision + 1);
    assert_eq!(
        session.document().undo_label(),
        Some("Move outline selection")
    );
    assert_eq!(session.selection(), &[a, b]);
    assert!(content_children(&session, left).is_empty());
    assert_eq!(content_children(&session, right), [c, a, b]);
    assert_eq!(
        session
            .document()
            .ast()
            .node(a)
            .unwrap()
            .namespace_uri
            .as_deref(),
        Some("clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls")
    );
    assert!(
        session
            .document()
            .source()
            .contains("<!-- authored comment -->")
    );
    assert!(session.can_manipulate());
    let preview = session
        .preview
        .visual(right)
        .unwrap()
        .cast::<StackPanel>()
        .unwrap();
    let names = preview
        .children()
        .to_vec()
        .iter()
        .map(|child| child.name().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, ["C", "A", "B"]);
    session.undo().unwrap();
    assert_eq!(session.document().source(), SOURCE);
    assert!(!session.can_undo());
    session.redo().unwrap();
    assert_eq!(content_children(&session, right), [c, a, b]);
    assert!(session.can_manipulate());
    session.undo().unwrap();
    let mixed = OutlineDrag::capture(&session, [c, a]).unwrap();
    mixed
        .commit(&mut session, right, DropPosition::Inside)
        .unwrap();
    assert_eq!(content_children(&session, right), [a, c]);
    assert_eq!(content_children(&session, left), [b]);
    session.undo().unwrap();
    assert_eq!(session.document().source(), SOURCE);
}

#[test]
fn outline_move_indices_match_all_nonempty_selections_before_and_after_unselected_siblings() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let session = new_session(
        r#"<StackPanel xmlns="https://github.com/ferroui">
      <StackPanel.Resources><SolidColorBrush x:Key="Accent" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Color="Red"/></StackPanel.Resources>
      <!-- retained -->
      <Button Name="A"/><Button Name="B"/><Button Name="C"/><Button Name="D"/><Button Name="E"/>
    </StackPanel>"#,
    );
    let root = session.document().ast().root();
    let original = session.document().ast().element_children(root);
    let movable = original
        .iter()
        .copied()
        .filter(|id| session.document().ast().node(*id).unwrap().kind == NodeKind::Element)
        .collect::<Vec<_>>();
    for mask in 1..1usize << movable.len() {
        let nodes = movable
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, id)| *id)
            .collect::<Vec<_>>();
        let selected = nodes.iter().copied().collect::<BTreeSet<_>>();
        let drag = OutlineDrag::capture(&session, nodes.iter().rev().copied()).unwrap();
        for target in movable.iter().filter(|id| !selected.contains(id)) {
            for position in [DropPosition::Before, DropPosition::After] {
                let mut expected = original
                    .iter()
                    .copied()
                    .filter(|id| !selected.contains(id))
                    .collect::<Vec<_>>();
                let index = expected.iter().position(|id| id == target).unwrap()
                    + usize::from(position == DropPosition::After);
                expected.splice(index..index, nodes.iter().copied());
                let edits = drag.plan(&session, *target, position).unwrap();
                let mut candidate = session.document().clone();
                candidate.apply("Move", edits).unwrap();
                assert_eq!(
                    candidate.ast().element_children(root),
                    expected,
                    "mask={mask}, target={target}, position={position:?}"
                );
                assert!(candidate.source().contains("<!-- retained -->"));
            }
        }
    }
}

#[test]
fn outline_drag_moves_selected_subtrees_once_and_rejects_cycles_and_noncontent_rows() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = new_session(SOURCE);
    let (a, left, right) = (
        named(&session, "A"),
        named(&session, "Left"),
        named(&session, "Right"),
    );
    let root = session.document().ast().root();
    assert!(OutlineDrag::capture(&session, [root]).is_err());
    let property = ContentHost::of_owner(&session, right).unwrap().container;
    assert!(OutlineDrag::capture(&session, [property]).is_err());
    let drag = OutlineDrag::capture(&session, [a, left]).unwrap();
    assert_eq!(drag.nodes, [left]);
    assert!(drag.plan(&session, left, DropPosition::Inside).is_err());
    assert!(drag.plan(&session, a, DropPosition::After).is_err());
    assert!(drag.plan(&session, u64::MAX, DropPosition::Inside).is_err());
    assert!(drag.plan(&session, right, DropPosition::None).is_err());
    assert_eq!(session.document().source(), SOURCE);
    drag.commit(&mut session, property, DropPosition::Inside)
        .unwrap();
    assert_eq!(
        session.document().ast().node(left).unwrap().parent,
        Some(property)
    );
    assert_eq!(session.document().ast().node(a).unwrap().parent, Some(left));
    assert!(session.can_manipulate());
}

#[test]
fn outline_drag_checks_single_content_hosts_generated_panels_and_current_write_access() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = new_session(SOURCE);
    let (a, b, empty, occupied, text) = (
        named(&session, "A"),
        named(&session, "B"),
        named(&session, "Empty"),
        named(&session, "Occupied"),
        named(&session, "TextOccupied"),
    );
    let single = OutlineDrag::capture(&session, [a]).unwrap();
    assert!(single.can_drop(&session, empty, DropPosition::Inside));
    assert!(
        single
            .plan(&session, occupied, DropPosition::Inside)
            .is_err()
    );
    assert!(single.plan(&session, text, DropPosition::Inside).is_err());
    assert!(
        OutlineDrag::capture(&session, [a, b])
            .unwrap()
            .plan(&session, empty, DropPosition::Inside)
            .is_err()
    );
    session.set_read_only(true);
    assert!(
        !single.can_drop(&session, empty, DropPosition::Inside),
        "a cached hover must not bypass a changed collaboration role"
    );
    assert!(
        single
            .commit(&mut session, empty, DropPosition::Inside)
            .is_err()
    );
    assert!(OutlineDrag::capture(&session, [a]).is_err());
    assert_eq!(session.document().source(), SOURCE);
    session.set_read_only(false);
    single
        .commit(&mut session, empty, DropPosition::Inside)
        .unwrap();
    assert_eq!(content_children(&session, empty), [a]);

    let generated = new_session(
        r#"<StackPanel xmlns="https://github.com/ferroui"><VirtualizingStackPanel Name="Generated"/><Button Name="A"/></StackPanel>"#,
    );
    let drag = OutlineDrag::capture(&generated, [named(&generated, "A")]).unwrap();
    assert!(
        drag.plan(
            &generated,
            named(&generated, "Generated"),
            DropPosition::Inside
        )
        .is_err()
    );
}

#[test]
fn custom_panel_insertion_permission_does_not_imply_reordering_or_single_content() {
    use ferroui_designer_tools::{LayoutAction, LayoutAdapter, LayoutContext, ToolError};

    struct InsertionOnly;
    impl LayoutAdapter for InsertionOnly {
        fn panel_type(&self) -> &str {
            "StackPanel"
        }
        fn capabilities(&self) -> LayoutCapabilities {
            LayoutCapabilities {
                move_items: true,
                resize_items: true,
                reorder: false,
                drop_items: true,
                generated_items: false,
            }
        }
        fn panel_properties(&self) -> &[&str] {
            &[]
        }
        fn plan(&self, _: &LayoutContext<'_>, _: &LayoutAction) -> Result<Vec<Edit>, ToolError> {
            Err(ToolError(
                "This test adapter supplies outline permissions".into(),
            ))
        }
    }

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = new_session(SOURCE);
    session.layout_tools_mut().register(Box::new(InsertionOnly));
    let (a, b, c, right) = (
        named(&session, "A"),
        named(&session, "B"),
        named(&session, "C"),
        named(&session, "Right"),
    );
    let drag = OutlineDrag::capture(&session, [a, b]).unwrap();
    assert!(drag.plan(&session, c, DropPosition::Before).is_err());
    drag.commit(&mut session, right, DropPosition::Inside)
        .unwrap();
    assert_eq!(content_children(&session, right), [c, a, b]);
    let reorder = OutlineDrag::capture(&session, [c]).unwrap();
    assert!(reorder.plan(&session, right, DropPosition::Inside).is_err());
}

#[test]
fn outline_drag_rejects_changed_and_independent_documents_even_when_node_ids_match() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let original = new_session(SOURCE);
    let (a, right) = (named(&original, "A"), named(&original, "Right"));
    let drag = OutlineDrag::capture(&original, [a]).unwrap();
    let uri = original.document_uri().to_owned();
    let revision = original.document().revision();
    // The gesture keeps the old syntax alive, while the retired host releases the URI's
    // hot-reload ownership before an unrelated document is opened at that same location.
    drop(original);
    let mut independent = DesignerSession::with_uri(SOURCE, ContentControl::new(), uri).unwrap();
    assert_eq!(revision, independent.document().revision());
    assert_eq!(named(&independent, "A"), a);
    assert!(
        drag.commit(&mut independent, right, DropPosition::Inside)
            .is_err()
    );
    assert_eq!(independent.document().source(), SOURCE);
    let mut original = independent;
    let drag = OutlineDrag::capture(&original, [a]).unwrap();
    original.set_property(&[a], "Width", "123").unwrap();
    let edited = original.document().source().to_owned();
    assert!(
        drag.commit(&mut original, right, DropPosition::Inside)
            .is_err()
    );
    assert_eq!(original.document().source(), edited);
    let drag = OutlineDrag::capture(&original, [a]).unwrap();
    original.stage_source("<StackPanel");
    assert!(
        drag.commit(&mut original, right, DropPosition::Inside)
            .is_err()
    );
    assert_eq!(original.document().source(), "<StackPanel");
}

fn row(workspace: &DesignerWorkspace, path: &[i32]) -> BoxedValue {
    workspace
        .inspectors
        .outline
        .source
        .try_get_model_at(&IndexPath::from_indexes(path))
        .unwrap()
}

fn row_event(
    grid: &ferroui_base::Ref<TreeDataGrid>,
    model: BoxedValue,
    position: DropPosition,
    drop: bool,
) -> TreeDataGridRowDragEventArgs {
    let row = TreeDataGridRow::new();
    row.set_data_context(Some(model));
    let inner = DragEventArgs::new(
        None,
        DataTransfer::new(),
        grid.clone().upcast::<Interactive>(),
        Point::default(),
        KeyModifiers::NONE,
    );
    inner.set_drag_effects(DragDropEffects::MOVE);
    let event = TreeDataGridRowDragEventArgs::new(
        if drop {
            TreeDataGrid::row_drop_event()
        } else {
            TreeDataGrid::row_drag_over_event()
        },
        Some(row),
        inner,
    );
    event.set_position(position);
    event
}

#[test]
fn native_outline_row_events_commit_ast_moves_and_block_stale_runtime_and_foreign_drops() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let grid = workspace.outline_grid();
    assert!(grid.auto_drag_drop_rows());
    let left = row(&workspace, &[0, 0]);
    let a = row(&workspace, &[0, 0, 0]);
    let right = row(&workspace, &[0, 1]);
    let (a_id, right_id) = {
        let session = workspace.session.borrow();
        (named(&session, "A"), named(&session, "Right"))
    };
    workspace.session.borrow_mut().select([a_id]);
    workspace.refresh(true);
    let start = TreeDataGridRowDragStartedEventArgs::new(vec![Some(a.clone())]);
    grid.raise_event(&start);
    assert_eq!(start.allowed_effects(), DragDropEffects::MOVE);
    let hover = row_event(&grid, right.clone(), DropPosition::Inside, false);
    grid.raise_event(&hover);
    assert_eq!(hover.position(), DropPosition::Inside);
    assert_eq!(hover.inner().drag_effects(), DragDropEffects::MOVE);
    let drop = row_event(&grid, right, DropPosition::Inside, true);
    grid.raise_event(&drop);
    assert!(
        drop.handled(),
        "automatic TreeDataGrid row mutation must be intercepted"
    );
    assert_eq!(drop.inner().drag_effects(), DragDropEffects::MOVE);
    assert!(content_children(&workspace.session.borrow(), right_id).contains(&a_id));
    assert_eq!(
        workspace.source.text().as_deref(),
        Some(workspace.session.borrow().document().source())
    );

    // No active gesture means a foreign row drop cannot change either document or models.
    let source = workspace.session.borrow().document().source().to_owned();
    let foreign = row_event(&grid, left, DropPosition::Inside, true);
    grid.raise_event(&foreign);
    assert!(foreign.handled());
    assert_eq!(foreign.inner().drag_effects(), DragDropEffects::NONE);
    assert_eq!(workspace.session.borrow().document().source(), source);

    // A stale outline cannot turn old rows into edits against a newer source revision.
    workspace
        .session
        .borrow_mut()
        .set_property(&[a_id], "Width", "115")
        .unwrap();
    let stale = TreeDataGridRowDragStartedEventArgs::new(vec![Some(a)]);
    grid.raise_event(&stale);
    assert_eq!(stale.allowed_effects(), DragDropEffects::NONE);
    workspace.set_outline_mode(OutlineMode::Visual);
    let runtime = row(&workspace, &[0]);
    let runtime = TreeDataGridRowDragStartedEventArgs::new(vec![Some(runtime)]);
    grid.raise_event(&runtime);
    assert_eq!(runtime.allowed_effects(), DragDropEffects::NONE);
}

#[test]
fn resource_and_named_reference_drags_preserve_the_inspector_destination_and_cancel_cleanly() {
    use ferroui_base::input::platform::IPlatformDragSource;
    use ferroui_base::input::{
        DataTransferExtensions, IDataTransfer, KeyEventArgs, LocalBoxFuture, MouseButton, Pointer,
        PointerEventArgs, PointerPointProperties, PointerPressedEventArgs,
        PointerReleasedEventArgs, PointerUpdateKind, RawInputModifiers,
    };
    use ferroui_base::{FerroLocator, Ref};
    use ferroui_controls::{TextBox, ToolTip};

    #[derive(Default)]
    struct Capture {
        data: RefCell<Option<Rc<dyn IDataTransfer>>>,
        effects: Cell<DragDropEffects>,
    }
    impl IPlatformDragSource for Capture {
        fn do_drag_drop_async(
            &self,
            _: &PointerPressedEventArgs,
            data: Rc<dyn IDataTransfer>,
            effects: DragDropEffects,
        ) -> LocalBoxFuture<DragDropEffects> {
            *self.data.borrow_mut() = Some(data);
            self.effects.set(effects);
            Box::pin(std::future::ready(DragDropEffects::NONE))
        }
    }
    fn native_row(workspace: &DesignerWorkspace, node: NodeId) -> Ref<TreeDataGridRow> {
        workspace
            .outline_grid()
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<TreeDataGridRow>())
            .find(|row| {
                row.model()
                    .as_ref()
                    .and_then(|model| model.downcast_ref::<TreeRow>())
                    .is_some_and(|row| row.entry.source == Some(node))
            })
            .expect("realized authored source row")
    }
    fn press(row: &TreeDataGridRow, modifiers: KeyModifiers) -> PointerPressedEventArgs {
        let event = PointerPressedEventArgs::new(
            row.to_ref(),
            Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
            row,
            Point::new(1.0, 1.0),
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            modifiers,
            1,
        );
        row.raise_event(&event);
        event
    }
    fn move_pointer(row: &TreeDataGridRow, press: &PointerPressedEventArgs) {
        row.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            row.to_ref(),
            press.pointer().clone(),
            Some(row),
            Point::new(18.0, 18.0),
            2,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            press.key_modifiers(),
        ));
    }
    fn release(row: &TreeDataGridRow, press: &PointerPressedEventArgs) {
        row.raise_event(&PointerReleasedEventArgs::new(
            row.to_ref(),
            press.pointer().clone(),
            row,
            Point::new(1.0, 1.0),
            3,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            press.key_modifiers(),
            MouseButton::Left,
        ));
    }

    let _app = UnitTestApplication::start(TestServices::styled_window());
    const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><StackPanel.Resources><SolidColorBrush x:Key="Accent" Color="Red"/></StackPanel.Resources><Button Name="Source" Content="Reference"/><Border Name="Target" Height="40" Background="Blue"/></StackPanel>"#;
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1000.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let platform = Rc::new(Capture::default());
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformDragSource>()
        .to_constant(platform.clone());
    let (target, named_source, resource) = {
        let session = workspace.session.borrow();
        let resource = session
            .document()
            .ast()
            .nodes_in_source_order()
            .find(|node| node.attribute("x:Key") == Some("Accent"))
            .unwrap()
            .id;
        (
            named(&session, "Target"),
            named(&session, "Source"),
            resource,
        )
    };
    workspace
        .inspectors
        .outline
        .source
        .expand(&IndexPath::from_indexes(&[0, 0]));
    let mut resource_data = None;
    for (source, modifiers, value) in [
        (resource, KeyModifiers::NONE, "{StaticResource 'Accent'}"),
        (resource, KeyModifiers::SHIFT, "{DynamicResource 'Accent'}"),
        (
            named_source,
            KeyModifiers::ALT,
            "{Binding ElementName='Source'}",
        ),
    ] {
        workspace.session.borrow_mut().select([target]);
        workspace.refresh(true);
        window.update_layout();
        let row = native_row(&workspace, source);
        let pressed = press(&row, modifiers);
        assert_eq!(
            workspace.session.borrow().selection(),
            &[target],
            "press must keep property targets"
        );
        move_pointer(&row, &pressed);
        assert_eq!(platform.effects.get(), DragDropEffects::COPY);
        let data = platform
            .data
            .borrow_mut()
            .take()
            .expect("native reference drag");
        assert_eq!(
            data.try_get_value(&crate::property_controls::property_value_data_format())
                .as_deref(),
            Some(value)
        );
        assert_eq!(
            data.try_get_value(&DataFormat::text()).as_deref(),
            Some(value)
        );
        let metadata = data
            .try_get_value(&crate::property_controls::property_reference_data_format())
            .unwrap();
        if modifiers == KeyModifiers::NONE {
            resource_data = Some(data.clone());
        }
        assert_eq!(metadata.node, source);
        assert_eq!(
            metadata.document_uri,
            workspace.session.borrow().document_uri()
        );
        assert!(Arc::ptr_eq(
            &metadata.ast,
            &workspace.session.borrow().document().ast_snapshot()
        ));
        release(&row, &pressed);
        assert_eq!(
            workspace.session.borrow().selection(),
            &[target],
            "cancelled native copy must keep property targets"
        );
        assert_eq!(workspace.session.borrow().document().source(), SOURCE);
        assert!(!workspace.session.borrow().can_undo());
    }

    // Send the actual native payload to a realized destination value cell. This checks
    // the complete source-selection / reference-identity / property transaction path.
    workspace
        .root
        .get_control::<TextBox>("PropertySearch")
        .set_text(Some("Background"));
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
    let cell = workspace
        .property_grid()
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find(|control| {
            control.classes().contains("property-value-cell")
                && ToolTip::get_tip(control)
                    .and_then(|tip| tip.downcast_ref::<String>().cloned())
                    .is_some_and(|tip| tip.starts_with("Background ·"))
        })
        .expect("realized Background value cell");
    let dropped = DragEventArgs::new(
        Some(DragDrop::drop_event()),
        resource_data.unwrap(),
        cell.clone().upcast::<Interactive>(),
        Point::default(),
        KeyModifiers::NONE,
    );
    dropped.set_drag_effects(DragDropEffects::COPY);
    cell.raise_event(&dropped);
    assert!(dropped.handled());
    assert_eq!(dropped.drag_effects(), DragDropEffects::COPY);
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Background"),
        Some("{StaticResource 'Accent'}")
    );
    assert_eq!(workspace.session.borrow().selection(), &[target]);
    assert_eq!(workspace.session.borrow().document().revision(), 1);
    assert!(
        workspace.session.borrow().can_manipulate(),
        "{:?}",
        workspace.session.borrow().preview_status()
    );
    workspace.session.borrow_mut().undo().unwrap();
    workspace.refresh(true);
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    workspace
        .root
        .get_control::<TextBox>("PropertySearch")
        .set_text(None);
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();

    // Normal resource clicks still select the resource for inspection.
    let row = native_row(&workspace, resource);
    let pressed = press(&row, KeyModifiers::NONE);
    assert_eq!(workspace.session.borrow().selection(), &[target]);
    release(&row, &pressed);
    assert_eq!(workspace.session.borrow().selection(), &[resource]);

    // Escape cancels even before the native threshold; a later pointer movement must
    // not become a structural move of the original inspector destination.
    workspace.session.borrow_mut().select([target]);
    workspace.refresh(true);
    window.update_layout();
    let row = native_row(&workspace, resource);
    let pressed = press(&row, KeyModifiers::NONE);
    let mut escape = KeyEventArgs::new();
    escape.key = Key::Escape;
    escape.set_routed_event(Some(InputElement::key_down_event()));
    workspace.outline_grid().raise_event(&escape);
    assert!(escape.handled());
    move_pointer(&row, &pressed);
    assert!(platform.data.borrow().is_none());
    release(&row, &pressed);
    assert_eq!(workspace.session.borrow().selection(), &[target]);

    // A document change between press and drag cannot emit a fresh reference for an
    // old row or silently reinterpret the gesture against the replacement hierarchy.
    let pressed = press(&row, KeyModifiers::ALT);
    let revision = workspace.session.borrow().document().revision();
    workspace
        .session
        .borrow_mut()
        .set_property(&[target], "Width", "120")
        .unwrap();
    workspace.refresh(true);
    workspace.outline_grid().raise_row_drag_started(&pressed);
    assert!(platform.data.borrow().is_none());
    release(&row, &pressed);
    assert_eq!(workspace.session.borrow().selection(), &[target]);
    assert_eq!(
        workspace.session.borrow().document().revision(),
        revision + 1
    );
    window.close();
}

#[test]
fn reference_arguments_preserve_quotes_backslashes_and_literal_markup_braces() {
    for value in [
        "Accent",
        "accent, secondary",
        "owner's color",
        r"folder\key",
        "{literal}",
        "a=b",
    ] {
        let serialized = format!("{{StaticResource {}}}", markup_argument(value));
        let MarkupValue::Extension(extension) = MarkupValue::parse(&serialized).unwrap() else {
            panic!("extension")
        };
        assert_eq!(extension.name, "StaticResource");
        assert_eq!(extension.arguments.len(), 1);
        assert_eq!(
            extension.arguments[0].value,
            MarkupValue::Literal(value.into())
        );
    }
}

#[test]
fn structural_commands_preserve_disjoint_blocks_and_use_one_undo_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui"><Button Name="A"/><Button Name="B"/><Button Name="C"/><Button Name="D"/><Button Name="E"/><Button Name="F"/></StackPanel>"#;
    let mut session = new_session(SOURCE);
    let ids = ["A", "B", "C", "D", "E", "F"].map(|name| named(&session, name));
    let root = session.document().ast().root();
    session.select([ids[4], ids[2], ids[1]]);
    session.edit_outline(OutlineCommand::MoveUp).unwrap();
    assert_eq!(
        content_children(&session, root),
        [ids[1], ids[2], ids[0], ids[4], ids[3], ids[5]]
    );
    assert_eq!(session.selection(), [ids[1], ids[2], ids[4]]);
    assert_eq!(session.document().revision(), 1);
    session.undo().unwrap();
    assert_eq!(session.document().source(), SOURCE);
    session.edit_outline(OutlineCommand::MoveDown).unwrap();
    assert_eq!(
        content_children(&session, root),
        [ids[0], ids[3], ids[1], ids[2], ids[5], ids[4]]
    );
    session.undo().unwrap();
    session.edit_outline(OutlineCommand::MoveFirst).unwrap();
    assert_eq!(
        content_children(&session, root),
        [ids[1], ids[2], ids[4], ids[0], ids[3], ids[5]]
    );
    session.undo().unwrap();
    session.edit_outline(OutlineCommand::MoveLast).unwrap();
    assert_eq!(
        content_children(&session, root),
        [ids[0], ids[3], ids[5], ids[1], ids[2], ids[4]]
    );
    session.undo().unwrap();
    session.select([ids[0], ids[1]]);
    let revision = session.document().revision();
    assert!(!session.can_edit_outline(OutlineCommand::MoveUp));
    session.edit_outline(OutlineCommand::MoveUp).unwrap();
    assert_eq!(session.document().revision(), revision);
    assert!(!session.can_undo());
}

#[test]
fn indent_and_outdent_preserve_explicit_content_slots_and_reject_incompatible_selections() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui"><StackPanel Name="Group"><StackPanel.Children><Button Name="Existing"/></StackPanel.Children></StackPanel><Button Name="A" Content="Occupied"/><Button Name="B"/><Border Name="Empty"/></StackPanel>"#;
    let mut session = new_session(SOURCE);
    let (group, existing, a, b, empty) = (
        named(&session, "Group"),
        named(&session, "Existing"),
        named(&session, "A"),
        named(&session, "B"),
        named(&session, "Empty"),
    );
    let root = session.document().ast().root();
    session.select([b, a]);
    assert!(session.can_edit_outline(OutlineCommand::Indent));
    session.edit_outline(OutlineCommand::Indent).unwrap();
    assert_eq!(content_children(&session, group), [existing, a, b]);
    assert_eq!(content_children(&session, root), [group, empty]);
    assert!(session.can_edit_outline(OutlineCommand::Outdent));
    session.edit_outline(OutlineCommand::Outdent).unwrap();
    assert_eq!(content_children(&session, root), [group, a, b, empty]);
    assert_eq!(content_children(&session, group), [existing]);
    session.undo().unwrap();
    session.undo().unwrap();
    assert_eq!(session.document().source(), SOURCE);
    session.select([existing, a]);
    assert!(session.edit_outline(OutlineCommand::MoveDown).is_err());
    assert_eq!(session.document().source(), SOURCE);
    session.select([b]);
    assert!(
        session.edit_outline(OutlineCommand::Indent).is_err(),
        "an occupied content slot cannot accept another child"
    );
    session.select([group]);
    assert!(session.edit_outline(OutlineCommand::Outdent).is_err());
    session.set_read_only(true);
    session.select([a]);
    assert!(!session.can_edit_outline(OutlineCommand::MoveUp));
    assert!(session.edit_outline(OutlineCommand::MoveUp).is_err());
    assert_eq!(session.document().source(), SOURCE);
}

#[test]
fn outline_keyboard_commands_update_source_and_decline_pending_and_runtime_hierarchies() {
    use ferroui_base::input::KeyEventArgs;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let (a, b, left) = {
        let session = workspace.session.borrow();
        (
            named(&session, "A"),
            named(&session, "B"),
            named(&session, "Left"),
        )
    };
    workspace.session.borrow_mut().select([b]);
    workspace.refresh(true);
    let mut event = KeyEventArgs::new();
    event.key = Key::Up;
    event.key_modifiers = KeyModifiers::ALT;
    event.set_routed_event(Some(InputElement::key_down_event()));
    workspace.outline_grid().raise_event(&event);
    assert!(event.handled());
    assert_eq!(content_children(&workspace.session.borrow(), left), [b, a]);
    assert_eq!(
        workspace.source.text().as_deref(),
        Some(workspace.session.borrow().document().source())
    );
    let authored = workspace.session.borrow().document().source().to_owned();
    workspace.set_outline_mode(OutlineMode::Visual);
    assert!(workspace.edit_outline(OutlineCommand::MoveDown).is_err());
    workspace.set_outline_mode(OutlineMode::Source);
    workspace.source.set_text(Some("<StackPanel"));
    let staged = workspace.session.borrow().document().source().to_owned();
    let revision = workspace.session.borrow().document().revision();
    assert!(workspace.edit_outline(OutlineCommand::MoveDown).is_err());
    assert_eq!(workspace.session.borrow().document().source(), staged);
    assert_eq!(
        workspace.session.borrow().document().valid_source(),
        authored
    );
    assert_eq!(workspace.session.borrow().document().revision(), revision);
}

#[test]
fn delayed_outline_hover_expands_only_a_current_authored_drop_target() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let (a, right) = {
        let session = workspace.session.borrow();
        (named(&session, "A"), named(&session, "Right"))
    };
    let drag = RefCell::new(Some(
        OutlineDrag::capture(&workspace.session.borrow(), [a]).unwrap(),
    ));
    let path = IndexPath::from_indexes(&[0, 1]);
    let source = &workspace.inspectors.outline.source;
    source.collapse(&path);
    let expanded = Rc::new(Cell::new(0));
    let count = expanded.clone();
    source.add_row_expanded(Rc::new(move |_| count.set(count.get() + 1)));
    assert!(expand_hover_target(&workspace, &drag, right, &path));
    assert_eq!(expanded.get(), 1);
    source.collapse(&path);
    assert!(
        !expand_hover_target(&workspace, &drag, a, &path),
        "recycled paths cannot expand another row"
    );
    workspace
        .session
        .borrow_mut()
        .set_property(&[a], "Width", "130")
        .unwrap();
    assert!(!expand_hover_target(&workspace, &drag, right, &path));
    assert_eq!(expanded.get(), 1);
}

#[test]
fn canvas_reparent_preserves_affine_world_geometry_margin_and_authored_transform_graph() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" Width="700" Height="500" UseLayoutRounding="False">
      <Canvas Name="Old" Canvas.Left="280" Canvas.Top="70" Width="240" Height="200" RenderTransformOrigin="0,0" RenderTransform="matrix(-1.3,0.35,0.6,0.9,0,0)">
        <Border Name="Target" Width="80" Height="50" Canvas.Right="35" Canvas.Bottom="20" Margin="3,4,5,6" RenderTransformOrigin="20%,80%">
          <Border.RenderTransform><RotateTransform Angle="17"/></Border.RenderTransform>
        </Border>
      </Canvas>
      <Canvas Name="New" Canvas.Left="430" Canvas.Top="140" Width="200" Height="220" RenderTransformOrigin="0,0" RenderTransform="matrix(0.8,-0.15,0.2,1.2,0,0)"/>
    </Canvas>"#;
    let host = ContentControl::new();
    let mut session = DesignerSession::new(source, host.clone()).unwrap();
    let window = Window::new();
    window.set_width(700.0);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    let (target, destination) = (named(&session, "Target"), named(&session, "New"));
    let old = session.preview.visual(target).unwrap();
    let old_size = old.bounds().size();
    let before = old.transform_to_visual(&host).unwrap();
    let drag = OutlineDrag::capture(&session, [target]).unwrap();
    drag.commit(&mut session, destination, DropPosition::Inside)
        .unwrap();
    window.update_layout();
    assert!(session.can_manipulate(), "{:?}", session.preview_status());
    let visual = session.preview.visual(target).unwrap();
    assert_eq!(visual.bounds().size(), old_size);
    let after = visual.transform_to_visual(&host).unwrap();
    for point in [
        Point::default(),
        Point::new(80.0, 0.0),
        Point::new(0.0, 50.0),
        Point::new(37.0, 28.0),
    ] {
        let expected = point.transform(before);
        let actual = point.transform(after);
        assert!(
            (actual.x - expected.x).abs() < 1e-5 && (actual.y - expected.y).abs() < 1e-5,
            "{actual:?} != {expected:?}; bounds={:?}; before={before:?}; after={after:?}; render={:?}; source={}",
            visual.bounds(),
            visual.render_transform().map(|transform| transform.value()),
            session.document().source()
        );
    }
    let authored = session.document().ast().node(target).unwrap();
    assert_eq!(authored.attribute("Margin"), Some("3,4,5,6"));
    assert!(authored.attribute("Canvas.Right").is_none());
    assert!(authored.attribute("Canvas.Bottom").is_none());
    assert!(authored.attribute("RenderTransform").is_none());
    assert!(
        session
            .document()
            .source()
            .contains("<RotateTransform Angle=\"17\"/>")
    );
    assert_eq!(session.document().revision(), 1);
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
    window.close();
}

#[test]
fn canvas_reparent_materializes_stretched_size_without_redundant_transform_attributes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" Width="600" Height="300" UseLayoutRounding="False"><Grid Name="Old" Width="180" Height="120" Canvas.Left="20" Canvas.Top="30"><Border Name="Target" Margin="2"/></Grid><Canvas Name="New" Canvas.Left="250" Canvas.Top="30" Width="200" Height="200"/></Canvas>"#;
    let host = ContentControl::new();
    let mut session = DesignerSession::new(source, host.clone()).unwrap();
    let window = Window::new();
    window.set_width(600.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    let (target, destination) = (named(&session, "Target"), named(&session, "New"));
    let before = session.preview.visual(target).unwrap();
    let size = before.bounds().size();
    let origin = Point::default().transform(before.transform_to_visual(&host).unwrap());
    OutlineDrag::capture(&session, [target])
        .unwrap()
        .commit(&mut session, destination, DropPosition::Inside)
        .unwrap();
    window.update_layout();
    let after = session.preview.visual(target).unwrap();
    assert_eq!(after.bounds().size(), size);
    assert_eq!(
        Point::default().transform(after.transform_to_visual(&host).unwrap()),
        origin
    );
    let node = session.document().ast().node(target).unwrap();
    assert_eq!(
        node.attribute("Width").unwrap().parse::<f64>().unwrap(),
        size.width
    );
    assert_eq!(
        node.attribute("Height").unwrap().parse::<f64>().unwrap(),
        size.height
    );
    assert!(node.attribute("RenderTransform").is_none());
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
    window.close();
}

#[test]
fn canvas_reparent_rejects_authored_anchor_expressions_without_partial_moves() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" Width="600" Height="300"><Canvas Name="Old" Width="180" Height="120"><Border Name="Target" Width="80" Height="40" Canvas.Left="{Binding X}"/></Canvas><Canvas Name="New" Width="180" Height="120" Canvas.Left="250"/></Canvas>"#;
    let host = ContentControl::new();
    let mut session = DesignerSession::new(source, host.clone()).unwrap();
    let window = Window::new();
    window.set_width(600.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    let (target, destination) = (named(&session, "Target"), named(&session, "New"));
    let result = OutlineDrag::capture(&session, [target]).unwrap().commit(
        &mut session,
        destination,
        DropPosition::Inside,
    );
    assert!(result.unwrap_err().contains("Canvas.Left is an expression"));
    assert_eq!(session.document().source(), source);
    assert_eq!(session.document().revision(), 0);
    assert!(!session.can_undo());
    window.close();
}
