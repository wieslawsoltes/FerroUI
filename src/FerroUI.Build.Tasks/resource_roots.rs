//! Opt-in closed-world pruning before XamlX expands merged dictionaries.
//! Keep every declaration of a reachable name (including overrides and theme variants).
//! Unrecognized key expressions stay in the document. A retained resource is an atomic subtree,
//! so its local dictionaries, template scopes and names keep their original semantics.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use ferroui_base::metadata::FERRO_XML_NAMESPACE;
use roxmltree::{Document, Node};
use xamlx::XamlNamespaces;

fn identifier(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

// Only prune direct, explicitly keyed entries of the framework's own ResourceDictionary.
fn key<'a>(node: Node<'a, '_>) -> Option<&'a str> {
    let parent = node.parent_element()?;
    if !parent.has_tag_name((FERRO_XML_NAMESPACE, "ResourceDictionary")) {
        return None;
    }
    let key = node.attribute((XamlNamespaces::XAML2006, "Key"))?;
    if identifier(key) {
        return Some(key);
    }
    let expression = key.strip_prefix('{')?.strip_suffix('}')?;
    let (extension, type_name) = expression.split_once(char::is_whitespace)?;
    let (prefix, local) = extension.split_once(':')?;
    if local != "Type" || node.lookup_namespace_uri(Some(prefix)) != Some(XamlNamespaces::XAML2006)
    {
        return None;
    }
    let type_name = type_name.trim();
    let local = type_name
        .split_once(':')
        .map_or(type_name, |(_, local)| local);
    identifier(local).then_some(local)
}

struct Entry {
    document: usize,
    range: Range<usize>,
    dependencies: HashSet<usize>,
}

#[derive(Default)]
struct Graph<'a> {
    names: HashMap<&'a str, usize>,
    definitions: Vec<Vec<usize>>,
    entries: Vec<Entry>,
}

impl Graph<'_> {
    fn references(&self, text: &str, found: &mut HashSet<usize>) {
        // XAML contains references in extension arguments, property-element text, selectors and
        // implicit type keys. Matching complete identifier tokens also conservatively keeps keys
        // that happen to have the same spelling in another scope or namespace.
        for token in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
            if let Some(&id) = self.names.get(token) {
                found.insert(id);
            }
        }
    }

    fn scan(&self, node: Node<'_, '_>, found: &mut HashSet<usize>) {
        if node.is_text() {
            self.references(node.text().unwrap_or_default(), found);
        } else if node.is_element() {
            let tag = node.tag_name();
            self.references(tag.name(), found);
            let include = tag.namespace() == Some(FERRO_XML_NAMESPACE)
                && matches!(
                    tag.name(),
                    "ResourceInclude" | "MergeResourceInclude" | "StyleInclude"
                );
            for attr in node.attributes() {
                if include
                    && attr.namespace().is_none()
                    && attr.name() == "Source"
                    && !attr.value().starts_with('{')
                {
                    continue;
                }
                self.references(attr.value(), found);
            }
        }
    }
}

/// Blanks unreachable elements in place, preserving the original positions for diagnostics.
pub(super) fn trim(documents: &mut [(String, String)], roots: &[String]) -> Result<(), String> {
    let parsed: Vec<_> = documents
        .iter()
        .map(|(name, xaml)| Document::parse(xaml).map_err(|e| format!("{name}: {e}")))
        .collect::<Result<_, _>>()?;
    let mut graph = Graph::default();
    // Only the outermost keyed entry is removable. Each document is traversed in source order.
    let mut per_document = Vec::with_capacity(parsed.len());
    for (document, xml) in parsed.iter().enumerate() {
        let mut entries = Vec::new();
        let mut end = 0;
        for node in xml.descendants().filter(|n| n.is_element()) {
            if node.range().start < end {
                continue;
            }
            if let Some(name) = key(node) {
                let count = graph.definitions.len();
                let id = *graph.names.entry(name).or_insert(count);
                if id == count {
                    graph.definitions.push(Vec::new());
                }
                let index = graph.entries.len();
                graph.definitions[id].push(index);
                let range = node.range();
                end = range.end;
                graph.entries.push(Entry {
                    document,
                    range,
                    dependencies: HashSet::new(),
                });
                entries.push((node.id(), index));
            }
        }
        per_document.push(entries);
    }

    let mut ambient = HashSet::new();
    for (xml, entries) in parsed.iter().zip(&per_document) {
        let mut entry_iter = entries.iter().peekable();
        let mut current: Option<(usize, usize)> = None;
        let mut deps = HashSet::new();
        let mut design_end = 0;
        for node in xml.descendants() {
            let start = node.range().start;
            if current.is_some_and(|(_, end)| start >= end) {
                let (index, _) = current.take().unwrap();
                graph.entries[index].dependencies = std::mem::take(&mut deps);
            }
            if entry_iter.peek().is_some_and(|(id, _)| *id == node.id()) {
                let (_, index) = *entry_iter.next().unwrap();
                current = Some((index, node.range().end));
            }
            if start < design_end {
                continue;
            }
            if node.has_tag_name((FERRO_XML_NAMESPACE, "Design.PreviewWith")) {
                design_end = node.range().end;
                continue;
            }
            graph.scan(
                node,
                if current.is_some() {
                    &mut deps
                } else {
                    &mut ambient
                },
            );
        }
        if let Some((index, _)) = current {
            graph.entries[index].dependencies = deps;
        }
    }
    let mut pending: Vec<_> = ambient.into_iter().collect();
    for root in roots {
        let id = graph.names.get(root.as_str()).ok_or_else(|| {
            format!("unknown XAML resource root '{root}' (expected an explicit string key or local x:Type key in this group)")
        })?;
        pending.push(*id);
    }
    let mut live = vec![false; graph.definitions.len()];
    while let Some(id) = pending.pop() {
        if !std::mem::replace(&mut live[id], true) {
            for &entry in &graph.definitions[id] {
                pending.extend(&graph.entries[entry].dependencies);
            }
        }
    }
    let mut removed = vec![Vec::new(); documents.len()];
    for (id, entries) in graph.definitions.iter().enumerate() {
        if !live[id] {
            for &index in entries {
                let entry = &graph.entries[index];
                removed[entry.document].push(entry.range.clone());
            }
        }
    }
    drop(graph);
    drop(parsed);
    for ((_, source), ranges) in documents.iter_mut().zip(&mut removed) {
        if ranges.is_empty() {
            continue;
        }
        ranges.sort_unstable_by_key(|r| r.start);
        let mut output = String::with_capacity(source.len());
        let mut pos = 0;
        for r in ranges {
            output.push_str(&source[pos..r.start]);
            // XamlX reports columns in UTF-16 code units, including after a non-BMP scalar.
            output.extend(source[r.clone()].encode_utf16().map(|unit| match unit {
                10 => '\n',
                13 => '\r',
                _ => ' ',
            }));
            pos = r.end;
        }
        output.push_str(&source[pos..]);
        *source = output;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dictionary(body: &str) -> (String, String) {
        (
            "Theme.xaml".into(),
            format!(
                r#"<ResourceDictionary xmlns="{FERRO_XML_NAMESPACE}" xmlns:x="{}">{body}</ResourceDictionary>"#,
                XamlNamespaces::XAML2006
            ),
        )
    }

    #[test]
    fn follows_cycles_types_overrides_and_variant_keys_across_documents() {
        let mut docs = vec![
            dictionary(r#"<ControlTheme x:Key="{x:Type Button}" BasedOn="{StaticResource Default}"/><Style x:Key="Dead"><Other/></Style>"#),
            dictionary(r#"<Style x:Key="Default"><TextBlock Text="{DynamicResource Accent}"/></Style><Style x:Key="{x:Type TextBlock}"/><Color x:Key="Accent">Default</Color>"#),
            dictionary("<Color x:Key=\"Accent\"/>\n<Color x:Key=\"Other\"/>\n<Color x:Key=\"special.key\"/>"),
        ];
        trim(&mut docs, &["Button".into()]).unwrap();
        assert!(docs[0].1.contains("{x:Type Button}"));
        assert!(!docs[0].1.contains("Dead"));
        assert!(docs[1].1.contains("{x:Type TextBlock}"));
        assert!(docs[1].1.contains("Accent"));
        assert!(docs[2].1.contains("Accent"));
        assert!(!docs[2].1.contains("Other"));
        assert!(docs[2].1.contains("special.key"));
        assert_eq!(docs[2].1.lines().count(), 3);
    }

    #[test]
    fn pruning_preserves_utf16_positions_of_surviving_resources() {
        let mut docs = vec![dictionary("<Color x:Key=\"Gone\">😀</Color><Color x:Key=\"Keep\"/>")];
        let position = |s: &str| s[..s.find("Keep").unwrap()].encode_utf16().count();
        let before = position(&docs[0].1);
        trim(&mut docs, &["Keep".into()]).unwrap();
        assert_eq!(position(&docs[0].1), before);
        assert!(!docs[0].1.contains("Gone"));
    }

    #[test]
    fn includes_are_paths_ambient_markup_roots_references_and_unknown_roots_fail() {
        let mut docs = vec![dictionary(
            r#"<StyleInclude Source="/Unused.xaml"/><Style BasedOn="{StaticResource Kept}"/><Brush x:Key="Kept"/><Brush x:Key="Unused"/>"#,
        )];
        let original = docs.clone();
        assert!(trim(&mut docs, &["Typo".into()])
            .unwrap_err()
            .contains("Typo"));
        assert_eq!(docs, original);
        trim(&mut docs, &[]).unwrap();
        assert!(!docs[0].1.contains("x:Key=\"Unused\""));
        assert!(docs[0].1.contains("x:Key=\"Kept\""));
    }
}
