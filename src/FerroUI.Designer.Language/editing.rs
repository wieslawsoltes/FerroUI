use crate::{DocumentEdits, LanguageService, Range, Result, TextEdit, apply_text_edits};
use ferroui_designer_core::{Document, Node, SourceSpan};

#[derive(Clone, Debug)]
pub struct QuickFix {
    pub title: String,
    pub edits: DocumentEdits,
}

impl LanguageService {
    /// Return the authored opening and closing names from the same XML element.
    pub fn linked_tags(&self, uri: &str, offset: usize) -> Result<Option<[Range; 2]>> {
        let Some(document) = self.document(uri).filter(|document| document.is_valid()) else {
            return Ok(None);
        };
        self.index(uri)?.position(offset)?;
        let pair = document.ast().elements().find_map(|node| {
            let names = name_pair(node)?;
            names
                .iter()
                .any(|span| span.start <= offset && offset <= span.end)
                .then_some(names)
        });
        pair.map(|[open, close]| {
            Ok([
                self.index(uri)?.range(open)?,
                self.index(uri)?.range(close)?,
            ])
        })
        .transpose()
    }

    pub fn quick_fixes(&self, uri: &str, range: Range) -> Result<Vec<QuickFix>> {
        let index = self.index(uri)?;
        let span = index.span(range)?;
        let document = self.document(uri).expect("indexed document");
        let version = self.version(uri).expect("indexed version");
        let identity = self.document_uri(uri)?.into_owned();
        let mut fixes = Vec::new();
        for diagnostic in self.diagnostics(uri)? {
            if diagnostic.span.end < span.start || diagnostic.span.start > span.end {
                continue;
            }
            let proposed = match diagnostic.code.as_str() {
                "unknown-type" => self.namespace_fixes(uri, document, diagnostic.span)?,
                "unknown-property" => self.property_fix(uri, document, diagnostic.span)?,
                _ => Vec::new(),
            };
            for (title, patches) in proposed {
                let edits = patches
                    .into_iter()
                    .map(|(span, new_text)| {
                        Ok(TextEdit {
                            range: index.range(span)?,
                            new_text,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Document::new(apply_text_edits(document.source(), &edits)?)?;
                fixes.push(QuickFix {
                    title,
                    edits: DocumentEdits {
                        uri: identity.clone(),
                        version,
                        edits,
                    },
                });
            }
        }
        Ok(fixes)
    }

    fn namespace_fixes(
        &self,
        uri: &str,
        document: &Document,
        name: SourceSpan,
    ) -> Result<Vec<Proposal>> {
        let ast = document.ast();
        let Some(node) = ast
            .elements()
            .find(|node| node.name_span == name && !node.name.contains(':'))
        else {
            return Ok(Vec::new());
        };
        let scope = ast.namespaces(node.id);
        let root = ast.node(ast.root()).expect("valid document root");
        let registry = self.schema_for(uri);
        Ok(registry
            .types
            .values()
            .filter(|info| {
                info.constructible && info.name == node.name && !info.namespace.is_empty()
            })
            .map(|info| {
                let mut edits = Vec::new();
                let prefix = scope
                    .iter()
                    .find(|(prefix, ns)| !prefix.is_empty() && *ns == &info.namespace)
                    .map(|(prefix, _)| prefix.clone())
                    .unwrap_or_else(|| {
                        let prefix = unused_prefix(document);
                        let namespace = info
                            .namespace
                            .replace('&', "&amp;")
                            .replace('"', "&quot;")
                            .replace('<', "&lt;");
                        let declaration = format!(" xmlns:{prefix}=\"{namespace}\"");
                        edits.push((
                            SourceSpan::new(root.name_span.end, root.name_span.end),
                            declaration,
                        ));
                        prefix
                    });
                let qualified = format!("{prefix}:{}", node.name);
                edits.push((node.name_span, qualified.clone()));
                if let Some([_, close]) = name_pair(node) {
                    edits.push((close, qualified));
                }
                (
                    format!("Import {} for {}", info.namespace, info.name),
                    edits,
                )
            })
            .collect())
    }

    fn property_fix(
        &self,
        uri: &str,
        document: &Document,
        name: SourceSpan,
    ) -> Result<Vec<Proposal>> {
        let Some((node, attribute)) = document.ast().elements().find_map(|node| {
            node.attributes
                .iter()
                .find(|attr| attr.name_span == name && !attr.name.contains([':', '.']))
                .map(|attr| (node, attr))
        }) else {
            return Ok(Vec::new());
        };
        let properties = self
            .schema_for(uri)
            .properties(&node.name, node.namespace_uri.as_deref());
        let mut matches = properties.values().filter(|property| {
            !property.read_only
                && !property.event
                && !property.attached
                && property.name.eq_ignore_ascii_case(&attribute.name)
                && node.attribute(&property.name).is_none()
        });
        Ok(match (matches.next(), matches.next()) {
            (Some(property), None) => vec![(
                format!("Use {}", property.name),
                vec![(name, property.name.clone())],
            )],
            _ => Vec::new(),
        })
    }
}

type Proposal = (String, Vec<(SourceSpan, String)>);

fn name_pair(node: &Node) -> Option<[SourceSpan; 2]> {
    let close = node.closing_span?;
    let start = close.start + 2; // </
    Some([
        node.name_span,
        SourceSpan::new(start, start + node.name_span.len()),
    ])
}

fn unused_prefix(document: &Document) -> String {
    let used: std::collections::BTreeSet<_> = document
        .ast()
        .elements()
        .flat_map(|node| {
            node.attributes
                .iter()
                .filter_map(|attribute| attribute.name.strip_prefix("xmlns:"))
        })
        .collect();
    (1..)
        .map(|number| {
            if number == 1 {
                "app".into()
            } else {
                format!("app{number}")
            }
        })
        .find(|prefix: &String| !used.contains(prefix.as_str()))
        .expect("finite XML namespace declarations")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FormattingOptions, LineIndex, TypeInfo};

    const URI: &str = "untitled:/View.xaml";

    #[test]
    fn links_paired_tags_at_utf16_positions_and_keeps_range_formatting_local() {
        let source =
            "<Grid>\r\n<!-- 😀 --><StackPanel><Button/></StackPanel><TextBlock/>\r\n</Grid>";
        let mut service = LanguageService::default();
        service.open(URI, source, 1).unwrap();
        let offset = source.find("<StackPanel>").unwrap() + 2;
        let [opening, closing] = service.linked_tags(URI, offset).unwrap().unwrap();
        let index = LineIndex::new(source);
        for range in [opening, closing] {
            assert_eq!(&source[index.span(range).unwrap().range()], "StackPanel");
        }
        assert!(
            service
                .linked_tags(URI, source.find("Button").unwrap())
                .unwrap()
                .is_none()
        );
        let start = source.find("<StackPanel>").unwrap();
        let end = source.find("</StackPanel>").unwrap() + "</StackPanel>".len();
        let range = index.range(SourceSpan::new(start, end)).unwrap();
        let edits = service
            .format_range(URI, range, FormattingOptions::default())
            .unwrap();
        assert_eq!(
            apply_text_edits(source, &edits).unwrap(),
            "<Grid>\r\n<!-- 😀 --><StackPanel>\r\n        <Button/>\r\n    </StackPanel><TextBlock/>\r\n</Grid>"
        );
        service.replace(URI, "<Grid>", 2).unwrap();
        assert!(service.linked_tags(URI, 1).unwrap().is_none());
    }

    #[test]
    fn import_corrects_both_element_names_and_preserves_an_existing_prefix() {
        let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:app="using:Other"><StatusCard Width="40"></StatusCard></Grid>"#;
        let mut service = LanguageService::default();
        service.schema_mut().register(TypeInfo {
            name: "StatusCard".into(),
            namespace: "using:Example".into(),
            constructible: true,
            ..Default::default()
        });
        service.open(URI, source, 7).unwrap();
        let index = LineIndex::new(source);
        let range = index
            .range(SourceSpan::new(
                source.find("StatusCard").unwrap(),
                source.find(" Width").unwrap(),
            ))
            .unwrap();
        let fixes = service.quick_fixes(URI, range).unwrap();
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].edits.version, 7);
        let fixed = apply_text_edits(source, &fixes[0].edits.edits).unwrap();
        assert!(fixed.contains(r#"xmlns:app2="using:Example""#));
        assert!(fixed.contains(r#"<app2:StatusCard Width="40"></app2:StatusCard>"#));
    }
}
