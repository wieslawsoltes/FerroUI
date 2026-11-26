//! Describe the compiled shape of a source file. The RSX parser is the authority
//! for which literal indices the macro actually reads through `hot::lit`.
use ferroui_view_syntax::{Attribute, Node};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::{collections::BTreeSet, fmt::Write};
use syn::spanned::Spanned;

type Position = (usize, usize);

pub(super) struct Source {
    pub(super) rsx: Vec<Invocation>,
    pub(super) signature: String,
    revision: u64,
}

pub(super) struct Invocation {
    pub(super) line: u32,
    pub(super) column: u32,
    pub(super) literals: Vec<String>,
    pub(super) shape: String,
    elements: Vec<Position>,
}

pub(super) struct Locations {
    revision: u64,
    rsx: Vec<(Position, Vec<Position>)>,
}

impl Locations {
    pub(super) fn resolve(
        &self,
        built: &Source,
        compiled: crate::RsxSourceLocation,
    ) -> Option<crate::RsxSourceLocation> {
        if compiled.revision != Some(built.revision) {
            return None;
        }
        let index = compiled.index as usize;
        let invocation = built.rsx.iter().position(|invocation| {
            invocation.position() == compiled.invocation
                && invocation.elements.get(index) == Some(&compiled.element)
        })?;
        let (invocation, elements) = self.rsx.get(invocation)?;
        Some(crate::RsxSourceLocation {
            element: *elements.get(index)?,
            invocation: *invocation,
            revision: Some(self.revision),
            ..compiled
        })
    }
}

impl Invocation {
    fn position(&self) -> Position {
        (self.line as usize, self.column as usize)
    }
}

impl Source {
    pub(super) fn parse(text: &str, managed: bool) -> Result<Self, String> {
        let tokens = text
            .parse::<TokenStream>()
            .map_err(|error| error.to_string())?;
        let mut source = Self {
            rsx: Vec::new(),
            signature: String::new(),
            revision: ferroui_reload_protocol::fingerprint(text.as_bytes()),
        };
        source.signature = source.describe_file(tokens, managed)?;
        Ok(source)
    }

    pub(super) fn into_locations(self) -> Locations {
        Locations {
            revision: self.revision,
            rsx: self
                .rsx
                .into_iter()
                .map(|invocation| (invocation.position(), invocation.elements))
                .collect(),
        }
    }

    fn describe_file(&mut self, tokens: TokenStream, managed: bool) -> Result<String, String> {
        let tokens: Vec<_> = tokens.into_iter().collect();
        let mut signature = String::new();
        for (at, token) in tokens.iter().enumerate() {
            let TokenTree::Group(group) = token else {
                if managed {
                    describe_token(&mut signature, token);
                }
                continue;
            };
            let (open, close) = delimiters(group.delimiter());
            let is_rsx = at >= 2
                && matches!(&tokens[at - 1], TokenTree::Punct(p) if p.as_char() == '!')
                && matches!(&tokens[at - 2], TokenTree::Ident(ident) if ident == "rsx")
                && group.delimiter() != Delimiter::None;
            if is_rsx {
                let location = tokens[path_start(&tokens, at - 2)].span().start();
                let mut literals = Vec::new();
                let mut shape = String::new();
                let mut hot = BTreeSet::new();
                let mut elements = Vec::new();
                if managed {
                    let nodes = ferroui_view_syntax::parse(group.stream())
                        .map_err(|error| error.to_string())?;
                    authored_nodes(&nodes, &mut hot, &mut elements);
                }
                describe_invocation(
                    group.stream(),
                    &mut literals,
                    &mut shape,
                    managed.then_some(&hot),
                );
                if managed {
                    let _ = write!(signature, "{open}{shape}{close}");
                }
                self.rsx.push(Invocation {
                    line: location.line as u32,
                    column: location.column as u32 + 1,
                    literals,
                    shape,
                    elements,
                });
            }
            let contents = self.describe_file(group.stream(), managed)?;
            if managed && !is_rsx {
                let _ = write!(signature, "{open}{contents}{close}");
            }
        }
        Ok(signature)
    }
}

fn authored_nodes(nodes: &[Node], hot: &mut BTreeSet<usize>, elements: &mut Vec<Position>) {
    for node in nodes {
        match node {
            Node::Text(_, index) => {
                hot.insert(*index as usize);
            }
            Node::Element(element) => {
                let position = element.path.span().start();
                elements.push((position.line, position.column + 1));
                for attribute in &element.attributes {
                    if let Attribute::Value {
                        hot: Some(index), ..
                    } = attribute
                    {
                        hot.insert(*index as usize);
                    }
                }
                authored_nodes(&element.children, hot, elements);
            }
            Node::If(_, then, otherwise, _) => {
                authored_nodes(then, hot, elements);
                if let Some(otherwise) = otherwise {
                    authored_nodes(otherwise, hot, elements);
                }
            }
            Node::For(_, _, body, _) => authored_nodes(body, hot, elements),
            Node::Expr(..) => (),
        }
    }
}

fn describe_invocation(
    tokens: TokenStream,
    literals: &mut Vec<String>,
    shape: &mut String,
    hot: Option<&BTreeSet<usize>>,
) {
    for token in tokens {
        match &token {
            TokenTree::Literal(literal) => {
                let text = literal.to_string();
                if hot.is_none() {
                    shape.push_str("$ ");
                } else if hot.is_some_and(|indices| indices.contains(&literals.len())) {
                    let _ = write!(shape, "hot:{} ", literal_kind(&text));
                } else {
                    describe_token(shape, &token);
                }
                literals.push(text);
            }
            TokenTree::Group(group) => {
                let (open, close) = delimiters(group.delimiter());
                shape.push(open);
                describe_invocation(group.stream(), literals, shape, hot);
                shape.push(close);
            }
            _ => describe_token(shape, &token),
        }
    }
}

fn literal_kind(text: &str) -> String {
    match syn::parse_str::<syn::Lit>(text) {
        Ok(syn::Lit::Int(value)) => format!("int:{}", value.suffix()),
        Ok(syn::Lit::Float(value)) => format!("float:{}", value.suffix()),
        Ok(syn::Lit::Str(_)) => "str".into(),
        Ok(syn::Lit::Char(_)) => "char".into(),
        _ => text.to_owned(),
    }
}

fn describe_token(out: &mut String, token: &TokenTree) {
    let text = token.to_string();
    let _ = write!(out, "{}:{text}", text.len());
    if let TokenTree::Punct(punct) = token {
        let _ = write!(out, "{:?}", punct.spacing());
    }
    out.push(' ');
}

fn delimiters(delimiter: Delimiter) -> (char, char) {
    match delimiter {
        Delimiter::Brace => ('{', '}'),
        Delimiter::Bracket => ('[', ']'),
        Delimiter::Parenthesis => ('(', ')'),
        Delimiter::None => ('\u{1e}', '\u{1f}'),
    }
}

fn path_start(tokens: &[TokenTree], mut at: usize) -> usize {
    while at >= 3
        && matches!(&tokens[at - 1], TokenTree::Punct(p) if p.as_char() == ':')
        && matches!(&tokens[at - 2], TokenTree::Punct(p) if p.as_char() == ':')
        && matches!(&tokens[at - 3], TokenTree::Ident(_))
    {
        at -= 3;
    }
    if at >= 2
        && matches!(&tokens[at - 1], TokenTree::Punct(p) if p.as_char() == ':')
        && matches!(&tokens[at - 2], TokenTree::Punct(p) if p.as_char() == ':')
    {
        return at - 2;
    }
    at
}

#[cfg(test)]
mod tests {
    use super::Source;

    #[test]
    fn distinguishes_hot_literals_from_compiled_expressions_and_structure() {
        let base =
            r#"fn view() { rsx! { Button { width: 10.0, on_click: |_| count(1), "Saved" } }; }"#;
        let signature = |text: &str| Source::parse(text, true).unwrap().signature;
        assert_eq!(
            signature(base),
            signature(&base.replace("10.0", "20.0").replace("Saved", "Done"))
        );
        for (old, new) in [
            ("count(1)", "count(2)"),
            ("Button", "TextBlock"),
            ("10.0", "10u32"),
            ("fn view", "fn other"),
        ] {
            assert_ne!(signature(base), signature(&base.replace(old, new)));
        }
        assert!(Source::parse(&base.replace("width: 10.0", "width:"), true).is_err());
    }

    #[test]
    fn maps_exact_compiler_anchors_inside_multiple_rsx_blocks() {
        let before = "fn first() { rsx! { TextBlock { text: \"first\" } }; }\r\n\
                      fn view() { ::ferroui_view::rsx! { StackPanel {\r\n\
                      if cond { Button { \"yes\" } } else { TextBox { text: \"no\" } }\r\n\
                      for row in rows { TextBlock { text: row.title } }\r\n\
                      } }; }";
        let after = before.replace("Button { \"yes\" }", "Button { \"👋\" }\r\n");
        let built = Source::parse(before, true).unwrap();
        let next = Source::parse(&after, true).unwrap();
        assert_eq!(built.signature, next.signature);
        let expected = next.rsx[1].elements[2];
        let locations = next.into_locations();
        let anchor = crate::RsxSourceLocation {
            element: built.rsx[1].elements[2],
            invocation: built.rsx[1].position(),
            index: 2,
            revision: Some(built.revision),
        };
        let mapped = locations.resolve(&built, anchor).unwrap();
        assert_eq!(mapped.element, expected);
        assert_eq!(mapped.element.0, anchor.element.0 + 1);
        assert_eq!(
            mapped.revision,
            Some(ferroui_reload_protocol::fingerprint(after.as_bytes()))
        );
        assert!(locations
            .resolve(
                &built,
                crate::RsxSourceLocation {
                    revision: Some(0),
                    ..anchor
                }
            )
            .is_none());
        assert!(locations
            .resolve(&built, crate::RsxSourceLocation { index: 1, ..anchor })
            .is_none());
    }
}
