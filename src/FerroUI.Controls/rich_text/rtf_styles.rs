//! RTF stylesheet resolution, independent of the unavailable native RichEdit engine.
//!
//! Microsoft RTF 1.9.1, "Style Sheet", pages 24–26: paragraph/character handles,
//! based-on chains, additive character styles, and direct formatting after a style.
//! Definitions are resolved to the existing FerroUI character/paragraph model.
//! Style names, editing shortcuts, next-style policies and live named-style identity
//! are not reconstructed from the resolved appearance.

use super::{Node, RtfError};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StyleKind {
    Paragraph,
    Character,
    Section,
    Table,
}

impl StyleKind {
    pub(super) fn from_word(word: &str) -> Option<Self> {
        Some(match word {
            "s" => Self::Paragraph,
            "cs" => Self::Character,
            "ds" => Self::Section,
            "ts" => Self::Table,
            _ => return None,
        })
    }
}

pub(super) struct StyleControl {
    pub(super) word: String,
    pub(super) number: Option<i32>,
}

pub(super) struct StyleDefinition {
    kind: StyleKind,
    based_on: Option<i32>,
    pub(super) additive: bool,
    pub(super) controls: Vec<StyleControl>,
}

#[derive(Default)]
pub(super) struct StyleTable {
    definitions: BTreeMap<i32, Rc<StyleDefinition>>,
}

pub(super) fn style_handle(number: Option<i32>) -> Result<i32, RtfError> {
    number
        .filter(|value| (i16::MIN as i32..=u16::MAX as i32).contains(value))
        .ok_or(RtfError::InvalidParameter("style handle"))
}

impl StyleTable {
    pub(super) fn read(nodes: &[Node<'_>]) -> Result<Self, RtfError> {
        let mut table = Self::default();
        for node in nodes {
            let Node::Group(children) = node else {
                continue;
            };
            if super::destination(children)
                .is_some_and(|(word, starred)| starred && StyleKind::from_word(word).is_none())
            {
                // Unknown starred destinations are ignorable extensions, not
                // an implicit paragraph style zero. In particular, metadata
                // cannot replace Normal or inject formatting into its body.
                continue;
            }
            // The optional default definition has neither a style tag nor a
            // name. Character, section and table definitions use a starred tag.
            let mut id = 0;
            let mut declared = false;
            let mut definition = StyleDefinition {
                kind: StyleKind::Paragraph,
                based_on: None,
                additive: false,
                controls: Vec::new(),
            };
            for child in children {
                let Node::Control(word, number) = child else {
                    // Plain/escaped/Unicode text is the style name, not story
                    // content. Nested keycode/metadata destinations are also
                    // intentionally excluded from executable formatting.
                    continue;
                };
                if let Some(kind) = StyleKind::from_word(word) {
                    if declared {
                        return Err(RtfError::InvalidParameter("multiple style handles"));
                    }
                    id = style_handle(*number)?;
                    definition.kind = kind;
                    declared = true;
                } else if *word == "sbasedon" {
                    // RTF 1.9.1 assigns 222 to the no-base sentinel.
                    let base = style_handle(*number)?;
                    definition.based_on = (base != 222).then_some(base);
                } else if *word == "additive" {
                    definition.additive = *number != Some(0);
                } else if formatting_control(word) {
                    definition.controls.push(StyleControl {
                        word: (*word).to_owned(),
                        number: *number,
                    });
                }
            }
            if table.definitions.insert(id, Rc::new(definition)).is_some() {
                return Err(RtfError::InvalidParameter("duplicate style handle"));
            }
        }
        Ok(table)
    }

    /// Return base-first immutable definitions. A chain is walked only when used,
    /// so unused section/table styles in ordinary Word documents do not prevent
    /// importing their supported body content. No recursion or exponential merge.
    pub(super) fn resolve(
        &self,
        kind: StyleKind,
        id: i32,
        max_depth: usize,
    ) -> Result<Vec<Rc<StyleDefinition>>, RtfError> {
        if matches!(kind, StyleKind::Section | StyleKind::Table) {
            return Err(RtfError::UnsupportedFeature("section or table styles"));
        }
        let mut chain = Vec::new();
        let mut visited = BTreeSet::new();
        let mut current = Some(id);
        while let Some(id) = current {
            if !visited.insert(id) {
                return Err(RtfError::InvalidParameter("cyclic style inheritance"));
            }
            let Some(style) = self.definitions.get(&id) else {
                if id == 0 && kind == StyleKind::Paragraph {
                    break;
                }
                return Err(RtfError::InvalidParameter("style reference"));
            };
            if chain.len() >= max_depth {
                return Err(RtfError::LimitExceeded("style inheritance"));
            }
            if style.kind != kind {
                return Err(RtfError::InvalidParameter("style kind"));
            }
            if kind == StyleKind::Character
                && style
                    .controls
                    .iter()
                    .any(|control| paragraph_control(&control.word))
            {
                return Err(RtfError::UnsupportedFeature(
                    "paragraph properties in character styles",
                ));
            }
            chain.push(style.clone());
            current = style.based_on;
        }
        chain.reverse();
        Ok(chain)
    }
}

fn paragraph_control(word: &str) -> bool {
    matches!(
        word,
        "pard"
            | "ql"
            | "qr"
            | "qc"
            | "qj"
            | "ltrpar"
            | "rtlpar"
            | "fi"
            | "li"
            | "ri"
            | "sb"
            | "sa"
            | "tql"
            | "tqc"
            | "tqr"
            | "tqdec"
            | "tldot"
            | "tlhyph"
            | "tlul"
            | "tlth"
            | "tleq"
            | "tlmdot"
            | "tx"
            | "tb"
            | "sl"
            | "slmult"
    )
}

fn formatting_control(word: &str) -> bool {
    paragraph_control(word)
        || matches!(
            word,
            "plain"
                | "lang"
                | "b"
                | "i"
                | "ul"
                | "ulnone"
                | "strike"
                | "f"
                | "fs"
                | "cf"
                | "highlight"
                | "cb"
                | "super"
                | "sub"
                | "nosupersub"
                | "charscalex"
                | "expnd"
                | "expndtw"
        )
}
