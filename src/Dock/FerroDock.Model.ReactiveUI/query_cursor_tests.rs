// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original source query disposal and reentry traces through the shared model cursor.
use crate::{
    controls::{Document, RootDock},
    test_support::TestControl,
    Factory,
};
use ferrodock_model::{core::*, DockableQueryEnumerator};
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
fn boolean(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}
fn name(value: Option<Rc<dyn IDockable>>) -> String {
    value.map_or_else(|| "null".into(), |item| item.id())
}
fn failure_name(error: Box<dyn std::any::Any + Send>) -> &'static str {
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap_or("");
    assert!(
        message.starts_with("source ") && message.contains("enumerator."),
        "unexpected failure: {message}"
    );
    "NullReferenceException"
}
#[test]
fn direct_query_callback_disposal_matches_original_source_traces() {
    for line in include_str!("../../../tests/Dock/data/query-cursor-reference.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let parts: Vec<_> = line.split('\t').collect();
        let all = parts[0] == "True";
        if all {
            continue;
        }
        let target = parts[1].to_owned();
        let matched = parts[2] == "True";
        let twice = parts[3] == "True";
        let f = Factory::new();
        let root = RootDock::new();
        root.set_id("root".into());
        let nested = RootDock::new();
        nested.set_id("nested".into());
        let leaf = Document::new();
        leaf.set_id("leaf".into());
        let tail = Document::new();
        tail.set_id("tail".into());
        nested.set_visible_dockables(Some(f.create_list(vec![Some(leaf)])));
        root.set_visible_dockables(Some(f.create_list(vec![Some(nested), Some(tail)])));
        if all {
            f.dock_controls().add(TestControl::new(Some(root.clone())));
        }
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        let holder: Rc<RefCell<Option<Rc<DockableQueryEnumerator>>>> = Default::default();
        let input = holder.clone();
        let disposed = Cell::new(false);
        let predicate: DockablePredicate = Rc::new(move |item| {
            let item = item.expect("source nonnull fixture item");
            log.borrow_mut().push(format!("predicate:{}", item.id()));
            if item.id() == target && !disposed.replace(true) {
                let cursor = input.borrow().as_ref().unwrap().clone();
                log.borrow_mut()
                    .push(format!("before:{}", name(cursor.current())));
                cursor.dispose();
                log.borrow_mut()
                    .push(format!("after:{}", name(cursor.current())));
                if twice {
                    let reentry = cursor.move_next();
                    log.borrow_mut()
                        .push(format!("reentry:{}", boolean(reentry)));
                    cursor.dispose();
                }
            }
            matched
        });
        let query = if all {
            f.find(Some(predicate))
        } else {
            f.find_in_dock(Some(root), Some(predicate))
        };
        let cursor = query.get_enumerator();
        holder.replace(Some(cursor.clone()));
        let result = catch_unwind(AssertUnwindSafe(|| {
            for _ in 0..9 {
                let moved = cursor.move_next();
                trace.borrow_mut().push(format!(
                    "move:{}:{}",
                    boolean(moved),
                    name(cursor.current())
                ));
                if !moved {
                    break;
                }
            }
        }));
        if let Err(error) = result {
            trace
                .borrow_mut()
                .push(format!("error:{}", failure_name(error)));
        }
        for _ in 0..2 {
            match catch_unwind(AssertUnwindSafe(|| cursor.move_next())) {
                Ok(moved) => trace.borrow_mut().push(format!(
                    "post:{}:{}",
                    boolean(moved),
                    name(cursor.current())
                )),
                Err(error) => trace
                    .borrow_mut()
                    .push(format!("post-error:{}", failure_name(error))),
            }
        }
        cursor.dispose();
        trace
            .borrow_mut()
            .push(format!("disposed-end:{}", name(cursor.current())));
        assert_eq!(
            trace.borrow().join("|"),
            parts[4],
            "{}",
            parts[..4].join(" ")
        );
        holder.take();
    }
}
#[test]
fn all_controls_callback_disposal_matches_original_source_traces() {
    for line in include_str!("../../../tests/Dock/data/query-cursor-reference.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let parts: Vec<_> = line.split('\t').collect();
        let all = parts[0] == "True";
        if !all {
            continue;
        }
        let target = parts[1].to_owned();
        let matched = parts[2] == "True";
        let twice = parts[3] == "True";
        let f = Factory::new();
        let root = RootDock::new();
        root.set_id("root".into());
        let nested = RootDock::new();
        nested.set_id("nested".into());
        let leaf = Document::new();
        leaf.set_id("leaf".into());
        let tail = Document::new();
        tail.set_id("tail".into());
        nested.set_visible_dockables(Some(f.create_list(vec![Some(leaf)])));
        root.set_visible_dockables(Some(f.create_list(vec![Some(nested), Some(tail)])));
        if all {
            f.dock_controls().add(TestControl::new(Some(root.clone())));
        }
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        let holder: Rc<RefCell<Option<Rc<DockableQueryEnumerator>>>> = Default::default();
        let input = holder.clone();
        let disposed = Cell::new(false);
        let predicate: DockablePredicate = Rc::new(move |item| {
            let item = item.expect("source nonnull fixture item");
            log.borrow_mut().push(format!("predicate:{}", item.id()));
            if item.id() == target && !disposed.replace(true) {
                let cursor = input.borrow().as_ref().unwrap().clone();
                log.borrow_mut()
                    .push(format!("before:{}", name(cursor.current())));
                cursor.dispose();
                log.borrow_mut()
                    .push(format!("after:{}", name(cursor.current())));
                if twice {
                    let reentry = cursor.move_next();
                    log.borrow_mut()
                        .push(format!("reentry:{}", boolean(reentry)));
                    cursor.dispose();
                }
            }
            matched
        });
        let query = if all {
            f.find(Some(predicate))
        } else {
            f.find_in_dock(Some(root), Some(predicate))
        };
        let cursor = query.get_enumerator();
        holder.replace(Some(cursor.clone()));
        let result = catch_unwind(AssertUnwindSafe(|| {
            for _ in 0..9 {
                let moved = cursor.move_next();
                trace.borrow_mut().push(format!(
                    "move:{}:{}",
                    boolean(moved),
                    name(cursor.current())
                ));
                if !moved {
                    break;
                }
            }
        }));
        if let Err(error) = result {
            trace
                .borrow_mut()
                .push(format!("error:{}", failure_name(error)));
        }
        for _ in 0..2 {
            match catch_unwind(AssertUnwindSafe(|| cursor.move_next())) {
                Ok(moved) => trace.borrow_mut().push(format!(
                    "post:{}:{}",
                    boolean(moved),
                    name(cursor.current())
                )),
                Err(error) => trace
                    .borrow_mut()
                    .push(format!("post-error:{}", failure_name(error))),
            }
        }
        cursor.dispose();
        trace
            .borrow_mut()
            .push(format!("disposed-end:{}", name(cursor.current())));
        assert_eq!(
            trace.borrow().join("|"),
            parts[4],
            "{}",
            parts[..4].join(" ")
        );
        holder.take();
    }
}
#[test]
fn model_cursor_before_start_disposal_reset_and_independent_enumeration() {
    let f = Factory::new();
    let root = RootDock::new();
    let query = f.find_in_dock(Some(root.clone()), Some(Rc::new(|_| true)));
    let before = query.get_enumerator();
    assert!(before.current().is_none());
    before.dispose();
    assert!(!before.move_next());
    assert!(before.current().is_none());
    before.dispose();
    assert!(!before.move_next());
    assert!(before.current().is_none());
    assert_eq!(
        before.reset().unwrap_err(),
        "Specified method is not supported."
    );
    let other = query.get_enumerator();
    assert!(other.move_next());
    assert!(Rc::ptr_eq(
        &other.current().unwrap(),
        &(root as Rc<dyn IDockable>)
    ));
    let previous = other.current().unwrap();
    other.dispose();
    assert!(!other.move_next());
    assert!(Rc::ptr_eq(&other.current().unwrap(), &previous));
    assert_eq!(query.iter().count(), 1);
}
