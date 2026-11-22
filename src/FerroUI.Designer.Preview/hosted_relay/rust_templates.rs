//! Resolve an effective native template against the accepted Rust invocation and modules.
use super::templates::{template_member, Location, Template, Templates};
use crate::{document::DesignDocument, rsx_animation::AuthoredRsxFrame};
use ferroui_designer_core::{NodeId, SourceSpan};
use ferroui_designer_language::LineIndex;
use ferroui_designer_project::{canonical_file_uri, rust_source_offset};
use ferroui_designer_rsx::{RsxSourceLocation, RsxTemplateBody};
use ferroui_designer_tools::table_columns;
use ferroui_reload_protocol::fingerprint;
use ferroui_remote_protocol::designer::{
    DesignTemplateCatalogMessage, DesignTemplateSource, DesignTemplateUse,
};
use std::collections::BTreeMap;

impl Templates {
    pub(super) fn capture_rust(
        document: &DesignDocument<'_>,
        authored: &AuthoredRsxFrame,
        catalog: DesignTemplateCatalogMessage,
    ) -> Option<Self> {
        let rsx = document.rsx.as_ref()?;
        let ast = document.xaml.ast();
        let mut controls = BTreeMap::new();
        for node in ast.nodes_in_source_order() {
            if let Some((_, source)) = document.rsx_node(node.id) {
                controls.insert(source.name.start, node.id);
            }
        }
        let revision = fingerprint(document.source.as_bytes());
        let mut matching = BTreeMap::<(NodeId, &'static str), Option<Template>>::new();
        for usage in &catalog.templates {
            if usage.owner.fingerprint as u64 != revision
                || !same_file(&usage.owner.address, &authored.uri)
            {
                continue;
            }
            let Some(offset) = offset(document.source, &usage.owner) else {
                continue;
            };
            let Some(node) = controls.get(&offset).and_then(|id| ast.node(*id)) else {
                continue;
            };
            if node.local_name() != usage.type_name {
                continue;
            }
            let Some(member) = usage.member.as_deref().and_then(template_member) else {
                continue;
            };
            let label = node.attribute("Name").unwrap_or_else(|| node.local_name());
            let (owner, label) = if let Some(index) = usage.column {
                let Ok(table) = table_columns(&document.xaml, node.id) else {
                    continue;
                };
                let Some(column) = usize::try_from(index)
                    .ok()
                    .and_then(|index| table.columns.get(index))
                else {
                    continue;
                };
                let column = if usage.column_inner == Some(true) {
                    let [inner] = column.children.as_slice() else { continue };
                    inner
                } else {
                    column
                };
                (column.node, format!("{label} / {}", column.label))
            } else {
                (node.id, label.to_owned())
            };
            let candidate = rsx
                .item_templates(&document.xaml, owner)
                .into_iter()
                .find(|slot| slot.property == member)
                .and_then(|slot| {
                    let definition = matched_definition(authored, usage, &slot.body?)?;
                    Some(Template {
                        owner,
                        label,
                        type_name: ast.node(owner)?.local_name().to_owned(),
                        definition,
                        can_copy: false,
                        member: Some(member),
                        assignment: slot
                            .expression
                            .and_then(|location| source_range(authored, location)),
                    })
                });
            matching
                .entry((owner, member.name()))
                .and_modify(|seen| {
                    if *seen != candidate {
                        *seen = None;
                    }
                })
                .or_insert(candidate);
        }
        Some(Self(matching.into_values().flatten().collect()))
    }
}

fn matched_definition(
    authored: &AuthoredRsxFrame,
    usage: &DesignTemplateUse,
    body: &RsxTemplateBody,
) -> Option<Location> {
    let (uri, source) = source(authored, &body.location)?;
    let revision = fingerprint(source.as_bytes());
    let compiled_caller = usage
        .rust_factory
        .as_ref()
        .is_some_and(|site| matches_site(uri, source, revision, site, body.constructor));
    let realized_body = usage
        .definition
        .as_ref()
        .is_some_and(|site| matches_site(uri, source, revision, site, body.macro_path));
    if !compiled_caller && !realized_body {
        return None;
    }
    location(uri, source, body.macro_path)
}

fn matches_site(
    uri: &str,
    source: &str,
    revision: u64,
    stamp: &DesignTemplateSource,
    span: SourceSpan,
) -> bool {
    stamp.fingerprint as u64 == revision
        && same_file(uri, &stamp.address)
        && offset(source, stamp).is_some_and(|offset| span.range().contains(&offset))
}

fn offset(source: &str, stamp: &DesignTemplateSource) -> Option<usize> {
    rust_source_offset(
        source,
        u32::try_from(stamp.line).ok()?,
        u32::try_from(stamp.column).ok()?,
    )
}

fn source<'a>(
    authored: &'a AuthoredRsxFrame,
    location: &'a RsxSourceLocation,
) -> Option<(&'a str, &'a str)> {
    let uri = location.uri.as_deref().unwrap_or(&authored.uri);
    let source = if uri == authored.uri {
        authored.source()
    } else {
        authored.module_source(uri)?
    };
    Some((uri, source))
}

fn source_range(
    authored: &AuthoredRsxFrame,
    source_location: RsxSourceLocation,
) -> Option<Location> {
    let (uri, source) = source(authored, &source_location)?;
    location(uri, source, source_location.span)
}

fn location(uri: &str, source: &str, span: SourceSpan) -> Option<Location> {
    Some(Location {
        address: uri.to_owned(),
        uri: uri.to_owned(),
        node: None,
        range: LineIndex::new(source).range(span).ok()?,
    })
}

fn same_file(left: &str, right: &str) -> bool {
    left == right
        || canonical_file_uri(left)
            .is_ok_and(|left| canonical_file_uri(right).is_ok_and(|right| left == right))
}
