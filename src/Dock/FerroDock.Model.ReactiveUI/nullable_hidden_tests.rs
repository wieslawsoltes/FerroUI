//! Original nullable hide/restore, partial commits and observer reentry.
use crate::{controls::*, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{
    collections::ListHandle,
    controls::{IRootDock, IToolDock},
    core::*,
    FactoryBase, FactoryBaseImpl,
};
use std::{
    cell::RefCell,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
type Dockables = ListHandle<Option<Rc<dyn IDockable>>>;
struct HiddenFactory {
    component: Factory,
    root: RefCell<Option<Rc<dyn IRootDock>>>,
    after_pin: RefCell<Option<Rc<dyn IRootDock>>>,
    trace: Rc<RefCell<Vec<String>>>,
}
impl HiddenFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            root: Default::default(),
            after_pin: Default::default(),
            trace: Default::default(),
        })
    }
}
impl FactoryBaseImpl for HiddenFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
    fn pin_dockable_core(&self, item: Option<Rc<dyn IDockable>>) {
        assert!(item.is_none());
        self.trace.borrow_mut().push("pin:null".into());
        if let Some(next) = self.after_pin.borrow().clone() {
            self.root.replace(Some(next));
        }
    }
}
impl FactoryImpl for HiddenFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        assert!(item.is_none());
        this.trace.borrow_mut().push(format!(
            "find:null:{}",
            if predicate.is_some() {
                "all"
            } else {
                "default"
            }
        ));
        this.root.borrow().clone()
    }
}
impl_factory!(HiddenFactory);
fn failure(action: impl FnOnce()) -> String {
    let e = catch_unwind(AssertUnwindSafe(action)).expect_err("source failure");
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
#[test]
fn nullable_hide_and_restore_reach_original_root_queries_and_keep_no_root_noops() {
    let f = HiddenFactory::new();
    f.hide_dockable(None);
    assert_eq!(&*f.trace.borrow(), &["find:null:default", "find:null:all"]);
    f.trace.borrow_mut().clear();
    f.restore_dockable(None);
    assert_eq!(&*f.trace.borrow(), &["find:null:all"]);
    assert!(failure(|| Factory::new().hide_dockable(None)).contains("dockable.Owner"));
    assert!(failure(|| Factory::new().restore_dockable(None)).contains("dockable.Owner"));
}
#[test]
fn hide_null_preserves_pin_hook_reroot_and_hidden_list_creation_before_owner_failure() {
    let f = HiddenFactory::new();
    let first = RootDock::new();
    let second = RootDock::new();
    let pins = f.create_list(vec![None]);
    first.set_left_pinned_dockables(Some(pins.clone()));
    f.root.replace(Some(first.clone()));
    f.after_pin.replace(Some(second.clone()));
    assert!(failure(|| f.hide_dockable(None)).contains("hide dockable.Owner"));
    assert_eq!(
        &*f.trace.borrow(),
        &["find:null:default", "pin:null", "find:null:all"]
    );
    assert!(first.hidden_dockables().is_none());
    assert_eq!(second.hidden_dockables().unwrap().count(), 0);
    assert_eq!(pins.count(), 1);
    assert!(pins.get(0).is_none());
}
#[test]
fn restore_null_returns_before_item_access_unless_null_membership_commits_removal_and_event() {
    for case in 0..4 {
        let f = HiddenFactory::new();
        let root = RootDock::new();
        f.root.replace(Some(root.clone()));
        if case > 0 {
            root.set_hidden_dockables(Some(f.create_list::<Option<Rc<dyn IDockable>>>(
                match case {
                    1 => vec![],
                    2 => vec![Some(Tool::new())],
                    _ => vec![None],
                },
            )));
        }
        let trace = f.trace.clone();
        let observed = root.hidden_dockables();
        let event = f.dockable_restored(Rc::new(move |args| {
            assert!(args.dockable().is_none());
            assert_eq!(observed.as_ref().unwrap().count(), 0);
            trace.borrow_mut().push("restored:null".into());
        }));
        if case == 3 {
            assert!(failure(|| f.restore_dockable(None)).contains("restore dockable.OriginalOwner"));
            assert_eq!(&*f.trace.borrow(), &["find:null:all", "restored:null"]);
            assert_eq!(root.hidden_dockables().unwrap().count(), 0);
        } else {
            f.restore_dockable(None);
            assert_eq!(&*f.trace.borrow(), &["find:null:all"]);
            assert_eq!(
                root.hidden_dockables().as_ref().map(Dockables::count),
                match case {
                    0 => None,
                    1 => Some(0),
                    _ => Some(1),
                }
            );
        }
        event.dispose();
    }
}
#[test]
fn restore_null_keeps_collection_payload_index_and_callback_replacement_before_owner_failure() {
    let f = HiddenFactory::new();
    let root = RootDock::new();
    let old = f.create_list(vec![None, None]);
    let replacement = f.create_list(vec![Some(Tool::new() as Rc<dyn IDockable>)]);
    root.set_hidden_dockables(Some(old.clone()));
    f.root.replace(Some(root.clone()));
    let trace = f.trace.clone();
    let token = old
        .add_collection_changed(Rc::new(move |args| {
            assert_eq!(args.old_items.len(), 1);
            assert!(args.old_items[0].is_none());
            assert_eq!(args.old_starting_index, 0);
            trace.borrow_mut().push("list:null:0".into());
        }))
        .unwrap();
    let weak = Rc::downgrade(&root);
    let original = old.clone();
    let new_list = replacement.clone();
    let trace = f.trace.clone();
    let event = f.dockable_restored(Rc::new(move |args| {
        assert!(args.dockable().is_none());
        assert_eq!(original.count(), 1);
        trace.borrow_mut().push("restored:null".into());
        weak.upgrade()
            .unwrap()
            .set_hidden_dockables(Some(new_list.clone()));
    }));
    assert!(failure(|| f.restore_dockable(None)).contains("restore dockable.OriginalOwner"));
    assert_eq!(
        &*f.trace.borrow(),
        &["find:null:all", "list:null:0", "restored:null"]
    );
    assert_eq!(old.count(), 1);
    assert!(old.get(0).is_none());
    assert!(replacement.ptr_eq(&root.hidden_dockables().unwrap()));
    assert_eq!(replacement.count(), 1);
    event.dispose();
    old.remove_collection_changed(token);
}
struct Scene {
    factory: Rc<Factory>,
    root: Rc<dyn IRootDock>,
    owner: Rc<dyn IToolDock>,
    alternate: Rc<dyn IToolDock>,
    item: Rc<dyn IDockable>,
    visible: Dockables,
    other_visible: Dockables,
}
fn scene() -> Scene {
    let f = Factory::new();
    let root = f.create_root_dock();
    let owner = f.create_tool_dock();
    let alternate = f.create_tool_dock();
    let item = f.create_tool();
    item.set_id("item".into());
    f.add_dockable(Some(root.clone()), Some(owner.clone()));
    f.add_dockable(Some(root.clone()), Some(alternate.clone()));
    f.add_dockable(Some(owner.clone()), Some(item.clone()));
    f.init_layout(root.clone());
    let visible = owner.visible_dockables().unwrap();
    let other_visible = alternate.visible_dockables().unwrap();
    Scene {
        factory: f,
        root,
        owner,
        alternate,
        item,
        visible,
        other_visible,
    }
}
#[test]
fn hide_rereads_hidden_list_after_removed_reentry_and_retains_partial_commit_on_missing_list() {
    for replace in [true, false] {
        let s = scene();
        assert!(s.root.hidden_dockables().is_none());
        let replacement = s.factory.create_list(vec![]);
        let captured: Rc<RefCell<Option<Dockables>>> = Default::default();
        let capture = captured.clone();
        let weak = Rc::downgrade(&s.root);
        let new_list = replacement.clone();
        let trace = Rc::new(RefCell::new(Vec::new()));
        let log = trace.clone();
        let removed = s.factory.dockable_removed(Rc::new(move |args| {
            assert_eq!(args.dockable().unwrap().id(), "item");
            let root = weak.upgrade().unwrap();
            capture.replace(root.hidden_dockables());
            root.set_hidden_dockables(if replace {
                Some(new_list.clone())
            } else {
                None
            });
            log.borrow_mut().push("removed");
        }));
        let log = trace.clone();
        let hidden = s
            .factory
            .dockable_hidden(Rc::new(move |_| log.borrow_mut().push("hidden")));
        if replace {
            s.factory.hide_dockable(Some(s.item.clone()));
            assert!(replacement.ptr_eq(&s.root.hidden_dockables().unwrap()));
            assert_eq!(replacement.count(), 1);
            assert!(Rc::ptr_eq(&replacement.get(0).unwrap(), &s.item));
            assert_eq!(&*trace.borrow(), &["removed", "hidden"]);
        } else {
            assert!(failure(|| s.factory.hide_dockable(Some(s.item.clone())))
                .contains("hide root.HiddenDockables.Add"));
            assert!(s.root.hidden_dockables().is_none());
            assert_eq!(&*trace.borrow(), &["removed"]);
        }
        assert_eq!(captured.borrow().as_ref().unwrap().count(), 0);
        assert_eq!(s.visible.count(), 0);
        assert!(s.visible.ptr_eq(&s.owner.visible_dockables().unwrap()));
        assert!(Rc::ptr_eq(
            &s.item.owner().unwrap(),
            &(s.root.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &s.item.original_owner().unwrap(),
            &(s.owner as Rc<dyn IDockable>)
        ));
        removed.dispose();
        hidden.dispose();
    }
}
#[test]
fn restore_observer_can_redirect_original_owner_while_all_original_list_identities_survive() {
    let s = scene();
    s.factory.hide_dockable(Some(s.item.clone()));
    let hidden = s.root.hidden_dockables().unwrap();
    assert_eq!(hidden.count(), 1);
    let target = s.alternate.clone();
    let original_root = s.root.clone();
    let expected = s.item.clone();
    let event = s.factory.dockable_restored(Rc::new(move |args| {
        let item = args.dockable().unwrap();
        assert!(Rc::ptr_eq(&item, &expected));
        assert!(Rc::ptr_eq(
            &item.owner().unwrap(),
            &(original_root.clone() as Rc<dyn IDockable>)
        ));
        item.set_original_owner(Some(target.clone()));
    }));
    s.factory.restore_dockable(Some(s.item.clone()));
    assert_eq!(hidden.count(), 0);
    assert!(hidden.ptr_eq(&s.root.hidden_dockables().unwrap()));
    assert_eq!(s.visible.count(), 0);
    assert_eq!(s.other_visible.count(), 1);
    assert!(s
        .other_visible
        .ptr_eq(&s.alternate.visible_dockables().unwrap()));
    assert!(Rc::ptr_eq(&s.other_visible.get(0).unwrap(), &s.item));
    assert!(Rc::ptr_eq(
        &s.item.owner().unwrap(),
        &(s.alternate as Rc<dyn IDockable>)
    ));
    assert!(s.item.original_owner().is_none());
    assert!(Rc::ptr_eq(
        &s.item.factory().unwrap(),
        &(s.factory.clone() as Rc<dyn IFactory>)
    ));
    event.dispose();
}
