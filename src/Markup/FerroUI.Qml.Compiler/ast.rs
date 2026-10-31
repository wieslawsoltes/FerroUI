//! Source-located component representation shared by the loader and compiler.
use serde::{Deserialize, Serialize};

/// Half-open byte range in the original UTF-8 source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Referenced module singletons, each privately scoped and owned by the engine.
    #[serde(default)]
    pub singletons: Vec<Object>,
    pub pragmas: Vec<Pragma>,
    pub imports: Vec<Import>,
    pub root: Object,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pragma {
    pub name: String,
    pub values: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Import {
    pub uri: String,
    pub is_path: bool,
    /// Decimal spelling is preserved, so 6.10 is distinct from 6.1.
    pub version: Option<String>,
    pub alias: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Object {
    /// Lexical property lookup entries introduced when flattening source inheritance.
    #[serde(default)]
    pub shadows: Vec<PropertyShadow>,
    /// Nominal component identities assigned by the linker, in inheritance order.
    pub composite_types: Vec<String>,
    /// Imports in force at the declaration site after linking external components.
    pub type_imports: Option<Vec<Import>>,
    pub type_name: String,
    pub members: Vec<Member>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertyShadow {
    pub scope: u32,
    pub name: String,
    pub storage: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Member {
    /// The source type that declared this member, if it came from a template.
    #[serde(default)]
    pub scope: Option<u32>,
    pub kind: MemberKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MemberKind {
    Id(String),
    Property {
        name: String,
        ty: String,
        modifiers: Modifiers,
        value: Option<Binding>,
    },
    Binding {
        name: String,
        value: Binding,
    },
    Child(Object),
    /// A property value source or interceptor, outside the default property list.
    OnProperty {
        name: String,
        object: Object,
    },
    Group {
        name: String,
        members: Vec<Member>,
    },
    Signal {
        name: String,
        parameters: Vec<Parameter>,
    },
    Function {
        name: String,
        parameters: Vec<Parameter>,
        return_type: Option<String>,
        body: Vec<Stmt>,
    },
    Handler {
        name: String,
        parameters: Vec<String>,
        body: Vec<Stmt>,
    },
    Component {
        name: String,
        object: Object,
    },
    Enum {
        name: String,
        values: Vec<(String, i32)>,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifiers {
    pub required: bool,
    pub readonly: bool,
    pub default: bool,
    pub final_: bool,
    pub virtual_: bool,
    pub override_: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Binding {
    Expr(Expr),
    Object(Object),
    Objects(Vec<Object>),
    Script(Vec<Stmt>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// Index in the native expression table of a compiled component.
    Native(usize),
    Null,
    Undefined,
    Bool(bool),
    Number(f64),
    String(String),
    Name(String),
    /// Unqualified lookup anchored to the declaring component after linking.
    ScopedName {
        name: String,
        root: String,
    },
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Array(Vec<Expr>),
    Record(Vec<(String, Expr)>),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    /// Index in the compiled module's native statement table.
    Native(usize),
    Expr(Expr),
    Assign {
        target: Expr,
        op: String,
        value: Expr,
    },
    Let {
        name: String,
        value: Option<Expr>,
    },
    Return(Option<Expr>),
    If {
        test: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    Block(Vec<Stmt>),
    While {
        test: Expr,
        body: Vec<Stmt>,
        post_test: bool,
    },
    For {
        init: Option<Box<Stmt>>,
        test: Option<Expr>,
        update: Option<Box<Stmt>>,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
}

/// A scalar initializer is a stored value. It has no binding to restore when a
/// conditional Binding stops overriding it.
pub enum Literal<'a> {
    Null,
    Undefined,
    Bool(bool),
    Number(f64),
    String(&'a str),
}

impl Expr {
    pub fn as_literal(&self) -> Option<Literal<'_>> {
        Some(match self {
            Self::Null => Literal::Null,
            Self::Undefined => Literal::Undefined,
            Self::Bool(b) => Literal::Bool(*b),
            Self::Number(n) => Literal::Number(*n),
            Self::String(s) => Literal::String(s),
            Self::Unary(op, operand) if op == "-" || op == "+" => {
                let Self::Number(n) = operand.as_ref() else {
                    return None;
                };
                Literal::Number(if op == "-" { -n } else { *n })
            }
            _ => return None,
        })
    }

    /// A name guaranteed to be read before any other property or call. This
    /// allows forward chains to be evaluated iteratively without speculating
    /// into a branch or re-running user code.
    pub fn first_read(&self) -> Option<(&str, Option<&str>)> {
        match self {
            Self::Name(name) => Some((name, None)),
            Self::ScopedName { name, root } => Some((name, Some(root))),
            Self::Unary(_, e) | Self::Conditional(e, ..) => e.first_read(),
            Self::Binary(op, a, b)
                if a.as_literal().is_some() && !matches!(op.as_str(), "&&" | "||" | "??") =>
            {
                b.first_read()
            }
            Self::Binary(_, a, _) => a.first_read(),
            _ => None,
        }
    }
}
