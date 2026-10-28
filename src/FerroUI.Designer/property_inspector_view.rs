//! Property-inspector presentation options, independent of document authoring.
use ferroui_designer_core::{Document, MarkupValue, NodeId};

/// How property rows are arranged. This changes only the inspector view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PropertyInspectorSort {
    #[default]
    Category,
    Alphabetical,
    ValueSource,
}

/// A source filter applies in addition to the property search text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PropertyInspectorFilter {
    #[default]
    All,
    Authored,
    Bindings,
    Resources,
    Objects,
    Expressions,
    Mixed,
    Inherited,
}

/// Local, reusable inspector preferences. No source or undo entries are created.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PropertyInspectorView {
    pub sort: PropertyInspectorSort,
    pub filter: PropertyInspectorFilter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectorSource {
    Mixed,
    Local,
    Binding,
    Resource,
    Object,
    Expression,
    Inherited,
    Runtime,
}

impl InspectorSource {
    pub(crate) fn group(self) -> (usize, &'static str) {
        match self {
            Self::Mixed => (0, "Multiple values"),
            Self::Local => (1, "Local values"),
            Self::Binding => (2, "Bindings"),
            Self::Resource => (3, "Resources"),
            Self::Object => (4, "Object values"),
            Self::Expression => (5, "Expressions"),
            Self::Inherited => (6, "Inherited"),
            Self::Runtime => (7, "Runtime values"),
        }
    }

    pub(crate) fn matches(self, filter: PropertyInspectorFilter, authored: bool) -> bool {
        match filter {
            PropertyInspectorFilter::All => true,
            PropertyInspectorFilter::Authored => authored,
            PropertyInspectorFilter::Bindings => self == Self::Binding,
            PropertyInspectorFilter::Resources => self == Self::Resource,
            PropertyInspectorFilter::Objects => self == Self::Object,
            PropertyInspectorFilter::Expressions => {
                matches!(self, Self::Binding | Self::Resource | Self::Expression)
            }
            PropertyInspectorFilter::Mixed => self == Self::Mixed,
            PropertyInspectorFilter::Inherited => self == Self::Inherited,
        }
    }
}

/// Resolve extension types in their lexical namespace. A custom `Binding` or
/// `StaticResource` is not a framework binding or resource solely by its local name.
pub(crate) fn framework_extension(
    document: &Document,
    node: NodeId,
    name: &str,
) -> Option<&'static str> {
    static SCHEMA: std::sync::OnceLock<ferroui_designer_language::Schema> =
        std::sync::OnceLock::new();
    let schema = SCHEMA.get_or_init(ferroui_designer_language::Schema::ferroui);
    let (prefix, local) = name.split_once(':').unwrap_or(("", name));
    let namespaces = document.ast().namespaces(node);
    let namespace = namespaces.get(prefix).map(String::as_str);
    if !prefix.is_empty() && namespace.is_none() {
        return None;
    }
    // Binding is a loader directive choosing compiled/reflection binding, not a
    // generated runtime type. The remaining extensions use reflected type aliases.
    if matches!(local, "Binding" | "BindingExtension")
        && namespace.is_none_or(|uri| uri == "https://github.com/ferroui")
    {
        return Some("Binding");
    }
    schema
        .type_info(local, namespace)
        .filter(|info| info.namespace == "https://github.com/ferroui")
        .map(|info| info.name.strip_suffix("Extension").unwrap_or(&info.name))
}

pub(crate) fn property_source(
    document: &Document,
    node: NodeId,
    value: Option<&str>,
    object: bool,
    mixed: bool,
) -> InspectorSource {
    if mixed {
        return InspectorSource::Mixed;
    }
    if object {
        return InspectorSource::Object;
    }
    let Some(value) = value else {
        return InspectorSource::Inherited;
    };
    match MarkupValue::parse(value) {
        Ok(MarkupValue::Extension(extension)) => {
            if let Some(name) = framework_extension(document, node, &extension.name) {
                match name {
                    "Binding" | "CompiledBinding" | "ReflectionBinding" | "TemplateBinding" => {
                        return InspectorSource::Binding;
                    }
                    "StaticResource" | "DynamicResource" => return InspectorSource::Resource,
                    _ => {}
                }
            }
            InspectorSource::Expression
        }
        Err(_) if value.starts_with('{') && !value.starts_with("{}") => InspectorSource::Expression,
        _ => InspectorSource::Local,
    }
}

pub(crate) fn matches_search(
    name: &str,
    category: &str,
    query: &str,
    source: InspectorSource,
    authored: bool,
) -> bool {
    let haystack = format!("{} {}", name.to_lowercase(), category.to_lowercase());
    query.split_whitespace().all(|term| match term {
        "@modified" | "@local" => authored,
        "@binding" => source == InspectorSource::Binding,
        "@resource" => source == InspectorSource::Resource,
        "@object" => source == InspectorSource::Object,
        "@mixed" => source == InspectorSource::Mixed,
        "@default" | "@inherited" => source == InspectorSource::Inherited,
        "@expression" => source.matches(PropertyInspectorFilter::Expressions, authored),
        _ => haystack.contains(term),
    })
}
