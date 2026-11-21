//! Unchanged-source model corpus and native ownership/subscription checks.
use crate::controls::{
    RibbonApplicationMenuItem as Item, RibbonApplicationMenuRecentItem as Recent,
};
use ferroui_base::{
    BoxedValue,
    input::ICommand,
    reactive::{Disposable, IDisposable},
};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Command;
impl ICommand for Command {
    fn can_execute(&self, _: Option<&BoxedValue>) -> bool {
        true
    }
    fn execute(&self, _: Option<&BoxedValue>) {}
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
fn text(value: Option<BoxedValue>) -> String {
    value
        .and_then(|v| v.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}
#[test]
fn application_menu_items_match_unchanged_source_model_corpus() {
    crate::tests::run(|| {
        let item = Item::new();
        let recent = Recent::new();
        let defaults = json!({"Id":item.id(),"Label":item.label(),"Description":item.description(),"Icon":item.icon().is_some().then_some("unexpected"),"Command":item.command().is_some().then_some("unexpected"),"CommandParameter":item.command_parameter().is_some().then_some("unexpected"),"IsEnabled":item.is_enabled(),"HasSeparatorBefore":item.has_separator_before(),"Count":item.items().count(),"RecentTitle":recent.title(),"RecentPath":recent.path(),"RecentIcon":recent.icon().is_some().then_some("unexpected"),"RecentCommand":recent.command().is_some().then_some("unexpected"),"Tag":recent.tag().is_some().then_some("unexpected"),"IsPinned":recent.is_pinned()});
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        let subscription = item.changed(move |_| c.set(c.get() + 1));
        let mut changed = Vec::new();
        item.set_id(Some("plain".into()));
        item.set_command(Some(Rc::new(Command)));
        item.set_command_parameter(Some(Rc::new("param".to_owned())));
        changed.push(count.get());
        item.set_label(Some("Open".into()));
        changed.push(count.get());
        item.set_label(Some("Open".into()));
        changed.push(count.get());
        item.set_description(Some("desc".into()));
        changed.push(count.get());
        item.set_icon(Some(Rc::new(1i32)));
        changed.push(count.get());
        item.set_is_enabled(false);
        changed.push(count.get());
        item.set_has_separator_before(true);
        changed.push(count.get());
        let child = Item::new();
        item.items().add(Some(child.clone()));
        changed.push(count.get());
        item.items().add(Some(child.clone()));
        changed.push(count.get());
        item.items().move_item(0, 1);
        changed.push(count.get());
        item.items().set(0, Some(child.clone()));
        changed.push(count.get());
        item.items().clear();
        changed.push(count.get());
        item.items().clear();
        changed.push(count.get());
        let rc = Rc::new(Cell::new(0));
        let c = rc.clone();
        let recent_subscription = recent.changed(move |_| c.set(c.get() + 1));
        let mut recent_changed = Vec::new();
        recent.set_tag(Some(Rc::new("plain".to_owned())));
        recent.set_command(Some(Rc::new(Command)));
        recent_changed.push(rc.get());
        recent.set_title(Some("Title".into()));
        recent_changed.push(rc.get());
        recent.set_title(Some("Title".into()));
        recent_changed.push(rc.get());
        recent.set_path(Some("Path".into()));
        recent_changed.push(rc.get());
        recent.set_icon(Some(Rc::new(1i32)));
        recent_changed.push(rc.get());
        recent.set_is_pinned(true);
        recent_changed.push(rc.get());
        let order = Rc::new(RefCell::new(Vec::<String>::new()));
        let owner = Item::new();
        let t = order.clone();
        let owner_subscription = owner.changed(move |s| {
            t.borrow_mut()
                .push(format!("changed:{}", s.items().count()))
        });
        let t = order.clone();
        let property_token = owner.items().property_changed().add(Rc::new(move |p| {
            t.borrow_mut().push(format!("property:{p}"))
        }));
        let t = order.clone();
        let collection_token = owner.items().add_collection_changed(Rc::new(move |e| {
            t.borrow_mut().push(format!("collection:{:?}", e.action))
        }));
        owner.items().add(Some(child.clone()));
        owner.items().add(Some(child.clone()));
        owner.items().add(None);
        owner.items().move_item(0, 2);
        owner.items().set(0, Some(child));
        owner.items().remove(&None);
        owner.items().clear();
        owner.items().clear();
        let clicks = Rc::new(RefCell::new(Vec::<String>::new()));
        let a = Item::new();
        a.set_command_parameter(Some(Rc::new("old".to_owned())));
        let identity = Rc::new(Cell::new(true));
        let first = Rc::new(RefCell::new(None));
        let b = Rc::new(RefCell::new(None::<Rc<dyn IDisposable>>));
        let added = Rc::new(RefCell::new(Vec::new()));
        let t = clicks.clone();
        let same = identity.clone();
        let f = first.clone();
        let remove = b.clone();
        let keep = added.clone();
        let first_handler = a.click(move |sender, args| {
            f.replace(Some(args.clone()));
            same.set(same.get() && args.item().ptr_eq(&sender.to_ref()));
            t.borrow_mut().push(format!("A:{}", text(args.parameter())));
            sender.set_command_parameter(Some(Rc::new("new".to_owned())));
            if let Some(h) = remove.borrow().as_ref() {
                h.dispose();
            }
            let t = t.clone();
            keep.borrow_mut().push(
                sender.click(move |_, e| t.borrow_mut().push(format!("C:{}", text(e.parameter())))),
            );
        });
        let t = clicks.clone();
        let same = identity.clone();
        let f = first.clone();
        b.replace(Some(a.click(move |_, args| {
            same.set(same.get() && Rc::ptr_eq(f.borrow().as_ref().unwrap(), args));
            t.borrow_mut().push(format!("B:{}", text(args.parameter())));
        })));
        a.raise_click(a.command_parameter());
        a.raise_click(Some(Rc::new("next".to_owned())));
        let recent_clicks = Rc::new(RefCell::new(Vec::new()));
        let t = recent_clicks.clone();
        let weak = recent.downgrade();
        let click_subscription = recent.click(move |sender| {
            t.borrow_mut()
                .push(weak.upgrade().is_some_and(|r| r.ptr_eq(&sender.to_ref())))
        });
        recent.raise_click();
        let mut reentry = Vec::new();
        for external_observer in [false, true] {
            let target = Item::new();
            let trace = Rc::new(RefCell::new(Vec::<String>::new()));
            let attempted = Cell::new(false);
            let t = trace.clone();
            let subscription = target.changed(move |item| {
                t.borrow_mut()
                    .push(format!("changed:{}", item.items().count()));
                if !attempted.replace(true)
                    && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        item.items().add(Some(Item::new()))
                    }))
                    .is_err()
                {
                    t.borrow_mut().push("rejected".into());
                }
            });
            let t = trace.clone();
            let weak = target.downgrade();
            let external_token = external_observer.then(|| {
                target.items().add_collection_changed(Rc::new(move |_| {
                    t.borrow_mut().push(format!(
                        "external:{}",
                        weak.upgrade().unwrap().items().count()
                    ))
                }))
            });
            target.items().add(Some(Item::new()));
            reentry.push(json!({"Count":target.items().count(),"trace":*trace.borrow()}));
            subscription.dispose();
            if let Some(token) = external_token {
                target.items().remove_collection_changed(token);
            }
        }
        let actual = json!({"defaults":defaults,"changed":changed,"recentChanged":recent_changed,"order":*order.borrow(),"clicks":*clicks.borrow(),"identity":identity.get(),"recentClicks":*recent_clicks.borrow(),"reentry":reentry});
        let expected: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/application-menu-items.json"
        ))
        .unwrap();
        assert_eq!(actual, expected);
        for h in [
            subscription,
            recent_subscription,
            owner_subscription,
            first_handler,
            click_subscription,
        ] {
            h.dispose();
        }
        for h in added.borrow().iter() {
            h.dispose();
        }
        if let Some(h) = b.borrow().as_ref() {
            h.dispose();
        }
        owner.items().property_changed().remove(property_token);
        owner.items().remove_collection_changed(collection_token);
    });
}
#[test]
fn application_menu_native_collection_identity_and_subscriptions_do_not_retain_owner() {
    crate::tests::run(|| {
        let item = Item::new();
        let items = item.items();
        assert!(items.ptr_eq(&item.items()));
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        let h = item.changed(move |_| c.set(c.get() + 1));
        item.set_label(Some("one".into()));
        assert_eq!(count.get(), 1);
        h.dispose();
        item.set_label(Some("two".into()));
        assert_eq!(count.get(), 1);
        let weak = item.downgrade();
        drop(item);
        assert!(weak.upgrade().is_none());
        items.add(None);
        assert_eq!(items.count(), 1);
        h.dispose();
        let recent = Recent::new();
        let weak = recent.downgrade();
        let h = recent.click(|_| {});
        drop(recent);
        assert!(weak.upgrade().is_none());
        h.dispose();
    });
}
