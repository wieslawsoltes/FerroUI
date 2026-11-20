//! Source ranges for Rust literals whose XAML path has a known base.
mod build_script;
mod macros;
mod tokens;
pub use tokens::check_rust_rename_tokens;

use self::build_script::{BuildScript, Site};
use self::macros::MacroSyntax;
use crate::{ProjectError, Result};
use proc_macro2::Span;
use std::ops::Range;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Expr, ExprLit, Lit, LitStr, Macro, Stmt,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustPathAnchor {
    SourceFile,
    Package,
    XamlRoot,
    ResourceUri,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustXamlPath {
    pub range: Range<usize>,
    pub path: String,
    pub anchor: RustPathAnchor,
    /// A registration key can be checked against the actual file passed to `documents`.
    pub input: Option<(String, RustPathAnchor)>,
    prefix: String,
}

impl RustXamlPath {
    pub fn replacement(&self, source: &str, path: &str) -> String {
        let value = format!("{}{path}", self.prefix);
        if let Some((hashes, _)) = source[self.range.clone()]
            .strip_prefix('r')
            .and_then(|raw| raw.split_once('"'))
        {
            if !value.contains(&format!("\"{hashes}")) {
                return format!("r{hashes}\"{value}\"{hashes}");
            }
        }
        LitStr::new(&value, Span::call_site()).token().to_string()
    }
}

#[derive(Debug, Default)]
pub struct RustXamlReferences {
    pub paths: Vec<RustXamlPath>,
    /// Explicit documents passed to a Cargo compilation group.
    pub registrations: Vec<RustXamlRegistration>,
    /// File-backed candidates. The compiler's input index identifies which call sites it used.
    pub file_documents: Vec<RustXamlFileInput>,
    pub uses_file_documents: bool,
    /// Explicit document discovery roots authored on a Cargo build builder.
    pub xaml_roots: Vec<String>,
    pub discovers_xaml: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustXamlFileInput {
    pub callee: Range<usize>,
    pub site: RustXamlPath,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustXamlRegistration {
    pub module: String,
    pub key: String,
    pub input: String,
    pub anchor: RustPathAnchor,
    pub exported: bool,
}

impl RustXamlReferences {
    pub fn may_contain_paths(source: &str, cargo_build: bool) -> bool {
        cargo_build
            || [
                "include_str",
                "include_bytes",
                "CARGO_MANIFEST_DIR",
                "ferres",
                "resm",
            ]
            .iter()
            .any(|name| source.contains(name))
    }

    pub fn parse(source: &str, cargo_build: bool) -> Result<Self> {
        let body = rust_body(source);
        let file =
            syn::parse_str::<syn::File>(body).map_err(|error| ProjectError(error.to_string()))?;
        let mut collector = Collector {
            offset: source.len() - body.len(),
            cargo_build,
            build: cargo_build.then(|| BuildScript::new(&file)),
            macros: MacroSyntax::new(&file),
            references: Self::default(),
        };
        collector.visit_file(&file);
        collector
            .references
            .paths
            .sort_by_key(|site| (site.range.start, site.range.end));
        collector.references.paths.dedup();
        collector.references.xaml_roots.sort();
        collector.references.xaml_roots.dedup();
        Ok(collector.references)
    }
}

fn rust_body(source: &str) -> &str {
    let body = source.strip_prefix('\u{feff}').unwrap_or(source);
    match body.strip_prefix("#!") {
        Some(after) if !after.trim_start().starts_with('[') => {
            body.find('\n').map_or("", |end| &body[end..])
        }
        _ => body,
    }
}

struct Collector<'source> {
    offset: usize,
    cargo_build: bool,
    build: Option<BuildScript<'source>>,
    macros: MacroSyntax,
    references: RustXamlReferences,
}

impl Collector<'_> {
    fn included_file(&self, input: &Macro) -> Option<(String, RustPathAnchor)> {
        let (value, anchor, prefix) = self.macros.include_path(input)?;
        let value = value.value();
        Some((value.strip_prefix(prefix)?.to_owned(), anchor))
    }

    fn record(
        &mut self,
        literal: &LitStr,
        anchor: RustPathAnchor,
        prefix: &str,
        input: Option<(String, RustPathAnchor)>,
    ) {
        if let Some(site) = self.path(literal, anchor, prefix, input) {
            self.references.paths.push(site);
        }
    }

    fn path(
        &self,
        literal: &LitStr,
        anchor: RustPathAnchor,
        prefix: &str,
        input: Option<(String, RustPathAnchor)>,
    ) -> Option<RustXamlPath> {
        let value = literal.value();
        let path = value.strip_prefix(prefix)?;
        let filename = if anchor == RustPathAnchor::ResourceUri {
            path.split(['?', '#']).next().unwrap_or(path)
        } else {
            path
        };
        if !filename.to_ascii_lowercase().ends_with(".xaml") {
            return None;
        }
        let range = literal.span().byte_range();
        Some(RustXamlPath {
            range: self.offset + range.start..self.offset + range.end,
            path: path.to_owned(),
            anchor,
            input,
            prefix: prefix.to_owned(),
        })
    }
}

impl<'source> Visit<'source> for Collector<'source> {
    fn visit_attribute(&mut self, _attribute: &'source syn::Attribute) {}

    fn visit_lit_str(&mut self, literal: &'source LitStr) {
        let value = literal.value();
        if value.starts_with("ferres://") || value.starts_with("resm:") {
            self.record(literal, RustPathAnchor::ResourceUri, "", None);
        }
    }

    fn visit_macro(&mut self, expression: &'source Macro) {
        if self.cargo_build {
            let site = self
                .macros
                .file_input(expression)
                .and_then(|literal| self.path(&literal, RustPathAnchor::Package, "", None));
            if let Some(site) = site {
                let callee = expression.path.span().byte_range();
                self.references.file_documents.push(RustXamlFileInput {
                    callee: self.offset + callee.start..self.offset + callee.end,
                    site,
                });
                return;
            }
        }
        if let Some((literal, anchor, prefix)) = self
            .macros
            .include_path(expression)
            .or_else(|| self.macros.manifest_path(expression))
        {
            self.record(&literal, anchor, prefix, None);
        } else if self.cargo_build {
            if let Some(args) = self.macros.arguments(expression, "println") {
                let directive = args.first().and_then(string_literal);
                for prefix in ["cargo::rerun-if-changed=", "cargo:rerun-if-changed="] {
                    if let Some(literal) =
                        directive.filter(|value| value.value().starts_with(prefix))
                    {
                        match args.len() {
                            1 => self.record(literal, RustPathAnchor::Package, prefix, None),
                            2 if literal.value().strip_prefix(prefix) == Some("{}") => {
                                if let Some(path) = args.last().and_then(string_literal) {
                                    self.record(path, RustPathAnchor::Package, "", None);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn visit_expr_method_call(&mut self, call: &'source syn::ExprMethodCall) {
        if let Some(build) = &self.build {
            let entries = build.paths(call);
            for entry in entries {
                match entry {
                    Site::Path(literal, anchor) => self.record(literal, anchor, "", None),
                    Site::Root(literal) => self.references.xaml_roots.push(literal.value()),
                    Site::Discovery => self.references.discovers_xaml = true,
                    Site::FileDocuments => self.references.uses_file_documents = true,
                    Site::Key(literal, input) => {
                        if let Some(input) = self.included_file(input) {
                            self.record(literal, RustPathAnchor::XamlRoot, "", Some(input));
                        }
                    }
                    Site::Registration(module, key, input, exported) => {
                        if let Some((input, anchor)) = self.included_file(input) {
                            self.references.registrations.push(RustXamlRegistration {
                                module: module.value(),
                                key: key.value(),
                                input,
                                anchor,
                                exported,
                            });
                        }
                    }
                }
            }
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_block(&mut self, block: &'source syn::Block) {
        if let Some(build) = &mut self.build {
            build.enter();
        }
        for statement in &block.stmts {
            self.visit_stmt(statement);
            if let (Some(build), Stmt::Local(local)) = (&mut self.build, statement) {
                build.bind(local);
            }
        }
        if let Some(build) = &mut self.build {
            build.leave();
        }
    }
}

fn string_literal(expr: &Expr) -> Option<&LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(literal),
            ..
        }) => Some(literal),
        Expr::Paren(paren) => string_literal(&paren.expr),
        Expr::Group(group) => string_literal(&group.expr),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_byte_ranges_and_raw_string_delimiters_after_a_script_header() {
        let source = "\u{feff}#!/usr/bin/env rust-script\r\nconst A: &str = { /* 🎨 */ include_str!(r#\"../views/My Card.xaml\"#) };\r\n";
        let paths = RustXamlReferences::parse(source, false).unwrap().paths;
        let [site] = paths.as_slice() else {
            panic!("expected one include")
        };
        assert_eq!(&source[site.range.clone()], "r#\"../views/My Card.xaml\"#");
        assert_eq!(site.anchor, RustPathAnchor::SourceFile);
        assert_eq!(
            site.replacement(source, "../🎨/My Card.xaml"),
            "r#\"../🎨/My Card.xaml\"#"
        );
    }

    #[test]
    fn reads_manifest_includes_and_exact_resource_literals_without_treating_text_as_a_path() {
        let source = r#"
        ///ferres://Chart/Document.xaml
        fn view() {
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/View.xaml",));
            include_str!(concat!(env!("OUT_DIR"), "/View.xaml"));
            let _ = "View.xaml";
            let _ = "ferres://Chart/Document.xaml#root";
            let _ = "See ferres://Chart/Document.xaml";
        }
        "#;
        let paths = RustXamlReferences::parse(source, false).unwrap().paths;
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].anchor, RustPathAnchor::Package);
        assert_eq!(paths[0].path, "View.xaml");
        assert_eq!(
            paths[0].replacement(source, "Archive/View.xaml"),
            "\"/Archive/View.xaml\""
        );
        assert_eq!(paths[1].path, "ferres://Chart/Document.xaml#root");
    }

    #[test]
    fn recognizes_the_cargo_builder_and_the_include_that_owns_a_registered_key() {
        let source = r#"
        use ferroui_build::{Build, XamlGroup as Group};
        const VIEWS: &[(&str, &str)] = &[("Views/Card.xaml", include_str!("Xaml/Views/Card.xaml"))];
        fn main() {
            println!("cargo::rerun-if-changed={}", "Xaml/Views/Card.xaml");
            let mut build = Build::from_env().xaml_root("Xaml");
            build = build.input("Xaml/Views/Card.xaml");
            let views = VIEWS;
            build.compile_group(Group::new("views").documents(views).class_document("Views/Card.xaml")).run();
            other.input("Views/Card.xaml");
        }
        "#;
        let references = RustXamlReferences::parse(source, true).unwrap();
        assert_eq!(references.xaml_roots, ["Xaml"]);
        assert_eq!(
            references.registrations,
            [RustXamlRegistration {
                module: "views".into(),
                key: "Views/Card.xaml".into(),
                input: "Xaml/Views/Card.xaml".into(),
                anchor: RustPathAnchor::SourceFile,
                exported: true,
            }]
        );
        let paths = references.paths;
        assert_eq!(paths.len(), 5);
        assert_eq!(
            paths[0].input,
            Some(("Xaml/Views/Card.xaml".into(), RustPathAnchor::SourceFile))
        );
        assert_eq!(paths[1].anchor, RustPathAnchor::SourceFile);
        assert_eq!(paths[2].anchor, RustPathAnchor::Package);
        assert_eq!(paths[3].anchor, RustPathAnchor::Package);
        assert_eq!(paths[4].input, paths[0].input);
        assert_eq!(
            RustXamlReferences::parse(source, false)
                .unwrap()
                .paths
                .len(),
            1
        );
    }

    #[test]
    fn registrations_distinguish_exported_and_internal_compilation_groups() {
        let source = r#"
        use ferroui_build::{Build, XamlGroup};
        const MODULE: &str = "customer_views";
        fn main() {
            let _unused = XamlGroup::new("unused").documents(&[("Unused.xaml", include_str!("Physical.xaml"))]);
            let public = XamlGroup::new(MODULE).documents(&[("Published/Card.xaml", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Physical.xaml")))]);
            let internal = XamlGroup::new("offscreen").documents(&[("Internal.xaml", include_str!("Physical.xaml"))]).internal();
            Build::from_env().compile_group(public).compile_group(internal).run();
        }
        "#;
        let references = RustXamlReferences::parse(source, true).unwrap();
        assert_eq!(
            references.registrations,
            [
                RustXamlRegistration {
                    module: "offscreen".into(),
                    key: "Internal.xaml".into(),
                    input: "Physical.xaml".into(),
                    anchor: RustPathAnchor::SourceFile,
                    exported: false,
                },
                RustXamlRegistration {
                    module: "customer_views".into(),
                    key: "Published/Card.xaml".into(),
                    input: "Physical.xaml".into(),
                    anchor: RustPathAnchor::Package,
                    exported: true,
                },
            ]
        );
    }

    #[test]
    fn does_not_treat_a_shadowed_macro_as_a_builtin_include() {
        let source = r#"
        macro_rules! include_str { ($value:literal) => { $value } }
        fn view() {
            let _label = include_str!("Label.xaml");
            let _document = ::std::include_str!("View.xaml");
        }
        "#;
        let paths = RustXamlReferences::parse(source, false).unwrap().paths;
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "View.xaml");
    }
}
