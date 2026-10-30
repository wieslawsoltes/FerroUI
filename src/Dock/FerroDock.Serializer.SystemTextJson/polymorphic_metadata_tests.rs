// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::core::IDockSerializer;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct FallbackNode {
    next: RefCell<Option<Rc<FallbackNode>>>,
    reads: Rc<Cell<usize>>,
}
fn serializer(reads: Rc<Cell<usize>>) -> Rc<DockSerializer> {
    let base = JsonTypeInfo::object(
        move || {
            Rc::new(FallbackNode {
                next: RefCell::new(None),
                reads: reads.clone(),
            })
        },
        |node: &Rc<FallbackNode>| Rc::as_ptr(node) as usize,
        vec![JsonPropertyInfo::new(
            "Next",
            |node: &Rc<FallbackNode>| {
                node.reads.set(node.reads.get() + 1);
                node.next.borrow().clone()
            },
            |node, value| {
                node.next.replace(value);
            },
        )],
    );
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(vec![
        JsonTypeInfo::polymorphic::<Rc<FallbackNode>>(Vec::new(), Some(base), true),
        JsonTypeInfo::optional::<Rc<FallbackNode>>(),
    ])))
    .unwrap()
}

#[test]
fn fallback_depth_stops_before_additional_source_getters() {
    let reads = Rc::new(Cell::new(0));
    let serializer = serializer(reads.clone());
    let source = include_str!("../../../tests/Dock/data/serializer-json-audit.fallback.tsv");
    let mut cases = 0;
    for row in source.lines().filter(|row| row.starts_with("depth\t")) {
        let columns: Vec<_> = row.split('\t').collect();
        let length: usize = columns[1].parse().unwrap();
        let mut root = None;
        for _ in 0..length {
            root = Some(Rc::new(FallbackNode {
                next: RefCell::new(root),
                reads: reads.clone(),
            }));
        }
        reads.set(0);
        let result = serializer.serialize(root.unwrap());
        assert_eq!(result.is_ok(), columns[2] == "1", "length {length}");
        assert_eq!(
            reads.get(),
            columns[3].parse::<usize>().unwrap(),
            "length {length}"
        );
        cases += 1;
    }
    assert_eq!(cases, 5);
}

#[test]
fn fallback_preserves_base_self_reference_and_unknown_discriminator() {
    let reads = Rc::new(Cell::new(0));
    let serializer = serializer(reads.clone());
    let node = Rc::new(FallbackNode {
        next: RefCell::new(None),
        reads,
    });
    node.next.replace(Some(node.clone()));
    let text = serializer.serialize(node.clone()).unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.fallback.json")
    );
    let loaded = serializer
        .deserialize::<Rc<FallbackNode>>(Some(&text))
        .unwrap()
        .unwrap();
    assert!(Rc::ptr_eq(&loaded, loaded.next.borrow().as_ref().unwrap()));
    let unknown = serializer
        .deserialize::<Rc<FallbackNode>>(Some("{\"$type\":\"future\",\"Next\":null}"))
        .unwrap()
        .unwrap();
    assert!(unknown.next.borrow().is_none());
    let source = include_str!("../../../tests/Dock/data/serializer-json-audit.fallback.tsv");
    assert!(source.lines().any(|line| line == "base-self-reference\t1"));
    assert!(source.lines().any(|line| line == "unrecognized-to-base\t1"));
    node.next.take();
    loaded.next.take();
}
