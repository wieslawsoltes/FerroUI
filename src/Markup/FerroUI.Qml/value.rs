use std::fmt;

/// A handle belongs to exactly one component instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId {
    pub(crate) instance: u64,
    pub(crate) index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Undefined,
    External(crate::ExternalValue),
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Object(ObjectId),
    ModelRow(crate::RowId),
    Record(crate::Record),
    Namespace(std::rc::Rc<crate::Namespace>),
    List(Vec<Value>),
}

impl Value {
    pub(crate) fn truthy(&self) -> bool {
        match self {
            Self::Undefined | Self::Null => false,
            Self::Bool(value) => *value,
            Self::Number(value) => *value != 0.0 && !value.is_nan(),
            Self::String(value) => !value.is_empty(),
            Self::External(_)
            | Self::ModelRow(_)
            | Self::Record(_)
            | Self::Object(_)
            | Self::Namespace(_)
            | Self::List(_) => true,
        }
    }

    pub(crate) fn number(&self) -> Result<f64, Error> {
        Ok(match self {
            Self::Number(n) => *n,
            Self::Bool(value) => f64::from(u8::from(*value)),
            Self::Null => 0.0,
            Self::Undefined => f64::NAN,
            Self::String(s) if s.trim().is_empty() => 0.0,
            Self::String(s) => s.trim().parse().unwrap_or(f64::NAN),
            Self::External(_)
            | Self::ModelRow(_)
            | Self::Record(_)
            | Self::Object(_)
            | Self::Namespace(_)
            | Self::List(_) => return Err(Error::new("expected a scalar number")),
        })
    }

    pub(crate) fn js_string(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Number(n) if n.is_infinite() => if n.is_sign_positive() {
                "Infinity"
            } else {
                "-Infinity"
            }
            .into(),
            Self::Number(n) if *n == 0.0 => "0".into(),
            Self::Number(n) => n.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Null => "null".into(),
            Self::Undefined => "undefined".into(),
            Self::External(_)
            | Self::ModelRow(_)
            | Self::Record(_)
            | Self::Object(_)
            | Self::Namespace(_) => "[object Object]".into(),
            Self::List(values) => values
                .iter()
                .map(|v| match v {
                    Self::Undefined | Self::Null => String::new(),
                    _ => v.js_string(),
                })
                .collect::<Vec<_>>()
                .join(","),
        }
    }

    pub(crate) fn unchanged(&self, rhs: &Self) -> bool {
        self == rhs
            || matches!((self,rhs), (Self::Number(a),Self::Number(b)) if a.is_nan() && b.is_nan())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.message)
    }
}
impl std::error::Error for Error {}

impl From<crate::ir::Literal<'_>> for Value {
    fn from(literal: crate::ir::Literal<'_>) -> Self {
        match literal {
            crate::ir::Literal::Null => Self::Null,
            crate::ir::Literal::Undefined => Self::Undefined,
            crate::ir::Literal::Bool(b) => Self::Bool(b),
            crate::ir::Literal::Number(n) => Self::Number(n),
            crate::ir::Literal::String(s) => Self::String(s.into()),
        }
    }
}
