use super::*;
use ferroui_base::input::{DataTransfer, DragEventArgs, KeyModifiers};
use ferroui_base::interactivity::Interactive;
use ferroui_base::{BoxedValue, Point};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, StackPanel};
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
    assert!(session
        .document()
        .source()
        .contains("<!-- authored comment -->"));
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
    assert!(single
        .plan(&session, occupied, DropPosition::Inside)
        .is_err());
    assert!(single.plan(&session, text, DropPosition::Inside).is_err());
    assert!(OutlineDrag::capture(&session, [a, b])
        .unwrap()
        .plan(&session, empty, DropPosition::Inside)
        .is_err());
    session.set_read_only(true);
    assert!(
        !single.can_drop(&session, empty, DropPosition::Inside),
        "a cached hover must not bypass a changed collaboration role"
    );
    assert!(single
        .commit(&mut session, empty, DropPosition::Inside)
        .is_err());
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
    assert!(drag
        .plan(
            &generated,
            named(&generated, "Generated"),
            DropPosition::Inside
        )
        .is_err());
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
    assert!(drag
        .commit(&mut independent, right, DropPosition::Inside)
        .is_err());
    assert_eq!(independent.document().source(), SOURCE);
    let mut original = independent;
    let drag = OutlineDrag::capture(&original, [a]).unwrap();
    original.set_property(&[a], "Width", "123").unwrap();
    let edited = original.document().source().to_owned();
    assert!(drag
        .commit(&mut original, right, DropPosition::Inside)
        .is_err());
    assert_eq!(original.document().source(), edited);
    let drag = OutlineDrag::capture(&original, [a]).unwrap();
    original.stage_source("<StackPanel");
    assert!(drag
        .commit(&mut original, right, DropPosition::Inside)
        .is_err());
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
