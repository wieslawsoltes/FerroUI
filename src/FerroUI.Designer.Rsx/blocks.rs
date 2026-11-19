use ferroui_designer_core::SourceSpan;
use proc_macro2::{Group, TokenStream, TokenTree};

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
