use super::i_active_element::ActiveElementRegistration;
use super::snippet_session::SnippetLifecycle;
use super::{
    SnippetElementRef, SnippetEventArgs, SnippetInsertionContext, SnippetInsertionOptions,
    SnippetSession, SnippetTextTransform,
};
use crate::document::{DocumentError, TextDocument};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnippetElement {
    Text(String),
    Replaceable {
        id: usize,
        text: String,
    },
    Bound {
        id: usize,
    },
    TransformedBound {
        id: usize,
        transform: SnippetTextTransform,
    },
    Anchor {
        name: String,
    },
    Custom(SnippetElementRef),
    Caret,
    CaretIfSelection,
    Selection,
    SelectionWithIndentation {
        indentation: usize,
    },
    Container(Vec<SnippetElement>),
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snippet {
    pub elements: Vec<SnippetElement>,
}
#[derive(Debug)]
pub enum SnippetError {
    Document(DocumentError),
    UnknownPlaceholder(usize),
    InvalidSyntax(usize),
    MultipleSelections,
    ExpansionMismatch,
    InvalidExpansion(&'static str),
    Extension(String),
    Closed,
}
impl fmt::Display for SnippetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(e) => e.fmt(f),
            Self::UnknownPlaceholder(id) => write!(f, "No replaceable element {id} exists"),
            Self::InvalidSyntax(offset) => {
                write!(f, "Invalid snippet syntax at UTF-8 byte {offset}")
            }
            Self::InvalidExpansion(message) => f.write_str(message),
            Self::Extension(message) => f.write_str(message),
            Self::Closed => f.write_str("The snippet session is closed"),
            Self::MultipleSelections => f.write_str("A linked snippet requires one selection"),
            Self::ExpansionMismatch => {
                f.write_str("The inserted text does not match the prepared snippet")
            }
        }
    }
}
impl std::error::Error for SnippetError {}
impl From<DocumentError> for SnippetError {
    fn from(value: DocumentError) -> Self {
        Self::Document(value)
    }
}

pub(crate) struct Placeholder {
    pub id: usize,
    pub start: usize,
    pub end: usize,
    pub editable: bool,
    pub transform: Option<SnippetTextTransform>,
}

/// A validated expansion that can be inserted by a host's editing transaction.
/// Separating preparation from activation lets a host enforce protected ranges
/// without removing selected text before an invalid snippet is diagnosed.
pub struct PreparedSnippet {
    pub(crate) text: String,
    pub(crate) placeholders: Vec<Placeholder>,
    pub(crate) caret: usize,
    pub(crate) caret_order: usize,
    pub(crate) lifecycle: SnippetLifecycle,
    pub(crate) anchors: Vec<(String, usize)>,
    pub(crate) registrations: Vec<ActiveElementRegistration>,
}
impl PreparedSnippet {
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Registers a callback before activation, so even a snippet with no tab
    /// stops reports completion before its immediate deactivation.
    pub fn on_insertion_completed(&mut self, handler: impl Fn(&SnippetSession) + 'static) {
        self.lifecycle.insertion_completed.push(Rc::new(handler));
    }
    pub fn on_deactivated(
        &mut self,
        handler: impl Fn(&SnippetSession, &SnippetEventArgs) + 'static,
    ) {
        self.lifecycle.deactivated.push(Rc::new(handler));
    }
    pub fn activate(
        self,
        document: TextDocument,
        offset: usize,
    ) -> Result<SnippetSession, SnippetError> {
        self.activate_with_validator(document, offset, None)
    }
    /// Installs the host edit policy before custom activation callbacks run.
    pub fn activate_with_validator(
        self,
        document: TextDocument,
        offset: usize,
        edit_validator: Option<Rc<super::SnippetEditValidator>>,
    ) -> Result<SnippetSession, SnippetError> {
        if document.get_text(offset, self.text.encode_utf16().count())? != self.text {
            return Err(SnippetError::ExpansionMismatch);
        }
        let session = SnippetSession::new(
            document,
            offset,
            self.placeholders,
            self.caret,
            self.caret_order,
            self.text.encode_utf16().count(),
            self.lifecycle,
            self.anchors,
            self.registrations,
            edit_validator,
        )?;
        session.track_insertion_undo()?;
        Ok(session)
    }
}
impl Snippet {
    /// Parses numeric tab stops, `${1:default}`, linked `$1` references and `$0`.
    /// Variables, choices and regular-expression transformations are deliberately
    /// rejected; the structured element API is unambiguous for custom snippets.
    pub fn parse(source: &str) -> Result<Self, SnippetError> {
        let mut elements = Vec::new();
        let mut literal = String::new();
        let mut chars = source.char_indices().peekable();
        while let Some((offset, ch)) = chars.next() {
            if ch == '\\' {
                if let Some((_, escaped)) = chars.next() {
                    literal.push(escaped);
                } else {
                    literal.push('\\');
                }
                continue;
            }
            if ch != '$' {
                literal.push(ch);
                continue;
            }
            if !literal.is_empty() {
                elements.push(SnippetElement::Text(std::mem::take(&mut literal)));
            }
            let braced = chars.peek().is_some_and(|(_, c)| *c == '{');
            if braced {
                chars.next();
            }
            let mut digits = String::new();
            while let Some(&(_, c)) = chars.peek() {
                if !c.is_ascii_digit() {
                    break;
                }
                digits.push(c);
                chars.next();
            }
            let id = digits
                .parse::<usize>()
                .map_err(|_| SnippetError::InvalidSyntax(offset))?;
            let mut default = None;
            if braced {
                match chars.next() {
                    Some((_, ':')) => {
                        let mut text = String::new();
                        let mut closed = false;
                        while let Some((_, c)) = chars.next() {
                            if c == '}' {
                                closed = true;
                                break;
                            }
                            if c == '\\' {
                                if let Some((_, c)) = chars.next() {
                                    text.push(c);
                                } else {
                                    return Err(SnippetError::InvalidSyntax(offset));
                                }
                            } else if c == '$' || c == '{' {
                                return Err(SnippetError::InvalidSyntax(offset));
                            } else {
                                text.push(c);
                            }
                        }
                        if !closed {
                            return Err(SnippetError::InvalidSyntax(offset));
                        }
                        default = Some(text);
                    }
                    Some((_, '}')) => {}
                    _ => return Err(SnippetError::InvalidSyntax(offset)),
                }
            }
            elements.push(if id == 0 {
                SnippetElement::Caret
            } else if let Some(text) = default {
                SnippetElement::Replaceable { id, text }
            } else {
                SnippetElement::Bound { id }
            });
        }
        if !literal.is_empty() {
            elements.push(SnippetElement::Text(literal));
        }
        // A bare tab stop with no default anywhere is an empty editable stop.
        let declared: std::collections::BTreeSet<_> = elements
            .iter()
            .filter_map(|e| {
                if let SnippetElement::Replaceable { id, .. } = e {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        let mut first = std::collections::BTreeSet::new();
        for element in &mut elements {
            if let SnippetElement::Bound { id } = element {
                if !declared.contains(id) && first.insert(*id) {
                    *element = SnippetElement::Replaceable {
                        id: *id,
                        text: String::new(),
                    };
                }
            }
        }
        Ok(Self { elements })
    }
    pub fn insert(
        &self,
        document: TextDocument,
        offset: usize,
        selected_text: &str,
    ) -> Result<SnippetSession, SnippetError> {
        let options = SnippetInsertionOptions::from_document(&document, offset, "\t")?;
        let version = document.version();
        let prepared = self.prepare_with_options(selected_text, options)?;
        if document.version() != version {
            return Err(DocumentError::ReentrantChange.into());
        }
        let _update = document.begin_update()?;
        if document.version() != version {
            return Err(DocumentError::ReentrantChange.into());
        }
        document.insert(offset, prepared.text())?;
        prepared.activate(document.clone(), offset)
    }

    pub fn prepare(&self, selected_text: &str) -> Result<PreparedSnippet, SnippetError> {
        self.prepare_with_options(selected_text, SnippetInsertionOptions::default())
    }

    pub fn prepare_with_options(
        &self,
        selected_text: &str,
        options: SnippetInsertionOptions,
    ) -> Result<PreparedSnippet, SnippetError> {
        let mut defaults = BTreeMap::new();
        collect_defaults(&self.elements, &mut defaults);
        let mut context = SnippetInsertionContext::new(selected_text, options, defaults)?;
        for element in &self.elements {
            context.insert_element(element)?;
        }
        Ok(context.finish())
    }
}
fn collect_defaults(elements: &[SnippetElement], defaults: &mut BTreeMap<usize, String>) {
    let mut pending: Vec<_> = elements.iter().rev().collect();
    while let Some(element) = pending.pop() {
        match element {
            SnippetElement::Replaceable { id, text } => {
                defaults.entry(*id).or_insert_with(|| text.clone());
            }
            SnippetElement::Container(children) => pending.extend(children.iter().rev()),
            _ => {}
        }
    }
}
