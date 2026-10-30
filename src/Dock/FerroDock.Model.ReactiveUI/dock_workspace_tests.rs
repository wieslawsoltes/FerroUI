use crate::controls::*;
use crate::dock_state_tests::Content;
use crate::Factory;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::io::{MemoryStream, StreamIo};
use ferrodock_model::{
    BoxedValue, DockWorkspace, DockWorkspaceManager, DockWorkspaceTrackingOptions,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::io::{Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

// Like the source test serializer, preserves model identity rather than standing
// in for production JSON. The real serializer projects remain pending.
#[derive(Default)]
struct Serializer {
    payload: RefCell<String>,
    last: RefCell<Option<Rc<dyn IDock>>>,
    next: RefCell<Option<Rc<dyn IDock>>>,
    calls: Cell<usize>,
    fail: Cell<bool>,
    throw: Cell<bool>,
    result_error: RefCell<Option<DockSerializationError>>,
    before_serialize: RefCell<Option<Rc<dyn Fn()>>>,
}
impl IDockSerializer for Serializer {
    fn serialize<T: 'static>(&self, value: T) -> SerializationResult<String> {
        let value: Box<dyn Any> = Box::new(value);
        let value = *value
            .downcast::<Rc<dyn IDock>>()
            .unwrap_or_else(|_| panic!("unexpected declared type"));
        let callback = self.before_serialize.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        assert!(!self.throw.get(), "serializer failure");
        if let Some(error) = self.result_error.borrow().clone() {
            return Err(error);
        }
        self.last.replace(Some(value));
        self.calls.set(self.calls.get() + 1);
        let payload = format!("snapshot-{}", self.calls.get());
        self.payload.replace(payload.clone());
        Ok(payload)
    }
    fn deserialize<T: 'static>(&self, text: Option<&str>) -> SerializationResult<Option<T>> {
        let text = text.expect("source fixture text");
        assert!(!self.throw.get(), "serializer failure");
        if let Some(error) = self.result_error.borrow().clone() {
            return Err(error);
        }
        if self.fail.get() || text != self.payload.borrow().as_str() {
            return Ok(None);
        }
        let value = self
            .next
            .borrow()
            .clone()
            .or_else(|| self.last.borrow().clone());
        let Some(value) = value else {
            return Ok(None);
        };
        let value: Box<dyn Any> = Box::new(value);
        Ok(value.downcast::<T>().ok().map(|value| *value))
    }
    fn load<T: 'static>(
        &self,
        stream: Option<Rc<dyn DockStream>>,
    ) -> SerializationResult<Option<T>> {
        let mut text = String::new();
        StreamIo::new(stream.expect("source stream"))
            .read_to_string(&mut text)
            .unwrap();
        self.deserialize(Some(&text))
    }
    fn save<T: 'static>(
        &self,
        stream: Option<Rc<dyn DockStream>>,
        value: T,
    ) -> SerializationResult<()> {
        let text = self.serialize(value)?;
        StreamIo::new(stream.expect("source stream"))
            .write_all(text.as_bytes())
            .map_err(DockSerializationError::new)
    }
}
type Scene = (
    Rc<Serializer>,
    Rc<DockWorkspaceManager<Serializer>>,
    Rc<Factory>,
    Rc<dyn IRootDock>,
);
fn scene() -> Scene {
    let serializer = Rc::new(Serializer::default());
    let manager = DockWorkspaceManager::new(serializer.clone());
    let factory = Factory::new();
    let root = factory.create_root_dock();
    root.set_visible_dockables(Some(ListHandle::new()));
    factory.init_layout(root.clone());
    (serializer, manager, factory, root)
}

#[test]
fn capture_restore_state_and_generic_serializer_stream_contract() {
    let (serializer, manager, _, root) = scene();
    let value: BoxedValue = Rc::new("cached content".to_owned());
    let content = Content::new("tool", true, false, Some(value.clone()));
    root.visible_dockables().unwrap().add(Some(content.clone()));
    let workspace = manager.capture_default("A", root.clone()).unwrap();
    assert!(workspace.state().is_some());
    assert!(Rc::ptr_eq(&manager.active_workspace().unwrap(), &workspace));
    assert_eq!(workspace.name().as_deref(), Some("A"));
    IToolContent::set_content(content.as_ref(), None);
    let restored = manager.restore(workspace.clone()).unwrap().unwrap();
    assert!(Rc::ptr_eq(&restored, &(root.clone() as Rc<dyn IDock>)));
    assert!(Rc::ptr_eq(
        &IToolContent::content(content.as_ref()).unwrap(),
        &value
    ));
    assert!(Rc::ptr_eq(
        &manager.try_restore("a").unwrap().unwrap(),
        &restored
    ));
    assert!(manager.try_restore("missing").unwrap().is_none());
    assert!(manager.try_restore(" \t").unwrap().is_none());
    let stream = MemoryStream::new();
    serializer
        .save(Some(stream.clone()), root as Rc<dyn IDock>)
        .unwrap();
    stream.set_position(0).unwrap();
    let loaded = serializer
        .load::<Rc<dyn IDock>>(Some(stream.clone()))
        .unwrap()
        .unwrap();
    assert!(Rc::ptr_eq(&loaded, &restored));
    assert_eq!(stream.position().unwrap(), stream.to_array().len() as u64);
    assert!(serializer
        .deserialize::<String>(Some(&serializer.payload.borrow()))
        .unwrap()
        .is_none());
}

#[test]
fn capture_updates_identity_metadata_and_case_insensitive_ids_without_trimming() {
    let (_, manager, _, root) = scene();
    let workspace = manager
        .capture(
            " Space ",
            root.clone(),
            true,
            Some("name".into()),
            Some("description".into()),
        )
        .unwrap();
    let state = workspace.state().unwrap();
    let updated = manager
        .capture(
            " sPACE ",
            root.clone(),
            false,
            Some(" \t".into()),
            Some("".into()),
        )
        .unwrap();
    assert!(Rc::ptr_eq(&workspace, &updated));
    assert_eq!(updated.id(), " Space ");
    assert_eq!(updated.name().as_deref(), Some("name"));
    assert_eq!(updated.description().as_deref(), Some("description"));
    assert!(updated.state().is_none());
    assert_eq!(Rc::strong_count(&state), 1);
    assert_eq!(updated.layout(), "snapshot-2");
    assert!(manager.get("Space").is_none());
    manager
        .capture(
            " SPACE ",
            root.clone(),
            true,
            Some("new".into()),
            Some("changed".into()),
        )
        .unwrap();
    assert_eq!(workspace.name().as_deref(), Some("new"));
    assert_eq!(workspace.description().as_deref(), Some("changed"));
    let other = manager
        .capture("B", root, false, Some(" \n".into()), Some("".into()))
        .unwrap();
    assert_eq!(other.name().as_deref(), Some("B"));
    assert_eq!(other.description().as_deref(), Some(""));
    assert_eq!(manager.workspaces().count(), 2);
}

#[test]
fn all_subscribed_dockable_and_window_events_mark_dirty_and_other_events_do_not() {
    let (_, manager, factory, root) = scene();
    manager.track_factory(factory.clone(), None);
    let document = factory.create_document();
    document.set_owner(Some(root.clone()));
    let window = factory.create_dock_window();
    window.set_layout(Some(root.clone()));
    let factory: Rc<dyn IFactory> = factory;
    let dockable: Rc<dyn IDockable> = document;
    let events: Vec<Box<dyn Fn()>> = vec![
        Box::new(|| factory.on_dockable_added(Some(dockable.clone()))),
        Box::new(|| factory.on_dockable_removed(Some(dockable.clone()))),
        Box::new(|| factory.on_dockable_closed(Some(dockable.clone()))),
        Box::new(|| factory.on_dockable_moved(Some(dockable.clone()))),
        Box::new(|| factory.on_dockable_docked(Some(dockable.clone()), DockOperation::Fill)),
        Box::new(|| factory.on_dockable_undocked(Some(dockable.clone()), DockOperation::Fill)),
        Box::new(|| factory.on_dockable_swapped(Some(dockable.clone()))),
        Box::new(|| factory.on_dockable_pinned(Some(dockable.clone()))),
        Box::new(|| factory.on_dockable_unpinned(Some(dockable.clone()))),
        Box::new(|| factory.on_dockable_hidden(Some(dockable.clone()))),
        Box::new(|| factory.on_dockable_restored(Some(dockable.clone()))),
        Box::new(|| factory.on_window_opened(Some(window.clone()))),
        Box::new(|| factory.on_window_closed(Some(window.clone()))),
        Box::new(|| factory.on_window_added(Some(window.clone()))),
        Box::new(|| factory.on_window_removed(Some(window.clone()))),
        Box::new(|| factory.on_window_move_drag_end(Some(window.clone()))),
    ];
    for event in events {
        assert!(!manager.is_dirty());
        event();
        assert!(manager.is_dirty());
        manager.mark_clean();
    }
    factory.on_active_dockable_changed(Some(dockable.clone()));
    factory.on_dockable_init(Some(dockable));
    factory.on_window_move_drag(Some(window));
    assert!(!manager.is_dirty());
    let workspace = manager.capture("active", root, false, None, None).unwrap();
    factory.on_dockable_moved(None);
    assert!(workspace.is_dirty());
    assert!(manager.is_dirty());
    manager
        .capture("active", factory.create_root_dock(), false, None, None)
        .unwrap();
    assert!(!workspace.is_dirty());
    assert!(!manager.is_dirty());
}

#[test]
fn track_layout_root_scope_filters_owners_missing_roots_windows_and_null_events() {
    let (_, manager, factory, root) = scene();
    let other = factory.create_root_dock();
    factory.init_layout(other.clone());
    let pane = factory.create_document_dock();
    pane.set_owner(Some(root.clone()));
    pane.set_factory(Some(factory.clone()));
    manager.track_layout(pane, None);
    let doc = factory.create_document();
    doc.set_owner(Some(other.clone()));
    factory.on_dockable_moved(Some(doc.clone()));
    assert!(!manager.is_dirty());
    doc.set_owner(None);
    factory.on_dockable_moved(Some(doc.clone()));
    assert!(!manager.is_dirty());
    doc.set_owner(Some(root.clone()));
    factory.on_dockable_moved(Some(doc.clone()));
    assert!(manager.is_dirty());
    manager.mark_clean();
    let window = factory.create_dock_window();
    window.set_owner(Some(doc.clone()));
    window.set_layout(Some(other));
    factory.on_window_opened(Some(window.clone()));
    assert!(!manager.is_dirty());
    window.set_layout(None);
    factory.on_window_opened(Some(window.clone()));
    assert!(manager.is_dirty());
    manager.mark_clean();
    window.set_owner(None);
    factory.on_window_opened(Some(window.clone()));
    assert!(!manager.is_dirty());
    window.set_layout(Some(root));
    factory.on_window_opened(Some(window));
    assert!(manager.is_dirty());
    manager.mark_clean();
    // Null contexts skip the source root check, and are still passed to the filter.
    factory.on_dockable_moved(None);
    assert!(manager.is_dirty());
    manager.mark_clean();
    factory.on_window_opened(None);
    assert!(manager.is_dirty());
}

#[test]
fn shared_tracking_options_are_live_and_filter_receives_window_owner() {
    let (_, manager, factory, root) = scene();
    let options = DockWorkspaceTrackingOptions::new();
    assert!(options.track_window_moves());
    assert!(options.dockable_filter().is_none());
    options.set_track_window_moves(false);
    options.set_dockable_filter(Some(Rc::new(|_| false)));
    manager.track_factory(factory.clone(), Some(options.clone()));
    assert!(Rc::ptr_eq(&options, &manager.tracking_options()));
    let document = factory.create_document();
    factory.on_dockable_moved(Some(document.clone()));
    assert!(!manager.is_dirty());
    let window = factory.create_dock_window();
    window.set_layout(Some(root));
    window.set_owner(Some(document.clone()));
    let inputs = Rc::new(RefCell::new(Vec::new()));
    let seen = inputs.clone();
    options.set_dockable_filter(Some(Rc::new(move |dockable| {
        seen.borrow_mut().push(dockable);
        true
    })));
    factory.on_window_move_drag_end(Some(window.clone()));
    assert!(!manager.is_dirty());
    assert!(inputs.borrow().is_empty());
    factory.on_window_opened(Some(window.clone()));
    assert!(manager.is_dirty());
    manager.mark_clean();
    assert!(Rc::ptr_eq(
        inputs.borrow()[0].as_ref().unwrap(),
        &(document as Rc<dyn IDockable>)
    ));
    options.set_track_window_moves(true);
    factory.on_window_move_drag_end(Some(window));
    assert!(manager.is_dirty());
    manager.mark_clean();
    factory.on_window_opened(None);
    assert!(inputs.borrow().last().unwrap().is_none());
}

#[test]
fn tracking_same_factory_clears_scope_without_duplicate_handlers_and_switching_unsubscribes() {
    let (_, manager, factory, root) = scene();
    let options = DockWorkspaceTrackingOptions::new();
    let hits = Rc::new(Cell::new(0));
    let seen = hits.clone();
    options.set_dockable_filter(Some(Rc::new(move |_| {
        seen.set(seen.get() + 1);
        true
    })));
    manager.track_layout(root, None);
    manager.track_factory(factory.clone(), Some(options.clone()));
    let doc = factory.create_document();
    // Keep the owner alive: native parent references follow the port guide's weak ownership.
    let owner = factory.create_root_dock();
    doc.set_owner(Some(owner.clone()));
    factory.on_dockable_moved(Some(doc.clone()));
    assert!(manager.is_dirty());
    assert_eq!(hits.get(), 1);
    let next = Factory::new();
    manager.track_factory(next.clone(), None);
    manager.mark_clean();
    factory.on_dockable_moved(Some(doc.clone()));
    assert!(!manager.is_dirty());
    assert_eq!(hits.get(), 1);
    next.on_dockable_moved(Some(doc));
    assert!(manager.is_dirty());
    manager.stop_tracking();
    assert!(!manager.is_tracking());
    assert!(!Rc::ptr_eq(&manager.tracking_options(), &options));
    assert!(manager.is_dirty());
    manager.mark_clean();
    next.on_dockable_moved(None);
    assert!(!manager.is_dirty());
    let idle_options = manager.tracking_options();
    idle_options.set_track_window_moves(false);
    manager.stop_tracking();
    assert!(Rc::ptr_eq(&idle_options, &manager.tracking_options()));
}

#[test]
fn dirty_events_are_transitions_and_support_reentrant_clean_and_dispose() {
    let (_, manager, factory, root) = scene();
    manager.track_factory(factory.clone(), None);
    let workspace = manager.capture("Active", root, false, None, None).unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let weak = Rc::downgrade(&manager);
    let subscription = manager.workspace_dirty_changed(Rc::new(move |args| {
        log.borrow_mut().push(args.clone());
        if args.is_dirty() {
            weak.upgrade().unwrap().mark_clean();
        }
    }));
    factory.on_dockable_moved(None);
    assert_eq!(events.borrow().len(), 2);
    assert!(!manager.is_dirty());
    assert!(events.borrow()[0].is_dirty());
    assert!(!events.borrow()[1].is_dirty());
    assert!(Rc::ptr_eq(
        &events.borrow()[0].workspace().unwrap(),
        &workspace
    ));
    manager.mark_clean();
    assert_eq!(events.borrow().len(), 2);
    subscription.dispose();
    subscription.dispose();
    factory.on_dockable_moved(None);
    assert!(workspace.is_dirty());
    assert_eq!(events.borrow().len(), 2);
}

#[test]
fn remove_preserves_source_case_sensitive_active_check_and_collection_free_slot_order() {
    let (_, manager, factory, root) = scene();
    let view = manager.workspaces();
    assert!(view.is_empty());
    let a = manager
        .capture("A", root.clone(), false, None, None)
        .unwrap();
    manager
        .capture("B", root.clone(), false, None, None)
        .unwrap();
    manager
        .capture("C", root.clone(), false, None, None)
        .unwrap();
    assert!(manager.remove("B"));
    manager
        .capture("D", root.clone(), false, None, None)
        .unwrap();
    let ids: Vec<_> = view.iter().map(|w| w.id().to_owned()).collect();
    assert_eq!(ids, ["A", "D", "C"]);
    manager.capture("a", root, false, None, None).unwrap();
    manager.track_factory(factory.clone(), None);
    factory.on_dockable_moved(None);
    assert!(manager.remove("a"));
    assert!(Rc::ptr_eq(&manager.active_workspace().unwrap(), &a));
    assert!(manager.is_dirty());
    assert!(!manager.remove("A"));
    assert!(!manager.remove("\t"));
    manager.clear();
    assert!(view.is_empty());
    assert!(manager.active_workspace().is_none());
    assert!(!manager.is_dirty());
}

#[test]
fn workspace_collection_is_live_and_enumerator_preserves_source_mutation_rules() {
    let (_, manager, _, root) = scene();
    let a = manager
        .capture("A", root.clone(), false, None, None)
        .unwrap();
    manager
        .capture("B", root.clone(), false, None, None)
        .unwrap();
    manager
        .capture("C", root.clone(), false, None, None)
        .unwrap();
    let mut iterator = manager.workspaces().iter();
    assert!(Rc::ptr_eq(&iterator.next().unwrap(), &a));
    manager.remove("A");
    manager.remove("B");
    manager
        .capture("C", root.clone(), false, Some("updated".into()), None)
        .unwrap();
    assert_eq!(iterator.next().unwrap().name().as_deref(), Some("updated"));
    assert!(iterator.next().is_none());
    manager
        .capture("D", root.clone(), false, None, None)
        .unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| iterator.next())).is_err());
    let mut iterator = manager.workspaces().iter();
    manager.clear();
    assert!(iterator.next().is_none());
    let mut empty = manager.workspaces().iter();
    manager.capture("E", root, false, None, None).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| empty.next())).is_err());
}

#[test]
fn failed_restores_and_serializer_panics_preserve_current_active_dirty_and_payload() {
    let (serializer, manager, factory, root) = scene();
    let workspace = manager
        .capture("A", root.clone(), false, None, None)
        .unwrap();
    manager.track_factory(factory.clone(), None);
    factory.on_dockable_moved(None);
    serializer.fail.set(true);
    assert!(manager.restore(workspace.clone()).unwrap().is_none());
    assert!(manager.is_dirty());
    assert!(workspace.is_dirty());
    assert!(Rc::ptr_eq(&manager.active_workspace().unwrap(), &workspace));
    serializer.throw.set(true);
    assert!(catch_unwind(AssertUnwindSafe(|| manager
        .capture("A", root.clone(), true, None, None)
        .unwrap()))
    .is_err());
    assert_eq!(workspace.layout(), "snapshot-1");
    assert!(workspace.is_dirty());
    assert!(catch_unwind(AssertUnwindSafe(|| manager.restore(workspace).unwrap())).is_err());
    assert_eq!(manager.workspaces().count(), 1);
}

#[test]
fn default_workspace_metadata_and_validation_reject_only_dotnet_whitespace_ids() {
    let state = ferrodock_model::DockState::new();
    let workspace = DockWorkspace::new("id".into(), "".into(), Some(state.clone()));
    assert_eq!(workspace.id(), "id");
    assert_eq!(workspace.layout(), "");
    assert!(workspace.name().is_none());
    assert!(workspace.description().is_none());
    assert!(!workspace.is_dirty());
    assert!(Rc::ptr_eq(&workspace.state().unwrap(), &state));
    workspace.set_name(Some("".into()));
    workspace.set_description(Some(" \t".into()));
    assert_eq!(workspace.name().as_deref(), Some(""));
    assert_eq!(workspace.description().as_deref(), Some(" \t"));
    let (serializer, manager, factory, root) = scene();
    for id in [
        "",
        " \t\n\r",
        "\u{0085}\u{00A0}\u{1680}\u{2007}\u{2028}\u{202F}\u{205F}\u{3000}",
    ] {
        assert!(catch_unwind(AssertUnwindSafe(|| manager
            .capture(id, root.clone(), false, None, None)
            .unwrap()))
        .is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| DockWorkspace::new(
            id.into(),
            "".into(),
            None
        )))
        .is_err());
        assert!(manager.get(id).is_none());
    }
    assert_eq!(serializer.calls.get(), 0);
    for id in ["\0", "\u{200B}", "\u{FEFF}", " id "] {
        manager
            .capture(id, root.clone(), false, None, None)
            .unwrap();
    }
    manager.track_factory(factory.clone(), None);
    let invalid = RootDock::new();
    assert!(catch_unwind(AssertUnwindSafe(|| manager.track_layout(invalid, None))).is_err());
    factory.on_dockable_moved(None);
    assert!(manager.is_dirty());
    assert!(manager.is_tracking());
}

#[test]
fn capture_saves_content_before_serializing_and_restore_accepts_unregistered_workspace() {
    let (serializer, manager, _, root) = scene();
    let original: BoxedValue = Rc::new("original");
    let content = Content::new("tool", true, false, Some(original.clone()));
    root.visible_dockables().unwrap().add(Some(content.clone()));
    let probe = content.clone();
    serializer.before_serialize.replace(Some(Rc::new(move || {
        IToolContent::set_content(probe.as_ref(), None)
    })));
    let workspace = manager
        .capture("source", root.clone(), true, None, None)
        .unwrap();
    assert!(IToolContent::content(content.as_ref()).is_none());
    manager.restore(workspace.clone()).unwrap().unwrap();
    assert!(Rc::ptr_eq(
        &IToolContent::content(content.as_ref()).unwrap(),
        &original
    ));
    let external = DockWorkspace::new("external".into(), workspace.layout(), workspace.state());
    let restored_root = RootDock::new();
    let restored_content = Content::new("tool", true, false, None);
    restored_root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        restored_content.clone() as Rc<dyn IDockable>,
    )])));
    serializer.next.replace(Some(restored_root.clone()));
    let restored = manager.restore(external.clone()).unwrap().unwrap();
    assert!(Rc::ptr_eq(&restored, &(restored_root as Rc<dyn IDock>)));
    assert!(Rc::ptr_eq(
        &IToolContent::content(restored_content.as_ref()).unwrap(),
        &original
    ));
    assert!(Rc::ptr_eq(&manager.active_workspace().unwrap(), &external));
    assert!(manager.get("external").is_none());
}

#[test]
fn reentrant_tracking_filter_uses_current_active_workspace_even_after_stopping_tracking() {
    let (_, manager, factory, root) = scene();
    let workspace = manager.capture("A", root, false, None, None).unwrap();
    let options = DockWorkspaceTrackingOptions::new();
    let weak = Rc::downgrade(&manager);
    options.set_dockable_filter(Some(Rc::new(move |_| {
        let manager = weak.upgrade().unwrap();
        manager.stop_tracking();
        manager.clear();
        true
    })));
    manager.track_factory(factory.clone(), Some(options));
    factory.on_dockable_moved(None);
    assert!(!workspace.is_dirty());
    assert!(manager.active_workspace().is_none());
    assert!(manager.is_dirty());
    assert!(!manager.is_tracking());
}

#[test]
fn dropping_manager_releases_tracking_and_snapshot_content_without_ownership_cycles() {
    let (serializer, manager, factory, root) = scene();
    manager.track_layout(root.clone(), None);
    let workspace = manager.capture("A", root, true, None, None).unwrap();
    let weak = Rc::downgrade(&manager);
    let weak_workspace = Rc::downgrade(&workspace);
    drop(workspace);
    drop(manager);
    assert!(weak.upgrade().is_none());
    assert!(weak_workspace.upgrade().is_none());
    factory.on_dockable_moved(None); // no retained callback target
    assert_eq!(Rc::strong_count(&serializer), 1);
}

#[test]
fn removing_exact_active_id_resets_layout_but_does_not_clean_the_removed_snapshot() {
    let (_, manager, factory, root) = scene();
    let workspace = manager.capture("Active", root, false, None, None).unwrap();
    manager.track_factory(factory.clone(), None);
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let _subscription =
        manager.workspace_dirty_changed(Rc::new(move |args| seen.borrow_mut().push(args.clone())));
    factory.on_dockable_moved(None);
    assert!(workspace.is_dirty());
    assert!(manager.remove("Active"));
    assert!(!manager.is_dirty());
    assert!(manager.active_workspace().is_none());
    assert!(workspace.is_dirty());
    let events = events.borrow();
    assert_eq!(events.len(), 2);
    assert!(events[0].workspace().is_some());
    assert!(events[0].is_dirty());
    assert!(events[1].workspace().is_none());
    assert!(!events[1].is_dirty());
}

#[test]
fn serializer_result_errors_preserve_original_cause_and_workspace_state() {
    #[derive(Debug)]
    struct Failure;
    impl std::fmt::Display for Failure {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("source serializer failure")
        }
    }
    impl std::error::Error for Failure {}
    let (serializer, manager, factory, root) = scene();
    let workspace = manager
        .capture("A", root.clone(), false, None, None)
        .unwrap();
    manager.track_factory(factory.clone(), None);
    factory.on_dockable_moved(None);
    let failure = DockSerializationError::new(Failure);
    serializer.result_error.replace(Some(failure.clone()));
    let capture = manager.capture("A", root, false, None, None).err().unwrap();
    assert!(capture.ptr_eq(&failure));
    let restore = manager.restore(workspace.clone()).err().unwrap();
    assert!(restore.ptr_eq(&failure));
    let lookup = manager.try_restore("A").err().unwrap();
    assert!(lookup.ptr_eq(&failure));
    assert!(capture.cause().downcast_ref::<Failure>().is_some());
    assert_eq!(capture.to_string(), "source serializer failure");
    assert!(std::error::Error::source(&capture).is_some());
    assert!(!format!("{capture:?}").is_empty());
    assert_eq!(workspace.layout(), "snapshot-1");
    assert!(manager.is_dirty() && workspace.is_dirty());
    assert!(Rc::ptr_eq(&manager.active_workspace().unwrap(), &workspace));
    assert!(manager.try_restore("missing").unwrap().is_none());
}
