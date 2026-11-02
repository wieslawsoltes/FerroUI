//! Source queries for tools. Scan only the declaration header, skipping QML trivia
//! with the same scanner as the parser. JavaScript bodies are never tokenized here.
use crate::lexer::{Kind, Tokens};
use crate::{Member, MemberKind, SourceSpan};

/// Range of the declared name, excluding modifiers, annotations and initializers.
pub fn member_name_span(source: &str, member: &Member) -> Option<SourceSpan> {
    let (name, skip, last) = match &member.kind {
        MemberKind::Id(name) => (name.as_str(), 2, false),
        MemberKind::Property { name, .. } => (name.as_str(), 0, true),
        MemberKind::Signal { name, .. }
        | MemberKind::Enum { name, .. }
        | MemberKind::Component { name, .. }
        | MemberKind::Function { name, .. } => (name.as_str(), 1, false),
        MemberKind::Binding { name, .. }
        | MemberKind::Group { name, .. }
        | MemberKind::Handler { name, .. } => (name.as_str(), 0, false),
        MemberKind::Child(obj) | MemberKind::OnProperty { object: obj, .. } => {
            (obj.type_name.as_str(), 0, false)
        }
    };
    let slice = source.get(member.span.start..member.span.end)?;
    let tokens = Tokens::external(slice);
    let mut found = None;
    let mut segments = name.split('.').peekable();
    let mut segment = segments.next()?;
    let mut first = None;
    for index in skip.. {
        let Some(token) = tokens.get(index) else {
            break;
        };
        match token.kind {
            Kind::Ident(id) if id == segment => {
                let start = *first.get_or_insert(member.span.start + token.span.start);
                if let Some(next) = segments.next() {
                    segment = next;
                } else {
                    found = Some(SourceSpan {
                        start,
                        end: member.span.start + token.span.end,
                    });
                    if !last {
                        break;
                    }
                    first = None;
                }
            }
            Kind::Ident(_) if last => {}
            Kind::Symbol(":" | ";" | "{" | "(") | Kind::End | Kind::Invalid(_) => break,
            _ => {}
        }
    }
    found
}

/// Qualified type name in an object declaration, allowing trivia between segments.
pub fn object_type_span(source: &str, object: &crate::Object) -> Option<SourceSpan> {
    let tokens = Tokens::external(source.get(object.span.start..object.span.end)?);
    let mut span = tokens.get(0)?.span;
    for (index, name) in object.type_name.split('.').enumerate() {
        let token = tokens.get(index * 2)?;
        if token.kind != Kind::Ident(name) {
            return None;
        }
        span.end = token.span.end;
    }
    Some(SourceSpan {
        start: object.span.start + span.start,
        end: object.span.start + span.end,
    })
}

/// An authored name potentially supplied by the QML scope. A receiver is present
/// only for a named member of `this` or of a free identifier.
pub struct ScriptReference {
    pub span: SourceSpan,
    pub receiver: Option<String>,
    pub name: String,
}

/// An authored JavaScript fragment, sufficient to analyze references later without
/// retaining the executable text, module graph or complete component IR.
#[derive(Clone, Copy)]
pub struct ScriptFragment {
    span: SourceSpan,
    form: FragmentForm,
}

#[derive(Clone, Copy)]
enum FragmentForm {
    Body,
    Expression,
    Name,
    Member(usize),
}

impl ScriptFragment {
    pub fn new(script: &crate::JavaScript) -> Self {
        let expression = script.form == crate::ScriptForm::Expression
            || (script.form == crate::ScriptForm::HandlerBody
                && script
                    .executable
                    .as_deref()
                    .is_some_and(|s| s.starts_with("return (")));
        let form = if !expression {
            FragmentForm::Body
        } else {
            // Only reuse exact spellings recognized by the ECMAScript parser.
            // Whitespace, escaping, comments and parentheses use the resolver.
            match script
                .property_read
                .as_ref()
                .map(|read| (read.receiver.as_deref(), read.property.as_str()))
            {
                Some((None, name)) if name == script.source => FragmentForm::Name,
                Some((Some(receiver), name))
                    if script
                        .source
                        .strip_prefix(receiver)
                        .and_then(|tail| tail.strip_prefix('.'))
                        == Some(name) =>
                {
                    FragmentForm::Member(receiver.len())
                }
                _ => FragmentForm::Expression,
            }
        };
        Self {
            span: script.span,
            form,
        }
    }

    pub fn span(self) -> SourceSpan {
        self.span
    }

    /// Resolve references in the document bytes used to create this fragment.
    pub fn references(self, document: &str, parameters: &[&str]) -> Vec<ScriptReference> {
        document
            .get(self.span.start..self.span.end)
            .map_or_else(Vec::new, |source| references(self, source, parameters))
    }
}

/// Locate QML candidates after resolving JavaScript lexical declarations and
/// arguments. Index authored bytes rather than lowered or generated JavaScript.
pub fn script_references(script: &crate::JavaScript, parameters: &[&str]) -> Vec<ScriptReference> {
    references(ScriptFragment::new(script), &script.source, parameters)
}

fn references(fragment: ScriptFragment, source: &str, parameters: &[&str]) -> Vec<ScriptReference> {
    use swc_common::{Globals, Mark, GLOBALS};
    use swc_ecma_ast::{Pass, Program};
    use swc_ecma_transforms_base::resolver;
    use swc_ecma_visit::VisitWith;

    if fragment.span.end.checked_sub(fragment.span.start) != Some(source.len()) {
        return Vec::new();
    }
    if parameters.is_empty() {
        let span = fragment.span;
        match fragment.form {
            FragmentForm::Name => {
                return vec![ScriptReference {
                    span,
                    name: source.into(),
                    receiver: None,
                }]
            }
            FragmentForm::Member(dot) => {
                if let Some((owner, member)) =
                    source.split_once('.').filter(|(id, _)| id.len() == dot)
                {
                    let end = span.start + dot;
                    let mut reads = Vec::with_capacity(2);
                    if owner != "this" {
                        reads.push(ScriptReference {
                            span: SourceSpan {
                                start: span.start,
                                end,
                            },
                            name: owner.into(),
                            receiver: None,
                        });
                    }
                    reads.push(ScriptReference {
                        span: SourceSpan {
                            start: end + 1,
                            end: span.end,
                        },
                        name: member.into(),
                        receiver: Some(owner.into()),
                    });
                    return reads;
                }
            }
            _ => {}
        }
    }
    let expression = !matches!(fragment.form, FragmentForm::Body);
    let prefix = format!("(({}) => {{\n", parameters.join(","));
    let prefix = if expression {
        format!("{prefix}return (")
    } else {
        prefix
    };
    let suffix = if expression { "\n);\n});" } else { "\n});" };
    let input = format!("{prefix}{source}{suffix}");
    GLOBALS.set(&Globals::new(), || {
        let Some(program) = crate::javascript::parser(&input, 0, false)
            .ok()
            .and_then(|mut p| p.parse_script().ok())
        else {
            return vec![];
        };
        let mut program = Program::Script(program);
        let mark = Mark::new();
        resolver(mark, Mark::new(), false).process(&mut program);
        let mut references = References {
            mark,
            prefix: prefix.len(),
            source: fragment.span,
            dynamic_this: 0,
            found: Vec::new(),
        };
        program.visit_with(&mut references);
        references.found
    })
}

struct References {
    mark: swc_common::Mark,
    prefix: usize,
    source: SourceSpan,
    dynamic_this: usize,
    found: Vec<ScriptReference>,
}
impl References {
    fn record(&mut self, span: swc_common::Span, receiver: Option<&str>, name: &str) {
        let range = crate::javascript::span(span);
        let Some(offset) = range.start.checked_sub(self.prefix) else {
            return;
        };
        let Some(start) = self.source.start.checked_add(offset) else {
            return;
        };
        let Some(end) = start.checked_add(range.end - range.start) else {
            return;
        };
        if end <= self.source.end {
            self.found.push(ScriptReference {
                span: SourceSpan { start, end },
                receiver: receiver.map(str::to_owned),
                name: name.into(),
            });
        }
    }
}
impl swc_ecma_visit::Visit for References {
    fn visit_ident(&mut self, id: &swc_ecma_ast::Ident) {
        if id.ctxt.has_mark(self.mark) {
            self.record(id.span, None, id.sym.as_str());
        }
    }
    fn visit_function(&mut self, func: &swc_ecma_ast::Function) {
        use swc_ecma_visit::VisitWith;
        self.dynamic_this += 1;
        func.visit_children_with(self);
        self.dynamic_this -= 1;
    }
    fn visit_member_expr(&mut self, member: &swc_ecma_ast::MemberExpr) {
        use swc_ecma_ast::{Expr, MemberProp};
        use swc_ecma_visit::VisitWith;
        member.visit_children_with(self);
        let MemberProp::Ident(prop) = &member.prop else {
            return;
        };
        let mut obj = member.obj.as_ref();
        while let Expr::Paren(p) = obj {
            obj = &p.expr;
        }
        let receiver = match obj {
            Expr::This(_) if self.dynamic_this == 0 => "this",
            Expr::Ident(id) if id.ctxt.has_mark(self.mark) => id.sym.as_str(),
            _ => return,
        };
        self.record(prop.span, Some(receiver), prop.sym.as_str());
    }
}
