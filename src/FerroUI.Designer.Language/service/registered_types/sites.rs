//! Lex type-bearing XAML syntax while retaining the original local-name span.
use crate::service::style_selectors::{self, References};
use crate::{
    binding,
    references::markup_tokens, require, LanguageError, Result, Schema, TypeInfo,
    FRAMEWORK_NAMESPACE,
};
use ferroui_designer_core::{Attribute, Document, Node, NodeKind, SourceSpan, TextPatch};
use ferroui_designer_project::MarkupSource;
use std::{borrow::Cow, collections::BTreeMap, path::Path};

const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

pub(super) struct TypeSite<'a> {
    pub span: SourceSpan,
    pub spelling: Cow<'a, str>,
    context: Context<'a>,
}

enum Context<'a> {
    Markup(Option<String>),
    Binding(Option<String>),
    Class(&'a str),
    Selector(String),
}

impl TypeSite<'_> {
    pub(super) fn resolve<'s>(&self, schema: &'s Schema, package: &Path) -> Option<&'s TypeInfo> {
        let namespace = match &self.context {
            Context::Markup(namespace) => {
                return schema.type_info(&self.spelling, namespace.as_deref())
            }
            Context::Binding(namespace) => {
                return schema.type_info(&self.spelling, Some(namespace.as_deref()?));
            }
            Context::Selector(namespace) => {
                return schema.exact_type_info(&self.spelling, Some(namespace))
            }
            Context::Class(namespace) => namespace,
        };
        let prefix = format!("clr-namespace:{namespace};assembly=");
        let mut candidates = schema.types.values().filter(|info| {
            info.name == self.spelling
                && info.namespace.starts_with(&prefix)
                && info
                    .authored
                    .as_ref()
                    .is_some_and(|source| source.is_class() && source.package() == package)
        });
        let found = candidates.next()?;
        candidates
            .all(|other| {
                other
                    .authored
                    .as_ref()
                    .zip(found.authored.as_ref())
                    .is_some_and(|(first, second)| first.same_registration(second))
            })
            .then_some(found)
    }
}

pub(super) fn of_node<'a>(
    schema: &Schema,
    document: &'a Document,
    node: &'a Node,
) -> Vec<TypeSite<'a>> {
    let selectors = style_selectors::references(schema, document, node);
    with_selectors(schema, document, node, selectors.as_ref())
}

fn with_selectors<'a>(
    schema: &Schema,
    document: &'a Document,
    node: &'a Node,
    selectors: Option<&References>,
) -> Vec<TypeSite<'a>> {
    let mut sites = Vec::new();
    let namespaces = document.ast().namespaces(node.id);
    let node_type = schema.type_info(&node.name, node.namespace_uri.as_deref());
    let object_name = node
        .name
        .rsplit_once('.')
        .map_or(node.name.as_str(), |(owner, _)| owner);
    xml_name(&mut sites, object_name, node.name_span.start, &namespaces);
    if let Some(span) = node.closing_span {
        xml_name(&mut sites, object_name, span.start + 2, &namespaces);
    }
    for attribute in &node.attributes {
        if attribute.name != "xmlns" && !attribute.name.starts_with("xmlns:") {
            if let Some((owner, _)) = attribute.name.rsplit_once('.') {
                xml_name(&mut sites, owner, attribute.name_span.start, &namespaces);
            }
        }
        let context = AttributeContext {
            document,
            node,
            node_type,
            schema,
            namespaces: &namespaces,
        };
        context.type_values(attribute, &mut sites);
    }
    if node.namespace_uri.as_deref() == Some(XAML) && local(&node.name) == "Type" {
        for child in node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
        {
            if child.kind == NodeKind::Text {
                if let Some(raw) = document.source().get(child.span.range()) {
                    xml_name(&mut sites, raw, child.span.start, &namespaces);
                }
            }
        }
    }
    if let Some(selectors) = selectors {
        sites.extend(selectors.types.iter().filter_map(|site| {
            Some(TypeSite {
                span: site.span,
                spelling: Cow::Owned(site.spelling.clone()),
                context: Context::Selector(site.namespace.clone()?),
            })
        }));
    }
    sites
}

struct AttributeContext<'a, 's> {
    document: &'a Document,
    node: &'a Node,
    node_type: Option<&'s TypeInfo>,
    schema: &'s Schema,
    namespaces: &'s BTreeMap<String, String>,
}

impl<'a> AttributeContext<'a, '_> {
    fn type_values(&self, attribute: &Attribute, sites: &mut Vec<TypeSite<'a>>) {
        let Some(raw) = self.document.source().get(attribute.value_span.range()) else {
            return;
        };
        let start = attribute.value_span.start;
        let raw_offset = raw.len() - raw.trim_start().len();
        let raw = raw.trim();
        let start = start + raw_offset;
        if raw.starts_with('{') {
            if attribute.markup().is_ok() {
                self.extensions(raw, start, sites);
                sites.extend(binding::path_type_references(self.schema, self.document, self.node.id, attribute)
                    .into_iter().map(|operand| TypeSite {
                        span: operand.span,
                        spelling: Cow::Owned(operand.spelling),
                        context: Context::Binding(operand.namespace),
                    }));
            }
            return;
        }
        let name = local(&attribute.name);
        if attribute.namespace_uri.as_deref() == Some(XAML) {
            match name {
                "Class" if self.node.id == self.document.ast().root() => {
                    let (namespace, spelling) = raw.rsplit_once('.').unwrap_or(("", raw));
                    if crate::is_xaml_identifier(spelling) {
                        sites.push(TypeSite {
                            spelling: Cow::Borrowed(spelling),
                            span: trim_span(raw, start, spelling),
                            context: Context::Class(namespace),
                        });
                    }
                }
                "DataType" => xml_name(sites, raw, start, self.namespaces),
                "TypeArguments" => {
                    // A type argument can itself contain a nested generic type list.
                    for part in raw.split_inclusive([',', '(', ')']) {
                        let name = part.trim_end_matches([',', '(', ')']);
                        let offset = part.as_ptr() as usize - raw.as_ptr() as usize;
                        xml_name(sites, name, start + offset, self.namespaces);
                    }
                }
                _ => {}
            }
            return;
        }
        let kind = self
            .node_type
            .filter(|info| info.namespace == FRAMEWORK_NAMESPACE)
            .map(|info| info.name.as_str());
        let own_type_operand = matches!(
            (kind, name),
            (
                Some(
                    "DataTemplate"
                        | "TreeDataTemplate"
                        | "Binding"
                        | "CompiledBinding"
                        | "ReflectionBinding"
                ),
                "DataType"
            ) | (
                Some("ControlTemplate" | "ControlTheme" | "Style" | "Template"),
                "TargetType"
            )
        ) || self.node.namespace_uri.as_deref() == Some(XAML)
            && local(&self.node.name) == "Type"
            && name == "TypeName";
        if own_type_operand {
            xml_name(sites, raw, start, self.namespaces);
        } else if matches!((kind, name), (Some("Setter" | "Transition"), "Property"))
            || name == "Storyboard.TargetProperty"
            || self.node.namespace_uri.as_deref() == Some(XAML)
                && local(&self.node.name) == "Static"
                && name == "Member"
        {
            self.qualified_owner(raw, start, sites);
        }
    }

    fn extensions(&self, source: &'a str, start: usize, sites: &mut Vec<TypeSite<'a>>) {
        let tokens = markup_tokens(source);
        for (decoded, span) in tokens.names {
            if let Some(raw) = source.get(span.range()).filter(|raw| *raw == decoded) {
                xml_name(sites, raw, start + span.start, self.namespaces);
            }
        }
        for argument in tokens.arguments {
            let Some(raw) = source
                .get(argument.span.range())
                .filter(|raw| *raw == argument.value)
            else {
                continue;
            };
            let position = start + argument.span.start;
            let (prefix, form) = argument
                .extension
                .split_once(':')
                .unwrap_or(("", &argument.extension));
            let namespace = self.namespaces.get(prefix).map(String::as_str);
            let parameter = argument.name.as_deref();
            if namespace == Some(XAML) {
                if form == "Type" && parameter.is_none_or(|name| name == "TypeName") {
                    xml_name(sites, raw, position, self.namespaces);
                } else if form == "Static" && parameter.is_none_or(|name| name == "Member") {
                    self.qualified_owner(raw, position, sites);
                }
            } else if match parameter {
                Some("AncestorType") => binding::is_relative_source(
                    self.schema, self.document, self.node.id, &argument.extension,
                ),
                Some("DataType") => binding::is_native_binding(
                    self.schema, self.document, self.node.id, &argument.extension,
                ),
                _ => false,
            } {
                xml_name(sites, raw, position, self.namespaces);
            }
        }
    }

    fn qualified_owner(&self, raw: &'a str, start: usize, sites: &mut Vec<TypeSite<'a>>) {
        let (text, start) = match raw
            .strip_prefix('(')
            .and_then(|text| text.strip_suffix(')'))
        {
            Some(inner) => (inner, start + 1),
            None => (raw, start),
        };
        if let Some((owner, property)) = text.rsplit_once('.') {
            if crate::is_xaml_identifier(property) {
                xml_name(sites, owner, start, self.namespaces);
            }
        }
    }
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn xml_name<'a>(
    sites: &mut Vec<TypeSite<'a>>,
    text: &'a str,
    offset: usize,
    namespaces: &BTreeMap<String, String>,
) {
    let (prefix, spelling) = text.trim().split_once(':').unwrap_or(("", text.trim()));
    if !crate::is_xaml_identifier(spelling)
        || (!prefix.is_empty() && !namespaces.contains_key(prefix))
    {
        return;
    }
    sites.push(TypeSite {
        span: trim_span(text, offset, spelling),
        spelling: Cow::Borrowed(spelling),
        context: Context::Markup(namespaces.get(prefix).cloned()),
    });
}

fn trim_span(text: &str, start: usize, local: &str) -> SourceSpan {
    let offset = start + (local.as_ptr() as usize - text.as_ptr() as usize);
    SourceSpan::new(offset, offset + local.len())
}

pub(super) fn has_reference(
    schema: &Schema,
    document: &Document,
    package: &Path,
    target: &MarkupSource,
) -> bool {
    document.ast().elements().any(|node| {
        of_node(schema, document, node)
            .into_iter()
            .any(|site| resolves(&site, schema, package, target))
    })
}

pub(super) fn review(
    before: &Schema,
    after: &Schema,
    document: &Document,
    package: &Path,
    target: &MarkupSource,
    old: &str,
    new: &str,
) -> Result<(Vec<TextPatch>, Vec<SourceSpan>)> {
    let mut patches = Vec::new();
    let mut unmapped = Vec::new();
    let short = super::schema::shortened(old);
    for node in document.ast().elements() {
        let selector = style_selectors::references(before, document, node);
        let sites = with_selectors(before, document, node, selector.as_ref());
        for site in &sites {
            if resolves(site, before, package, target) {
                let spelling = if Some(site.spelling.as_ref()) == short {
                    super::schema::shortened(new).unwrap_or(new)
                } else {
                    new
                };
                if spelling != site.spelling {
                    patches.push(super::replacement(site.span, spelling));
                }
            } else {
                if matches!(site.context, Context::Binding(_))
                    && (site.spelling == old || Some(site.spelling.as_ref()) == short)
                    && site.resolve(before, package).is_none()
                {
                    unmapped.push(site.span);
                }
                require(
                    !resolves(site, after, package, target),
                    "The new type name would capture an existing XAML reference",
                )?;
            }
        }
        for attribute in &node.attributes {
            if selector
                .as_ref()
                .is_some_and(|selector| selector.span == attribute.value_span)
            {
                continue;
            }
            if matches!(
                local(&attribute.name),
                "Selector" | "DataType" | "TargetType" | "Member" | "Class" | "TypeArguments"
            ) && type_mentioned(&attribute.value, old, short)
                && !sites.iter().any(|site| {
                    attribute.value_span.contains(site.span.start)
                        && resolves(site, before, package, target)
                })
            {
                unmapped.push(attribute.value_span);
            }
        }
        if let Some(selector) = selector {
            if selector.unparsed
                && document
                    .valid_source()
                    .get(selector.span.range())
                    .is_some_and(|value| type_mentioned(value, old, short))
            {
                unmapped.push(selector.span);
            }
            for operand in selector.types {
                if (operand.spelling == old || Some(operand.spelling.as_str()) == short)
                    && operand
                        .namespace
                        .as_deref()
                        .and_then(|ns| before.exact_type_info(&operand.spelling, Some(ns)))
                        .is_none()
                {
                    unmapped.push(operand.span);
                }
            }
        }
    }
    patches.sort_by_key(|edit| edit.span.start);
    patches.dedup();
    Ok((patches, unmapped))
}

fn type_mentioned(value: &str, name: &str, short: Option<&str>) -> bool {
    value
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
        .any(|part| part == name || Some(part) == short)
}

pub(super) fn validate(
    schema: &Schema,
    document: &Document,
    package: &Path,
    target: &MarkupSource,
    patches: &[TextPatch],
) -> Result<()> {
    let mut shift = 0isize;
    for patch in patches {
        let offset = patch.span.start.checked_add_signed(shift).ok_or_else(|| {
            LanguageError("A type reference moved outside its reviewed source".into())
        })?;
        let matched = document
            .ast()
            .element_at(offset)
            .and_then(|id| document.ast().node(id))
            .is_some_and(|node| {
                of_node(schema, document, node).into_iter().any(|site| {
                    site.span.start == offset && resolves(&site, schema, package, target)
                })
            });
        require(
            matched,
            "A renamed XAML reference would resolve to a different type",
        )?;
        shift += patch.replacement.len() as isize - patch.span.len() as isize;
    }
    Ok(())
}

fn resolves(site: &TypeSite<'_>, schema: &Schema, package: &Path, target: &MarkupSource) -> bool {
    site.resolve(schema, package)
        .and_then(|info| info.authored.as_deref())
        .is_some_and(|source| source.same_registration(target))
}
