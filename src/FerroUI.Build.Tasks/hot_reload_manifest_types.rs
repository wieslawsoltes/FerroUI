//! Build-only event contracts. Never invokes constructors or member accessors.
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use xamlx::type_system::{IXamlType, IXamlTypeSystem};

pub(super) struct Catalog {
    system: Rc<dyn IXamlTypeSystem>,
}
impl Catalog {
    pub(super) fn new(system: Rc<dyn IXamlTypeSystem>) -> Self {
        Self { system }
    }
    pub(super) fn describe(&self, namespace: &str, name: &str) -> Value {
        const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
        // The binding transformer rewrites this name before type resolution,
        // choosing the implementation from the document's CompileBindings scope.
        if namespace == "https://github.com/ferroui" && name == "Binding" {
            let mut compiled = self.describe(namespace, "CompiledBinding");
            let reflection = self.describe(namespace, "ReflectionBinding");
            compiled["name"] = json!(name);
            compiled["known"] = json!(compiled["known"] == true && reflection["known"] == true);
            for (name, signatures) in reflection["events"].as_object().unwrap() {
                let values = compiled["events"]
                    .as_object_mut()
                    .unwrap()
                    .entry(name.clone())
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .unwrap();
                for signature in signatures.as_array().unwrap() {
                    if !values.contains(signature) {
                        values.push(signature.clone());
                    }
                }
                values.sort_by_key(|value| value.as_str().unwrap_or("").to_owned());
            }
            return compiled;
        }
        if namespace == XAML && matches!(name, "Arguments" | "Null") {
            return json!({"namespace": namespace, "name": name, "known": true, "events": {}});
        }
        let using = namespace
            .strip_prefix("using:")
            .or_else(|| namespace.strip_prefix("clr-namespace:"));
        let mut found: Vec<_> = if namespace == XAML {
            self.system
                .find_type(&format!("System.{name}"))
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        for assembly in self.system.assemblies() {
            let mut namespaces = Vec::new();
            if let Some(using) = using {
                let mut parts = using.split(';');
                let namespace = parts.next().unwrap_or("");
                if parts.all(|part| {
                    part.strip_prefix("assembly=").is_some_and(|name| {
                        self.system
                            .find_assembly(name)
                            .is_some_and(|selected| selected.equals(&*assembly))
                    })
                }) {
                    namespaces.push(namespace.to_owned());
                }
            } else {
                for attribute in assembly.custom_attributes() {
                    if attribute.type_().name().trim_end_matches("Attribute") != "XmlnsDefinition" {
                        continue;
                    }
                    let args = attribute.parameters();
                    if args.first().and_then(|a| a.as_str()) == Some(namespace) {
                        if let Some(namespace) = args.get(1).and_then(|a| a.as_str()) {
                            namespaces.push(namespace.to_owned());
                        }
                    }
                }
            }
            for namespace in namespaces {
                let full_name = if namespace.is_empty() {
                    name.to_owned()
                } else {
                    format!("{namespace}.{name}")
                };
                if let Some(type_) = assembly
                    .find_type(&full_name)
                    .or_else(|| assembly.find_type(&format!("{full_name}Extension")))
                {
                    found.push(type_);
                }
            }
        }
        let known = !found.is_empty();
        let mut events: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for type_ in found {
            let mut next = Some(type_);
            let mut visited = BTreeSet::new();
            while let Some(type_) = next {
                if !visited.insert(identity(&*type_)) {
                    break;
                }
                for event in type_.events() {
                    if let Some(add) = event.add() {
                        let signature = format!(
                            "{}({})",
                            identity(&*event.declaring_type()),
                            add.parameters()
                                .iter()
                                .map(|t| identity(&**t))
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                        events.entry(event.name()).or_default().insert(signature);
                    }
                }
                // Routed events are declared as static fields or properties.
                for (name, field_type) in type_
                    .fields()
                    .into_iter()
                    .map(|f| (f.name(), f.field_type()))
                    .chain(
                        type_
                            .properties()
                            .into_iter()
                            .filter(|p| p.getter().is_some_and(|g| g.is_static()))
                            .map(|p| (p.name(), p.property_type())),
                    )
                {
                    if let Some(member) = name.strip_suffix("Event") {
                        if field_type.name().split('`').next() == Some("RoutedEvent") {
                            events
                                .entry(member.into())
                                .or_default()
                                .insert(identity(&*field_type));
                        }
                    }
                }
                next = type_.base_type();
            }
        }
        json!({"namespace": namespace, "name": name, "known": known, "events": events})
    }
}
fn identity(type_: &dyn IXamlType) -> String {
    format!(
        "{}@{}",
        type_.full_name(),
        type_.assembly().map(|a| a.name()).unwrap_or_default()
    )
}
