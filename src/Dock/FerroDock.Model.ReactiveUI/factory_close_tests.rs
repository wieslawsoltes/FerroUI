//! Actual close eligibility/lifecycle and generated-container tracking regressions.
use crate::controls::*;
use crate::core::{DockWindow, DockableBase, DockableBaseImpl};
use crate::factory_init_tests::factory;
use crate::factory_parameter_tests::RecordingFactory;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferrodock_model::{BoxedValue, FactoryBaseImpl};
use ferroui_reactiveui::reactive_object;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

struct Probe {
    base: DockableBase,
    allow: Cell<bool>,
    closes: Cell<usize>,
    on_drop: RefCell<Option<Rc<dyn Fn()>>>,
}
reactive_object!(Probe, base);
crate::impl_dockable!(Probe);
impl DockableBaseImpl for Probe {
    fn on_close(this: &Self) -> bool {
        this.closes.set(this.closes.get() + 1);
        this.allow.get()
    }
}
impl Probe {
    fn new(allow: bool) -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: DockableBase::construct(weak.clone()),
            allow: Cell::new(allow),
            closes: Cell::new(0),
            on_drop: RefCell::new(None),
        })
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        let callback = self.on_drop.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
    }
}
struct Layout {
    factory: Rc<RecordingFactory>,
    root: Rc<RootDock>,
    dock: Rc<DocumentDock>,
    first: Rc<Document>,
    second: Rc<Document>,
}
impl Layout {
    fn new() -> Self {
        let factory = factory();
        let root = RootDock::new();
        let dock = DocumentDock::new();
        dock.set_is_collapsable(false);
        let first = Document::new();
        first.set_id("first".into());
        let second = Document::new();
        second.set_id("second".into());
        dock.set_visible_dockables(Some(ListHandle::from_items(vec![
            Some(first.clone() as Rc<dyn IDockable>),
            Some(second.clone()),
        ])));
        dock.set_active_dockable(Some(first.clone()));
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            dock.clone() as Rc<dyn IDockable>
        )])));
        factory.init_dockable(Some(root.clone()), None);
        Self {
            factory,
            root,
            dock,
            first,
            second,
        }
    }
}

#[test]
fn null_close_returns_before_policy_callbacks_or_events() {
    let factory = factory();
    *factory.on_close_core.borrow_mut() = Some(Rc::new(|_| panic!("no core for null")));
    factory.dockable_closing(Rc::new(|_| panic!("no closing for null")));
    factory.close_dockable(None);
    assert!(factory.calls.borrow().is_empty());
}

#[test]
fn null_item_lookup_preserves_source_defensive_null_result() {
    assert!(factory().get_container_from_item(None).is_none());
}

#[test]
fn close_last_policy_distinguishes_null_zero_one_and_two_visible_children() {
    for count in [None, Some(0), Some(1), Some(2)] {
        let factory = factory();
        let dock = DocumentDock::new();
        dock.set_can_close_last_dockable(false);
        let item = Probe::new(true);
        item.set_owner(Some(dock.clone()));
        if let Some(count) = count {
            let items = if count == 0 {
                vec![]
            } else if count == 1 {
                vec![item.clone() as Rc<dyn IDockable>]
            } else {
                vec![item.clone() as Rc<dyn IDockable>, Document::new()]
            };
            dock.set_visible_dockables(Some(ListHandle::from_items(
                (items).into_iter().map(Some).collect(),
            )));
        }
        let core_calls = Rc::new(Cell::new(0));
        let log = core_calls.clone();
        *factory.on_close_core.borrow_mut() = Some(Rc::new(move |_| log.set(log.get() + 1)));
        factory.close_dockable(Some(item.clone()));
        assert_eq!(
            core_calls.get(),
            usize::from(count.is_none_or(|count| count > 1))
        );
        assert_eq!(item.closes.get(), 0);
    }
}

#[test]
fn capability_denial_skips_core_and_model_on_close() {
    let factory = factory();
    let item = Probe::new(true);
    item.set_can_close(false);
    *factory.on_close_core.borrow_mut() = Some(Rc::new(|_| panic!("capability denies close")));
    factory.close_dockable(Some(item.clone()));
    assert_eq!(item.closes.get(), 0);
}

#[test]
fn protected_core_bypasses_eligibility_but_preserves_model_and_event_cancellation() {
    let factory = factory();
    let root = RootDock::new();
    let dock = DocumentDock::new();
    dock.set_can_close_last_dockable(false);
    dock.set_is_collapsable(false);
    let item = Probe::new(false);
    item.set_can_close(false);
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        item.clone() as Rc<dyn IDockable>
    )])));
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        dock.clone() as Rc<dyn IDockable>
    )])));
    factory.init_dockable(Some(root.clone()), None);
    FactoryBaseImpl::close_dockable_core(factory.as_ref(), item.clone());
    assert_eq!(item.closes.get(), 1);
    assert_eq!(dock.visible_dockables().unwrap().count(), 1);
    factory.dockable_closing(Rc::new(|args| args.set_cancel(false)));
    FactoryBaseImpl::close_dockable_core(factory.as_ref(), item.clone());
    assert_eq!(item.closes.get(), 2);
    assert_eq!(dock.visible_dockables().unwrap().count(), 0);
}

#[test]
fn canceled_close_does_not_resolve_source_hide_remove_or_raise_closed() {
    let layout = Layout::new();
    *layout.factory.on_source_closing.borrow_mut() =
        Some(Rc::new(|_| panic!("cancel before source resolution")));
    layout
        .factory
        .dockable_closing(Rc::new(|args| args.set_cancel(true)));
    layout
        .factory
        .dockable_closed(Rc::new(|_| panic!("canceled close has no closed event")));
    layout.factory.close_dockable(Some(layout.first));
    assert_eq!(layout.dock.visible_dockables().unwrap().count(), 2);
    assert!(layout.root.hidden_dockables().is_none());
}

#[test]
fn removing_non_active_and_active_documents_preserves_source_selection_and_closed_event_order() {
    for active in [false, true] {
        let layout = Layout::new();
        let closing: Rc<dyn IDockable> = if active {
            layout.first.clone()
        } else {
            layout.second.clone()
        };
        let remaining: Rc<dyn IDockable> = if active {
            layout.second.clone()
        } else {
            layout.first.clone()
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        layout
            .factory
            .dockable_closing(Rc::new(move |_| log.borrow_mut().push("closing")));
        let log = events.clone();
        layout
            .factory
            .dockable_removed(Rc::new(move |_| log.borrow_mut().push("removed")));
        let log = events.clone();
        let source = layout.dock.clone();
        let expected = closing.clone();
        layout.factory.dockable_closed(Rc::new(move |args| {
            assert!(Rc::ptr_eq(&args.dockable().unwrap(), &expected));
            assert!(!source
                .visible_dockables()
                .unwrap()
                .contains(&Some((expected).clone())));
            log.borrow_mut().push("closed");
        }));
        layout.factory.close_dockable(Some(closing));
        assert_eq!(*events.borrow(), ["closing", "removed", "closed"]);
        assert!(Rc::ptr_eq(
            &layout.dock.active_dockable().unwrap(),
            &remaining
        ));
        assert_eq!(layout.dock.visible_dockables().unwrap().count(), 1);
    }
}

#[test]
fn hide_flags_cover_tools_documents_and_tools_document_membership() {
    for is_tool in [false, true] {
        for hide_tools in [false, true] {
            for hide_documents in [false, true] {
                let factory = factory();
                factory.set_hide_tools_on_close(hide_tools);
                factory.set_hide_documents_on_close(hide_documents);
                let root = RootDock::new();
                let dock = DocumentDock::new();
                dock.set_is_collapsable(false);
                let item: Rc<dyn IDockable> = if is_tool {
                    Tool::new()
                } else {
                    Document::new()
                };
                dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(item.clone())])));
                root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
                    dock.clone() as Rc<dyn IDockable>
                )])));
                factory.init_dockable(Some(root.clone()), None);
                factory.close_dockable(Some(item.clone()));
                let expected_hidden = hide_documents || (is_tool && hide_tools);
                assert_eq!(
                    root.hidden_dockables()
                        .is_some_and(|items| items.contains(&Some((item).clone()))),
                    expected_hidden
                );
                assert_eq!(dock.visible_dockables().unwrap().count(), 0);
            }
        }
    }
}

#[test]
fn source_closing_override_controls_hide_bypass_and_is_invoked_between_closing_and_removal() {
    let layout = Layout::new();
    layout.factory.set_hide_documents_on_close(true);
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    layout
        .factory
        .dockable_closing(Rc::new(move |_| log.borrow_mut().push("closing")));
    let log = events.clone();
    *layout.factory.on_source_closing.borrow_mut() = Some(Rc::new(move |_| {
        log.borrow_mut().push("source");
        true
    }));
    let log = events.clone();
    layout
        .factory
        .dockable_removed(Rc::new(move |_| log.borrow_mut().push("removed")));
    let log = events.clone();
    layout
        .factory
        .dockable_closed(Rc::new(move |_| log.borrow_mut().push("closed")));
    layout.factory.close_dockable(Some(layout.first));
    assert_eq!(*events.borrow(), ["closing", "source", "removed", "closed"]);
    assert!(layout.root.hidden_dockables().is_none());
}

#[test]
fn source_handler_failure_and_removal_observer_failure_keep_their_distinct_partial_effects() {
    for source_fails in [true, false] {
        let layout = Layout::new();
        if source_fails {
            *layout.factory.on_source_closing.borrow_mut() =
                Some(Rc::new(|_| panic!("source failure")));
        } else {
            layout
                .factory
                .dockable_removed(Rc::new(|_| panic!("removed observer failure")));
        }
        layout
            .factory
            .dockable_closed(Rc::new(|_| panic!("closed must not run after failure")));
        assert!(catch_unwind(AssertUnwindSafe(|| layout
            .factory
            .close_dockable(Some(layout.first.clone()))))
        .is_err());
        assert_eq!(
            layout.dock.visible_dockables().unwrap().count(),
            if source_fails { 2 } else { 1 }
        );
    }
}

#[test]
fn close_ranges_now_remove_real_documents_and_honor_last_child_policy() {
    let layout = Layout::new();
    layout.dock.set_can_close_last_dockable(false);
    layout.factory.close_all_dockables(layout.first.clone());
    assert_eq!(layout.dock.visible_dockables().unwrap().count(), 1);
    assert!(Rc::ptr_eq(
        (layout.dock.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &(layout.first as Rc<dyn IDockable>)
    ));
}

#[test]
fn close_window_runs_real_child_close_operations_before_container_close() {
    let layout = Layout::new();
    let window = DockWindow::new();
    window.set_layout(Some(layout.root.clone()));
    let closed = Rc::new(RefCell::new(Vec::new()));
    let log = closed.clone();
    layout.factory.dockable_closed(Rc::new(move |args| {
        log.borrow_mut().push(args.dockable().unwrap().id())
    }));
    layout.factory.close_window(Some(window));
    assert_eq!(*closed.borrow(), ["first", "second", ""]);
    assert_eq!(layout.root.visible_dockables().unwrap().count(), 0);
}

#[test]
fn tracking_keeps_duplicate_item_order_and_retracking_moves_the_container_to_the_end() {
    let factory = factory();
    let owner = DocumentDock::new();
    let first = Document::new();
    let second = Document::new();
    let item: BoxedValue = Rc::new("same".to_owned());
    for document in [first.clone(), second.clone()] {
        FactoryBaseImpl::track_items_source_dockable_with_item(
            factory.as_ref(),
            document,
            owner.clone(),
            Some(item.clone()),
        );
    }
    assert!(Rc::ptr_eq(
        &factory
            .get_container_from_item(Some(Rc::new("same".to_owned())))
            .unwrap(),
        &(first.clone() as Rc<dyn IDockable>)
    ));
    FactoryBaseImpl::track_items_source_dockable_with_item(
        factory.as_ref(),
        first.clone(),
        owner.clone(),
        Some(item.clone()),
    );
    assert!(Rc::ptr_eq(
        &factory.get_container_from_item(Some(item.clone())).unwrap(),
        &(second.clone() as Rc<dyn IDockable>)
    ));
    FactoryBaseImpl::untrack_items_source_dockable(factory.as_ref(), second);
    assert!(Rc::ptr_eq(
        &factory.get_container_from_item(Some(item.clone())).unwrap(),
        &(first.clone() as Rc<dyn IDockable>)
    ));
    FactoryBaseImpl::untrack_items_source_dockable(factory.as_ref(), first.clone());
    FactoryBaseImpl::untrack_items_source_dockable(factory.as_ref(), first);
    assert!(factory.get_container_from_item(Some(item)).is_none());
}

#[test]
fn implicit_context_tracking_replaces_old_item_and_explicit_null_retains_only_owner_mapping() {
    let factory = factory();
    let owner = DocumentDock::new();
    let other = ToolDock::new();
    let document = Document::new();
    let original: BoxedValue = Rc::new(12_i32);
    let next: BoxedValue = Rc::new(13_i32);
    document.set_context(Some(original.clone()));
    FactoryBaseImpl::track_items_source_dockable(factory.as_ref(), document.clone(), owner.clone());
    document.set_context(Some(next.clone()));
    assert!(factory
        .get_container_from_item(Some(next.clone()))
        .is_none());
    FactoryBaseImpl::track_items_source_dockable(factory.as_ref(), document.clone(), other.clone());
    assert!(factory.get_container_from_item(Some(original)).is_none());
    assert!(factory
        .get_container_from_item(Some(next.clone()))
        .is_some());
    assert!(Rc::ptr_eq(
        &FactoryBaseImpl::get_tracked_items_source_owner(factory.as_ref(), document.clone())
            .unwrap(),
        &(other.clone() as Rc<dyn IDockable>)
    ));
    FactoryBaseImpl::track_items_source_dockable_with_item(
        factory.as_ref(),
        document.clone(),
        owner.clone(),
        None,
    );
    assert!(factory.get_container_from_item(Some(next)).is_none());
    assert!(Rc::ptr_eq(
        &FactoryBaseImpl::get_tracked_items_source_owner(factory.as_ref(), document).unwrap(),
        &(owner as Rc<dyn IDockable>)
    ));
}

#[test]
fn tracking_preserves_container_and_source_owner_lifetimes_until_explicit_untrack() {
    let factory = factory();
    let owner = DocumentDock::new();
    let document = Document::new();
    let weak_document = Rc::downgrade(&document);
    let weak_owner = Rc::downgrade(&owner);
    let item: BoxedValue = Rc::new(10_i32);
    FactoryBaseImpl::track_items_source_dockable_with_item(
        factory.as_ref(),
        document.clone(),
        owner.clone(),
        Some(item.clone()),
    );
    drop(owner);
    drop(document);
    assert!(weak_document.upgrade().is_some() && weak_owner.upgrade().is_some());
    let retained = factory.get_container_from_item(Some(item.clone())).unwrap();
    FactoryBaseImpl::untrack_items_source_dockable(factory.as_ref(), retained.clone());
    assert!(weak_owner.upgrade().is_none());
    assert!(factory.get_container_from_item(Some(item)).is_none());
    drop(retained);
    assert!(weak_document.upgrade().is_none());
}

#[test]
fn null_item_mapping_does_not_retain_its_key_and_expired_noncyclic_owner_is_pruned() {
    let factory = factory();
    let owner = DocumentDock::new();
    let weak_owner = Rc::downgrade(&owner);
    let document = Document::new();
    let weak_document = Rc::downgrade(&document);
    FactoryBaseImpl::track_items_source_dockable_with_item(
        factory.as_ref(),
        document.clone(),
        owner.clone(),
        None,
    );
    drop(owner);
    drop(document);
    assert!(weak_document.upgrade().is_none());
    // This performs the native expired-key pruning; arbitrary ephemeron cycle collection is tracked separately.
    assert!(
        FactoryBaseImpl::get_tracked_items_source_owner(factory.as_ref(), Document::new())
            .is_none()
    );
    assert!(weak_owner.upgrade().is_none());
}

#[test]
fn item_lookup_preserves_float_nan_signed_zero_and_geometry_value_equality_and_numeric_type_identity(
) {
    let factory = factory();
    let owner = DocumentDock::new();
    let nan = f64::from_bits(0xfff8_0000_0000_0042);
    let pairs: Vec<(BoxedValue, BoxedValue)> = vec![
        (Rc::new(f64::NAN), Rc::new(nan)),
        (Rc::new(-0.0_f64), Rc::new(0.0_f64)),
        (Rc::new(f32::NAN), Rc::new(f32::from_bits(0xffc0_0042))),
        (Rc::new(-0.0_f32), Rc::new(0.0_f32)),
        (
            Rc::new(DockPoint::new(nan, -0.0)),
            Rc::new(DockPoint::new(f64::NAN, 0.0)),
        ),
        (
            Rc::new(DockRect::new(nan, -0.0, 10.0, 20.0)),
            Rc::new(DockRect::new(f64::NAN, 0.0, 10.0, 20.0)),
        ),
        (Rc::new(123_i32), Rc::new(123_i32)),
    ];
    for (item, equal) in pairs {
        let document = Document::new();
        FactoryBaseImpl::track_items_source_dockable_with_item(
            factory.as_ref(),
            document.clone(),
            owner.clone(),
            Some(item.clone()),
        );
        assert!(Rc::ptr_eq(
            &factory.get_container_from_item(Some(equal)).unwrap(),
            &(document.clone() as Rc<dyn IDockable>)
        ));
        FactoryBaseImpl::untrack_items_source_dockable(factory.as_ref(), document);
        assert!(factory.get_container_from_item(Some(item)).is_none());
    }
    FactoryBaseImpl::track_items_source_dockable_with_item(
        factory.as_ref(),
        Document::new(),
        owner,
        Some(Rc::new(123_i32)),
    );
    assert!(factory
        .get_container_from_item(Some(Rc::new(123_i64)))
        .is_none());
}

#[test]
fn owner_release_can_reenter_item_lookup_during_untrack_without_borrow_conflicts() {
    let factory = factory();
    let owner = Probe::new(true);
    let weak_owner = Rc::downgrade(&owner);
    let document = Document::new();
    let value: BoxedValue = Rc::new("item".to_owned());
    let weak_factory = Rc::downgrade(&factory);
    let item = value.clone();
    let calls = Rc::new(Cell::new(0));
    let log = calls.clone();
    *owner.on_drop.borrow_mut() = Some(Rc::new(move || {
        let factory = weak_factory.upgrade().unwrap();
        // Owner entry is removed, while reverse-item cleanup has not run yet.
        assert!(factory
            .get_container_from_item(Some(item.clone()))
            .is_none());
        log.set(log.get() + 1);
    }));
    FactoryBaseImpl::track_items_source_dockable_with_item(
        factory.as_ref(),
        document.clone(),
        owner.clone(),
        Some(value.clone()),
    );
    drop(owner);
    assert!(weak_owner.upgrade().is_some());
    FactoryBaseImpl::untrack_items_source_dockable(factory.as_ref(), document);
    assert!(weak_owner.upgrade().is_none());
    assert_eq!(calls.get(), 1);
    assert!(factory.get_container_from_item(Some(value)).is_none());
}

#[test]
fn implicit_tracking_overload_dispatches_explicit_item_override_with_current_context_identity() {
    let factory = factory();
    let document = Document::new();
    let owner = DocumentDock::new();
    let context: BoxedValue = Rc::new(42_i32);
    document.set_context(Some(context.clone()));
    let expected_item = document.clone();
    let expected_owner = owner.clone();
    let expected_context = context.clone();
    let calls = Rc::new(Cell::new(0));
    let log = calls.clone();
    *factory.on_track_item.borrow_mut() = Some(Rc::new(move |item, owner, value| {
        assert!(Rc::ptr_eq(
            &item,
            &(expected_item.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &owner,
            &(expected_owner.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(&value.unwrap(), &expected_context));
        log.set(log.get() + 1);
    }));
    FactoryBaseImpl::track_items_source_dockable(factory.as_ref(), document, owner);
    assert_eq!(calls.get(), 1);
    assert!(factory.get_container_from_item(Some(context)).is_none());
}

#[test]
fn closing_the_roots_only_active_document_clears_selection_and_focus() {
    let factory = factory();
    let root = RootDock::new();
    root.set_visible_dockables(Some(ListHandle::new()));
    factory.init_dockable(Some(root.clone()), None);
    let document = Document::new();
    factory.add_dockable(Some(root.clone()), Some(document.clone()));
    root.set_active_dockable(Some(document.clone()));
    assert!(Rc::ptr_eq(
        &root.focused_dockable().unwrap(),
        &(document.clone() as Rc<dyn IDockable>)
    ));
    factory.close_dockable(Some(document));
    assert!(root.active_dockable().is_none());
    assert!(root.focused_dockable().is_none());
    assert_eq!(root.visible_dockables().unwrap().count(), 0);
}
