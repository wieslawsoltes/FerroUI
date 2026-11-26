use super::{rust_body, MacroSyntax, RustFilePath};
use crate::{ProjectError, Result};
use proc_macro2::{TokenStream, TokenTree};
use syn::{visit::Visit, Expr, Macro};

/// Context for checking standard include macros in an authored Rust file.
pub struct RustIncludeSyntax(MacroSyntax);

impl RustIncludeSyntax {
    pub fn parse(source: &str) -> Result<Self> {
        let file = syn::parse_str::<syn::File>(rust_body(source))
            .map_err(|error| ProjectError(error.to_string()))?;
        Ok(Self(MacroSyntax::new(&file)))
    }

    /// The include must come from the captured file. The replacement range
    /// uses the same coordinates as the supplied macro tokens.
    pub fn include_bytes(&self, expression: &Macro) -> Option<RustFilePath> {
        self.0.include_operand(expression, "include_bytes", 0)
    }
}

/// Find static standard `include_bytes!` and `include_str!` operands, including
/// strings assembled with `concat!` and the Cargo manifest directory.
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
        if let Some(site) = self.macros.include_path(expression, self.offset) {
            self.paths.push(site);
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

    #[test]
    fn assembled_paths_preserve_the_anchor_comments_and_unchanged_terms() {
        let source = concat!(
            "\u{feff}const LOGO: &[u8] = include_bytes!(::std::concat!(::core::env!(\"CARGO_MANIFEST_DIR\"),\r\n",
            "    r#\"/Assets/\"#, /* path 🎨 */ concat!(\"Logo Ł\", \".png\"),\r\n",
            "));\r\n",
            "const CARD: &str = include_str!(concat!(\"view/\", \"Card.xaml\"));\r\n",
        );
        let paths = rust_file_includes(source).unwrap();
        assert_eq!(paths.len(), 2);
        let logo = &paths[0];
        assert_eq!(
            (&logo.path[..], logo.anchor),
            ("Assets/Logo Ł.png", RustPathAnchor::Package)
        );
        assert_eq!(
            logo.replacement(source, "Icons/New #.png"),
            "r#\"/Icons/\"#, /* path 🎨 */ concat!(\"New #\", \".png\""
        );
        assert_eq!(
            (&paths[1].path[..], paths[1].anchor),
            ("view/Card.xaml", RustPathAnchor::SourceFile)
        );
        assert_eq!(
            paths[1].replacement(source, "view/New.xaml"),
            "\"view/\", \"New.xaml\""
        );
        let xaml = super::super::RustXamlReferences::parse(source, false).unwrap();
        assert_eq!(xaml.paths, [paths[1].clone()]);
    }

    #[test]
    fn dynamic_or_shadowed_macro_paths_have_no_editable_range() {
        let source = r#"
            macro_rules! concat { ($a:literal, $b:literal) => { "elsewhere" }; }
            const SHADOWED: &[u8] = include_bytes!(concat!("Assets/", "logo.png"));
            const GENERATED: &[u8] = include_bytes!(::std::concat!(env!("OUT_DIR"), "/logo.png"));
            const COMPUTED: &[u8] = include_bytes!(::std::concat!("Assets/", FILE));
            const MISPLACED: &[u8] = include_bytes!(::std::concat!("x", env!("CARGO_MANIFEST_DIR"), "/logo.png"));
        "#;
        assert!(rust_file_includes(source).unwrap().is_empty());
    }
}
