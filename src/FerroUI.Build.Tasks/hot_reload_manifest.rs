use serde_json::json;
use std::path::{Path, PathBuf};
#[path = "hot_reload_manifest_types.rs"]
mod types;
pub(super) fn write(
    manifest_dir: &Path,
    root: &Path,
    out: &Path,
    assembly: &str,
    system: std::rc::Rc<dyn xamlx::type_system::IXamlTypeSystem>,
) -> Result<PathBuf, String> {
    let catalog = types::Catalog::new(system);
    let mut files = Vec::new();
    super::collect(manifest_dir, root, &mut files)?;
    let mut documents = Vec::new();
    let mut contracts = std::collections::BTreeMap::new();
    for (relative, path) in files {
        if path.extension().and_then(|value| value.to_str()) != Some("xaml") {
            continue;
        }
        let source = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let xml = roxmltree::Document::parse(&source)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let mut names = Vec::new();
        let mut data_types = Vec::new();
        let mut attributes = Vec::new();
        let mut includes = Vec::new();
        let mut handlers = Vec::new();
        for node in xml.descendants().filter(|node| node.is_element()) {
            if !node.tag_name().name().contains('.') {
                let namespace = node.tag_name().namespace().unwrap_or("");
                let name = node.tag_name().name();
                contracts
                    .entry((namespace.to_string(), name.to_string()))
                    .or_insert_with(|| catalog.describe(namespace, name));
            }
            for attr in node.attributes() {
                let xaml = attr.namespace() == Some("http://schemas.microsoft.com/winfx/2006/xaml");
                if attr.name() == "Name" && (xaml || attr.namespace().is_none()) {
                    names.push(json!({"name": attr.value(), "type": node.tag_name().name(), "namespace": node.tag_name().namespace()}));
                }
                if attr.name() == "DataType" && xaml {
                    data_types.push(expanded_data_type(node, attr.value()));
                }
                if attr.name() == "TypeArguments" && xaml {
                    data_types.push(format!(
                        "type-arguments:{}",
                        expanded_data_type(node, attr.value())
                    ));
                }
                // Preserve every member for metadata-based handler classification by consumers.
                attributes.push(json!({"type": node.tag_name().name(), "namespace": node.tag_name().namespace(), "member": attr.name(), "value": attr.value()}));
                if attr.namespace().is_none() || attr.name().contains('.') {
                    let (namespace, owner, member) =
                        if let Some((owner, member)) = attr.name().rsplit_once('.') {
                            (
                                attr.namespace()
                                    .or_else(|| node.lookup_namespace_uri(None))
                                    .unwrap_or(""),
                                owner,
                                member,
                            )
                        } else {
                            (
                                node.tag_name().namespace().unwrap_or(""),
                                node.tag_name().name(),
                                attr.name(),
                            )
                        };
                    let contract = contracts
                        .entry((namespace.to_string(), owner.to_string()))
                        .or_insert_with(|| catalog.describe(namespace, owner));
                    if let Some(signature) = contract["events"].get(member) {
                        handlers.push(
                            json!({"namespace": namespace, "owner": owner, "member": member,
                            "handler": attr.value(), "signature": signature}),
                        );
                    }
                }
            }
            if matches!(
                node.tag_name().name(),
                "StyleInclude" | "ResourceInclude" | "MergeResourceInclude"
            ) {
                if let Some(source) = node.attribute("Source") {
                    includes.push(source);
                }
            }
        }
        documents.push(json!({"uri": format!("ferres://{assembly}/{}", relative.trim_start_matches('/')),
            "source_path": path.canonicalize().map_err(|error| error.to_string())?, "assembly": assembly,
            "class": xml.root_element().attribute(("http://schemas.microsoft.com/winfx/2006/xaml", "Class")),
            "source": source, "names": names, "data_types": data_types, "members": attributes,
            "handlers": handlers, "includes": includes }));
    }
    let path = out.join("hot-reload.json");
    std::fs::create_dir_all(out).map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(&json!({"version": 1, "documents": documents,
        "type_contracts": contracts.into_values().collect::<Vec<_>>() }))
    .map_err(|error| error.to_string())?;
    if std::fs::read_to_string(&path).ok().as_deref() != Some(&text) {
        std::fs::write(&path, text).map_err(|error| error.to_string())?;
    }
    Ok(path)
}

fn expanded_data_type(node: roxmltree::Node<'_, '_>, value: &str) -> String {
    let value = value.trim();
    if !value.contains(['{', '}', ' ', ',']) {
        let (prefix, name) = value
            .split_once(':')
            .map_or((None, value), |(prefix, name)| (Some(prefix), name));
        if let Some(namespace) = node.lookup_namespace_uri(prefix) {
            return format!("{{{namespace}}}{name}");
        }
    }
    let mut namespaces: Vec<_> = node
        .namespaces()
        .map(|ns| (ns.name().unwrap_or(""), ns.uri()))
        .collect();
    namespaces.sort();
    format!("{value}|{namespaces:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::AssemblyModel, type_system::ModelTypeSystem};
    #[cfg(feature = "runtime-host")]
    #[test]
    fn runtime_host_uses_the_same_contract_classifier_without_invoking_accessors() {
        use ferroui_base::metadata::{
            MarkupAssembly, MarkupEvent, MarkupType, MarkupTypeKind, XmlnsDefinition,
        };
        static ASSEMBLY: MarkupAssembly = MarkupAssembly {
            name: "Manifest.Runtime",
            crate_name: "manifest_runtime",
            xmlns_definitions: &[XmlnsDefinition {
                xml_namespace: "urn:manifest-runtime",
                namespace: "Manifest.Runtime",
            }],
            xmlns_prefixes: &[],
            metadata: &[],
        };
        static CUSTOM: MarkupType = MarkupType {
            explicit_namespace: Some("Manifest.Runtime"),
            events: &[MarkupEvent {
                name: "Recomputed",
                arguments: &[],
                add: |_| panic!("manifest generation must not invoke event accessors"),
            }],
            ..MarkupType::new("CustomExtension", MarkupTypeKind::Class, "manifest_runtime")
        };
        MarkupAssembly::register(&ASSEMBLY);
        MarkupType::register(&CUSTOM);
        ferroui_markup_xaml::register_types();
        let catalog = types::Catalog::new(
            ferroui_markup_xaml_loader::runtime::type_system::RuntimeTypeSystem::new(),
        );
        for namespace in [
            "urn:manifest-runtime",
            "clr-namespace:Manifest.Runtime;assembly=manifest.runtime",
        ] {
            let description = catalog.describe(namespace, "Custom");
            assert_eq!(description["known"], true);
            assert!(description["events"]["Recomputed"]
                .as_array()
                .is_some_and(|values| !values.is_empty()));
        }
        assert_eq!(
            catalog.describe("urn:manifest-runtime", "Missing")["known"],
            false
        );
        assert_eq!(
            catalog.describe("http://schemas.microsoft.com/winfx/2006/xaml", "String")["known"],
            true
        );
        let binding = catalog.describe("https://github.com/ferroui", "Binding");
        assert_eq!(binding["known"], true);
        assert!(binding["events"].as_object().unwrap().is_empty());
    }
    #[test]
    fn build_metadata_records_custom_inherited_and_qualified_handlers() {
        use crate::model::{
            MemberModel, ParameterModel, RustType, TypeKind, TypeModel, XmlnsDefinitionModel,
        };
        let mut model = AssemblyModel::new("Demo", "demo");
        model.xmlns_definitions.push(XmlnsDefinitionModel {
            xml_namespace: "urn:demo".into(),
            namespace: "Demo".into(),
        });
        let mut base = TypeModel::new(
            "Base",
            TypeKind::Class,
            RustType::resolved("::demo::Base"),
            "demo",
        );
        base.namespace = "Demo".into();
        base.handles.push(base.rust_path.clone());
        base.events.push(MemberModel {
            name: "Recomputed".into(),
            parameters: vec![ParameterModel {
                type_: RustType::resolved("bool"),
                ..Default::default()
            }],
            ..Default::default()
        });
        let mut child = TypeModel::new(
            "Child",
            TypeKind::Class,
            RustType::resolved("::demo::Child"),
            "demo",
        );
        child.namespace = "Demo".into();
        child.handles.push(child.rust_path.clone());
        child.base = Some(RustType::resolved("::demo::Base"));
        let mut routed = TypeModel::new(
            "RoutedEvent",
            TypeKind::Class,
            RustType::resolved("::demo::RoutedEvent"),
            "demo",
        );
        routed.namespace = "Demo".into();
        routed.handles.push(routed.rust_path.clone());
        base.fields.push(MemberModel {
            name: "ChangedEvent".into(),
            is_static: true,
            return_type: Some(RustType::resolved("::demo::RoutedEvent")),
            ..Default::default()
        });
        model.types = vec![base, child, routed];
        let root =
            std::env::temp_dir().join(format!("ferroui-manifest-events-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("View.xaml"), r#"<Child xmlns="urn:demo" xmlns:a="using:Demo;assembly=Demo" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Recomputed="OnFirst" a:Base.Recomputed="OnSecond" Changed="OnRouted" x:DataType="a:Model" Width="42"/>"#).unwrap();
        std::fs::copy(root.join("View.xaml"), root.join("Second.xaml")).unwrap();
        let output = write(
            &root,
            &root,
            &root.join("generated"),
            "Demo",
            ModelTypeSystem::new(vec![model]),
        )
        .unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(output).unwrap()).unwrap();
        let handlers = value["documents"][0]["handlers"].as_array().unwrap();
        assert_eq!(handlers.len(), 3);
        assert_eq!(handlers[0]["handler"], "OnFirst");
        assert_eq!(handlers[1]["handler"], "OnSecond");
        assert!(handlers[0]["signature"][0]
            .as_str()
            .unwrap()
            .contains("System.Boolean"));
        assert_eq!(handlers[2]["handler"], "OnRouted");
        assert!(handlers[2]["signature"][0]
            .as_str()
            .unwrap()
            .contains("Demo.RoutedEvent"));
        assert_eq!(
            value["documents"][0]["data_types"][0],
            "{using:Demo;assembly=Demo}Model"
        );
        assert_eq!(value["documents"].as_array().unwrap().len(), 2);
        assert_eq!(
            value["type_contracts"].as_array().unwrap().len(),
            2,
            "type metadata must be shared across documents"
        );
        assert!(value["type_contracts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|contract| contract["known"] == true));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn manifest_records_the_built_source_and_include_contract() {
        let root = std::env::temp_dir().join(format!("ferroui-manifest-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let source = r#"<Border xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Demo.View" x:Name="Root"><Border.Styles><StyleInclude Source="Theme.xaml"/></Border.Styles></Border>"#;
        std::fs::write(root.join("View.xaml"), source).unwrap();
        let output = write(
            &root,
            &root,
            &root.join("generated"),
            "Demo",
            ModelTypeSystem::new(vec![]),
        )
        .unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(output).unwrap()).unwrap();
        assert_eq!(value["version"], 1);
        assert_eq!(value["documents"][0]["class"], "Demo.View");
        assert_eq!(value["documents"][0]["source"], source);
        assert_eq!(value["documents"][0]["includes"][0], "Theme.xaml");
        std::fs::remove_dir_all(root).unwrap();
    }
}
