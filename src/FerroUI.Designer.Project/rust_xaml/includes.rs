use super::{MacroSyntax, RustFilePath, rust_body};
use crate::{ProjectError, Result};
use proc_macro2::{TokenStream, TokenTree};
use syn::{Expr, Macro, visit::Visit};

/// Find literal standard `include_bytes!` and `include_str!` operands. RSX macro
/// bodies are searched for embedded Rust expressions using the same include parser.
pub fn rust_file_includes(source: &str) -> Result<Vec<RustFilePath>> {
    let body = rust_body(source);
    let syntax =
        syn::parse_str::<syn::File>(body).map_err(|error| ProjectError(error.to_string()))?;
    let mut collector = Includes {
        offset: source.len() - body.len(),
        macros: MacroSyntax::new(&syntax),
        paths: Vec::new(),
    };
    collector.visit_file(&syntax);
    collector
        .paths
        .sort_by_key(|site| (site.range.start, site.range.end));
    collector.paths.dedup();
    Ok(collector.paths)
}

struct Includes {
    offset: usize,
    macros: MacroSyntax,
    paths: Vec<RustFilePath>,
}

impl Includes {
    fn embedded_expressions(&mut self, tokens: TokenStream, depth: usize) {
        if depth > 96 {
            return;
        }
        if let Ok(expression) = syn::parse2::<Expr>(tokens.clone()) {
            self.visit_expr(&expression);
            return;
        }
        for token in tokens {
            if let TokenTree::Group(group) = token {
                self.embedded_expressions(group.stream(), depth + 1);
            }
        }
    }
}

impl<'ast> Visit<'ast> for Includes {
    fn visit_macro(&mut self, expression: &'ast Macro) {
        if let Some((literal, anchor, prefix)) = self.macros.include_path(expression) {
            if let Some(site) = RustFilePath::from_literal(&literal, anchor, prefix, self.offset) {
                self.paths.push(site);
            }
        } else if expression
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "rsx")
        {
            self.embedded_expressions(expression.tokens.clone(), 0);
        }
    }

    fn visit_attribute(&mut self, _attribute: &'ast syn::Attribute) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RustPathAnchor;

    #[test]
    fn file_includes_map_complete_rust_and_nested_rsx_expression_literals() {
        let source = concat!(
            "\u{feff}const FONT: &[u8] = include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/Assets/Font.otf\"));\r\n",
            "fn image() { rsx! { StackPanel { spacing: 4.0, Image { source: ::ferroui_view::embedded_image(\r\n",
            "    /* art 🎨 */ ::core::include_bytes!(r#\"../Assets/Logo Ł.png\"#)) } TextBlock { text: \"Logo Ł.png\" } } }; }\r\n",
            "fn other() { custom::include_bytes!(\"../Assets/Logo Ł.png\"); }",
        );
        let paths = rust_file_includes(source).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(
            (&paths[0].path[..], paths[0].anchor),
            ("Assets/Font.otf", RustPathAnchor::Package)
        );
        assert_eq!(
            (&paths[1].path[..], paths[1].anchor),
            ("../Assets/Logo Ł.png", RustPathAnchor::SourceFile)
        );
        assert_eq!(
            &source[paths[1].range.clone()],
            "r#\"../Assets/Logo Ł.png\"#"
        );
        assert_eq!(
            paths[1].replacement(source, "../Icons/Logo.png"),
            "r#\"../Icons/Logo.png\"#"
        );
    }
}
