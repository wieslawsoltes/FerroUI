//! Port of `Markup/Parsers/SelectorParser.cs`: creates a selector from its
//! text at run time.

use super::{SelectorGrammar, SelectorSyntax};
use ferroui_base::data::core::{ExpressionParseException, ValueType, ValueTypes};
use ferroui_base::styling::{Selector, Selectors};
use ferroui_base::{BoxedValue, FerroProperty, FerroPropertyRegistry, TypeInfo};
use std::fmt;
use std::rc::Rc;

/// What [`SelectorParser::parse`] fails with: the exceptions the managed
/// original throws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorParserError {
    /// The text is not a selector (`ExpressionParseException`).
    Parse(ExpressionParseException),
    /// The selector names a type, a property or a value that does not exist
    /// or does not apply (`InvalidOperationException`).
    InvalidOperation(String),
    /// The selector cannot be created from this syntax
    /// (`NotSupportedException`).
    NotSupported(String),
}

impl fmt::Display for SelectorParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectorParserError::Parse(error) => error.fmt(f),
            SelectorParserError::InvalidOperation(message) | SelectorParserError::NotSupported(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for SelectorParserError {}

impl From<ExpressionParseException> for SelectorParserError {
    fn from(error: ExpressionParseException) -> Self {
        SelectorParserError::Parse(error)
    }
}

/// Resolves the type a selector names: the namespace prefix (empty for
/// none) and the type name.
pub type SelectorTypeResolver = dyn Fn(&str, &str) -> Option<&'static TypeInfo>;

/// Parses a [`Selector`] from text.
pub struct SelectorParser {
    type_resolver: Box<SelectorTypeResolver>,
}

impl SelectorParser {
    /// Creates a parser.
    ///
    /// `type_resolver` resolves the types the selector names: it is given
    /// the namespace prefix and the type name.
    pub fn new(type_resolver: impl Fn(&str, &str) -> Option<&'static TypeInfo> + 'static) -> Self {
        Self { type_resolver: Box::new(type_resolver) }
    }

    /// Parses a [`Selector`] from text. `None` for an empty selector.
    pub fn parse(&self, s: &str) -> Result<Option<Selector>, SelectorParserError> {
        let syntax = SelectorGrammar::parse(s)?;
        self.create(&syntax)
    }

    fn create(&self, syntax: &[SelectorSyntax]) -> Result<Option<Selector>, SelectorParserError> {
        let mut result: Option<Selector> = None;
        let mut results: Option<Vec<Selector>> = None;

        for i in syntax {
            match i {
                SelectorSyntax::OfType { type_name, xmlns } => {
                    result = Some(Selectors::of_type_info(result, self.resolve(xmlns, type_name)?));
                }
                SelectorSyntax::Is { type_name, xmlns } => {
                    result = Some(Selectors::is_type_info(result, self.resolve(xmlns, type_name)?));
                }
                SelectorSyntax::Class { class } => {
                    result = Some(Selectors::class(result, class));
                }
                SelectorSyntax::Name { name } => {
                    result = Some(Selectors::name(result, name));
                }
                SelectorSyntax::Property { property, value } => {
                    let Some(type_) = result.as_ref().and_then(Selector::target_type) else {
                        return Err(SelectorParserError::InvalidOperation(
                            "Property selectors must be applied to a type.".to_string(),
                        ));
                    };

                    let Some(target_property) = FerroPropertyRegistry::instance().find_registered(type_, property)
                    else {
                        return Err(SelectorParserError::InvalidOperation(format!(
                            "Cannot find '{property}' on '{type_}"
                        )));
                    };

                    match try_convert(target_property, value) {
                        Some(typed_value) => {
                            result = Some(Selectors::property_equals_untyped(result, target_property, typed_value));
                        }
                        None => {
                            return Err(SelectorParserError::InvalidOperation(format!(
                                "Could not convert '{value}' to '{}",
                                target_property.property_type_name()
                            )));
                        }
                    }
                }
                SelectorSyntax::AttachedProperty { xmlns, type_name, property, value } => {
                    let Some(target_type) = result.as_ref().and_then(Selector::target_type) else {
                        return Err(SelectorParserError::InvalidOperation(
                            "Attached Property selectors must be applied to a type.".to_string(),
                        ));
                    };
                    // As in the managed original, a type that does not resolve is reported
                    // by `resolve`, before the check of the owner type.
                    let attached_property_owner_type = self.resolve(xmlns, type_name)?;

                    let registered = FerroPropertyRegistry::instance().get_registered_attached(target_type);
                    let target_attached_property = registered
                        .iter()
                        .copied()
                        .find(|ap| std::ptr::eq(ap.owner_type(), attached_property_owner_type) && ap.name() == property);
                    let Some(target_attached_property) = target_attached_property else {
                        return Err(SelectorParserError::InvalidOperation(format!(
                            "Cannot find '{property}' on '{attached_property_owner_type}"
                        )));
                    };

                    match try_convert(target_attached_property, value) {
                        Some(typed_value) => {
                            result =
                                Some(Selectors::property_equals_untyped(result, target_attached_property, typed_value));
                        }
                        None => {
                            return Err(SelectorParserError::InvalidOperation(format!(
                                "Could not convert '{value}' to '{}",
                                target_attached_property.property_type_name()
                            )));
                        }
                    }
                }
                SelectorSyntax::Child => {
                    result = Some(Selectors::child(result));
                }
                SelectorSyntax::Descendant => {
                    result = Some(Selectors::descendant(result));
                }
                SelectorSyntax::Template => {
                    result = Some(Selectors::template(result));
                }
                SelectorSyntax::Not { argument } => {
                    // The managed original dereferences the selector of the argument: an
                    // empty argument is a null reference there.
                    let argument = self
                        .create(argument)?
                        .ok_or_else(|| SelectorParserError::NotSupported("Invalid selector!".to_string()))?;
                    result = Some(Selectors::not(result, argument));
                }
                SelectorSyntax::NthChild { offset, step } => {
                    result = Some(Selectors::nth_child(result, *step, *offset));
                }
                SelectorSyntax::NthLastChild { offset, step } => {
                    result = Some(Selectors::nth_last_child(result, *step, *offset));
                }
                SelectorSyntax::Comma => {
                    let selector = result
                        .take()
                        .ok_or_else(|| SelectorParserError::NotSupported("Invalid selector!".to_string()))?;
                    results.get_or_insert_with(Vec::new).push(selector);
                }
                SelectorSyntax::Nesting => {
                    return Err(SelectorParserError::NotSupported(
                        "Unsupported selector grammar 'NestingSyntax'.".to_string(),
                    ));
                }
            }
        }

        if let Some(mut results) = results {
            if let Some(result) = result.take() {
                results.push(result);
            }

            result = if results.len() > 1 { Some(Selectors::or(results)) } else { results.into_iter().next() };
        }

        Ok(result)
    }

    fn resolve(&self, xmlns: &str, type_name: &str) -> Result<&'static TypeInfo, SelectorParserError> {
        match (self.type_resolver)(xmlns, type_name) {
            Some(result) => Ok(result),
            None => {
                let type_ =
                    if xmlns.trim().is_empty() { type_name.to_string() } else { format!("{xmlns}:{type_name}") };
                Err(SelectorParserError::InvalidOperation(format!("Could not resolve type '{type_}'")))
            }
        }
    }
}

/// `TypeUtilities.TryConvert(property.PropertyType, value, ..)`: the text of
/// a property selector as a value of the type of the property.
fn try_convert(property: &'static FerroProperty, value: &str) -> Option<BoxedValue> {
    let text: BoxedValue = Rc::new(value.to_string());
    let target = ValueType::new(property.property_type(), property.property_type_name());
    ValueTypes::try_convert(Some(&text), target).flatten()
}
