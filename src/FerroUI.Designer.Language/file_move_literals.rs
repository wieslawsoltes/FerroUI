use super::{escape_xml, native_resource_type, Schema};
use crate::{schema::PropertyInfo, Result};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeKind, SourceSpan, TextPatch};
use ferroui_designer_project::{relative_file_reference, resolve_uri};

struct Part {
    span: SourceSpan,
    cdata: bool,
}

pub(super) struct LiteralPath {
    pub value: String,
    parts: Vec<Part>,
}

impl LiteralPath {
    pub fn patches(self, replacement: &str) -> Vec<TextPatch> {
        self.parts
            .into_iter()
            .enumerate()
            .map(|(index, part)| TextPatch {
                span: part.span,
                replacement: if index > 0 {
                    String::new()
                } else if part.cdata {
                    replacement.to_owned()
                } else {
                    escape_xml(replacement)
                },
            })
            .collect()
    }
}

/// Preserve typed document-relative asset and include literals when transplanting
/// a namespace-complete XAML fragment. No filesystem reads or markup execution.
pub fn relocate_xaml_fragment(
    document: &Document,
    schema: &Schema,
    from: &str,
    to: &str,
) -> Result<String> {
    if from == to {
        return Ok(document.source().into());
    }
    let mut patches = Vec::new();
    for site in paths(document, schema) {
        let split = site.value.find(['?', '#']).unwrap_or(site.value.len());
        let (path, suffix) = site.value.split_at(split);
        if path
            .find(':')
            .is_some_and(|colon| !path[..colon].contains('/'))
        {
            continue;
        }
        let resolved = resolve_uri(from, path)?;
        if resolve_uri(to, path).is_ok_and(|candidate| candidate == resolved) {
            continue;
        }
        let target = if to.starts_with("file:") && resolved.starts_with("file:") {
            relative_file_reference(to, &resolved)?
        } else {
            resolved
        };
        let replacement = format!("{target}{suffix}");
        patches.extend(site.patches(&replacement));
    }
    Ok(TextPatch::apply_all(document.source(), patches)?)
}

pub(super) fn paths(document: &Document, schema: &Schema) -> Vec<LiteralPath> {
    let mut paths = Vec::new();
    for node in document.ast().elements() {
        for attribute in &node.attributes {
            let member = attribute.name.rsplit(':').next().unwrap_or_default();
            let info = schema.attribute_property(document, node, attribute);
            if !is_path(node, member, info.as_ref()) {
                continue;
            }
            if let Some(value) = literal(&attribute.value) {
                let span = trimmed_span(document, attribute.value_span);
                paths.push(LiteralPath {
                    value,
                    parts: vec![Part { span, cdata: false }],
                });
            }
        }
        let path_content = if node.kind == NodeKind::PropertyElement {
            node.parent
                .and_then(|parent| document.ast().node(parent))
                .is_some_and(|owner| {
                    is_path(
                        owner,
                        node.local_name(),
                        schema.element_property(owner, node).as_ref(),
                    )
                })
        } else {
            matches!(node.local_name(), "Bitmap" | "WindowIcon" | "Uri")
                && native_resource_type(node.local_name(), node.namespace_uri.as_deref())
        };
        if let Some(path) = path_content.then(|| text_path(document, node)).flatten() {
            paths.push(path);
        }
    }
    paths
}

fn is_path(owner: &Node, member: &str, property: Option<&PropertyInfo>) -> bool {
    let name = member.rsplit('.').next().unwrap_or(member);
    let native_source = name == "Source"
        && matches!(
            owner.local_name(),
            "ResourceInclude" | "MergeResourceInclude" | "StyleInclude"
        )
        && native_resource_type(owner.local_name(), owner.namespace_uri.as_deref());
    native_source
        || property.is_some_and(|property| {
            let kind = property
                .type_name
                .strip_prefix("Option<")
                .and_then(|kind| kind.strip_suffix('>'))
                .unwrap_or(&property.type_name);
            !property.event
                && matches!(
                    kind,
                    "Uri"
                        | "UriRef"
                        | "Rc<dyn IImage>"
                        | "Rc<dyn IImageBrushSource>"
                        | "Rc<dyn IBitmap>"
                        | "Rc<WindowIcon>"
                )
        })
}

fn text_path(document: &Document, node: &Node) -> Option<LiteralPath> {
    let mut text = String::new();
    let mut parts = Vec::new();
    for child in node
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
    {
        let (span, cdata) = match child.kind {
            NodeKind::Text => (child.span, false),
            NodeKind::CData => (
                SourceSpan::new(child.span.start + 9, child.span.end - 3),
                true,
            ),
            NodeKind::Comment => continue,
            _ => return None,
        };
        text.push_str(child.text.as_deref()?);
        let span = trimmed_span(document, span);
        if span.start < span.end {
            parts.push(Part { span, cdata });
        }
    }
    Some(LiteralPath {
        value: literal(&text)?,
        parts,
    })
}

fn literal(source: &str) -> Option<String> {
    let MarkupValue::Literal(text) = MarkupValue::parse(source.trim()).ok()? else {
        return None;
    };
    (!text.is_empty()).then_some(text)
}

fn trimmed_span(document: &Document, span: SourceSpan) -> SourceSpan {
    let raw = &document.source()[span.range()];
    let first = raw.trim_start();
    let start = span.start + raw.len() - first.len();
    SourceSpan::new(start, start + first.trim_end().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copied_fragments_keep_file_and_compiled_relative_asset_identities() {
        let source = "<Grid xmlns='https://github.com/ferroui'>\r\n  <!-- 🖼️ -->\r\n  <Image Source='../Art/Customer%20Icon.png#sample'/>\r\n  <TextBlock Text='../Art/Customer%20Icon.png#sample'/>\r\n</Grid>";
        let document = Document::new(source).unwrap();
        let schema = Schema::ferroui();
        let root = std::env::temp_dir().join("template-copy-assets");
        let uri = |name: &str| ferroui_designer_project::file_uri(root.join(name)).unwrap();
        let copied = relocate_xaml_fragment(
            &document,
            &schema,
            &uri("Templates/Rows.xaml"),
            &uri("View.xaml"),
        )
        .unwrap();
        assert_eq!(
            copied,
            source.replacen("../Art/Customer", "Art/Customer", 1)
        );
        let compiled = relocate_xaml_fragment(
            &document,
            &schema,
            "ferres://Sample/Templates/Rows.xaml",
            &uri("View.xaml"),
        )
        .unwrap();
        assert_eq!(
            compiled,
            source.replacen("../Art/Customer", "ferres://Sample/Art/Customer", 1)
        );
    }
}
