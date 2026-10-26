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
impl Schema {
    /// Generated from the framework's own class/property reader, with no native dependency.
    pub fn ferroui() -> Self {
        static BUILTIN: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
        BUILTIN
            .get_or_init(|| {
                let definitions: Vec<TypeInfo> = serde_json::from_str(include_str!("schema.json"))
                    .expect("checked generated schema");
                let mut schema = Self::default();
                for definition in definitions {
                    schema.register(definition);
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
        let namespace = if namespace.starts_with("using:FerroUI")
            || namespace.starts_with("clr-namespace:FerroUI")
        {
            FRAMEWORK_NAMESPACE
        } else {
            namespace
        };
        self.types.get(&format!(
            "{namespace}|{}",
            name.rsplit(':').next().unwrap_or(name)
        ))
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
        if let Some((attached, property)) = name.rsplit_once('.') {
            let attached_namespace = if self.type_info(attached, namespace).is_some() {
                namespace
            } else {
                None
            };
            self.properties(attached, attached_namespace)
                .get(property)
                .filter(|p| p.attached)
                .cloned()
        } else {
            self.properties(owner, namespace).get(name).cloned()
        }
    }
}
