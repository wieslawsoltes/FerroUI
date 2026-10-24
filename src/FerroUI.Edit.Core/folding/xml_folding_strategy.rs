use super::{FoldingError, FoldingManager, NewFolding};
use crate::document::TextDocument;

#[derive(Clone, Copy, Debug, Default)]
pub struct XmlFoldingStrategy {
    pub show_attributes_when_folded: bool,
}
impl XmlFoldingStrategy {
    /// Produces UTF-16 ranges for multiline elements and comments. Invalid XML
    /// returns an error at zero to retain existing regions during incomplete edits.
    pub fn create_new_foldings(&self, text: &str) -> (Vec<NewFolding>, Option<usize>) {
        let Ok(xml) = roxmltree::Document::parse(text) else {
            return (Vec::new(), Some(0));
        };
        let mut folds = Vec::new();
        for node in xml.descendants() {
            if !node.is_element() && !node.is_comment() {
                continue;
            }
            let range = node.range();
            let content = &text[range.clone()];
            if !content.contains('\n') && !content.contains('\r') {
                continue;
            }
            let name = if node.is_comment() {
                let first = content
                    .trim_start_matches("<!--")
                    .split(['\r', '\n'])
                    .next()
                    .unwrap_or("");
                format!("<!--{first}-->")
            } else if self.show_attributes_when_folded {
                let mut quoted = None;
                let mut end = content.len();
                for (i, ch) in content.char_indices() {
                    if let Some(quote) = quoted {
                        if quote == ch {
                            quoted = None;
                        }
                    } else if ch == '\'' || ch == '"' {
                        quoted = Some(ch);
                    } else if ch == '>' {
                        end = i + 1;
                        break;
                    }
                }
                content[..end]
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                let tag = content[1..]
                    .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .next()
                    .unwrap_or(node.tag_name().name());
                format!("<{tag}>")
            };
            let mut fold = NewFolding::new(
                text[..range.start].encode_utf16().count(),
                text[..range.end].encode_utf16().count(),
            );
            fold.name = Some(name);
            folds.push(fold);
        }
        folds.sort_by_key(|s| s.start_offset);
        (folds, None)
    }
    pub fn update_foldings(
        &self,
        manager: &FoldingManager,
        document: &TextDocument,
    ) -> Result<(), FoldingError> {
        let (folds, error) = self.create_new_foldings(&document.text());
        manager.update_foldings(&folds, error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multiline_xml_and_unicode() {
        let text = "<x a='😀'>\n<!-- hi\nthere -->\n<y/>\n</x>";
        let (folds, error) = XmlFoldingStrategy::default().create_new_foldings(text);
        assert_eq!(error, None);
        assert_eq!(folds.len(), 2);
        assert_eq!(folds[0].end_offset, text.encode_utf16().count());
        assert_eq!(folds[1].name.as_deref(), Some("<!-- hi-->"));
        assert_eq!(
            XmlFoldingStrategy::default().create_new_foldings("<x>").1,
            Some(0)
        );
    }
}
