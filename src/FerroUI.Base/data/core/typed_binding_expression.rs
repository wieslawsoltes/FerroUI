use super::{IPropertyInfo, SinkRef, Value, ValueType, ValueTypes};
use crate::data::core::plugins::property_value_type;
use crate::data::core::PropertyKind;
use crate::data::model::INotifyPropertyChanged;
use crate::data::{BindingError, BindingExpressionBase, BindingMode, BindingPriority, BindingValueType};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::property_store::{ImmediateValueFrame, IValueEntry};
use crate::reactive::IDisposable;
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

fn needs_handle(name: &str) -> BindingError {
    BindingError::message(format!("Property {name} is accessed through the shared handle of its owner."))
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

    /// The owner behind an untyped value: the box is the shared object, or
    /// holds a handle of it (the form in which an items control gives its
    /// items to their containers as data context), or an object that is
    /// assignable to the owner type. As the typed expression reads its
    /// source.
    fn owner_of(target: &BoxedValue) -> Option<Rc<TSource>> {
        let any: Rc<dyn Any> = target.clone();
        any.downcast::<TSource>().ok().or_else(|| {
            ValueTypes::try_cast(target, ValueType::of::<Rc<TSource>>())
                .and_then(|cast| cast.downcast_ref::<Rc<TSource>>().cloned())
        })
    }

    /// Converts an untyped value to the type of the property (null to the
    /// null of a nullable property type).
    fn value_from_untyped(&self, value: Option<&BoxedValue>) -> Result<TValue, BindingError> {
        let typed = match value {
            Some(_) => Value::<TValue>::from_untyped(value),
            None => ValueTypes::null_value(ValueType::of::<TValue>()).and_then(|v| v.downcast_ref::<TValue>().cloned()),
        };
        typed.ok_or_else(|| {
            BindingError::message(format!(
                "Object of type '{}' cannot be converted to type '{}' for property '{}'.",
                value.map_or("null", |v| v.type_name()),
                ValueType::of::<TValue>(),
                self.name
            ))
        })
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
        self.set_typed(target, self.value_from_untyped(value)?);
        Ok(())
    }

    fn get_boxed(&self, target: &BoxedValue) -> Option<BoxedValue> {
        self.try_get_boxed(target).ok().flatten()
    }

    fn try_get_boxed(&self, target: &BoxedValue) -> Result<Option<BoxedValue>, BindingError> {
        let Some(owner) = Self::owner_of(target) else { return Ok(None) };
        Ok(ValueTypes::normalize(BooleanBoxes::box_value(&self.try_get_from(&owner)?)))
    }

    fn set_boxed(&self, target: &BoxedValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        if self.setter.is_none() {
            return Err(BindingError::message(format!("Property {} doesn't have a setter", self.name)));
        }
        let owner = Self::owner_of(target)
            .ok_or_else(|| BindingError::message("The property owner is of the wrong type."))?;
        self.set_on(&owner, self.value_from_untyped(value)?);
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

/// A binding expression for the common case of a compiled binding: a single
/// plain property read from the data context of the target element, whose
/// type is assignable to the type of the target property. When the two types
/// are the same, values travel typed in both directions and nothing is boxed
/// per update; otherwise the value is cast to the type of the target property
/// (see [`ValueTypes::is_assignable`]).
pub struct TypedBindingExpression<TSource: PartialEq + 'static, TValue: PropertyValue> {
    this: Weak<TypedBindingExpression<TSource, TValue>>,
    property_info: Rc<TypedClrPropertyInfo<TSource, TValue>>,
    mode: BindingMode,
    default_priority: BindingPriority,
    priority: Cell<BindingPriority>,
    target_property: Cell<Option<&'static FerroProperty>>,
    /// Whether the type of the target property differs from `TValue`: values
    /// are then cast between the two types.
    casts_value: Cell<bool>,
    is_running: Cell<bool>,
    produce_value: Cell<bool>,
    writing_value_to_target: Cell<bool>,
    sink: RefCell<Option<SinkRef>>,
    frame: RefCell<Option<Weak<ImmediateValueFrame>>>,
    source: RefCell<Option<Weak<TSource>>>,
    source_token: Cell<Option<u64>>,
    target: RefCell<Option<WeakRef<FerroObject>>>,
    target_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    source_value: RefCell<Option<TValue>>,
    should_update_one_time_binding_target: Cell<bool>,
}

impl<TSource: PartialEq + 'static, TValue: PropertyValue> TypedBindingExpression<TSource, TValue> {
    pub fn new(
        property_info: Rc<TypedClrPropertyInfo<TSource, TValue>>,
        mode: BindingMode,
        default_priority: BindingPriority,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            property_info,
            mode,
            default_priority,
            priority: Cell::new(default_priority),
            target_property: Cell::new(None),
            casts_value: Cell::new(false),
            is_running: Cell::new(false),
            produce_value: Cell::new(true),
            writing_value_to_target: Cell::new(false),
            sink: RefCell::new(None),
            frame: RefCell::new(None),
            source: RefCell::new(None),
            source_token: Cell::new(None),
            target: RefCell::new(None),
            target_subscription: RefCell::new(None),
            source_value: RefCell::new(None),
            should_update_one_time_binding_target: Cell::new(mode == BindingMode::OneTime),
        })
    }

    /// A description of the binding expression.
    pub fn description(&self) -> &str {
        self.property_info.name()
    }

    fn try_get_source(&self) -> Option<Rc<TSource>> {
        self.source.borrow().as_ref().and_then(Weak::upgrade)
    }

    fn try_get_target(&self) -> Option<crate::Ref<FerroObject>> {
        self.target.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn log(&self, error: &str) {
        let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::BINDING) else { return };
        let Some(target) = self.try_get_target() else { return };
        let property = self.target_property.get().map_or("(unknown)", |p| p.name());
        let target: &FerroObject = &target;
        log.log_with_values(
            Some(target as &dyn Any),
            "An error occurred binding {Property} to {Expression}: {Message}",
            &[&property, &self.description(), &error],
        );
    }

    fn start_core(&self) {
        let (Some(target), Some(_)) = (self.try_get_target(), self.target_property.get()) else { return };
        let weak = self.this.clone();
        let subscription = target.property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_target_property_changed(e);
            }
        });
        self.target_subscription.replace(Some(subscription));
        let data_context = target.downcast_ref::<StyledElement>().and_then(|e| e.data_context());
        self.update_source(data_context);
    }

    fn stop_core(&self) {
        let subscription = self.target_subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
        if self.try_get_target().is_some() {
            self.update_source(None);
        } else {
            self.unsubscribe_source();
        }
    }

    fn unsubscribe_source(&self) {
        if let Some(token) = self.source_token.take() {
            if let Some(old) = self.try_get_source() {
                if let Some(inpc) = self.property_info.notifier(&old) {
                    inpc.property_changed().remove(token);
                }
            }
        }
    }

    fn update_source(&self, data_context: Option<BoxedValue>) {
        let source: Option<Rc<TSource>> = data_context.as_ref().and_then(|d| {
            let any: Rc<dyn Any> = d.clone();
            // The object itself, or an object that is assignable to the
            // source type (the cast `(TSource)dataContext` to a base type).
            any.downcast::<TSource>().ok().or_else(|| {
                ValueTypes::try_cast(d, ValueType::of::<Rc<TSource>>())
                    .and_then(|cast| cast.downcast_ref::<Rc<TSource>>().cloned())
            })
        });
        if let (Some(d), None) = (&data_context, &source) {
            self.log(&format!(
                "Could not convert DataContext of type '{}' to '{}'.",
                d.type_name(),
                std::any::type_name::<TSource>()
            ));
        }

        self.unsubscribe_source();
        self.source.replace(source.as_ref().map(Rc::downgrade));
        self.should_update_one_time_binding_target.set(true);

        if let Some(source) = &source {
            if let Some(inpc) = self.property_info.notifier(source) {
                let weak = self.this.clone();
                let token = inpc.property_changed().add(Rc::new(move |name: &str| {
                    if let Some(this) = weak.upgrade() {
                        // An empty name means "all properties changed".
                        if name.is_empty() || name == this.property_info.name() {
                            let source = this.try_get_source();
                            if source.is_some() {
                                this.write_source_value_to_target(source.as_ref());
                            }
                        }
                    }
                }));
                self.source_token.set(Some(token));
            }
        }

        if self.mode == BindingMode::OneWayToSource {
            if let Some(value) = self.try_get_target_value() {
                self.write_value_to_source(value);
            }
        } else {
            self.write_source_value_to_target(source.as_ref());
        }
    }

    fn write_value_to_source(&self, value: TValue) {
        if self.target_property.get().is_some() && self.try_get_target().is_some() {
            if let Some(source) = self.try_get_source() {
                self.property_info.set_on(&source, value);
            }
        }
    }

    fn write_source_value_to_target(&self, source: Option<&Rc<TSource>>) {
        if self.mode == BindingMode::OneTime && !self.should_update_one_time_binding_target.get() {
            return;
        }
        let new_value = match source {
            None => None,
            Some(source) => match self.property_info.try_get_from(source) {
                Ok(value) => Some(value),
                Err(e) => {
                    // Getter failures must not escape into the source's
                    // change notification, so log the error and clear the
                    // value, as the untyped binding path does.
                    self.log(&format!("Error getting '{}': {}", self.property_info.name(), e));
                    None
                }
            },
        };
        let had_value = self.source_value.replace(new_value).is_some();
        let has_value = self.source_value.borrow().is_some();

        if self.produce_value.get() && self.mode != BindingMode::OneWayToSource {
            // An expression which has no value, and had no value before, must
            // not notify: doing so would push the target property's default
            // value, overriding values from styles or property inheritance.
            // Otherwise always notify, even if the value is unchanged, as the
            // target may hold an uncommitted value.
            if had_value || has_value {
                self.publish_value();
            }
            if self.mode == BindingMode::OneTime {
                self.should_update_one_time_binding_target.set(false);
            }
        }
    }

    fn publish_value(&self) {
        // Flag that the source value is being pushed to the target so that
        // the resulting change notification isn't echoed straight back to the
        // source in two-way mode.
        let sink = self.sink.borrow().clone();
        let (Some(sink), Some(this)) = (sink, self.this.upgrade()) else { return };
        let this: Rc<dyn BindingExpressionBase> = this;
        self.writing_value_to_target.set(true);
        sink.on_changed(&this, true, false);
        self.writing_value_to_target.set(false);
    }

    fn on_target_property_changed(&self, e: &crate::FerroPropertyChangedEventArgs<'_>) {
        if e.property().id() == StyledElement::data_context_property().id() {
            let data_context =
                self.try_get_target().and_then(|t| t.downcast_ref::<StyledElement>().and_then(|e| e.data_context()));
            self.update_source(data_context);
        } else if self.target_property.get().is_some_and(|p| p.id() == e.property().id()) {
            // Don't write back to the source if this change is the binding
            // pushing the source value to the target.
            if !self.writing_value_to_target.get()
                && matches!(self.mode, BindingMode::TwoWay | BindingMode::OneWayToSource)
            {
                let value = if self.casts_value.get() {
                    self.try_get_target_value()
                } else {
                    e.new_value().downcast_ref::<TValue>().cloned()
                };
                if let Some(value) = value {
                    self.write_value_to_source(value);
                }
            }
        }
    }

    fn try_get_target_value(&self) -> Option<TValue> {
        let (Some(property), Some(target)) = (self.target_property.get(), self.try_get_target()) else {
            return None;
        };
        // Only reached when the source changes in one-way-to-source mode, or
        // when the target changes and its type differs from the source type.
        let value = target.get_value_untyped(property);
        if self.casts_value.get() {
            ValueTypes::try_cast(&value, ValueType::of::<TValue>())?.downcast_ref::<TValue>().cloned()
        } else {
            value.downcast_ref::<TValue>().cloned()
        }
    }
}

impl<TSource: PartialEq + 'static, TValue: PropertyValue> IDisposable for TypedBindingExpression<TSource, TValue> {
    fn dispose(&self) {
        if self.sink.borrow().is_none() {
            return;
        }
        // Clear the sink before stopping so that the unsubscribe doesn't push
        // a final value to a value store that is about to clear this entry.
        let sink = self.sink.borrow_mut().take();
        let frame = self.frame.borrow_mut().take();
        self.stop_core();
        self.is_running.set(false);
        if let (Some(sink), Some(this)) = (sink, self.this.upgrade()) {
            let this: Rc<dyn BindingExpressionBase> = this;
            sink.on_completed(&this);
        }
        if let (Some(frame), Some(property)) = (frame.and_then(|f| f.upgrade()), self.target_property.get()) {
            frame.on_entry_disposed(property);
        }
    }
}

impl<TSource: PartialEq + 'static, TValue: PropertyValue> IValueEntry for TypedBindingExpression<TSource, TValue> {
    fn property(&self) -> &'static FerroProperty {
        self.target_property.get().expect("The binding expression is not attached.")
    }

    fn has_value(&self) -> bool {
        BindingExpressionBase::start(self, false);
        self.source_value.borrow().is_some()
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        BindingExpressionBase::start(self, false);
        // A value that has to be cast to the type of the target property is
        // read through `get_value_boxed`.
        if self.casts_value.get() {
            return false;
        }
        match (out.downcast_mut::<Option<TValue>>(), &*self.source_value.borrow()) {
            (Some(out), Some(value)) => {
                *out = Some(value.clone());
                true
            }
            _ => false,
        }
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        BindingExpressionBase::start(self, false);
        // A boolean is boxed as its cached box, so that reading it into an
        // untyped target property does not allocate per read.
        let value = self.source_value.borrow().as_ref().map(BooleanBoxes::box_value)?;
        match self.target_property.get() {
            // The store requires a value of exactly the target property type.
            Some(property) if self.casts_value.get() => ValueTypes::try_cast(&value, property_value_type(property)),
            _ => Some(value),
        }
    }

    fn get_data_validation_state(&self) -> Option<(BindingValueType, Option<BindingError>)> {
        // Data validation is not supported by the typed expression: bindings
        // whose target property enables it use the untyped expression.
        None
    }

    fn unsubscribe(&self) {
        // Reset the running state so that the expression can be restarted if
        // the value store reactivates this entry later.
        self.stop_core();
        self.is_running.set(false);
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
        self.priority.get()
    }

    fn target_property(&self) -> Option<&'static FerroProperty> {
        self.target_property.get()
    }

    fn default_priority(&self) -> BindingPriority {
        self.default_priority
    }

    fn is_data_validation_enabled(&self) -> bool {
        false
    }

    fn attach(
        &self,
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
        if self.target_property.get().is_some_and(|p| p.id() != target_property.id()) {
            panic!("TypedBindingExpression was already attached to a different property.");
        }
        let target_type = property_value_type(target_property);
        if !ValueTypes::is_assignable(ValueType::of::<TValue>(), target_type) {
            panic!(
                "TypedBindingExpression of type '{}' cannot be bound to a property of type '{}'.",
                std::any::type_name::<TValue>(),
                target_property.property_type_name()
            );
        }
        self.sink.replace(Some(sink));
        self.frame.replace(frame);
        self.target.replace(Some(target.to_weak()));
        self.target_property.set(Some(target_property));
        self.casts_value.set(target_type != ValueType::of::<TValue>());
        self.priority.set(priority);
    }

    fn start(&self, produce_value: bool) {
        if self.is_running.get() {
            return;
        }
        self.is_running.set(true);
        self.produce_value.set(produce_value);
        self.start_core();
        self.produce_value.set(true);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<TSource: PartialEq + 'static, TValue: PropertyValue> Drop for TypedBindingExpression<TSource, TValue> {
    fn drop(&mut self) {
        self.unsubscribe_source();
        if let Some(s) = self.target_subscription.get_mut().take() {
            s.dispose();
        }
    }
}
