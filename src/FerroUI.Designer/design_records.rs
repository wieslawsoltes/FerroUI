//! Read-only binding values shared by inferred samples and authored fixtures.
use ferroui_base::data::core::plugins::{
    AccessorListener, BindingPlugins, IPropertyAccessor, IPropertyAccessorPlugin,
};
use ferroui_base::data::core::{ValueType, WeakValue};
use ferroui_base::data::{BindingError, BindingPriority};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{AnyValue, BoxedValue};
use ferroui_controls::ItemsSource;
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DesignRecord {
    fields: BTreeMap<String, SampleValue>,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SampleValue {
    Text(String),
    Boolean(bool),
    Number(f64),
    Integer(i64),
    Unsigned(u64),
    Record(DesignRecord),
    Items(Vec<SampleValue>),
    Null,
}
impl SampleValue {
    fn from_json(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(value) => Self::Boolean(*value),
            Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Self::Integer(value)
                } else if let Some(value) = value.as_u64() {
                    Self::Unsigned(value)
                } else {
                    Self::Number(value.as_f64().expect("validated JSON number"))
                }
            }
            Value::String(value) => Self::Text(value.clone()),
            Value::Array(items) => Self::Items(items.iter().map(Self::from_json).collect()),
            Value::Object(fields) => Self::Record(DesignRecord::from_json(fields)),
        }
    }

    fn boxed(&self) -> Option<BoxedValue> {
        Some(match self {
            Self::Text(value) => Rc::new(value.clone()),
            Self::Boolean(value) => Rc::new(*value),
            Self::Number(value) => Rc::new(*value),
            Self::Integer(value) => Rc::new(*value),
            Self::Unsigned(value) => Rc::new(*value),
            Self::Record(value) => Rc::new(value.clone()),
            Self::Items(items) => {
                Rc::new(Some(ItemsSource::from_items(items.iter().map(Self::boxed))))
            }
            Self::Null => return None,
        })
    }

    fn value_type(&self) -> Option<ValueType> {
        Some(match self {
            Self::Text(_) => ValueType::of::<String>(),
            Self::Boolean(_) => ValueType::of::<bool>(),
            Self::Number(_) => ValueType::of::<f64>(),
            Self::Integer(_) => ValueType::of::<i64>(),
            Self::Unsigned(_) => ValueType::of::<u64>(),
            Self::Record(_) => ValueType::of::<DesignRecord>(),
            Self::Items(_) => ValueType::of::<Option<ItemsSource>>(),
            Self::Null => return None,
        })
    }
}

impl DesignRecord {
    pub(crate) fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }

    pub(crate) fn from_json(fields: &Map<String, Value>) -> Self {
        Self {
            fields: fields
                .iter()
                .map(|(name, value)| (name.clone(), SampleValue::from_json(value)))
                .collect(),
        }
    }

    pub(crate) fn insert(&mut self, path: &[&str], value: SampleValue) {
        if path.len() == 1 {
            self.fields.entry(path[0].into()).or_insert(value);
            return;
        }
        let nested = self.fields.entry(path[0].into()).or_insert_with(|| {
            SampleValue::Record(DesignRecord {
                fields: BTreeMap::new(),
            })
        });
        if let SampleValue::Record(record) = nested {
            record.insert(&path[1..], value);
        }
    }
}

pub(crate) fn register_accessors() {
    let probe = DesignRecord {
        fields: BTreeMap::from([(
            "__FerroDesignerSampleProbe".into(),
            SampleValue::Text(String::new()),
        )]),
    };
    BindingPlugins::update_property_accessors(|plugins| {
        if !plugins
            .iter()
            .any(|plugin| plugin.match_(&probe, "__FerroDesignerSampleProbe"))
        {
            plugins.insert(0, Rc::new(SampleAccessorPlugin));
        }
    });
}
struct SampleAccessorPlugin;
impl IPropertyAccessorPlugin for SampleAccessorPlugin {
    fn match_(&self, object: &dyn AnyValue, name: &str) -> bool {
        object
            .downcast_ref::<DesignRecord>()
            .is_some_and(|record| record.fields.contains_key(name))
    }
    fn start(&self, reference: &WeakValue, name: &str) -> Option<Rc<dyn IPropertyAccessor>> {
        let object = reference.upgrade()?;
        let value = object
            .downcast_ref::<DesignRecord>()?
            .fields
            .get(name)?
            .clone();
        Some(Rc::new(SampleAccessor {
            value,
            listener: RefCell::new(None),
        }))
    }
}
struct SampleAccessor {
    value: SampleValue,
    listener: RefCell<Option<AccessorListener>>,
}
impl IDisposable for SampleAccessor {
    fn dispose(&self) {
        self.listener.borrow_mut().take();
    }
}
impl IPropertyAccessor for SampleAccessor {
    fn property_type(&self) -> Option<ValueType> {
        self.value.value_type()
    }
    fn value(&self) -> Option<BoxedValue> {
        self.value.boxed()
    }
    fn set_value(&self, _: Option<&BoxedValue>, _: BindingPriority) -> Result<bool, BindingError> {
        Ok(false)
    }
    fn subscribe(&self, listener: AccessorListener) {
        *self.listener.borrow_mut() = Some(listener.clone());
        listener(self.value());
    }
    fn unsubscribe(&self) {
        self.listener.borrow_mut().take();
    }
}
