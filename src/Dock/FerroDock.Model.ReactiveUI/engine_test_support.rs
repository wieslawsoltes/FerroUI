//! Real production factory subclass for source engine policy and observer tests.
use crate::{Factory, FactoryImpl};
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::{FactoryBase, FactoryBaseImpl};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

type Callback = Rc<dyn Fn()>;
type DropCallback = Rc<dyn Fn(Rc<dyn IDockable>, Rc<dyn IDockable>, DockOperation) -> bool>;
type ToolCreator = Rc<dyn Fn() -> Rc<dyn IToolDock>>;
type DocumentCreator = Rc<dyn Fn() -> Rc<dyn IDocumentDock>>;

pub(crate) struct PolicyFactory {
    component: Factory,
    runtime: Weak<Self>,
    pub(crate) region: RefCell<Option<Rc<dyn IDock>>>,
    pub(crate) drop_allowed: Cell<bool>,
    pub(crate) drop_calls: RefCell<Vec<(String, String, DockOperation)>>,
    pub(crate) on_drop: RefCell<Option<DropCallback>>,
    pub(crate) split_document: Cell<bool>,
    pub(crate) split_calls: Cell<usize>,
    pub(crate) observer: Cell<bool>,
    pub(crate) omit_scope: Cell<bool>,
    pub(crate) throw_move: Cell<bool>,
    pub(crate) throw_complete: Cell<bool>,
    pub(crate) throw_dispose: Cell<bool>,
    pub(crate) trace: RefCell<Vec<&'static str>>,
    pub(crate) before: RefCell<Option<Callback>>,
    pub(crate) after: RefCell<Option<Callback>>,
    pub(crate) dispose: RefCell<Option<Callback>>,
    pub(crate) create_tool: RefCell<Option<ToolCreator>>,
    pub(crate) create_document: RefCell<Option<DocumentCreator>>,
}
impl PolicyFactory {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime: &Weak<Self>| Self {
            component: Factory::construct(runtime.clone()),
            runtime: runtime.clone(),
            region: RefCell::new(None),
            drop_allowed: Cell::new(true),
            drop_calls: RefCell::new(Vec::new()),
            on_drop: RefCell::new(None),
            split_document: Cell::new(false),
            split_calls: Cell::new(0),
            observer: Cell::new(false),
            omit_scope: Cell::new(false),
            throw_move: Cell::new(false),
            throw_complete: Cell::new(false),
            throw_dispose: Cell::new(false),
            trace: RefCell::new(Vec::new()),
            before: RefCell::new(None),
            after: RefCell::new(None),
            dispose: RefCell::new(None),
            create_tool: RefCell::new(None),
            create_document: RefCell::new(None),
        })
    }
}
impl FactoryBaseImpl for PolicyFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for PolicyFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn as_dock_split_policy(this: Rc<Self>) -> Option<Rc<dyn IDockSplitPolicy>> {
        Some(this)
    }
    fn as_dock_drop_policy(this: Rc<Self>) -> Option<Rc<dyn IDockDropPolicy>> {
        Some(this)
    }
    fn as_dock_outer_split_policy(this: Rc<Self>) -> Option<Rc<dyn IDockOuterSplitPolicy>> {
        Some(this)
    }
    fn as_dock_operation_observer(this: Rc<Self>) -> Option<Rc<dyn IDockOperationObserver>> {
        if this.observer.get() {
            Some(this)
        } else {
            None
        }
    }
    fn create_tool_dock(this: &Self) -> Rc<dyn IToolDock> {
        let callback = this.create_tool.borrow().clone();
        callback.map_or_else(|| this.component.create_tool_dock(), |callback| callback())
    }
    fn create_document_dock(this: &Self) -> Rc<dyn IDocumentDock> {
        let callback = this.create_document.borrow().clone();
        callback.map_or_else(
            || this.component.create_document_dock(),
            |callback| callback(),
        )
    }
    fn move_dockable_between_docks(
        this: &Self,
        source: Option<Rc<dyn IDock>>,
        target: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
        anchor: Option<Rc<dyn IDockable>>,
    ) {
        assert!(!this.throw_move.get(), "execution failed");
        this.component
            .base()
            .move_dockable_between_docks(source, target, item, anchor);
    }
}
crate::impl_factory!(PolicyFactory);
impl IDockSplitPolicy for PolicyFactory {
    fn use_document_dock(
        &self,
        _source: Rc<dyn IDockable>,
        _target: Rc<dyn IDock>,
        _operation: DockOperation,
    ) -> bool {
        self.split_calls.set(self.split_calls.get() + 1);
        self.split_document.get()
    }
}
impl IDockDropPolicy for PolicyFactory {
    fn can_drop(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> bool {
        self.drop_calls
            .borrow_mut()
            .push((source.id(), target.id(), operation));
        let callback = self.on_drop.borrow().clone();
        callback.map_or_else(
            || self.drop_allowed.get(),
            |callback| callback(source, target, operation),
        )
    }
}
impl IDockOuterSplitPolicy for PolicyFactory {
    fn get_outer_split_target(
        &self,
        _source: Rc<dyn IDockable>,
        _target: Rc<dyn IDockable>,
        _operation: DockOperation,
    ) -> Option<Rc<dyn IDock>> {
        self.region.borrow().clone()
    }
}
impl IDockOperationObserver for PolicyFactory {
    fn begin_dock_operation(
        &self,
        _source: Rc<dyn IDockable>,
        _target: Rc<dyn IDockable>,
        _operation: DockOperation,
    ) -> Option<Rc<dyn IDockOperationScope>> {
        self.trace.borrow_mut().push("begin");
        let callback = self.before.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        if self.omit_scope.get() {
            None
        } else {
            Some(Rc::new(Scope(self.runtime.upgrade().unwrap())))
        }
    }
}
struct Scope(Rc<PolicyFactory>);
impl IDockOperationScope for Scope {
    fn complete(&self) {
        self.0.trace.borrow_mut().push("complete");
        assert!(!self.0.throw_complete.get(), "completion failed");
        let callback = self.0.after.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}
impl IDisposable for Scope {
    fn dispose(&self) {
        self.0.trace.borrow_mut().push("dispose");
        assert!(!self.0.throw_dispose.get(), "disposal failed");
        let callback = self.0.dispose.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}

pub(crate) fn list(items: Vec<Rc<dyn IDockable>>) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
    Some(ListHandle::from_items(
        (items).into_iter().map(Some).collect(),
    ))
}
