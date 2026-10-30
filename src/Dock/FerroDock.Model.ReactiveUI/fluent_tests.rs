use crate::controls::*;
use crate::factory_metadata_tests::{DocumentMetadata, ToolMetadata};
use crate::{Factory, FactoryImpl};
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::fluent_extensions::*;
use ferrodock_model::{BoxedValue, FactoryBase, FactoryBaseImpl};
use ferroui_system::component_model::{INotifyPropertyChanged, INotifyPropertyChanging};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

#[test]
fn creation_helpers_configure_once_and_preserve_factory_and_output_identity() {
    let factory: Rc<dyn IFactory> = Factory::new();
    macro_rules! check {
        ($method:ident, $into:ident, $type:ident $(, $arg:expr)*) => {{
            let calls = Cell::new(0);
            let seen = RefCell::new(None::<Rc<dyn $type>>);
            let configured = |item: Rc<dyn $type>| {
                calls.set(calls.get() + 1); item.set_id("configured".into()); seen.replace(Some(item));
            };
            let item = factory.clone().$method($($arg,)* Some(&configured));
            assert_eq!(calls.get(), 1); assert_eq!(item.id(), "configured");
            assert!(Rc::ptr_eq(&item, &seen.borrow().clone().unwrap()));
            calls.set(0);
            let mut output = None;
            let returned = factory.clone().$into(&mut output, $($arg,)* Some(&configured));
            assert!(Rc::ptr_eq(&factory, &returned)); assert_eq!(calls.get(), 1);
            assert!(Rc::ptr_eq(&output.unwrap(), &seen.borrow().clone().unwrap()));
            assert!(factory.clone().$method($($arg,)* None).factory().is_none());
        }};
    }
    check!(document, document_into, IDocument);
    check!(tool, tool_into, ITool);
    check!(root_dock, root_dock_into, IRootDock);
    check!(
        proportional_dock,
        proportional_dock_into,
        IProportionalDock,
        Orientation::Vertical
    );
    check!(dock_dock, dock_dock_into, IDockDock);
    check!(stack_dock, stack_dock_into, IStackDock);
    check!(grid_dock, grid_dock_into, IGridDock);
    check!(wrap_dock, wrap_dock_into, IWrapDock);
    check!(uniform_grid_dock, uniform_grid_dock_into, IUniformGridDock);
    check!(
        proportional_dock_splitter,
        proportional_dock_splitter_into,
        IProportionalDockSplitter,
        false,
        true
    );
    check!(
        grid_dock_splitter,
        grid_dock_splitter_into,
        IGridDockSplitter
    );
    check!(tool_dock, tool_dock_into, IToolDock, Alignment::Right);
    check!(document_dock, document_dock_into, IDocumentDock);
    check!(dock_window, dock_window_into, IDockWindow);
    check!(split_view_dock, split_view_dock_into, ISplitViewDock);
}

#[test]
fn initial_creation_properties_precede_configure_and_defaults_are_the_source_defaults() {
    let factory = Factory::new();
    let proportional = factory.clone().proportional_dock(
        Orientation::Vertical,
        Some(&|dock| {
            assert_eq!(dock.orientation(), Orientation::Vertical);
            dock.set_orientation(Orientation::Horizontal);
        }),
    );
    assert_eq!(proportional.orientation(), Orientation::Horizontal);
    assert_eq!(
        factory
            .clone()
            .proportional_dock_default(None)
            .orientation(),
        Orientation::Horizontal
    );
    let splitter = factory.clone().proportional_dock_splitter(
        false,
        true,
        Some(&|splitter| {
            assert!(!splitter.can_resize());
            assert!(splitter.resize_preview());
            splitter.set_can_resize(true);
        }),
    );
    assert!(splitter.can_resize());
    let defaults = factory.clone().proportional_dock_splitter_default(None);
    assert!(defaults.can_resize());
    assert!(!defaults.resize_preview());
    let tool = factory.clone().tool_dock(
        Alignment::Bottom,
        Some(&|dock| {
            assert_eq!(dock.alignment(), Alignment::Bottom);
            dock.set_alignment(Alignment::Top);
        }),
    );
    assert_eq!(tool.alignment(), Alignment::Top);
    assert_eq!(
        factory.clone().tool_dock_default(None).alignment(),
        Alignment::Left
    );
    let mut proportional = None;
    let returned = factory
        .clone()
        .proportional_dock_into_default(&mut proportional, None);
    assert!(Rc::ptr_eq(&returned, &factory));
    assert_eq!(proportional.unwrap().orientation(), Orientation::Horizontal);
    let mut splitter = None;
    factory
        .clone()
        .proportional_dock_splitter_into_default(&mut splitter, None);
    assert!(splitter.as_ref().unwrap().can_resize());
    assert!(!splitter.unwrap().resize_preview());
    let mut tool = None;
    factory.tool_dock_into_default(&mut tool, None);
    assert_eq!(tool.unwrap().alignment(), Alignment::Left);
}

#[test]
fn source_out_assignment_order_survives_configuration_failure() {
    let factory = Factory::new();
    let previous: Rc<dyn IDocument> = Document::new();
    let mut document = Some(previous.clone());
    assert!(catch_unwind(AssertUnwindSafe(|| factory
        .clone()
        .document_into(&mut document, Some(&|_| panic!("configure")))))
    .is_err());
    assert!(Rc::ptr_eq(document.as_ref().unwrap(), &previous));
    let previous: Rc<dyn ITool> = Tool::new();
    let mut tool = Some(previous.clone());
    assert!(catch_unwind(AssertUnwindSafe(|| factory
        .clone()
        .tool_into(&mut tool, Some(&|_| panic!("configure")))))
    .is_err());
    assert!(Rc::ptr_eq(tool.as_ref().unwrap(), &previous));
    let previous: Rc<dyn ISplitViewDock> = SplitViewDock::new();
    let mut split = Some(previous.clone());
    let configured = RefCell::new(None);
    assert!(
        catch_unwind(AssertUnwindSafe(|| factory.clone().split_view_dock_into(
            &mut split,
            Some(&|dock| {
                configured.replace(Some(dock));
                panic!("configure");
            })
        )))
        .is_err()
    );
    assert!(!Rc::ptr_eq(split.as_ref().unwrap(), &previous));
    assert!(Rc::ptr_eq(
        split.as_ref().unwrap(),
        configured.borrow().as_ref().unwrap()
    ));
}

#[test]
fn dockable_chaining_retains_concrete_type_identity_and_source_values_without_validation() {
    let factory = Factory::new();
    let owner = ToolDock::new();
    let original = DocumentDock::new();
    let context: BoxedValue = Rc::new("context".to_owned());
    let model = Document::new();
    let returned: Rc<Document> = model
        .clone()
        .with_id("doc".into())
        .with_title("title".into())
        .with_context(Some(context.clone()))
        .with_owner(Some(owner.clone()))
        .with_original_owner(Some(original.clone()))
        .with_factory(Some(factory.clone()))
        .with_is_empty(true)
        .with_is_collapsable(false)
        .with_proportion(f64::INFINITY)
        .with_dock_mode(DockMode::Bottom)
        .with_grid(-2, 3, 4, -5)
        .with_shared_size_scope(true)
        .with_collapsed_proportion(f64::NAN)
        .with_min_width(-1.0)
        .with_max_width(f64::INFINITY)
        .with_min_height(6.0)
        .with_max_height(7.0)
        .with_can_close(false)
        .with_can_pin(false)
        .with_keep_pinned_dockable_visible(true)
        .with_pinned_dock_display_mode_override(Some(PinnedDockDisplayMode::Overlay))
        .with_can_float(false)
        .with_can_drag(false)
        .with_can_drop(false)
        .with_can_dock_as_document(false)
        .with_is_modified(true)
        .with_dock_group(Some("group".into()))
        .with_allowed_dock_operations(DockOperationMask::LEFT)
        .with_allowed_drop_operations(DockOperationMask::FILL);
    assert!(Rc::ptr_eq(&returned, &model));
    assert_eq!(model.id(), "doc");
    assert_eq!(model.title(), "title");
    assert!(Rc::ptr_eq(&model.context().unwrap(), &context));
    assert!(Rc::ptr_eq(
        &model.owner().unwrap(),
        &(owner as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &model.original_owner().unwrap(),
        &(original as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &model.factory().unwrap(),
        &(factory as Rc<dyn IFactory>)
    ));
    assert!(model.is_empty());
    assert!(!model.is_collapsable());
    assert_eq!(model.proportion(), f64::INFINITY);
    assert_eq!(model.dock(), DockMode::Bottom);
    assert_eq!(
        (
            model.column(),
            model.row(),
            model.column_span(),
            model.row_span()
        ),
        (-2, 3, 4, -5)
    );
    assert!(model.is_shared_size_scope());
    assert!(model.collapsed_proportion().is_nan());
    assert_eq!(
        (
            model.min_width(),
            model.max_width(),
            model.min_height(),
            model.max_height()
        ),
        (-1.0, f64::INFINITY, 6.0, 7.0)
    );
    assert!(!model.can_close());
    assert!(!model.can_pin());
    assert!(model.keep_pinned_dockable_visible());
    assert_eq!(
        model.pinned_dock_display_mode_override(),
        Some(PinnedDockDisplayMode::Overlay)
    );
    assert!(!model.can_float());
    assert!(!model.can_drag());
    assert!(!model.can_drop());
    assert!(!model.can_dock_as_document());
    assert!(model.is_modified());
    assert_eq!(model.dock_group().as_deref(), Some("group"));
    assert_eq!(model.allowed_dock_operations(), DockOperationMask::LEFT);
    assert_eq!(model.allowed_drop_operations(), DockOperationMask::FILL);
    model
        .clone()
        .with_grid_default()
        .with_context(None)
        .with_owner(None)
        .with_original_owner(None)
        .with_factory(None)
        .with_pinned_dock_display_mode_override(None)
        .with_dock_group(None);
    assert_eq!(
        (
            model.column(),
            model.row(),
            model.column_span(),
            model.row_span()
        ),
        (0, 0, 1, 1)
    );
    assert!(model.context().is_none());
    assert!(model.owner().is_none());
    assert!(model.original_owner().is_none());
    assert!(model.factory().is_none());
    assert!(model.pinned_dock_display_mode_override().is_none());
    assert!(model.dock_group().is_none());
}

#[test]
fn split_view_original_creation_and_full_chain_cases_use_same_models_and_children() {
    let factory = Factory::new();
    let tool = Tool::new();
    let document = DocumentDock::new();
    let split = factory.clone().split_view_dock(Some(&|dock| {
        dock.set_id("TestSplitView".into());
        dock.set_title("Test".into());
    }));
    assert_eq!(split.id(), "TestSplitView");
    assert_eq!(split.title(), "Test");
    let mut plain_output = None;
    let returned = factory
        .clone()
        .split_view_dock_into(&mut plain_output, None);
    assert!(Rc::ptr_eq(&returned, &factory));
    assert!(plain_output.is_some());
    let mut output = None;
    let returned = factory.clone().split_view_dock_into(
        &mut output,
        Some(&|dock| {
            dock.set_id("MySplitView".into());
            dock.set_display_mode(SplitViewDisplayMode::CompactOverlay);
        }),
    );
    assert!(Rc::ptr_eq(&returned, &factory));
    assert_eq!(output.as_ref().unwrap().id(), "MySplitView");
    assert_eq!(
        output.unwrap().display_mode(),
        SplitViewDisplayMode::CompactOverlay
    );
    let returned = split
        .clone()
        .with_id("MainSplitView".into())
        .with_title("Main".into())
        .with_compact_pane_length(48.0)
        .with_open_pane_length(320.0)
        .with_display_mode(SplitViewDisplayMode::CompactOverlay)
        .with_is_pane_open(true)
        .with_pane_placement(SplitViewPanePlacement::Left)
        .with_use_light_dismiss_overlay_mode(false)
        .with_pane_dockable(Some(tool.clone()))
        .with_content_dockable(Some(document.clone()));
    assert!(Rc::ptr_eq(&returned, &split));
    assert_eq!(split.id(), "MainSplitView");
    assert_eq!(split.title(), "Main");
    assert_eq!(split.compact_pane_length(), 48.0);
    assert_eq!(split.open_pane_length(), 320.0);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::CompactOverlay);
    assert!(split.is_pane_open());
    assert_eq!(split.pane_placement(), SplitViewPanePlacement::Left);
    assert!(!split.use_light_dismiss_overlay_mode());
    assert!(Rc::ptr_eq(
        &split.pane_dockable().unwrap(),
        &(tool as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &split.content_dockable().unwrap(),
        &(document as Rc<dyn IDockable>)
    ));
    split
        .clone()
        .with_compact_pane_length(64.0)
        .with_open_pane_length(256.0)
        .with_display_mode(SplitViewDisplayMode::Inline)
        .with_pane_placement(SplitViewPanePlacement::Right)
        .with_use_light_dismiss_overlay_mode(true)
        .with_pane_dockable(None)
        .with_content_dockable(None);
    assert_eq!(split.compact_pane_length(), 64.0);
    assert_eq!(split.open_pane_length(), 256.0);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::Inline);
    assert_eq!(split.pane_placement(), SplitViewPanePlacement::Right);
    assert!(split.use_light_dismiss_overlay_mode());
    assert!(split.pane_dockable().is_none());
    assert!(split.content_dockable().is_none());
}

#[test]
fn visible_helpers_append_keep_duplicates_use_plain_fallback_and_do_not_initialize_children() {
    let pane = DocumentDock::new();
    let document: Rc<dyn IDockable> = Document::new();
    let tool: Rc<dyn IDockable> = Tool::new();
    let returned = pane
        .clone()
        .with_visible_dockables([Some(document.clone()), Some(tool.clone())]);
    assert!(Rc::ptr_eq(&returned, &pane));
    let list = pane.visible_dockables().unwrap();
    assert!(list.as_observable_collection().is_none());
    assert_eq!(list.count(), 2);
    assert!(document.owner().is_none());
    assert!(document.factory().is_none());
    assert!(pane.active_dockable().is_none());
    let returned = pane.clone().add([Some(document.clone())]);
    assert!(Rc::ptr_eq(&returned, &pane));
    assert!(list.ptr_eq(&pane.visible_dockables().unwrap()));
    assert_eq!(list.count(), 3);
    assert!(Rc::ptr_eq(
        list.get(0).as_ref().expect("nonnull fixture"),
        list.get(2).as_ref().expect("nonnull fixture")
    ));
    let other = ListHandle::from_plain(vec![Some(tool.clone()), Some(document.clone())]);
    pane.clone().with_visible_dockables(other.iter());
    assert_eq!(list.count(), 5);
    let visible = ListHandle::from_items(vec![Some(document.clone())]);
    pane.set_visible_dockables(Some(visible.clone()));
    pane.clone().with_visible_dockables([Some(tool)]);
    assert!(visible.ptr_eq(&pane.visible_dockables().unwrap()));
    assert!(visible.as_observable_collection().is_some());
    assert_eq!(visible.count(), 2);
    pane.clone().with_visible_dockables(std::iter::empty());
    assert_eq!(visible.count(), 2);
    // Appending a plain list to itself uses its fail-fast source enumeration,
    // rather than an invented snapshot or deduplicated merge.
    pane.set_visible_dockables(Some(list.clone()));
    assert!(catch_unwind(AssertUnwindSafe(|| pane
        .clone()
        .with_visible_dockables(list.iter())))
    .is_err());
    assert_eq!(list.count(), 6);
}

#[test]
fn root_lists_append_independently_without_using_factory_creators() {
    let root = RootDock::new();
    let first: Rc<dyn IDockable> = Document::new();
    let second: Rc<dyn IDockable> = Tool::new();
    macro_rules! check {
        ($with:ident,$get:ident,$set:ident) => {{
            let initial = ListHandle::from_items(vec![Some(first.clone())]);
            root.$set(Some(initial.clone()));
            let returned = root
                .clone()
                .$with([Some(second.clone()), Some(first.clone())]);
            assert!(Rc::ptr_eq(&returned, &root));
            assert!(initial.ptr_eq(&root.$get().unwrap()));
            assert_eq!(initial.count(), 3);
            root.$set(None);
            root.clone().$with([Some(first.clone())]);
            assert!(root.$get().unwrap().as_observable_collection().is_none());
        }};
    }
    check!(
        with_hidden_dockables,
        hidden_dockables,
        set_hidden_dockables
    );
    check!(
        with_left_pinned,
        left_pinned_dockables,
        set_left_pinned_dockables
    );
    check!(
        with_right_pinned,
        right_pinned_dockables,
        set_right_pinned_dockables
    );
    check!(
        with_top_pinned,
        top_pinned_dockables,
        set_top_pinned_dockables
    );
    check!(
        with_bottom_pinned,
        bottom_pinned_dockables,
        set_bottom_pinned_dockables
    );
    let window: Rc<dyn IDockWindow> = crate::core::DockWindow::new();
    root.clone()
        .with_windows([Some(window.clone()), Some(window.clone())]);
    let windows = root.windows().unwrap();
    assert_eq!(windows.count(), 2);
    assert!(windows.as_observable_collection().is_none());
    root.clone().with_windows([Some(window.clone())]);
    assert_eq!(windows.count(), 3);
    assert!(window.owner().is_none());
    assert!(first.owner().is_none());
    assert!(second.owner().is_none());
}

#[test]
fn container_properties_and_template_helpers_retain_identity() {
    let proportional = ProportionalDock::new();
    assert!(Rc::ptr_eq(
        &proportional.clone().with_orientation(Orientation::Vertical),
        &proportional
    ));
    assert_eq!(proportional.orientation(), Orientation::Vertical);
    let wrap = WrapDock::new();
    wrap.clone().with_orientation(Orientation::Vertical);
    assert_eq!(wrap.orientation(), Orientation::Vertical);
    let stack = StackDock::new();
    stack
        .clone()
        .with_orientation(Orientation::Vertical)
        .with_spacing(-5.0);
    assert_eq!(stack.orientation(), Orientation::Vertical);
    assert_eq!(stack.spacing(), -5.0);
    let grid = GridDock::new();
    grid.clone()
        .with_column_definitions(Some("*,2*".into()))
        .with_row_definitions(Some("Auto,*".into()));
    assert_eq!(grid.column_definitions().as_deref(), Some("*,2*"));
    assert_eq!(grid.row_definitions().as_deref(), Some("Auto,*"));
    grid.clone()
        .with_column_definitions(None)
        .with_row_definitions(None);
    assert!(grid.column_definitions().is_none());
    assert!(grid.row_definitions().is_none());
    let uniform = UniformGridDock::new();
    uniform.clone().with_rows(-1).with_columns(3);
    assert_eq!(uniform.rows(), -1);
    assert_eq!(uniform.columns(), 3);
    let dock = DockDock::new();
    dock.clone().with_last_child_fill(false);
    assert!(!dock.last_child_fill());
    let splitter = ProportionalDockSplitter::new();
    splitter
        .clone()
        .with_can_resize(false)
        .with_resize_preview(true);
    assert!(!splitter.can_resize());
    assert!(splitter.resize_preview());
    let grid_splitter = GridDockSplitter::new();
    grid_splitter
        .clone()
        .with_resize_direction(GridResizeDirection::Columns);
    assert_eq!(
        grid_splitter.resize_direction(),
        GridResizeDirection::Columns
    );
    let document = DocumentMetadata::new();
    let template: Rc<dyn IDocumentTemplate> =
        Rc::new(crate::dock_state_tests::Template(RefCell::new(None)));
    document.set_document_template(None);
    assert!(Rc::ptr_eq(
        &document
            .clone()
            .with_document_template(Some(template.clone())),
        &document
    ));
    assert!(Rc::ptr_eq(
        &document.document_template().unwrap(),
        &template
    ));
    document.clone().with_document_template(None);
    assert!(document.document_template().is_none());
    let tool = ToolMetadata::new();
    let template: Rc<dyn IToolTemplate> =
        Rc::new(crate::dock_state_tests::Template(RefCell::new(None)));
    tool.set_tool_template(None);
    tool.clone().with_tool_template(Some(template.clone()));
    assert!(Rc::ptr_eq(&tool.tool_template().unwrap(), &template));
    tool.clone().with_tool_template(None);
    assert!(tool.tool_template().is_none());
}

#[test]
fn document_and_tool_helpers_preserve_commands_placeholder_and_virtual_add_behavior() {
    let factory = Factory::new();
    let dock = DocumentDock::new();
    dock.set_factory(Some(factory.clone()));
    let command = dock.create_document().unwrap();
    let content: BoxedValue = Rc::new("Nothing to show".to_owned());
    let returned = dock
        .clone()
        .with_can_create_document(true)
        .with_create_document(Some(command.clone()))
        .with_enable_window_drag(false)
        .with_tabs_layout(DocumentTabLayout::Left)
        .with_layout_mode(DocumentLayoutMode::Mdi)
        .with_close_button_show_mode(DocumentCloseButtonShowMode::Never)
        .with_empty_content(Some(content.clone()));
    assert!(Rc::ptr_eq(&returned, &dock));
    assert!(dock.can_create_document());
    assert!(Rc::ptr_eq(&dock.create_document().unwrap(), &command));
    assert!(!dock.enable_window_drag());
    assert_eq!(dock.tabs_layout(), DocumentTabLayout::Left);
    assert_eq!(dock.layout_mode(), DocumentLayoutMode::Mdi);
    assert_eq!(
        dock.close_button_show_mode(),
        DocumentCloseButtonShowMode::Never
    );
    assert!(Rc::ptr_eq(&dock.empty_content().unwrap(), &content));
    let document: Rc<dyn IDockable> = Document::new();
    let tool: Rc<dyn IDockable> = Tool::new();
    assert!(Rc::ptr_eq(
        &dock.clone().append_document(document.clone()),
        &dock
    ));
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &document));
    dock.clone().append_tool(tool.clone());
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &tool));
    assert!(Rc::ptr_eq(
        &document.owner().unwrap(),
        &(dock.clone() as Rc<dyn IDockable>)
    ));
    dock.clone()
        .with_create_document(None)
        .with_empty_content(None);
    assert!(dock.create_document().is_none());
    assert!(dock.empty_content().is_none());
    let tools = ToolDock::new();
    tools.set_factory(Some(factory.clone()));
    tools
        .clone()
        .with_alignment(Alignment::Right)
        .with_is_expanded(false)
        .with_auto_hide(false)
        .with_grip_mode(GripMode::AutoHide)
        .append_tool(tool.clone());
    assert_eq!(tools.alignment(), Alignment::Right);
    assert!(!tools.is_expanded());
    assert!(!tools.auto_hide());
    assert_eq!(tools.grip_mode(), GripMode::AutoHide);
    assert!(Rc::ptr_eq(&tools.active_dockable().unwrap(), &tool));
}

#[test]
fn root_dock_and_base_selection_helpers_assign_only_the_requested_properties() {
    let root = RootDock::new();
    let child: Rc<dyn IDockable> = Document::new();
    let focused: Rc<dyn IDockable> = Tool::new();
    let preview: Rc<dyn IToolDock> = ToolDock::new();
    let window: Rc<dyn IDockWindow> = crate::core::DockWindow::new();
    let returned = root
        .clone()
        .with_active_dockable(Some(child.clone()))
        .with_default_dockable(Some(child.clone()))
        .with_focused_dockable(Some(focused.clone()))
        .with_is_active(true)
        .with_can_close_last_dockable(false)
        .with_enable_global_docking(false)
        .with_is_focusable_root(true)
        .with_pinned_dock(Some(preview.clone()))
        .with_pinned_dock_display_mode(PinnedDockDisplayMode::Overlay)
        .with_window(Some(window.clone()))
        .with_floating_window_host_mode(DockFloatingWindowHostMode::Managed);
    assert!(Rc::ptr_eq(&returned, &root));
    assert!(Rc::ptr_eq(&root.active_dockable().unwrap(), &child));
    assert!(Rc::ptr_eq(&root.default_dockable().unwrap(), &child));
    assert!(Rc::ptr_eq(&root.focused_dockable().unwrap(), &focused));
    assert!(root.is_active());
    assert!(!root.can_close_last_dockable());
    assert!(!root.enable_global_docking());
    assert!(root.is_focusable_root());
    assert!(Rc::ptr_eq(&root.pinned_dock().unwrap(), &preview));
    assert_eq!(
        root.pinned_dock_display_mode(),
        PinnedDockDisplayMode::Overlay
    );
    assert!(Rc::ptr_eq(&root.window().unwrap(), &window));
    assert_eq!(
        root.floating_window_host_mode(),
        DockFloatingWindowHostMode::Managed
    );
    assert!(child.owner().is_none());
    root.clone()
        .with_active_dockable(None)
        .with_default_dockable(None)
        .with_focused_dockable(None)
        .with_pinned_dock(None)
        .with_window(None);
    assert!(root.active_dockable().is_none());
    assert!(root.default_dockable().is_none());
    assert!(root.focused_dockable().is_none());
    assert!(root.pinned_dock().is_none());
    assert!(root.window().is_none());
}

#[test]
fn window_helpers_preserve_source_geometry_order_and_nullable_references() {
    let factory = Factory::new();
    let window = crate::core::DockWindow::new();
    let parent: Rc<dyn IDockWindow> = crate::core::DockWindow::new();
    let owner: Rc<dyn IDockable> = Tool::new();
    let root: Rc<dyn IRootDock> = RootDock::new();
    let order = Rc::new(RefCell::new(Vec::new()));
    let events = order.clone();
    window.property_changed().add(Rc::new(move |name| {
        events.borrow_mut().push(name.to_owned());
    }));
    let returned = window
        .clone()
        .with_id("window".into())
        .with_position(-1.0, f64::INFINITY)
        .with_size(f64::NAN, -3.0)
        .with_window_state(DockWindowState::Maximized)
        .with_topmost(true)
        .with_title("title".into())
        .with_owner_mode(DockWindowOwnerMode::None)
        .with_parent_window(Some(parent.clone()))
        .with_modal(true)
        .with_show_in_taskbar(Some(false))
        .with_owner(Some(owner.clone()))
        .with_factory(Some(factory.clone()))
        .with_layout(Some(root.clone()));
    assert!(Rc::ptr_eq(&returned, &window));
    assert_eq!(window.id(), "window");
    assert_eq!(window.title(), "title");
    assert_eq!(window.x(), -1.0);
    assert_eq!(window.y(), f64::INFINITY);
    assert!(window.width().is_nan());
    assert_eq!(window.height(), -3.0);
    let sequence = order.borrow();
    let x = sequence.iter().position(|name| name == "X").unwrap();
    let y = sequence.iter().position(|name| name == "Y").unwrap();
    assert!(x < y);
    drop(sequence);
    assert_eq!(window.window_state(), DockWindowState::Maximized);
    assert!(window.topmost());
    assert_eq!(window.owner_mode(), DockWindowOwnerMode::None);
    assert!(Rc::ptr_eq(&window.parent_window().unwrap(), &parent));
    assert!(window.is_modal());
    assert_eq!(window.show_in_taskbar(), Some(false));
    assert!(Rc::ptr_eq(&window.owner().unwrap(), &owner));
    assert!(Rc::ptr_eq(&window.layout().unwrap(), &root));
    window.clone().with_size(2.0, 4.0);
    let sequence = order.borrow();
    let width = sequence.iter().position(|name| name == "Width").unwrap();
    let height = sequence.iter().rposition(|name| name == "Height").unwrap();
    assert!(width < height);
    drop(sequence);
    let host: Rc<dyn IHostWindow> = crate::test_host::RecordingHost::new();
    window.clone().with_host(Some(host.clone()));
    assert!(Rc::ptr_eq(&window.host().unwrap(), &host));
    window
        .clone()
        .with_host(None)
        .with_layout(None)
        .with_factory(None)
        .with_owner(None)
        .with_parent_window(None)
        .with_show_in_taskbar(None);
    assert!(window.host().is_none());
    assert!(window.layout().is_none());
    assert!(window.factory().is_none());
    assert!(window.owner().is_none());
    assert!(window.parent_window().is_none());
    assert!(window.show_in_taskbar().is_none());
}

struct ThrowingFactory {
    component: Factory,
    proportional: Rc<ProportionalDock>,
    created: Cell<usize>,
}
impl ThrowingFactory {
    fn new(proportional: Rc<ProportionalDock>) -> Rc<Self> {
        Rc::new_cyclic(|runtime: &Weak<Self>| Self {
            component: Factory::construct(runtime.clone()),
            proportional,
            created: Cell::new(0),
        })
    }
}
impl FactoryBaseImpl for ThrowingFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for ThrowingFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn create_proportional_dock(this: &Self) -> Rc<dyn IProportionalDock> {
        this.created.set(this.created.get() + 1);
        this.proportional.clone()
    }
}
crate::impl_factory!(ThrowingFactory);
#[test]
fn factory_helpers_dispatch_overrides_and_assign_out_before_initial_property_failure() {
    let dock = ProportionalDock::new();
    dock.property_changing().add(Rc::new(|name| {
        if name == "Orientation" {
            panic!("property setter failed");
        }
    }));
    let factory = ThrowingFactory::new(dock.clone());
    let runtime: Rc<dyn IFactory> = factory.clone();
    let previous: Rc<dyn IProportionalDock> = ProportionalDock::new();
    let mut output = Some(previous.clone());
    let configured = Cell::new(false);
    assert!(
        catch_unwind(AssertUnwindSafe(|| runtime.clone().proportional_dock_into(
            &mut output,
            Orientation::Vertical,
            Some(&|_| configured.set(true))
        )))
        .is_err()
    );
    assert_eq!(factory.created.get(), 1);
    assert!(!configured.get());
    assert!(!Rc::ptr_eq(output.as_ref().unwrap(), &previous));
    assert!(Rc::ptr_eq(
        output.as_ref().unwrap(),
        &(dock.clone() as Rc<dyn IProportionalDock>)
    ));
    assert_eq!(dock.orientation(), Orientation::Horizontal);
    assert!(catch_unwind(AssertUnwindSafe(
        || runtime.proportional_dock(Orientation::Vertical, Some(&|_| configured.set(true)))
    ))
    .is_err());
    assert_eq!(factory.created.get(), 2);
    assert!(!configured.get());
}

#[test]
fn collection_helpers_reread_properties_after_reentrant_notifications_and_iterator_advancement() {
    let dock = DocumentDock::new();
    let first_list = ListHandle::new();
    let replacement = ListHandle::from_plain(Vec::new());
    dock.set_visible_dockables(Some(first_list.clone()));
    let weak = Rc::downgrade(&dock);
    let target = replacement.clone();
    first_list
        .as_notify_collection_changed()
        .unwrap()
        .collection_changed()
        .add(Rc::new(move |_| {
            weak.upgrade()
                .unwrap()
                .set_visible_dockables(Some(target.clone()));
        }));
    let a: Rc<dyn IDockable> = Document::new();
    let b: Rc<dyn IDockable> = Tool::new();
    dock.clone()
        .with_visible_dockables([Some(a.clone()), Some(b.clone())]);
    assert_eq!(first_list.count(), 1);
    assert!(Rc::ptr_eq(
        (first_list.get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &a
    ));
    assert_eq!(replacement.count(), 1);
    assert!(Rc::ptr_eq(
        (replacement.get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &b
    ));
    let root = RootDock::new();
    let first_list = ListHandle::new();
    let replacement = ListHandle::from_plain(Vec::new());
    root.set_left_pinned_dockables(Some(first_list.clone()));
    let weak = Rc::downgrade(&root);
    let target = replacement.clone();
    first_list
        .as_notify_collection_changed()
        .unwrap()
        .collection_changed()
        .add(Rc::new(move |_| {
            weak.upgrade()
                .unwrap()
                .set_left_pinned_dockables(Some(target.clone()));
        }));
    root.clone()
        .with_left_pinned([Some(a.clone()), Some(b.clone())]);
    assert_eq!(first_list.count(), 1);
    assert_eq!(replacement.count(), 1);
    assert!(Rc::ptr_eq(
        (replacement.get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &b
    ));
    // IEnumerable.MoveNext can itself replace the property. The next Add must
    // use that replacement and must not pre-enumerate the supplied sequence.
    let third_list = ListHandle::from_plain(Vec::new());
    let target = third_list.clone();
    let runtime = dock.clone();
    let items = std::iter::once(Some(a.clone())).inspect(move |_| {
        runtime.set_visible_dockables(Some(target.clone()));
    });
    dock.clone().with_visible_dockables(items);
    assert_eq!(third_list.count(), 1);
    assert!(Rc::ptr_eq(
        (third_list.get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &a
    ));
    let runtime = dock.clone();
    let items = std::iter::once(Some(b)).inspect(move |_| {
        runtime.set_visible_dockables(None);
    });
    assert!(catch_unwind(AssertUnwindSafe(|| dock.with_visible_dockables(items))).is_err());
}
