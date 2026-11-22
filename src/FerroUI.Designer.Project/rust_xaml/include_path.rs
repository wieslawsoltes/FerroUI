use super::macros::take_literal;
use super::{rust_string_like, string_literal, MacroSyntax, RustFilePath, RustPathAnchor};
use std::ops::Range;
use syn::{Expr, LitStr, Macro};

const MAX_CONCAT_DEPTH: usize = 96;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Fragment {
    range: Range<usize>,
    value: String,
}

#[derive(Default)]
struct Concatenation {
    manifest: bool,
    literals: Vec<LitStr>,
}

impl MacroSyntax {
    pub(super) fn include_path(&self, expression: &Macro, offset: usize) -> Option<RustFilePath> {
        self.include_operand(expression, "include_str", offset)
            .or_else(|| self.include_operand(expression, "include_bytes", offset))
    }

    pub(super) fn include_operand(
        &self,
        expression: &Macro,
        name: &str,
        offset: usize,
    ) -> Option<RustFilePath> {
        let args = self.arguments(expression, name)?;
        if args.len() != 1 {
            return None;
        }
        self.static_string_path(args.first()?, offset)
    }

    pub(super) fn static_string_path(&self, argument: &Expr, offset: usize) -> Option<RustFilePath> {
        if let Some(literal) = string_literal(argument) {
            return RustFilePath::from_literal(literal, RustPathAnchor::SourceFile, "", offset);
        }
        let Expr::Macro(expression) = argument else {
            return None;
        };
        self.static_concat_path(&expression.mac, offset)
    }

    pub(super) fn static_concat_path(&self, expression: &Macro, offset: usize) -> Option<RustFilePath> {
        let mut path = Concatenation::default();
        self.concat_path(expression, &mut path, 0)?;
        path.into_path(offset)
    }

    fn concat_path(
        &self,
        expression: &Macro,
        path: &mut Concatenation,
        depth: usize,
    ) -> Option<()> {
        if depth == MAX_CONCAT_DEPTH {
            return None;
        }
        for argument in self.arguments(expression, "concat")? {
            let Expr::Macro(expression) = argument else {
                path.literals.push(take_literal(argument)?);
                continue;
            };
            if let Some(env) = self.arguments(&expression.mac, "env") {
                if path.manifest
                    || !path.literals.is_empty()
                    || env.len() != 1
                    || env.first().and_then(string_literal)?.value() != "CARGO_MANIFEST_DIR"
                {
                    return None;
                }
                path.manifest = true;
            } else {
                self.concat_path(&expression.mac, path, depth + 1)?;
            }
        }
        Some(())
    }
}

impl Concatenation {
    fn into_path(self, offset: usize) -> Option<RustFilePath> {
        let (anchor, prefix) = if self.manifest {
            (RustPathAnchor::Package, "/")
        } else {
            (RustPathAnchor::SourceFile, "")
        };
        if let [literal] = self.literals.as_slice() {
            return RustFilePath::from_literal(literal, anchor, prefix, offset);
        }
        let fragments = self
            .literals
            .iter()
            .map(|literal| {
                let range = literal.span().byte_range();
                Fragment {
                    range: offset + range.start..offset + range.end,
                    value: literal.value(),
                }
            })
            .collect::<Vec<_>>();
        let value: String = fragments
            .iter()
            .map(|fragment| fragment.value.as_str())
            .collect();
        Some(RustFilePath {
            range: fragments.first()?.range.start..fragments.last()?.range.end,
            path: value.strip_prefix(prefix)?.into(),
            anchor,
            input: None,
            shares_input_path: false,
            prefix: prefix.into(),
            fragments,
        })
    }
}

/// Retain unchanged terms and the text between literal tokens, including nested
/// calls and comments. Keep directory separators at existing term boundaries
/// when the new address has the same shape.
pub(super) fn replace_fragments(
    source: &str,
    range: Range<usize>,
    mut fragments: &[Fragment],
    mut value: &str,
) -> String {
    while let Some((first, rest)) = fragments.split_first() {
        let Some(suffix) = value.strip_prefix(&first.value) else {
            break;
        };
        fragments = rest;
        value = suffix;
    }
    while let Some((last, rest)) = fragments.split_last() {
        let Some(prefix) = value.strip_suffix(&last.value) else {
            break;
        };
        fragments = rest;
        value = prefix;
    }
    let mut edits = Vec::new();
    let mut terms = fragments.iter().peekable();
    while let Some(term) = terms.next() {
        let (text, remaining) = if terms.peek().is_some() {
            split_directory_term(&term.value, value)
        } else {
            (value, "")
        };
        edits.push((
            &term.range,
            rust_string_like(&source[term.range.clone()], text),
        ));
        value = remaining;
    }
    let mut result = source[range.clone()].to_owned();
    for (token, replacement) in edits.into_iter().rev() {
        result.replace_range(
            token.start - range.start..token.end - range.start,
            &replacement,
        );
    }
    result
}

fn split_directory_term<'a>(old: &str, value: &'a str) -> (&'a str, &'a str) {
    let end = old
        .ends_with('/')
        .then(|| old.bytes().filter(|byte| *byte == b'/').count());
    let split = end
        .and_then(|count| value.match_indices('/').nth(count - 1))
        .map_or(0, |(offset, _)| offset + 1);
    value.split_at(split)
}
