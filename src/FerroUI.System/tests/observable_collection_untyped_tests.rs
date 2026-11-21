//! One managed event through typed and INotifyCollectionChanged native views.
use crate::collections::{
    object_model::ObservableCollection, specialized::INotifyCollectionChanged,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone, Copy)]
enum Token {
    Typed(u64),
    Untyped(u64),
}

#[test]
fn untyped_notifications_added_peer_blocks_nested_change_in_same_dispatch() {
    for first_typed in [false, true] {
        for peer_typed in [false, true] {
            let c = ObservableCollection::<i32>::new();
            let peer = Rc::new(Cell::new(None));
            let inner = c.clone();
            let added = peer.clone();
            let first = subscribe(
                &c,
                first_typed,
                Rc::new(move || {
                    if added.get().is_none() {
                        added.set(Some(subscribe(&inner, peer_typed, Rc::new(|| {}))));
                        inner.add(2);
                    }
                }),
            );
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.add(1)));
            remove(&c, first);
            remove(&c, peer.get().unwrap());
            assert!(result.is_err());
            assert_eq!(c.iter().collect::<Vec<_>>(), vec![1]);
            c.add(3);
        }
    }
}
fn subscribe(c: &ObservableCollection<i32>, typed: bool, f: Rc<dyn Fn()>) -> Token {
    if typed {
        Token::Typed(c.add_collection_changed(Rc::new(move |_| f())))
    } else {
        Token::Untyped(c.collection_changed().add(Rc::new(move |_| f())))
    }
}
fn remove(c: &ObservableCollection<i32>, token: Token) {
    match token {
        Token::Typed(t) => {
            c.remove_collection_changed(t);
        }
        Token::Untyped(t) => {
            c.collection_changed().remove(t);
        }
    }
}
#[test]
fn untyped_notifications_count_every_subscriber_and_restore_monitor_after_error() {
    for count in 1..=3 {
        for mask in 0..(1 << count) {
            let c = ObservableCollection::<i32>::new();
            let attempted = Rc::new(Cell::new(false));
            let inner = c.clone();
            let guard = attempted.clone();
            let mut tokens = vec![subscribe(
                &c,
                mask & 1 != 0,
                Rc::new(move || {
                    if !guard.replace(true) {
                        inner.add(2);
                    }
                }),
            )];
            for i in 1..count {
                tokens.push(subscribe(&c, mask & (1 << i) != 0, Rc::new(|| {})));
            }
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.add(1)));
            for token in tokens {
                remove(&c, token);
            }
            let values = c.iter().collect::<Vec<_>>();
            c.add(3);
            assert_eq!(
                result.is_err(),
                count > 1,
                "count={count}, typed-mask={mask}"
            );
            assert_eq!(values, if count == 1 { vec![1, 2] } else { vec![1] });
            assert_eq!(c.iter().last(), Some(3), "monitor reset after exception");
        }
    }
}
#[test]
fn untyped_notifications_and_typed_views_share_registration_order() {
    for mask in 0..16 {
        let c = ObservableCollection::<i32>::new();
        let log = Rc::new(RefCell::new(vec![]));
        let mut tokens = vec![];
        for i in 0..4 {
            let log = log.clone();
            tokens.push(subscribe(
                &c,
                mask & (1 << i) != 0,
                Rc::new(move || log.borrow_mut().push(i)),
            ));
        }
        c.add(1);
        for token in tokens {
            remove(&c, token);
        }
        assert_eq!(*log.borrow(), vec![0, 1, 2, 3], "typed-mask={mask}");
    }
}
#[test]
fn untyped_notifications_take_one_snapshot_before_cross_view_edits() {
    for first_typed in [false, true] {
        let c = ObservableCollection::<i32>::new();
        let log = Rc::new(RefCell::new(vec![]));
        let peer = Rc::new(Cell::new(None));
        let added = Rc::new(Cell::new(None));
        let once = Rc::new(Cell::new(false));
        let inner = c.clone();
        let out = log.clone();
        let current_peer = peer.clone();
        let current_added = added.clone();
        let guard = once.clone();
        let first = subscribe(
            &c,
            first_typed,
            Rc::new(move || {
                out.borrow_mut().push("first");
                if !guard.replace(true) {
                    remove(&inner, current_peer.get().unwrap());
                    let out = out.clone();
                    current_added.set(Some(subscribe(
                        &inner,
                        !first_typed,
                        Rc::new(move || out.borrow_mut().push("new")),
                    )));
                }
            }),
        );
        let out = log.clone();
        peer.set(Some(subscribe(
            &c,
            !first_typed,
            Rc::new(move || out.borrow_mut().push("old")),
        )));
        c.add(1);
        let first_log = log.borrow().clone();
        log.borrow_mut().clear();
        c.add(2);
        let second_log = log.borrow().clone();
        remove(&c, first);
        remove(&c, added.get().unwrap());
        assert_eq!(first_log, vec!["first", "old"], "first typed={first_typed}");
        assert_eq!(second_log, vec!["first", "new"]);
    }
}
#[test]
fn untyped_notifications_use_live_count_but_keep_removed_snapshot_handler() {
    for first_typed in [false, true] {
        let c = ObservableCollection::<i32>::new();
        let log = Rc::new(RefCell::new(vec![]));
        let peer = Rc::new(Cell::new(None));
        let once = Rc::new(Cell::new(false));
        let inner = c.clone();
        let out = log.clone();
        let current_peer = peer.clone();
        let guard = once.clone();
        let first = subscribe(
            &c,
            first_typed,
            Rc::new(move || {
                out.borrow_mut().push(format!("first:{}", inner.count()));
                if !guard.replace(true) {
                    remove(&inner, current_peer.get().unwrap());
                    inner.add(2);
                }
            }),
        );
        let out = log.clone();
        let observed = c.clone();
        peer.set(Some(subscribe(
            &c,
            !first_typed,
            Rc::new(move || out.borrow_mut().push(format!("old:{}", observed.count()))),
        )));
        c.add(1);
        remove(&c, first);
        assert_eq!(
            *log.borrow(),
            vec!["first:1", "first:2", "old:2"],
            "first typed={first_typed}"
        );
    }
}
