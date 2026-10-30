//! Original nullable preview lifecycle and live/cached preview reentry.
use crate::{controls::*, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{
    collections::ListHandle,
    controls::{IRootDock, IToolDock},
    core::*,
    FactoryBase, FactoryBaseImpl,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
struct PreviewFactory {
    component: Factory,
    root: RefCell<Option<Rc<dyn IRootDock>>>,
    sequence: RefCell<Vec<Option<Rc<dyn IRootDock>>>>,
    trace: RefCell<Vec<String>>,
    init_mode: Cell<u8>,
    restore: RefCell<Option<Rc<dyn IToolDock>>>,
    last_init: RefCell<Option<Rc<dyn IDockable>>>,
}
fn name(item: &Option<Rc<dyn IDockable>>) -> String {
    item.as_ref().map_or_else(|| "null".into(), |i| i.id())
}
impl PreviewFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            root: Default::default(),
            sequence: Default::default(),
            trace: Default::default(),
            init_mode: Cell::new(0),
            restore: Default::default(),
            last_init: Default::default(),
        })
    }
}
impl FactoryBaseImpl for PreviewFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for PreviewFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        _: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        this.trace
            .borrow_mut()
            .push(format!("find:{}", name(&item)));
        if this.sequence.borrow().is_empty() {
            this.root.borrow().clone()
        } else {
            this.sequence.borrow_mut().remove(0)
        }
    }
    fn init_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        this.trace.borrow_mut().push(format!(
            "init:{}:{}",
            if item.is_none() { "null" } else { "some" },
            name(&owner)
        ));
        this.last_init.replace(item);
        if this.init_mode.get() != 0 {
            let root = owner.unwrap().as_root_dock().unwrap();
            root.set_pinned_dock(if this.init_mode.get() == 1 {
                None
            } else {
                this.restore.borrow().clone()
            });
        }
    }
}
impl_factory!(PreviewFactory);
fn failure(action: impl FnOnce()) -> String {
    let e = catch_unwind(AssertUnwindSafe(action)).expect_err("source failure");
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
fn install(
    f: &Rc<PreviewFactory>,
    root: &Rc<RootDock>,
    preview: &Rc<ToolDock>,
    items: Vec<Option<Rc<dyn IDockable>>>,
) -> ListHandle<Option<Rc<dyn IDockable>>> {
    let list = f.create_list(items);
    preview.set_visible_dockables(Some(list.clone()));
    root.set_pinned_dock(Some(preview.clone()));
    f.root.replace(Some(root.clone()));
    list
}
#[test]
fn nullable_preview_entries_preserve_root_lookup_noop_and_first_default_dereferences() {
    let f = PreviewFactory::new();
    f.preview_pinned_dockable(None);
    f.toggle_preview_pinned_dockable(None);
    assert_eq!(&*f.trace.borrow(), &["find:null", "find:null"]);
    assert!(failure(|| f.hide_previewing_dockables(None)).contains("hide root.PinnedDock"));
    assert!(failure(|| Factory::new().preview_pinned_dockable(None)).contains("dockable.Owner"));
    assert!(
        failure(|| Factory::new().toggle_preview_pinned_dockable(None)).contains("dockable.Owner")
    );
    let root = RootDock::new();
    f.hide_previewing_dockables(Some(root.clone()));
    let preview = ToolDock::new();
    root.set_pinned_dock(Some(preview.clone()));
    f.hide_previewing_dockables(Some(root.clone()));
    assert!(Rc::ptr_eq(
        &root.pinned_dock().unwrap(),
        &(preview as Rc<dyn IToolDock>)
    ));
}
#[test]
fn null_incoming_preview_finishes_existing_cleanup_before_owner_failure() {
    let f = PreviewFactory::new();
    let root = RootDock::new();
    let preview = ToolDock::new();
    let item = Tool::new();
    item.set_keep_pinned_dockable_visible(true);
    item.set_owner(Some(preview.clone()));
    preview.set_active_dockable(Some(item.clone()));
    let list = install(&f, &root, &preview, vec![Some(item.clone())]);
    assert!(failure(|| f.preview_pinned_dockable(None)).contains("preview dockable.Owner"));
    assert!(root.pinned_dock().is_none());
    assert_eq!(list.count(), 0);
    assert!(list.ptr_eq(&preview.visible_dockables().unwrap()));
    assert!(preview.active_dockable().is_none());
    assert!(Rc::ptr_eq(
        &item.owner().unwrap(),
        &(root as Rc<dyn IDockable>)
    ));
    assert!(item.original_owner().is_none());
    assert_eq!(&*f.trace.borrow(), &["find:null"]);
}
#[test]
fn toggle_null_uses_fixed_preview_relookup_or_original_null_snapshot_cleanup() {
    let f = PreviewFactory::new();
    let root = RootDock::new();
    f.sequence.replace(vec![Some(root.clone()), None]);
    f.toggle_preview_pinned_dockable(None);
    assert_eq!(&*f.trace.borrow(), &["find:null", "find:null"]);
    f.trace.borrow_mut().clear();
    let preview = ToolDock::new();
    let list = install(&f, &root, &preview, vec![None]);
    assert!(
        failure(|| f.toggle_preview_pinned_dockable(None)).contains("hide dockable.OriginalOwner")
    );
    assert_eq!(list.count(), 1);
    assert!(list.get(0).is_none());
    assert_eq!(&*f.trace.borrow(), &["find:null"]);
}
#[test]
fn hide_preserves_keep_visible_check_and_partial_snapshot_mutation_before_null_failure() {
    for keep in [true, false] {
        let f = PreviewFactory::new();
        let root = RootDock::new();
        let preview = ToolDock::new();
        let item = Tool::new();
        item.set_keep_pinned_dockable_visible(keep);
        item.set_owner(Some(preview.clone()));
        let later = Tool::new();
        preview.set_active_dockable(Some(item.clone()));
        let list = install(
            &f,
            &root,
            &preview,
            vec![Some(item.clone()), None, Some(later)],
        );
        assert!(failure(|| f.hide_previewing_dockables(Some(root.clone())))
            .contains("hide dockable.KeepPinnedDockableVisible"));
        assert_eq!(list.count(), if keep { 3 } else { 2 });
        assert!(Rc::ptr_eq(
            &preview.active_dockable().unwrap(),
            &(item.clone() as Rc<dyn IDockable>)
        ));
        let owner: Rc<dyn IDockable> = if keep { preview.clone() } else { root.clone() };
        assert!(Rc::ptr_eq(&item.owner().unwrap(), &owner));
        assert!(Rc::ptr_eq(
            &root.pinned_dock().unwrap(),
            &(preview as Rc<dyn IToolDock>)
        ));
    }
}
#[test]
fn hide_removal_rereads_live_preview_but_final_selection_uses_cached_preview_and_list() {
    for clear in [true, false] {
        let f = PreviewFactory::new();
        let root = RootDock::new();
        let old = ToolDock::new();
        let replacement = ToolDock::new();
        let a = Tool::new();
        let b = Tool::new();
        a.set_owner(Some(old.clone()));
        b.set_owner(Some(old.clone()));
        old.set_active_dockable(Some(a.clone()));
        let old_list = install(&f, &root, &old, vec![Some(a.clone()), Some(b.clone())]);
        let new_list = f.create_list(vec![
            Some(a.clone() as Rc<dyn IDockable>),
            Some(b.clone() as Rc<dyn IDockable>),
        ]);
        replacement.set_visible_dockables(Some(new_list.clone()));
        let weak_root = Rc::downgrade(&root);
        let weak_new = Rc::downgrade(&replacement);
        let once = Cell::new(false);
        a.property_changed().add(Rc::new(move |property| {
            if property == "Owner" && !once.replace(true) {
                weak_root.upgrade().unwrap().set_pinned_dock(if clear {
                    None
                } else {
                    Some(weak_new.upgrade().unwrap())
                });
            }
        }));
        if clear {
            assert!(failure(|| f.hide_previewing_dockables(Some(root.clone())))
                .contains("remove_visible_dockable dock.VisibleDockables"));
            assert!(root.pinned_dock().is_none());
            assert_eq!(new_list.count(), 2);
            assert!(Rc::ptr_eq(
                &b.owner().unwrap(),
                &(old.clone() as Rc<dyn IDockable>)
            ));
        } else {
            f.hide_previewing_dockables(Some(root.clone()));
            assert!(Rc::ptr_eq(
                &root.pinned_dock().unwrap(),
                &(replacement as Rc<dyn IToolDock>)
            ));
            assert_eq!(new_list.count(), 0);
            assert!(Rc::ptr_eq(
                &b.owner().unwrap(),
                &(root.clone() as Rc<dyn IDockable>)
            ));
        }
        assert_eq!(old_list.count(), 2);
        assert!(old_list.ptr_eq(&old.visible_dockables().unwrap()));
        assert!(Rc::ptr_eq(
            &old.active_dockable().unwrap(),
            &(a.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &a.owner().unwrap(),
            &(root as Rc<dyn IDockable>)
        ));
    }
}
#[test]
fn preview_list_reentry_passes_actual_null_to_init_and_allows_source_recovery() {
    for behavior in [0, 1, 2] {
        let f = PreviewFactory::new();
        let root = RootDock::new();
        root.set_id("root".into());
        let owner = ToolDock::new();
        owner.set_owner(Some(root.clone()));
        let item = Tool::new();
        item.set_id("item".into());
        item.set_owner(Some(owner.clone()));
        let old = ToolDock::new();
        let restored = ToolDock::new();
        root.set_pinned_dock(Some(old.clone()));
        f.root.replace(Some(root.clone()));
        f.restore.replace(Some(restored.clone()));
        f.init_mode.set(if behavior == 2 { 2 } else { 0 });
        let weak = Rc::downgrade(&root);
        old.property_changed().add(Rc::new(move |property| {
            if property == "VisibleDockables" {
                weak.upgrade().unwrap().set_pinned_dock(None);
            }
        }));
        if behavior == 1 {
            assert!(
                failure(|| Factory::new().preview_pinned_dockable(Some(item.clone())))
                    .contains("dockable.Context")
            );
            assert!(f.trace.borrow().is_empty());
        } else if behavior == 0 {
            assert!(failure(|| f.preview_pinned_dockable(Some(item.clone())))
                .contains("preview.ActiveDockable"));
            assert_eq!(&*f.trace.borrow(), &["find:item", "init:null:root"]);
        } else {
            f.preview_pinned_dockable(Some(item.clone()));
            assert_eq!(&*f.trace.borrow(), &["find:item", "init:null:root"]);
            assert!(Rc::ptr_eq(
                &root.pinned_dock().unwrap(),
                &(restored.clone() as Rc<dyn IToolDock>)
            ));
            assert!(Rc::ptr_eq(
                &restored.active_dockable().unwrap(),
                &(item.clone() as Rc<dyn IDockable>)
            ));
        }
        if behavior != 2 {
            assert!(root.pinned_dock().is_none());
        }
        let list = old.visible_dockables().unwrap();
        assert_eq!(list.count(), 1);
        assert!(Rc::ptr_eq(
            &list.get(0).unwrap(),
            &(item.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &item.original_owner().unwrap(),
            &(owner as Rc<dyn IDockable>)
        ));
        assert!(old.active_dockable().is_none());
    }
}
#[test]
fn preview_nonnull_init_can_clear_current_preview_before_original_active_access() {
    let f = PreviewFactory::new();
    let root = RootDock::new();
    root.set_id("root".into());
    f.root.replace(Some(root.clone()));
    f.init_mode.set(1);
    let item = Tool::new();
    item.set_id("item".into());
    assert!(failure(|| f.preview_pinned_dockable(Some(item.clone())))
        .contains("preview.ActiveDockable"));
    let saved = f
        .last_init
        .borrow()
        .clone()
        .unwrap()
        .as_tool_dock()
        .unwrap();
    assert!(root.pinned_dock().is_none());
    assert_eq!(saved.visible_dockables().unwrap().count(), 1);
    assert!(Rc::ptr_eq(
        &saved.visible_dockables().unwrap().get(0).unwrap(),
        &(item as Rc<dyn IDockable>)
    ));
    assert!(saved.active_dockable().is_none());
    assert_eq!(&*f.trace.borrow(), &["find:item", "init:some:root"]);
}
#[test]
fn keep_visible_preview_reselection_keeps_cached_list_and_first_nonsplitter_identity() {
    let f = PreviewFactory::new();
    let root = RootDock::new();
    let preview = ToolDock::new();
    let splitter = ProportionalDockSplitter::new();
    splitter.set_keep_pinned_dockable_visible(true);
    let item = Tool::new();
    item.set_keep_pinned_dockable_visible(true);
    let list = install(
        &f,
        &root,
        &preview,
        vec![Some(splitter), Some(item.clone())],
    );
    f.hide_previewing_dockables(Some(root.clone()));
    assert_eq!(list.count(), 2);
    assert!(list.ptr_eq(&preview.visible_dockables().unwrap()));
    assert!(Rc::ptr_eq(
        &preview.active_dockable().unwrap(),
        &(item as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &root.pinned_dock().unwrap(),
        &(preview as Rc<dyn IToolDock>)
    ));
}
