use super::{
    is_native_binding, is_path, named_target, source_type, template_target_type, type_reference,
    type_value,
};
use crate::{references::markup_arguments_with_extensions, Schema, TypeInfo};
use ferroui_designer_core::{
    Attribute, Document, MarkupExtension, MarkupValue, NodeId, SourceSpan, XmlText,
};
use ferroui_markup_syntax::binding_expression_grammar::{
    BindingExpressionGrammar, SourceMode, SpannedNode,
};

pub(super) struct PathSource<'a, 's> {
    pub text: XmlText<'a>,
    pub nodes: Vec<SpannedNode>,
    pub owner: Option<&'s TypeInfo>,
}

impl<'a, 's> PathSource<'a, 's> {
    pub fn from_attribute(
        schema: &'s Schema,
        document: &'a Document,
        node: NodeId,
        attribute: &Attribute,
    ) -> Option<Self> {
        if !attribute.value.starts_with('{') {
            return None;
        }
        let MarkupValue::Extension(binding) = attribute.markup().ok()? else {
            return None;
        };
        if !is_native_binding(schema, document, node, &binding.name) {
            return None;
        }
        let raw = document.source().get(attribute.value_span.range())?;
        let arguments = markup_arguments_with_extensions(raw);
        let mut paths = arguments
            .iter()
            .filter(|argument| argument.depth == 0 && is_path(argument));
        let path = paths.next()?;
        if paths.next().is_some() {
            return None;
        }
        let start = attribute.value_span.start;
        let text = XmlText::attribute(
            document.source(),
            SourceSpan::new(start + path.span.start, start + path.span.end),
        )?;
        // The XML map can identify complete entities. Escaped markup arguments
        // require their own map before they can supply an editable path.
        if text.value() != path.value {
            return None;
        }
        let (nodes, mode) = BindingExpressionGrammar::parse_with_spans(text.value()).ok()?;
        let mut sources = binding.arguments.iter().filter(|argument| {
            matches!(
                argument.name.as_deref(),
                Some("ElementName" | "RelativeSource" | "Source")
            )
        });
        let anchor = sources.next();
        if sources.next().is_some() || mode == SourceMode::Control && anchor.is_some() {
            return None;
        }
        let owner = match anchor
            .and_then(|argument| argument.name.as_deref().map(|name| (name, &argument.value)))
        {
            Some(("ElementName", MarkupValue::Literal(name))) => {
                named_type(schema, document, node, name)
            }
            Some(("RelativeSource", MarkupValue::Extension(relative))) => {
                relative_owner(schema, document, node, relative)
            }
            Some(_) => None,
            None if mode == SourceMode::Data => {
                let arguments = arguments
                    .into_iter()
                    .filter(|argument| argument.depth == 0)
                    .collect::<Vec<_>>();
                let mut path = text.value();
                source_type(schema, document, node, "", &arguments, &mut path, |name| {
                    named_target(document, node, name)
                })
            }
            None => None,
        };
        Some(Self { text, nodes, owner })
    }
}

pub(super) fn named_type<'s>(
    schema: &'s Schema,
    document: &Document,
    node: NodeId,
    name: &str,
) -> Option<&'s TypeInfo> {
    element_type(schema, document, named_target(document, node, name)?)
}

pub(super) fn element_type<'s>(
    schema: &'s Schema,
    document: &Document,
    node: NodeId,
) -> Option<&'s TypeInfo> {
    let element = document.ast().node(node)?;
    schema.type_info(&element.name, element.namespace_uri.as_deref())
}

pub(crate) fn is_relative_source(
    schema: &Schema,
    document: &Document,
    node: NodeId,
    name: &str,
) -> bool {
    let (prefix, _) = name.split_once(':').unwrap_or(("", name));
    let namespaces = document.ast().namespaces(node);
    schema
        .extension(name, namespaces.get(prefix).map(String::as_str))
        .is_some_and(|extension| extension.name == "RelativeSource")
}

pub(super) fn type_parameter_at<'s>(
    schema: &'s Schema,
    document: &Document,
    node: NodeId,
    raw: &str,
    offset: usize,
) -> Option<&'s TypeInfo> {
    for argument in markup_arguments_with_extensions(raw) {
        if !argument.span.contains(offset) {
            continue;
        }
        match argument.name.as_deref() {
            Some("DataType") if is_native_binding(schema, document, node, &argument.extension) => {
                return type_reference(schema, document, node, &argument.value);
            }
            Some("AncestorType")
                if argument.depth == 0
                    && is_relative_source(schema, document, node, &argument.extension) =>
            {
                return type_reference(schema, document, node, &argument.value);
            }
            Some("RelativeSource")
                if is_native_binding(schema, document, node, &argument.extension) =>
            {
                let nested_offset = offset - argument.span.start;
                let ancestor = markup_arguments_with_extensions(raw.get(argument.span.range())?)
                    .into_iter()
                    .find(|candidate| {
                        candidate.depth == 0
                            && candidate.name.as_deref() == Some("AncestorType")
                            && candidate.span.contains(nested_offset)
                            && is_relative_source(schema, document, node, &candidate.extension)
                    });
                if let Some(ancestor) = ancestor {
                    return type_reference(schema, document, node, &ancestor.value);
                }
            }
            _ => {}
        }
    }
    None
}

fn relative_owner<'s>(
    schema: &'s Schema,
    document: &Document,
    node: NodeId,
    relative: &MarkupExtension,
) -> Option<&'s TypeInfo> {
    if !is_relative_source(schema, document, node, &relative.name) {
        return None;
    }
    let mut modes = relative
        .arguments
        .iter()
        .filter(|argument| argument.name.as_deref().is_none_or(|name| name == "Mode"));
    let mode = match modes.next().map(|argument| &argument.value) {
        None => "FindAncestor",
        Some(MarkupValue::Literal(mode)) => mode.as_str(),
        _ => return None,
    };
    if modes.next().is_some() {
        return None;
    }
    match mode {
        "Self" => element_type(schema, document, node),
        "TemplatedParent" => template_target_type(schema, document, node),
        "FindAncestor" => type_value(
            schema,
            document,
            node,
            &relative
                .arguments
                .iter()
                .find(|argument| argument.name.as_deref() == Some("AncestorType"))?
                .value,
        ),
        _ => None,
    }
}
