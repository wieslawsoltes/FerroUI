use crate::service::setter_properties::setter_property;
use crate::service::style_selectors::{self, References};
use crate::{binding, require, LanguageError, PropertyInfo, Result, Schema};
use ferroui_designer_core::{Document, Node, SourceSpan, TextPatch};

use ferroui_designer_project::MarkupSource;

pub(super) struct Site {
    pub span: SourceSpan,
    pub property: PropertyInfo,
}

pub(super) fn of_node(schema: &Schema, document: &Document, node: &Node) -> Vec<Site> {
    let selector = style_selectors::references(schema, document, node);
    with_selector(schema, document, node, selector.as_ref())
}

fn with_selector(
    schema: &Schema,
    document: &Document,
    node: &Node,
    selector: Option<&References>,
) -> Vec<Site> {
    let mut sites = Vec::new();
    if node.name.contains('.') {
        if let Some(property) = node
            .parent
            .and_then(|parent| document.ast().node(parent))
            .and_then(|owner| schema.element_property(owner, node))
        {
            push(&mut sites, &node.name, node.name_span, &property);
            if let Some(close) = node.closing_span {
                let start = close.start + 2;
                push(
                    &mut sites,
                    &node.name,
                    SourceSpan::new(start, start + node.name.len()),
                    &property,
                );
            }
        }
    }
    for attribute in &node.attributes {
        if let Some(property) = schema.attribute_property(document, node, attribute) {
            push(&mut sites, &attribute.name, attribute.name_span, &property);
        }
        if let Some(property) = setter_property(schema, document, node, attribute) {
            push(
                &mut sites,
                &attribute.value,
                attribute.value_span,
                &property,
            );
        }
        if attribute.value.starts_with('{') {
            sites.extend(
                binding::property_references(schema, document, node.id, attribute)
                    .into_iter()
                    .map(|(span, property)| Site { span, property }),
            );
        }
    }
    if let Some(selector) = selector {
        for operand in &selector.properties {
            if let Some(property) = operand
                .member
                .as_ref()
                .filter(|member| member.authored.is_some())
            {
                sites.push(Site {
                    span: operand.span,
                    property: property.clone(),
                });
            }
        }
    }
    sites
}

fn push(sites: &mut Vec<Site>, text: &str, span: SourceSpan, property: &PropertyInfo) {
    if property.authored.is_some()
        && !property.event
        && text.rsplit([':', '.']).next() == Some(&property.name)
    {
        let start = span.start + text.len() - property.name.len();
        sites.push(Site {
            span: SourceSpan::new(start, span.end),
            property: property.clone(),
        });
    }
}

/// Property strings with dynamic targets or syntax outside the source mapping.
/// Keep these visible in the review when they use the same spelling.
fn unmapped_operands(
    document: &Document,
    node: &Node,
    name: &str,
    mapped: &[Site],
    selector: Option<&References>,
) -> Vec<SourceSpan> {
    node.attributes
        .iter()
        .filter_map(|attribute| {
            if selector.is_some_and(|selector| selector.span == attribute.value_span) {
                return None;
            }
            let local = attribute.name.rsplit([':', '.']).next()?;
            let property_operand =
                matches!(local, "Property" | "TargetProperty" | "Path" | "Selector")
                    || attribute.value.starts_with('{') && attribute.value.contains("Binding");
            if !property_operand || !mentions(&attribute.value, name) {
                return None;
            }
            let covered = mapped
                .iter()
                .filter(|site| {
                    site.property.name == name
                        && attribute.value_span.contains(site.span.start)
                        && site.span.end <= attribute.value_span.end
                })
                .count();
            let occurrences = words(&attribute.value).filter(|word| *word == name).count();
            let raw = document.source().get(attribute.value_span.range())?;
            (covered < occurrences && !raw.is_empty()).then_some(attribute.value_span)
        })
        .collect()
}

pub(super) fn mentions(source: &str, name: &str) -> bool {
    words(source).any(|word| word == name)
}

fn words(source: &str) -> impl Iterator<Item = &str> {
    source.split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
}

pub(super) fn review_uses(
    document: &Document,
    before_schema: &Schema,
    after_schema: &Schema,
    registration: &MarkupSource,
    old: &str,
    new: &str,
) -> Result<(Vec<TextPatch>, Vec<SourceSpan>)> {
    let mut patches = Vec::new();
    let mut unmapped = Vec::new();
    for node in document.ast().elements() {
        let selector = style_selectors::references(before_schema, document, node);
        let before = with_selector(before_schema, document, node, selector.as_ref());
        let after = of_node(after_schema, document, node);
        require(
            !after.iter().any(|site| {
                site.property.name == new && site.property.authored.as_deref() == Some(registration)
            }),
            "The new name would capture an existing XAML property or binding member",
        )?;
        patches.extend(
            before
                .iter()
                .filter(|site| {
                    site.property.name == old
                        && site.property.authored.as_deref() == Some(registration)
                })
                .map(|site| TextPatch {
                    span: site.span,
                    replacement: new.into(),
                }),
        );
        unmapped.extend(unmapped_operands(
            document,
            node,
            old,
            &before,
            selector.as_ref(),
        ));
        if let Some(selector) = selector {
            if selector.unparsed
                && document
                    .valid_source()
                    .get(selector.span.range())
                    .is_some_and(|value| mentions(value, old))
            {
                unmapped.push(selector.span);
            }
            unmapped.extend(
                selector
                    .properties
                    .into_iter()
                    .filter(|operand| operand.spelling == old && operand.member.is_none())
                    .map(|operand| operand.span),
            );
        }
    }
    Ok((patches, unmapped))
}

pub(super) fn has_mapped_reference(
    schema: &Schema,
    document: &Document,
    old: &str,
    registration: &MarkupSource,
) -> bool {
    (mentions(document.source(), old) || document.source().contains('&'))
        && document.ast().elements().any(|node| {
            of_node(schema, document, node)
                .iter()
                .any(|site| site.property.authored.as_deref() == Some(registration))
        })
}

pub(super) fn validate_references(
    schema: &Schema,
    document: &Document,
    patches: &[TextPatch],
    registration: &MarkupSource,
    new: &str,
) -> Result<()> {
    let mut shift = 0isize;
    for patch in patches {
        let offset = patch.span.start.checked_add_signed(shift).ok_or_else(|| {
            LanguageError("A property reference moved outside the reviewed source".into())
        })?;
        let resolved = document
            .ast()
            .element_at(offset)
            .and_then(|id| document.ast().node(id))
            .is_some_and(|node| {
                of_node(schema, document, node).into_iter().any(|site| {
                    site.span.start == offset
                        && site.property.name == new
                        && site.property.authored.as_deref() == Some(registration)
                })
            });
        require(
            resolved,
            "A renamed XAML reference changes which property it resolves to",
        )?;
        shift += new.len() as isize - patch.span.len() as isize;
    }
    Ok(())
}
