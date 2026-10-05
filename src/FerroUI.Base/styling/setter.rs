use super::{
    DirectPropertySetterBindingInstance, DirectPropertySetterInstance, IStyleInstance, ISetterInstance,
    ISetterValue, ITemplate, PropertySetterTemplateInstance, SetterBase, SetterInstance, SetterInstanceKind,
    StyleInstance,
};
use crate::data::core::SinkRef;
use crate::data::{BindingBase, BindingPriority};
use crate::property_store::{IValueEntry, ValueFrame};
use crate::reactive::IObservable;
use crate::{AnyValue, BoxedValue, FerroProperty, PropertyValue, StyledElement, StyledProperty};
use std::any::Any;
use std::cell::{Cell, OnceCell, RefCell};
use std::fmt;
use std::rc::Rc;

/// The value of a [`Setter`].
#[derive(Clone)]
pub enum SetterValue {
    /// A plain value, which must hold exactly the value type of the setter's
    /// property.
    Value(BoxedValue),
    /// A source of values: the setter binds the property to it on every
    /// control the style is applied to.
    ///
    /// This is the seam for binding descriptions: a binding class will
    /// create its per-control source where this variant uses the same
    /// observable for every control.
    Binding(Rc<dyn IObservable<BoxedValue>>),
    /// A binding description: the setter instantiates the binding on every
    /// control the style is applied to.
    BindingBase(Rc<dyn BindingBase>),
    /// A template: the setter sets the property to the object the template
    /// builds, building it lazily once per control.
    Template(Rc<dyn ITemplate>),
}

/// A setter for a [`Style`](super::Style).
///
/// A setter describes what will take place when the requirements for a style
/// are met: it sets a property to a value, binds it to a source of values or
/// sets it to the result of a template.
pub struct Setter {
    property: Cell<Option<&'static FerroProperty>>,
    value: RefCell<Option<SetterValue>>,
    direct: OnceCell<Rc<DirectPropertySetterInstance>>,
}


/// Compares by identity (reference equality), as the reference type this
/// mirrors.
impl PartialEq for Setter {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Default for Setter {
    fn default() -> Self {
        Self { property: Cell::new(None), value: RefCell::new(None), direct: OnceCell::new() }
    }
}

impl Setter {
    /// Creates a setter without a property or value.
    pub fn empty() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Creates a setter that sets a styled property to a value.
    pub fn new<T: PropertyValue>(property: &'static StyledProperty<T>, value: T) -> Rc<Self> {
        Self::with_value(property, SetterValue::Value(Rc::new(value)))
    }

    /// Creates a setter that sets a property to an untyped value, which must
    /// hold exactly the property's value type.
    pub fn new_untyped(property: &'static FerroProperty, value: BoxedValue) -> Rc<Self> {
        Self::with_value(property, SetterValue::Value(value))
    }

    /// Creates a setter that binds a property to a source of values.
    pub fn new_binding(property: &'static FerroProperty, source: Rc<dyn IObservable<BoxedValue>>) -> Rc<Self> {
        Self::with_value(property, SetterValue::Binding(source))
    }

    /// Creates a setter that binds a property with a binding (a compiled,
    /// string-path, template or multi binding).
    pub fn new_binding_base(property: &'static FerroProperty, binding: Rc<dyn BindingBase>) -> Rc<Self> {
        Self::with_value(property, SetterValue::BindingBase(binding))
    }

    /// Creates a setter that sets a property to the object built by a
    /// template.
    pub fn new_template(property: &'static FerroProperty, template: Rc<dyn ITemplate>) -> Rc<Self> {
        Self::with_value(property, SetterValue::Template(template))
    }

    /// Creates a setter for a property with the given kind of value.
    pub fn with_value(property: &'static FerroProperty, value: SetterValue) -> Rc<Self> {
        let setter = Self::default();
        setter.property.set(Some(property));
        *setter.value.borrow_mut() = Some(value);
        Rc::new(setter)
    }

    /// The property to set.
    pub fn property(&self) -> Option<&'static FerroProperty> {
        self.property.get()
    }

    pub fn set_property(&self, value: Option<&'static FerroProperty>) {
        self.property.set(value)
    }

    /// The property value.
    pub fn value(&self) -> Option<SetterValue> {
        self.value.borrow().clone()
    }

    pub fn set_value(&self, value: Option<SetterValue>) {
        *self.value.borrow_mut() = value;
    }

    /// Sets a plain value that wants to be notified that it has become a
    /// setter value. `boxed` is the value as it will be set on the property.
    pub fn set_setter_value(&self, value: &dyn ISetterValue, boxed: BoxedValue) {
        value.initialize(self);
        *self.value.borrow_mut() = Some(SetterValue::Value(boxed));
    }

    fn ensure_property(&self) -> &'static FerroProperty {
        match self.property.get() {
            Some(property) => property,
            None => panic!("Setter.Property must be set."),
        }
    }

    fn plain_value(&self) -> Option<BoxedValue> {
        match &*self.value.borrow() {
            Some(SetterValue::Value(value)) => Some(value.clone()),
            // A setter without a value sets null, where the property takes it.
            None => self.property.get().and_then(Self::null_value),
            _ => None,
        }
    }

    /// The null of the value type of `property` (`None` of an optional
    /// type), if the type has one: what a setter whose value is null sets.
    fn null_value(property: &'static FerroProperty) -> Option<BoxedValue> {
        crate::data::core::ValueTypes::null_value(crate::data::core::ValueType::new(
            property.property_type(),
            property.property_type_name(),
        ))
    }

    fn set_binding(
        &self,
        property: &'static FerroProperty,
        instance: &Rc<StyleInstance>,
        target: &StyledElement,
        source: Rc<dyn IObservable<BoxedValue>>,
    ) -> SetterInstance {
        if !property.is_direct() {
            let frame: Rc<dyn ValueFrame> = instance.clone();
            let entry = property.routes().route_create_binding_entry(target, Rc::downgrade(&frame), source);
            SetterInstance { kind: SetterInstanceKind::Entry(entry), is_setter: false }
        } else {
            target.bind_property_untyped(property, source, BindingPriority::LocalValue);
            SetterInstance {
                kind: SetterInstanceKind::Other(Rc::new(DirectPropertySetterBindingInstance)),
                is_setter: false,
            }
        }
    }

    fn set_binding_base(
        &self,
        property: &'static FerroProperty,
        instance: &Rc<StyleInstance>,
        target: &StyledElement,
        binding: Rc<dyn BindingBase>,
    ) -> SetterInstance {
        if !property.is_direct() {
            // The expression is an entry of the style instance's frame: it is
            // started when the frame's values are evaluated and stopped when
            // the style is deactivated or detached.
            let expression = binding.create_instance(target, Some(property), None);
            let owner: &crate::FerroObject = target;
            let frame: &dyn ValueFrame = &**instance;
            expression.attach(SinkRef::Store(owner.to_weak()), None, owner, property, frame.base().priority());
            let entry: Rc<dyn IValueEntry> = expression;
            SetterInstance { kind: SetterInstanceKind::Entry(entry), is_setter: false }
        } else {
            target.bind_binding(property, &*binding);
            SetterInstance {
                kind: SetterInstanceKind::Other(Rc::new(DirectPropertySetterBindingInstance)),
                is_setter: false,
            }
        }
    }

    fn set_direct_value(&self, property: &'static FerroProperty, target: &StyledElement, value: &BoxedValue) -> SetterInstance {
        let value: &dyn AnyValue = &**value;
        target.set_value_untyped(property, value.as_any(), BindingPriority::LocalValue);
        let direct: Rc<dyn ISetterInstance> =
            self.direct.get_or_init(|| Rc::new(DirectPropertySetterInstance)).clone();
        // The value is set on the control itself, so the style instance
        // cannot be shared: every control needs its own attach.
        SetterInstance { kind: SetterInstanceKind::Other(direct), is_setter: false }
    }
}

impl ISetterInstance for Setter {}

impl SetterBase for Setter {
    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn instance(self: Rc<Self>, instance: &Rc<StyleInstance>, target: &StyledElement) -> SetterInstance {
        let Some(property) = self.property.get() else {
            panic!("Setter.Property must be set.");
        };

        if property.is_direct() && instance.has_activator() {
            panic!(
                "Cannot set direct property '{}' in '{}' because the style has an activator.",
                property,
                instance.source_display()
            );
        }

        if instance.has_activator() {
            if let Some(class_property_name) = crate::ClassBindingManager::is_classes_binding_property(property) {
                panic!(
                    "Cannot set Class Binding property '(Classes.{})' in '{}' because the style has an activator.",
                    class_property_name,
                    instance.source_display()
                );
            }
        }

        let value = self.value.borrow().clone();
        match value {
            Some(SetterValue::Binding(source)) => self.set_binding(property, instance, target, source),
            Some(SetterValue::BindingBase(binding)) => self.set_binding_base(property, instance, target, binding),
            Some(SetterValue::Template(template)) => SetterInstance {
                kind: SetterInstanceKind::Entry(Rc::new(PropertySetterTemplateInstance::new(property, template))),
                is_setter: false,
            },
            Some(SetterValue::Value(value)) => {
                let inner: &dyn AnyValue = &*value;
                if !property.is_valid_value(inner.as_any()) {
                    panic!("Setter value '{:?}' is not a valid value for property '{}'.", inner, property);
                }
                if property.is_direct() {
                    self.set_direct_value(property, target, &value)
                } else {
                    SetterInstance { kind: SetterInstanceKind::Entry(self), is_setter: true }
                }
            }
            // Null is a value of a property whose type admits it (a reference or
            // nullable type in the managed original); of a value type it is not.
            None => match Self::null_value(property) {
                Some(null) if property.is_direct() => self.set_direct_value(property, target, &null),
                Some(_) => SetterInstance { kind: SetterInstanceKind::Entry(self), is_setter: true },
                None => panic!("Setter value '(null)' is not a valid value for property '{}'.", property),
            },
        }
    }
}

impl IValueEntry for Setter {
    fn property(&self) -> &'static FerroProperty {
        self.ensure_property()
    }

    fn has_value(&self) -> bool {
        true
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        match self.plain_value() {
            Some(value) => {
                let value: &dyn AnyValue = &*value;
                self.ensure_property().routes().route_copy_value(value.as_any(), out)
            }
            None => false,
        }
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        self.plain_value()
    }

    fn unsubscribe(&self) {}
}

impl fmt::Display for Setter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let property = self.property.get().map(|p| p.name().to_string()).unwrap_or_default();
        match &*self.value.borrow() {
            Some(SetterValue::Value(value)) => write!(f, "Setter: {property} = {:?}", &**value),
            Some(SetterValue::Binding(_)) | Some(SetterValue::BindingBase(_)) => {
                write!(f, "Setter: {property} = (binding)")
            }
            Some(SetterValue::Template(_)) => write!(f, "Setter: {property} = (template)"),
            None => write!(f, "Setter: {property} = "),
        }
    }
}
