//! Deterministic, immutable design records. No application viewmodel constructor or
//! setter is invoked to populate sample data; ordinary preview bindings read this plugin.
use ferroui_base::data::core::plugins::{
    AccessorListener, BindingPlugins, IPropertyAccessor, IPropertyAccessorPlugin,
};
use ferroui_base::data::core::{ValueType, ValueTypes, WeakValue};
use ferroui_base::data::{BindingError, BindingPriority};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{AnyValue, BoxedValue, FerroPropertyRegistry, Ref, StyledElement, Visual};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeId};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub const MAX_SAMPLE_ITEMS: usize = 200;
const MAX_FIELDS: usize = 128;
const MAX_PATH_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SampleKind {
    Text,
    Boolean,
    Number,
}

#[derive(Clone, Debug)]
pub(crate) struct SampleRequest {
    pub node: NodeId,
    pub property: String,
    pub count: usize,
    fields: BTreeMap<String, SampleKind>,
}

pub(crate) fn sample_count(
    document: &Document,
    node: NodeId,
    value: &str,
) -> Result<Option<usize>, String> {
    let Ok(MarkupValue::Extension(extension)) = MarkupValue::parse(value) else {
        return Ok(None);
    };
    let Some((prefix, name)) = extension.name.split_once(':') else {
        return Ok(None);
    };
    if !matches!(name, "SampleData" | "SampleDataExtension")
        || document
            .ast()
            .namespaces(node)
            .get(prefix)
            .map(String::as_str)
            != Some(crate::design_time::DESIGN_NAMESPACE)
    {
        return Ok(None);
    }
    let mut count = 5;
    for argument in &extension.arguments {
        if argument.name.as_deref() != Some("ItemCount") {
            return Err("Design SampleData accepts only ItemCount.".into());
        }
        let MarkupValue::Literal(value) = &argument.value else {
            return Err("Sample ItemCount must be an integer.".into());
        };
        count = value
            .parse::<usize>()
            .map_err(|_| "Sample ItemCount must be an integer.")?;
    }
    if count > MAX_SAMPLE_ITEMS {
        return Err(format!(
            "Sample ItemCount must be between 0 and {MAX_SAMPLE_ITEMS}."
        ));
    }
    Ok(Some(count))
}

pub(crate) fn request(
    document: &Document,
    node: NodeId,
    property: &str,
    count: usize,
    notices: &mut Vec<String>,
) -> SampleRequest {
    let mut fields = BTreeMap::new();
    let owner = document.ast().node(node).unwrap();
    let mut roots = vec![node];
    if property == "ItemsSource" {
        // Inline templates and column bindings are in the same source subtree. A
        // referenced local DataTemplate can be found without loading application code.
        if let Some(attribute) = owner.attribute_info("ItemTemplate") {
            if let Ok(MarkupValue::Extension(extension)) = attribute.markup() {
                let (prefix, local) = extension
                    .name
                    .split_once(':')
                    .unwrap_or(("", &extension.name));
                if local == "StaticResource"
                    && document
                        .ast()
                        .namespaces(node)
                        .get(prefix)
                        .is_some_and(|uri| uri == "https://github.com/ferroui")
                {
                    if let Some(MarkupValue::Literal(key)) =
                        extension.arguments.first().map(|argument| &argument.value)
                    {
                        if let Ok(resources) =
                            ferroui_designer_tools::visible_component_resources(document, node)
                        {
                            if let Some(resource) = resources
                                .iter()
                                .find(|resource| resource.key.as_deref() == Some(key))
                            {
                                roots.push(resource.node);
                            }
                        }
                    }
                }
            }
        }
    }
    for root in roots {
        for element in document
            .ast()
            .subtree(root)
            .filter(|element| element.is_element())
        {
            for attribute in &element.attributes {
                if attribute.namespace_uri.as_deref() == Some(crate::design_time::DESIGN_NAMESPACE)
                {
                    continue;
                }
                let Ok(MarkupValue::Extension(extension)) = attribute.markup() else {
                    continue;
                };
                let (prefix, name) = extension
                    .name
                    .split_once(':')
                    .unwrap_or(("", &extension.name));
                if !matches!(name, "Binding" | "ReflectionBinding")
                    || document
                        .ast()
                        .namespaces(element.id)
                        .get(prefix)
                        .is_none_or(|uri| uri != "https://github.com/ferroui")
                {
                    continue;
                }
                if extension.arguments.iter().any(|argument| {
                    matches!(
                        argument.name.as_deref(),
                        Some("Source" | "ElementName" | "RelativeSource")
                    )
                }) {
                    continue;
                }
                let Some(MarkupValue::Literal(path)) = extension
                    .arguments
                    .iter()
                    .find(|argument| {
                        argument.name.is_none() || argument.name.as_deref() == Some("Path")
                    })
                    .map(|argument| &argument.value)
                else {
                    continue;
                };
                if path.is_empty() || path == "." {
                    continue;
                }
                if !simple_path(path) {
                    notices.push(format!("Sample data leaves complex binding path '{path}' to explicit design values."));
                    continue;
                }
                if fields.len() >= MAX_FIELDS {
                    continue;
                }
                let kind = property_kind(
                    element,
                    attribute.name.rsplit(':').next().unwrap_or(&attribute.name),
                );
                fields.entry(path.clone()).or_insert(kind);
            }
        }
    }
    SampleRequest {
        node,
        property: property.into(),
        count,
        fields,
    }
}

fn simple_path(path: &str) -> bool {
    path.len() <= 1024
        && path.split('.').count() <= MAX_PATH_DEPTH
        && path.split('.').all(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .is_some_and(|character| character.is_alphabetic() || character == '_')
                && characters.all(|character| character.is_alphanumeric() || character == '_')
        })
}

fn property_kind(node: &Node, property: &str) -> SampleKind {
    let native = crate::editors::markup_type_for_node(node)
        .and_then(|type_| type_.type_info)
        .map(|type_| type_())
        .and_then(|type_| FerroPropertyRegistry::instance().find_registered(type_, property));
    let name = native.map_or("", |property| property.property_type_name());
    if name == "bool" || name.ends_with("Option<bool>") {
        SampleKind::Boolean
    } else if matches!(
        name,
        "u8" | "u16"
            | "u32"
            | "u64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "f32"
            | "f64"
            | "usize"
            | "isize"
    ) {
        SampleKind::Number
    } else {
        SampleKind::Text
    }
}

#[derive(Clone, PartialEq)]
struct DesignRecord {
    fields: BTreeMap<String, SampleValue>,
}
#[derive(Clone, PartialEq)]
enum SampleValue {
    Text(String),
    Boolean(bool),
    Number(f64),
    Record(DesignRecord),
}
impl SampleValue {
    fn boxed(&self) -> BoxedValue {
        match self {
            Self::Text(value) => Rc::new(value.clone()),
            Self::Boolean(value) => Rc::new(*value),
            Self::Number(value) => Rc::new(*value),
            Self::Record(value) => Rc::new(value.clone()),
        }
    }
    fn value_type(&self) -> ValueType {
        match self {
            Self::Text(_) => ValueType::of::<String>(),
            Self::Boolean(_) => ValueType::of::<bool>(),
            Self::Number(_) => ValueType::of::<f64>(),
            Self::Record(_) => ValueType::of::<DesignRecord>(),
        }
    }
}

impl DesignRecord {
    fn insert(&mut self, path: &[&str], value: SampleValue) {
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

impl SampleRequest {
    pub fn value(&self) -> BoxedValue {
        register_accessors();
        if self.property == "DataContext" {
            self.record(0)
        } else {
            Rc::new(Some(ferroui_controls::ItemsSource::from_items(
                (0..self.count).map(|row| Some(self.record(row))),
            )))
        }
    }
    fn record(&self, row: usize) -> BoxedValue {
        if self.fields.is_empty() {
            return Rc::new(format!("Sample item {}", row + 1));
        }
        let mut record = DesignRecord {
            fields: BTreeMap::new(),
        };
        for (path, kind) in &self.fields {
            let parts = path.split('.').collect::<Vec<_>>();
            let value = match kind {
                SampleKind::Boolean => SampleValue::Boolean(row % 2 == 0),
                SampleKind::Number => SampleValue::Number((row + 1) as f64 * 10.0),
                SampleKind::Text => SampleValue::Text(format!(
                    "{} {}",
                    display_name(parts.last().unwrap()),
                    row + 1
                )),
            };
            record.insert(&parts, value);
        }
        Rc::new(record)
    }
}

pub(crate) fn apply(visual: &Ref<Visual>, sample: &SampleRequest) -> Result<(), String> {
    let value = sample.value();
    if sample.property == "DataContext" {
        let element = visual
            .cast::<StyledElement>()
            .ok_or("Design sample data requires a styled element.")?;
        element.set_data_context(Some(value));
        return Ok(());
    }
    let property = FerroPropertyRegistry::instance()
        .find_registered(visual.get_type(), &sample.property)
        .ok_or_else(|| {
            format!(
                "{} has no registered {} property for sample data.",
                visual.get_type().name(),
                sample.property
            )
        })?;
    let target = ValueType::new(property.property_type(), property.property_type_name());
    let converted = ValueTypes::try_convert(Some(&value), target)
        .flatten()
        .ok_or_else(|| {
            format!(
                "{} needs explicit design-time items for its custom collection type.",
                visual.get_type().name()
            )
        })?;
    visual.set_value_untyped(property, converted.as_any(), BindingPriority::LocalValue);
    Ok(())
}
fn display_name(name: &str) -> String {
    let mut result = String::new();
    for (index, character) in name.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            result.push(' ');
        }
        result.push(character);
    }
    result
}

fn register_accessors() {
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
        Some(self.value.value_type())
    }
    fn value(&self) -> Option<BoxedValue> {
        Some(self.value.boxed())
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
