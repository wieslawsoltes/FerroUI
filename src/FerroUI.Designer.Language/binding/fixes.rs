use super::{is_path, property_type, source_type};
use crate::references::markup_arguments;
use crate::{is_xaml_identifier, Schema, FRAMEWORK_NAMESPACE};
use ferroui_designer_core::{Document, MarkupExtension, MarkupValue, Node, NodeId, SourceSpan};

pub(crate) struct BindingCaseMismatch {
    pub span: SourceSpan,
    pub authored: String,
    pub registered: String,
}

/// Only use a binding's own source parameters. Nested converters and arbitrary
/// source expressions can't establish the type of an outer binding path.
pub(crate) fn case_mismatch(
    schema: &Schema,
    document: &Document,
    node: &Node,
    span: SourceSpan,
    binding: &MarkupExtension,
    named_element: impl FnOnce(&str) -> Option<NodeId>,
) -> Option<BindingCaseMismatch> {
    let (prefix, local) = binding.name.split_once(':').unwrap_or(("", &binding.name));
    let local = local.strip_suffix("Extension").unwrap_or(local);
    if !matches!(local, "Binding" | "CompiledBinding" | "ReflectionBinding") {
        return None;
    }
    let namespaces = document.ast().namespaces(node.id);
    let namespace = namespaces.get(prefix).map(String::as_str);
    if namespace != Some(FRAMEWORK_NAMESPACE) && (namespace.is_some() || !prefix.is_empty()) {
        return None;
    }
    if binding.arguments.iter().any(|arg| {
        matches!(arg.name.as_deref(), Some("Source" | "RelativeSource"))
            || matches!(arg.name.as_deref(), Some("DataType" | "ElementName"))
                && matches!(arg.value, MarkupValue::Extension(_))
    }) || ["DataType", "ElementName"].iter().any(|parameter| {
        binding
            .arguments
            .iter()
            .filter(|arg| arg.name.as_deref() == Some(parameter))
            .nth(1)
            .is_some()
    }) {
        return None;
    }
    let raw = &document.source()[span.range()];
    let args: Vec<_> = markup_arguments(raw)
        .into_iter()
        .filter(|arg| arg.depth == 0)
        .collect();
    let mut paths = args.iter().filter(|arg| is_path(arg));
    let path_arg = paths.next()?;
    let mut path = path_arg.value.as_str();
    if paths.next().is_some() || raw.get(path_arg.span.range())? != path {
        return None;
    }
    // Reflection may use data of any runtime type. An explicit named control
    // or self anchor and a compiled path do have a known source type.
    let named = args
        .iter()
        .any(|arg| arg.name.as_deref() == Some("ElementName"));
    let anchored = path.starts_with('#') || path.starts_with("$self.");
    if (named && anchored) || (local != "CompiledBinding" && !anchored && !named) {
        return None;
    }
    let mut owner = source_type(
        schema,
        document,
        node.id,
        "",
        &args,
        &mut path,
        named_element,
    )?;
    let mut offset = span.start + path_arg.span.end - path.len();
    if !path.split('.').all(is_xaml_identifier) {
        return None;
    }
    for member in path.split('.') {
        if let Some(property) = schema.property(&owner.name, Some(&owner.namespace), member) {
            owner = property_type(
                schema,
                &property.type_name,
                &owner.namespace,
                document,
                node.id,
            )?;
        } else {
            let available = schema.properties(&owner.name, Some(&owner.namespace));
            let mut matching = available.values().filter(|property| {
                !property.event && !property.attached && property.name.eq_ignore_ascii_case(member)
            });
            let known = matching.next()?;
            if matching.next().is_some() {
                return None;
            }
            return Some(BindingCaseMismatch {
                span: SourceSpan::new(offset, offset + member.len()),
                authored: member.into(),
                registered: known.name.clone(),
            });
        }
        offset += member.len() + 1;
    }
    None
}
