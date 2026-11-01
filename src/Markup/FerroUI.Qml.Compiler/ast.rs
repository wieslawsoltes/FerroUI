//! Source-located component representation shared by the loader and compiler.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Half-open byte range in the original UTF-8 source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Stable type identities indexed by this link unit's declaration scope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_keys: Vec<String>,
    /// JavaScript resources embedded from the local import graph during linking.
    #[serde(default)]
    pub scripts: BTreeMap<String, ScriptResource>,
    /// Referenced module singletons, each privately scoped and owned by the engine.
    #[serde(default)]
    pub singletons: Vec<Object>,
    /// Explicitly linked component entry points, keyed by relocatable project URL.
    #[serde(default)]
    pub components: BTreeMap<String, Object>,
    pub pragmas: Vec<Pragma>,
    pub imports: Vec<Import>,
    pub root: Object,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptForm {
    Expression,
    BindingBlock,
    FunctionBody,
    HandlerBody,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyRead {
    /// None for an unqualified name; otherwise this, parent, or a lexical object ID.
    pub receiver: Option<String>,
    pub property: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JavaScript {
    /// An exact read identified by the JS parser. The runtime must still
    /// prove the receiver and storage type before replacing this expression.
    #[serde(default)]
    pub property_read: Option<PropertyRead>,
    /// Generated invocation or import lowering, separate from the source the user wrote.
    pub executable: Option<String>,
    pub import_hook: Option<String>,
    /// Candidate QML singleton names collected by the ECMAScript parser.
    #[serde(default)]
    pub references: Vec<String>,
    pub form: ScriptForm,
    pub source: String,
    pub span: SourceSpan,
    pub origin: String,
    pub root: Option<String>,
    pub ids: BTreeMap<String, String>,
    /// Scripts in one lexical document share the immutable import namespace.
    pub imports: std::sync::Arc<BTreeMap<String, String>>,
}

impl JavaScript {
    pub fn code(&self) -> &str {
        self.executable.as_deref().unwrap_or(&self.source)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptKind {
    Module,
    Script,
    Library,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptResource {
    /// Executable JavaScript. ES modules use the System.register module ABI.
    pub executable: String,
    pub import_hook: Option<String>,
    /// True for a static script import in a QML document.
    pub preload: bool,
    /// Qualified .import bindings in a classic JavaScript resource.
    pub imports: BTreeMap<String, String>,
    pub source: String,
    pub kind: ScriptKind,
    pub dependencies: BTreeMap<String, String>,
    pub exports: Vec<String>,
    /// Writable classic script bindings exposed by Qt (var and function declarations).
    pub writable_exports: Vec<String>,
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

impl Import {
    pub fn is_script(&self) -> bool {
        let path = self.uri.split(['?', '#']).next().unwrap_or_default();
        self.is_path && (path.ends_with(".js") || path.ends_with(".mjs"))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Object {
    /// Virtual source location for declaration-relative resources.
    #[serde(default)]
    pub source_url: Option<String>,
    /// Declaration and instantiation URLs on a flattened composite root.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_layers: Vec<SourceLayer>,
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
pub struct SourceLayer {
    pub scope: Option<u32>,
    pub url: String,
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
    JavaScript(Box<JavaScript>),
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
    JavaScript(Box<JavaScript>),
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

impl Object {
    pub fn set_source_url(&mut self, url: &str) {
        fn visit(members: &mut [Member], url: &str) {
            for member in members {
                match &mut member.kind {
                    MemberKind::Component { object, .. }
                    | MemberKind::Child(object)
                    | MemberKind::OnProperty { object, .. } => object.set_source_url(url),
                    MemberKind::Group { members, .. } => visit(members, url),
                    MemberKind::Property {
                        value: Some(binding),
                        ..
                    }
                    | MemberKind::Binding { value: binding, .. } => match binding {
                        Binding::Object(child) => child.set_source_url(url),
                        Binding::Objects(items) => {
                            for item in items {
                                item.set_source_url(url)
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
        }
        self.source_url.get_or_insert_with(|| url.into());
        visit(&mut self.members, url);
    }
}
