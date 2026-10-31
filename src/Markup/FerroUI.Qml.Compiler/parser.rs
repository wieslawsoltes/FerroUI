//! Recursive-descent object grammar and precedence-climbing expression parser.
use crate::lexer::{tokenize, Kind, Token};
use crate::*;
use std::collections::HashSet;

type Result<T> = std::result::Result<T, Diagnostic>;
const MAX_NESTING: usize = 64;

/// Parse one UTF-8 QML component. Offsets on diagnostics are byte offsets;
/// displayed lines and Unicode scalar columns are one-based.
pub fn parse(source: &str) -> Result<Document> {
    let document = Parser {
        source,
        tokens: tokenize(source)?,
        pos: 0,
        depth: 0,
        loops: 0,
    }
    .document()?;
    validate(&document, source)?;
    Ok(document)
}

struct Parser<'s> {
    source: &'s str,
    tokens: Vec<Token<'s>>,
    pos: usize,
    depth: usize,
    loops: usize,
}

impl<'s> Parser<'s> {
    fn token(&self) -> &Token<'s> {
        &self.tokens[self.pos]
    }
    fn at(&self, text: &str) -> bool {
        token_is(&self.token().kind, text)
    }
    fn eat(&mut self, text: &str) -> bool {
        if self.at(text) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, text: &str) -> Result<()> {
        if self.eat(text) {
            Ok(())
        } else {
            Err(self.error(format!("expected '{text}'")))
        }
    }
    fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.source, self.token().span, message)
    }
    fn span(&self, start: usize) -> Span {
        Span {
            start,
            end: self.tokens[self.pos - 1].span.end,
        }
    }
    fn nested<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        if self.depth >= MAX_NESTING {
            return Err(self.error("maximum syntax nesting exceeded"));
        }
        self.depth += 1;
        let result = f(self);
        self.depth -= 1;
        result
    }
    fn ident(&mut self) -> Result<String> {
        if let Kind::Ident(value) = self.token().kind {
            self.pos += 1;
            Ok(value.to_owned())
        } else {
            Err(self.error("expected an identifier"))
        }
    }
    fn name(&mut self) -> Result<String> {
        let start = self.token().span;
        let name = self.ident()?;
        if !valid_name(&name) {
            return Err(Diagnostic::new(
                self.source,
                start,
                format!("invalid QML name '{name}'"),
            ));
        }
        Ok(name)
    }
    fn qualified(&mut self) -> Result<String> {
        let mut name = self.ident()?;
        while self.eat(".") {
            name.push('.');
            name.push_str(&self.ident()?);
        }
        Ok(name)
    }
    fn ty(&mut self) -> Result<String> {
        let name = self.qualified()?;
        if name == "list" && self.eat("<") {
            let element = self.qualified()?;
            self.expect(">")?;
            Ok(format!("list<{element}>"))
        } else {
            Ok(name)
        }
    }
    fn end_statement(&mut self) -> Result<()> {
        if self.eat(";")
            || self.at("}")
            || self.token().line_break
            || self.token().kind == Kind::End
        {
            Ok(())
        } else {
            Err(self.error("expected ';' or a line break"))
        }
    }

    fn document(&mut self) -> Result<Document> {
        let mut pragmas = Vec::new();
        while self.at("pragma") {
            pragmas.push(self.pragma()?);
        }
        let mut imports = Vec::new();
        while self.at("import") {
            imports.push(self.import()?);
        }
        let root = self.object()?;
        if self.token().kind != Kind::End {
            return Err(self.error("expected end of document after the root object"));
        }
        Ok(Document {
            singletons: Vec::new(),
            pragmas,
            imports,
            root,
        })
    }
    fn pragma(&mut self) -> Result<Pragma> {
        let start = self.token().span.start;
        self.expect("pragma")?;
        let name = self.ident()?;
        let mut values = Vec::new();
        if self.eat(":") {
            loop {
                values.push(match &self.token().kind {
                    Kind::Ident(text) => (*text).to_owned(),
                    Kind::String(text) => text.clone(),
                    _ => return Err(self.error("expected a pragma value")),
                });
                self.pos += 1;
                if !self.eat(",") {
                    break;
                }
            }
        }
        let span = self.span(start);
        self.end_statement()?;
        Ok(Pragma { name, values, span })
    }
    fn import(&mut self) -> Result<Import> {
        let start = self.token().span.start;
        self.expect("import")?;
        let (uri, is_path) = if let Kind::String(text) = &self.token().kind {
            let uri = text.clone();
            self.pos += 1;
            (uri, true)
        } else {
            (self.qualified()?, false)
        };
        let version = match self.token().kind {
            Kind::Number(text) if !is_path => {
                let parts: Vec<_> = text.split('.').collect();
                if parts.len() > 2
                    || !parts.iter().all(|s| {
                        !s.is_empty()
                            && s.bytes().all(|b| b.is_ascii_digit())
                            && s.parse::<u16>().is_ok()
                    })
                {
                    return Err(self.error("invalid QML import version"));
                }
                self.pos += 1;
                Some(text.to_owned())
            }
            _ => None,
        };
        let alias = if self.eat("as") {
            let alias_span = self.token().span;
            let alias = self.ident()?;
            if !is_type(&alias) {
                return Err(Diagnostic::new(
                    self.source,
                    alias_span,
                    "import qualifiers must start with an uppercase letter",
                ));
            }
            Some(alias)
        } else {
            None
        };
        let span = self.span(start);
        self.end_statement()?;
        Ok(Import {
            uri,
            is_path,
            version,
            alias,
            span,
        })
    }
    fn object(&mut self) -> Result<Object> {
        let start = self.token().span.start;
        let type_name = self.qualified()?;
        self.object_body(start, type_name)
    }
    fn object_body(&mut self, start: usize, type_name: String) -> Result<Object> {
        if !is_type(&type_name) {
            return Err(Diagnostic::new(
                self.source,
                self.span(start),
                "expected an uppercase QML type name",
            ));
        }
        let members = self.members()?;
        Ok(Object {
            type_imports: None,
            composite_types: Vec::new(),
            type_name,
            members,
            span: self.span(start),
        })
    }
    fn members(&mut self) -> Result<Vec<Member>> {
        self.nested(|p| {
            p.expect("{")?;
            let mut members = Vec::new();
            while !p.eat("}") {
                if p.token().kind == Kind::End {
                    return Err(p.error("expected '}' to close the object"));
                }
                if p.eat(";") {
                    continue;
                }
                let start = p.token().span.start;
                let kind = p.member()?;
                members.push(Member {
                    kind,
                    span: p.span(start),
                });
            }
            Ok(members)
        })
    }
    fn member(&mut self) -> Result<MemberKind> {
        let start = self.token().span.start;
        // Contextual keywords can also name native properties (Binding.property).
        if self
            .tokens
            .get(self.pos + 1)
            .is_some_and(|t| token_is(&t.kind, ":"))
        {
            return self.named_member(start);
        }
        let mut modifiers = Modifiers::default();
        loop {
            let slot = match self.token().kind {
                Kind::Ident("required") => &mut modifiers.required,
                Kind::Ident("readonly") => &mut modifiers.readonly,
                Kind::Ident("default") => &mut modifiers.default,
                Kind::Ident("final") => &mut modifiers.final_,
                Kind::Ident("virtual") => &mut modifiers.virtual_,
                Kind::Ident("override") => &mut modifiers.override_,
                _ => break,
            };
            if *slot {
                return Err(self.error("duplicate property modifier"));
            }
            *slot = true;
            self.pos += 1;
        }
        if self.eat("property") {
            let ty = self.ty()?;
            let name = self.name()?;
            let value = if self.eat(":") {
                Some(self.binding()?)
            } else {
                None
            };
            if modifiers.required && value.is_some() {
                return Err(Diagnostic::new(
                    self.source,
                    self.span(start),
                    "a required property cannot have an initializer",
                ));
            }
            if modifiers.readonly && (value.is_none() || modifiers.default) {
                return Err(Diagnostic::new(
                    self.source,
                    self.span(start),
                    "a readonly property needs an initializer and cannot be default",
                ));
            }
            if ty == "alias" && !matches!(&value, Some(Binding::Expr(expr)) if alias_path(expr)) {
                return Err(Diagnostic::new(
                    self.source,
                    self.span(start),
                    "expected an alias target of the form id, id.property or id.property.field",
                ));
            }
            self.member_end(value.as_ref())?;
            return Ok(MemberKind::Property {
                name,
                ty,
                modifiers,
                value,
            });
        }
        if modifiers != Modifiers::default() {
            return Err(self.error("expected 'property' after modifier"));
        }
        if self.eat("signal") {
            let name = self.name()?;
            let parameters = if self.at("(") {
                self.parameters(true)?
            } else {
                vec![]
            };
            self.end_statement()?;
            return Ok(MemberKind::Signal { name, parameters });
        }
        if self.eat("function") {
            let name = self.name()?;
            let parameters = self.parameters(false)?;
            let return_type = if self.eat(":") {
                Some(self.ty()?)
            } else {
                None
            };
            let body = self.block()?;
            return Ok(MemberKind::Function {
                name,
                parameters,
                return_type,
                body,
            });
        }
        if self.eat("component") {
            let name = self.ident()?;
            if !is_type(&name) {
                return Err(
                    self.error("inline component names must start with an uppercase letter")
                );
            }
            self.expect(":")?;
            let object = self.object()?;
            return Ok(MemberKind::Component { name, object });
        }
        if self.eat("enum") {
            return self.enumeration();
        }
        self.named_member(start)
    }
    fn named_member(&mut self, start: usize) -> Result<MemberKind> {
        let name = self.qualified()?;
        if is_type(&name) && self.eat("on") {
            let target = self.qualified()?;
            return Ok(MemberKind::OnProperty {
                name: target,
                object: self.object_body(start, name)?,
            });
        }
        if self.at("{") {
            if is_type(&name) {
                return self.object_body(start, name).map(MemberKind::Child);
            }
            return Ok(MemberKind::Group {
                name,
                members: self.members()?,
            });
        }
        self.expect(":")?;
        if name == "id" {
            let id = self.name()?;
            self.end_statement()?;
            Ok(MemberKind::Id(id))
        } else if name.rsplit('.').next().is_some_and(|s| {
            s.strip_prefix("on")
                .and_then(|tail| tail.chars().next())
                .is_some_and(char::is_uppercase)
        }) {
            let (parameters, body) = self.handler()?;
            Ok(MemberKind::Handler {
                name,
                parameters,
                body,
            })
        } else {
            let value = self.binding()?;
            self.member_end(Some(&value))?;
            Ok(MemberKind::Binding { name, value })
        }
    }

    fn member_end(&mut self, value: Option<&Binding>) -> Result<()> {
        if matches!(value, Some(Binding::Object(_) | Binding::Script(_))) {
            Ok(())
        } else {
            self.end_statement()
        }
    }
    fn parameters(&mut self, signal: bool) -> Result<Vec<Parameter>> {
        self.expect("(")?;
        let mut params = Vec::new();
        let mut names = HashSet::new();
        if !self.eat(")") {
            loop {
                let (name, ty) = if signal {
                    let first = self.ty()?;
                    if self.eat(":") {
                        (first, Some(self.ty()?))
                    } else {
                        (self.ident()?, Some(first))
                    }
                } else {
                    let name = self.ident()?;
                    let ty = if self.eat(":") {
                        Some(self.ty()?)
                    } else {
                        None
                    };
                    (name, ty)
                };
                if reserved(&name) || name.contains(['.', '<']) {
                    return Err(self.error("invalid parameter name"));
                }
                if !names.insert(name.clone()) {
                    return Err(self.error("duplicate parameter name"));
                }
                params.push(Parameter { name, ty });
                if self.eat(")") {
                    break;
                }
                self.expect(",")?;
            }
        }
        Ok(params)
    }
    fn enumeration(&mut self) -> Result<MemberKind> {
        let name = self.ident()?;
        self.expect("{")?;
        let mut values = Vec::new();
        let mut next = Some(0_i32);
        let mut names = HashSet::new();
        while !self.eat("}") {
            let key = self.ident()?;
            if !names.insert(key.clone()) {
                return Err(self.error("duplicate enum key"));
            }
            let value = if self.eat("=") {
                let negative = self.eat("-");
                let Kind::Number(text) = self.token().kind else {
                    return Err(self.error("expected an integer enum value"));
                };
                let raw = number_value(text).unwrap_or(f64::NAN);
                let value = if negative { -raw } else { raw };
                if value.fract() != 0. || !(i32::MIN as f64..=i32::MAX as f64).contains(&value) {
                    return Err(self.error("enum value is outside the 32-bit integer range"));
                }
                self.pos += 1;
                value as i32
            } else {
                next.ok_or_else(|| self.error("implicit enum value overflow"))?
            };
            next = value.checked_add(1);
            values.push((key, value));
            if self.eat("}") {
                break;
            }
            self.expect(",")?;
        }
        Ok(MemberKind::Enum { name, values })
    }
    fn object_ahead(&self, mut i: usize) -> bool {
        let Some(Token {
            kind: Kind::Ident(mut name),
            ..
        }) = self.tokens.get(i)
        else {
            return false;
        };
        i += 1;
        while self.tokens.get(i).is_some_and(|t| token_is(&t.kind, ".")) {
            let Some(Token {
                kind: Kind::Ident(next),
                ..
            }) = self.tokens.get(i + 1)
            else {
                return false;
            };
            name = next;
            i += 2;
        }
        is_type(name) && self.tokens.get(i).is_some_and(|t| token_is(&t.kind, "{"))
    }
    fn binding(&mut self) -> Result<Binding> {
        if self.at("{") {
            return self.block().map(Binding::Script);
        }
        if self.object_ahead(self.pos) {
            return self.object().map(Binding::Object);
        }
        if self.at("[") && self.object_ahead(self.pos + 1) {
            self.expect("[")?;
            let mut objects = Vec::new();
            loop {
                objects.push(self.object()?);
                if self.eat("]") {
                    break;
                }
                self.expect(",")?;
                if self.eat("]") {
                    break;
                }
            }
            return Ok(Binding::Objects(objects));
        }
        self.expression().map(Binding::Expr)
    }

    fn handler(&mut self) -> Result<(Vec<String>, Vec<Stmt>)> {
        let mut params = Vec::new();
        if self.eat("function") {
            params = self
                .parameters(false)?
                .into_iter()
                .map(|p| p.name)
                .collect();
            return Ok((params, self.block()?));
        }
        if self.arrow_params_ahead() {
            if self.eat("(") {
                if !self.eat(")") {
                    loop {
                        let name = self.ident()?;
                        if reserved(&name) || params.contains(&name) {
                            return Err(self.error("invalid arrow parameter"));
                        }
                        params.push(name);
                        if self.eat(")") {
                            break;
                        }
                        self.expect(",")?;
                    }
                }
            } else {
                let name = self.ident()?;
                if reserved(&name) {
                    return Err(self.error("invalid arrow parameter"));
                }
                params.push(name);
            }
            self.expect("=>")?;
        }
        let body = if self.at("{") {
            self.block()?
        } else {
            vec![self.statement()?]
        };
        Ok((params, body))
    }
    fn arrow_params_ahead(&self) -> bool {
        let mut i = self.pos;
        let tokens = &self.tokens;
        let at = |i: usize, symbol: &str| tokens.get(i).is_some_and(|t| token_is(&t.kind, symbol));
        let ident = |i: usize| matches!(tokens.get(i).map(|t| &t.kind), Some(Kind::Ident(_)));
        if at(i, "(") {
            i += 1;
            while ident(i) {
                i += 1;
                if !at(i, ",") {
                    break;
                }
                i += 1;
            }
            if !at(i, ")") {
                return false;
            }
        } else if !ident(i) {
            return false;
        }
        at(i + 1, "=>") && !tokens[i + 1].line_break
    }
    fn block(&mut self) -> Result<Vec<Stmt>> {
        self.nested(|p| {
            p.expect("{")?;
            let mut body = Vec::new();
            while !p.eat("}") {
                if p.token().kind == Kind::End {
                    return Err(p.error("unterminated script block"));
                }
                if p.eat(";") {
                    continue;
                }
                body.push(p.statement()?);
            }
            Ok(body)
        })
    }
    fn statement(&mut self) -> Result<Stmt> {
        self.nested(|p| p.statement_inner())
    }
    fn statement_inner(&mut self) -> Result<Stmt> {
        if self.at("{") {
            return self.block().map(Stmt::Block);
        }
        if self.eat("if") {
            self.expect("(")?;
            let test = self.expression()?;
            self.expect(")")?;
            let then_body = vec![self.statement()?];
            let else_body = if self.eat("else") {
                vec![self.statement()?]
            } else {
                Vec::new()
            };
            return Ok(Stmt::If {
                test,
                then_body,
                else_body,
            });
        }
        if self.eat("return") {
            let value = if self.at(";") || self.at("}") || self.token().line_break {
                None
            } else {
                Some(self.expression()?)
            };
            self.end_statement()?;
            return Ok(Stmt::Return(value));
        }
        if self.eat("while") {
            self.expect("(")?;
            let test = self.expression()?;
            self.expect(")")?;
            let body = self.loop_body()?;
            return Ok(Stmt::While {
                test,
                body,
                post_test: false,
            });
        }
        if self.eat("do") {
            let body = self.loop_body()?;
            self.expect("while")?;
            self.expect("(")?;
            let test = self.expression()?;
            self.expect(")")?;
            self.eat(";");
            return Ok(Stmt::While {
                test,
                body,
                post_test: true,
            });
        }
        if self.eat("for") {
            self.expect("(")?;
            let init = if self.at(";") {
                None
            } else {
                Some(Box::new(self.simple_statement(true)?))
            };
            self.expect(";")?;
            let test = if self.at(";") {
                None
            } else {
                Some(self.expression()?)
            };
            self.expect(";")?;
            let update = if self.at(")") {
                None
            } else {
                Some(Box::new(self.simple_statement(false)?))
            };
            self.expect(")")?;
            let body = self.loop_body()?;
            return Ok(Stmt::For {
                init,
                test,
                update,
                body,
            });
        }
        if self.at("break") || self.at("continue") {
            if self.loops == 0 {
                return Err(self.error("loop control outside a loop"));
            }
            let stmt = if self.eat("break") {
                Stmt::Break
            } else {
                self.expect("continue")?;
                Stmt::Continue
            };
            self.end_statement()?;
            return Ok(stmt);
        }
        if self.eat(";") {
            return Ok(Stmt::Block(Vec::new()));
        }
        let stmt = self.simple_statement(true)?;
        self.end_statement()?;
        Ok(stmt)
    }

    fn loop_body(&mut self) -> Result<Vec<Stmt>> {
        self.loops += 1;
        let stmt = self.statement();
        self.loops -= 1;
        Ok(vec![stmt?])
    }

    fn simple_statement(&mut self, declaration: bool) -> Result<Stmt> {
        if declaration && self.eat("let") {
            let name = self.ident()?;
            if reserved(&name) {
                return Err(self.error("invalid local name"));
            }
            let value = if self.eat("=") {
                Some(self.expression()?)
            } else {
                None
            };
            return Ok(Stmt::Let { name, value });
        }
        let prefix = match self.token().kind {
            Kind::Symbol(op @ ("++" | "--")) => {
                self.pos += 1;
                Some(op)
            }
            _ => None,
        };
        let target = self.expression()?;
        let op = match self.token().kind {
            Kind::Symbol(
                op @ ("=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "**=" | "<<="
                | ">>=" | ">>>=" | "??=" | "||=" | "&&="),
            ) if prefix.is_none() => Some(op),
            Kind::Symbol(op @ ("++" | "--")) if prefix.is_none() && !self.token().line_break => {
                Some(op)
            }
            _ => None,
        };
        let result = match prefix.or(op) {
            Some(op) => {
                if !matches!(target, Expr::Name(_) | Expr::Member(..) | Expr::Index(..)) {
                    return Err(self.error("expected an assignment target"));
                }
                if prefix.is_none() {
                    self.pos += 1;
                }
                let (op, value) = match op {
                    "++" | "--" if prefix.is_some() => (op, Expr::Undefined),
                    "++" => ("post++", Expr::Undefined),
                    "--" => ("post--", Expr::Undefined),
                    _ => (op, self.expression()?),
                };
                Stmt::Assign {
                    target,
                    op: op.to_owned(),
                    value,
                }
            }
            None => Stmt::Expr(target),
        };
        Ok(result)
    }

    fn expression(&mut self) -> Result<Expr> {
        self.expr_bp(0).map(|node| node.expr)
    }
    fn expr_bp(&mut self, min: u8) -> Result<Node> {
        self.nested(|p| p.expr_inner(min))
    }
    fn expr_inner(&mut self, min: u8) -> Result<Node> {
        let mut node = match &self.token().kind {
            Kind::Symbol(op @ ("!" | "-" | "+" | "~")) | Kind::Ident(op @ ("typeof" | "void")) => {
                let op = (*op).to_owned();
                self.pos += 1;
                let operand = self.expr_bp(27)?;
                Node {
                    grouped: false,
                    depth: operand.depth + 1,
                    expr: Expr::Unary(op, Box::new(operand.expr)),
                }
            }
            Kind::Number(text) => {
                let value = number_value(text)
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| self.error("number is outside the supported finite range"))?;
                self.pos += 1;
                Node::leaf(Expr::Number(value))
            }
            Kind::String(value) => {
                let value = value.clone();
                self.pos += 1;
                Node::leaf(Expr::String(value))
            }
            Kind::Ident(name) => {
                let expr = match *name {
                    "null" => Expr::Null,
                    "undefined" => Expr::Undefined,
                    "true" | "false" => Expr::Bool(*name == "true"),
                    "this" => Expr::Name("this".into()),
                    name if !reserved(name) => Expr::Name(name.to_owned()),
                    _ => return Err(self.error("unsupported expression keyword")),
                };
                self.pos += 1;
                Node::leaf(expr)
            }
            Kind::Symbol("(") => {
                self.pos += 1;
                let mut value = self.expr_bp(0)?;
                self.expect(")")?;
                value.grouped = true;
                value
            }
            Kind::Symbol("[") => {
                self.pos += 1;
                let (args, depth) = self.arguments("]")?;
                Node {
                    grouped: false,
                    expr: Expr::Array(args),
                    depth,
                }
            }
            _ => return Err(self.error("expected an expression")),
        };
        loop {
            if node.depth > MAX_NESTING {
                return Err(self.error("maximum expression depth exceeded"));
            }
            if self.eat(".") {
                node = Node {
                    grouped: false,
                    depth: node.depth + 1,
                    expr: Expr::Member(Box::new(node.expr), self.ident()?),
                };
            } else if self.eat("[") {
                let sub = self.expr_bp(0)?;
                self.expect("]")?;
                node = Node {
                    grouped: false,
                    depth: node.depth.max(sub.depth) + 1,
                    expr: Expr::Index(Box::new(node.expr), Box::new(sub.expr)),
                };
            } else if self.eat("(") {
                let (args, depth) = self.arguments(")")?;
                node = Node {
                    grouped: false,
                    depth: (node.depth + 1).max(depth),
                    expr: Expr::Call(Box::new(node.expr), args),
                };
            } else if min <= 1 && self.eat("?") {
                let yes = self.expr_bp(0)?;
                self.expect(":")?;
                let no = self.expr_bp(1)?;
                node = Node {
                    grouped: false,
                    depth: node.depth.max(yes.depth).max(no.depth) + 1,
                    expr: Expr::Conditional(
                        Box::new(node.expr),
                        Box::new(yes.expr),
                        Box::new(no.expr),
                    ),
                };
            } else if let Kind::Symbol(op) | Kind::Ident(op) = self.token().kind {
                let Some((left, right)) = binding_power(op) else {
                    break;
                };
                if left < min {
                    break;
                }
                if op == "**" && !node.grouped && matches!(node.expr, Expr::Unary(..)) {
                    return Err(self.error("parenthesize the unary base of exponentiation"));
                }
                let operator = self.token().span;
                self.pos += 1;
                let rhs = self.expr_bp(right)?;
                if node.mixes_nullish(op) || rhs.mixes_nullish(op) {
                    return Err(Diagnostic::new(
                        self.source,
                        operator,
                        "parenthesize mixed nullish and logical operators",
                    ));
                }
                node = Node {
                    grouped: false,
                    depth: node.depth.max(rhs.depth) + 1,
                    expr: Expr::Binary(op.to_owned(), Box::new(node.expr), Box::new(rhs.expr)),
                };
            } else {
                break;
            }
        }
        Ok(node)
    }
    fn arguments(&mut self, close: &str) -> Result<(Vec<Expr>, usize)> {
        let mut values = Vec::new();
        let mut depth = 1;
        while !self.eat(close) {
            let value = self.expr_bp(0)?;
            depth = depth.max(value.depth + 1);
            values.push(value.expr);
            if self.eat(close) {
                break;
            }
            self.expect(",")?;
        }
        Ok((values, depth))
    }
}

struct Node {
    grouped: bool,
    expr: Expr,
    depth: usize,
}
impl Node {
    fn mixes_nullish(&self, op: &str) -> bool {
        if self.grouped {
            return false;
        }
        matches!(&self.expr,Expr::Binary(inner,..) if matches!((op,inner.as_str()),("??","&&"|"||")|("&&"|"||","??")))
    }
    fn leaf(expr: Expr) -> Self {
        Self {
            expr,
            depth: 1,
            grouped: false,
        }
    }
}

fn token_is(token: &Kind<'_>, text: &str) -> bool {
    matches!(token, Kind::Ident(value) | Kind::Symbol(value) if *value == text)
}

fn number_value(text: &str) -> Option<f64> {
    let radix = match text.as_bytes().get(..2) {
        Some(b"0x" | b"0X") => 16,
        Some(b"0b" | b"0B") => 2,
        Some(b"0o" | b"0O") => 8,
        _ => return text.parse().ok(),
    };
    text[2..].chars().try_fold(0.0, |n, c| {
        Some(n * f64::from(radix) + f64::from(c.to_digit(radix)?))
    })
}

fn binding_power(op: &str) -> Option<(u8, u8)> {
    let left = match op {
        "||" | "??" => 2,
        "&&" => 4,
        "|" => 6,
        "^" => 8,
        "&" => 10,
        "==" | "!=" | "===" | "!==" => 12,
        "<" | ">" | "<=" | ">=" | "in" | "instanceof" => 14,
        "<<" | ">>" | ">>>" => 16,
        "+" | "-" => 18,
        "*" | "/" | "%" => 20,
        "**" => return Some((24, 24)),
        _ => return None,
    };
    Some((left, left + 1))
}

fn is_type(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .and_then(|n| n.chars().next())
        .is_some_and(char::is_uppercase)
}

fn valid_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_lowercase())
        && name.chars().all(|ch| ch == '_' || ch.is_alphanumeric())
        && !reserved(name)
}

fn reserved(name: &str) -> bool {
    matches!(
        name,
        "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "instanceof"
            | "interface"
            | "let"
            | "new"
            | "null"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

fn alias_path(expr: &Expr) -> bool {
    let mut path = expr;
    let mut depth = 0;
    while let Expr::Member(target, _) = path {
        depth += 1;
        path = target;
    }
    depth <= 2 && matches!(path, Expr::Name(id) if valid_name(id))
}
