use crate::controls::*;
use crate::factory_parameter_tests::{ModelCall, RecordingFactory};
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn document_dock_defaults() {
    let dock = DocumentDock::new();
    assert!(!dock.can_create_document() && !dock.enable_window_drag());
    assert!(dock.document_factory().is_none());
    assert_eq!(dock.tabs_layout(), DocumentTabLayout::Top);
    assert_eq!(dock.layout_mode(), DocumentLayoutMode::Tabbed);
    assert_eq!(
        dock.close_button_show_mode(),
        DocumentCloseButtonShowMode::Always
    );
    assert_eq!(
        dock.empty_content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "No documents open"
    );
    for command in [
        dock.create_document(),
        dock.cascade_documents(),
        dock.tile_documents_horizontal(),
        dock.tile_documents_vertical(),
        dock.restore_documents(),
    ] {
        assert!(command.unwrap().can_execute(None));
    }
    assert!(dock.clone().as_document_dock().is_some() && dock.as_document_dock_factory().is_some());
}

fn assert_add_calls(factory: &RecordingFactory, dock: Rc<dyn IDock>, dockable: Rc<dyn IDockable>) {
    let calls = factory.model_calls.borrow();
    assert_eq!(calls.len(), 3);
    assert!(
        matches!(&calls[0], ModelCall::Add(owner, value) if Rc::ptr_eq(owner, &dock) && Rc::ptr_eq(value, &dockable))
    );
    assert!(matches!(&calls[1], ModelCall::SetActive(value) if Rc::ptr_eq(value, &dockable)));
    assert!(
        matches!(&calls[2], ModelCall::SetFocused(owner, Some(value)) if Rc::ptr_eq(owner, &dock) && Rc::ptr_eq(value, &dockable))
    );
}

#[test]
fn add_document_uses_factory_methods() {
    let dock = DocumentDock::new();
    let factory = Rc::new(RecordingFactory::default());
    let document = Document::new();
    dock.set_factory(Some(factory.clone()));
    dock.add_document(document.clone());
    assert_add_calls(factory.as_ref(), dock, document);
}

#[test]
fn add_tool_uses_factory_methods() {
    let dock = DocumentDock::new();
    let factory = Rc::new(RecordingFactory::default());
    let tool = Tool::new();
    dock.set_factory(Some(factory.clone()));
    dock.add_tool(tool.clone());
    assert_add_calls(factory.as_ref(), dock, tool);
}

#[test]
fn create_document_uses_the_delegate_and_adds_even_when_can_create_is_false() {
    let dock = DocumentDock::new();
    let factory = Rc::new(RecordingFactory::default());
    dock.set_factory(Some(factory.clone()));
    dock.create_document().unwrap().execute(None);
    assert!(factory.model_calls.borrow().is_empty());
    let document = Document::new();
    let created = document.clone();
    let weak = Rc::downgrade(&dock);
    dock.set_document_factory(Some(Rc::new(move || {
        weak.upgrade().unwrap().set_document_factory(None);
        Some(created.clone())
    })));
    assert!(!dock.can_create_document());
    dock.create_document().unwrap().execute(None);
    assert!(dock.document_factory().is_none());
    assert_add_calls(factory.as_ref(), dock, document);
}

#[test]
fn add_document_rechecks_factory_after_the_add_callback() {
    let dock = DocumentDock::new();
    let factory = Rc::new(RecordingFactory::default());
    *factory.on_add.borrow_mut() = Some(Rc::new(|dock, _| dock.set_factory(None)));
    dock.set_factory(Some(factory.clone()));
    dock.add_document(Document::new());
    assert_eq!(factory.model_calls.borrow().len(), 1);
    assert!(dock.factory().is_none());
}

#[test]
fn command_and_factory_auto_properties_are_replaceable_without_notifications() {
    let dock = DocumentDock::new();
    let count = Rc::new(Cell::new(0));
    let log = count.clone();
    dock.property_changed()
        .add(Rc::new(move |_| log.set(log.get() + 1)));
    let command = dock.create_document();
    dock.set_create_document(None);
    dock.set_create_document(command.clone());
    dock.set_cascade_documents(command.clone());
    dock.set_tile_documents_horizontal(command.clone());
    dock.set_tile_documents_vertical(command.clone());
    dock.set_restore_documents(command.clone());
    dock.set_document_factory(Some(Rc::new(|| Some(Document::new()))));
    dock.set_document_factory(None);
    for actual in [
        dock.create_document(),
        dock.cascade_documents(),
        dock.tile_documents_horizontal(),
        dock.tile_documents_vertical(),
        dock.restore_documents(),
    ] {
        assert!(Rc::ptr_eq(&actual.unwrap(), command.as_ref().unwrap()));
    }
    assert_eq!(count.get(), 0);
}

#[test]
fn empty_content_preserves_value_equality_and_reference_identity() {
    let dock = DocumentDock::new();
    let count = Rc::new(Cell::new(0));
    let log = count.clone();
    dock.property_changed().add(Rc::new(move |name| {
        assert_eq!(name, "EmptyContent");
        log.set(log.get() + 1);
    }));
    let original = dock.empty_content().unwrap();
    dock.set_empty_content(Some(Rc::new("No documents open".to_owned())));
    assert_eq!(count.get(), 0);
    assert!(Rc::ptr_eq(&original, &dock.empty_content().unwrap()));
    let object: BoxedValue = Rc::new(RefCell::new(42));
    dock.set_empty_content(Some(object.clone()));
    dock.set_empty_content(Some(object.clone()));
    assert!(Rc::ptr_eq(&object, &dock.empty_content().unwrap()));
    dock.set_empty_content(None);
    dock.set_empty_content(None);
    assert_eq!(count.get(), 2);
}

fn layout(documents: &[Rc<dyn IDockable>], width: f64, height: f64) -> Rc<DocumentDock> {
    let dock = DocumentDock::new();
    dock.set_visible_bounds(101.0, 202.0, width, height);
    dock.set_visible_dockables(Some(ListHandle::from_items(
        documents.iter().cloned().map(Some).collect(),
    )));
    dock
}

#[test]
fn cascade_assigns_source_bounds_and_puts_the_active_document_last() {
    let first = Document::new();
    let second = Document::new();
    first.set_mdi_state(MdiWindowState::Maximized);
    let dock = layout(&[first.clone(), second.clone()], 800.0, 600.0);
    dock.set_active_dockable(Some(first.clone()));
    MdiLayoutHelper::cascade_documents(dock.as_ref());
    assert_eq!(second.mdi_bounds(), DockRect::new(0.0, 0.0, 776.0, 576.0));
    assert_eq!(first.mdi_bounds(), DockRect::new(24.0, 24.0, 776.0, 576.0));
    assert_eq!(first.mdi_state(), MdiWindowState::Normal);
    assert_eq!((second.mdi_z_index(), first.mdi_z_index()), (0, 1));
}

#[test]
fn cascade_reduces_offsets_near_minimum_size_and_preserves_the_source_clamp_order() {
    let first = Document::new();
    let second = Document::new();
    let third = Document::new();
    let dock = layout(
        &[first.clone(), second.clone(), third.clone()],
        220.0,
        140.0,
    );
    MdiLayoutHelper::cascade_documents(dock.as_ref());
    assert_eq!(first.mdi_bounds(), DockRect::new(0.0, 0.0, 200.0, 120.0));
    assert_eq!(third.mdi_bounds(), DockRect::new(20.0, 20.0, 200.0, 120.0));
    dock.set_visible_bounds(0.0, 0.0, 100.0, 90.0);
    MdiLayoutHelper::cascade_documents(dock.as_ref());
    assert_eq!(second.mdi_bounds(), DockRect::new(24.0, 24.0, 200.0, 120.0));
}

#[test]
fn tiling_reserves_one_minimized_strip_and_leaves_minimized_bounds_untouched() {
    let first = Document::new();
    let second = Tool::new();
    let minimized = Document::new();
    let other_minimized = Document::new();
    minimized.set_mdi_state(MdiWindowState::Minimized);
    other_minimized.set_mdi_state(MdiWindowState::Minimized);
    let sentinel = DockRect::new(5.0, 6.0, 7.0, 8.0);
    minimized.set_mdi_bounds(sentinel);
    let dock = layout(
        &[
            first.clone(),
            minimized.clone(),
            second.clone(),
            other_minimized.clone(),
            StackDock::new(),
        ],
        800.0,
        600.0,
    );
    MdiLayoutHelper::tile_documents_horizontal(dock.as_ref());
    assert_eq!(first.mdi_bounds(), DockRect::new(0.0, 0.0, 800.0, 284.0));
    assert_eq!(second.mdi_bounds(), DockRect::new(0.0, 284.0, 800.0, 284.0));
    assert_eq!(minimized.mdi_bounds(), sentinel);
    assert_eq!(minimized.mdi_state(), MdiWindowState::Minimized);
    assert_eq!(
        (
            first.mdi_z_index(),
            minimized.mdi_z_index(),
            second.mdi_z_index(),
            other_minimized.mdi_z_index()
        ),
        (0, 1, 2, 3)
    );
    MdiLayoutHelper::tile_documents_vertical(dock.as_ref());
    assert_eq!(first.mdi_bounds(), DockRect::new(0.0, 0.0, 400.0, 568.0));
    assert_eq!(second.mdi_bounds(), DockRect::new(400.0, 0.0, 400.0, 568.0));
}

#[test]
fn all_minimized_arrangements_are_noops_while_restore_needs_no_bounds() {
    let document = Document::new();
    document.set_mdi_state(MdiWindowState::Minimized);
    document.set_mdi_z_index(42);
    let sentinel = DockRect::new(1.0, 2.0, 3.0, 4.0);
    document.set_mdi_bounds(sentinel);
    let erased: Rc<dyn IDockable> = document.clone();
    let dock = layout(std::slice::from_ref(&erased), 800.0, 600.0);
    for arrange in [
        MdiLayoutHelper::cascade_documents,
        MdiLayoutHelper::tile_documents_horizontal,
        MdiLayoutHelper::tile_documents_vertical,
    ] {
        arrange(dock.as_ref());
        assert_eq!(document.mdi_bounds(), sentinel);
        assert_eq!(document.mdi_z_index(), 42);
    }
    dock.set_visible_bounds(f64::NAN, f64::NAN, f64::NAN, f64::NAN);
    MdiLayoutHelper::restore_documents(dock.as_ref());
    assert_eq!(document.mdi_state(), MdiWindowState::Normal);
    assert_eq!(document.mdi_z_index(), 0);
    assert_eq!(document.mdi_bounds(), sentinel);
}

#[test]
fn invalid_layout_dimensions_leave_document_state_bounds_and_z_order_unchanged() {
    let document = Document::new();
    document.set_mdi_state(MdiWindowState::Maximized);
    document.set_mdi_z_index(7);
    let sentinel = DockRect::new(3.0, 4.0, 5.0, 6.0);
    document.set_mdi_bounds(sentinel);
    let erased: Rc<dyn IDockable> = document.clone();
    let dock = layout(std::slice::from_ref(&erased), 800.0, 600.0);
    for (width, height) in [
        (f64::NAN, 600.0),
        (800.0, f64::NAN),
        (0.0, 600.0),
        (800.0, 0.0),
        (-1.0, 600.0),
        (800.0, -1.0),
        (f64::NEG_INFINITY, 600.0),
    ] {
        dock.set_visible_bounds(0.0, 0.0, width, height);
        for arrange in [
            MdiLayoutHelper::cascade_documents,
            MdiLayoutHelper::tile_documents_horizontal,
            MdiLayoutHelper::tile_documents_vertical,
        ] {
            arrange(dock.as_ref());
            assert_eq!(document.mdi_state(), MdiWindowState::Maximized);
            assert_eq!(document.mdi_z_index(), 7);
            assert_eq!(document.mdi_bounds(), sentinel);
        }
    }
}

#[test]
fn active_document_is_arranged_even_when_absent_from_visible_dockables() {
    let dock = DocumentDock::new();
    let document = Document::new();
    dock.set_active_dockable(Some(document.clone()));
    dock.set_visible_bounds(0.0, 0.0, 800.0, 600.0);
    MdiLayoutHelper::tile_documents_vertical(dock.as_ref());
    assert_eq!(document.mdi_bounds(), DockRect::new(0.0, 0.0, 800.0, 600.0));
}

#[test]
fn positive_infinity_is_not_rejected_and_retains_ieee_coordinate_results() {
    let first = Document::new();
    let second = Document::new();
    let dock = layout(&[first.clone(), second.clone()], f64::INFINITY, 600.0);
    MdiLayoutHelper::tile_documents_vertical(dock.as_ref());
    assert!(first.mdi_bounds().x.is_nan());
    assert_eq!(first.mdi_bounds().width, f64::INFINITY);
    assert_eq!(second.mdi_bounds().x, f64::INFINITY);
}

#[test]
fn layout_uses_snapshots_and_finishes_all_bounds_before_updating_z_order() {
    let first = Document::new();
    let second = Document::new();
    first.set_mdi_state(MdiWindowState::Maximized);
    second.set_mdi_state(MdiWindowState::Maximized);
    first.set_mdi_z_index(-1);
    second.set_mdi_z_index(-1);
    let dock = layout(&[first.clone(), second.clone()], 800.0, 600.0);
    let visible = dock.visible_dockables().unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    first.property_changed().add(Rc::new(move |name| {
        log.borrow_mut().push(format!("first:{name}"));
        if name == "MdiState" {
            visible.clear();
        }
    }));
    let log = events.clone();
    second.property_changed().add(Rc::new(move |name| {
        log.borrow_mut().push(format!("second:{name}"))
    }));
    MdiLayoutHelper::tile_documents_horizontal(dock.as_ref());
    assert!(dock.visible_dockables().unwrap().is_empty());
    assert_eq!(
        *events.borrow(),
        [
            "first:MdiState",
            "first:MdiBounds",
            "second:MdiState",
            "second:MdiBounds",
            "first:MdiZIndex",
            "second:MdiZIndex"
        ]
    );
}

#[test]
fn document_arrangement_commands_require_mdi_layout_mode() {
    let first = Document::new();
    let second = Document::new();
    let dock = layout(&[first.clone(), second.clone()], 800.0, 600.0);
    first.set_mdi_state(MdiWindowState::Minimized);
    for command in [
        dock.cascade_documents(),
        dock.tile_documents_horizontal(),
        dock.tile_documents_vertical(),
        dock.restore_documents(),
    ] {
        command.unwrap().execute(None);
    }
    assert_eq!(first.mdi_state(), MdiWindowState::Minimized);
    assert_eq!(second.mdi_bounds(), DockRect::default());
    dock.set_layout_mode(DocumentLayoutMode::Mdi);
    dock.restore_documents().unwrap().execute(None);
    assert_eq!(first.mdi_state(), MdiWindowState::Normal);
    dock.cascade_documents().unwrap().execute(None);
    assert_eq!(second.mdi_bounds(), DockRect::new(24.0, 24.0, 776.0, 576.0));
    dock.tile_documents_horizontal().unwrap().execute(None);
    assert_eq!(second.mdi_bounds(), DockRect::new(0.0, 300.0, 800.0, 300.0));
    dock.tile_documents_vertical().unwrap().execute(None);
    assert_eq!(second.mdi_bounds(), DockRect::new(400.0, 0.0, 400.0, 600.0));
}

#[test]
fn mdi_helpers_accept_absent_or_empty_visible_collections() {
    let dock = DocumentDock::new();
    dock.set_visible_bounds(0.0, 0.0, 800.0, 600.0);
    for visible in [None, Some(ListHandle::new())] {
        dock.set_visible_dockables(visible);
        MdiLayoutHelper::cascade_documents(dock.as_ref());
        MdiLayoutHelper::tile_documents_horizontal(dock.as_ref());
        MdiLayoutHelper::tile_documents_vertical(dock.as_ref());
        MdiLayoutHelper::restore_documents(dock.as_ref());
        assert!(dock.active_dockable().is_none());
    }
}

#[test]
fn original_null_document_callback_without_factory_invokes_once_and_preserves_the_noop_body() {
    let source = DocumentDock::new();
    let calls = Rc::new(std::cell::Cell::new(0));
    let seen = calls.clone();
    source.set_document_factory(Some(Rc::new(move || {
        seen.set(seen.get() + 1);
        None
    })));
    source.create_document().unwrap().execute(None);
    assert_eq!(calls.get(), 1);
    assert!(source.visible_dockables().is_none());
    assert!(source.active_dockable().is_none());
    source.add_document_nullable(None);
    source.add_tool_nullable(None);
    assert!(source.visible_dockables().is_none());
}
#[test]
fn null_callback_with_original_base_factory_fails_before_graph_mutation() {
    let source = DocumentDock::new();
    let f = crate::Factory::new();
    source.set_factory(Some(f.clone()));
    source.set_document_factory(Some(Rc::new(|| None)));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        source.create_document().unwrap().execute(None)
    }));
    assert!(result.is_err());
    assert!(source.visible_dockables().is_none());
    assert!(source.active_dockable().is_none());
}
