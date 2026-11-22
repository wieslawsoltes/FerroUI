//! [`ferro_idents!`]: joins identifiers inside declarative macros.
//!
//! `macro_rules!` cannot make a new identifier on stable Rust, and the classes, the markup
//! accessors and the reactive properties of FerroUI need names such as `ButtonVTable`,
//! `parent_measure` or `set_title`. The macro takes any tokens and replaces each `[< ... >]`
//! by one identifier, made of the identifiers inside:
//!
//! ```
//! macro_rules! accessor {
//!     ($field:ident) => {
//!         ferroui_ident_macros::ferro_idents! {
//!             fn [<set_ $field>]() -> &'static str {
//!                 stringify!([<$field:camel>])
//!             }
//!         }
//!     };
//! }
//!
//! accessor!(search_term);
//! assert_eq!(set_search_term(), "SearchTerm");
//! ```
//!
//! It has the bracket syntax of the `paste` crate (MIT OR Apache-2.0), which is no longer
//! maintained (RUSTSEC-2024-0436), for the forms FerroUI uses, so that the expansion of
//! every macro stays the same:
//!
//! - the parts are identifiers, a leading `r#` is dropped; other tokens are an error;
//! - `:camel` after a part turns `snake_case` into `PascalCase`;
//! - the new identifier has the span of the brackets;
//! - the invisible group of a macro fragment is removed where it holds one identifier,
//!   path, literal or lifetime (not inside an attribute), and next to a `::`.
//!
//! The crate has no dependencies.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

/// Replaces every `[< ... >]` in the tokens by the identifier it spells (see the crate
/// documentation).
#[proc_macro]
pub fn ferro_idents(input: TokenStream) -> TokenStream {
    match rewrite(input.clone(), true) {
        Ok(Some(output)) => output,
        Ok(None) => input,
        Err(failure) => failure.into_tokens(),
    }
}

/// A message for `compile_error!` and where it applies.
struct Failure {
    message: String,
    span: Span,
}

impl Failure {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self { message: message.into(), span }
    }

    /// `::core::compile_error! { "message" }`, all of it at the span of the failure.
    fn into_tokens(self) -> TokenStream {
        let span = self.span;
        let spanned = |mut token: TokenTree| {
            token.set_span(span);
            token
        };
        let mut message = Literal::string(&self.message);
        message.set_span(span);
        let body = TokenStream::from(TokenTree::Literal(message));
        [
            TokenTree::Punct(Punct::new(':', Spacing::Joint)),
            TokenTree::Punct(Punct::new(':', Spacing::Alone)),
            TokenTree::Ident(Ident::new("core", span)),
            TokenTree::Punct(Punct::new(':', Spacing::Joint)),
            TokenTree::Punct(Punct::new(':', Spacing::Alone)),
            TokenTree::Ident(Ident::new("compile_error", span)),
            TokenTree::Punct(Punct::new('!', Spacing::Alone)),
            TokenTree::Group(Group::new(Delimiter::Brace, body)),
        ]
        .into_iter()
        .map(spanned)
        .collect()
    }
}

/// The tokens with their joins done, or `None` when nothing in them changes (the caller then
/// keeps the original tokens, with their spans). `flatten_plain` is off inside attributes.
fn rewrite(input: TokenStream, flatten_plain: bool) -> Result<Option<TokenStream>, Failure> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let mut output = Vec::with_capacity(tokens.len());
    let mut changed = false;

    for (index, token) in tokens.iter().enumerate() {
        let TokenTree::Group(group) = token else {
            output.push(token.clone());
            continue;
        };
        let delimiter = group.delimiter();
        let span = group.span();
        let content = group.stream();

        if delimiter == Delimiter::Bracket && is_join(&content) {
            output.push(TokenTree::Ident(join(content, span)?));
            changed = true;
        } else if delimiter == Delimiter::None && flatten_plain && is_plain_fragment(&content) {
            output.extend(content);
            changed = true;
        } else {
            let in_attribute = delimiter == Delimiter::Bracket && follows_pound(&tokens, index);
            let rebuilt = match rewrite(content, flatten_plain && !in_attribute)? {
                Some(stream) => {
                    changed = true;
                    let mut rebuilt = Group::new(delimiter, stream);
                    rebuilt.set_span(span);
                    rebuilt
                }
                None => group.clone(),
            };
            if delimiter == Delimiter::None && touches_path_separator(&tokens, index) {
                output.extend(rebuilt.stream());
                changed = true;
            } else {
                output.push(TokenTree::Group(rebuilt));
            }
        }
    }

    Ok(changed.then(|| output.into_iter().collect()))
}

fn is_punct(token: &TokenTree, ch: char) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == ch)
}

fn is_joint_colon(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == ':' && punct.spacing() == Spacing::Joint)
}

/// Whether the group at `index` is the bracket of `#[..]` or `#![..]`.
fn follows_pound(tokens: &[TokenTree], index: usize) -> bool {
    match index {
        0 => false,
        1 => is_punct(&tokens[0], '#'),
        _ => {
            is_punct(&tokens[index - 1], '#')
                || (is_punct(&tokens[index - 1], '!') && is_punct(&tokens[index - 2], '#'))
        }
    }
}

/// Whether a `::` stands directly before or after the token at `index`.
fn touches_path_separator(tokens: &[TokenTree], index: usize) -> bool {
    let before = index >= 2 && is_joint_colon(&tokens[index - 2]) && is_punct(&tokens[index - 1], ':');
    let after = index + 2 < tokens.len() && is_joint_colon(&tokens[index + 1]) && is_punct(&tokens[index + 2], ':');
    before || after
}

/// Whether the content of a bracket is `< ... >`, with at least one token and no other `>`.
fn is_join(content: &TokenStream) -> bool {
    let tokens: Vec<TokenTree> = content.clone().into_iter().collect();
    match tokens.as_slice() {
        [open, middle @ .., close] if is_punct(open, '<') && is_punct(close, '>') => {
            !middle.is_empty() && !middle.iter().any(|token| is_punct(token, '>'))
        }
        _ => false,
    }
}

/// Whether the content of an invisible group is one literal, one lifetime or one path of
/// plain identifiers (`a`, `a::b`).
fn is_plain_fragment(content: &TokenStream) -> bool {
    let tokens: Vec<TokenTree> = content.clone().into_iter().collect();
    match tokens.as_slice() {
        [TokenTree::Literal(_)] => true,
        [apostrophe, TokenTree::Ident(_)] if is_punct(apostrophe, '\'') => true,
        [TokenTree::Ident(_), rest @ ..] if rest.len() % 3 == 0 => rest.chunks_exact(3).all(|step| {
            is_joint_colon(&step[0])
                && matches!(&step[1], TokenTree::Punct(p) if p.as_char() == ':' && p.spacing() == Spacing::Alone)
                && matches!(&step[2], TokenTree::Ident(_))
        }),
        _ => false,
    }
}

/// The identifier spelled by the content of `[< ... >]`, at the span of the brackets.
fn join(content: TokenStream, span: Span) -> Result<Ident, Failure> {
    let tokens: Vec<TokenTree> = content.into_iter().collect();
    let mut parts = Vec::new();
    collect_parts(&tokens[1..tokens.len() - 1], &mut parts)?;
    let name = parts.concat();
    if !is_identifier(&name) {
        return Err(Failure::new(format!("`{name}` is not a valid identifier"), span));
    }
    Ok(Ident::new(&name, span))
}

/// The text of the parts of a join, with the modifiers applied.
fn collect_parts(tokens: &[TokenTree], parts: &mut Vec<String>) -> Result<(), Failure> {
    let mut rest = tokens;
    while let Some((token, tail)) = rest.split_first() {
        rest = tail;
        match token {
            TokenTree::Ident(ident) => {
                let text = ident.to_string();
                parts.push(text.strip_prefix("r#").unwrap_or(&text).to_owned());
            }
            TokenTree::Group(group) if group.delimiter() == Delimiter::None => {
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                collect_parts(&inner, parts)?;
            }
            TokenTree::Punct(punct) if punct.as_char() == ':' => {
                let Some((TokenTree::Ident(modifier), tail)) = rest.split_first() else {
                    return Err(Failure::new("expected a modifier after `:`", punct.span()));
                };
                rest = tail;
                if modifier.to_string() != "camel" {
                    return Err(Failure::new("the only modifier is `:camel`", modifier.span()));
                }
                let Some(last) = parts.last_mut() else {
                    return Err(Failure::new("`:camel` needs a part before it", punct.span()));
                };
                *last = to_camel_case(last);
            }
            other => {
                return Err(Failure::new("`[< ... >]` joins identifiers only", other.span()));
            }
        }
    }
    Ok(())
}

/// `search_term` to `SearchTerm`: a letter after `_` (or first) is upper case, a letter after
/// an upper case one is lower case, the underscores go.
fn to_camel_case(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut previous = '_';
    for ch in text.chars() {
        if ch != '_' {
            if previous == '_' {
                result.extend(ch.to_uppercase());
            } else if previous.is_uppercase() {
                result.extend(ch.to_lowercase());
            } else {
                result.push(ch);
            }
        }
        previous = ch;
    }
    result
}

/// A cheap check before `Ident::new`, which panics on what is not an identifier.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|first| first == '_' || first.is_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::{is_identifier, to_camel_case};

    #[test]
    fn camel_case_of_field_names() {
        assert_eq!(to_camel_case("search_term"), "SearchTerm");
        assert_eq!(to_camel_case("page"), "Page");
        assert_eq!(to_camel_case("x2_y"), "X2Y");
        assert_eq!(to_camel_case("_leading"), "Leading");
        assert_eq!(to_camel_case("trailing_"), "Trailing");
        assert_eq!(to_camel_case("two__gaps"), "TwoGaps");
        assert_eq!(to_camel_case("already_Upper"), "AlreadyUpper");
        assert_eq!(to_camel_case("HTTP_url"), "HttpUrl");
    }

    #[test]
    fn identifiers() {
        assert!(is_identifier("set_title"));
        assert!(is_identifier("_private"));
        assert!(!is_identifier(""));
        assert!(!is_identifier("1st"));
        assert!(!is_identifier("a-b"));
    }
}
