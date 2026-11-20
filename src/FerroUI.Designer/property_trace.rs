//! Sources in the authored buffer for the live value-store diagnostic.
use crate::{DesignerSession, PreviewStatus};
use ferroui_base::diagnostics::ValueContributorKind;
use ferroui_designer_core::{
    Document, MarkupExtension, MarkupValue, Node, NodeId, NodeKind, SourceSpan,
};
#[cfg(feature = "mcp")]
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::RsxSourceMap;
use ferroui_designer_tools::{component_property_attribute, component_property_elements};
use ferroui_inspect::{InspectedPropertyTrace, Inspector, PropertyContributor};
#[cfg(feature = "mcp")]
use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub(crate) enum TraceTarget {
    Authored(NodeId),
    Runtime(u64),
}

pub(crate) struct PropertyTrace {
    pub native: InspectedPropertyTrace,
    pub authored: Option<TraceLocation>,
    pub sources: Vec<Option<TraceLocation>>,
    pub resource: Option<TraceLocation>,
}

pub(crate) struct TraceLocation {
    node: NodeId,
    span: SourceSpan,
    property: Option<String>,
    exact_value: bool,
}

impl TraceLocation {
    fn element(node: &Node) -> Self {
        Self {
            node: node.id,
            span: node.name_span,
            property: None,
            exact_value: false,
        }
    }

    fn value(node: NodeId, property: &str, span: SourceSpan, exact_value: bool) -> Self {
        Self {
            node,
            span,
            property: Some(property.into()),
            exact_value,
        }
    }

    pub(crate) fn span(
        &self,
        document: &Document,
        map: Option<&RsxSourceMap>,
    ) -> Option<SourceSpan> {
        let Some(map) = map else {
            return Some(self.span);
        };
        let node = document.ast().node(self.node)?;
        if let Some(property) = &self.property {
            return map.property(node.span.start, property);
        }
        map.control(node.span.start).map(|(_, span)| span)
    }

    pub(crate) fn text<'a>(
        &self,
        document: &Document,
        source: &'a str,
        map: Option<&RsxSourceMap>,
    ) -> Option<&'a str> {
        self.exact_value
            .then(|| source.get(self.span(document, map)?.range()))
            .flatten()
    }

    #[cfg(feature = "mcp")]
    fn json(
        &self,
        document: &Document,
        source: &str,
        map: Option<&RsxSourceMap>,
        index: &LineIndex,
    ) -> Option<Value> {
        let range = index.range(self.span(document, map)?).ok()?;
        Some(json!({"node":self.node, "range":range, "text":self.text(document, source, map)}))
    }
}

#[cfg(feature = "mcp")]
impl PropertyTrace {
    pub(crate) fn json(
        &self,
        document: &Document,
        source: &str,
        map: Option<&RsxSourceMap>,
    ) -> Value {
        let index = LineIndex::new(source);
        let locate = |location: &TraceLocation| location.json(document, source, map, &index);
        let mut result = self
            .native
            .json(|index, _| self.sources[index].as_ref().and_then(locate));
        result["authored"] = json!(self.authored.as_ref().and_then(locate));
        result["resource"] = json!(self.resource.as_ref().and_then(locate));
        result
    }
}

impl DesignerSession {
    pub(crate) fn property_trace(
        &self,
        target: TraceTarget,
        property: &str,
    ) -> Result<PropertyTrace, String> {
        if !self.authored_is_valid()
            || !matches!(self.preview_status(), PreviewStatus::Ready { revision } if *revision == self.document().revision())
        {
            return Err("Wait for the current native preview before tracing a value".into());
        }
        let (object, authored) = match target {
            TraceTarget::Authored(node) => (
                self.preview
                    .visual(node)
                    .ok_or("The selected source element has no current native object")?
                    .upcast(),
                Some(node),
            ),
            TraceTarget::Runtime(id) => (self.inspected_preview_object(id)?, None),
        };
        let native = Inspector::property_trace(&object, property)?;
        let document = self.document();
        let authored_value = authored.and_then(|node| property_location(document, node, property));
        let resource = authored.and_then(|node| resource_location(self, node, property));
        let sources = native
            .contributors
            .iter()
            .map(|entry| self.contributor_location(entry, property))
            .collect();
        Ok(PropertyTrace {
            native,
            authored: authored_value,
            sources,
            resource,
        })
    }

    fn contributor_location(
        &self,
        contributor: &PropertyContributor,
        property: &str,
    ) -> Option<TraceLocation> {
        let object = contributor.source.as_ref()?;
        let document = self.document();
        let node = self.preview.authored_object(document, object)?;
        let node = document.ast().node(node)?;
        if matches!(
            contributor.kind,
            ValueContributorKind::Style | ValueContributorKind::Theme
        ) {
            return setter_location(document, node, property)
                .or_else(|| Some(TraceLocation::element(node)));
        }
        property_location(document, node.id, property)
            .or_else(|| Some(TraceLocation::element(node)))
    }
}

fn property_location(document: &Document, node: NodeId, property: &str) -> Option<TraceLocation> {
    if let Some(attribute) = component_property_attribute(document, node, property)
        .ok()
        .flatten()
    {
        return Some(TraceLocation::value(
            node,
            property,
            attribute.value_span,
            true,
        ));
    }
    let members = component_property_elements(document, node, property).ok()?;
    let [member] = members.as_slice() else {
        return None;
    };
    let declaration = document.ast().node(*member)?;
    Some(TraceLocation::value(
        node,
        property,
        declaration.name_span,
        false,
    ))
}

fn setter_location(document: &Document, owner: &Node, property: &str) -> Option<TraceLocation> {
    let children = document
        .ast()
        .element_children(owner.id)
        .into_iter()
        .filter_map(|id| document.ast().node(id));
    let mut candidates = children
        .flat_map(|child| {
            if child.kind == NodeKind::PropertyElement && child.local_name().ends_with(".Setters") {
                document.ast().element_children(child.id)
            } else {
                vec![child.id]
            }
        })
        .filter_map(|id| document.ast().node(id))
        .filter(|child| {
            child.local_name() == "Setter"
                && child.attribute("Property").is_some_and(|name| {
                    let name = name.rsplit(':').next().unwrap_or(name);
                    name == property
                        || (!property.contains('.') && name.rsplit('.').next() == Some(property))
                })
        });
    let setter = candidates.next()?;
    if candidates.next().is_some() {
        return None;
    }
    property_location(document, setter.id, "Value").or_else(|| Some(TraceLocation::element(setter)))
}

fn resource_location(
    session: &DesignerSession,
    node: NodeId,
    property: &str,
) -> Option<TraceLocation> {
    let document = session.document();
    let attribute = component_property_attribute(document, node, property)
        .ok()
        .flatten()?;
    let MarkupValue::Extension(extension) = attribute.markup().ok()? else {
        return None;
    };
    let target = resolve_property_resource(session, node, &extension).ok()?;
    document.ast().node(target).map(TraceLocation::element)
}

pub(crate) fn resolve_property_resource(
    session: &DesignerSession,
    node: NodeId,
    extension: &MarkupExtension,
) -> Result<NodeId, String> {
    let document = session.document();
    if !matches!(
        crate::property_inspector_view::framework_extension(document, node, &extension.name),
        Some("StaticResource" | "DynamicResource")
    ) {
        return Err("This property does not reference a framework resource".into());
    }
    let mut keys = extension.arguments.iter().filter(|argument| {
        argument
            .name
            .as_deref()
            .is_none_or(|name| name == "ResourceKey")
    });
    let key = keys
        .next()
        .ok_or("The resource expression must have one string key")?;
    if keys.next().is_some() {
        return Err("The resource expression must have one string key".into());
    }
    let MarkupValue::Literal(key) = &key.value else {
        return Err("An application resource key must be inspected in XAML".into());
    };
    let context = session.component_resource_context(node);
    ferroui_designer_tools::component_resource_resolution(document, node, key, context.as_ref())
        .map_err(|error| error.to_string())?
        .resolved()
        .ok_or_else(|| {
            "The resource is external, missing or ambiguous in this preview context".into()
        })
}
