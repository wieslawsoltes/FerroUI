// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Both native factory views retain the domain query's source disposal state.
use crate::markup_extension_tests::run;
use ferrodock_ferro::controls::DockControl;
use ferrodock_model::core::*;
use ferrodock_model_ferro::{
    DockablePredicateReference, DockableQueryEnumeratorReference, DockableQueryReference,
    FactoryBaseReference, IFactoryMetadata,
};
use ferrodock_model_reactiveui::{
    controls::{Document, RootDock},
    Factory,
};
use ferroui_base::{
    metadata::{from_markup_value, MarkupTyped},
    BoxedValue,
};
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
fn both_native_factory_views_preserve_source_callback_disposal_and_reentry_traces() {
    run(|| {
        for base_view in [false, true] {
            for line in include_str!("../data/query-cursor-reference.tsv")
                .lines()
                .filter(|line| !line.starts_with('#'))
            {
                let parts: Vec<_> = line.split('\t').collect();
                let all = parts[0] == "True";

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
                    let control = DockControl::new();
                    control.set_initialize_layout(false);
                    control.set_layout(Some(root.clone()));
                    f.dock_controls().add(DockControl::as_dock_control(control));
                }
                let trace = Rc::new(RefCell::new(Vec::<String>::new()));
                let log = trace.clone();
                let holder: Rc<RefCell<Option<DockableQueryEnumeratorReference>>> =
                    Default::default();
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
                let (receiver, metadata) = if base_view {
                    (
                        Rc::new(FactoryBaseReference::new(f.clone())) as BoxedValue,
                        FactoryBaseReference::MARKUP,
                    )
                } else {
                    (
                        Rc::new(f.clone() as Rc<dyn IFactory>) as BoxedValue,
                        <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
                    )
                };
                let method = metadata
                    .find_methods("Find")
                    .find(|m| m.parameters.len() == if all { 1 } else { 2 })
                    .unwrap();
                let mut args = vec![Some(receiver)];
                if !all {
                    args.push(Some(Rc::new(root as Rc<dyn IDock>) as BoxedValue));
                }
                args.push(Some(
                    Rc::new(DockablePredicateReference::new(predicate)) as BoxedValue
                ));
                let value = (method.invoke)(&args).unwrap();
                let cursor = from_markup_value::<DockableQueryReference>(&value)
                    .unwrap()
                    .get_enumerator();
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
    });
}
