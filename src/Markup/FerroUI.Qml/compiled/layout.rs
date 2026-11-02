//! Typed, read-only layout emitted by the Cargo compiler. The tables are Rust
//! constants. Materializing them allocates the same owned IR used by the source
//! loader, with one shared implementation for common member constructors.
use super::ScriptImportTables;
use crate::ir;

pub const PLAIN: ir::Modifiers = ir::Modifiers {
    required: false,
    readonly: false,
    default: false,
    final_: false,
    virtual_: false,
    override_: false,
};

/// The generated factory initializes the metadata shared by objects of this shape.
#[derive(Clone, Copy)]
pub struct Object(
    pub fn(Vec<ir::Member>, ir::SourceSpan) -> ir::Object,
    pub usize,
    pub usize,
    pub &'static [Member],
);

impl Object {
    pub fn build(&self, imports: &mut ScriptImportTables) -> ir::Object {
        let Self(shape, start, end, records) = *self;
        let members = records.iter().map(|record| record.build(imports)).collect();
        shape(members, ir::SourceSpan { start, end })
    }
}

/// Declaration scope and authored byte range.
#[derive(Clone, Copy)]
pub struct Site(pub Option<u32>, pub usize, pub usize);

#[derive(Clone, Copy)]
pub enum Value {
    Null,
    Undefined,
    Bool(bool),
    Number(u64),
    String(&'static str),
    Native(usize),
}

impl Value {
    fn binding(self) -> ir::Binding {
        ir::Binding::Expr(match self {
            Self::Null => ir::Expr::Null,
            Self::Undefined => ir::Expr::Undefined,
            Self::Bool(v) => ir::Expr::Bool(v),
            Self::Number(bits) => ir::Expr::Number(f64::from_bits(bits)),
            Self::String(s) => ir::Expr::String(s.into()),
            Self::Native(index) => ir::Expr::Native(index),
        })
    }
}

pub enum Member {
    Id(Site, &'static str),
    Binding(Site, &'static str, Value),
    Property(
        Site,
        &'static str,
        &'static str,
        ir::Modifiers,
        Option<Value>,
    ),
    Child(Site, Object),
    /// Other IR forms retain their compiler-generated typed constructor.
    Direct(fn(&mut ScriptImportTables) -> ir::Member),
}

impl Member {
    fn build(&self, imports: &mut ScriptImportTables) -> ir::Member {
        let (Site(scope, start, end), kind) = match *self {
            Self::Id(site, id) => (site, ir::MemberKind::Id(id.into())),
            Self::Binding(site, name, value) => (
                site,
                ir::MemberKind::Binding {
                    name: name.into(),
                    value: value.binding(),
                },
            ),
            Self::Property(site, name, ty, modifiers, value) => (
                site,
                ir::MemberKind::Property {
                    name: name.into(),
                    ty: ty.into(),
                    modifiers,
                    value: value.map(Value::binding),
                },
            ),
            Self::Child(site, object) => (site, ir::MemberKind::Child(object.build(imports))),
            Self::Direct(build) => return build(imports),
        };
        ir::Member {
            annotation_imports: None,
            scope,
            kind,
            span: ir::SourceSpan { start, end },
        }
    }
}
