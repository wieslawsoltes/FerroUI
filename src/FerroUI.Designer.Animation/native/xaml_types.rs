//! Resolve authored XAML type and setter scopes against the loaded native assemblies.
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;
use ferroui_designer_core::{Document, MarkupValue, NodeId, NodeKind};
use ferroui_designer_tools::component_property_attribute;
use ferroui_markup::markup::parsers::{SelectorGrammar, SelectorParser, SelectorSyntax};

const FRAMEWORK: &str = "https://github.com/ferroui";
const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

/// Resolve the same registered XML namespace mappings used by the runtime loader.
/// A short-name search is insufficient: an application's Button must not make the
/// framework Button ambiguous, and libraries can declare their own XML namespaces.
/// Native classes need not declare their own MarkupType to inherit registered properties.
pub fn registered_xml_type(uri: &str, name: &str) -> Option<&'static TypeInfo> {
    if let Some(namespace) = uri
        .strip_prefix("using:")
        .or_else(|| uri.strip_prefix("clr-namespace:"))
    {
        let mut parts = namespace.split(';');
        let type_ = TypeInfo::find(parts.next()?, name)?;
        for part in parts {
            let assembly = part.strip_prefix("assembly=")?;
            if MarkupAssembly::of_module(type_.module_path())?.name != assembly {
                return None;
            }
        }
        return Some(type_);
    }
    let mut resolved: Option<&'static TypeInfo> = None;
    for definition in MarkupAssembly::registered_assemblies()
        .into_iter()
        .flat_map(|assembly| assembly.xmlns_definitions)
        .filter(|definition| definition.xml_namespace == uri)
    {
        if let Some(candidate) = TypeInfo::find(definition.namespace, name) {
            if resolved.is_some_and(|previous| !std::ptr::eq(previous, candidate)) {
                return None;
            }
            resolved = Some(candidate);
        }
    }
    resolved
}

pub fn registered_type(
    document: &Document,
    node: NodeId,
    token: &str,
) -> Option<&'static TypeInfo> {
    let (prefix, name) = token.split_once(':').unwrap_or(("", token));
    let namespaces = document.ast().namespaces(node);
    let uri = match namespaces.get(prefix) {
        Some(uri) => uri.as_str(),
        None if prefix.is_empty() => FRAMEWORK,
        None => return None,
    };
    registered_xml_type(uri, name)
}

pub fn element_type(document: &Document, node: NodeId) -> Option<&'static TypeInfo> {
    let element = document.ast().node(node)?;
    if let Some(class) = element.attributes.iter().find(|attribute| {
        attribute.namespace_uri.as_deref() == Some(XAML)
            && attribute.name.rsplit(':').next() == Some("Class")
    }) {
        let (namespace, name) = class.value.rsplit_once('.')?;
        return TypeInfo::find(namespace, name);
    }
    registered_type(document, node, &element.name)
}

pub fn setter_target_type(document: &Document, owner: NodeId) -> Option<&'static TypeInfo> {
    let type_ = element_type(document, owner)?;
    if ferroui_base::styling::Style::TYPE.is_assignable_from(type_) {
        return selector_target(document, owner);
    }
    if !ferroui_base::styling::ControlTheme::TYPE.is_assignable_from(type_) {
        return None;
    }
    let value = document.ast().node(owner)?.attribute("TargetType")?;
    let token = match MarkupValue::parse(value).ok()? {
        MarkupValue::Literal(value) => value,
        MarkupValue::Extension(extension) => {
            let (prefix, name) = extension.name.split_once(':')?;
            if name != "Type"
                || document
                    .ast()
                    .namespaces(owner)
                    .get(prefix)
                    .map(String::as_str)
                    != Some(XAML)
            {
                return None;
            }
            extension.arguments.into_iter().find_map(|argument| {
                if argument.name.is_none() || argument.name.as_deref() == Some("TypeName") {
                    if let MarkupValue::Literal(value) = argument.value {
                        return Some(value);
                    }
                }
                None
            })?
        }
    };
    registered_type(document, owner, token.trim())
}

pub fn parent_style(document: &Document, node: NodeId) -> Option<NodeId> {
    let parent = document.ast().node(document.ast().node(node)?.parent?)?;
    let parent = if parent.kind == NodeKind::PropertyElement {
        document.ast().node(parent.parent?)?
    } else {
        parent
    };
    let type_ = registered_type(document, parent.id, &parent.name)?;
    (ferroui_base::styling::Style::TYPE.is_assignable_from(type_)
        || ferroui_base::styling::ControlTheme::TYPE.is_assignable_from(type_))
    .then_some(parent.id)
}

pub fn selector_parser(document: &Document, node: NodeId) -> SelectorParser {
    let namespaces = document.ast().namespaces(node);
    SelectorParser::new(move |prefix, name| {
        let uri = namespaces.get(prefix)?;
        registered_xml_type(uri, name)
    })
    .unknown_types_by_name()
}

fn selector_target(document: &Document, node: NodeId) -> Option<&'static TypeInfo> {
    let selector = component_property_attribute(document, node, "Selector").ok()??;
    let MarkupValue::Literal(selector) = selector.markup().ok()? else {
        return None;
    };
    selector_parser(document, node)
        .parse(&selector)
        .ok()??
        .target_type()
        .or_else(|| {
            let syntax = SelectorGrammar::parse(&selector).ok()?;
            if syntax
                .iter()
                .any(|part| matches!(part, SelectorSyntax::Nesting))
                && !syntax.iter().any(|part| {
                    matches!(
                        part,
                        SelectorSyntax::Child
                            | SelectorSyntax::Descendant
                            | SelectorSyntax::Template
                            | SelectorSyntax::Comma
                    )
                })
            {
                return setter_target_type(document, parent_style(document, node)?);
            }
            None
        })
}
