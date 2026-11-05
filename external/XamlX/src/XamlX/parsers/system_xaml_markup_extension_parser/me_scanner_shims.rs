//! Port of `Parsers/SystemXamlMarkupExtensionParser/MeScanner.Shims.cs`.

use std::fmt;
use std::rc::Rc;

use crate::ast::{
    IXamlLineInfo, XamlAstNamePropertyReference, XamlAstXmlTypeReference, XamlLineInfo,
};
use crate::exceptions::{XamlError, XamlResult};

/// Resolves a (possibly prefixed) type name to an XML type reference.
pub type MeScannerTypeResolver<'a> = &'a dyn Fn(&str) -> XamlResult<Rc<XamlAstXmlTypeReference>>;

pub struct MeScannerContext<'a> {
    type_resolver: MeScannerTypeResolver<'a>,
    line_info: XamlLineInfo,
    pub current_bracket_mode_parse_parameters: MeScannerBracketModeParseParameters,
    pub current_type: MeScannerTypeName,
}

impl<'a> MeScannerContext<'a> {
    pub fn new(type_resolver: MeScannerTypeResolver<'a>, line_info: &dyn IXamlLineInfo) -> Self {
        Self {
            type_resolver,
            line_info: XamlLineInfo::new(line_info.line(), line_info.position())
                .with_namespace_scope(line_info.namespace_scope()),
            current_bracket_mode_parse_parameters: MeScannerBracketModeParseParameters::default(),
            current_type: MeScannerTypeName::new(XamlAstXmlTypeReference::new(
                line_info,
                Some("invalid"),
                "invalid",
            )),
        }
    }

    pub fn type_resolver(&self) -> MeScannerTypeResolver<'a> {
        self.type_resolver
    }

    pub fn resolve_property_name(
        &self,
        pname: &str,
    ) -> XamlResult<Rc<XamlAstNamePropertyReference>> {
        if let Some((type_name, property_name)) = pname.split_once('.') {
            let declaring_type = (self.type_resolver)(type_name)?;
            Ok(XamlAstNamePropertyReference::new(
                &self.line_info,
                declaring_type,
                property_name,
                self.current_type.type_reference.clone(),
            ))
        } else {
            Ok(XamlAstNamePropertyReference::new(
                &self.line_info,
                self.current_type.type_reference.clone(),
                pname,
                self.current_type.type_reference.clone(),
            ))
        }
    }
}

#[derive(Clone)]
pub struct MeScannerTypeName {
    pub type_reference: Rc<XamlAstXmlTypeReference>,
}

impl MeScannerTypeName {
    pub fn new(type_reference: Rc<XamlAstXmlTypeReference>) -> Self {
        Self { type_reference }
    }

    pub fn parse_internal(
        long_name: &str,
        context: &MeScannerContext<'_>,
    ) -> XamlResult<MeScannerTypeName> {
        Ok(MeScannerTypeName::new((context.type_resolver())(
            long_name,
        )?))
    }

    pub fn name(&self) -> String {
        self.type_reference.name()
    }

    pub fn namespace(&self) -> Option<String> {
        self.type_reference.xml_namespace()
    }

    /// Whether the type is the type of a markup extension. The scanner only knows the types it
    /// was given by the parser, which marks the type of every extension it reads.
    pub fn is_markup_extension(&self) -> bool {
        self.type_reference.is_markup_extension.get()
    }

    /// Whether the type is `x:Bind` (the name `Bind` in the XAML language namespace) or the
    /// markup extension it is mapped onto (`XBind` / `XBindExtension`).
    pub fn is_x_bind(&self) -> bool {
        let name = self.type_reference.name();
        match name.as_str() {
            "Bind" => self.type_reference.xml_namespace().as_deref() == Some(crate::XamlNamespaces::XAML2006),
            "XBind" | "XBindExtension" => true,
            _ => false,
        }
    }
}

pub struct MeScannerSRID;

impl MeScannerSRID {
    pub const UNEXPECTED_TOKEN_AFTER_ME: &'static str = "Unexpected token after Markup Extension";
    pub const MALFORMED_BRACKET_CHARACTERS: &'static str = "Malformed bracket characters: {0}";
    pub const UNCLOSED_QUOTE: &'static str = "Unclosed quote";
    pub const QUOTE_CHARACTERS_OUT_OF_PLACE: &'static str = "Quote characters out of place";
    pub const INVALID_CLOSING_BRACKET_CHARACERS: &'static str =
        "Invalid closing bracket characters: {0}";
    pub const MALFORMED_PROPERTY_NAME: &'static str = "Malformed property name";
}

pub struct MeScannerSr;

impl MeScannerSr {
    pub fn get(error: &str) -> String {
        error.to_string()
    }

    pub fn get_with_arg(error: &str, arg: &str) -> String {
        error.replace("{0}", arg)
    }
}

/// `MeScannerParseException`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeScannerParseException {
    pub message: String,
}

impl MeScannerParseException {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            message: error.into(),
        }
    }
}

impl fmt::Display for MeScannerParseException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MeScannerParseException {}

/// What parsing a markup extension can fail with: a scanner/parser error, or an error raised
/// by the type resolver callback.
#[derive(Debug, Clone)]
pub enum MeScannerError {
    Parse(MeScannerParseException),
    Xaml(XamlError),
}

impl From<MeScannerParseException> for MeScannerError {
    fn from(value: MeScannerParseException) -> Self {
        MeScannerError::Parse(value)
    }
}

impl From<XamlError> for MeScannerError {
    fn from(value: XamlError) -> Self {
        MeScannerError::Xaml(value)
    }
}

impl fmt::Display for MeScannerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeScannerError::Parse(e) => e.fmt(f),
            MeScannerError::Xaml(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for MeScannerError {}

pub struct MeScannerBracketModeParseParameters {
    pub is_constructor_parsing_mode: bool,
    pub current_constructor_param: i32,
    pub max_constructor_params: i32,
    pub is_bracket_escape_mode: bool,
    pub bracket_character_stack: Vec<char>,
}

impl Default for MeScannerBracketModeParseParameters {
    fn default() -> Self {
        Self {
            is_constructor_parsing_mode: true,
            current_constructor_param: 0,
            max_constructor_params: i32::MAX,
            is_bracket_escape_mode: false,
            bracket_character_stack: Vec::new(),
        }
    }
}

/// The bracket characters of a markup extension property (the `SpecialBracketCharacters` of
/// System.Xaml): inside a pair of them a value is read as it is, separators included, so that
/// `{x:Bind Format(Name, 'a, b'), Mode=OneWay}` keeps the function call in one value.
///
/// System.Xaml reads them from the `MarkupExtensionBracketCharacters` attribute of a property;
/// markup extension types are resolved after the initial AST is parsed, so only the one
/// property WinUI declares them for is known: the `Path` of `x:Bind`, with `()` and `[]`
/// (`ProxyDirectUIXamlMember.LookupMarkupExtensionBracketCharacters` of WinUI's XAML compiler).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeScannerSpecialBracketCharacters {
    start_characters: Vec<char>,
    end_characters: Vec<char>,
}

impl MeScannerSpecialBracketCharacters {
    /// The opening and closing characters, in pairs.
    pub fn new(pairs: &[(char, char)]) -> Self {
        Self {
            start_characters: pairs.iter().map(|(start, _)| *start).collect(),
            end_characters: pairs.iter().map(|(_, end)| *end).collect(),
        }
    }

    /// The bracket characters of the property `property_name` of the markup extension
    /// `extension` (`None` is its first positional parameter): `()` and `[]` for the path of
    /// `x:Bind`, none for every other property.
    pub fn for_property(extension: &MeScannerTypeName, property_name: Option<&str>) -> Option<Self> {
        let is_path = matches!(property_name, None | Some("Path"));
        (is_path && extension.is_x_bind()).then(|| Self::new(&[('(', ')'), ('[', ']')]))
    }

    pub fn starts_escape_sequence(&self, ch: char) -> Result<bool, MeScannerError> {
        Ok(self.start_characters.contains(&ch))
    }

    pub fn ends_escape_sequence(&self, ch: char) -> Result<bool, MeScannerError> {
        Ok(self.end_characters.contains(&ch))
    }

    /// Whether `ch` closes the bracket `peek` opened.
    pub fn r#match(&self, peek: char, ch: char) -> Result<bool, MeScannerError> {
        let start = self.start_characters.iter().position(|c| *c == peek);
        let end = self.end_characters.iter().position(|c| *c == ch);
        Ok(start.is_some() && start == end)
    }
}
