// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source traversal thresholds, wrapper/splitter exclusion and custom enumerator ordering.
use super::*;
use ferrodock_model::{
    collections::{IList, ListHandle},
    core::{IDockable, Orientation},
};
use ferrodock_model_reactiveui::controls::{
    Document, DocumentDock, ProportionalDock, ProportionalDockSplitter, RootDock,
};
use std::cell::Cell;
fn layout(orientation: Orientation, count: usize) -> Rc<ProportionalDock> {
    let p = ProportionalDock::new();
    p.set_orientation(orientation);
    let items: Vec<Rc<dyn IDockable>> = (0..count)
        .map(|_| Document::new() as Rc<dyn IDockable>)
        .collect();
    p.set_visible_dockables(Some(ListHandle::from_plain(
        items.into_iter().map(Some).collect(),
    )));
    p
}
fn root(children: Vec<Rc<dyn IDockable>>) -> Rc<RootDock> {
    let r = RootDock::new();
    r.set_visible_dockables(Some(ListHandle::from_plain(
        children.into_iter().map(Some).collect(),
    )));
    r
}
#[test]
fn orientation_counts_enable_opposite_global_direction_only_at_two() {
    for count in 0..4 {
        assert_eq!(
            GlobalDockingHelper::can_global_dock(root(vec![layout(
                Orientation::Horizontal,
                count
            )])),
            (false, count >= 2)
        );
        assert_eq!(
            GlobalDockingHelper::can_global_dock(root(vec![layout(Orientation::Vertical, count)])),
            (count >= 2, false)
        );
    }
    assert_eq!(
        GlobalDockingHelper::can_global_dock(root(vec![
            layout(Orientation::Vertical, 2),
            layout(Orientation::Horizontal, 2)
        ])),
        (true, true)
    );
}
#[test]
fn source_ignores_splitters_and_single_child_proportional_wrappers_but_counts_empty_ones() {
    let outer = layout(Orientation::Horizontal, 0);
    let one = layout(Orientation::Vertical, 1);
    outer.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(one.clone() as Rc<dyn IDockable>),
        Some(ProportionalDockSplitter::new()),
    ])));
    assert_eq!(
        GlobalDockingHelper::can_global_dock(root(vec![outer.clone()])),
        (false, false)
    );
    outer
        .visible_dockables()
        .unwrap()
        .add(Some(layout(Orientation::Vertical, 0)));
    outer
        .visible_dockables()
        .unwrap()
        .add(Some(layout(Orientation::Vertical, 0)));
    assert_eq!(
        GlobalDockingHelper::can_global_dock(root(vec![outer])),
        (false, true)
    );
}
#[test]
fn source_descends_only_into_direct_proportional_children_and_retains_null_fallback() {
    let nested = layout(Orientation::Horizontal, 2);
    let ordinary = DocumentDock::new();
    ordinary.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(nested)])));
    assert_eq!(
        GlobalDockingHelper::can_global_dock(root(vec![ordinary.clone()])),
        (false, false)
    );
    let prop = layout(Orientation::Vertical, 0);
    prop.set_visible_dockables(None);
    assert_eq!(
        GlobalDockingHelper::can_global_dock(root(vec![prop])),
        (false, false)
    );
    assert_eq!(
        GlobalDockingHelper::can_global_dock(RootDock::new()),
        (false, false)
    );
}
struct Sequence {
    items: Vec<Rc<dyn IDockable>>,
    visits: Rc<Cell<usize>>,
    disposed: Rc<Cell<usize>>,
}
struct Cursor {
    items: std::vec::IntoIter<Rc<dyn IDockable>>,
    visits: Rc<Cell<usize>>,
    disposed: Rc<Cell<usize>>,
}
impl Iterator for Cursor {
    type Item = Rc<dyn IDockable>;
    fn next(&mut self) -> Option<Self::Item> {
        let v = self.items.next();
        if v.is_some() {
            self.visits.set(self.visits.get() + 1);
        }
        v
    }
}
impl Drop for Cursor {
    fn drop(&mut self) {
        self.disposed.set(self.disposed.get() + 1);
    }
}
impl IList<Option<Rc<dyn IDockable>>> for Sequence {
    fn count(&self) -> usize {
        self.items.len()
    }
    fn get(&self, i: usize) -> Option<Rc<dyn IDockable>> {
        Some(self.items[i].clone())
    }
    fn set(&self, _: usize, _: Option<Rc<dyn IDockable>>) {
        panic!("read only source")
    }
    fn insert(&self, _: usize, _: Option<Rc<dyn IDockable>>) {
        panic!("read only source")
    }
    fn remove_at(&self, _: usize) {
        panic!("read only source")
    }
    fn clear(&self) {
        panic!("read only source")
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn index_of(&self, d: &Option<Rc<dyn IDockable>>) -> Option<usize> {
        self.items
            .iter()
            .position(|i| d.as_ref().is_some_and(|d| Rc::ptr_eq(i, d)))
    }
    fn get_enumerator(self: Rc<Self>) -> Box<dyn Iterator<Item = Option<Rc<dyn IDockable>>>> {
        Box::new(
            (Cursor {
                items: self.items.clone().into_iter(),
                visits: self.visits.clone(),
                disposed: self.disposed.clone(),
            })
            .map(Some),
        )
    }
}
#[test]
fn source_early_exit_disposes_actual_custom_enumerator_before_later_children() {
    let visits = Rc::new(Cell::new(0));
    let disposed = Rc::new(Cell::new(0));
    let r = RootDock::new();
    r.set_visible_dockables(Some(ListHandle::from_list(Rc::new(Sequence {
        items: vec![
            layout(Orientation::Horizontal, 2),
            layout(Orientation::Vertical, 2),
            layout(Orientation::Horizontal, 10),
        ],
        visits: visits.clone(),
        disposed: disposed.clone(),
    }))));
    assert_eq!(GlobalDockingHelper::can_global_dock(r), (true, true));
    assert_eq!(visits.get(), 2);
    assert_eq!(disposed.get(), 1);
}
#[test]
fn repeated_queries_read_original_mutable_collection_without_retaining_a_cached_tree() {
    let r = root(vec![]);
    let list = r.visible_dockables().unwrap();
    assert_eq!(
        GlobalDockingHelper::can_global_dock(r.clone()),
        (false, false)
    );
    let h = layout(Orientation::Horizontal, 2);
    list.add(Some(h.clone()));
    assert_eq!(
        GlobalDockingHelper::can_global_dock(r.clone()),
        (false, true)
    );
    h.visible_dockables().unwrap().clear();
    assert_eq!(GlobalDockingHelper::can_global_dock(r), (false, false));
}
