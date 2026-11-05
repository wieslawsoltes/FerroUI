//! The per-root lifecycle of `{x:Bind}`: the `Bindings` object WinUI's compiler generates for a
//! page or a template instance (NOTICE.md of the crate: `CSharpPagePass2.tt`).

use super::event::XBindEvent;
use super::root_node::XBindRootNode;
use crate::XamlLoadError;
use ferroui_base::controls::{NameScope, NameScopeRef};
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::{ValueType, ValueTypes, WeakValue};
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_markup_type, ferro_properties, ferro_static_type, AttachedProperty, BoxedValue, FerroObject, FerroProperty,
    StyledElement, Visual, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

/// Where the bindings of a root read their source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum XBindScope {
    /// The root object of the document.
    Root,
    /// The data context of the root of a template instance, of the template's `x:DataType`.
    Template(ValueType),
}

/// A binding that failed when it was read: its path and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XBindDiagnostic {
    /// The path as the compiler built it (empty for the root source itself).
    pub path: String,
    pub message: String,
}

impl fmt::Display for XBindDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "x:Bind '{}': {}", self.path, self.message)
    }
}

/// The failures of one `initialize` or `update`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XBindErrors(pub Vec<XBindDiagnostic>);

impl fmt::Display for XBindErrors {
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

impl std::error::Error for XBindErrors {}

/// The `{x:Bind}` bindings of one root: a document root or the root of one template instance.
///
/// It is the object WinUI's compiler generates as the `Bindings` of a page, with the same
/// lifecycle:
///
/// - [`initialize`](Self::initialize) reads every binding once, unless the bindings are already
///   initialized. The root of a document calls it when it is attached to a visual tree, the
///   counterpart of WinUI's `Loading`: after construction and after code-behind has set what the
///   bindings read. A root that is not a visual, or that is already in a tree when its document
///   is built, is initialized by calling it.
/// - [`update`](Self::update) reads every binding again, `OneTime` bindings included.
/// - [`stop_tracking`](Self::stop_tracking) releases what the `OneWay` and `TwoWay` bindings
///   listen to and leaves the targets as they are; the bindings are uninitialized again, and the
///   next attach (or `initialize`) reads them.
///
/// The event bindings of the root (`Click="{x:Bind OnClick}"`) subscribe their handlers when the
/// bindings are initialized or updated and release them when the bindings stop tracking; a
/// handler reads the path of its method when the event is raised.
///
/// The root of a template instance (a `DataTemplate` with `x:DataType`) is updated whenever its
/// data context changes to an item, as the generated `DataContextChangedHandler`: a recycled
/// container reads its new item, `OneTime` bindings included. A null data context reads nothing.
///
/// [`XBindings::get_for_root`] finds the object of a root; the first `x:Bind` of a root creates
/// it. The root owns it; it holds the root and its bindings weakly.
pub struct XBindings {
    this: Weak<Self>,
    root: WeakRef<FerroObject>,
    /// The root as the document gave it, in the handle type the typed accessors of the paths take.
    root_value: WeakValue,
    scope: XBindScope,
    nodes: RefCell<Vec<Weak<XBindRootNode>>>,
    /// The event bindings, held here: the handler on the target holds its binding weakly.
    events: RefCell<Vec<Rc<XBindEvent>>>,
    initialized: Cell<bool>,
    updating: Cell<bool>,
    /// The attach to a visual tree (a document root) or the data context changes (a template
    /// instance) that read the bindings.
    hook: RefCell<Option<Subscription>>,
}

/// What the root holds: dropping the root stops the bindings, also when the application keeps
/// the object of its bindings.
struct RootOwnership(Rc<XBindings>);

crate::identity_eq!(RootOwnership);

impl Drop for RootOwnership {
    fn drop(&mut self) {
        self.0.stop_tracking();
    }
}

ferro_static_type!(XBindings);

ferro_properties! {
    impl XBindings {
        fn bindings_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<XBindings, FerroObject, _>("Bindings", None)
        }
    }
}

crate::identity_eq!(XBindings);

impl XBindings {
    /// The bindings of `root` (a boxed root object), created on the first `x:Bind` of the root.
    ///
    /// The root of a template instance keeps `name_scope`, the name scope of the template, by
    /// holding it as its own (`NameScope::set_name_scope`, as a document root holds its scope):
    /// the template returns only its root, and a path that starts at a named element of the
    /// template finds the element in that scope, which compiled paths hold weakly.
    pub(crate) fn for_root_value(
        root: &BoxedValue,
        scope: XBindScope,
        name_scope: Option<&NameScopeRef>,
    ) -> Result<Rc<Self>, XamlLoadError> {
        let object = ValueTypes::as_object(&**root).ok_or_else(|| {
            XamlLoadError::with_message(format!(
                "x:Bind requires the root of its document to be an object of the object model; '{}' is not",
                ValueTypes::type_full_name(ValueType::of_value(&**root))
            ))
        })?;
        if let Some(existing) = Self::get_for_root(&object) {
            if existing.scope != scope {
                return Err(XamlLoadError::with_message(
                    "x:Bind: the bindings of this root read another source (a document root and a template instance root are distinct objects)",
                ));
            }
            return Ok(existing);
        }
        let result = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            root: object.to_weak(),
            root_value: WeakValue::new(root),
            scope,
            nodes: RefCell::new(Vec::new()),
            events: RefCell::new(Vec::new()),
            initialized: Cell::new(false),
            updating: Cell::new(false),
            hook: RefCell::new(None),
        });
        object.set_value(Self::bindings_property(), Some(Rc::new(RootOwnership(result.clone()))));
        if let (XBindScope::Template(_), Some(name_scope)) = (scope, name_scope) {
            if let Some(element) = object.to_ref().cast::<StyledElement>() {
                if NameScope::get_name_scope(&element).is_none() {
                    NameScope::set_name_scope(&element, Some(name_scope.clone()));
                }
            }
        }
        result.hook(&object);
        Ok(result)
    }

    /// The bindings of `root`, if an `x:Bind` of its document (or template) created them.
    pub fn get_for_root(root: &FerroObject) -> Option<Rc<Self>> {
        root.get_value(Self::bindings_property())
            .and_then(|owner| owner.downcast_ref::<RootOwnership>().map(|owner| owner.0.clone()))
    }

    /// Whether the bindings were read and are tracking their sources.
    pub fn is_initialized(&self) -> bool {
        self.initialized.get()
    }

    /// Whether these are the bindings of a template instance.
    pub fn is_template(&self) -> bool {
        matches!(self.scope, XBindScope::Template(_))
    }

    /// The number of live property bindings of the root.
    pub fn binding_count(&self) -> usize {
        self.snapshot().len()
    }

    /// The number of event bindings of the root.
    pub fn event_binding_count(&self) -> usize {
        self.events.borrow().len()
    }

    /// The number of event bindings whose handler is subscribed to its event.
    pub fn subscribed_event_count(&self) -> usize {
        self.events_snapshot().iter().filter(|event| event.is_subscribed()).count()
    }

    /// `Initialize`: reads the bindings if they are not initialized.
    pub fn initialize(&self) -> Result<(), XBindErrors> {
        if self.is_initialized() {
            self.result()
        } else {
            self.update()
        }
    }

    /// `Update`: reads every binding again, `OneTime` bindings included. The failures are
    /// returned together and stay available through [`diagnostics`](Self::diagnostics).
    pub fn update(&self) -> Result<(), XBindErrors> {
        if self.updating.replace(true) {
            return Err(XBindErrors(vec![XBindDiagnostic {
                path: String::new(),
                message: "Bindings.Update cannot be called during an update".into(),
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
        for node in self.snapshot() {
            // A target callback can stop the tracking of the whole root.
            if !self.is_initialized() {
                break;
            }
            node.refresh();
        }
        for event in self.events_snapshot() {
            if !self.is_initialized() {
                break;
            }
            event.start();
        }
        self.result()
    }

    /// `StopTracking`: releases the sources the bindings listen to and keeps the values of the
    /// targets. The bindings are uninitialized; the next `Initialize` (or attach) reads them.
    pub fn stop_tracking(&self) {
        self.initialized.set(false);
        for node in self.snapshot() {
            node.pause();
        }
        for event in self.events_snapshot() {
            event.stop();
        }
    }

    /// The current failures, including those of a source change of a `OneWay` or `TwoWay`
    /// binding. Reading them evaluates nothing.
    pub fn diagnostics(&self) -> Vec<XBindDiagnostic> {
        let mut diagnostics: Vec<XBindDiagnostic> =
            self.snapshot().into_iter().filter_map(|node| node.diagnostic()).collect();
        diagnostics.extend(self.events_snapshot().into_iter().filter_map(|event| event.diagnostic()));
        diagnostics
    }

    fn result(&self) -> Result<(), XBindErrors> {
        let errors = self.diagnostics();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(XBindErrors(errors))
        }
    }

    pub(super) fn register(&self, node: &Rc<XBindRootNode>) {
        let mut nodes = self.nodes.borrow_mut();
        nodes.retain(|node| node.strong_count() > 0);
        nodes.push(Rc::downgrade(node));
    }

    fn snapshot(&self) -> Vec<Rc<XBindRootNode>> {
        self.nodes.borrow().iter().filter_map(Weak::upgrade).collect()
    }

    /// Adds an event binding; it subscribes its handler at once when the bindings are
    /// initialized (a document built into a live root).
    pub(super) fn register_event(&self, event: Rc<XBindEvent>) {
        self.events.borrow_mut().push(event.clone());
        if self.is_initialized() {
            event.start();
        }
    }

    fn events_snapshot(&self) -> Vec<Rc<XBindEvent>> {
        self.events.borrow().clone()
    }

    /// What the bindings read: the root of the document, or the item of a template instance
    /// (`Ok(None)` while the instance has none, as WinUI's `SetDataRoot` ignores a null item).
    pub(super) fn data_root(&self) -> Result<Option<BoxedValue>, String> {
        match self.scope {
            XBindScope::Root => match self.root_value.upgrade() {
                Some(root) => Ok(Some(root)),
                None => Err("the root of the document has been released".to_string()),
            },
            XBindScope::Template(data_type) => {
                let Some(root) = self.root.upgrade() else {
                    return Err("the root of the template instance has been released".to_string());
                };
                match root.cast::<StyledElement>().and_then(|root| root.data_context()) {
                    None => Ok(None),
                    Some(item) if CastTarget::Value(data_type).is_instance(&item) => Ok(Some(item)),
                    Some(_) => Err(format!(
                        "the data context of the template instance is not a '{}', the x:DataType of its template",
                        data_type.name()
                    )),
                }
            }
        }
    }

    /// The root of a document is initialized when it is attached to a visual tree (WinUI:
    /// `Loading`, `Initialize`); a template instance is updated when its data context changes
    /// to an item (`DataContextChangedHandler`).
    fn hook(&self, root: &FerroObject) {
        let weak = self.this.clone();
        let subscription = match self.scope {
            XBindScope::Root => root.to_ref().cast::<Visual>().map(|visual| {
                visual.attached_to_visual_tree(move |_| {
                    if let Some(bindings) = weak.upgrade() {
                        // The failures stay available through `diagnostics`; an event has no
                        // channel for them.
                        let _ = bindings.initialize();
                    }
                })
            }),
            XBindScope::Template(_) => root.to_ref().cast::<StyledElement>().map(|element| {
                element.data_context_changed(move || {
                    if let Some(bindings) = weak.upgrade() {
                        if bindings.data_root().is_ok_and(|item| item.is_none()) {
                            return;
                        }
                        let _ = bindings.update();
                    }
                })
            }),
        };
        *self.hook.borrow_mut() = subscription;
    }
}

type This = Rc<XBindings>;

ferro_markup_type!(class XBindings {
    this: Rc<XBindings>,
    handles: [Rc<XBindings>, Option<Rc<XBindings>>],
    properties: [
        IsInitialized: bool { get: |this: &This| this.is_initialized() },
    ],
    methods: [
        try fn Initialize() => |this: &This| this.initialize(),
        try fn Update() => |this: &This| this.update(),
        fn StopTracking() => |this: &This| this.stop_tracking(),
    ],
});
