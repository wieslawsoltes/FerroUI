//! The root source of an `{x:Bind}` expression: the first node, before the nodes the compiled
//! path builds, which gives them the root of the document (or the item of a template instance)
//! only while the [`XBindings`] of the root are initialized.

use super::{XBindDiagnostic, XBindings};
use ferroui_base::data::core::expression_nodes::{ExpressionNode, NodeSource, NodeState};
use ferroui_base::data::core::UntypedBindingExpression;
use ferroui_base::data::{BindingError, BindingErrorType};
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(super) struct XBindRootNode {
    state: NodeState,
    bindings: Weak<XBindings>,
    /// The path, for diagnostics.
    path: String,
    /// Whether the binding has a `FallbackValue`: a path that cannot be read gives the target
    /// the fallback, which is what WinUI's generated code does instead of failing, so it is not
    /// a diagnostic.
    has_fallback_value: bool,
    /// Off while the bindings are stopped: a source change of the expression publishes nothing.
    enabled: Cell<bool>,
    error: RefCell<Option<String>>,
    /// The nodes of the path after this one, whose sources a refresh or a stop clears.
    path_nodes: RefCell<Vec<Weak<dyn ExpressionNode>>>,
}

impl XBindRootNode {
    pub fn new(bindings: &Rc<XBindings>, path: String, has_fallback_value: bool) -> Rc<Self> {
        Rc::new(Self {
            // The root is held weakly, as the elements a binding locates are: the root owns the
            // targets, which own their expressions.
            state: NodeState::locating_an_element(),
            bindings: Rc::downgrade(bindings),
            path,
            has_fallback_value,
            enabled: Cell::new(bindings.is_initialized()),
            error: RefCell::new(None),
            path_nodes: RefCell::new(Vec::new()),
        })
    }

    pub fn set_path_nodes(&self, nodes: &[Rc<dyn ExpressionNode>]) {
        *self.path_nodes.borrow_mut() = nodes.iter().map(Rc::downgrade).collect();
    }

    /// Reads the path again from the root source (`Update`).
    pub fn refresh(&self) {
        self.enabled.set(true);
        self.clear_path_sources();
        if self.state.owner().is_some_and(|owner| owner.base().is_running()) {
            self.publish_source();
        }
    }

    /// Releases what the path listens to (`StopTracking`); the target keeps its value.
    pub fn pause(&self) {
        self.enabled.set(false);
        self.clear_path_sources();
    }

    fn clear_path_sources(&self) {
        let nodes: Vec<_> = self.path_nodes.borrow().iter().filter_map(Weak::upgrade).collect();
        // An unset source releases the listeners of a node without publishing a value.
        for node in nodes {
            node.set_source(NodeSource::Unset, None);
        }
    }

    fn publish_source(&self) {
        let Some(bindings) = self.bindings.upgrade().filter(|bindings| bindings.is_initialized()) else {
            return;
        };
        if !self.enabled.get() {
            return;
        }
        self.error.borrow_mut().take();
        match bindings.data_root() {
            Ok(Some(source)) => self.state.set_value(Some(source), None),
            // A template instance without an item yet: nothing to read.
            Ok(None) => {}
            Err(message) => self.fail(&message),
        }
    }

    fn fail(&self, message: &str) {
        *self.error.borrow_mut() = Some(message.to_string());
        self.state.set_error(message);
    }

    /// The failure of the binding: of its root source, or of its expression (a path that could
    /// not be read or a value that could not be converted to the target), unless a fallback
    /// value stands for the expression.
    pub fn diagnostic(&self) -> Option<XBindDiagnostic> {
        let message = self.error.borrow().clone().or_else(|| {
            self.state
                .owner()
                .filter(|_| !self.has_fallback_value)
                .filter(|owner| owner.error_type() != BindingErrorType::None)
                .map(|_| "the path could not be read or its value converted to the target property".to_string())
        })?;
        Some(XBindDiagnostic { path: self.path.clone(), message })
    }
}

impl ExpressionNode for XBindRootNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn on_source_changed(&self, _: Option<&BoxedValue>, _: Option<&BindingError>) {
        self.publish_source();
    }

    // A OneTime expression reads its leaf again only after its data context node changed: the
    // root source is that node, so an `Update` reads a OneTime binding again.
    fn is_data_context_node(&self) -> bool {
        true
    }
}
