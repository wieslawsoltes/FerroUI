//! Port of `Exceptions.cs`.
//!
//! Upstream throws `XamlParseException` (derived from `System.Xml.XmlException`),
//! `XamlTransformException`, `XamlLoadException` and `XamlTypeSystemException`.
//! The port maps them to a single error enum returned through [`XamlResult`].

use std::fmt;

use crate::ast::IXamlLineInfo;

/// Payload shared by all `XmlException`-derived errors (message plus line info).
#[derive(Debug, Clone)]
pub struct XamlParseException {
    /// The message as passed to the constructor (without the line info suffix).
    pub title: String,
    pub line_number: i32,
    pub line_position: i32,
    pub document: Option<String>,
    pub inner_exception: Option<Box<XamlError>>,
    /// The .NET type name of a class a host derives from `XamlParseException`,
    /// `XamlTransformException` or `XamlLoadException` (C# `exception.GetType().Name`).
    /// `None` for the exception types of this crate. Hosts tag their derived exceptions with
    /// [`XamlError::with_derived_type_name`] and test them with [`XamlError::is_derived_type`],
    /// which is what a C# type pattern (`e is XamlFooException`) on the derived class does.
    pub derived_type_name: Option<&'static str>,
}

impl XamlParseException {
    pub fn new(
        message: impl Into<String>,
        line: i32,
        position: i32,
        inner: Option<XamlError>,
    ) -> Self {
        Self {
            title: message.into(),
            line_number: line,
            line_position: position,
            document: None,
            inner_exception: inner.map(Box::new),
            derived_type_name: None,
        }
    }

    /// `XmlException.Message`: the user message followed by the position when it is known.
    pub fn message(&self) -> String {
        if self.line_number == 0 && self.line_position == 0 {
            self.title.clone()
        } else {
            format!(
                "{} Line {}, position {}.",
                self.title, self.line_number, self.line_position
            )
        }
    }
}

/// `XamlTypeSystemException` payload.
#[derive(Debug, Clone)]
pub struct XamlTypeSystemException {
    pub message: String,
    pub inner_exception: Option<Box<XamlError>>,
}

/// Any other exception type the upstream code may raise (`InvalidOperationException`,
/// `ArgumentException`, `InvalidCastException`, ...). These never carry line info.
#[derive(Debug, Clone)]
pub struct XamlInternalException {
    /// .NET exception type name, kept for diagnostics.
    pub type_name: &'static str,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum XamlError {
    /// A raw `System.Xml.XmlException` (malformed XML).
    Xml(XamlParseException),
    /// `XamlParseException`.
    Parse(XamlParseException),
    /// `XamlTransformException`.
    Transform(XamlParseException),
    /// `XamlLoadException`.
    Load(XamlParseException),
    /// `XamlTypeSystemException`.
    TypeSystem(XamlTypeSystemException),
    /// Any other exception.
    Internal(XamlInternalException),
    /// `AggregateException`.
    Aggregate(Vec<XamlError>),
}

pub type XamlResult<T> = Result<T, XamlError>;

fn li(line_info: Option<&dyn IXamlLineInfo>) -> (i32, i32) {
    line_info
        .map(|l| (l.line(), l.position()))
        .unwrap_or((0, 0))
}

impl XamlError {
    pub fn xml_exception(message: impl Into<String>, line: i32, position: i32) -> Self {
        XamlError::Xml(XamlParseException::new(message, line, position, None))
    }

    pub fn parse_exception_at(message: impl Into<String>, line: i32, position: i32) -> Self {
        XamlError::Parse(XamlParseException::new(message, line, position, None))
    }

    pub fn parse_exception(
        message: impl Into<String>,
        line_info: Option<&dyn IXamlLineInfo>,
    ) -> Self {
        let (l, p) = li(line_info);
        XamlError::Parse(XamlParseException::new(message, l, p, None))
    }

    pub fn parse_exception_with_inner(
        message: impl Into<String>,
        line_info: Option<&dyn IXamlLineInfo>,
        inner: XamlError,
    ) -> Self {
        let (l, p) = li(line_info);
        XamlError::Parse(XamlParseException::new(message, l, p, Some(inner)))
    }

    pub fn transform_exception(
        message: impl Into<String>,
        line_info: Option<&dyn IXamlLineInfo>,
    ) -> Self {
        let (l, p) = li(line_info);
        XamlError::Transform(XamlParseException::new(message, l, p, None))
    }

    pub fn transform_exception_with_inner(
        message: impl Into<String>,
        line_info: Option<&dyn IXamlLineInfo>,
        inner: XamlError,
    ) -> Self {
        let (l, p) = li(line_info);
        XamlError::Transform(XamlParseException::new(message, l, p, Some(inner)))
    }

    pub fn load_exception(
        message: impl Into<String>,
        line_info: Option<&dyn IXamlLineInfo>,
    ) -> Self {
        let (l, p) = li(line_info);
        XamlError::Load(XamlParseException::new(message, l, p, None))
    }

    pub fn type_system_exception(message: impl Into<String>) -> Self {
        XamlError::TypeSystem(XamlTypeSystemException {
            message: message.into(),
            inner_exception: None,
        })
    }

    pub fn internal(type_name: &'static str, message: impl Into<String>) -> Self {
        XamlError::Internal(XamlInternalException {
            type_name,
            message: message.into(),
        })
    }

    pub fn invalid_operation(message: impl Into<String>) -> Self {
        Self::internal("InvalidOperationException", message)
    }

    pub fn argument(message: impl Into<String>) -> Self {
        Self::internal("ArgumentException", message)
    }

    pub fn invalid_cast(message: impl Into<String>) -> Self {
        Self::internal("InvalidCastException", message)
    }

    pub fn not_supported(message: impl Into<String>) -> Self {
        Self::internal("NotSupportedException", message)
    }

    /// `e is XmlException`.
    pub fn is_xml_exception(&self) -> bool {
        matches!(
            self,
            XamlError::Xml(_) | XamlError::Parse(_) | XamlError::Transform(_) | XamlError::Load(_)
        )
    }

    /// `e is XamlParseException`.
    pub fn is_xaml_parse_exception(&self) -> bool {
        matches!(
            self,
            XamlError::Parse(_) | XamlError::Transform(_) | XamlError::Load(_)
        )
    }

    pub fn is_type_system_exception(&self) -> bool {
        matches!(self, XamlError::TypeSystem(_))
    }

    /// The `XmlException` payload, when this error derives from `XmlException`.
    pub fn as_xml_exception(&self) -> Option<&XamlParseException> {
        match self {
            XamlError::Xml(e)
            | XamlError::Parse(e)
            | XamlError::Transform(e)
            | XamlError::Load(e) => Some(e),
            _ => None,
        }
    }

    fn as_xml_exception_mut(&mut self) -> Option<&mut XamlParseException> {
        match self {
            XamlError::Xml(e)
            | XamlError::Parse(e)
            | XamlError::Transform(e)
            | XamlError::Load(e) => Some(e),
            _ => None,
        }
    }

    /// Marks this error as an instance of a host class derived from the exception type it was
    /// created as (e.g. a `XamlTransformException` subclass). No effect on errors that do not
    /// derive from `XmlException`.
    pub fn with_derived_type_name(mut self, type_name: &'static str) -> Self {
        if let Some(e) = self.as_xml_exception_mut() {
            e.derived_type_name = Some(type_name);
        }
        self
    }

    /// The name set by [`XamlError::with_derived_type_name`].
    pub fn derived_type_name(&self) -> Option<&'static str> {
        self.as_xml_exception().and_then(|e| e.derived_type_name)
    }

    /// `e is TDerived` for a host exception class named `type_name`.
    pub fn is_derived_type(&self, type_name: &str) -> bool {
        self.derived_type_name() == Some(type_name)
    }

    /// .NET exception type name.
    pub fn type_name(&self) -> &'static str {
        if let Some(derived) = self.derived_type_name() {
            return derived;
        }
        match self {
            XamlError::Xml(_) => "XmlException",
            XamlError::Parse(_) => "XamlParseException",
            XamlError::Transform(_) => "XamlTransformException",
            XamlError::Load(_) => "XamlLoadException",
            XamlError::TypeSystem(_) => "XamlTypeSystemException",
            XamlError::Internal(e) => e.type_name,
            XamlError::Aggregate(_) => "AggregateException",
        }
    }

    /// `Exception.Message`.
    pub fn message(&self) -> String {
        match self {
            XamlError::Xml(e)
            | XamlError::Parse(e)
            | XamlError::Transform(e)
            | XamlError::Load(e) => e.message(),
            XamlError::TypeSystem(e) => e.message.clone(),
            XamlError::Internal(e) => e.message.clone(),
            XamlError::Aggregate(errors) => {
                let inner: Vec<String> = errors
                    .iter()
                    .map(|e| format!("({})", e.message()))
                    .collect();
                format!("One or more errors occurred. {}", inner.join(" "))
            }
        }
    }

    pub fn line_number(&self) -> Option<i32> {
        self.as_xml_exception().map(|e| e.line_number)
    }

    pub fn line_position(&self) -> Option<i32> {
        self.as_xml_exception().map(|e| e.line_position)
    }

    /// `XamlParseException.Document` (only present on `XamlParseException` and derived).
    pub fn document(&self) -> Option<&str> {
        match self {
            XamlError::Parse(e) | XamlError::Transform(e) | XamlError::Load(e) => {
                e.document.as_deref()
            }
            _ => None,
        }
    }

    /// Object-initializer `{ Document = ... }`.
    pub fn with_document(mut self, document: Option<String>) -> Self {
        if let Some(e) = self.as_xml_exception_mut() {
            e.document = document;
        }
        self
    }

    pub fn inner_exception(&self) -> Option<&XamlError> {
        match self {
            XamlError::Xml(e)
            | XamlError::Parse(e)
            | XamlError::Transform(e)
            | XamlError::Load(e) => e.inner_exception.as_deref(),
            XamlError::TypeSystem(e) => e.inner_exception.as_deref(),
            _ => None,
        }
    }
}

impl fmt::Display for XamlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.type_name(), self.message())
    }
}

impl std::error::Error for XamlError {}
