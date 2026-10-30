use crate::controls::*;
use crate::core::DockWindow;
use crate::factory_parameter_tests::{ModelCall, RecordingFactory};
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

struct Layout {
    factory: Rc<RecordingFactory>,
    root: Rc<RootDock>,
    owner: Rc<ToolDock>,
    tool: Rc<Tool>,
}
impl Layout {
    fn new() -> Self {
        let factory = RecordingFactory::with_base();
        let root = RootDock::new();
        let owner = ToolDock::new();
        let tool = Tool::new();
        root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(owner.clone())])));
        owner.set_owner(Some(root.clone()));
        owner.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(tool.clone())])));
        tool.set_owner(Some(owner.clone()));
        Self {
            factory,
            root,
            owner,
            tool,
        }
    }
    fn update(&self) {
        self.factory
            .base
            .get()
            .unwrap()
            .update_docking_window_state(Some(self.tool.clone()));
    }
    fn connect(&self) {
        self.factory.supports_state_sync.set(true);
        self.tool.set_factory(Some(self.factory.clone()));
    }
    fn resolve(&self, item: Rc<dyn IDockable>) -> DockingWindowState {
        self.factory
            .base
            .get()
            .unwrap()
            .resolve_docking_window_state(Some(item))
    }
    fn calls(&self) -> Vec<&'static str> {
        self.factory
            .calls
            .borrow()
            .iter()
            .map(|(name, _)| *name)
            .collect()
    }
}

#[test]
fn resolves_document_location_on_container_and_descendant_and_hidden_ancestors() {
    let layout = Layout::new();
    assert_eq!(
        layout.resolve(layout.tool.clone()),
        DockingWindowState::DOCKED
    );
    let docs = DocumentDock::new();
    docs.set_owner(Some(layout.root.clone()));
    layout.tool.set_owner(Some(docs.clone()));
    assert_eq!(layout.resolve(docs.clone()), DockingWindowState::DOCUMENT);
    assert_eq!(
        layout.resolve(layout.tool.clone()),
        DockingWindowState::DOCUMENT
    );
    layout
        .root
        .set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(docs.clone())])));
    assert_eq!(
        layout.resolve(layout.tool.clone()),
        DockingWindowState::DOCUMENT | DockingWindowState::HIDDEN
    );
    layout.root.set_hidden_dockables(None);
    layout.tool.set_owner(Some(layout.owner.clone()));
    layout.tool.set_original_owner(Some(docs.clone()));
    assert_eq!(
        layout.resolve(layout.tool.clone()),
        DockingWindowState::DOCUMENT
    );
}

#[test]
fn every_pinned_strip_overrides_document_location_and_missing_root_is_not_pinned() {
    let layout = Layout::new();
    let docs = DocumentDock::new();
    layout.tool.set_original_owner(Some(docs.clone()));
    for side in 0..4 {
        let items = Some(ListHandle::from_plain(vec![Some(
            layout.tool.clone() as Rc<dyn IDockable>
        )]));
        match side {
            0 => layout.root.set_left_pinned_dockables(items),
            1 => layout.root.set_right_pinned_dockables(items),
            2 => layout.root.set_top_pinned_dockables(items),
            _ => layout.root.set_bottom_pinned_dockables(items),
        }
        assert!(layout
            .factory
            .base
            .get()
            .unwrap()
            .is_dockable_pinned(Some(layout.tool.clone()), None));
        assert_eq!(
            layout.resolve(layout.tool.clone()),
            DockingWindowState::PINNED
        );
        layout.root.set_left_pinned_dockables(None);
        layout.root.set_right_pinned_dockables(None);
        layout.root.set_top_pinned_dockables(None);
        layout.root.set_bottom_pinned_dockables(None);
    }
    layout.tool.set_owner(None);
    assert!(!layout
        .factory
        .base
        .get()
        .unwrap()
        .is_dockable_pinned(Some(layout.tool.clone()), None));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .unpin_dockable(Some(layout.tool.clone()));
    assert!(layout.calls().is_empty());
}

#[test]
fn floating_state_uses_current_or_original_owner_root_window_relationship() {
    let layout = Layout::new();
    let floating = RootDock::new();
    let window = DockWindow::new();
    window.set_owner(Some(layout.root.clone()));
    floating.set_window(Some(window.clone()));
    let docs = DocumentDock::new();
    docs.set_owner(Some(floating.clone()));
    layout.tool.set_original_owner(Some(docs.clone()));
    layout
        .root
        .set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(
            layout.tool.clone(),
        )])));
    assert_eq!(
        layout.resolve(layout.tool.clone()),
        DockingWindowState::DOCUMENT | DockingWindowState::FLOATING | DockingWindowState::HIDDEN
    );
    layout.tool.set_owner(Some(docs.clone()));
    layout.tool.set_original_owner(None);
    assert_eq!(
        layout.resolve(layout.tool.clone()),
        DockingWindowState::DOCUMENT | DockingWindowState::FLOATING
    );
    window.set_owner(None);
    assert_eq!(
        layout.resolve(layout.tool.clone()),
        DockingWindowState::DOCUMENT
    );
}

#[test]
fn state_flags_notify_in_source_order_and_do_not_dispatch_docking_operations() {
    let layout = Layout::new();
    let docs = DocumentDock::new();
    docs.set_owner(Some(layout.root.clone()));
    docs.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(
        layout.tool.clone(),
    )])));
    docs.set_active_dockable(Some(layout.tool.clone()));
    docs.set_is_active(true);
    layout.tool.set_owner(Some(docs.clone()));
    layout.root.set_focused_dockable(Some(layout.tool.clone()));
    layout.connect();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    layout
        .tool
        .property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_string())));
    layout.update();
    assert_eq!(
        *changes.borrow(),
        ["DockingState", "IsOpen", "IsSelected", "IsActive"]
    );
    assert!(layout.tool.is_open() && layout.tool.is_selected() && layout.tool.is_active());
    assert!(layout.calls().is_empty());
    layout.update();
    assert_eq!(changes.borrow().len(), 4);
}

#[test]
fn hidden_container_forces_open_selection_and_activation_flags_false() {
    let layout = Layout::new();
    layout.owner.set_active_dockable(Some(layout.tool.clone()));
    layout.owner.set_is_active(true);
    layout.root.set_focused_dockable(Some(layout.tool.clone()));
    layout.update();
    assert!(layout.tool.is_open() && layout.tool.is_selected() && layout.tool.is_active());
    layout
        .root
        .set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(
            layout.owner.clone(),
        )])));
    layout.update();
    assert!(!layout.tool.is_open() && !layout.tool.is_selected() && !layout.tool.is_active());
}

#[test]
fn recursive_update_preserves_visible_duplicates_and_deduplicates_only_split_extras() {
    let layout = Layout::new();
    let split = SplitViewDock::new();
    split.set_id("split".into());
    let pane = Tool::new();
    pane.set_id("pane".into());
    let content = Tool::new();
    content.set_id("content".into());
    split.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(pane.clone()),
        Some(pane.clone()),
    ])));
    split.set_pane_dockable(Some(pane));
    split.set_content_dockable(Some(content.clone()));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    *layout.factory.on_update_state.borrow_mut() =
        Some(Rc::new(move |item| log.borrow_mut().push(item.id())));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .update_docking_window_state_recursive(Some(split.clone()));
    assert_eq!(*calls.borrow(), ["split", "pane", "pane", "content"]);
    calls.borrow_mut().clear();
    split.set_visible_dockables(None);
    split.set_pane_dockable(Some(content));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .update_docking_window_state_recursive(Some(split));
    assert_eq!(*calls.borrow(), ["split", "content"]);
}

#[test]
fn external_open_requests_restore_hidden_items_and_reconcile_rejected_operations() {
    let layout = Layout::new();
    layout
        .root
        .set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(
            layout.tool.clone(),
        )])));
    layout.connect();
    layout.tool.set_is_open(true);
    assert_eq!(layout.calls(), ["restore_dockable"]);
    assert!(!layout.tool.is_open()); // Recording operation leaves the item hidden.
    let root = layout.root.clone();
    *layout.factory.on_operation.borrow_mut() = Some(Rc::new(move |name, _| {
        assert_eq!(name, "restore_dockable");
        root.set_hidden_dockables(None);
    }));
    layout.tool.set_is_open(true);
    assert!(layout.tool.is_open());
    assert_eq!(layout.calls(), ["restore_dockable", "restore_dockable"]);
}

#[test]
fn close_request_suppresses_reentrant_writes_and_updates_flags_after_operation() {
    let layout = Layout::new();
    layout.update();
    layout.connect();
    let root = layout.root.clone();
    let weak = Rc::downgrade(&layout.tool);
    *layout.factory.on_operation.borrow_mut() = Some(Rc::new(move |name, item| {
        assert_eq!(name, "hide_dockable");
        root.set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(item)])));
        weak.upgrade().unwrap().set_is_open(true);
    }));
    layout.tool.set_is_open(false);
    assert_eq!(layout.calls(), ["hide_dockable"]);
    assert!(!layout.tool.is_open());
    assert!(layout
        .tool
        .docking_state()
        .contains(DockingWindowState::HIDDEN));
}

#[test]
fn failed_operation_releases_suppression_and_coalescing_guards() {
    let layout = Layout::new();
    layout.update();
    layout.connect();
    *layout.factory.on_operation.borrow_mut() = Some(Rc::new(|_, _| panic!("operation failure")));
    assert!(catch_unwind(AssertUnwindSafe(|| layout.tool.set_is_open(false))).is_err());
    *layout.factory.on_operation.borrow_mut() = None;
    layout
        .factory
        .base
        .get()
        .unwrap()
        .on_docking_window_state_property_changed(
            layout.tool.clone(),
            DockingWindowStateProperty::IsOpen,
        );
    assert_eq!(layout.calls(), ["hide_dockable", "hide_dockable"]);
    assert!(layout.tool.is_open());
}

#[test]
fn selection_request_uses_first_non_splitter_and_includes_split_view_extras() {
    let layout = Layout::new();
    let split = SplitViewDock::new();
    split.set_owner(Some(layout.root.clone()));
    let splitter = ProportionalDockSplitter::new();
    split.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(splitter),
        Some(layout.tool.clone()),
        Some(layout.tool.clone()),
    ])));
    let fallback = Tool::new();
    split.set_pane_dockable(Some(layout.tool.clone()));
    split.set_content_dockable(Some(fallback.clone()));
    layout.tool.set_owner(Some(split.clone()));
    split.set_active_dockable(Some(layout.tool.clone()));
    layout.update();
    layout.connect();
    layout.tool.set_is_selected(false);
    assert!(Rc::ptr_eq(
        &split.active_dockable().unwrap(),
        &(fallback as Rc<dyn IDockable>)
    ));
    layout.tool.set_is_selected(true);
    assert!(Rc::ptr_eq(
        &split.active_dockable().unwrap(),
        &(layout.tool.clone() as Rc<dyn IDockable>)
    ));
}

#[test]
fn active_request_selects_then_dispatches_focus_and_deactivation_uses_fallback() {
    let layout = Layout::new();
    layout.connect();
    layout.tool.set_is_active(true);
    assert!(Rc::ptr_eq(
        &layout.owner.active_dockable().unwrap(),
        &(layout.tool.clone() as Rc<dyn IDockable>)
    ));
    assert!(
        matches!(layout.factory.model_calls.borrow().last(), Some(ModelCall::SetFocused(_, Some(item))) if Rc::ptr_eq(item, &(layout.tool.clone() as Rc<dyn IDockable>)))
    );
    let other = Tool::new();
    layout
        .owner
        .visible_dockables()
        .unwrap()
        .add(Some(other.clone()));
    layout.owner.set_is_active(true);
    layout.root.set_focused_dockable(Some(layout.tool.clone()));
    layout.update();
    layout.tool.set_is_active(false);
    assert!(
        matches!(layout.factory.model_calls.borrow().last(), Some(ModelCall::SetFocused(_, Some(item))) if Rc::ptr_eq(item, &(other as Rc<dyn IDockable>)))
    );
}

#[test]
fn state_request_priority_and_re_resolution_follow_each_completed_operation() {
    let cases = [
        (
            DockingWindowState::DOCKED,
            DockingWindowState::HIDDEN
                | DockingWindowState::DOCUMENT
                | DockingWindowState::FLOATING,
            vec!["hide_dockable"],
        ),
        (
            DockingWindowState::HIDDEN | DockingWindowState::DOCKED,
            DockingWindowState::DOCUMENT
                | DockingWindowState::PINNED
                | DockingWindowState::FLOATING,
            vec!["restore_dockable", "dock_as_document", "float_dockable"],
        ),
        (
            DockingWindowState::DOCKED,
            DockingWindowState::PINNED | DockingWindowState::FLOATING,
            vec!["pin_dockable", "float_dockable"],
        ),
        (
            DockingWindowState::PINNED,
            DockingWindowState::DOCKED,
            vec!["pin_dockable"],
        ),
        (DockingWindowState::DOCKED, DockingWindowState::NONE, vec![]),
    ];
    for (current, requested, expected) in cases {
        let layout = Layout::new();
        if current.contains(DockingWindowState::PINNED) {
            layout
                .root
                .set_left_pinned_dockables(Some(ListHandle::from_plain(vec![Some(
                    layout.tool.clone(),
                )])));
        }
        let state = Rc::new(Cell::new(current));
        let result = state.clone();
        *layout.factory.on_resolve_state.borrow_mut() = Some(Rc::new(move |_| result.get()));
        let next = state.clone();
        *layout.factory.on_operation.borrow_mut() = Some(Rc::new(move |name, _| {
            next.set(match name {
                "restore_dockable" => next.get() & !DockingWindowState::HIDDEN,
                "dock_as_document" => DockingWindowState::DOCUMENT,
                "pin_dockable" => {
                    if next.get().contains(DockingWindowState::PINNED) {
                        DockingWindowState::DOCKED
                    } else {
                        DockingWindowState::PINNED
                    }
                }
                "float_dockable" => next.get() | DockingWindowState::FLOATING,
                "hide_dockable" => next.get() | DockingWindowState::HIDDEN,
                _ => panic!("unexpected operation"),
            })
        }));
        layout.tool.set_docking_state(requested);
        layout
            .factory
            .base
            .get()
            .unwrap()
            .on_docking_window_state_property_changed(
                layout.tool.clone(),
                DockingWindowStateProperty::DockingState,
            );
        assert_eq!(layout.calls(), expected);
        assert_eq!(layout.tool.docking_state(), state.get());
    }
}

#[test]
fn floating_request_does_not_repeat_float_when_previous_operation_already_floated() {
    let layout = Layout::new();
    let state = Rc::new(Cell::new(DockingWindowState::DOCKED));
    let result = state.clone();
    *layout.factory.on_resolve_state.borrow_mut() = Some(Rc::new(move |_| result.get()));
    *layout.factory.on_operation.borrow_mut() = Some(Rc::new(move |name, _| {
        assert_eq!(name, "dock_as_document");
        state.set(DockingWindowState::DOCUMENT | DockingWindowState::FLOATING);
    }));
    layout
        .tool
        .set_docking_state(DockingWindowState::DOCUMENT | DockingWindowState::FLOATING);
    layout
        .factory
        .base
        .get()
        .unwrap()
        .on_docking_window_state_property_changed(
            layout.tool.clone(),
            DockingWindowStateProperty::DockingState,
        );
    assert_eq!(layout.calls(), ["dock_as_document"]);
}

#[test]
fn docked_request_prefers_attached_original_owner_and_appends_after_last_item() {
    let layout = Layout::new();
    let docs = DocumentDock::new();
    docs.set_owner(Some(layout.root.clone()));
    layout
        .root
        .visible_dockables()
        .unwrap()
        .add(Some(docs.clone()));
    layout.tool.set_owner(Some(docs.clone()));
    layout.tool.set_original_owner(Some(layout.owner.clone()));
    let last = Tool::new();
    layout
        .owner
        .visible_dockables()
        .unwrap()
        .add(Some(last.clone()));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .on_docking_window_state_property_changed(
            layout.tool.clone(),
            DockingWindowStateProperty::DockingState,
        );
    assert!(
        matches!(layout.factory.model_calls.borrow().last(), Some(ModelCall::Move(_, target, item, Some(after)))
        if Rc::ptr_eq(target, &(layout.owner.clone() as Rc<dyn IDock>)) && Rc::ptr_eq(item, &(layout.tool.clone() as Rc<dyn IDockable>)) && Rc::ptr_eq(after, &(last as Rc<dyn IDockable>)))
    );
}

#[test]
fn docked_request_ignores_unattached_original_owner_and_root_pinned_dock() {
    let layout = Layout::new();
    let docs = DocumentDock::new();
    docs.set_owner(Some(layout.root.clone()));
    layout.tool.set_owner(Some(docs.clone()));
    let detached = ToolDock::new();
    layout.tool.set_original_owner(Some(detached.clone()));
    layout.root.set_pinned_dock(Some(layout.owner.clone()));
    let target = ToolDock::new();
    layout
        .root
        .visible_dockables()
        .unwrap()
        .add(Some(target.clone()));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .on_docking_window_state_property_changed(
            layout.tool.clone(),
            DockingWindowStateProperty::DockingState,
        );
    assert!(
        matches!(layout.factory.model_calls.borrow().last(), Some(ModelCall::Move(_, actual, _, None)) if Rc::ptr_eq(actual, &(target as Rc<dyn IDock>)))
    );
}

#[test]
fn init_active_dockable_preserves_events_init_focus_and_optional_focus_hook() {
    let layout = Layout::new();
    layout.owner.set_active_dockable(Some(layout.tool.clone()));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .init_active_dockable(Some(layout.tool.clone()), Some(layout.owner.clone()));
    assert!(matches!(
        &layout.factory.model_calls.borrow()[..],
        [
            ModelCall::ActiveChanged(Some(_)),
            ModelCall::InitDockable(_, Some(_)),
            ModelCall::SetFocused(_, Some(_))
        ]
    ));
    assert!(layout.tool.is_open() && layout.tool.is_selected());
    layout.factory.model_calls.borrow_mut().clear();
    layout.factory.suppress_active_focus.set(true);
    layout
        .factory
        .base
        .get()
        .unwrap()
        .init_active_dockable(Some(layout.tool.clone()), Some(layout.owner.clone()));
    assert!(matches!(
        &layout.factory.model_calls.borrow()[..],
        [
            ModelCall::ActiveChanged(Some(_)),
            ModelCall::InitDockable(_, Some(_))
        ]
    ));
    layout.factory.model_calls.borrow_mut().clear();
    layout
        .factory
        .base
        .get()
        .unwrap()
        .init_active_dockable(None, Some(layout.owner.clone()));
    assert!(matches!(
        &layout.factory.model_calls.borrow()[..],
        [ModelCall::ActiveChanged(None)]
    ));
}

#[test]
fn active_and_window_entry_points_dispatch_current_model_and_handle_missing_owners() {
    let layout = Layout::new();
    layout
        .factory
        .base
        .get()
        .unwrap()
        .set_active_dockable(Some(layout.tool.clone()));
    assert_eq!(layout.calls(), ["on_dockable_activated"]);
    layout.tool.set_owner(None);
    layout
        .factory
        .base
        .get()
        .unwrap()
        .set_active_dockable(Some(layout.tool.clone()));
    assert_eq!(layout.calls().len(), 1);
    layout.tool.set_owner(Some(layout.owner.clone()));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .activate_window(Some(layout.tool.clone()));
    assert!(layout.factory.model_calls.borrow().is_empty());
    let window = DockWindow::new();
    layout.root.set_window(Some(window.clone()));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .activate_window(Some(layout.tool.clone()));
    assert!(
        matches!(layout.factory.model_calls.borrow().last(), Some(ModelCall::WindowActivated(Some(actual))) if Rc::ptr_eq(actual, &(window as Rc<dyn IDockWindow>)))
    );
}

#[test]
fn original_owner_attachment_checks_visible_hidden_and_floating_hierarchies() {
    for placement in 0..7 {
        let layout = Layout::new();
        let docs = DocumentDock::new();
        docs.set_owner(Some(layout.root.clone()));
        layout.tool.set_owner(Some(docs.clone()));
        let candidate = ToolDock::new();
        let hidden = ProportionalDock::new();
        let floating = RootDock::new();
        let window = DockWindow::new();
        let original: Rc<dyn IDock> = match placement {
            0 => layout.root.clone(),
            4 => floating.clone(),
            _ => candidate.clone(),
        };
        match placement {
            0 => {}
            1 => layout
                .root
                .visible_dockables()
                .unwrap()
                .add(Some(candidate.clone())),
            2 => layout
                .root
                .set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(candidate.clone())]))),
            3 => {
                hidden.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(
                    candidate.clone(),
                )])));
                layout
                    .root
                    .set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(hidden.clone())])));
            }
            4 | 5 => {
                window.set_layout(Some(floating.clone()));
                floating.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(
                    candidate.clone(),
                )])));
                layout
                    .root
                    .set_windows(Some(ListHandle::from_plain(vec![Some(window.clone())])));
            }
            _ => {}
        }
        layout.tool.set_original_owner(Some(original.clone()));
        layout
            .factory
            .base
            .get()
            .unwrap()
            .on_docking_window_state_property_changed(
                layout.tool.clone(),
                DockingWindowStateProperty::DockingState,
            );
        let expected: Rc<dyn IDock> = if placement == 6 {
            layout.owner.clone()
        } else {
            original
        };
        assert!(
            matches!(layout.factory.model_calls.borrow().last(), Some(ModelCall::Move(_, actual, _, _)) if Rc::ptr_eq(actual, &expected)),
            "placement {placement}"
        );
    }
}

#[test]
fn non_tool_document_cannot_be_converted_to_a_tool_by_a_state_request() {
    let layout = Layout::new();
    let docs = DocumentDock::new();
    docs.set_owner(Some(layout.root.clone()));
    let document = Document::new();
    document.set_owner(Some(docs.clone()));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .on_docking_window_state_property_changed(
            document.clone(),
            DockingWindowStateProperty::DockingState,
        );
    assert!(layout.calls().is_empty());
    assert_eq!(document.docking_state(), DockingWindowState::DOCUMENT);
}

#[test]
fn requests_without_owner_or_focusable_root_reconcile_without_dispatching_focus() {
    let layout = Layout::new();
    layout.root.set_is_focusable_root(false);
    layout.connect();
    layout.tool.set_is_active(true);
    assert!(!layout.tool.is_active());
    assert!(layout.factory.model_calls.borrow().is_empty());
    layout.tool.set_owner(None);
    layout.tool.set_is_selected(true);
    assert!(!layout.tool.is_selected());
    layout.tool.set_is_active(true);
    assert!(!layout.tool.is_active());
    assert!(layout.factory.model_calls.borrow().is_empty());
    assert!(!layout.tool.is_open());
}

#[test]
fn selected_request_clears_selection_if_no_non_splitter_fallback_exists() {
    let layout = Layout::new();
    layout.owner.set_active_dockable(Some(layout.tool.clone()));
    layout.update();
    layout.connect();
    layout.tool.set_is_selected(false);
    assert!(layout.owner.active_dockable().is_none());
    assert!(!layout.tool.is_selected());
}

#[test]
fn coalescing_blocks_reentry_during_post_request_resolution() {
    let layout = Layout::new();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let weak = Rc::downgrade(&layout.factory);
    *layout.factory.on_resolve_state.borrow_mut() = Some(Rc::new(move |item| {
        count.set(count.get() + 1);
        let factory = weak.upgrade().unwrap();
        IDockingWindowStateSync::on_docking_window_state_property_changed(
            factory.base.get().unwrap(),
            item,
            DockingWindowStateProperty::IsOpen,
        );
        DockingWindowState::DOCKED
    }));
    layout
        .factory
        .base
        .get()
        .unwrap()
        .on_docking_window_state_property_changed(
            layout.tool.clone(),
            DockingWindowStateProperty::IsOpen,
        );
    assert_eq!(calls.get(), 1);
    assert_eq!(layout.calls(), ["hide_dockable"]);
}

#[test]
fn sibling_synchronization_uses_live_versioned_enumeration() {
    let layout = Layout::new();
    let items = layout.owner.visible_dockables().unwrap();
    *layout.factory.on_resolve_state.borrow_mut() = Some(Rc::new(move |_| {
        items.add(Some(Tool::new()));
        DockingWindowState::DOCKED
    }));
    assert!(catch_unwind(AssertUnwindSafe(|| {
        layout
            .factory
            .base
            .get()
            .unwrap()
            .init_active_dockable(None, Some(layout.owner.clone()));
    }))
    .is_err());
    *layout.factory.on_resolve_state.borrow_mut() = None;
    layout.connect();
    layout.tool.set_is_open(false);
    assert_eq!(layout.calls(), ["hide_dockable"]);
}

#[test]
fn failed_notification_during_flag_update_releases_suppression_scope() {
    let layout = Layout::new();
    layout.connect();
    let armed = Cell::new(true);
    layout.tool.property_changed().add(Rc::new(move |name| {
        if name == "IsOpen" && armed.replace(false) {
            panic!("observer failure");
        }
    }));
    assert!(catch_unwind(AssertUnwindSafe(|| layout.update())).is_err());
    layout.tool.set_is_open(false);
    assert_eq!(layout.calls(), ["hide_dockable"]);
}

#[test]
fn failed_post_request_resolution_releases_coalescing_scope() {
    let layout = Layout::new();
    *layout.factory.on_resolve_state.borrow_mut() = Some(Rc::new(|_| panic!("resolve failure")));
    assert!(catch_unwind(AssertUnwindSafe(|| {
        layout
            .factory
            .base
            .get()
            .unwrap()
            .on_docking_window_state_property_changed(
                layout.tool.clone(),
                DockingWindowStateProperty::IsOpen,
            );
    }))
    .is_err());
    *layout.factory.on_resolve_state.borrow_mut() = None;
    layout
        .factory
        .base
        .get()
        .unwrap()
        .on_docking_window_state_property_changed(
            layout.tool.clone(),
            DockingWindowStateProperty::IsOpen,
        );
    assert_eq!(layout.calls(), ["hide_dockable", "hide_dockable"]);
}
