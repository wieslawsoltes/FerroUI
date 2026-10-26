use std::collections::{BTreeMap, BTreeSet};

use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::{BuildDiagnostic, DiagnosticSeverity};

/// A static or dynamic translation use. `argument_count = None` means lookup, or an
/// argument slice whose length the caller cannot prove at build time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyUsage {
    pub key: Option<String>,
    pub generated_constant: Option<String>,
    pub argument_count: Option<usize>,
    pub file: String,
    pub line: Option<usize>,
}

impl KeyUsage {
    pub fn lookup(key: impl Into<String>) -> Self {
        Self {
            key: Some(key.into()),
            generated_constant: None,
            argument_count: None,
            file: String::new(),
            line: None,
        }
    }

    pub fn format(key: impl Into<String>, argument_count: usize) -> Self {
        Self {
            argument_count: Some(argument_count),
            ..Self::lookup(key)
        }
    }
}

/// Rust syntax analysis is opt-in and scoped to explicit receiver expressions.
/// This avoids claiming semantic type analysis that a build script cannot perform.
#[derive(Clone, Debug, Default)]
pub struct SourceAnalysisOptions {
    pub receivers: BTreeSet<String>,
    /// Inspect generated get/value/format/observe methods on selected receivers.
    pub generated_accessors: bool,
}

impl SourceAnalysisOptions {
    pub fn receiver(mut self, expression: impl Into<String>) -> Self {
        self.receivers.insert(expression.into());
        self
    }

    pub fn generated_accessors(mut self, enabled: bool) -> Self {
        self.generated_accessors = enabled;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct SourceAnalysis {
    pub usages: Vec<KeyUsage>,
    pub diagnostics: Vec<BuildDiagnostic>,
}

/// Inspect literal keys, file-local string constants, generated key constants, and
/// statically sized argument arrays for selected translation service receivers.
/// Macros are opaque except for a `vec![...]` used as the format argument list.
pub fn analyze_rust_source(
    name: &str,
    source: &str,
    options: &SourceAnalysisOptions,
) -> SourceAnalysis {
    let file = match syn::parse_file(source) {
        Ok(file) => file,
        Err(error) => {
            return SourceAnalysis {
                usages: Vec::new(),
                diagnostics: vec![BuildDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    code: "PTA005".into(),
                    message: format!("Could not parse Rust source: {error}"),
                    file: Some(name.into()),
                    line: Some(error.span().start().line),
                    key: None,
                    culture: None,
                }],
            };
        }
    };
    // Resolve only unambiguous file-level constants. Local scopes and shadowed
    // names require rustc's semantic model and remain dynamic here.
    let mut constants = BTreeMap::new();
    for item in &file.items {
        if let syn::Item::Const(value) = item {
            insert_constant(&mut constants, value);
        }
    }
    let mut visitor = UsageVisitor {
        file: name,
        options,
        constants,
        usages: Vec::new(),
        nested_module: 0,
        local_names: Vec::new(),
    };
    visitor.visit_file(&file);
    SourceAnalysis {
        usages: visitor.usages,
        diagnostics: Vec::new(),
    }
}

struct UsageVisitor<'a> {
    file: &'a str,
    options: &'a SourceAnalysisOptions,
    constants: BTreeMap<String, Option<String>>,
    usages: Vec<KeyUsage>,
    nested_module: usize,
    local_names: Vec<BTreeMap<String, Option<String>>>,
}

impl UsageVisitor<'_> {
    fn record_call(
        &mut self,
        method: &str,
        arguments: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
        line: usize,
    ) {
        if matches!(
            method,
            "get_string"
                | "get_string_for"
                | "format"
                | "format_for"
                | "observe"
                | "translate"
                | "t"
        ) {
            let Some(expression) = arguments.first() else {
                return;
            };
            let mut key = literal_string(expression);
            let mut generated_constant = None;
            if let syn::Expr::Path(path) = unwrap_expression(expression) {
                let parts = path
                    .path
                    .segments
                    .iter()
                    .map(|part| part.ident.unraw().to_string())
                    .collect::<Vec<_>>();
                if parts.len() >= 2 && parts[parts.len() - 2] == "ProTranslateKeys" {
                    generated_constant = parts.last().cloned();
                } else if parts.len() == 1 {
                    key = match self
                        .local_names
                        .iter()
                        .rev()
                        .find_map(|names| names.get(&parts[0]))
                    {
                        Some(value) => value.clone(),
                        None if self.nested_module == 0 => {
                            self.constants.get(&parts[0]).cloned().flatten()
                        }
                        None => None,
                    };
                }
            }
            self.usages.push(KeyUsage {
                key,
                generated_constant,
                argument_count: if matches!(
                    method,
                    "format" | "format_for" | "observe" | "translate"
                ) {
                    arguments
                        .iter()
                        .nth(1)
                        .and_then(array_length)
                        .filter(|count| *count != 0 || !matches!(method, "observe" | "translate"))
                } else {
                    None
                },
                file: self.file.into(),
                line: Some(expression.span().start().line),
            });
        } else if self.options.generated_accessors {
            let Some((prefix, name)) = method.split_once('_') else {
                return;
            };
            if !matches!(prefix, "get" | "value" | "format" | "observe") || name.is_empty() {
                return;
            }
            self.usages.push(KeyUsage {
                key: None,
                generated_constant: Some(name.to_ascii_uppercase()),
                argument_count: if matches!(prefix, "format" | "observe") {
                    arguments.first().and_then(array_length)
                } else {
                    None
                },
                file: self.file.into(),
                line: Some(line),
            });
        }
    }

    fn push_patterns<'a>(&mut self, patterns: impl IntoIterator<Item = &'a syn::Pat>) {
        let mut bindings = PatternBindings::default();
        for pattern in patterns {
            bindings.visit_pat(pattern);
        }
        self.local_names
            .push(bindings.0.into_iter().map(|name| (name, None)).collect());
    }

    fn push_signature(&mut self, signature: &syn::Signature) {
        self.push_patterns(
            signature
                .inputs
                .iter()
                .filter_map(|argument| match argument {
                    syn::FnArg::Typed(argument) => Some(argument.pat.as_ref()),
                    _ => None,
                }),
        );
        let scope = self
            .local_names
            .last_mut()
            .expect("signature scope was pushed");
        for parameter in &signature.generics.params {
            if let syn::GenericParam::Const(parameter) = parameter {
                scope.insert(parameter.ident.unraw().to_string(), None);
            }
        }
    }

    fn visit_condition(&mut self, expression: &syn::Expr) {
        match unwrap_expression(expression) {
            syn::Expr::Let(binding) => {
                self.visit_expr(&binding.expr);
                self.push_patterns([binding.pat.as_ref()]);
            }
            syn::Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
                self.visit_condition(&binary.left);
                self.visit_condition(&binary.right);
            }
            _ => self.visit_expr(expression),
        }
    }
}

impl<'ast> Visit<'ast> for UsageVisitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.push_signature(&item.sig);
        syn::visit::visit_item_fn(self, item);
        self.local_names.pop();
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.push_signature(&item.sig);
        syn::visit::visit_impl_item_fn(self, item);
        self.local_names.pop();
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.push_signature(&item.sig);
        syn::visit::visit_trait_item_fn(self, item);
        self.local_names.pop();
    }

    fn visit_expr_closure(&mut self, expression: &'ast syn::ExprClosure) {
        self.push_patterns(expression.inputs.iter());
        syn::visit::visit_expr_closure(self, expression);
        self.local_names.pop();
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.visit_expr(&expression.expr);
        self.push_patterns([expression.pat.as_ref()]);
        self.visit_block(&expression.body);
        self.local_names.pop();
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.push_patterns([&arm.pat]);
        syn::visit::visit_arm(self, arm);
        self.local_names.pop();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        let depth = self.local_names.len();
        self.visit_condition(&expression.cond);
        self.visit_block(&expression.then_branch);
        self.local_names.truncate(depth);
        if let Some((_, otherwise)) = &expression.else_branch {
            self.visit_expr(otherwise);
        }
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        let depth = self.local_names.len();
        self.visit_condition(&expression.cond);
        self.visit_block(&expression.body);
        self.local_names.truncate(depth);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let mut names = BTreeMap::new();
        for statement in &block.stmts {
            match statement {
                syn::Stmt::Item(syn::Item::Const(value)) => {
                    insert_constant(&mut names, value);
                }
                syn::Stmt::Item(syn::Item::Use(value)) => {
                    let mut imports = ImportBindings::default();
                    imports.visit_use_tree(&value.tree);
                    for name in imports.names {
                        names.insert(name, None);
                    }
                    if imports.glob {
                        for name in self
                            .constants
                            .keys()
                            .chain(self.local_names.iter().flat_map(|scope| scope.keys()))
                        {
                            names.insert(name.clone(), None);
                        }
                    }
                }
                syn::Stmt::Local(local) => {
                    let mut bindings = PatternBindings::default();
                    bindings.visit_pat(&local.pat);
                    for name in bindings.0 {
                        // Conservative for uses before the declaration too: this scanner
                        // must not accidentally validate a shadowed dynamic expression.
                        names.insert(name, None);
                    }
                }
                _ => {}
            }
        }
        self.local_names.push(names);
        syn::visit::visit_block(self, block);
        self.local_names.pop();
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        // Modules have their own lexical namespace, including when declared
        // inside a function. They do not inherit that function's block bindings.
        let outer_scopes = std::mem::take(&mut self.local_names);
        self.nested_module += 1;
        syn::visit::visit_item_mod(self, item);
        self.nested_module -= 1;
        self.local_names = outer_scopes;
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let selected = receiver_name(&call.receiver)
            .is_some_and(|receiver| self.options.receivers.contains(&receiver));
        if selected {
            self.record_call(
                &call.method.unraw().to_string(),
                &call.args,
                call.span().start().line,
            );
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = unwrap_expression(&call.func) {
            if path.qself.is_none() {
                let parts = path
                    .path
                    .segments
                    .iter()
                    .map(|part| part.ident.unraw().to_string())
                    .collect::<Vec<_>>();
                if let Some((method, owner)) = parts.split_last() {
                    if self.options.receivers.contains(&owner.join("::")) {
                        self.record_call(method, &call.args, call.span().start().line);
                    }
                }
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

fn insert_constant(names: &mut BTreeMap<String, Option<String>>, value: &syn::ItemConst) {
    let text =
        if value.attrs.iter().any(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        }) {
            None
        } else {
            literal_string(&value.expr)
        };
    names
        .entry(value.ident.unraw().to_string())
        .and_modify(|value| *value = None)
        .or_insert(text);
}

#[derive(Default)]
struct ImportBindings {
    names: Vec<String>,
    glob: bool,
}
impl<'ast> Visit<'ast> for ImportBindings {
    fn visit_use_name(&mut self, name: &'ast syn::UseName) {
        self.names.push(name.ident.unraw().to_string());
    }
    fn visit_use_rename(&mut self, name: &'ast syn::UseRename) {
        self.names.push(name.rename.unraw().to_string());
    }
    fn visit_use_glob(&mut self, _: &'ast syn::UseGlob) {
        self.glob = true;
    }
}

#[derive(Default)]
struct PatternBindings(Vec<String>);

impl<'ast> Visit<'ast> for PatternBindings {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.0.push(pattern.ident.unraw().to_string());
        syn::visit::visit_pat_ident(self, pattern);
    }
}

fn unwrap_expression(mut expression: &syn::Expr) -> &syn::Expr {
    loop {
        expression = match expression {
            syn::Expr::Paren(value) => &value.expr,
            syn::Expr::Group(value) => &value.expr,
            syn::Expr::Reference(value) => &value.expr,
            _ => return expression,
        };
    }
}

fn literal_string(expression: &syn::Expr) -> Option<String> {
    match unwrap_expression(expression) {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(value),
            ..
        }) => Some(value.value()),
        _ => None,
    }
}

fn receiver_name(expression: &syn::Expr) -> Option<String> {
    match unwrap_expression(expression) {
        syn::Expr::Path(path) => Some(
            path.path
                .segments
                .iter()
                .map(|p| p.ident.unraw().to_string())
                .collect::<Vec<_>>()
                .join("::"),
        ),
        syn::Expr::Field(field) => match &field.member {
            syn::Member::Named(name) => {
                Some(format!("{}.{}", receiver_name(&field.base)?, name.unraw()))
            }
            _ => None,
        },
        _ => None,
    }
}

fn array_length(expression: &syn::Expr) -> Option<usize> {
    match unwrap_expression(expression) {
        syn::Expr::Array(array) => Some(array.elems.len()),
        syn::Expr::Repeat(repeat) => match &*repeat.len {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(value),
                ..
            }) => value.base10_parse().ok(),
            _ => None,
        },
        syn::Expr::Macro(value) if value.mac.path.is_ident("vec") => {
            // Parse as an array expression: this supports both list and repetition forms.
            syn::parse_str::<syn::Expr>(&format!("[{}]", value.mac.tokens))
                .ok()
                .as_ref()
                .and_then(array_length)
        }
        _ => None,
    }
}
