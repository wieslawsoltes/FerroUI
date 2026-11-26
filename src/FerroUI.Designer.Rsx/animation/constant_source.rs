//! Typed values retained from the authored Cargo snapshot.

use super::color::ColorSource;
use super::color_integer::IntegerLiteral;
use super::number_source::FloatSource;
use crate::local_rust::RustSourceScope;
use std::collections::BTreeMap;
use syn::Expr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ConstantType {
    F64,
    U8,
    U32,
    Color,
}

impl ConstantType {
    pub(super) fn accepts(self, path: &str) -> bool {
        self.paths().contains(&path)
    }

    fn paths(self) -> &'static [&'static str] {
        match self {
            Self::F64 => &["f64", "core::primitive::f64", "std::primitive::f64"],
            Self::U8 => &["u8", "core::primitive::u8", "std::primitive::u8"],
            Self::U32 => &["u32", "core::primitive::u32", "std::primitive::u32"],
            Self::Color => &["ferroui_base::media::Color"],
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum ConstantValue {
    Float(f64),
    Integer(IntegerLiteral),
    Color([u8; 4]),
}

pub(super) type ConstantValues = BTreeMap<(String, ConstantType), ConstantValue>;

type Lookup<'a> = dyn Fn(&Expr, ConstantType) -> Option<ConstantValue> + 'a;

pub(super) struct ConstantReader<'a>(pub(super) &'a Lookup<'a>);

impl ConstantReader<'_> {
    pub(super) const LITERALS: Self = Self(&|_, _| None);

    pub(super) fn get(&self, path: &Expr, ty: ConstantType) -> Option<ConstantValue> {
        (self.0)(path, ty)
    }

    pub(super) fn float(&self, expression: &Expr) -> Option<FloatSource> {
        FloatSource::read(expression, self)
    }
}

pub(super) fn value_in_scope(
    path: &Expr,
    ty: ConstantType,
    scope: RustSourceScope<'_>,
) -> Option<ConstantValue> {
    let (initializer, owner) = scope.typed_constant_initializer(path, ty.paths())?;
    match ty {
        ConstantType::F64 => FloatSource::read(initializer, &ConstantReader::LITERALS)
            .map(|source| ConstantValue::Float(source.value)),
        ConstantType::U8 | ConstantType::U32 => {
            IntegerLiteral::read(initializer, ty).map(|(value, _)| ConstantValue::Integer(value))
        }
        ConstantType::Color => {
            // Alias chains were resolved above. Read a native color constructor
            // or a known swatch from the scope that owns the final initializer.
            if !matches!(initializer, Expr::Call(_) | Expr::Struct(_) | Expr::Path(_)) {
                return None;
            }
            let operands = ConstantReader(&|path, ty| match ty {
                ConstantType::U8 | ConstantType::U32 => value_in_scope(path, ty, owner),
                _ => None,
            });
            let (value, _) =
                ColorSource::read(initializer, false, &|path| owner.path(path), &operands)?;
            Some(ConstantValue::Color(value.argb8()?))
        }
    }
}
