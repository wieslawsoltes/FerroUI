//! Parse JavaScript at QML syntax boundaries using the ECMAScript grammar.
use crate::{Diagnostic, Expr, JavaScript, ScriptForm, SourceSpan};
use swc_common::{BytePos, Spanned};
use swc_ecma_ast as js;
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};

pub(super) fn parser(source: &str, start: usize) -> Result<Parser<Lexer<'_>>, Diagnostic> {
    let len = u32::try_from(source.len()).map_err(|_| {
        Diagnostic::new(
            source,
            SourceSpan::default(),
            "JavaScript source exceeds 4 GiB",
        )
    })?;
    let input = StringInput::new(&source[start..], BytePos(start as u32), BytePos(len));
    let syntax = EsSyntax {
        allow_return_outside_function: true,
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
fn script(source: &str, range: SourceSpan, form: ScriptForm) -> JavaScript {
    JavaScript {
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
    let mut p = parser(source, start)?;
    let expr = p.parse_expr().map_err(|e| error(source, e))?;
    if let Some(err) = p.take_errors().into_iter().next() {
        return Err(error(source, err));
    }
    Ok((span(expr.span()), expr))
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
                && matches!(u.arg.as_ref(), js::Expr::Lit(js::Lit::Num(_))) =>
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
        _ => Expr::JavaScript(script(source, range, ScriptForm::Expression)),
    };
    if let Expr::JavaScript(js) = &mut value {
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
    let mut p = parser(source, start)?;
    let stmt = p.parse_stmt().map_err(|e| error(source, e))?;
    if let Some(err) = p.take_errors().into_iter().next() {
        return Err(error(source, err));
    }
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
    code.references = inspect.names.into_iter().collect();
    Ok((range, code))
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
        code.source = format!(
            "return ({}).call(this{}{});",
            code.source,
            if count > 0 { "," } else { "" },
            parameters.join(",")
        );
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
    missing: Option<SourceSpan>,
}

impl Visit for Inspect {
    fn visit_ident(&mut self, name: &js::Ident) {
        let name = name.sym.as_str();
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
    fn visit_call_expr(&mut self, e: &js::CallExpr) {
        if matches!(e.callee, js::Callee::Import(_)) {
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
    let mut p = parser(&clean, 0)?;
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
        for stmt in &ast.body {
            if let js::Stmt::Decl(decl) = stmt {
                declared(decl, &mut exports);
            }
        }
    }
    if let Some(err) = p.take_errors().into_iter().next() {
        return Err(error(source, err));
    }
    exports.sort();
    exports.dedup();
    Ok(ParsedScript {
        resource: ScriptResource {
            preload: false,
            imports: Default::default(),
            source: clean,
            kind,
            dependencies: Default::default(),
            exports,
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

/// Find dynamic imports in a QML script island. Offsets are relative to its
/// executable text (including any generated parameter forwarding).
pub fn inline_dependencies(script: &JavaScript) -> Result<Vec<Dependency>, Diagnostic> {
    if !script.source.contains("import") {
        return Ok(Vec::new());
    }
    let mut p = parser(&script.source, 0)?;
    let mut inspect = Inspect::default();
    if script.form == ScriptForm::Expression {
        p.parse_expr()
            .map_err(|e| error(&script.source, e))?
            .visit_with(&mut inspect);
    } else {
        p.parse_script()
            .map_err(|e| error(&script.source, e))?
            .visit_with(&mut inspect);
    }
    inspect.dependencies(&script.source)
}
