//! Transfer deferred declarations from a detached candidate to preserved owners.
use super::*;

pub(super) struct Context {
    pub(super) nodes: Rc<CapturedResourceNodes>,
    pub(super) root: Option<WeakValue>,
}
type Captures = Rc<RefCell<Vec<Weak<DeferredContent>>>>;
ferroui_base::ferro_thread_local! {
    static CAPTURES: RefCell<Option<Captures>> = const { RefCell::new(None) };
}
pub(super) fn record(content: &Rc<DeferredContent>) {
    CAPTURES.with(|slot| {
        if let Some(captures) = slot.borrow().as_ref() {
            captures.borrow_mut().push(Rc::downgrade(content));
        }
    });
}

/// Captures only deferred declarations created in this UI-thread scope. Nested
/// scopes restore the outer scope, including when candidate construction unwinds.
/// Neither the scope nor committed contexts keep resource owners alive.
pub struct DeferredContentReloadScope {
    captures: Captures,
    previous: Option<Captures>,
}
impl Default for DeferredContentReloadScope {
    fn default() -> Self {
        Self::new()
    }
}
impl DeferredContentReloadScope {
    pub fn new() -> Self {
        let captures = Rc::new(RefCell::new(Vec::new()));
        let previous = CAPTURES.with(|slot| slot.replace(Some(captures.clone())));
        Self { captures, previous }
    }
    /// Prepare while candidate owners are alive; publish only at commit. Mappings
    /// normally replace the candidate root and dictionary with preserved owners.
    pub fn prepare_rebase(&self, mappings: &[(BoxedValue, BoxedValue)]) -> DeferredContentRebase {
        let remap = |value: &WeakValue| {
            let alive = value.upgrade();
            mappings.iter()
                .find(|(from, _)| {
                    alive.as_ref().is_some_and(|value| {
                        match (ValueTypes::as_object(&**value), ValueTypes::as_object(&**from)) {
                            (Some(a), Some(b)) => a.ptr_eq(&b),
                            _ => ValueTypes::identity_equals(Some(value), Some(from)),
                        }
                    })
                })
                .map(|(_, to)| WeakValue::new(to))
                .unwrap_or_else(|| value.clone())
        };
        let entries = self.captures.borrow().iter().filter_map(Weak::upgrade).map(|content| {
            let previous = content.reload_context.borrow().clone();
            let (nodes, root) = previous.as_ref()
                .map(|context| (&context.nodes, &context.root))
                .unwrap_or((&content.parent_resource_nodes, &content.root_object));
            let next = Rc::new(Context {
                nodes: Rc::new(CapturedResourceNodes {
                    nodes: nodes.nodes.iter().map(&remap).collect(),
                    alive: RefCell::new(Weak::new()),
                }),
                root: root.as_ref().map(&remap),
            });
            (content, previous, next)
        }).collect();
        DeferredContentRebase { entries, committed: false }
    }
}
impl Drop for DeferredContentReloadScope {
    fn drop(&mut self) {
        CAPTURES.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

/// Reversible ambient-context change. Dropping after commit leaves the new weak
/// contexts installed; rollback explicitly restores the candidate contexts.
pub struct DeferredContentRebase {
    entries: Vec<(Rc<DeferredContent>, Option<Rc<Context>>, Rc<Context>)>,
    committed: bool,
}
impl DeferredContentRebase {
    pub fn commit(&mut self) {
        for (content, _, next) in &self.entries {
            *content.reload_context.borrow_mut() = Some(next.clone());
        }
        self.committed = true;
    }
    pub fn rollback(&mut self) {
        if self.committed {
            for (content, previous, _) in &self.entries {
                *content.reload_context.borrow_mut() = previous.clone();
            }
            self.committed = false;
        }
    }
}
