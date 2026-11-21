//! The grammar of `ferroui_view::rsx!` shared by macro expansion and source tools.

use proc_macro2::{extra::DelimSpan, Span, TokenStream};
use quote::ToTokens;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream, Parser};
use syn::{braced, token, Expr, Ident, LitStr, Pat, Path, Token};

/// Parses the body of an RSX invocation while retaining original token spans.
pub fn parse(tokens: TokenStream) -> syn::Result<Vec<Node>> {
    let counter = Counter::default();
    (|input: ParseStream| parse_nodes(input, &counter)).parse2(tokens)
}

/// Numbers the literal tokens of an `rsx!` in the order of the source: the
/// key of a literal for hot reload (`ferroui_view::hot`), which the runtime
/// finds again by counting the literal tokens of the macro in the file.
#[derive(Default)]
struct Counter(std::cell::Cell<u32>);

impl Counter {
    /// The number of the next literal, which this one takes.
    fn take(&self) -> u32 {
        let index = self.0.get();
        self.0.set(index + 1);
        index
    }

    /// Skips the literals of tokens that are not hot literals.
    fn skip(&self, tokens: TokenStream) {
        fn count(tokens: TokenStream) -> u32 {
            tokens
                .into_iter()
                .map(|token| match token {
                    proc_macro2::TokenTree::Literal(_) => 1,
                    proc_macro2::TokenTree::Group(group) => count(group.stream()),
                    _ => 0,
                })
                .sum()
        }
        self.0.set(self.0.get() + count(tokens));
    }
}

fn parse_nodes(input: ParseStream, counter: &Counter) -> syn::Result<Vec<Node>> {
    let mut nodes = Vec::new();
    while !input.is_empty() {
        nodes.push(parse_node(input, counter)?);
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }
    Ok(nodes)
}

/// A parsed child or root of an RSX tree. Literal indices match hot reload.
pub enum Node {
    Element(ElementNode),
    Text(LitStr, u32),
    Expr(Expr, Span),
    If(Box<Expr>, Vec<Node>, Option<Vec<Node>>, Span),
    For(Box<Pat>, Box<Expr>, Vec<Node>, Span),
}

/// A control or component invocation, with the braces retained for source edits.
pub struct ElementNode {
    pub path: Path,
    pub braces: DelimSpan,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
}

/// A builder call in a control or component body.
pub enum Attribute {
    /// `name: value` or `Owner.name: value`.
    Value {
        /// `Owner` of `Owner.name`.
        owner: Option<Ident>,
        name: Ident,
        value: Box<Expr>,
        /// The number of the literal when the value is one that hot reload
        /// can change.
        hot: Option<u32>,
    },
    /// `name(arguments)`: a method of the builder with several arguments.
    Call {
        name: Ident,
        arguments: syn::punctuated::Punctuated<Expr, Token![,]>,
    },
}

fn parse_children(input: ParseStream, counter: &Counter) -> syn::Result<Vec<Node>> {
    let content;
    braced!(content in input);
    parse_nodes(&content, counter)
}

/// Whether a value is a literal that hot reload can change: text, a number,
/// a character.
fn is_hot_literal(value: &Expr) -> bool {
    matches!(value, Expr::Lit(lit) if matches!(lit.lit, syn::Lit::Str(_) | syn::Lit::Int(_) | syn::Lit::Float(_) | syn::Lit::Char(_)))
}

fn parse_node(input: ParseStream, counter: &Counter) -> syn::Result<Node> {
    {
        if input.peek(LitStr) {
            let text: LitStr = input.parse()?;
            return Ok(Node::Text(text, counter.take()));
        }
        if input.peek(token::Brace) {
            let span = input.span();
            let content;
            braced!(content in input);
            let expr: Expr = content.parse()?;
            counter.skip(expr.to_token_stream());
            return Ok(Node::Expr(expr, span));
        }
        if input.peek(Token![if]) {
            let span = input.span();
            input.parse::<Token![if]>()?;
            let condition = Expr::parse_without_eager_brace(input)?;
            counter.skip(condition.to_token_stream());
            let then = parse_children(input, counter)?;
            let otherwise = if input.peek(Token![else]) {
                input.parse::<Token![else]>()?;
                if input.peek(Token![if]) {
                    Some(vec![parse_node(input, counter)?])
                } else {
                    Some(parse_children(input, counter)?)
                }
            } else {
                None
            };
            return Ok(Node::If(Box::new(condition), then, otherwise, span));
        }
        if input.peek(Token![for]) {
            let span = input.span();
            input.parse::<Token![for]>()?;
            let pattern = Pat::parse_single(input)?;
            counter.skip(pattern.to_token_stream());
            input.parse::<Token![in]>()?;
            let items = Expr::parse_without_eager_brace(input)?;
            counter.skip(items.to_token_stream());
            let body = parse_children(input, counter)?;
            return Ok(Node::For(Box::new(pattern), Box::new(items), body, span));
        }
        let path: Path = input.call(Path::parse_mod_style)?;
        let content;
        let braces = braced!(content in input).span;
        let mut attributes = Vec::new();
        let mut children = Vec::new();
        while !content.is_empty() {
            if content.peek(Ident::peek_any) && content.peek2(token::Paren) {
                let name = content.call(Ident::parse_any)?;
                let arguments;
                syn::parenthesized!(arguments in content);
                let arguments = arguments.parse_terminated(Expr::parse, Token![,])?;
                counter.skip(arguments.to_token_stream());
                attributes.push(Attribute::Call { name, arguments });
            } else if content.peek(Ident::peek_any)
                && (content.peek2(Token![,]) || is_last_token(&content))
            {
                // `name` alone: the setter of the name with the variable of
                // the name, as a field of a struct is given.
                let name = content.call(Ident::parse_any)?;
                let value: Expr = syn::parse_quote!(#name);
                attributes.push(Attribute::Value {
                    owner: None,
                    name,
                    value: Box::new(value),
                    hot: None,
                });
            } else if is_attribute(&content) {
                let owner = if content.peek2(Token![.]) {
                    let owner = content.call(Ident::parse_any)?;
                    content.parse::<Token![.]>()?;
                    Some(owner)
                } else {
                    None
                };
                let name = content.call(Ident::parse_any)?;
                content.parse::<Token![:]>()?;
                let value: Expr = content.parse()?;
                let hot = if is_hot_literal(&value) {
                    Some(counter.take())
                } else {
                    counter.skip(value.to_token_stream());
                    None
                };
                attributes.push(Attribute::Value {
                    owner,
                    name,
                    value: Box::new(value),
                    hot,
                });
            } else {
                children.push(parse_node(&content, counter)?);
            }
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(Node::Element(ElementNode {
            path,
            braces,
            attributes,
            children,
        }))
    }
}

/// Whether the next token is the last one of the stream.
fn is_last_token(input: ParseStream) -> bool {
    let fork = input.fork();
    fork.parse::<proc_macro2::TokenTree>().is_ok() && fork.is_empty()
}

/// `name: ...` or `Owner.name: ...`, as opposed to a child element
/// (`Name { ... }`, `module::Name { ... }`).
fn is_attribute(input: ParseStream) -> bool {
    let fork = input.fork();
    if fork.call(Ident::parse_any).is_err() {
        return false;
    }
    if fork.peek(Token![.]) {
        let _ = fork.parse::<Token![.]>();
        if fork.call(Ident::parse_any).is_err() {
            return false;
        }
    }
    fork.peek(Token![:]) && !fork.peek(Token![::])
}

/// `StackPanel` as `stack_panel`.
pub fn snake_case(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (index, &char) in chars.iter().enumerate() {
        if char.is_uppercase() {
            let previous_lower =
                index > 0 && (chars[index - 1].is_lowercase() || chars[index - 1].is_ascii_digit());
            let next_lower = chars.get(index + 1).is_some_and(|next| next.is_lowercase());
            let previous_upper = index > 0 && chars[index - 1].is_uppercase();
            if index > 0 && (previous_lower || (previous_upper && next_lower)) {
                out.push('_');
            }
            out.extend(char.to_lowercase());
        } else {
            out.push(char);
        }
    }
    out
}
