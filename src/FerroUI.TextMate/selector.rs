use crate::{Error, Result};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectorPriority {
    Left,
    #[default]
    Normal,
    Right,
}

#[derive(Clone, Debug)]
enum Expr {
    Never,
    Path(Vec<String>),
    All(Vec<Expr>),
    Any(Vec<Expr>),
    Not(Box<Expr>),
}

/// TextMate scope selector: ancestry, alternatives, grouped expressions and exclusions.
#[derive(Clone, Debug)]
pub struct ScopeSelector {
    expression: Expr,
    pub priority: SelectorPriority,
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ScopeSpecificity {
    target_index: usize,
    target_depth: usize,
    parent_count: usize,
    parent_lengths: Vec<usize>,
}

impl ScopeSelector {
    pub fn parse(value: &str) -> Result<Self> {
        let mut selectors = Self::parse_many(value)?;
        if selectors.len() == 1 {
            return Ok(selectors.remove(0));
        }
        let priority = selectors
            .iter()
            .map(|s| s.priority)
            .min()
            .unwrap_or_default();
        Ok(Self {
            expression: Expr::Any(selectors.into_iter().map(|s| s.expression).collect()),
            priority,
        })
    }

    /// Each top-level comma alternative has its own injection priority.
    pub fn parse_many(value: &str) -> Result<Vec<Self>> {
        let mut parser = Parser {
            tokens: lex(value),
            pos: 0,
        };
        let mut result = Vec::new();
        while parser.peek().is_some() {
            let priority = match parser.peek() {
                Some("L:") => {
                    parser.pos += 1;
                    SelectorPriority::Left
                }
                Some("R:") => {
                    parser.pos += 1;
                    SelectorPriority::Right
                }
                _ => SelectorPriority::Normal,
            };
            if let Some(expression) = parser.alternative(false, 0)? {
                result.push(Self {
                    expression,
                    priority,
                });
            }
            if parser.peek() != Some(",") {
                break;
            }
            parser.pos += 1;
        }
        Ok(result)
    }

    pub fn matches<S: AsRef<str>>(&self, scopes: &[S]) -> bool {
        self.matches_with(scopes, matches_path)
    }

    /// Evaluate the selector with a custom predicate for each scope path.
    /// The predicate receives the whole path in ancestor-to-descendant order,
    /// including any wildcard, and an unchanged reference to `input`.
    pub fn matches_with<T: ?Sized>(
        &self,
        input: &T,
        mut matches_path: impl FnMut(&[String], &T) -> bool,
    ) -> bool {
        self.expression.matches_with(input, &mut matches_path)
    }

    pub(crate) fn specificity<S: AsRef<str>>(&self, scopes: &[S]) -> Option<ScopeSpecificity> {
        self.expression.rank(scopes)
    }

    pub(crate) fn path(&self) -> Option<&[String]> {
        match &self.expression {
            Expr::Path(names) => Some(names),
            _ => None,
        }
    }
}

impl Expr {
    fn matches_with<T: ?Sized>(
        &self,
        input: &T,
        matches_path: &mut impl FnMut(&[String], &T) -> bool,
    ) -> bool {
        match self {
            Self::Never => false,
            Self::Path(names) => matches_path(names, input),
            Self::All(items) => items
                .iter()
                .all(|item| item.matches_with(input, matches_path)),
            Self::Any(items) => items
                .iter()
                .any(|item| item.matches_with(input, matches_path)),
            Self::Not(item) => !item.matches_with(input, matches_path),
        }
    }

    fn rank<S: AsRef<str>>(&self, scopes: &[S]) -> Option<ScopeSpecificity> {
        match self {
            Self::Never => None,
            Self::Path(names) => {
                let (target, parents) = names.split_last()?;
                let index = scopes
                    .iter()
                    .rposition(|scope| matches_name(target, scope.as_ref()))?;
                let mut cursor = index;
                for name in parents.iter().rev() {
                    let found = (0..cursor)
                        .rev()
                        .find(|&i| matches_name(name, scopes[i].as_ref()))?;
                    cursor = found;
                }

                // Ancestor positions establish a match. Specificity then comes
                // from the depth of the selector and the shape of its path.
                Some(ScopeSpecificity {
                    target_index: index + 1,
                    target_depth: if target == "*" {
                        0
                    } else {
                        target.split('.').count()
                    },
                    parent_count: parents.len(),
                    parent_lengths: parent_scope_lengths(parents).collect(),
                })
            }
            Self::Any(items) => items.iter().filter_map(|item| item.rank(scopes)).max(),
            Self::All(items) => {
                let mut best = ScopeSpecificity::default();
                for item in items {
                    best = best.max(item.rank(scopes)?);
                }
                Some(best)
            }
            Self::Not(item) => item.rank(scopes).is_none().then(ScopeSpecificity::default),
        }
    }
}

fn matches_name(pattern: &str, scope: &str) -> bool {
    pattern == "*"
        || scope == pattern
        || scope
            .strip_prefix(pattern)
            .is_some_and(|rest| rest.starts_with('.'))
}

pub(crate) fn matches_path<S: AsRef<str>>(names: &[String], scopes: &[S]) -> bool {
    let mut scopes = scopes.iter();
    names
        .iter()
        .all(|name| scopes.any(|scope| matches_name(name, scope.as_ref())))
}

pub(crate) fn parent_scope_lengths(parents: &[String]) -> impl Iterator<Item = usize> + '_ {
    parents.iter().rev().map(|name| name.encode_utf16().count())
}

fn lex(source: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut word_start = None;
    for (index, character) in source.char_indices() {
        let operator = matches!(character, '(' | ')' | ',' | '|')
            || (character == '-' && word_start.is_none());
        if operator || character.is_whitespace() {
            if let Some(start) = word_start.take() {
                tokens.push(&source[start..index]);
            }
            if operator {
                tokens.push(&source[index..index + character.len_utf8()]);
            }
        } else {
            let start = *word_start.get_or_insert(index);
            if character == ':' && matches!(&source[start..index], "L" | "R") {
                tokens.push(&source[start..=index]);
                word_start = None;
            }
        }
    }
    if let Some(start) = word_start {
        tokens.push(&source[start..]);
    }
    tokens
}

struct Parser<'a> {
    tokens: Vec<&'a str>,
    pos: usize,
}
impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.pos).copied()
    }
    fn alternative(&mut self, in_group: bool, depth: usize) -> Result<Option<Expr>> {
        let is_separator = |token| token == Some("|") || (in_group && token == Some(","));
        let Some(first) = self.conjunction(depth)? else {
            return Ok(None);
        };
        if !is_separator(self.peek()) {
            return Ok(Some(first));
        }
        let mut alternatives = vec![first];
        while is_separator(self.peek()) {
            while is_separator(self.peek()) {
                self.pos += 1;
            }
            let Some(item) = self.conjunction(depth)? else {
                break;
            };
            alternatives.push(item);
        }
        Ok(Some(if alternatives.len() == 1 {
            alternatives.remove(0)
        } else {
            Expr::Any(alternatives)
        }))
    }
    fn conjunction(&mut self, depth: usize) -> Result<Option<Expr>> {
        let mut items = Vec::new();
        while let Some(item) = self.operand(depth)? {
            items.push(item);
        }
        Ok(match items.len() {
            0 => None,
            1 => items.pop(),
            _ => Some(Expr::All(items)),
        })
    }
    fn operand(&mut self, depth: usize) -> Result<Option<Expr>> {
        const MAX_NESTING: usize = 128;
        if depth > MAX_NESTING {
            return Err(Error::InvalidSelector(
                "selector nesting exceeds 128".into(),
            ));
        }
        Ok(match self.peek() {
            Some("-") => {
                self.pos += 1;
                Some(match self.operand(depth + 1)? {
                    Some(operand) => Expr::Not(Box::new(operand)),
                    None => Expr::Never,
                })
            }
            Some("(") => {
                self.pos += 1;
                let expr = self.alternative(true, depth + 1)?;
                if self.peek() == Some(")") {
                    self.pos += 1;
                }
                expr
            }
            Some("," | "|" | ")") | None => None,
            Some(_) => {
                let mut names = Vec::new();
                while let Some(name) = self.peek() {
                    if matches!(name, "-" | "(" | ")" | "," | "|") {
                        break;
                    }
                    names.push(name.to_owned());
                    self.pos += 1;
                }
                Some(Expr::Path(names))
            }
        })
    }
}
