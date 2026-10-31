//! Parse JavaScript at QML syntax boundaries using the ECMAScript grammar.
use crate::{Diagnostic, Expr, JavaScript, ScriptForm, SourceSpan};
use swc_common::{BytePos, Spanned};
use swc_ecma_ast as js;
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};

fn parser(source: &str, start: usize, allow_return: bool) -> Result<Parser<Lexer<'_>>, Diagnostic> {
    let len = u32::try_from(source.len()).map_err(|_| {
        Diagnostic::new(
            source,
            SourceSpan::default(),
            "JavaScript source exceeds 4 GiB",
        )
    })?;
    let input = StringInput::new(&source[start..], BytePos(start as u32), BytePos(len));
    let syntax = EsSyntax {
        allow_return_outside_function: allow_return,
        ..Default::default()
    };
    Ok(Parser::new_from(Lexer::new(
        Syntax::Es(syntax),
        js::EsVersion::EsNext,
        input,
        None,
    )))
}

pub(super) fn span(span: swc_common::Span) -> SourceSpan {
    SourceSpan {
        start: span.lo.0 as usize,
        end: span.hi.0 as usize,
    }
}
pub(super) fn error(source: &str, error: swc_ecma_parser::error::Error) -> Diagnostic {
    Diagnostic::new(source, span(error.span()), error.kind().msg())
}
fn check_errors(source: &str, parser: &mut Parser<Lexer<'_>>) -> Result<(), Diagnostic> {
    // Classic QML scripts and inline bindings run with the module context
    // supplied by the host. The linker lowers import.meta for these contexts.
    if let Some(err) = parser.take_errors().into_iter().find(|e| {
        !matches!(
            e.kind(),
            swc_ecma_parser::error::SyntaxError::ImportMetaInScript
        )
    }) {
        return Err(error(source, err));
    }
    Ok(())
}
fn script(source: &str, range: SourceSpan, form: ScriptForm) -> JavaScript {
    JavaScript {
        property_read: None,
        executable: None,
        import_hook: None,
        references: Vec::new(),
        form,
        source: source[range.start..range.end].to_owned(),
        span: range,
        origin: String::new(),
        root: None,
        ids: Default::default(),
        imports: Default::default(),
    }
}
fn parsed_expr(source: &str, start: usize) -> Result<(SourceSpan, Box<js::Expr>), Diagnostic> {
    let mut p = parser(source, start, true)?;
    let expr = p.parse_expr().map_err(|e| error(source, e))?;
    check_errors(source, &mut p)?;
    Ok((span(expr.span()), expr))
}
fn property_read(expr: &js::Expr) -> Option<crate::PropertyRead> {
    fn unparen(mut expr: &js::Expr) -> &js::Expr {
        while let js::Expr::Paren(group) = expr {
            expr = &group.expr;
        }
        expr
    }
    let (receiver, property) = match unparen(expr) {
        js::Expr::Ident(name) => (None, name.sym.as_str()),
        js::Expr::Member(member) => {
            let js::MemberProp::Ident(property) = &member.prop else {
                return None;
            };
            let receiver = match unparen(&member.obj) {
                js::Expr::This(_) => "this",
                js::Expr::Ident(name) => name.sym.as_str(),
                _ => return None,
            };
            (Some(receiver), property.sym.as_str())
        }
        _ => return None,
    };
    let lexical = |name: &str| {
        name.chars()
            .next()
            .is_some_and(|c| c == '_' || c.is_lowercase())
    };
    if !lexical(property) || receiver.is_some_and(|r| !lexical(r)) {
        return None;
    }
    Some(crate::PropertyRead {
        receiver: receiver.map(str::to_owned),
        property: property.to_owned(),
    })
}
pub(super) fn expression(source: &str, start: usize) -> Result<(SourceSpan, Expr), Diagnostic> {
    let (range, expr) = parsed_expr(source, start)?;
    // Literals are initial values rather than removable bindings, including in
    // the external engine mode. All other expressions keep their exact text.
    let mut value = match expr.as_ref() {
        js::Expr::Lit(js::Lit::Null(_)) => Expr::Null,
        js::Expr::Lit(js::Lit::Bool(b)) => Expr::Bool(b.value),
        js::Expr::Lit(js::Lit::Num(n)) if n.value.is_finite() => Expr::Number(n.value),
        js::Expr::Lit(js::Lit::Str(s)) if s.value.as_str().is_some() => {
            Expr::String(s.value.as_str().unwrap().into())
        }
        js::Expr::Unary(u)
            if matches!(u.op, js::UnaryOp::Minus | js::UnaryOp::Plus)
                && matches!(u.arg.as_ref(), js::Expr::Lit(js::Lit::Num(n)) if n.value.is_finite()) =>
        {
            let js::Expr::Lit(js::Lit::Num(n)) = u.arg.as_ref() else {
                unreachable!()
            };
            Expr::Number(if u.op == js::UnaryOp::Minus {
                -n.value
            } else {
                n.value
            })
        }
        _ => Expr::JavaScript(Box::new(script(source, range, ScriptForm::Expression))),
    };
    if let Expr::JavaScript(js) = &mut value {
        js.property_read = property_read(&expr);
        let mut inspect = Inspect::default();
        expr.visit_with(&mut inspect);
        js.references = inspect.names.into_iter().collect();
    }
    Ok((range, value))
}
pub(super) fn block(
    source: &str,
    start: usize,
    form: ScriptForm,
) -> Result<(SourceSpan, JavaScript), Diagnostic> {
    let mut p = parser(source, start, true)?;
    let stmt = p.parse_stmt().map_err(|e| error(source, e))?;
    check_errors(source, &mut p)?;
    let range = span(stmt.span());
    if !matches!(stmt, js::Stmt::Block(_)) {
        return Err(Diagnostic::new(
            source,
            range,
            "expected a JavaScript block",
        ));
    }
    let body = SourceSpan {
        start: range.start + 1,
        end: range.end - 1,
    };
    let mut code = script(source, body, form);
    let mut inspect = Inspect::default();
    stmt.visit_with(&mut inspect);
    if form == ScriptForm::BindingBlock {
        code.executable = completion_block(source, body, &stmt, &inspect.identifiers);
    }
    code.references = inspect.names.into_iter().collect();
    Ok((range, code))
}
// ECMAScript eval computes completion records for blocks, branches, loops and
// try/finally. Convert only QML's additional function-level return into a
// labeled break carrying its completion value. This leaves nested function
// returns and the behavior of user catch/finally blocks untouched.
fn completion_block(
    source: &str,
    body: SourceSpan,
    stmt: &js::Stmt,
    identifiers: &std::collections::BTreeSet<String>,
) -> Option<String> {
    #[derive(Default)]
    struct Returns(Vec<(SourceSpan, Option<SourceSpan>)>);
    impl Visit for Returns {
        fn visit_return_stmt(&mut self, r: &js::ReturnStmt) {
            self.0
                .push((span(r.span), r.arg.as_ref().map(|a| span(a.span()))));
        }
        fn visit_arrow_expr(&mut self, _: &js::ArrowExpr) {}
        fn visit_function(&mut self, _: &js::Function) {}
        fn visit_class(&mut self, _: &js::Class) {}
        fn visit_getter_prop(&mut self, _: &js::GetterProp) {}
        fn visit_setter_prop(&mut self, _: &js::SetterProp) {}
    }
    let mut returns = Returns::default();
    stmt.visit_with(&mut returns);
    if returns.0.is_empty() {
        return None;
    }
    let mut label = "__ferroBindingExit".to_owned();
    while identifiers.contains(&label) {
        label.push('_');
    }
    let mut output = source[body.start..body.end].to_owned();
    for (range, value) in returns.0.into_iter().rev() {
        let value = value.map_or("void 0", |value| &source[value.start..value.end]);
        output.replace_range(
            range.start - body.start..range.end - body.start,
            &format!("{{({value}); break {label};}}"),
        );
    }
    Some(format!("{label}:{{\n{output}\n}}"))
}
pub(super) fn handler(
    source: &str,
    start: usize,
) -> Result<(SourceSpan, Vec<String>, JavaScript), Diagnostic> {
    let (range, expr) = parsed_expr(source, start)?;
    let mut code = script(source, range, ScriptForm::HandlerBody);
    let mut inspect = Inspect::default();
    expr.visit_with(&mut inspect);
    code.references = inspect.names.into_iter().collect();
    let count = match expr.as_ref() {
        js::Expr::Arrow(f) => Some(f.params.len()),
        js::Expr::Fn(f) => Some(f.function.params.len()),
        _ => None,
    };
    if let Some(count) = count {
        // Forward positionally. Destructuring, defaults, nested closures and
        // lexical capture are evaluated by JavaScript itself.
        let mut prefix = "$qmlArg".to_owned();
        while code.source.contains(&prefix) {
            prefix.push('_');
        }
        let parameters: Vec<_> = (0..count).map(|i| format!("{prefix}{i}")).collect();
        code.executable = Some(format!(
            "return ({}).call(this{}{});",
            code.source,
            if count > 0 { "," } else { "" },
            parameters.join(",")
        ));
        Ok((range, parameters, code))
    } else {
        Ok((range, Vec::new(), code))
    }
}

/// A static module dependency. The span covers the full quoted module specifier
/// in an ECMAScript module. Classic QML imports carry a namespace instead.
pub struct Dependency {
    pub specifier: String,
    pub namespace: Option<String>,
    pub span: Option<SourceSpan>,
}

#[derive(Default)]
struct Inspect {
    names: std::collections::BTreeSet<String>,
    imports: Vec<Dependency>,
    identifiers: std::collections::BTreeSet<String>,
    sites: Vec<(SourceSpan, bool)>,
    missing: Option<SourceSpan>,
}

impl Visit for Inspect {
    fn visit_ident(&mut self, name: &js::Ident) {
        let name = name.sym.as_str();
        self.identifiers.insert(name.to_owned());
        if name.chars().next().is_some_and(char::is_uppercase) {
            self.names.insert(name.to_owned());
        }
    }
    fn visit_member_expr(&mut self, e: &js::MemberExpr) {
        if let (js::Expr::Ident(ns), js::MemberProp::Ident(ty)) = (&*e.obj, &e.prop) {
            if [ns.sym.as_str(), ty.sym.as_str()]
                .iter()
                .all(|n| n.chars().next().is_some_and(char::is_uppercase))
            {
                self.names.insert(format!("{}.{}", ns.sym, ty.sym));
            }
        }
        e.visit_children_with(self);
    }
    fn visit_meta_prop_expr(&mut self, e: &js::MetaPropExpr) {
        if e.kind == js::MetaPropKind::ImportMeta {
            self.sites.push((span(e.span), true));
        }
    }
    fn visit_call_expr(&mut self, e: &js::CallExpr) {
        if let js::Callee::Import(import) = &e.callee {
            self.sites.push((span(import.span), false));
            if let Some(js::Expr::Lit(js::Lit::Str(s))) = e
                .args
                .first()
                .filter(|arg| arg.spread.is_none())
                .map(|arg| arg.expr.as_ref())
            {
                if let Some(path) = s.value.as_str() {
                    self.imports.push(Dependency {
                        specifier: path.into(),
                        namespace: None,
                        span: Some(span(s.span)),
                    });
                } else {
                    self.missing = Some(span(s.span));
                }
            } else {
                self.missing = Some(span(e.span));
            }
        }
        e.visit_children_with(self);
    }
}

impl Inspect {
    fn dependencies(self, source: &str) -> Result<Vec<Dependency>, Diagnostic> {
        if let Some(span) = self.missing {
            return Err(Diagnostic::new(
                source,
                span,
                "embedded dynamic imports require a string literal module path",
            ));
        }
        Ok(self.imports)
    }
}

pub struct ParsedScript {
    pub resource: crate::ScriptResource,
    pub dependencies: Vec<Dependency>,
}

fn pattern_names(p: &js::Pat, names: &mut Vec<String>) {
    match p {
        js::Pat::Ident(i) => names.push(i.id.sym.to_string()),
        js::Pat::Array(a) => {
            for e in a.elems.iter().flatten() {
                pattern_names(e, names)
            }
        }
        js::Pat::Object(o) => {
            for k in &o.props {
                match k {
                    js::ObjectPatProp::Assign(a) => names.push(a.key.id.sym.to_string()),
                    js::ObjectPatProp::KeyValue(k) => pattern_names(&k.value, names),
                    js::ObjectPatProp::Rest(r) => pattern_names(&r.arg, names),
                }
            }
        }
        js::Pat::Assign(a) => pattern_names(&a.left, names),
        js::Pat::Rest(r) => pattern_names(&r.arg, names),
        _ => (),
    }
}
fn declared(d: &js::Decl, names: &mut Vec<String>) {
    match d {
        js::Decl::Class(c) => names.push(c.ident.sym.to_string()),
        js::Decl::Fn(f) => names.push(f.ident.sym.to_string()),
        js::Decl::Var(v) => {
            for dec in &v.decls {
                pattern_names(&dec.name, names)
            }
        }
        js::Decl::Using(v) => {
            for dec in &v.decls {
                pattern_names(&dec.name, names)
            }
        }
        _ => (),
    }
}
// var is function-scoped even inside a conditional or loop. Nested functions,
// class bodies and accessor bodies have a separate variable environment.
struct ScriptVars(Vec<String>);
impl Visit for ScriptVars {
    fn visit_var_decl(&mut self, d: &js::VarDecl) {
        if d.kind == js::VarDeclKind::Var {
            for v in &d.decls {
                pattern_names(&v.name, &mut self.0);
            }
        }
    }
    fn visit_function(&mut self, _: &js::Function) {}
    fn visit_arrow_expr(&mut self, _: &js::ArrowExpr) {}
    fn visit_class(&mut self, _: &js::Class) {}
    fn visit_getter_prop(&mut self, _: &js::GetterProp) {}
    fn visit_setter_prop(&mut self, _: &js::SetterProp) {}
}

fn directive_error(source: &str, start: usize, end: usize, msg: &str) -> Diagnostic {
    Diagnostic::new(source, SourceSpan { start, end }, msg)
}

/// Validate a JavaScript resource, discover its dependencies and retain the
/// source consumed by the engine. Qt classic directives are removed while
/// preserving all following source offsets and lines.
pub fn parse_resource(source: &str, module: bool) -> Result<ParsedScript, Diagnostic> {
    use crate::{ScriptKind, ScriptResource};
    let mut kind = if module {
        ScriptKind::Module
    } else {
        ScriptKind::Script
    };
    let mut dependencies = Vec::new();
    let mut exports = Vec::new();
    let mut writable_exports = Vec::new();
    let mut clean = source.to_owned();
    if !module {
        // The QML scanner only reads the directive prefix. Stop immediately on
        // reaching JavaScript so regexes, templates and comments use its grammar.
        let tokens = crate::lexer::Tokens::external(source);
        let mut i = 0;
        while tokens
            .get(i)
            .is_some_and(|t| matches!(t.kind, crate::lexer::Kind::Symbol(".")))
        {
            use crate::lexer::Kind;
            let start = tokens.get(i).unwrap().span.start;
            let end = source[start..]
                .find(['\r', '\n', '\u{2028}', '\u{2029}'])
                .map_or(source.len(), |n| start + n);
            let mut fields = Vec::new();
            while let Some(t) = tokens.get(i) {
                if t.span.start >= end || matches!(t.kind, Kind::End) {
                    break;
                }
                fields.push(t.kind.clone());
                i += 1;
            }
            match &fields[..] {
                [Kind::Symbol("."), Kind::Ident("pragma"), Kind::Ident("library")]
                    if kind == ScriptKind::Script =>
                {
                    kind = ScriptKind::Library
                }
                [Kind::Symbol("."), Kind::Ident("import"), Kind::String(path), Kind::Ident("as"), Kind::Ident(alias)]
                    if alias.chars().next().is_some_and(char::is_uppercase) =>
                {
                    if dependencies
                        .iter()
                        .any(|d: &Dependency| d.namespace.as_deref() == Some(alias))
                    {
                        return Err(directive_error(
                            source,
                            start,
                            end,
                            "duplicate JavaScript import namespace",
                        ));
                    }
                    dependencies.push(Dependency {
                        specifier: path.clone(),
                        namespace: Some((*alias).to_owned()),
                        span: None,
                    });
                }
                _ => {
                    return Err(directive_error(
                        source,
                        start,
                        end,
                        "expected .pragma library or .import \"file.js\" as Namespace",
                    ))
                }
            }
            clean.replace_range(start..end, &" ".repeat(end - start));
        }
    }
    let mut p = parser(&clean, 0, false)?;
    if module {
        let ast = p.parse_module().map_err(|e| error(source, e))?;
        let mut inspect = Inspect::default();
        ast.visit_with(&mut inspect);
        dependencies.extend(inspect.dependencies(source)?);
        for m in &ast.body {
            if let js::ModuleItem::ModuleDecl(decl) = m {
                let dep = match decl {
                    js::ModuleDecl::Import(i) => Some(i.src.as_ref()),
                    js::ModuleDecl::ExportNamed(e) => e.src.as_deref(),
                    js::ModuleDecl::ExportAll(e) => Some(e.src.as_ref()),
                    _ => None,
                };
                if let Some(s) = dep {
                    let name = s.value.as_str().ok_or_else(|| {
                        Diagnostic::new(source, span(s.span), "module path must be valid Unicode")
                    })?;
                    dependencies.push(Dependency {
                        specifier: name.into(),
                        namespace: None,
                        span: Some(span(s.span)),
                    });
                }
            }
        }
    } else {
        let ast = p.parse_script().map_err(|e| error(source, e))?;
        let mut inspect = Inspect::default();
        ast.visit_with(&mut inspect);
        dependencies.extend(inspect.dependencies(source)?);
        let mut variables = ScriptVars(Vec::new());
        ast.visit_with(&mut variables);
        writable_exports = variables.0;
        for stmt in &ast.body {
            if let js::Stmt::Decl(decl) = stmt {
                declared(decl, &mut exports);
                if let js::Decl::Fn(f) = decl {
                    writable_exports.push(f.ident.sym.to_string());
                }
            }
        }
    }
    check_errors(source, &mut p)?;
    writable_exports.sort();
    writable_exports.dedup();
    exports.extend(writable_exports.iter().cloned());
    exports.sort();
    exports.dedup();
    Ok(ParsedScript {
        resource: ScriptResource {
            executable: clean,
            import_hook: None,
            preload: false,
            imports: Default::default(),
            source: source.to_owned(),
            kind,
            dependencies: Default::default(),
            exports,
            writable_exports,
        },
        dependencies,
    })
}

/// Visit each JavaScript island in source order. Inline definitions keep the
/// importing document's resource context when they are instantiated later.
pub fn visit_scripts<E>(
    root: &mut crate::Object,
    visit: &mut impl FnMut(&mut JavaScript) -> Result<(), E>,
) -> Result<(), E> {
    fn members<E>(
        list: &mut [crate::Member],
        visit: &mut impl FnMut(&mut JavaScript) -> Result<(), E>,
    ) -> Result<(), E> {
        use crate::{Binding as B, MemberKind as M, Stmt};
        for member in list {
            match &mut member.kind {
                M::Child(child)
                | M::OnProperty { object: child, .. }
                | M::Component { object: child, .. } => visit_scripts(child, visit)?,
                M::Group { members: list, .. } => members(list, visit)?,
                M::Binding { value, .. }
                | M::Property {
                    value: Some(value), ..
                } => match value {
                    B::Expr(Expr::JavaScript(js)) => visit(js)?,
                    B::Script(stmts) => {
                        for stmt in stmts {
                            if let Stmt::JavaScript(js) = stmt {
                                visit(js)?;
                            }
                        }
                    }
                    B::Object(child) => visit_scripts(child, visit)?,
                    B::Objects(children) => {
                        for child in children {
                            visit_scripts(child, visit)?;
                        }
                    }
                    _ => (),
                },
                M::Function { body, .. } | M::Handler { body, .. } => {
                    for stmt in body {
                        if let Stmt::JavaScript(js) = stmt {
                            visit(js)?;
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
    members(&mut root.members, visit)
}

/// Analyze dynamic imports and special module expressions without changing any
/// user identifiers. Ranges refer to the supplied executable text.
pub struct ImportSites {
    pub dependencies: Vec<Dependency>,
    pub hook: Option<String>,
    pub edits: Vec<(SourceSpan, String)>,
}

pub fn import_sites(source: &str, expression: bool) -> Result<ImportSites, Diagnostic> {
    if !source.contains("import") {
        return Ok(ImportSites {
            dependencies: Vec::new(),
            hook: None,
            edits: Vec::new(),
        });
    }
    let mut p = parser(source, 0, true)?;
    let mut inspect = Inspect::default();
    if expression {
        p.parse_expr()
            .map_err(|e| error(source, e))?
            .visit_with(&mut inspect);
    } else {
        p.parse_script()
            .map_err(|e| error(source, e))?
            .visit_with(&mut inspect);
    }
    if let Some(span) = inspect.missing {
        return Err(Diagnostic::new(
            source,
            span,
            "embedded dynamic imports require a string literal module path",
        ));
    }
    let mut hook = "__ferroModuleContext".to_owned();
    while inspect.identifiers.contains(&hook) {
        hook.push('_');
    }
    let edits: Vec<_> = inspect
        .sites
        .into_iter()
        .map(|(span, meta)| {
            (
                span,
                format!("{hook}.{}", if meta { "meta" } else { "import" }),
            )
        })
        .collect();
    Ok(ImportSites {
        dependencies: inspect.imports,
        hook: (!edits.is_empty()).then_some(hook),
        edits,
    })
}
