//! Export the public styled-property contract of a FerroUI control. The host
//! chooses the types to expose; there is no global scan or construction of
//! prototype controls. Defaults and validation come from each property's owner.
use super::*;
use ferroui_base::{
    data::BindingPriority, BoxedValue, FerroProperty, ObjectType, TypeInfo, Upcast,
};
use ferroui_controls::{Button, Panel};
use std::any::Any;

trait Scalar: Clone + PartialEq + 'static {
    const TYPE: &'static str;
    fn encode(&self) -> Value;
    fn decode(value: &Value) -> Option<Self>;
}

impl Scalar for bool {
    const TYPE: &'static str = "bool";
    fn encode(&self) -> Value {
        Value::Bool(*self)
    }
    fn decode(value: &Value) -> Option<Self> {
        if let Value::Bool(b) = value {
            Some(*b)
        } else {
            None
        }
    }
}
impl Scalar for f64 {
    const TYPE: &'static str = "real";
    fn encode(&self) -> Value {
        Value::Number(*self)
    }
    fn decode(value: &Value) -> Option<Self> {
        if let Value::Number(n) = value {
            Some(*n)
        } else {
            None
        }
    }
}
impl Scalar for i32 {
    const TYPE: &'static str = "int";
    fn encode(&self) -> Value {
        Value::Number(f64::from(*self))
    }
    fn decode(value: &Value) -> Option<Self> {
        let n = f64::decode(value)?;
        (n.fract() == 0.0 && (i32::MIN as f64..=i32::MAX as f64).contains(&n)).then_some(n as i32)
    }
}
impl Scalar for String {
    const TYPE: &'static str = "string";
    fn encode(&self) -> Value {
        Value::String(self.clone())
    }
    fn decode(value: &Value) -> Option<Self> {
        if let Value::String(s) = value {
            Some(s.clone())
        } else {
            None
        }
    }
}
impl<T: Scalar> Scalar for Option<T> {
    const TYPE: &'static str = "var";
    fn encode(&self) -> Value {
        self.as_ref().map_or(Value::Null, Scalar::encode)
    }
    fn decode(value: &Value) -> Option<Self> {
        if *value == Value::Null {
            Some(None)
        } else {
            T::decode(value).map(Some)
        }
    }
}

struct Field {
    property: &'static FerroProperty,
    ty: &'static str,
    default: Value,
    encode: fn(&dyn Any) -> Option<Value>,
    decode: fn(&Value) -> Option<BoxedValue>,
}

impl Field {
    fn typed<T: Scalar>(
        property: &'static FerroProperty,
        owner: &'static TypeInfo,
    ) -> Option<Self> {
        let default = property.as_styled::<T>()?.get_default_value(owner).encode();
        Some(Self {
            property,
            ty: T::TYPE,
            default,
            encode: |value| value.downcast_ref::<T>().map(Scalar::encode),
            decode: |value| T::decode(value).map(|v| Rc::new(v) as BoxedValue),
        })
    }

    fn from_property(property: &'static FerroProperty, owner: &'static TypeInfo) -> Option<Self> {
        Self::typed::<bool>(property, owner)
            .or_else(|| Self::typed::<f64>(property, owner))
            .or_else(|| Self::typed::<i32>(property, owner))
            .or_else(|| Self::typed::<String>(property, owner))
            .or_else(|| Self::typed::<Option<String>>(property, owner))
            .or_else(|| Self::typed::<Option<bool>>(property, owner))
            .or_else(|| Self::typed::<Option<f64>>(property, owner))
            .or_else(|| Self::typed::<Option<i32>>(property, owner))
    }

    fn value(&self, input: &Value) -> Result<BoxedValue, Error> {
        (self.decode)(input)
            .filter(|v| self.property.is_valid_value(v.as_any()))
            .ok_or_else(|| {
                Error::new(format!(
                    "invalid value {input:?} for FerroUI property {}",
                    self.property.name()
                ))
            })
    }
}

struct Fields {
    names: BTreeMap<String, Field>,
    ids: BTreeMap<u32, String>,
}

impl ControlRegistry {
    /// Register `T` under its real FerroUI type name in a chosen QML module.
    /// Exports inherited and declared bool, int, real, string and nullable
    /// counterparts as lower-camel-case QML properties, including two-way
    /// native notification. Panels expose their `children` default property,
    /// and Button subclasses expose their native `click` signal.
    pub fn register_ferro_control<T: ObjectType + Upcast<Control>>(
        &mut self,
        module: &str,
        create: impl Fn() -> Ref<T> + 'static,
    ) -> Result<(), Error> {
        let ty = T::TYPE;
        let mut fields = Fields {
            names: BTreeMap::new(),
            ids: BTreeMap::new(),
        };
        let mut definition = TypeDefinition::default().inherits("QtQml", "QtObject");
        for &property in ty.properties().iter() {
            if property.is_read_only() {
                continue;
            }
            let Some(field) = Field::from_property(property, ty) else {
                continue;
            };
            let name = property.name();
            let mut chars = name.chars();
            let Some(first) = chars.next() else { continue };
            let name = format!("{}{}", first.to_lowercase(), chars.as_str());
            // Derived metadata wins when a property shadows a base member.
            if fields.names.contains_key(&name) {
                continue;
            }
            definition = definition.property(&name, field.ty, field.default.clone());
            fields.ids.insert(property.id(), name.clone());
            fields.names.insert(name, field);
        }
        if Panel::TYPE.is_assignable_from(ty) {
            definition = definition
                .property("children", "list<QtObject>", Value::List(vec![]))
                .default_property("children")
                .visual_children("children");
        }
        if Button::TYPE.is_assignable_from(ty) {
            definition = definition.signal("click", vec![]);
        }
        let fields = Rc::new(fields);
        self.register(module, ty.name(), definition, move || {
            Box::new(Reflected {
                control: create().upcast(),
                fields: fields.clone(),
            })
        })
    }
}

struct Reflected {
    control: Ref<Control>,
    fields: Rc<Fields>,
}

impl NativeObject for Reflected {
    fn control(&self) -> Ref<Control> {
        self.control.clone()
    }

    fn content(&self) -> Option<(&str, Controls)> {
        self.control
            .cast::<Panel>()
            .map(|panel| ("children", panel.children()))
    }

    fn validate(&self, property: &str, value: &Value) -> Result<(), Error> {
        if let Some(field) = self.fields.names.get(property) {
            field.value(value)?;
        }
        Ok(())
    }

    fn apply_initial(&self, name: &str, value: &Value, assigned: bool) -> Result<(), Error> {
        if assigned {
            self.apply(name, value)?;
        }
        Ok(())
    }

    fn apply(&self, property: &str, value: &Value) -> Result<(), Error> {
        if let Some(field) = self.fields.names.get(property) {
            let boxed = field.value(value)?;
            self.control.set_value_untyped(
                field.property,
                boxed.as_any(),
                BindingPriority::LocalValue,
            );
        }
        Ok(())
    }

    fn connect(&self, endpoint: NativeEndpoint) -> Vec<Rc<dyn IDisposable>> {
        let mut subscriptions = Vec::new();
        if let Some(button) = self.control.cast::<Button>() {
            let click = endpoint.clone();
            let token = button.click(move |_, _| click.emit("click", vec![]));
            subscriptions.push(ferroui_base::reactive::Disposable::create(move || {
                button.remove_handler(Button::click_event(), token);
            }));
        }
        let fields = self.fields.clone();
        subscriptions.push(self.control.property_changed(move |args| {
            if !args.is_effective_value_change() {
                return;
            }
            let Some(name) = fields.ids.get(&args.property().id()) else {
                return;
            };
            if let Some(value) = (fields.names[name].encode)(args.new_value()) {
                endpoint.write(name, value);
            }
        }));
        subscriptions
    }
}
