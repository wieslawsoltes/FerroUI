use super::i_active_element::ActiveElementRegistration;
use super::snippet::{Placeholder, PreparedSnippet};
use super::snippet_session::SnippetLifecycle;
use super::{
    IActiveElement, SnippetElement, SnippetElementRef, SnippetError, SnippetEventArgs,
    SnippetSession, SnippetTextTransform,
};
use crate::document::{TextDocument, TextUtilities};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Insertion formatting captured before the selected text is replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnippetInsertionOptions {
    pub indentation: String,
    pub tab: String,
    pub line_terminator: String,
}

impl Default for SnippetInsertionOptions {
    fn default() -> Self {
        Self {
            indentation: String::new(),
            tab: "\t".into(),
            line_terminator: if cfg!(windows) { "\r\n" } else { "\n" }.into(),
        }
    }
}

impl SnippetInsertionOptions {
    pub fn from_document(
        document: &TextDocument,
        offset: usize,
        tab: impl Into<String>,
    ) -> Result<Self, SnippetError> {
        let line = document.get_line_by_offset(offset)?;
        let whitespace = TextUtilities::get_whitespace_after(document, line.offset)?;
        Ok(Self {
            indentation: document.get_text(
                line.offset,
                whitespace.end_offset().min(offset) - line.offset,
            )?,
            tab: tab.into(),
            line_terminator: TextUtilities::get_new_line_from_document(document, line.line_number)?,
        })
    }

    /// Expands tabs before normalizing CR/LF/CRLF. In particular, CRLF pairs
    /// spanning a tab expansion boundary are treated as one newline.
    pub fn format_text<'a>(&self, text: &'a str) -> Result<Cow<'a, str>, SnippetError> {
        if !TextUtilities::is_new_line(&self.line_terminator) {
            return Err(crate::document::DocumentError::InvalidNewLine.into());
        }
        let expanded = if self.tab != "\t" && text.contains('\t') {
            Cow::Owned(text.replace('\t', &self.tab))
        } else {
            Cow::Borrowed(text)
        };
        if !expanded.contains(['\r', '\n']) {
            return Ok(expanded);
        }
        let mut output = String::with_capacity(expanded.len());
        let bytes = expanded.as_bytes();
        let mut position = 0;
        let mut start = 0;
        while position < bytes.len() {
            if bytes[position] != b'\r' && bytes[position] != b'\n' {
                position += 1;
                continue;
            }
            output.push_str(&expanded[start..position]);
            position += if bytes[position] == b'\r' && bytes.get(position + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            output.push_str(&self.line_terminator);
            output.push_str(&self.indentation);
            start = position;
        }
        output.push_str(&expanded[start..]);
        Ok(Cow::Owned(output))
    }
}

/// Checked construction phase of an insertion context. The retained, active
/// phase is [`SnippetSession`]. Offsets here are relative UTF-16 positions.
pub struct SnippetInsertionContext {
    selected_text: String,
    options: SnippetInsertionOptions,
    defaults: BTreeMap<usize, String>,
    text: String,
    position: usize,
    placeholders: Vec<Placeholder>,
    caret: Option<(usize, usize)>,
    anchors: Vec<(String, usize)>,
    registrations: Vec<ActiveElementRegistration>,
    current_owner: Option<SnippetElementRef>,
    lifecycle: SnippetLifecycle,
    depth: usize,
}

impl SnippetInsertionContext {
    pub(crate) fn new(
        selected_text: &str,
        options: SnippetInsertionOptions,
        defaults: BTreeMap<usize, String>,
    ) -> Result<Self, SnippetError> {
        options.format_text("")?;
        Ok(Self {
            selected_text: selected_text.into(),
            options,
            defaults,
            text: String::new(),
            position: 0,
            placeholders: Vec::new(),
            caret: None,
            anchors: Vec::new(),
            registrations: Vec::new(),
            current_owner: None,
            lifecycle: SnippetLifecycle::default(),
            depth: 0,
        })
    }

    pub fn selected_text(&self) -> &str {
        &self.selected_text
    }
    pub fn options(&self) -> &SnippetInsertionOptions {
        &self.options
    }
    pub fn insertion_position(&self) -> usize {
        self.position
    }
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn insert_text(&mut self, text: &str) -> Result<(), SnippetError> {
        let formatted = self.options.format_text(text)?;
        self.position = self
            .position
            .checked_add(formatted.encode_utf16().count())
            .ok_or(SnippetError::InvalidExpansion("snippet length overflow"))?;
        self.text.push_str(&formatted);
        Ok(())
    }

    /// Inserts already-formatted text, as used by selected-text nodes.
    pub fn insert_verbatim(&mut self, text: &str) -> Result<(), SnippetError> {
        self.position = self
            .position
            .checked_add(text.encode_utf16().count())
            .ok_or(SnippetError::InvalidExpansion("snippet length overflow"))?;
        self.text.push_str(text);
        Ok(())
    }

    pub fn set_final_caret(&mut self) {
        self.caret = Some((self.position, self.placeholders.len()));
    }

    pub fn insert_selection(&mut self, indentation: usize) -> Result<(), SnippetError> {
        let indent_length = self.options.tab.len().checked_mul(indentation).ok_or(
            SnippetError::InvalidExpansion("selection indentation overflow"),
        )?;
        let mut indent = String::new();
        indent
            .try_reserve(indent_length)
            .map_err(|_| SnippetError::InvalidExpansion("selection indentation is too large"))?;
        for _ in 0..indentation {
            indent.push_str(&self.options.tab);
        }
        let text = self.selected_text.trim_start_matches([' ', '\t']).replace(
            &self.options.line_terminator,
            &format!("{}{indent}", self.options.line_terminator),
        );
        self.insert_verbatim(&text)?;
        if self.selected_text.is_empty() {
            self.set_final_caret();
        }
        Ok(())
    }

    pub fn insert_replaceable(&mut self, id: usize, text: &str) -> Result<(), SnippetError> {
        let value = self
            .defaults
            .entry(id)
            .or_insert_with(|| text.to_owned())
            .clone();
        self.insert_field(id, &value, true, None)
    }

    pub fn insert_bound(
        &mut self,
        id: usize,
        transform: Option<SnippetTextTransform>,
    ) -> Result<(), SnippetError> {
        let value = self
            .defaults
            .get(&id)
            .ok_or(SnippetError::UnknownPlaceholder(id))?
            .clone();
        let value = if let Some(transform) = &transform {
            transform.convert(&value)?
        } else {
            value
        };
        self.insert_field(id, &value, false, transform)
    }

    fn insert_field(
        &mut self,
        id: usize,
        text: &str,
        editable: bool,
        transform: Option<SnippetTextTransform>,
    ) -> Result<(), SnippetError> {
        let start = self.position;
        self.insert_text(text)?;
        self.placeholders.push(Placeholder {
            id,
            start,
            end: self.position,
            editable,
            transform,
        });
        Ok(())
    }

    pub fn insert_anchor(&mut self, name: impl Into<String>) -> Result<(), SnippetError> {
        let name = name.into();
        if self.anchors.iter().any(|(existing, _)| *existing == name) {
            return Err(SnippetError::InvalidExpansion("duplicate named anchor"));
        }
        self.anchors.push((name, self.position));
        Ok(())
    }

    /// Registers one active element for the currently expanding custom node.
    /// Factories run only after the host has accepted and inserted all text.
    pub fn register_active_element(
        &mut self,
        factory: impl Fn(&SnippetSession) -> Result<Rc<dyn IActiveElement>, SnippetError> + 'static,
    ) -> Result<(), SnippetError> {
        let owner = self
            .current_owner
            .clone()
            .ok_or(SnippetError::InvalidExpansion(
                "active elements must be registered from a custom snippet node",
            ))?;
        if self
            .registrations
            .iter()
            .any(|registration| registration.owner == owner)
        {
            return Err(SnippetError::InvalidExpansion(
                "a snippet node already has an active element",
            ));
        }
        self.registrations.push(ActiveElementRegistration {
            owner,
            factory: Rc::new(factory),
        });
        Ok(())
    }

    pub fn on_insertion_completed(&mut self, handler: impl Fn(&SnippetSession) + 'static) {
        self.lifecycle.insertion_completed.push(Rc::new(handler));
    }
    pub fn on_deactivated(
        &mut self,
        handler: impl Fn(&SnippetSession, &SnippetEventArgs) + 'static,
    ) {
        self.lifecycle.deactivated.push(Rc::new(handler));
    }

    pub fn insert_element(&mut self, element: &SnippetElement) -> Result<(), SnippetError> {
        if self.depth == 256 {
            return Err(SnippetError::InvalidExpansion(
                "snippet expansion recursion limit exceeded",
            ));
        }
        self.depth += 1;
        let result = self.insert_element_inner(element);
        self.depth -= 1;
        result
    }

    fn insert_element_inner(&mut self, element: &SnippetElement) -> Result<(), SnippetError> {
        match element {
            SnippetElement::Text(text) => self.insert_text(text),
            SnippetElement::Replaceable { id, text } => self.insert_replaceable(*id, text),
            SnippetElement::Bound { id } => self.insert_bound(*id, None),
            SnippetElement::TransformedBound { id, transform } => {
                self.insert_bound(*id, Some(transform.clone()))
            }
            SnippetElement::Caret => {
                self.set_final_caret();
                Ok(())
            }
            SnippetElement::CaretIfSelection => {
                if !self.selected_text.is_empty() {
                    self.set_final_caret();
                }
                Ok(())
            }
            SnippetElement::Selection => self.insert_selection(0),
            SnippetElement::SelectionWithIndentation { indentation } => {
                self.insert_selection(*indentation)
            }
            SnippetElement::Anchor { name } => self.insert_anchor(name.clone()),
            SnippetElement::Container(children) => {
                for child in children {
                    self.insert_element(child)?;
                }
                Ok(())
            }
            SnippetElement::Custom(element) => {
                let previous = self.current_owner.replace(element.clone());
                let result = element.0.insert(self);
                self.current_owner = previous;
                result
            }
        }
    }

    pub(crate) fn finish(self) -> PreparedSnippet {
        let (caret, caret_order) = self
            .caret
            .unwrap_or((self.position, self.placeholders.len()));
        PreparedSnippet {
            text: self.text,
            placeholders: self.placeholders,
            caret,
            caret_order,
            lifecycle: self.lifecycle,
            anchors: self.anchors,
            registrations: self.registrations,
        }
    }
}
