use crate::{apply_text_edits, require, LineIndex, Result, TextEdit};
use ferroui_designer_core::{Document, NodeKind, SourceSpan};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FormattingOptions {
    pub tab_size: u32,
    pub insert_spaces: bool,
    pub trim_trailing_whitespace: bool,
    pub insert_final_newline: bool,
}
impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: true,
            insert_final_newline: true,
        }
    }
}
pub(crate) fn format(document: &Document, options: FormattingOptions) -> Result<Vec<TextEdit>> {
    require(document.is_valid(), "Repair invalid XML before formatting")?;
    require(
        (1..=16).contains(&options.tab_size),
        "tabSize must be from 1 through 16",
    )?;
    let source = document.source();
    let index = LineIndex::new(source);
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let indent = if options.insert_spaces {
        " ".repeat(options.tab_size as usize)
    } else {
        "\t".into()
    };
    let mut patches = Vec::new();
    for node in document.ast().elements() {
        let Some(closing) = node.closing_span else {
            continue;
        };
        if preserves(document, node.id) {
            continue;
        }
        let children: Vec<_> = node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .collect();
        if children.iter().any(|n| {
            n.kind == NodeKind::CData
                || n.kind == NodeKind::Text
                    && n.text
                        .as_deref()
                        .is_some_and(|text| !text.trim().is_empty())
        }) {
            continue;
        }
        let children: Vec<_> = children
            .into_iter()
            .filter(|n| n.kind != NodeKind::Text)
            .collect();
        if children.is_empty() {
            continue;
        }
        let mut depth = 0;
        let mut parent = node.parent;
        while let Some(id) = parent {
            depth += 1;
            parent = document.ast().node(id).and_then(|n| n.parent);
        }
        let mut previous = node.opening_span.end;
        for child in children {
            let span = SourceSpan::new(previous, child.span.start);
            if source[span.range()].chars().all(char::is_whitespace) {
                patches.push((span, format!("{newline}{}", indent.repeat(depth + 1))));
            }
            previous = child.span.end;
        }
        let span = SourceSpan::new(previous, closing.start);
        if source[span.range()].chars().all(char::is_whitespace) {
            patches.push((span, format!("{newline}{}", indent.repeat(depth))));
        }
    }
    let root_end = document.ast().node(document.ast().root()).unwrap().span.end;
    if source[root_end..].chars().all(char::is_whitespace) {
        let trailing = &source[root_end..];
        if options.insert_final_newline {
            patches.push((SourceSpan::new(root_end, source.len()), newline.into()));
        } else if options.trim_trailing_whitespace && !trailing.contains('\n') {
            patches.push((SourceSpan::new(root_end, source.len()), String::new()));
        }
    }
    patches.sort_by_key(|(span, _)| (span.start, span.end));
    let edits = patches
        .into_iter()
        .filter(|(span, text)| &source[span.range()] != text)
        .map(|(span, new_text)| {
            Ok(TextEdit {
                range: index.range(span)?,
                new_text,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Document::new(apply_text_edits(source, &edits)?)?;
    Ok(edits)
}
fn preserves(document: &Document, id: u64) -> bool {
    let mut current = Some(id);
    while let Some(id) = current {
        let node = document.ast().node(id).unwrap();
        if let Some(value) = node.attribute("xml:space") {
            return value == "preserve";
        }
        current = node.parent;
    }
    false
}
