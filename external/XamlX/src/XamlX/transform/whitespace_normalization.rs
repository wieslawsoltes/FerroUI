//! Port of `Transform/WhitespaceNormalization.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstCast, XamlAstExtensions, XamlAstNodeExtensions,
    XamlAstObjectNode, XamlAstTextNode,
};
use crate::exceptions::XamlResult;

use super::TransformerConfiguration;

pub struct WhitespaceNormalization;

impl WhitespaceNormalization {
    /// NOTE: empty text nodes count as whitespace
    pub fn is_whitespace(text: &str) -> bool {
        text.chars().all(Self::is_whitespace_char)
    }

    fn is_whitespace_char(ch: char) -> bool {
        // While the XAML spec does not list \r, the implementation still considers it
        // Usually, the XML parser will already normalize newlines, but one can still
        // insert a character entity (&#x0D;) to bypass it, which will then be caught here.
        ch == ' ' || ch == '\n' || ch == '\t' || ch == '\r'
    }

    pub fn normalize_whitespace(text: &str, trim_start: bool, trim_end: bool) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut start = 0;
        if trim_start {
            while start < chars.len() && Self::is_whitespace_char(chars[start]) {
                start += 1;
            }

            if start >= chars.len() {
                return String::new(); // The entire string was whitespace
            }
        }

        // End is exclusive
        let mut end = chars.len();
        if trim_end {
            while end > start && Self::is_whitespace_char(chars[end - 1]) {
                end -= 1;
            }

            if end <= start {
                return String::new(); // The entire string was whitespace
            }
        }

        // 1) Convert all white-space characters into spaces (new-lines become spaces, etc.)
        // 2) Collapse all subsequent white-space characters into a single space
        let mut result = String::with_capacity(end - start);
        let mut i = start;
        while i < end {
            let mut ch = chars[i];
            if Self::is_whitespace_char(ch) {
                ch = ' '; // All whitespace is normalized to spaces
                          // Consume all whitespace directly following this
                while i + 1 < end && Self::is_whitespace_char(chars[i + 1]) {
                    i += 1;
                }
            }

            result.push(ch);
            i += 1;
        }

        result
    }

    /// Applies the whitespace normalization process and content model transformation.
    pub fn apply(
        content_nodes: &mut Vec<Rc<dyn IXamlAstValueNode>>,
        config: &TransformerConfiguration,
    ) -> XamlResult<()> {
        fn should_trim_whitespace_around(
            content_nodes: &[Rc<dyn IXamlAstValueNode>],
            index: usize,
            config: &TransformerConfiguration,
        ) -> XamlResult<bool> {
            match content_nodes[index].cast::<XamlAstObjectNode>() {
                Some(object_node) => Ok(config.is_trim_surrounding_whitespace_element(
                    &*object_node.type_.borrow().get_clr_type()?,
                )),
                None => Ok(false),
            }
        }

        let mut i = content_nodes.len();
        while i > 0 {
            i -= 1;
            let node = content_nodes[i].clone();

            if let Some(text_node) = node.cast::<XamlAstTextNode>() {
                // XAML whitespace normalization can be disabled via xml:space="preserve" on an element or
                // any of its ancestors.
                if !text_node.preserve_whitespace {
                    // Trim spaces immediately following the start tag or following a tag that wants surrounding
                    // whitespace trimmed
                    let trim_start =
                        i == 0 || should_trim_whitespace_around(content_nodes, i - 1, config)?;

                    // Trim spaces immediately preceding the end tag or preceding a tag that wants surrounding
                    // whitespace trimmed
                    let trim_end = i >= content_nodes.len() - 1
                        || should_trim_whitespace_around(content_nodes, i + 1, config)?;

                    let normalized =
                        Self::normalize_whitespace(&text_node.text.borrow(), trim_start, trim_end);
                    let is_empty = normalized.is_empty();
                    *text_node.text.borrow_mut() = normalized;
                    if is_empty {
                        // Remove text nodes that have been trimmed in their entirety
                        content_nodes.remove(i);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn remove_whitespace_nodes<K: ?Sized + XamlAstCast>(nodes: &mut Vec<Rc<K>>) {
        nodes.retain(|node| !Self::should_remove_node(&*node.as_node()));
    }

    fn should_remove_node(node: &dyn IXamlAstNode) -> bool {
        if let Some(text_child) = node.as_any().downcast_ref::<XamlAstTextNode>() {
            return Self::is_whitespace(&text_child.text.borrow());
        }
        if let Some(side_effect_node) = node.as_value_with_side_effect_node_base() {
            return Self::should_remove_node(&*side_effect_node.value());
        }
        false
    }
}
