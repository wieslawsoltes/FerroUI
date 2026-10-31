//! Source-located component representation shared by the loader and compiler.
use serde::{Deserialize, Serialize};

/// Half-open byte range in the original UTF-8 source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub pragmas: Vec<Pragma>,
    pub imports: Vec<Import>,
    pub root: Object,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pragma {
    pub name: String,
    pub values: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Import {
    pub uri: String,
    pub is_path: bool,
    /// Decimal spelling is preserved, so 6.10 is distinct from 6.1.
    pub version: Option<String>,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Object {
    /// Imports in force at the declaration site after linking external components.
    pub type_imports: Option<Vec<Import>>,
    pub type_name: String,
    pub members: Vec<Member>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub kind: MemberKind,
    pub span: Span,
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
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
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
}
