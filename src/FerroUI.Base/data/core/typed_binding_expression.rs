use super::{IPropertyInfo, SinkRef, Value, ValueType, ValueTypes};
use crate::data::core::plugins::property_value_type;
use crate::data::core::PropertyKind;
use crate::data::model::INotifyPropertyChanged;
use crate::data::{BindingError, BindingExpressionBase, BindingMode, BindingPriority, BindingValueType};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::property_store::{ImmediateValueFrame, IValueEntry};
use crate::reactive::{IDisposable, Subscription};
use crate::utilities::BooleanBoxes;
use crate::{AnyValue, BoxedValue, FerroObject, FerroProperty, PropertyValue, StyledElement, WeakRef};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The getter of a typed property description.
enum TypedGetter<TSource: 'static, TValue: PropertyValue> {
    Infallible(Rc<dyn Fn(&TSource) -> TValue>),
    /// A getter that can fail: the equivalent of a getter that throws.
    Fallible(Rc<dyn Fn(&TSource) -> Result<TValue, BindingError>>),
    /// A getter that takes the shared handle of the owner (the accessors of
    /// markup metadata).
    Handle(HandleGetter<TSource, TValue>),
}

/// The setter of a typed property description.
enum TypedSetter<TSource: 'static, TValue: PropertyValue> {
    Plain(Rc<dyn Fn(&TSource, TValue)>),
    /// A setter that takes the shared handle of the owner.
    Handle(HandleSetter<TSource, TValue>),
}

/// A getter that takes the shared handle of the owner and can fail.
pub type HandleGetter<TSource, TValue> = Rc<dyn Fn(&Rc<TSource>) -> Result<TValue, BindingError>>;
/// A setter that takes the shared handle of the owner.
pub type HandleSetter<TSource, TValue> = Rc<dyn Fn(&Rc<TSource>, TValue)>;

#[cold]
fn needs_handle(name: &str) -> BindingError {
    BindingError::message(format!("Property {name} is accessed through the shared handle of its owner."))
}

/// The owner behind an untyped value: the box is the shared object, or
/// holds a handle of it (the form in which an items control gives its
/// items to their containers as data context), or an object that is
/// assignable to the owner type. As the typed expression reads its
/// source.
fn owner_of<TSource: 'static>(target: &BoxedValue) -> Option<Rc<TSource>> {
    let any: Rc<dyn Any> = target.clone();
    any.downcast::<TSource>().ok().or_else(|| {
        ValueTypes::try_cast(target, ValueType::of::<Rc<TSource>>())
            .and_then(|cast| cast.downcast_ref::<Rc<TSource>>().cloned())
    })
}

/// Converts an untyped value to the type of the property (null to the
/// null of a nullable property type).
fn value_from_untyped<TValue: PropertyValue>(property_name: &str, value: Option<&BoxedValue>) -> Result<TValue, BindingError> {
    let typed = match value {
        Some(_) => Value::<TValue>::from_untyped(value),
        None => ValueTypes::null_value(ValueType::of::<TValue>()).and_then(|v| v.downcast_ref::<TValue>().cloned()),
    };
    typed.ok_or_else(|| conversion_error(property_name, value, ValueType::of::<TValue>()))
}

#[cold]
fn conversion_error(property_name: &str, value: Option<&BoxedValue>, target: ValueType) -> BindingError {
    BindingError::message(format!(
        "Object of type '{}' cannot be converted to type '{}' for property '{}'.",
        value.map_or("null", |v| v.type_name()), target, property_name
    ))
}

/// A property description with typed accessors: values are read and written
/// without boxing.
pub struct TypedClrPropertyInfo<TSource: 'static, TValue: PropertyValue> {
    name: Box<str>,
    getter: Option<TypedGetter<TSource, TValue>>,
    setter: Option<TypedSetter<TSource, TValue>>,
    inpc: Option<fn(&TSource) -> &dyn INotifyPropertyChanged>,
}

impl<TSource: 'static, TValue: PropertyValue> TypedClrPropertyInfo<TSource, TValue> {
    pub fn new(
        name: &str,
        getter: Option<Rc<dyn Fn(&TSource) -> TValue>>,
        setter: Option<Rc<dyn Fn(&TSource, TValue)>>,
    ) -> Self {
        Self {
            name: name.into(),
            getter: getter.map(TypedGetter::Infallible),
            setter: setter.map(TypedSetter::Plain),
            inpc: None,
        }
    }

    /// Creates a property description whose accessors take the shared
    /// handle of the owner (`Rc<TSource>`), as the accessors declared in
    /// markup metadata do. Such a description is read and written through
    /// [`try_get_from`](Self::try_get_from) / [`set_on`](Self::set_on) and
    /// the boxed forms of [`IPropertyInfo`]; the forms that are given only a
    /// view of the owner fail.
    pub fn from_handle_accessors(
        name: &str,
        getter: Option<HandleGetter<TSource, TValue>>,
        setter: Option<HandleSetter<TSource, TValue>>,
    ) -> Self {
        Self {
            name: name.into(),
            getter: getter.map(TypedGetter::Handle),
            setter: setter.map(TypedSetter::Handle),
            inpc: None,
        }
    }

    /// Creates a property description whose getter can fail. An `Err` is the
    /// equivalent of the getter throwing: a binding logs the error and has no
    /// value.
    pub fn new_fallible(
        name: &str,
        getter: impl Fn(&TSource) -> Result<TValue, BindingError> + 'static,
        setter: Option<Rc<dyn Fn(&TSource, TValue)>>,
    ) -> Self {
        Self {
            name: name.into(),
            getter: Some(TypedGetter::Fallible(Rc::new(getter))),
            setter: setter.map(TypedSetter::Plain),
            inpc: None,
        }
    }

    /// States that the owner raises property change notifications.
    pub fn notifying(mut self) -> Self
    where
        TSource: INotifyPropertyChanged,
    {
        self.inpc = Some(|s| s as &dyn INotifyPropertyChanged);
        self
    }

    /// Reads the property. A failing getter panics, as a throwing getter
    /// does for a caller that does not handle the failure: use
    /// [`try_get_typed`](Self::try_get_typed) for properties created with
    /// [`new_fallible`](Self::new_fallible).
    pub fn get_typed(&self, target: &TSource) -> TValue {
        match self.try_get_typed(target) {
            Ok(value) => value,
            Err(e) => panic!("Error getting '{}': {}", self.name, e),
        }
    }

    /// Reads the property. An `Err` is the equivalent of the getter throwing.
    pub fn try_get_typed(&self, target: &TSource) -> Result<TValue, BindingError> {
        match &self.getter {
            Some(TypedGetter::Infallible(getter)) => Ok(getter(target)),
            Some(TypedGetter::Fallible(getter)) => getter(target),
            Some(TypedGetter::Handle(_)) => Err(needs_handle(&self.name)),
            None => panic!("Property {} doesn't have a getter", self.name),
        }
    }

    /// Reads the property of an owner known by its shared handle: works for
    /// every kind of getter.
    pub fn try_get_from(&self, target: &Rc<TSource>) -> Result<TValue, BindingError> {
        match &self.getter {
            Some(TypedGetter::Handle(getter)) => getter(target),
            _ => self.try_get_typed(target),
        }
    }

    /// Writes the property. A setter that takes the shared handle of the
    /// owner ([`from_handle_accessors`](Self::from_handle_accessors)) cannot
    /// be called with a view of it: use [`set_on`](Self::set_on).
    pub fn set_typed(&self, target: &TSource, value: TValue) {
        match &self.setter {
            Some(TypedSetter::Plain(setter)) => setter(target, value),
            Some(TypedSetter::Handle(_)) => panic!("{}", needs_handle(&self.name)),
            None => panic!("Property {} doesn't have a setter", self.name),
        }
    }

    /// Writes the property of an owner known by its shared handle: works
    /// for every kind of setter.
    pub fn set_on(&self, target: &Rc<TSource>, value: TValue) {
        match &self.setter {
            Some(TypedSetter::Handle(setter)) => setter(target, value),
            _ => self.set_typed(target, value),
        }
    }

    pub(crate) fn notifier<'a>(&self, source: &'a TSource) -> Option<&'a dyn INotifyPropertyChanged> {
        self.inpc.map(|f| f(source))
    }
}

impl<TSource: 'static, TValue: PropertyValue> IPropertyInfo for TypedClrPropertyInfo<TSource, TValue> {
    fn name(&self) -> &str {
        &self.name
    }

    fn get(&self, target: &dyn AnyValue) -> Option<BoxedValue> {
        self.try_get(target).ok().flatten()
    }

    fn try_get(&self, target: &dyn AnyValue) -> Result<Option<BoxedValue>, BindingError> {
        let Some(target) = target.downcast_ref::<TSource>() else { return Ok(None) };
        // A nullable value reads as null or as its contents, as the values of
        // every other property description do.
        Ok(ValueTypes::normalize(BooleanBoxes::box_value(&self.try_get_typed(target)?)))
    }

    fn set(&self, target: &dyn AnyValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        let target = target
            .downcast_ref::<TSource>()
            .ok_or_else(|| BindingError::message("The property owner is of the wrong type."))?;
        if matches!(self.setter, Some(TypedSetter::Handle(_))) {
            return Err(needs_handle(&self.name));
        }
        if self.setter.is_none() {
            return Err(BindingError::message(format!("Property {} doesn't have a setter", self.name)));
        }
        self.set_typed(target, value_from_untyped(&self.name, value)?);
        Ok(())
    }

    fn get_boxed(&self, target: &BoxedValue) -> Option<BoxedValue> {
        self.try_get_boxed(target).ok().flatten()
    }

    fn try_get_boxed(&self, target: &BoxedValue) -> Result<Option<BoxedValue>, BindingError> {
        let Some(owner) = owner_of::<TSource>(target) else { return Ok(None) };
        Ok(ValueTypes::normalize(BooleanBoxes::box_value(&self.try_get_from(&owner)?)))
    }

    fn set_boxed(&self, target: &BoxedValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        if self.setter.is_none() {
            return Err(BindingError::message(format!("Property {} doesn't have a setter", self.name)));
        }
        let owner = owner_of::<TSource>(target)
            .ok_or_else(|| BindingError::message("The property owner is of the wrong type."))?;
        self.set_on(&owner, value_from_untyped(&self.name, value)?);
        Ok(())
    }

    fn can_set(&self) -> bool {
        self.setter.is_some()
    }

    fn can_get(&self) -> bool {
        self.getter.is_some()
    }

    fn property_type(&self) -> ValueType {
        ValueType::of::<TValue>()
    }
}

/// Target-side state shared by all typed binding expressions.
struct BindingTarget {
    property: Cell<Option<&'static FerroProperty>>,
    priority: Cell<BindingPriority>,
    casts_value: Cell<bool>,
    writing_value: Cell<bool>,
    sink: RefCell<Option<SinkRef>>,
    frame: RefCell<Option<Weak<ImmediateValueFrame>>>,
    owner: RefCell<Option<WeakRef<FerroObject>>>,
}

impl BindingTarget {
    fn new(priority: BindingPriority) -> Self {
        Self {
            property: Cell::new(None),
            priority: Cell::new(priority),
            casts_value: Cell::new(false),
            writing_value: Cell::new(false),
            sink: RefCell::new(None),
            frame: RefCell::new(None),
            owner: RefCell::new(None),
        }
    }

    #[inline(always)]
    fn owner(&self) -> Option<crate::Ref<FerroObject>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn attach(
        &self,
        value_type: ValueType,
        sink: SinkRef,
        frame: Option<Weak<ImmediateValueFrame>>,
        target: &FerroObject,
        target_property: &'static FerroProperty,
        priority: BindingPriority,
    ) {
        if self.sink.borrow().is_some() {
            panic!("TypedBindingExpression was already attached.");
        }
        if !target.is::<StyledElement>() {
            panic!("TypedBindingExpression may only target StyledElements");
        }
        if self.property.get().is_some_and(|p| p.id() != target_property.id()) {
            panic!("TypedBindingExpression was already attached to a different property.");
        }
        let target_type = property_value_type(target_property);
        if !ValueTypes::is_assignable(value_type, target_type) {
            panic!(
                "TypedBindingExpression of type '{}' cannot be bound to a property of type '{}'.",
                value_type.name(),
                target_property.property_type_name()
            );
        }
        self.sink.replace(Some(sink));
        self.frame.replace(frame);
        self.owner.replace(Some(target.to_weak()));
        self.property.set(Some(target_property));
        self.casts_value.set(target_type != value_type);
        self.priority.set(priority);
    }

    fn log(&self, description: &str, error: &str) {
        let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::BINDING) else { return };
        let Some(target) = self.owner() else { return };
        let property = self.property.get().map_or("(unknown)", |p| p.name());
        let target: &FerroObject = &target;
        log.log_with_values(
            Some(target as &dyn Any),
            "An error occurred binding {Property} to {Expression}: {Message}",
            &[&property, &description, &error],
        );
    }

    #[inline(always)]
    fn publish(&self, sink: SinkRef, this: Rc<dyn BindingExpressionBase>) {
        struct Restore<'a>(&'a Cell<bool>, bool);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }
        let _restore = Restore(&self.writing_value, self.writing_value.replace(true));
        sink.on_changed(&this, true, false);
    }

    fn complete(
        &self,
        sink: Option<SinkRef>,
        this: Option<Rc<dyn BindingExpressionBase>>,
        frame: Option<Weak<ImmediateValueFrame>>,
    ) {
        if let (Some(sink), Some(this)) = (sink, this) {
            sink.on_completed(&this);
        }
        if let (Some(frame), Some(property)) = (frame.and_then(|f| f.upgrade()), self.property.get()) {
            frame.on_entry_disposed(property);
        }
    }
}

/// The cold operations of a binding (start, stop, disposal, errors) do not depend on the model
/// or its property's Rust type, and are shared. They call back to the concrete expression only
/// for what needs the typed source or payload. Producing a value is not among them: that path
/// runs for every update and stays typed in the expression, from the change notification to
/// the sink.
trait BindingHost: BindingExpressionBase {
    fn expression(&self) -> Option<Rc<dyn BindingExpressionBase>>;
    fn subscribe_target(&self, target: &FerroObject) -> Subscription;
    fn refresh_source(&self, data_context: Option<BoxedValue>);
    fn disconnect_source(&self);
    fn write_target_change(&self, change: &crate::FerroPropertyChangedEventArgs<'_>);
}

impl BindingTarget {
    /// Whether a target change writes the value of the notification rather than rereading the
    /// target. Out of the typed write, which is compiled for every value type.
    #[inline(never)]
    fn writeback_uses_change_value(&self, e: &crate::FerroPropertyChangedEventArgs<'_>) -> bool {
        e.property()
            .get_metadata_for(e.sender())
            .binding_writeback_uses_change_value()
            .unwrap_or(!self.casts_value.get())
    }
}

struct BindingController {
    target: BindingTarget,
    mode: BindingMode,
    default_priority: BindingPriority,
    is_running: Cell<bool>,
    produce_value: Cell<bool>,
    should_update_one_time_binding_target: Cell<bool>,
    target_subscription: RefCell<Option<Subscription>>,
    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    reload_paused: Cell<bool>,
    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    reload_pending: Cell<bool>,
}

impl BindingController {
    fn start_core(&self, host: &dyn BindingHost) {
        let (Some(target), Some(_)) = (self.target.owner(), self.target.property.get()) else { return };
        self.target_subscription.replace(Some(host.subscribe_target(&target)));
        let data_context = target.downcast_ref::<StyledElement>().and_then(|e| e.data_context());
        host.refresh_source(data_context);
    }

    fn stop_core(&self, host: &dyn BindingHost) {
        let subscription = self.target_subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
        if self.target.owner().is_some() {
            host.refresh_source(None);
        } else {
            host.disconnect_source();
        }
    }

    /// Runs for every change of a property of the target: inlined into the typed handler, where
    /// the host is known and the call back to it is direct.
    #[inline]
    fn on_target_property_changed(&self, host: &dyn BindingHost, e: &crate::FerroPropertyChangedEventArgs<'_>) {
        if e.property().id() == StyledElement::data_context_property().id() {
            self.data_context_changed(host);
        } else if self.target.property.get().is_some_and(|p| p.id() == e.property().id()) {
            // Don't write back to the source if this change is the binding
            // pushing the source value to the target.
            if !self.target.writing_value.get()
                && matches!(self.mode, BindingMode::TwoWay | BindingMode::OneWayToSource)
            {
                host.write_target_change(e);
            }
        }
    }

    #[inline(never)]
    fn data_context_changed(&self, host: &dyn BindingHost) {
        let data_context =
            self.target.owner().and_then(|t| t.downcast_ref::<StyledElement>().and_then(|e| e.data_context()));
        host.refresh_source(data_context);
    }

    #[cold]
    fn log_get_error(&self, name: &str, error: &BindingError) {
        self.target.log(name, &format!("Error getting '{}': {}", name, error));
    }

    fn dispose(&self, host: &dyn BindingHost) {
        if self.target.sink.borrow().is_none() {
            return;
        }
        // Clear the sink before stopping so that the unsubscribe doesn't push
        // a final value to a value store that is about to clear this entry.
        let sink = self.target.sink.borrow_mut().take();
        let frame = self.target.frame.borrow_mut().take();
        self.stop_core(host);
        self.is_running.set(false);
        let this = host.expression();
        self.target.complete(sink, this, frame);
    }

    /// Starts a stopped expression. The check that it is running stays in the expression: every
    /// read of the value makes it.
    #[inline(never)]
    fn start_stopped(&self, host: &dyn BindingHost, produce_value: bool) {
        self.is_running.set(true);
        self.produce_value.set(produce_value);
        self.start_core(host);
        self.produce_value.set(true);
    }
}

/// Resolving an owner and replacing its notification subscription depend on
/// the model type alone. Return the strong handle to the typed caller so it
/// survives the initial read or write, including a reentrant notification.
struct SourceSlot<TSource> {
    weak: RefCell<Option<Weak<TSource>>>,
    token: Cell<Option<u64>>,
}

impl<TSource: 'static> SourceSlot<TSource> {
    fn get(&self) -> Option<Rc<TSource>> {
        self.weak.borrow().as_ref().and_then(Weak::upgrade)
    }

    fn unsubscribe(&self, notifier: Option<fn(&TSource) -> &dyn INotifyPropertyChanged>) {
        if let Some(token) = self.token.take() {
            if let Some(old) = self.get() {
                if let Some(inpc) = notifier.map(|f| f(&old)) {
                    inpc.property_changed().remove(token);
                }
            }
        }
    }

    fn replace(
        &self,
        controller: &BindingController,
        name: &str,
        notifier: Option<fn(&TSource) -> &dyn INotifyPropertyChanged>,
        data_context: Option<&BoxedValue>,
        subscribe: &dyn Fn(&dyn INotifyPropertyChanged) -> u64,
    ) -> Option<Rc<TSource>> {
        let source = data_context.and_then(owner_of::<TSource>);
        if let (Some(d), None) = (data_context, &source) {
            controller.target.log(name, &format!(
                "Could not convert DataContext of type '{}' to '{}'.",
                d.type_name(),
                std::any::type_name::<TSource>()
            ));
        }

        self.unsubscribe(notifier);
        self.weak.replace(source.as_ref().map(Rc::downgrade));
        controller.should_update_one_time_binding_target.set(true);

        if let Some(source) = &source {
            if let Some(inpc) = notifier.map(|f| f(source)) {
                self.token.set(Some(subscribe(inpc)));
            }
        }

        source
    }
}

/// Storage and property-store conversions are specialized only on the value
/// type. The model stays in the expression and is never boxed for a change.
struct TypedValueSlot<TValue>(RefCell<Option<TValue>>);

impl<TValue: PropertyValue> TypedValueSlot<TValue> {
    #[inline]
    fn try_get_value(&self, target: &BindingTarget, out: &mut dyn Any) -> bool {
        // A value that has to be cast to the type of the target property is
        // read through `get_value_boxed`.
        if target.casts_value.get() {
            return false;
        }
        match (out.downcast_mut::<Option<TValue>>(), &*self.0.borrow()) {
            (Some(out), Some(value)) => {
                *out = Some(value.clone());
                true
            }
            _ => false,
        }
    }

    fn get_value_boxed(&self, target: &BindingTarget) -> Option<BoxedValue> {
        // A boolean is boxed as its cached box, so that reading it into an
        // untyped target property does not allocate per read.
        let value = self.0.borrow().as_ref().map(BooleanBoxes::box_value)?;
        match target.property.get() {
            // The store requires a value of exactly the target property type.
            Some(property) if target.casts_value.get() => ValueTypes::try_cast(&value, property_value_type(property)),
            _ => Some(value),
        }
    }

    fn try_get_target_value(target_state: &BindingTarget) -> Option<TValue> {
        let (Some(property), Some(target)) = (target_state.property.get(), target_state.owner()) else {
            return None;
        };
        // Source updates in one-way-to-source mode, target casts, or a property
        // explicitly requesting effective-value writeback all reread the target.
        let value = target.get_value_untyped(property);
        if target_state.casts_value.get() {
            ValueTypes::try_cast(&value, ValueType::of::<TValue>())?.downcast_ref::<TValue>().cloned()
        } else {
            value.downcast_ref::<TValue>().cloned()
        }
    }
}

/// A binding expression for the common case of a compiled binding: a single
/// plain property read from the data context of the target element, whose
/// type is assignable to the type of the target property. When the two types
/// are the same, values travel typed in both directions and nothing is boxed
/// per update; otherwise the value is cast to the type of the target property
/// (see [`ValueTypes::is_assignable`]).
pub struct TypedBindingExpression<TSource: PartialEq + 'static, TValue: PropertyValue> {
    this: Weak<Self>,
    property_info: Rc<TypedClrPropertyInfo<TSource, TValue>>,
    controller: BindingController,
    source: SourceSlot<TSource>,
    source_value: TypedValueSlot<TValue>,
}

impl<TSource: PartialEq + 'static, TValue: PropertyValue> TypedBindingExpression<TSource, TValue> {
    pub fn new(
        property_info: Rc<TypedClrPropertyInfo<TSource, TValue>>,
        mode: BindingMode,
        default_priority: BindingPriority,
    ) -> Rc<Self> {
        crate::perf_count!(BindingExpressionsCreated);
        Rc::new_cyclic(|this: &Weak<Self>| Self {
            this: this.clone(),
            property_info,
            controller: BindingController {
                mode,
                default_priority,
                target: BindingTarget::new(default_priority),
                is_running: Cell::new(false),
                produce_value: Cell::new(true),
                #[cfg(all(debug_assertions, feature = "hot-reload"))]
                reload_paused: Cell::new(false),
                #[cfg(all(debug_assertions, feature = "hot-reload"))]
                reload_pending: Cell::new(false),
                target_subscription: RefCell::new(None),
                should_update_one_time_binding_target: Cell::new(mode == BindingMode::OneTime),
            },
            source: SourceSlot { weak: RefCell::new(None), token: Cell::new(None) },
            source_value: TypedValueSlot(RefCell::new(None)),
        })
    }

    /// A description of the binding expression.
    pub fn description(&self) -> &str {
        self.property_info.name()
    }

    fn update_source(&self, data_context: Option<BoxedValue>) {
        let subscribe = |inpc: &dyn INotifyPropertyChanged| {
            let weak = self.this.clone();
            inpc.property_changed().add(Rc::new(move |name: &str| {
                if let Some(this) = weak.upgrade() {
                    this.on_source_changed(name);
                }
            }))
        };
        let source = self.source.replace(
            &self.controller,
            self.property_info.name(),
            self.property_info.inpc,
            data_context.as_ref(),
            &subscribe,
        );
        if self.controller.mode == BindingMode::OneWayToSource {
            if let Some(value) = TypedValueSlot::<TValue>::try_get_target_value(&self.controller.target) {
                self.write_value_to_source(value);
            }
        } else {
            self.write_source_value_to_target(source.as_ref());
        }
    }

    fn write_value_to_source(&self, value: TValue) {
        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        if self.controller.reload_paused.get() {
            return;
        }
        if self.controller.target.property.get().is_some() && self.controller.target.owner().is_some() {
            if let Some(source) = self.source.get() {
                self.property_info.set_on(&source, value);
            }
        }
    }

    fn on_source_changed(&self, name: &str) {
        // An empty name means "all properties changed".
        if name.is_empty() || name == self.property_info.name() {
            let source = self.source.get();
            if source.is_some() {
                self.write_source_value_to_target(source.as_ref());
            }
        }
    }

    fn write_source_value_to_target(&self, source: Option<&Rc<TSource>>) {
        let controller = &self.controller;
        if controller.mode == BindingMode::OneTime && !controller.should_update_one_time_binding_target.get() {
            return;
        }
        let new_value = match source {
            None => None,
            Some(source) => match self.property_info.try_get_from(source) {
                Ok(value) => Some(value),
                Err(error) => {
                    // Getter failures must not escape into the source's
                    // change notification: the error is logged and the value
                    // cleared, as the untyped binding path does.
                    controller.log_get_error(self.property_info.name(), &error);
                    None
                }
            },
        };
        let had_value = self.source_value.0.replace(new_value).is_some();
        let has_value = self.source_value.0.borrow().is_some();

        if controller.produce_value.get() && controller.mode != BindingMode::OneWayToSource {
            // An expression which has no value, and had no value before, must
            // not notify: doing so would push the target property's default
            // value, overriding values from styles or property inheritance.
            // Otherwise always notify, even if the value is unchanged, as the
            // target may hold an uncommitted value.
            if had_value || has_value {
                self.publish_value();
            }
            if controller.mode == BindingMode::OneTime {
                controller.should_update_one_time_binding_target.set(false);
            }
        }
    }

    #[inline]
    fn publish_value(&self) {
        crate::perf_count!(BindingValuesPublished);
        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        if self.controller.reload_paused.get() {
            self.controller.reload_pending.set(true);
            return;
        }
        self.publish_value_now();
    }

    #[inline]
    fn publish_value_now(&self) {
        // Flag that the source value is being pushed to the target so that
        // the resulting change notification isn't echoed straight back to the
        // source in two-way mode.
        let sink = self.controller.target.sink.borrow().clone();
        let (Some(sink), Some(this)) = (sink, self.this.upgrade()) else { return };
        let this: Rc<dyn BindingExpressionBase> = this;
        self.controller.target.publish(sink, this);
    }
}

impl<TSource: PartialEq + 'static, TValue: PropertyValue> BindingHost for TypedBindingExpression<TSource, TValue> {
    fn expression(&self) -> Option<Rc<dyn BindingExpressionBase>> {
        self.this.upgrade().map(|this| this as Rc<dyn BindingExpressionBase>)
    }

    fn subscribe_target(&self, target: &FerroObject) -> Subscription {
        let weak = self.this.clone();
        Subscription::new(target.property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.controller.on_target_property_changed(&*this, e);
            }
        }))
    }

    fn refresh_source(&self, data_context: Option<BoxedValue>) {
        self.update_source(data_context);
    }

    fn disconnect_source(&self) {
        self.source.unsubscribe(self.property_info.inpc);
    }

    fn write_target_change(&self, e: &crate::FerroPropertyChangedEventArgs<'_>) {
        let casts = self.controller.target.casts_value.get();
        let value = if !self.controller.target.writeback_uses_change_value(e) {
            TypedValueSlot::<TValue>::try_get_target_value(&self.controller.target)
        } else if casts {
            e.to_owned_args()
                .new_value()
                .and_then(|v| ValueTypes::try_cast(&v, ValueType::of::<TValue>()))
                .and_then(|v| v.downcast_ref::<TValue>().cloned())
        } else {
            e.new_value().downcast_ref::<TValue>().cloned()
        };
        if let Some(value) = value {
            self.write_value_to_source(value);
        }
    }
}

impl<TSource: PartialEq + 'static, TValue: PropertyValue> IDisposable for TypedBindingExpression<TSource, TValue> {
    fn dispose(&self) {
        self.controller.dispose(self);
    }
}

impl<TSource: PartialEq + 'static, TValue: PropertyValue> IValueEntry for TypedBindingExpression<TSource, TValue> {
    fn property(&self) -> &'static FerroProperty {
        self.controller.target.property.get().expect("The binding expression is not attached.")
    }

    fn has_value(&self) -> bool {
        BindingExpressionBase::start(self, false);
        self.source_value.0.borrow().is_some()
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        BindingExpressionBase::start(self, false);
        self.source_value.try_get_value(&self.controller.target, out)
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        BindingExpressionBase::start(self, false);
        self.source_value.get_value_boxed(&self.controller.target)
    }

    fn get_data_validation_state(&self) -> Option<(BindingValueType, Option<BindingError>)> {
        // Data validation is not supported by the typed expression: bindings
        // whose target property enables it use the untyped expression.
        None
    }

    fn unsubscribe(&self) {
        // Reset the running state so that the expression can be restarted if
        // the value store reactivates this entry later.
        self.controller.stop_core(self);
        self.controller.is_running.set(false);
    }

    fn as_binding_expression(self: Rc<Self>) -> Option<Rc<dyn BindingExpressionBase>> {
        Some(self)
    }
}

#[allow(private_interfaces)]
impl<TSource: PartialEq + 'static, TValue: PropertyValue> BindingExpressionBase
    for TypedBindingExpression<TSource, TValue>
{
    fn priority(&self) -> BindingPriority {
        self.controller.target.priority.get()
    }

    fn target_property(&self) -> Option<&'static FerroProperty> {
        self.controller.target.property.get()
    }

    fn default_priority(&self) -> BindingPriority {
        self.controller.default_priority
    }

    fn is_data_validation_enabled(&self) -> bool {
        false
    }

    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    fn can_pause_for_reload(&self) -> bool {
        matches!(self.controller.mode, BindingMode::OneWay | BindingMode::TwoWay)
    }

    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    fn pause_for_reload(&self) -> bool {
        self.can_pause_for_reload() && !self.controller.reload_paused.replace(true)
    }

    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    fn resume_after_reload(&self) {
        if !self.controller.reload_paused.get() {
            return;
        }
        struct Resume<'a>(&'a Cell<bool>);
        impl Drop for Resume<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _resume = Resume(&self.controller.reload_paused);
        while self.controller.reload_pending.replace(false) {
            self.publish_value_now();
        }
    }

    fn attach(
        &self,
        sink: SinkRef,
        frame: Option<Weak<ImmediateValueFrame>>,
        target: &FerroObject,
        target_property: &'static FerroProperty,
        priority: BindingPriority,
    ) {
        self.controller.target.attach(ValueType::of::<TValue>(), sink, frame, target, target_property, priority);
    }

    #[inline]
    fn start(&self, produce_value: bool) {
        if !self.controller.is_running.get() {
            self.controller.start_stopped(self, produce_value);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<TSource: PartialEq + 'static, TValue: PropertyValue> Drop for TypedBindingExpression<TSource, TValue> {
    fn drop(&mut self) {
        self.source.unsubscribe(self.property_info.inpc);
        if let Some(s) = self.controller.target_subscription.get_mut().take() {
            s.dispose();
        }
    }
}
