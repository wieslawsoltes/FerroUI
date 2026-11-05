//! The per-root lifecycle of `{x:Bind}`: the `Bindings` object WinUI's compiler generates for a
//! page or a template instance (NOTICE.md of the crate: `CSharpPagePass2.tt`).

use super::event::XBindEvent;
use super::root_node::XBindRootNode;
use crate::XamlLoadError;
use ferroui_base::controls::{NameScope, NameScopeRef};
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::{ValueType, ValueTypes, WeakValue};
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::{IDisposable, Subscription};
use ferroui_base::{
    ferro_markup_type, ferro_properties, ferro_static_type, AttachedProperty, BoxedValue, FerroObject, FerroProperty,
    Ref, StyledElement, Visual, WeakRef,
};
use ferroui_controls::templates::{IDataTemplateComponent, XamlBindingHelper};
use ferroui_controls::{Control, WindowBase};
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
    /// The templated parent of the root of a control template instance, of the template's
    /// `TargetType` (step X5).
    ControlTemplate(ValueType),
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
/// The bindings of a template instance are also its [`IDataTemplateComponent`], which the root
/// carries (`XamlBindingHelper.DataTemplateComponent`): a container that drives the instance
/// through it (`ProcessBindings` with phase `0` when it prepares the element, then each phase
/// the previous call returned, `Recycle` before reusing it) reads its bindings phase by phase
/// (`x:Phase`) from the item it gives, and the data context no longer drives the instance. The
/// targets of a later phase are hidden until their phase, and again when the element is recycled
/// (WinUI's `SuspendRendering` and `ResumeRendering`).
///
/// The root of a control template instance (a `ControlTemplate` with `TargetType`) reads its
/// templated parent, and is updated whenever its templated parent changes to a control; it stops
/// tracking when the template is removed. The root of a `Window` document is initialized when
/// the window is activated (WinUI's `Activated`), every other document root when it is attached
/// to a visual tree.
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
    /// The attach to a visual tree or the activation of a window (a document root), the data
    /// context changes (a template instance) or the templated parent changes (a control
    /// template instance) that read the bindings.
    hook: RefCell<Option<Subscription>>,
    /// Whether a container drives the template instance through `IDataTemplateComponent` (WinUI
    /// removes the `DataContextChanged` handler on the first `ProcessBindings`), and the item it
    /// gave.
    container_driven: Cell<bool>,
    container_item: RefCell<Option<BoxedValue>>,
    /// The targets of the bindings of a later phase, hidden until their phase, with the value
    /// that hides them.
    suspended: RefCell<Vec<(WeakRef<FerroObject>, Rc<dyn IDisposable>)>>,
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
    /// template finds the element in that scope, which compiled paths hold weakly. The root of a
    /// data template instance carries the bindings as its template component.
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
            container_driven: Cell::new(false),
            container_item: RefCell::new(None),
            suspended: RefCell::new(Vec::new()),
        });
        object.set_value(Self::bindings_property(), Some(Rc::new(RootOwnership(result.clone()))));
        if let (XBindScope::Template(_) | XBindScope::ControlTemplate(_), Some(name_scope)) = (scope, name_scope) {
            if let Some(element) = object.to_ref().cast::<StyledElement>() {
                if NameScope::get_name_scope(&element).is_none() {
                    NameScope::set_name_scope(&element, Some(name_scope.clone()));
                }
            }
        }
        if let XBindScope::Template(_) = scope {
            if let Some(control) = object.to_ref().cast::<Control>() {
                let component: Rc<dyn IDataTemplateComponent> = result.clone();
                XamlBindingHelper::set_data_template_component(&control, Some(component));
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

    /// After a document populated `root` again, or populated a root that is already live (step
    /// X7: hot reload, a designer, a live root loaded into): what the bindings of the root need
    /// so that they reflect the markup that was just populated.
    ///
    /// - Initialized bindings are read again ([`update`](Self::update)): the bindings the new
    ///   markup added are read, the others read their sources again, `OneTime` included.
    /// - The bindings of a root that is already attached to a visual tree, or of a `Window` that
    ///   is already active, are initialized: no attach (or activation) is coming to do it.
    /// - Otherwise nothing: the attach to a visual tree (or the activation) initializes them, as
    ///   for a document built before it is shown.
    ///
    /// A root without `x:Bind` bindings, and the root of a template instance (its data context
    /// or its container drives it), have nothing to do.
    pub fn update_populated(root: &FerroObject) -> Result<(), XBindErrors> {
        let Some(bindings) = Self::get_for_root(root) else {
            return Ok(());
        };
        if bindings.is_template() {
            return Ok(());
        }
        if bindings.is_initialized() {
            return bindings.update();
        }
        let root = root.to_ref();
        let live = match root.clone().cast::<WindowBase>() {
            Some(window) => window.is_active(),
            None => root.cast::<Visual>().is_some_and(|visual| visual.is_attached_to_visual_tree()),
        };
        match live {
            true => bindings.initialize(),
            false => Ok(()),
        }
    }

    /// Whether the bindings were read and are tracking their sources.
    pub fn is_initialized(&self) -> bool {
        self.initialized.get()
    }

    /// Whether these are the bindings of a template instance: of a data template or of a
    /// control template.
    pub fn is_template(&self) -> bool {
        matches!(self.scope, XBindScope::Template(_) | XBindScope::ControlTemplate(_))
    }

    /// Whether these are the bindings of a control template instance, which read the templated
    /// parent of their root.
    pub fn is_control_template(&self) -> bool {
        matches!(self.scope, XBindScope::ControlTemplate(_))
    }

    /// Whether a container drives the template instance through its template component
    /// ([`IDataTemplateComponent`]) instead of its data context.
    pub fn is_container_driven(&self) -> bool {
        self.container_driven.get()
    }

    /// The number of targets of bindings of a later phase that are hidden until their phase.
    pub fn suspended_count(&self) -> usize {
        self.suspended.borrow().iter().filter(|(target, _)| target.upgrade().is_some()).count()
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
        self.read(None)
    }

    /// Reads the bindings of `phase`, or every binding for `None` (WinUI's `Update_` with the
    /// phase of `ProcessBindings`, or with `NOT_PHASED` for `Update`). The event bindings are
    /// started with the bindings of phase zero.
    fn read(&self, phase: Option<i32>) -> Result<(), XBindErrors> {
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
            if phase.is_some_and(|phase| node.phase() != phase) {
                continue;
            }
            node.refresh();
        }
        if phase.is_none_or(|phase| phase == 0) {
            for event in self.events_snapshot() {
                if !self.is_initialized() {
                    break;
                }
                event.start();
            }
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

    /// What the bindings read: the root of the document, the item of a template instance
    /// (`Ok(None)` while the instance has none, as WinUI's `SetDataRoot` ignores a null item),
    /// or the templated parent of a control template instance (`Ok(None)` while it has none).
    /// The item of a template instance is its data context, or the item its container gave
    /// once the container drives it.
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
                let item = match self.container_driven.get() {
                    true => self.container_item.borrow().clone(),
                    false => root.cast::<StyledElement>().and_then(|root| root.data_context()),
                };
                match item {
                    None => Ok(None),
                    Some(item) if CastTarget::Value(data_type).is_instance(&item) => Ok(Some(item)),
                    Some(_) => Err(format!(
                        "the data context of the template instance is not a '{}', the x:DataType of its template",
                        data_type.name()
                    )),
                }
            }
            XBindScope::ControlTemplate(target_type) => {
                let Some(root) = self.root.upgrade() else {
                    return Err("the root of the template instance has been released".to_string());
                };
                let Some(parent) = root.cast::<StyledElement>().and_then(|root| root.templated_parent()) else {
                    return Ok(None);
                };
                let parent: BoxedValue = Rc::new(parent);
                // The TargetType is a class: the templated parent is of it or of a class deriving
                // from it.
                let target = ValueTypes::class_of(target_type).map_or(CastTarget::Value(target_type), CastTarget::Class);
                match target.is_instance(&parent) {
                    true => Ok(Some(parent)),
                    false => Err(format!(
                        "the templated parent of the template instance is not a '{}', the TargetType of its template",
                        target_type.name()
                    )),
                }
            }
        }
    }

    /// The phase after `current` with bindings, or `-1` (WinUI's `GetNextPhase`).
    fn next_phase(&self, current: i32) -> i32 {
        self.snapshot().iter().map(|node| node.phase()).filter(|phase| *phase > current).min().unwrap_or(-1)
    }

    /// The live targets of the bindings whose phase `select` accepts, each once.
    fn phased_targets(&self, select: impl Fn(i32) -> bool) -> Vec<Ref<FerroObject>> {
        let mut targets: Vec<Ref<FerroObject>> = Vec::new();
        for node in self.snapshot() {
            if !select(node.phase()) {
                continue;
            }
            if let Some(target) = node.target() {
                if !targets.contains(&target) {
                    targets.push(target);
                }
            }
        }
        targets
    }

    /// Hides the targets of the bindings of a later phase until their phase (WinUI's
    /// `XamlBindingHelper.SuspendRendering`): a value of `IsVisible` above the local value, which
    /// resuming removes.
    fn suspend_phased(&self) {
        for target in self.phased_targets(|phase| phase > 0) {
            let suspended = self.suspended.borrow().iter().any(|(known, _)| known.points_to(&target));
            if suspended {
                continue;
            }
            let Some(visual) = target.clone().cast::<Visual>() else {
                continue;
            };
            if let Some(handle) =
                visual.set_value_with_priority(Visual::is_visible_property(), false, BindingPriority::Animation)
            {
                self.suspended.borrow_mut().push((target.downgrade(), handle));
            }
        }
    }

    /// Shows the targets of the bindings of `phase` again (`ResumeRendering`).
    fn resume_phase(&self, phase: i32) {
        let targets = self.phased_targets(|candidate| candidate == phase);
        let mut resumed = Vec::new();
        self.suspended.borrow_mut().retain(|(known, handle)| {
            let Some(target) = known.upgrade() else {
                return false;
            };
            if targets.contains(&target) {
                resumed.push(handle.clone());
                false
            } else {
                true
            }
        });
        for handle in resumed {
            handle.dispose();
        }
    }

    /// The root of a document is initialized when it is attached to a visual tree (WinUI:
    /// `Loading`, `Initialize`), the root of a `Window` document when the window is activated
    /// (`Activated`); a template instance is updated when its data context changes to an item
    /// (`DataContextChangedHandler`), a control template instance when its templated parent
    /// changes to a control (step X5).
    fn hook(&self, root: &FerroObject) {
        let weak = self.this.clone();
        let subscription = match self.scope {
            XBindScope::Root => match root.to_ref().cast::<WindowBase>() {
                Some(window) => Some(window.activated(move || {
                    if let Some(bindings) = weak.upgrade() {
                        // The failures stay available through `diagnostics`.
                        let _ = bindings.initialize();
                    }
                })),
                None => root.to_ref().cast::<Visual>().map(|visual| {
                    visual.attached_to_visual_tree(move |_| {
                        if let Some(bindings) = weak.upgrade() {
                            // The failures stay available through `diagnostics`; an event has
                            // no channel for them.
                            let _ = bindings.initialize();
                        }
                    })
                }),
            },
            XBindScope::Template(_) => root.to_ref().cast::<StyledElement>().map(|element| {
                element.data_context_changed(move || {
                    if let Some(bindings) = weak.upgrade() {
                        if bindings.container_driven.get() || bindings.data_root().is_ok_and(|item| item.is_none()) {
                            return;
                        }
                        let _ = bindings.update();
                    }
                })
            }),
            XBindScope::ControlTemplate(_) => {
                let templated_parent = StyledElement::templated_parent_property().id();
                Some(Subscription::new(root.property_changed(move |change| {
                    if change.property().id() != templated_parent {
                        return;
                    }
                    let Some(bindings) = weak.upgrade() else {
                        return;
                    };
                    // The template is removed: its bindings stop with it.
                    if bindings.data_root().is_ok_and(|parent| parent.is_none()) {
                        bindings.stop_tracking();
                        return;
                    }
                    let _ = bindings.update();
                })))
            }
        };
        *self.hook.borrow_mut() = subscription;
    }
}

/// WinUI's `IDataTemplateComponent` of the generated `Bindings` of a template
/// (`CSharpPagePass2.tt`, `ProcessBindings` and `Recycle`): phase zero sets the item, stops the
/// data context from driving the instance and hides the targets of the later phases; each phase
/// shows its targets and reads its bindings; the result is the next phase with bindings.
impl IDataTemplateComponent for XBindings {
    fn process_bindings(&self, data: Option<BoxedValue>, _item_index: i32, current_phase: i32) -> i32 {
        if current_phase == 0 {
            if let XBindScope::Template(_) = self.scope {
                self.container_driven.set(true);
                // `SetDataRoot` ignores a null item.
                if data.is_some() {
                    *self.container_item.borrow_mut() = data;
                }
                let hook = self.hook.borrow_mut().take();
                drop(hook);
            }
            self.suspend_phased();
        } else {
            self.resume_phase(current_phase);
        }
        // The failures stay available through `diagnostics`.
        let _ = self.read(Some(current_phase));
        self.next_phase(current_phase)
    }

    fn recycle(&self) {
        // `bindingsTracking.ReleaseAllListeners()` and `SuspendRendering` of the phased elements.
        for node in self.snapshot() {
            node.pause();
        }
        self.suspend_phased();
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
