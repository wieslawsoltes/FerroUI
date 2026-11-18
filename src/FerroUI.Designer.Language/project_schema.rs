use crate::{LanguageError, PropertyInfo, Result, Schema, TypeInfo, require};
use ferroui_build_scan::model::{AssemblyModel, RegisteredKind, TypeModel};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct ProjectSchema {
    types: Vec<TypeInfo>,
    pub(crate) effective: Schema,
}

impl ProjectSchema {
    pub(crate) fn load(
        base: &Schema,
        overrides: &[TypeInfo],
        directories: &[PathBuf],
    ) -> Result<Self> {
        require(
            directories.len() <= 1024,
            "Too many Cargo build output directories",
        )?;
        let mut paths = BTreeSet::new();
        for directory in directories {
            if !directory.is_absolute() {
                continue;
            }
            let Ok(directory) = directory.canonicalize() else {
                continue;
            };
            let Ok(entries) = directory.read_dir() else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .is_some_and(|extension| extension == "xamlmeta")
                    && entry.file_type().is_ok_and(|kind| kind.is_file())
                {
                    paths.insert(path);
                }
            }
        }
        require(paths.len() <= 256, "Too many XAML metadata files")?;
        let mut assemblies = Vec::new();
        let mut remaining = 64 * 1024 * 1024;
        for path in paths {
            let mut source = String::new();
            std::fs::File::open(&path)
                .and_then(|file| file.take(remaining + 1).read_to_string(&mut source))
                .map_err(|error| {
                    LanguageError(format!("Cannot read {}: {error}", path.display()))
                })?;
            require(
                source.len() as u64 <= remaining,
                "Cargo XAML metadata exceeds 64 MiB",
            )?;
            remaining -= source.len() as u64;
            assemblies.push(AssemblyModel::parse(&source).map_err(|error| {
                LanguageError(format!("Cannot read {}: {error}", path.display()))
            })?);
        }
        let types = catalog(&assemblies);
        let mut configured = Self {
            types,
            effective: base.clone(),
        };
        configured.refresh(base, overrides);
        Ok(configured)
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

pub(crate) fn package_root(manifest: &Path) -> Result<PathBuf> {
    require(
        manifest.is_absolute()
            && manifest
                .file_name()
                .is_some_and(|name| name == "Cargo.toml"),
        "Expected an absolute Cargo manifest",
    )?;
    manifest
        .canonicalize()
        .map_err(|error| LanguageError(error.to_string()))?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| LanguageError("Missing package directory".into()))
}

fn catalog(assemblies: &[AssemblyModel]) -> Vec<TypeInfo> {
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
                    }
                    namespaces
                        .into_iter()
                        .map(|namespace| type_info(item, namespace, &enums))
                        .collect::<Vec<_>>()
                })
        })
        .collect()
}

fn type_info(item: &TypeModel, namespace: String, enums: &BTreeMap<&str, Vec<String>>) -> TypeInfo {
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
        AccessorModel, PropertyModel, RustType, TypeKind, XmlnsDefinitionModel,
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
