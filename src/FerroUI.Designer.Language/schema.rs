use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const FRAMEWORK_NAMESPACE: &str = "https://github.com/ferroui";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    pub type_name: String,
    pub documentation: String,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub attached: bool,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub event: bool,
    #[serde(default)]
    pub definition: Option<crate::Location>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub namespace: String,
    pub documentation: String,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub constructible: bool,
    #[serde(default)]
    pub properties: BTreeMap<String, PropertyInfo>,
    #[serde(default)]
    pub definition: Option<crate::Location>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub name: String,
    pub parameters: Vec<String>,
    pub documentation: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Schema {
    pub types: BTreeMap<String, TypeInfo>,
    pub extensions: BTreeMap<String, ExtensionInfo>,
}

#[derive(Deserialize)]
struct GeneratedType {
    #[serde(flatten)]
    info: TypeInfo,
    #[serde(default)]
    namespace_aliases: Vec<String>,
}

fn generated_types() -> &'static [GeneratedType] {
    static TYPES: std::sync::OnceLock<Vec<GeneratedType>> = std::sync::OnceLock::new();
    TYPES.get_or_init(|| {
        serde_json::from_str(include_str!("schema.json")).expect("checked generated schema")
    })
}

fn builtin_aliases() -> &'static BTreeMap<String, String> {
    static ALIASES: std::sync::OnceLock<BTreeMap<String, String>> = std::sync::OnceLock::new();
    ALIASES.get_or_init(|| {
        generated_types()
            .iter()
            .flat_map(|definition| {
                definition.namespace_aliases.iter().map(|namespace| {
                    (
                        format!("{namespace}|{}", definition.info.name),
                        format!("{}|{}", definition.info.namespace, definition.info.name),
                    )
                })
            })
            .collect()
    })
}

impl Schema {
    /// Generated from the framework's own class/property reader, with no native dependency.
    pub fn ferroui() -> Self {
        static BUILTIN: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
        BUILTIN
            .get_or_init(|| {
                let mut schema = Self::default();
                for definition in generated_types() {
                    schema.register(definition.info.clone());
                }
                for (name, parameters, doc) in [
                    (
                        "Binding",
                        vec![
                            "Path",
                            "ElementName",
                            "RelativeSource",
                            "Source",
                            "Mode",
                            "Converter",
                            "ConverterParameter",
                            "FallbackValue",
                            "TargetNullValue",
                            "StringFormat",
                        ],
                        "Bind a property to a data path, named element or explicit source.",
                    ),
                    (
                        "CompiledBinding",
                        vec!["Path", "DataType", "Mode", "ElementName"],
                        "Bind using a statically described data type.",
                    ),
                    (
                        "ReflectionBinding",
                        vec!["Path", "Mode", "ElementName"],
                        "Resolve a binding path through runtime metadata.",
                    ),
                    (
                        "StaticResource",
                        vec!["ResourceKey"],
                        "Resolve a resource by its x:Key.",
                    ),
                    (
                        "DynamicResource",
                        vec!["ResourceKey"],
                        "Observe a resource by its x:Key.",
                    ),
                    (
                        "TemplateBinding",
                        vec!["Property"],
                        "Bind to a property on the templated parent.",
                    ),
                    (
                        "RelativeSource",
                        vec!["Mode", "AncestorType", "AncestorLevel"],
                        "Select a relative binding source.",
                    ),
                    (
                        "x:Reference",
                        vec!["Name"],
                        "Reference an object in the current XAML namescope.",
                    ),
                    ("x:Type", vec!["TypeName"], "Resolve a XAML type."),
                    ("x:Static", vec!["Member"], "Read a static member."),
                    ("x:Null", vec![], "An explicit null value."),
                ] {
                    schema.extensions.insert(
                        name.into(),
                        ExtensionInfo {
                            name: name.into(),
                            parameters: parameters.into_iter().map(str::to_owned).collect(),
                            documentation: doc.into(),
                        },
                    );
                }
                schema
            })
            .clone()
    }
    pub fn register(&mut self, info: TypeInfo) -> Option<TypeInfo> {
        self.types
            .insert(format!("{}|{}", info.namespace, info.name), info)
    }
    pub fn type_info(&self, name: &str, namespace: Option<&str>) -> Option<&TypeInfo> {
        let namespace = namespace.unwrap_or(FRAMEWORK_NAMESPACE);
        let name = name.rsplit(':').next().unwrap_or(name);
        let resolve = |name: &str| {
            let key = format!("{namespace}|{name}");
            self.types.get(&key).or_else(|| {
                builtin_aliases()
                    .get(&key)
                    .and_then(|canonical| self.types.get(canonical))
            })
        };
        // XamlX accepts the omitted Extension suffix in object-element syntax too.
        resolve(name).or_else(|| resolve(&format!("{name}Extension")))
    }
    pub fn properties(
        &self,
        name: &str,
        namespace: Option<&str>,
    ) -> BTreeMap<String, PropertyInfo> {
        let mut result = BTreeMap::new();
        let mut next = self.type_info(name, namespace);
        let mut seen = BTreeSet::new();
        while let Some(info) = next {
            if !seen.insert((info.namespace.clone(), info.name.clone())) {
                break;
            }
            for (name, property) in &info.properties {
                result
                    .entry(name.clone())
                    .or_insert_with(|| property.clone());
            }
            next = info.base.as_ref().and_then(|name| {
                self.type_info(name, Some(&info.namespace))
                    .or_else(|| self.type_info(name, None))
            });
        }
        result
    }
    pub fn property(
        &self,
        owner: &str,
        namespace: Option<&str>,
        name: &str,
    ) -> Option<PropertyInfo> {
        self.property_in_context(owner, namespace, name, namespace)
    }

    pub(crate) fn attribute_property(
        &self,
        document: &ferroui_designer_core::Document,
        node: &ferroui_designer_core::Node,
        attribute: &ferroui_designer_core::Attribute,
    ) -> Option<PropertyInfo> {
        let namespaces = document.ast().namespaces(node.id);
        let member_namespace = if attribute.name.contains(':') {
            attribute.namespace_uri.as_deref()
        } else if attribute.name.contains('.') {
            namespaces.get("").map(String::as_str)
        } else {
            node.namespace_uri.as_deref()
        };
        self.property_in_context(
            &node.name,
            node.namespace_uri.as_deref(),
            &attribute.name,
            member_namespace,
        )
    }

    /// An attached or owner-qualified member can use a different XML namespace
    /// from the target object, especially on a prefixed custom control.
    pub(crate) fn property_in_context(
        &self,
        owner: &str,
        owner_namespace: Option<&str>,
        name: &str,
        member_namespace: Option<&str>,
    ) -> Option<PropertyInfo> {
        if let Some((declaring, property)) = name.rsplit_once('.') {
            let declaring = self.type_info(declaring, member_namespace)?;
            let property = self
                .properties(&declaring.name, Some(&declaring.namespace))
                .get(property)?
                .clone();
            let mut next = self.type_info(owner, owner_namespace);
            let mut seen = BTreeSet::new();
            while let Some(info) = next {
                if !seen.insert((&info.namespace, &info.name)) {
                    break;
                }
                if property.attached
                    || (declaring.namespace == info.namespace && declaring.name == info.name)
                {
                    return Some(property);
                }
                next = info.base.as_ref().and_then(|base| {
                    self.type_info(base, Some(&info.namespace))
                        .or_else(|| self.type_info(base, None))
                });
            }
            // Attached properties also apply to controls with unregistered metadata.
            property.attached.then_some(property)
        } else {
            if name.contains(':') {
                let member_owner = self.type_info(owner, member_namespace)?;
                let target = self.type_info(owner, owner_namespace)?;
                if member_owner.namespace != target.namespace {
                    return None;
                }
            }
            self.properties(owner, owner_namespace)
                .get(name.rsplit(':').next().unwrap_or(name))
                .cloned()
        }
    }
}
