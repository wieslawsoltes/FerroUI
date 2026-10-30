//! The host-owned binding lifecycle; see NOTICE.md for the pinned WinUI source contract.

use super::path_node::RootGate;
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_properties, ferro_static_type, AttachedProperty, BoxedValue, FerroObject, FerroProperty,
    StyledElement, WeakRef,
};
use ferroui_controls::Control;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

/// A binding failure with the original WinUI document location, when migration supplied it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIBindingDiagnostic {
    pub path: String,
    pub message: String,
    pub line: i32,
    pub column: i32,
}

impl fmt::Display for WinUIBindingDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "{}:{}: ", self.line, self.column)?;
        }
        write!(f, "x:Bind '{}': {}", self.path, self.message)
    }
}

/// All failures observed during one explicit initialization or update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIBindingErrors(pub Vec<WinUIBindingDiagnostic>);

impl fmt::Display for WinUIBindingErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            error.fmt(f)?;
        }
        Ok(())
    }
}

impl std::error::Error for WinUIBindingErrors {}

/// The generated `Bindings` object's role for one document or typed template instance.
///
/// Call [`initialize`](Self::initialize) after construction and code-behind initialization.
/// It is idempotent. [`update`](Self::update) rereads every path, including OneTime paths.
/// [`stop_tracking`](Self::stop_tracking) releases source listeners and leaves the current
/// target values in place. A later Initialize resumes the group. The explicit API is necessary:
/// FerroUI's Loaded event runs at a different point from WinUI's Loading event.
///
/// Each template root has its own group. After Initialize its DataContext replacements refresh
/// the typed item bindings, including OneTime paths, as a recycled template receives a new item.
/// Unload handling is opt-in through [`set_stop_on_unload`](Self::set_stop_on_unload).
/// Roots and binding expressions are held weakly; retaining this controller does not retain UI.
pub struct WinUIBindings {
    this: Weak<Self>,
    pub(super) root: WeakRef<FerroObject>,
    gates: RefCell<Vec<Weak<RootGate>>>,
    initialized: Cell<bool>,
    updating: Cell<bool>,
    has_template_bindings: Cell<bool>,
    template_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    unload_token: Cell<Option<RoutedEventHandlerToken>>,
}

/// The root owns this token even when application code retains the controller separately.
/// Dropping the root must release subscriptions of targets that were retained elsewhere.
struct RootOwnership(Rc<WinUIBindings>);
crate::identity_eq!(RootOwnership);

impl Drop for RootOwnership {
    fn drop(&mut self) {
        self.0.stop_tracking();
    }
}

ferro_static_type!(WinUIBindings);

ferro_properties! {
    impl WinUIBindings {
        fn bindings_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<WinUIBindings, FerroObject, _>("Bindings", None)
        }
    }
}

crate::identity_eq!(WinUIBindings);

impl WinUIBindings {
    /// Obtains the group owned by this exact root, creating it if needed.
    pub fn for_root(root: &FerroObject) -> Rc<Self> {
        if let Some(existing) = Self::get_for_root(root) {
            return existing;
        }
        let result = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            root: root.to_weak(),
            gates: RefCell::new(Vec::new()),
            initialized: Cell::new(false),
            updating: Cell::new(false),
            has_template_bindings: Cell::new(false),
            template_subscription: RefCell::new(None),
            unload_token: Cell::new(None),
        });
        root.set_value(
            Self::bindings_property(),
            Some(Rc::new(RootOwnership(result.clone()))),
        );
        result
    }

    /// Reads the existing group without changing the root.
    pub fn get_for_root(root: &FerroObject) -> Option<Rc<Self>> {
        root.get_value(Self::bindings_property()).and_then(|owner| {
            owner
                .downcast_ref::<RootOwnership>()
                .map(|owner| owner.0.clone())
        })
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.get()
    }

    pub fn binding_count(&self) -> usize {
        self.snapshot().len()
    }

    pub fn initialize(&self) -> Result<(), WinUIBindingErrors> {
        if self.is_initialized() {
            self.result()
        } else {
            self.update()
        }
    }

    /// Forces a read of all registered paths. Runtime path/target-conversion failures are
    /// returned together and are also available through [`diagnostics`](Self::diagnostics).
    /// User property getters and callbacks that panic are not suppressed.
    pub fn update(&self) -> Result<(), WinUIBindingErrors> {
        if self.updating.replace(true) {
            return Err(WinUIBindingErrors(vec![WinUIBindingDiagnostic {
                path: String::new(),
                message: "Bindings.Update cannot be called during an update".into(),
                line: 0,
                column: 0,
            }]));
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.updating);
        self.initialized.set(true);
        self.observe_template_item();
        for gate in self.snapshot() {
            // A target callback can StopTracking the entire group.
            if !self.is_initialized() {
                break;
            }
            gate.refresh();
        }
        self.result()
    }

    /// Releases source subscriptions while retaining the last displayed values. It also
    /// suspends template-item replacement tracking until Initialize/Update resumes the group.
    pub fn stop_tracking(&self) {
        self.initialized.set(false);
        let subscription = self.template_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        for gate in self.snapshot() {
            gate.pause();
        }
    }

    /// Enables only unload stopping. The host still chooses the Initialize timing explicitly.
    /// Returns false if the root is not a Control or is already gone.
    pub fn set_stop_on_unload(&self, enabled: bool) -> bool {
        let Some(root) = self.root.upgrade().and_then(|root| root.cast::<Control>()) else {
            return false;
        };
        if enabled && self.unload_token.get().is_none() {
            let weak = self.this.clone();
            self.unload_token.set(Some(root.unloaded(move |_, _| {
                if let Some(group) = weak.upgrade() {
                    group.stop_tracking();
                }
            })));
        } else if !enabled {
            if let Some(token) = self.unload_token.take() {
                root.remove_handler(Control::unloaded_event(), token);
            }
        }
        true
    }

    /// Current failures, including failures that occurred after a OneWay/TwoWay source change
    /// or a template item replacement. Reading diagnostics never evaluates a property getter.
    pub fn diagnostics(&self) -> Vec<WinUIBindingDiagnostic> {
        self.snapshot()
            .into_iter()
            .filter_map(|gate| gate.diagnostic())
            .collect()
    }

    fn result(&self) -> Result<(), WinUIBindingErrors> {
        let errors = self.diagnostics();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(WinUIBindingErrors(errors))
        }
    }

    pub(super) fn register(&self, gate: &Rc<RootGate>, template: bool) {
        {
            let mut gates = self.gates.borrow_mut();
            gates.retain(|gate| gate.strong_count() > 0);
            gates.push(Rc::downgrade(gate));
        }
        if template {
            self.has_template_bindings.set(true);
            self.observe_template_item();
        }
    }

    fn snapshot(&self) -> Vec<Rc<RootGate>> {
        self.gates
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .collect()
    }

    fn observe_template_item(&self) {
        if !self.is_initialized()
            || !self.has_template_bindings.get()
            || self.template_subscription.borrow().is_some()
        {
            return;
        }
        let Some(root) = self
            .root
            .upgrade()
            .and_then(|root| root.cast::<StyledElement>())
        else {
            return;
        };
        let weak = self.this.clone();
        let subscription = root.data_context_changed(move || {
            if let Some(group) = weak.upgrade().filter(|group| group.is_initialized()) {
                // Node errors remain queryable through diagnostics and the framework's normal
                // binding diagnostics. An event callback has no Result return channel.
                let _ = group.update();
            }
        });
        self.template_subscription.replace(Some(subscription));
    }
}

impl Drop for WinUIBindings {
    fn drop(&mut self) {
        if let Some(subscription) = self.template_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(token) = self.unload_token.take() {
            if let Some(root) = self.root.upgrade().and_then(|root| root.cast::<Control>()) {
                root.remove_handler(Control::unloaded_event(), token);
            }
        }
    }
}
