use crate::ReloadError;
use ferroui_base::metadata::{MarkupAssembly, MarkupType};
/// Frozen build metadata supplements runtime registration, which may be lazy.
#[derive(Default)]
pub(crate) struct BuildContracts {
    types: std::collections::BTreeMap<
        (String, String),
        (bool, std::collections::BTreeMap<String, Vec<String>>),
    >,
}
impl BuildContracts {
    pub(crate) fn insert(&mut self, value: &serde_json::Value) -> Result<(), ReloadError> {
        let text = |key| {
            value[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| ReloadError::Failed(format!("Missing type contract {key}")))
        };
        let key = (text("namespace")?, text("name")?);
        let known = value["known"]
            .as_bool()
            .ok_or_else(|| ReloadError::Failed("Missing type contract resolution".into()))?;
        let descriptions = value["events"]
            .as_object()
            .ok_or_else(|| ReloadError::Failed("Missing type contract events".into()))?;
        for signatures in descriptions.values() {
            if !signatures.as_array().is_some_and(|values| {
                !values.is_empty()
                    && values
                        .iter()
                        .all(|value| value.as_str().is_some_and(|value| !value.is_empty()))
            }) {
                return Err(ReloadError::Failed("Invalid handler signature".into()));
            }
        }
        let events = descriptions
            .iter()
            .map(|(name, values)| {
                let mut signatures: Vec<_> = values
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap().to_owned())
                    .collect();
                signatures.sort();
                signatures.dedup();
                (name.clone(), signatures)
            })
            .collect();
        let contract = (known, events);
        if self
            .types
            .get(&key)
            .is_some_and(|previous| previous != &contract)
        {
            return Err(ReloadError::Failed(format!(
                "Conflicting type contract: {key:?}"
            )));
        }
        self.types.insert(key, contract);
        Ok(())
    }
    pub(crate) fn fingerprint(
        &self,
        source: &str,
    ) -> Result<Vec<(String, String, String)>, ReloadError> {
        let xml =
            roxmltree::Document::parse(source).map_err(|e| ReloadError::Failed(e.to_string()))?;
        let mut members = Vec::new();
        for node in xml.descendants().filter(|n| n.is_element()) {
            let ns = node.tag_name().namespace().unwrap_or("");
            let name = node.tag_name().name();
            if !name.contains('.') && !self.known(ns, name) {
                members.push((
                    format!("{{{ns}}}{name}"),
                    "#unresolved".into(),
                    String::new(),
                ));
            }
            for attr in node.attributes() {
                if attr.namespace().is_some() && !attr.name().contains('.') {
                    continue;
                }
                let (ns, owner, member) = member_owner(node, attr);
                let built_event = self
                    .types
                    .get(&(ns.into(), owner.into()))
                    .is_some_and(|(_, events)| events.contains_key(member));
                if built_event || !self.known(ns, owner) {
                    members.push((
                        format!("{{{ns}}}{owner}"),
                        member.into(),
                        attr.value().into(),
                    ));
                }
            }
        }
        members.sort();
        Ok(members)
    }
    fn known(&self, namespace: &str, name: &str) -> bool {
        self.types
            .get(&(namespace.into(), name.into()))
            .map_or_else(|| !types(namespace, name).is_empty(), |(known, _)| *known)
    }
    fn event(
        &self,
        node: roxmltree::Node<'_, '_>,
        attr: roxmltree::Attribute<'_, '_>,
    ) -> Option<bool> {
        if attr.namespace().is_some() && !attr.name().contains('.') {
            return Some(false);
        }
        let (ns, owner, member) = member_owner(node, attr);
        self.types
            .get(&(ns.into(), owner.into()))
            .map(|(_, events)| events.contains_key(member))
    }
}
/// Conservative structural contract. Expanded XML names avoid prefix-only false changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentContract {
    pub class: Option<String>,
    pub names: Vec<(String, String, String)>,
    pub data_types: Vec<String>,
    pub events: Vec<(String, String)>,
}
impl DocumentContract {
    pub fn parse(source: &str) -> Result<Self, ReloadError> {
        Self::parse_with_build(source, None)
    }
    pub(crate) fn parse_with_build(
        source: &str,
        build: Option<&BuildContracts>,
    ) -> Result<Self, ReloadError> {
        let xml = roxmltree::Document::parse(source)
            .map_err(|error| ReloadError::Failed(error.to_string()))?;
        let mut result = Self {
            class: None,
            names: Vec::new(),
            data_types: Vec::new(),
            events: Vec::new(),
        };
        for node in xml.descendants().filter(|node| node.is_element()) {
            for attr in node.attributes() {
                let xaml = attr.namespace() == Some("http://schemas.microsoft.com/winfx/2006/xaml");
                let built_event = build.and_then(|build| build.event(node, attr));
                match attr.name() {
                    "Class" if xaml => result.class = Some(attr.value().into()),
                    "Name" if xaml || attr.namespace().is_none() => result.names.push((
                        attr.value().into(),
                        node.tag_name().namespace().unwrap_or("").into(),
                        node.tag_name().name().into(),
                    )),
                    "DataType" if xaml => result.data_types.push(data_type(node, attr.value())),
                    "TypeArguments" if xaml => result
                        .data_types
                        .push(format!("type-arguments:{}", data_type(node, attr.value()))),
                    // Fallback for documents parsed before their runtime types are registered.
                    "Click"
                    | "Loaded"
                    | "Unloaded"
                    | "AttachedToVisualTree"
                    | "DetachedFromVisualTree"
                    | "PointerPressed"
                    | "PointerReleased"
                    | "KeyDown"
                    | "KeyUp"
                    | "SelectionChanged"
                    | "TextChanged"
                    | "Checked"
                    | "Unchecked"
                        if built_event.unwrap_or(true) =>
                    {
                        result
                            .events
                            .push((attr.name().into(), attr.value().into()))
                    }
                    _ if built_event.unwrap_or_else(|| metadata_event(node, attr)) => result
                        .events
                        .push((expanded_member(node, attr), attr.value().into())),
                    _ => {}
                }
            }
        }
        result.names.sort();
        result.data_types.sort();
        result.events.sort();
        Ok(result)
    }
}

// Namespace rebinding can change a compiled-binding type without changing the
// attribute text. Compare expanded type names, not just the spelling of a prefix.
fn data_type(node: roxmltree::Node<'_, '_>, value: &str) -> String {
    let value = value.trim();
    if !value.contains(['{', '}', ' ', ',']) {
        let (prefix, name) = value
            .split_once(':')
            .map_or((None, value), |(prefix, name)| (Some(prefix), name));
        if let Some(namespace) = node.lookup_namespace_uri(prefix) {
            return format!("{{{namespace}}}{name}");
        }
    }
    // Complex type expressions remain conservatively sensitive to namespace edits.
    let mut namespaces: Vec<_> = node
        .namespaces()
        .map(|ns| (ns.name().unwrap_or(""), ns.uri()))
        .collect();
    namespaces.sort();
    format!("{value}|{namespaces:?}")
}

// Match the namespace forms accepted by XAML without invoking constructors or getters.
// Ambiguous mappings are deliberately a union: a possible event must not evade admission.
fn types(namespace: &str, name: &str) -> Vec<&'static MarkupType> {
    let using = namespace
        .strip_prefix("using:")
        .or_else(|| namespace.strip_prefix("clr-namespace:"));
    let assemblies = MarkupAssembly::registered_assemblies();
    MarkupType::registered_types()
        .into_iter()
        .chain(
            ferroui_base::TypeInfo::registered_types()
                .into_iter()
                .filter_map(|type_| type_.markup()),
        )
        .filter(|type_| {
            if type_.name != name {
                return false;
            }
            if let Some(using) = using {
                let mut parts = using.split(';');
                if parts.next() != Some(type_.namespace()) {
                    return false;
                }
                return parts.all(|part| {
                    part.strip_prefix("assembly=").is_some_and(|name| {
                        MarkupAssembly::of_module(type_.module_path)
                            .is_some_and(|a| a.name.eq_ignore_ascii_case(name))
                    })
                });
            }
            assemblies.iter().any(|assembly| {
                assembly.crate_name == type_.module_path.split("::").next().unwrap_or("")
                    && assembly.xmlns_definitions.iter().any(|mapping| {
                        mapping.xml_namespace == namespace && mapping.namespace == type_.namespace()
                    })
            })
        })
        .collect()
}
fn member_owner<'a>(
    node: roxmltree::Node<'a, 'a>,
    attr: roxmltree::Attribute<'a, 'a>,
) -> (&'a str, &'a str, &'a str) {
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
    }
}
fn expanded_member(node: roxmltree::Node<'_, '_>, attr: roxmltree::Attribute<'_, '_>) -> String {
    let (namespace, owner, member) = member_owner(node, attr);
    format!("{{{namespace}}}{owner}.{member}")
}
fn metadata_event(node: roxmltree::Node<'_, '_>, attr: roxmltree::Attribute<'_, '_>) -> bool {
    // Namespaced directives are not instance event assignments.
    if attr.namespace().is_some() && !attr.name().contains('.') {
        return false;
    }
    let (namespace, owner, member) = member_owner(node, attr);
    types(namespace, owner).into_iter().any(|type_| {
        let mut current = Some(type_);
        let mut visited = std::collections::HashSet::new();
        while let Some(type_) = current {
            if !visited.insert(type_ as *const MarkupType as usize) {
                break;
            }
            if type_.find_event(member).is_some() {
                return true;
            }
            let definition = format!("{member}Event");
            let field_type = type_
                .find_field(&definition)
                .map(|field| (field.type_)())
                .or_else(|| {
                    type_
                        .find_static_property(&definition)
                        .map(|property| (property.type_)())
                });
            if field_type.is_some_and(|type_| {
                type_
                    .name()
                    .split('<')
                    .next()
                    .unwrap_or("")
                    .rsplit("::")
                    .next()
                    == Some("RoutedEvent")
            }) {
                return true;
            }
            current = type_.base_type();
        }
        false
    })
}

/// Routed event tokens have reversible ownership. Metadata events exposing only
/// an add accessor require an explicit subscription adapter.
pub(crate) fn validate_routed_events(source: &str) -> Result<(), ReloadError> {
    let xml = roxmltree::Document::parse(source).map_err(|error| ReloadError::Failed(error.to_string()))?;
    for node in xml.descendants().filter(|node| node.is_element()) {
        for attr in node.attributes().filter(|attr| metadata_event(node, *attr)) {
            let (namespace, owner, member) = member_owner(node, attr);
            let routed = types(namespace, owner).into_iter().any(|type_| {
                let mut current = Some(type_);
                let mut seen = std::collections::HashSet::new();
                while let Some(type_) = current {
                    if !seen.insert(type_ as *const MarkupType as usize) { break; }
                    let definition = format!("{member}Event");
                    let field = type_.find_field(&definition).map(|field| (field.type_)())
                        .or_else(|| type_.find_static_property(&definition).map(|property| (property.type_)()));
                    if field.is_some_and(|field| field.name().split('<').next().unwrap_or("").rsplit("::").next() == Some("RoutedEvent")) { return true; }
                    current = type_.base_type();
                }
                false
            });
            if !routed { return Err(ReloadError::RestartRequired(format!("{owner}.{member} needs a reversible subscription adapter"))); }
        }
    }
    Ok(())
}

#[cfg(test)]
mod metadata_tests {
    use super::*;
    use ferroui_base::metadata::{MarkupEvent, MarkupTypeKind};
    static CUSTOM: MarkupType = MarkupType {
        explicit_namespace: Some("Reload.Contract.Tests"),
        events: &[MarkupEvent {
            name: "Recomputed",
            arguments: &[],
            add: |_| panic!("contract parsing must not invoke event accessors"),
        }],
        ..MarkupType::new("Custom", MarkupTypeKind::Class, "reload_contract_tests")
    };
    #[test]
    fn non_routed_events_require_an_adapter_without_invoking_accessors() {
        MarkupType::register(&CUSTOM);
        let source = r#"<Custom xmlns="using:Reload.Contract.Tests" Recomputed="Handler"/>"#;
        assert!(matches!(validate_routed_events(source), Err(ReloadError::RestartRequired(_))));
        ferroui_controls::register_types();
        assert!(validate_routed_events(r#"<Button xmlns="https://github.com/ferroui" Click="Handler"/>"#).is_ok());
    }
    #[test]
    fn built_handler_contract_is_stable_across_lazy_runtime_registration() {
        static LAZY: MarkupType = MarkupType {
            explicit_namespace: Some("Reload.LazyContract.Tests"),
            events: &[MarkupEvent {
                name: "Recomputed",
                arguments: &[],
                add: |_| panic!("contract parsing must not subscribe"),
            }],
            ..MarkupType::new("Lazy", MarkupTypeKind::Class, "reload_lazy_contract_tests")
        };
        let source = r#"<Lazy xmlns="using:Reload.LazyContract.Tests" Recomputed="Before"/>"#;
        let mut build = BuildContracts::default();
        build
            .insert(&serde_json::json!({
                "namespace": "using:Reload.LazyContract.Tests", "name": "Lazy", "known": true,
                "events": {"Recomputed": ["Action"]}
            }))
            .unwrap();
        assert!(DocumentContract::parse(source).unwrap().events.is_empty());
        let before = DocumentContract::parse_with_build(source, Some(&build)).unwrap();
        assert_eq!(before.events.len(), 1);
        MarkupType::register(&LAZY);
        assert_eq!(DocumentContract::parse(source).unwrap().events.len(), 1);
        assert_eq!(
            before,
            DocumentContract::parse_with_build(source, Some(&build)).unwrap()
        );
        assert_ne!(
            before,
            DocumentContract::parse_with_build(&source.replace("Before", "After"), Some(&build))
                .unwrap()
        );
    }
    #[test]
    fn data_type_namespace_rebinding_changes_contract_but_prefix_renaming_does_not() {
        let source = r#"<View xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:vm="using:First" x:DataType="vm:Model"/>"#;
        for source in [
            source.to_owned(),
            source.replace("DataType", "TypeArguments"),
        ] {
            let baseline = DocumentContract::parse(&source).unwrap();
            assert_ne!(
                baseline,
                DocumentContract::parse(&source.replace("using:First", "using:Second")).unwrap()
            );
            assert_eq!(
                baseline,
                DocumentContract::parse(&source.replace("vm", "model")).unwrap()
            );
        }
    }
    #[test]
    fn custom_event_changes_are_contract_changes_without_invoking_user_code() {
        MarkupType::register(&CUSTOM);
        let source = r#"<t:Custom xmlns:t="using:Reload.Contract.Tests" Recomputed="Before"/>"#;
        let before = DocumentContract::parse(source).unwrap();
        assert_eq!(before.events.len(), 1);
        assert_ne!(
            before,
            DocumentContract::parse(&source.replace("Before", "After")).unwrap()
        );
        assert_eq!(
            before,
            DocumentContract::parse(
                &source
                    .replace("t:", "custom:")
                    .replace("xmlns:t", "xmlns:custom")
            )
            .unwrap()
        );
        assert!(
            DocumentContract::parse(&source.replace("Recomputed", "OrdinaryValue"))
                .unwrap()
                .events
                .is_empty()
        );
    }
    #[test]
    fn detects_inherited_routed_events_and_qualified_event_owners() {
        ferroui_controls::register_types();
        let source = r#"<Button xmlns="https://github.com/ferroui" PointerMoved="OnMove"/>"#;
        assert_eq!(DocumentContract::parse(source).unwrap().events.len(), 1);
        let attached = r#"<Border xmlns="https://github.com/ferroui" Button.Click="OnClick"/>"#;
        assert_eq!(DocumentContract::parse(attached).unwrap().events.len(), 1);
    }
}
