// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;

type Predicate = Option<DockablePredicate>;
type DockableIterator = Box<dyn Iterator<Item = Option<Rc<dyn IDockable>>>>;
type QueryFactory = Rc<dyn Fn() -> Rc<DockableQueryEnumerator>>;

/// A repeatable lazy IEnumerable query retaining the source factory and live tree.
#[derive(Clone)]
pub struct DockableQuery {
    create: QueryFactory,
}
impl DockableQuery {
    /// Creates a repeatable query from an existing Rust iterator factory.
    pub fn new(create: impl Fn() -> DockableIterator + 'static) -> Self {
        Self {
            create: Rc::new(move || {
                DockableQueryEnumerator::new(QueryState::Custom(CollectionCursor::new(create())))
            }),
        }
    }
    /// Creates an independent source cursor without running a predicate.
    pub fn get_enumerator(&self) -> Rc<DockableQueryEnumerator> {
        (self.create)()
    }
    /// Begins independent Rust enumeration over the same source cursor engine.
    pub fn iter(&self) -> DockableIterator {
        Box::new(QueryIterator(self.get_enumerator()))
    }
    pub(super) fn in_dock(
        owner: Rc<dyn IFactory>,
        dock: Option<Rc<dyn IDock>>,
        predicate: Predicate,
    ) -> Self {
        Self {
            create: Rc::new(move || {
                DockableQueryEnumerator::new(QueryState::InDock(FindInDock::new(
                    owner.clone(),
                    dock.clone(),
                    predicate.clone(),
                )))
            }),
        }
    }
    pub(super) fn all(owner: Rc<dyn IFactory>, predicate: Predicate) -> Self {
        Self {
            create: Rc::new(move || {
                DockableQueryEnumerator::new(QueryState::All(FindAll {
                    owner: owner.clone(),
                    predicate: predicate.clone(),
                    started: Cell::new(false),
                    finished: Cell::new(false),
                    controls: RefCell::new(None),
                    nested: RefCell::new(None),
                }))
            }),
        }
    }
}
impl IntoIterator for DockableQuery {
    type Item = Option<Rc<dyn IDockable>>;
    type IntoIter = DockableIterator;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &DockableQuery {
    type Item = Option<Rc<dyn IDockable>>;
    type IntoIter = DockableIterator;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The original query's shared Current, advancement and explicit disposal state.
/// Recursive advancement returns false while the same cursor is running.
pub struct DockableQueryEnumerator {
    state: QueryState,
    current: RefCell<Option<Rc<dyn IDockable>>>,
    moving: Cell<bool>,
}
impl DockableQueryEnumerator {
    fn new(state: QueryState) -> Rc<Self> {
        Rc::new(Self {
            state,
            current: RefCell::new(None),
            moving: Cell::new(false),
        })
    }
    /// Gets null before the first result, otherwise the last yielded source reference.
    pub fn current(&self) -> Option<Rc<dyn IDockable>> {
        self.current.borrow().clone()
    }
    /// Advances the original state machine, preserving Current on end or failure.
    pub fn move_next(&self) -> bool {
        if self.moving.replace(true) {
            return false;
        }
        let _moving = CursorMoveScope(&self.moving);
        match catch_unwind(AssertUnwindSafe(|| self.state.next())) {
            Ok(Some(value)) => {
                self.current.replace(value);
                true
            }
            Ok(None) => false,
            Err(error) => {
                self.state.finish();
                resume_unwind(error)
            }
        }
    }
    /// Runs the source cleanup for the current iterator state without clearing Current.
    /// During callbacks this can affect the running traversal's later live field reads.
    pub fn dispose(&self) {
        self.state.dispose();
    }
    /// Source iterator blocks do not support Reset.
    pub fn reset(&self) -> Result<(), String> {
        Err("Specified method is not supported.".into())
    }
}
struct CursorMoveScope<'a>(&'a Cell<bool>);
impl Drop for CursorMoveScope<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
struct QueryIterator(Rc<DockableQueryEnumerator>);
impl Iterator for QueryIterator {
    type Item = Option<Rc<dyn IDockable>>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.move_next().then(|| self.0.current())
    }
}
impl Drop for QueryIterator {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

enum QueryState {
    InDock(FindInDock),
    All(FindAll),
    Custom(CollectionCursor<Option<Rc<dyn IDockable>>>),
}
impl QueryState {
    fn next(&self) -> Option<Option<Rc<dyn IDockable>>> {
        match self {
            Self::InDock(q) => q.next(),
            Self::All(q) => q.next(),
            Self::Custom(q) => q.next(),
        }
    }
    fn dispose(&self) {
        match self {
            Self::InDock(q) => q.dispose(),
            Self::All(q) => q.finish(),
            Self::Custom(q) => q.dispose(),
        }
    }
    fn finish(&self) {
        match self {
            Self::InDock(q) => q.finish(),
            Self::All(q) => q.finish(),
            Self::Custom(q) => q.dispose(),
        }
    }
}

// Collection cursor storage is taken out before custom iteration or cleanup can
// invoke user code. No RefCell borrow crosses either operation.
struct CollectionCursor<T> {
    iterator: RefCell<Option<Box<dyn Iterator<Item = T>>>>,
    disposed: Cell<bool>,
}
impl<T> CollectionCursor<T> {
    fn new(iterator: Box<dyn Iterator<Item = T>>) -> Self {
        Self {
            iterator: RefCell::new(Some(iterator)),
            disposed: Cell::new(false),
        }
    }
    fn next(&self) -> Option<T> {
        if self.disposed.get() {
            return None;
        }
        let mut iterator = self.iterator.take()?;
        let result = iterator.next();
        if !self.disposed.get() {
            self.iterator.replace(Some(iterator));
        }
        result
    }
    fn dispose(&self) {
        self.disposed.set(true);
        drop(self.iterator.take());
    }
}
impl<T> Drop for CollectionCursor<T> {
    fn drop(&mut self) {
        self.dispose();
    }
}

type DockableChildren = Rc<CollectionCursor<Option<Rc<dyn IDockable>>>>;
type DockControlCursor = Rc<CollectionCursor<Rc<dyn IDockControl>>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DockStage {
    Start,
    RunningRoot,
    RootYield,
    Children,
    ChildYield,
    Nested,
    NestedYield,
    Finished,
}
struct FindInDock {
    owner: Rc<dyn IFactory>,
    dock: Option<Rc<dyn IDock>>,
    predicate: Predicate,
    stage: Cell<DockStage>,
    children: RefCell<Option<DockableChildren>>,
    child: RefCell<Option<Rc<dyn IDockable>>>,
    nested: RefCell<Option<Rc<DockableQueryEnumerator>>>,
}
impl FindInDock {
    fn new(owner: Rc<dyn IFactory>, dock: Option<Rc<dyn IDock>>, predicate: Predicate) -> Self {
        Self {
            owner,
            dock,
            predicate,
            stage: Cell::new(DockStage::Start),
            children: RefCell::new(None),
            child: RefCell::new(None),
            nested: RefCell::new(None),
        }
    }
    fn matches(&self, value: Option<Rc<dyn IDockable>>) -> bool {
        (self
            .predicate
            .as_ref()
            .expect("source predicate invocation"))(value)
    }
    fn begin_children(&self) {
        let dock = self
            .dock
            .as_ref()
            .expect("source dock.VisibleDockables access");
        if dock.visible_dockables().is_some() {
            let list = dock
                .visible_dockables()
                .expect("source visible access requires a list");
            self.children
                .replace(Some(Rc::new(CollectionCursor::new(list.iter()))));
            self.stage.set(DockStage::Children);
        } else {
            self.finish();
        }
    }
    fn begin_nested(&self) {
        let child = self.child.borrow().clone();
        if let Some(dock) = child.and_then(|child| child.as_dock()) {
            let query =
                DockableQuery::in_dock(self.owner.clone(), Some(dock), self.predicate.clone());
            self.nested.replace(Some(query.get_enumerator()));
            self.stage.set(DockStage::Nested);
        } else {
            self.child.take();
            self.stage.set(DockStage::Children);
        }
    }
    fn next(&self) -> Option<Option<Rc<dyn IDockable>>> {
        match self.stage.get() {
            DockStage::Start => {
                self.stage.set(DockStage::RunningRoot);
                if self.matches(self.dock.clone().map(|dock| dock as Rc<dyn IDockable>)) {
                    self.stage.set(DockStage::RootYield);
                    return Some(self.dock.clone().map(|dock| dock as Rc<dyn IDockable>));
                }
                self.begin_children();
                if self.stage.get() == DockStage::Finished {
                    return None;
                }
            }
            DockStage::RootYield => {
                self.stage.set(DockStage::RunningRoot);
                self.begin_children();
                if self.stage.get() == DockStage::Finished {
                    return None;
                }
            }
            DockStage::ChildYield => self.begin_nested(),
            DockStage::NestedYield => self.stage.set(DockStage::Nested),
            DockStage::Finished => return None,
            _ => {}
        }
        loop {
            if self.stage.get() == DockStage::Nested {
                let nested = self
                    .nested
                    .borrow()
                    .clone()
                    .expect("source query enumerator.MoveNext access");
                if nested.move_next() {
                    let live = self
                        .nested
                        .borrow()
                        .clone()
                        .expect("source query enumerator.Current access");
                    self.stage.set(DockStage::NestedYield);
                    return Some(live.current());
                }
                if let Some(nested) = self.nested.take() {
                    nested.dispose();
                }
                self.child.take();
                self.stage.set(DockStage::Children);
            }
            let children = self
                .children
                .borrow()
                .clone()
                .expect("source list enumerator.MoveNext access");
            let Some(child) = children.next() else {
                self.finish();
                return None;
            };
            self.child.replace(child.clone());
            if self.matches(child) {
                self.stage.set(DockStage::ChildYield);
                return Some(self.child.borrow().clone());
            }
            self.begin_nested();
        }
    }
    fn dispose(&self) {
        // Before the root predicate's yield there is no source foreach/finally scope.
        if self.stage.get() != DockStage::RunningRoot {
            self.finish();
        }
    }
    fn finish(&self) {
        self.stage.set(DockStage::Finished);
        if let Some(nested) = self.nested.take() {
            nested.dispose();
        }
        if let Some(children) = self.children.take() {
            children.dispose();
        }
        self.child.take();
    }
}

struct FindAll {
    owner: Rc<dyn IFactory>,
    predicate: Predicate,
    started: Cell<bool>,
    finished: Cell<bool>,
    controls: RefCell<Option<DockControlCursor>>,
    nested: RefCell<Option<Rc<DockableQueryEnumerator>>>,
}
impl FindAll {
    fn next(&self) -> Option<Option<Rc<dyn IDockable>>> {
        if self.finished.get() {
            return None;
        }
        if !self.started.replace(true) {
            self.controls.replace(Some(Rc::new(CollectionCursor::new(
                self.owner.dock_controls().iter(),
            ))));
        }
        loop {
            let nested = self.nested.borrow().clone();
            if let Some(nested) = nested {
                if nested.move_next() {
                    let live = self
                        .nested
                        .borrow()
                        .clone()
                        .expect("source query enumerator.Current access");
                    return Some(live.current());
                }
                if let Some(nested) = self.nested.take() {
                    nested.dispose();
                }
            }
            let controls = self
                .controls
                .borrow()
                .clone()
                .expect("source control enumerator.MoveNext access");
            let Some(control) = controls.next() else {
                self.finish();
                return None;
            };
            if let Some(dock) = control.layout() {
                self.nested.replace(Some(
                    DockableQuery::in_dock(self.owner.clone(), Some(dock), self.predicate.clone())
                        .get_enumerator(),
                ));
            }
        }
    }
    fn finish(&self) {
        self.finished.set(true);
        if let Some(nested) = self.nested.take() {
            nested.dispose();
        }
        if let Some(controls) = self.controls.take() {
            controls.dispose();
        }
    }
}
