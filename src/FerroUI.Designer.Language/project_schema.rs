use crate::{PropertyInfo, Schema, TypeInfo};
use ferroui_build_scan::model::{RegisteredKind, TypeModel};
use ferroui_designer_project::MarkupCatalog;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ProjectSchema {
    types: Vec<TypeInfo>,
    pub(crate) effective: Schema,
    pub(crate) catalog: Arc<MarkupCatalog>,
}

impl ProjectSchema {
    pub(crate) fn new(base: &Schema, overrides: &[TypeInfo], catalog: Arc<MarkupCatalog>) -> Self {
        let mut configured = Self {
            types: types_from_models(&catalog),
            effective: base.clone(),
            catalog,
        };
        configured.refresh(base, overrides);
        configured
    }

    pub(crate) fn refresh(&mut self, base: &Schema, overrides: &[TypeInfo]) {
        self.effective = base.clone();
        for info in &self.types {
            if base.type_info(&info.name, Some(&info.namespace)).is_none() {
                self.effective.register(info.clone());
            }
        }
        for info in overrides {
            self.effective.register(info.clone());
        }
    }
}

impl Schema {
    /// Add types exported by the selected Cargo package and its dependencies.
    pub fn register_project(&mut self, catalog: &MarkupCatalog) {
        for info in types_from_models(catalog) {
            if self.type_info(&info.name, Some(&info.namespace)).is_none() {
                self.register(info);
            }
        }
    }
}

fn types_from_models(catalog: &MarkupCatalog) -> Vec<TypeInfo> {
    let assemblies = catalog.models().models();
    let enums: BTreeMap<_, _> = assemblies
        .iter()
        .flat_map(|assembly| &assembly.types)
        .filter(|item| !item.enum_members.is_empty())
        .map(|item| {
            (
                item.rust_path.text.as_str(),
                item.enum_members
                    .iter()
                    .map(|member| member.name.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assemblies
        .iter()
        .flat_map(|assembly| {
            assembly
                .types
                .iter()
                .filter(|item| !item.name.is_empty())
                .flat_map(|item| {
                    let mut namespaces: BTreeSet<_> = assembly
                        .xmlns_definitions
                        .iter()
                        .filter(|namespace| namespace.namespace == item.namespace)
                        .map(|namespace| namespace.xml_namespace.clone())
                        .collect();
                    if !item.namespace.is_empty() {
                        namespaces.insert(format!("using:{}", item.namespace));
                        namespaces.insert(format!(
                            "clr-namespace:{};assembly={}",
                            item.namespace, assembly.name
                        ));
                    }
                    namespaces
                        .into_iter()
                        .map(|namespace| type_info(catalog, item, namespace, &enums))
                        .collect::<Vec<_>>()
                })
        })
        .collect()
}

fn type_info(catalog: &MarkupCatalog, item: &TypeModel, namespace: String, enums: &BTreeMap<&str, Vec<String>>) -> TypeInfo {
    let property = |name: &str, value: &str, read_only, attached, event| PropertyInfo {
        name: name.into(),
        type_name: local_type(value).into(),
        read_only,
        attached,
        event,
        values: enums.get(value).cloned().unwrap_or_default(),
        ..Default::default()
    };
    let mut properties = BTreeMap::new();
    for member in &item.properties {
        properties.insert(
            member.name.clone(),
            property(
                &member.name,
                &member.value_type.text,
                member.setter.is_none(),
                false,
                false,
            ),
        );
    }
    for member in &item.registered {
        if let Some(name) = &member.name {
            properties.insert(
                name.clone(),
                property(
                    name,
                    &member.value_type.text,
                    member.read_only,
                    member.kind == RegisteredKind::Attached,
                    false,
                ),
            );
        }
    }
    for event in &item.events {
        properties.insert(
            event.name.clone(),
            property(&event.name, "Event", false, false, true),
        );
    }
    for field in &item.fields {
        if let (Some(name), Some(argument)) = (field.name.strip_suffix("Event"), catalog.routed_event_argument(field)) {
            properties.insert(name.into(), property(name, &argument, false, false, true));
        }
    }
    TypeInfo {
        name: item.name.clone(),
        namespace,
        properties,
        base: item.base.as_ref().map(|base| local_type(&base.text).into()),
        constructible: !item.unregistered
            && (item.default_constructor.is_some()
                || item
                    .constructors
                    .iter()
                    .any(|constructor| constructor.parameters.is_empty())),
        ..Default::default()
    }
}

fn local_type(text: &str) -> &str {
    let text = text.trim();
    for wrapper in ["Option<", "Ref<", "Rc<"] {
        if let Some((prefix, value)) = text.split_once(wrapper) {
            if prefix.is_empty() || prefix.ends_with("::") {
                if let Some(value) = value.strip_suffix('>') {
                    return local_type(value);
                }
            }
        }
    }
    if text.contains('<') {
        text
    } else {
        text.rsplit("::").next().unwrap_or(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LanguageService;
    use ferroui_build_scan::model::{
        AccessorModel, AssemblyModel, PropertyModel, RustType, TypeKind, XmlnsDefinitionModel,
    };
    use ferroui_designer_project::{Solution, file_uri};

    #[test]
    fn metadata_refresh_for_nested_packages_is_scoped_and_workspace_overrides_win() {
        let folder = std::env::temp_dir().join(format!("ferroui-schema-{}", std::process::id()));
        let first = folder.join("first");
        let second = folder.join("second");
        let output = folder.join("out");
        for path in [&first, &second, &output] {
            std::fs::create_dir_all(path).unwrap();
        }
        for (root, name) in [(&first, "first"), (&second, "second")] {
            std::fs::write(
                root.join("Cargo.toml"),
                format!("[package]\nname='{name}'\nversion='0.0.1'\n[lib]\npath='lib.rs'"),
            )
            .unwrap();
            std::fs::write(root.join("lib.rs"), "").unwrap();
            std::fs::write(
                root.join("Card.xaml"),
                "<ui:StatusCard xmlns:ui='urn:test:controls' />",
            )
            .unwrap();
        }
        let mut card = TypeModel::new(
            "StatusCard",
            TypeKind::Class,
            RustType::resolved("first::StatusCard"),
            "first",
        );
        card.namespace = "Test.Controls".into();
        card.properties.push(PropertyModel {
            name: "Caption".into(),
            value_type: RustType::resolved("alloc::string::String"),
            setter: Some(AccessorModel::default()),
            ..Default::default()
        });
        let mut assembly = AssemblyModel::new("First", "first");
        assembly.xmlns_definitions.push(XmlnsDefinitionModel {
            xml_namespace: "urn:test:controls".into(),
            namespace: card.namespace.clone(),
        });
        assembly.types.push(card);
        std::fs::write(output.join("first.xamlmeta"), assembly.to_json()).unwrap();

        let mut service = LanguageService::default();
        service
            .index_solution(&Solution::discover(&folder).unwrap())
            .unwrap();
        let uri = file_uri(first.join("Card.xaml")).unwrap();
        let elsewhere = file_uri(second.join("Card.xaml")).unwrap();
        let nested = first.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("Cargo.toml"), "[package]\nname='nested'\nversion='0.1.0'\n").unwrap();
        let nested_uri = file_uri(nested.join("Draft.xaml")).unwrap();
        let manifest = first.join("Cargo.toml");
        service
            .set_project_metadata(&manifest, std::slice::from_ref(&output))
            .unwrap();
        let get = |schema: &Schema| {
            schema
                .type_info("StatusCard", Some("urn:test:controls"))
                .cloned()
        };
        let mut override_type = get(service.schema_for(&uri)).unwrap();
        assert_eq!(override_type.properties["Caption"].type_name, "String");
        assert!(get(service.schema_for(&elsewhere)).is_none());
        assert!(get(service.schema_for(&nested_uri)).is_none());
        override_type
            .properties
            .get_mut("Caption")
            .unwrap()
            .type_name = "Override".into();
        service.set_type_overrides(vec![override_type]);
        assert_eq!(
            get(service.schema_for(&uri)).unwrap().properties["Caption"].type_name,
            "Override"
        );
        service.set_type_overrides(vec![]);
        service.set_project_metadata(&manifest, &[]).unwrap();
        assert!(get(service.schema_for(&uri)).is_none());
        service
            .set_project_metadata(&manifest, std::slice::from_ref(&output))
            .unwrap();
        service.remove_solution(&folder);
        assert!(get(service.schema_for(&uri)).is_none());
        assert!(service.set_project_metadata(&manifest, &[]).is_err());
        std::fs::remove_dir_all(folder).unwrap();
    }
}
