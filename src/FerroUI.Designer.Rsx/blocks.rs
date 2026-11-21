use ferroui_designer_core::SourceSpan;
use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use std::collections::BTreeMap;
use syn::{Expr, Macro, Stmt};

pub(crate) struct Block {
    pub(crate) span: SourceSpan,
    pub(crate) group: Group,
}

pub(crate) fn blocks(source: &str) -> Result<Vec<Block>, String> {
    let mut found = Vec::new();
    let tokens: TokenStream = source
        .parse()
        .map_err(|e| format!("Invalid Rust tokens: {e}"))?;
    scan(tokens, &mut found, 0);
    Ok(found)
}

fn scan(stream: TokenStream, found: &mut Vec<Block>, depth: usize) {
    if depth > super::tree::MAX_DEPTH {
        return;
    }
    let tokens: Vec<_> = stream.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        if let [TokenTree::Ident(name), TokenTree::Punct(bang), TokenTree::Group(group), ..] =
            &tokens[index..]
        {
            if name == "rsx" && bang.as_char() == '!' {
                found.push(Block {
                    span: SourceSpan::new(
                        name.span().byte_range().start,
                        group.span_close().byte_range().end,
                    ),
                    group: group.clone(),
                });
            }
        }
        if let TokenTree::Group(group) = token {
            scan(group.stream(), found, depth + 1);
        }
    }
}

pub(crate) fn contains_trivia(source: &str, stream: TokenStream, span: SourceSpan) -> bool {
    visit_comments(source, stream, span, &mut |_, _| true)
}

/// A literal macro expression, optionally enclosed in parentheses or a Rust
/// block whose sole statement is the macro returned from that block.
pub(crate) fn expression_macro(expression: &Expr) -> Option<&Macro> {
    let mut current = expression;
    for _ in 0..super::tree::MAX_DEPTH {
        match current {
            Expr::Macro(body) if body.attrs.is_empty() => return Some(&body.mac),
            Expr::Paren(group) if group.attrs.is_empty() => current = &group.expr,
            Expr::Block(body) if body.attrs.is_empty() && body.label.is_none() => {
                match body.block.stmts.as_slice() {
                    [Stmt::Expr(tail, None)] => current = tail,
                    [Stmt::Macro(tail)] if tail.semi_token.is_none() && tail.attrs.is_empty() => return Some(&tail.mac),
                    _ => return None,
                }
            }
            _ => return None,
        }
    }
    None
}

/// Delimiters of builder calls and wrapped Rust children in the original stream.
#[derive(Default)]
pub(crate) struct SourceDelimiters {
    pub(crate) calls: BTreeMap<usize, SourceSpan>,
    pub(crate) blocks: BTreeMap<usize, SourceSpan>,
}

impl SourceDelimiters {
    pub(crate) fn read(stream: TokenStream) -> Self {
        let mut result = Self::default();
        result.scan(stream, 0);
        result
    }

    fn scan(&mut self, stream: TokenStream, depth: usize) {
        if depth > super::tree::MAX_DEPTH {
            return;
        }
        let mut name = None;
        for token in stream {
            if let TokenTree::Group(group) = &token {
                let end = group.span_close().byte_range().end;
                match group.delimiter() {
                    Delimiter::Parenthesis => {
                        if let Some(start) = name {
                            self.calls.insert(start, SourceSpan::new(start, end));
                        }
                    }
                    Delimiter::Brace => {
                        let start = group.span_open().byte_range().start;
                        self.blocks.insert(start, SourceSpan::new(start, end));
                    }
                    _ => {}
                }
                self.scan(group.stream(), depth + 1);
            }
            name = match token {
                TokenTree::Ident(name) => Some(name.span().byte_range().start),
                _ => None,
            };
        }
    }
}

/// Preserve comments, including the newline that terminates a line comment.
pub(crate) fn keep_comments(source: &str) -> Result<String, String> {
    let tokens: TokenStream = source.parse().map_err(|_| "Invalid Rust method tokens")?;
    let mut kept = String::new();
    visit_comments(source, tokens, SourceSpan::new(0, source.len()), &mut |_, text| {
        kept.push_str(text);
        false
    });
    Ok(kept)
}

/// Start of each skipped comment gap. Callers can indent the leading trivia without
/// changing tokens, string contents or the interior of a block comment.
pub(crate) fn comment_starts(source: &str, stream: TokenStream) -> Vec<usize> {
    let mut starts = Vec::new();
    visit_comments(source, stream, SourceSpan::new(0, source.len()), &mut |offset, gap| {
        starts.push(offset + gap.len() - gap.trim_start().len());
        false
    });
    starts
}

fn visit_comments(
    source: &str,
    stream: TokenStream,
    span: SourceSpan,
    visit: &mut impl FnMut(usize, &str) -> bool,
) -> bool {
    let mut end = span.start;
    for token in stream {
        let range = token.span().byte_range();
        if range.end <= span.start || range.start >= span.end {
            continue;
        }
        if range.start >= end {
            let gap = &source[end..range.start];
            if !gap.trim().is_empty() && visit(end, gap) {
                return true;
            }
        }
        if let TokenTree::Group(group) = token {
            let inner = SourceSpan::new(
                group.span_open().byte_range().end,
                group.span_close().byte_range().start,
            );
            if visit_comments(source, group.stream(), inner, visit) {
                return true;
            }
        }
        end = end.max(range.end);
    }
    end < span.end && !source[end..span.end].trim().is_empty() && visit(end, &source[end..span.end])
}
