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
    let mut end = span.start;
    for token in stream {
        let range = token.span().byte_range();
        if range.end <= span.start || range.start >= span.end {
            continue;
        }
        if range.start >= end && !source[end..range.start].trim().is_empty() {
            return true;
        }
        if let TokenTree::Group(group) = token {
            let inner = SourceSpan::new(
                group.span_open().byte_range().end,
                group.span_close().byte_range().start,
            );
            if contains_trivia(source, group.stream(), inner) {
                return true;
            }
        }
        end = end.max(range.end);
    }
    end < span.end && !source[end..span.end].trim().is_empty()
}
