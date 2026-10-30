use crate::controls::*;
use crate::core::{DockableBase, DockableBaseImpl};
use crate::factory_parameter_tests::RecordingFactory;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{
    reactive_object, IReactiveNotifyPropertyChanged, ReactiveUIBindingExtensions,
};
use ferroui_system::component_model::{INotifyPropertyChanged, INotifyPropertyChanging};
use ferroui_system::prop;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[test]
fn visible_bounds_are_tracked() {
    let doc = Document::new();
    let (x, y, width, height) = doc.get_visible_bounds();
    assert!(x.is_nan() && y.is_nan() && width.is_nan() && height.is_nan());
    doc.set_visible_bounds(1.0, 2.0, 3.0, 4.0);
    assert_eq!(doc.get_visible_bounds(), (1.0, 2.0, 3.0, 4.0));
}

#[test]
fn pinned_bounds_are_tracked() {
    let doc = Document::new();
    doc.set_pinned_bounds(5.0, 6.0, 7.0, 8.0);
    assert_eq!(doc.get_pinned_bounds(), (5.0, 6.0, 7.0, 8.0));
}

#[test]
fn tab_bounds_are_tracked() {
    let doc = Document::new();
    doc.set_tab_bounds(9.0, 10.0, 11.0, 12.0);
    assert_eq!(doc.get_tab_bounds(), (9.0, 10.0, 11.0, 12.0));
}

#[test]
fn pointer_positions_are_tracked() {
    let doc = Document::new();
    doc.set_pointer_position(13.0, 14.0);
    doc.set_pointer_screen_position(15.0, 16.0);
    assert_eq!(doc.get_pointer_position(), (13.0, 14.0));
    assert_eq!(doc.get_pointer_screen_position(), (15.0, 16.0));
}

#[test]
fn document_and_tool_preserve_dockable_and_lifecycle_defaults() {
    let document = Document::new();
    let tool = Tool::new();
    for model in [
        document.clone() as Rc<dyn IDockable>,
        tool.clone() as Rc<dyn IDockable>,
    ] {
        assert_eq!(model.id(), "");
        assert_eq!(model.title(), "");
        assert!(
            model.context().is_none()
                && model.owner().is_none()
                && model.original_owner().is_none()
                && model.factory().is_none()
        );
        assert!(!model.is_empty() && model.is_collapsable());
        assert!(model.proportion().is_nan() && model.collapsed_proportion().is_nan());
        assert_eq!(model.dock(), DockMode::Center);
        assert_eq!(model.docking_state(), DockingWindowState::DOCKED);
        assert_eq!(
            (
                model.column(),
                model.row(),
                model.column_span(),
                model.row_span()
            ),
            (0, 0, 1, 1)
        );
        assert!(!model.is_shared_size_scope());
        assert!(
            model.min_width().is_nan()
                && model.max_width().is_nan()
                && model.min_height().is_nan()
                && model.max_height().is_nan()
        );
        assert!(
            model.can_close()
                && model.can_pin()
                && model.can_float()
                && model.can_drag()
                && model.can_drop()
                && model.can_dock_as_document()
        );
        assert!(!model.keep_pinned_dockable_visible() && !model.is_modified());
        assert!(
            model.pinned_dock_display_mode_override().is_none() && model.pinned_bounds().is_none()
        );
        assert!(model.dock_capability_overrides().is_none() && model.dock_group().is_none());
        let selector = model.as_dock_selector_info().unwrap();
        assert!(selector.show_in_selector() && selector.selector_title().is_none());
        let restrictions = model.as_docking_restrictions().unwrap();
        assert_eq!(
            restrictions.allowed_dock_operations(),
            DockOperationMask::ALL
        );
        assert_eq!(
            restrictions.allowed_drop_operations(),
            DockOperationMask::ALL
        );
        let lifecycle = model.as_docking_window_state().unwrap();
        assert!(!lifecycle.is_open() && !lifecycle.is_active() && !lifecycle.is_selected());
        model.set_can_close(false);
        assert!(model.on_close()); // The default callback is independent of capability validation.
        model.on_selected();
        assert_eq!(model.get_control_recycling_id(), Some(String::new()));
    }
    assert_eq!(document.mdi_bounds(), DockRect::default());
    assert_eq!(tool.mdi_bounds(), DockRect::default());
    assert_eq!(document.mdi_state(), MdiWindowState::Normal);
    assert_eq!(tool.mdi_state(), MdiWindowState::Normal);
    assert_eq!(document.mdi_z_index(), 0);
    assert_eq!(tool.mdi_z_index(), 0);
}

#[test]
fn changed_observation_and_when_any_value_keep_the_concrete_model_identity() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let _subscription = doc
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push((change.property_name, change.sender)));
    let values = Rc::new(RefCell::new(Vec::new()));
    let log = values.clone();
    let _values_subscription = doc
        .when_any_value(prop!(Document => title))
        .subscribe_next(move |value: String| log.borrow_mut().push(value));
    doc.set_title("Document".into());
    doc.set_title("Document".into());
    doc.set_title("Renamed".into());
    assert_eq!(*values.borrow(), ["", "Document", "Renamed"]);
    assert_eq!(changes.borrow().len(), 2);
    for (name, sender) in changes.borrow().iter() {
        assert_eq!(name, "Title");
        assert!(Rc::ptr_eq(sender, &doc));
    }
    changes.borrow_mut().clear();
    _subscription.dispose();
    _values_subscription.dispose();
}

#[test]
fn property_changing_observes_old_value_and_property_changed_observes_new_value() {
    let doc = Document::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let before = events.clone();
    let weak = Rc::downgrade(&doc);
    doc.property_changing().add(Rc::new(move |property| {
        before.borrow_mut().push((
            "changing",
            property.to_owned(),
            weak.upgrade().unwrap().title(),
        ))
    }));
    let after = events.clone();
    let weak = Rc::downgrade(&doc);
    doc.property_changed().add(Rc::new(move |property| {
        after.borrow_mut().push((
            "changed",
            property.to_owned(),
            weak.upgrade().unwrap().title(),
        ))
    }));
    doc.set_title("Next".into());
    assert_eq!(
        *events.borrow(),
        [
            ("changing", "Title".into(), "".into()),
            ("changed", "Title".into(), "Next".into())
        ]
    );
}

#[test]
fn all_float_properties_treat_nan_and_signed_zero_as_dotnet_values() {
    type Setter = fn(&DockableBase, f64);
    type Getter = fn(&DockableBase) -> f64;
    let properties: &[(&str, Setter, Getter)] = &[
        (
            "Proportion",
            DockableBase::set_proportion,
            DockableBase::proportion,
        ),
        (
            "CollapsedProportion",
            DockableBase::set_collapsed_proportion,
            DockableBase::collapsed_proportion,
        ),
        (
            "MinWidth",
            DockableBase::set_min_width,
            DockableBase::min_width,
        ),
        (
            "MaxWidth",
            DockableBase::set_max_width,
            DockableBase::max_width,
        ),
        (
            "MinHeight",
            DockableBase::set_min_height,
            DockableBase::min_height,
        ),
        (
            "MaxHeight",
            DockableBase::set_max_height,
            DockableBase::max_height,
        ),
    ];
    for (name, set, get) in properties {
        let doc = Document::new();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let log = changes.clone();
        doc.property_changed()
            .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
        set(&doc, f64::NAN);
        set(&doc, f64::from_bits(0x7ff8_0000_0000_1234));
        assert!(changes.borrow().is_empty());
        set(&doc, -0.0);
        set(&doc, 0.0);
        assert_eq!(get(&doc).to_bits(), (-0.0_f64).to_bits());
        set(&doc, f64::INFINITY);
        set(&doc, f64::INFINITY);
        set(&doc, f64::NAN);
        set(&doc, f64::NAN);
        assert_eq!(*changes.borrow(), [*name, *name, *name]);
    }
}

#[test]
fn primitive_contexts_use_value_equality_and_reference_contexts_keep_identity() {
    let doc = Document::new();
    let count = Rc::new(Cell::new(0));
    let log = count.clone();
    doc.property_changed()
        .add(Rc::new(move |_| log.set(log.get() + 1)));
    let equal_pairs: Vec<(BoxedValue, BoxedValue)> = vec![
        (Rc::new("value".to_owned()), Rc::new("value".to_owned())),
        (Rc::new(42_i32), Rc::new(42_i32)),
        (Rc::new(f64::NAN), Rc::new(f64::NAN)),
        (Rc::new(-0.0_f64), Rc::new(0.0_f64)),
        (
            Rc::new(DockRect::new(f64::NAN, 0.0, 1.0, 2.0)),
            Rc::new(DockRect::new(f64::NAN, 0.0, 1.0, 2.0)),
        ),
        (Rc::new(DockMode::Left), Rc::new(DockMode::Left)),
    ];
    for (first, equal) in equal_pairs {
        doc.set_context(Some(first.clone()));
        let before = count.get();
        doc.set_context(Some(equal));
        assert_eq!(count.get(), before);
        assert!(Rc::ptr_eq(&doc.context().unwrap(), &first));
    }
    struct Reference;
    let reference: BoxedValue = Rc::new(Reference);
    doc.set_context(Some(reference.clone()));
    let before = count.get();
    doc.set_context(Some(reference));
    assert_eq!(count.get(), before);
    doc.set_context(Some(Rc::new(Reference)));
    assert_eq!(count.get(), before + 1);
    doc.set_context(None);
    assert!(doc.context().is_none());
}

#[test]
fn stored_pinned_bounds_check_only_finite_dimensions_and_do_not_notify_as_properties() {
    let doc = Document::new();
    let changes = Rc::new(Cell::new(0));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |_| log.set(log.get() + 1)));
    let bounds = DockRect::new(f64::NAN, f64::INFINITY, -1.0, 0.0);
    doc.set_pinned_bounds_rect(Some(bounds));
    assert_eq!(doc.pinned_bounds(), Some(bounds));
    for (width, height) in [
        (f64::NAN, 1.0),
        (1.0, f64::NAN),
        (f64::INFINITY, 1.0),
        (1.0, f64::NEG_INFINITY),
    ] {
        doc.set_pinned_bounds(1.0, 2.0, width, height);
        assert!(doc.pinned_bounds().is_none());
    }
    doc.set_pinned_bounds_rect(None);
    let (x, y, width, height) = doc.get_pinned_bounds();
    assert!(x.is_nan() && y.is_nan() && width.is_nan() && height.is_nan());
    assert_eq!(changes.get(), 0);
}

#[test]
fn lifecycle_requests_follow_property_notifications_and_skip_unchanged_values() {
    let factory = Rc::new(RecordingFactory::default());
    factory.supports_state_sync.set(true);
    let doc = Document::new();
    doc.set_factory(Some(factory.clone()));
    let before_sync = Rc::new(RefCell::new(Vec::new()));
    let log = before_sync.clone();
    let recorder = Rc::downgrade(&factory);
    doc.property_changed().add(Rc::new(move |name| {
        log.borrow_mut().push((
            name.to_owned(),
            recorder.upgrade().unwrap().state_requests.borrow().len(),
        ))
    }));
    doc.set_is_open(true);
    doc.set_is_open(true);
    doc.set_is_active(true);
    doc.set_is_selected(true);
    doc.set_docking_state(DockingWindowState::FLOATING);
    assert_eq!(
        *before_sync.borrow(),
        [
            ("IsOpen".into(), 0),
            ("IsActive".into(), 1),
            ("IsSelected".into(), 2),
            ("DockingState".into(), 3)
        ]
    );
    let expected = [
        DockingWindowStateProperty::IsOpen,
        DockingWindowStateProperty::IsActive,
        DockingWindowStateProperty::IsSelected,
        DockingWindowStateProperty::DockingState,
    ];
    let erased: Rc<dyn IDockable> = doc.clone();
    for ((sender, property), expected) in factory.state_requests.borrow().iter().zip(expected) {
        assert_eq!(*property, expected);
        assert!(Rc::ptr_eq(sender, &erased));
    }
}

#[test]
fn state_requests_use_the_current_factory_after_a_reentrant_property_notification() {
    let factory = Rc::new(RecordingFactory::default());
    factory.supports_state_sync.set(true);
    let doc = Document::new();
    doc.set_factory(Some(factory.clone()));
    let weak = Rc::downgrade(&doc);
    doc.property_changed().add(Rc::new(move |name| {
        if name == "IsOpen" {
            weak.upgrade().unwrap().set_factory(None);
        }
    }));
    doc.set_is_open(true);
    assert!(doc.is_open());
    assert!(factory.state_requests.borrow().is_empty());
}

#[test]
fn splitters_preserve_defaults_and_do_not_claim_window_state() {
    let proportional = ProportionalDockSplitter::new();
    assert!(proportional.can_resize());
    assert!(!proportional.resize_preview());
    let grid = GridDockSplitter::new();
    assert_eq!(grid.column(), 0);
    assert_eq!(grid.row(), 0);
    assert_eq!(grid.resize_direction(), GridResizeDirection::Columns);
    let factory = Rc::new(RecordingFactory::default());
    factory.supports_state_sync.set(true);
    for splitter in [proportional as Rc<dyn IDockable>, grid as Rc<dyn IDockable>] {
        assert!(splitter.clone().as_splitter().is_some());
        assert!(splitter.as_docking_window_state().is_none());
        splitter.set_factory(Some(factory.clone()));
        splitter.set_docking_state(DockingWindowState::PINNED);
    }
    assert!(factory.state_requests.borrow().is_empty());
}

#[test]
fn tool_is_also_a_document_and_all_models_compare_by_identity() {
    let tool = Tool::new();
    let erased: Rc<dyn IDockable> = tool.clone();
    assert!(erased.clone().as_tool().is_some());
    assert!(erased.clone().as_document().is_some());
    assert!(erased.as_mdi_document().is_some());
    let doc = Document::new();
    assert!(doc == doc.clone());
    assert!(doc != Document::new());
    let boxed = doc.boxed();
    let from_box = boxed.downcast_ref::<Rc<dyn IDockable>>().unwrap();
    assert!(Rc::ptr_eq(from_box, &(doc.clone() as Rc<dyn IDockable>)));
}

#[test]
fn serialized_owner_retains_parent_while_runtime_factory_links_remain_nonowning() {
    let child = Document::new();
    let parent = Document::new();
    let parent_weak = Rc::downgrade(&parent);
    child.set_owner(Some(parent.clone()));
    child.set_original_owner(Some(parent.clone()));
    let factory = Rc::new(RecordingFactory::default());
    let factory_weak = Rc::downgrade(&factory);
    child.set_factory(Some(factory.clone()));
    drop(parent);
    drop(factory);
    assert!(parent_weak.upgrade().is_some() && factory_weak.upgrade().is_none());
    assert!(
        child.owner().is_some() && child.original_owner().is_some() && child.factory().is_none()
    );
    child.set_owner(None);
    assert!(parent_weak.upgrade().is_none());
    assert!(child.owner().is_none() && child.original_owner().is_none());
}

struct CustomDockable {
    base: DockableBase,
    selected: Cell<usize>,
    bounds_callbacks: Cell<usize>,
    other_callbacks: RefCell<Vec<&'static str>>,
}
reactive_object!(CustomDockable, base);
crate::impl_dockable!(CustomDockable);
impl DockableBaseImpl for CustomDockable {
    fn on_close(_this: &Self) -> bool {
        false
    }
    fn on_selected(this: &Self) {
        this.selected.set(this.selected.get() + 1);
    }
    fn on_visible_bounds_changed(this: &Self, x: f64, y: f64, width: f64, height: f64) {
        assert_eq!(this.get_visible_bounds(), (x, y, width, height));
        this.bounds_callbacks.set(this.bounds_callbacks.get() + 1);
    }
    fn on_pinned_bounds_changed(this: &Self, x: f64, y: f64, width: f64, height: f64) {
        assert_eq!(this.get_pinned_bounds(), (x, y, width, height));
        this.other_callbacks.borrow_mut().push("pinned");
    }
    fn on_tab_bounds_changed(this: &Self, x: f64, y: f64, width: f64, height: f64) {
        assert_eq!(this.get_tab_bounds(), (x, y, width, height));
        this.other_callbacks.borrow_mut().push("tab");
    }
    fn on_pointer_position_changed(this: &Self, x: f64, y: f64) {
        assert_eq!(this.get_pointer_position(), (x, y));
        this.other_callbacks.borrow_mut().push("pointer");
    }
    fn on_pointer_screen_position_changed(this: &Self, x: f64, y: f64) {
        assert_eq!(this.get_pointer_screen_position(), (x, y));
        this.other_callbacks.borrow_mut().push("screen");
    }
}

#[test]
fn composed_subclasses_override_the_original_virtual_callbacks() {
    let model = Rc::new_cyclic(|weak: &Weak<CustomDockable>| {
        let runtime: Weak<dyn DockableBaseImpl> = weak.clone();
        CustomDockable {
            base: DockableBase::construct(runtime),
            selected: Cell::new(0),
            bounds_callbacks: Cell::new(0),
            other_callbacks: RefCell::new(Vec::new()),
        }
    });
    assert!(!model.on_close());
    model.on_selected();
    model.set_visible_bounds(1.0, 2.0, 3.0, 4.0);
    for _ in 0..2 {
        model.set_pinned_bounds(5.0, 6.0, 7.0, 8.0);
        model.set_tab_bounds(9.0, 10.0, 11.0, 12.0);
        model.set_pointer_position(13.0, 14.0);
        model.set_pointer_screen_position(15.0, 16.0);
    }
    model.set_visible_bounds(1.0, 2.0, 3.0, 4.0);
    assert_eq!(model.selected.get(), 1);
    assert_eq!(model.bounds_callbacks.get(), 2);
    assert_eq!(
        *model.other_callbacks.borrow(),
        ["pinned", "tab", "pointer", "screen", "pinned", "tab", "pointer", "screen"]
    );
}

#[test]
fn document_and_tool_mdi_properties_notify_only_on_value_changes() {
    let doc = Document::new();
    let tool = Tool::new();
    for model in [doc as Rc<dyn IDockable>, tool as Rc<dyn IDockable>] {
        let mdi = model.as_mdi_document().unwrap();
        mdi.set_mdi_bounds(DockRect::new(f64::NAN, 2.0, 3.0, 4.0));
        mdi.set_mdi_state(MdiWindowState::Maximized);
        mdi.set_mdi_z_index(-3);
        assert_eq!(mdi.mdi_bounds(), DockRect::new(f64::NAN, 2.0, 3.0, 4.0));
        assert_eq!(mdi.mdi_state(), MdiWindowState::Maximized);
        assert_eq!(mdi.mdi_z_index(), -3);
    }
    let doc = Document::new();
    let tool = Tool::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let log = changes.clone();
    tool.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    for mdi in [
        doc as Rc<dyn ferrodock_model::controls::IMdiDocument>,
        tool as Rc<dyn ferrodock_model::controls::IMdiDocument>,
    ] {
        for _ in 0..2 {
            mdi.set_mdi_bounds(DockRect::new(f64::NAN, 2.0, 3.0, 4.0));
            mdi.set_mdi_state(MdiWindowState::Maximized);
            mdi.set_mdi_z_index(-3);
        }
    }
    assert_eq!(
        *changes.borrow(),
        [
            "MdiBounds",
            "MdiState",
            "MdiZIndex",
            "MdiBounds",
            "MdiState",
            "MdiZIndex"
        ]
    );
}

#[test]
fn tool_lifecycle_uses_the_same_factory_sync_and_notification_contract_as_document() {
    let factory = Rc::new(RecordingFactory::default());
    factory.supports_state_sync.set(true);
    let tool = Tool::new();
    tool.set_factory(Some(factory.clone()));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let recorder = Rc::downgrade(&factory);
    tool.property_changed().add(Rc::new(move |name| {
        log.borrow_mut().push((
            name.to_owned(),
            recorder.upgrade().unwrap().state_requests.borrow().len(),
        ))
    }));
    let state = tool.as_docking_window_state().unwrap();
    for _ in 0..2 {
        state.set_is_open(true);
        state.set_is_active(true);
        state.set_is_selected(true);
        state.set_docking_state(DockingWindowState::FLOATING);
    }
    assert!(state.is_open() && state.is_active() && state.is_selected());
    assert_eq!(state.docking_state(), DockingWindowState::FLOATING);
    assert_eq!(
        *changes.borrow(),
        [
            ("IsOpen".into(), 0),
            ("IsActive".into(), 1),
            ("IsSelected".into(), 2),
            ("DockingState".into(), 3)
        ]
    );
    let tool: Rc<dyn IDockable> = tool;
    for (sender, _) in factory.state_requests.borrow().iter() {
        assert!(Rc::ptr_eq(sender, &tool));
    }
}

#[test]
fn backlink_setters_notify_for_identity_changes_without_duplicate_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let parent = Document::new();
    let other = Document::new();
    for value in [
        Some(parent.clone() as Rc<dyn IDockable>),
        Some(parent.clone()),
        Some(other.clone()),
        None,
        None,
    ] {
        doc.set_owner(value.clone());
        doc.set_original_owner(value);
    }
    let factory = Rc::new(RecordingFactory::default());
    let other_factory = Rc::new(RecordingFactory::default());
    for value in [
        Some(factory.clone() as Rc<dyn IFactory>),
        Some(factory),
        Some(other_factory),
        None,
        None,
    ] {
        doc.set_factory(value);
    }
    assert_eq!(
        *changes.borrow(),
        [
            "Owner",
            "OriginalOwner",
            "Owner",
            "OriginalOwner",
            "Owner",
            "OriginalOwner",
            "Factory",
            "Factory",
            "Factory"
        ]
    );
}

#[test]
fn splitter_properties_mutate_through_their_original_interfaces_and_notify_once() {
    use ferrodock_model::controls::{IGridDockSplitter, IProportionalDockSplitter};
    let proportional = ProportionalDockSplitter::new();
    let grid = GridDockSplitter::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    proportional
        .property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let log = changes.clone();
    grid.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let proportional: Rc<dyn IProportionalDockSplitter> = proportional;
    let grid: Rc<dyn IGridDockSplitter> = grid;
    for _ in 0..2 {
        proportional.set_can_resize(false);
        proportional.set_resize_preview(true);
        grid.set_resize_direction(GridResizeDirection::Rows);
    }
    assert!(!proportional.can_resize() && proportional.resize_preview());
    assert_eq!(grid.resize_direction(), GridResizeDirection::Rows);
    assert_eq!(
        *changes.borrow(),
        ["CanResize", "ResizePreview", "ResizeDirection"]
    );
}

#[test]
fn dialog_reference_results_reuse_factory_type_queries_without_cloning_models() {
    use ferrodock_model::services::DialogResult;
    let document = Document::new();
    let canonical: ferrodock_model::BoxedValue = Rc::new(document.clone() as Rc<dyn IDockable>);
    let concrete = Option::<Rc<Document>>::from_dialog_result(Some(canonical.clone())).unwrap();
    assert!(Rc::ptr_eq(&concrete, &document));
    let contract =
        Option::<Rc<dyn ferrodock_model::controls::IDocument>>::from_dialog_result(Some(canonical))
            .unwrap();
    assert!(Rc::ptr_eq(
        &contract,
        &(document as Rc<dyn ferrodock_model::controls::IDocument>)
    ));
}
