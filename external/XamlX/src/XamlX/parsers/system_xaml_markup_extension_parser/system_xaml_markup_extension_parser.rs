//! Port of `Parsers/SystemXamlMarkupExtensionParser/SystemXamlMarkupExtensionParser.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstValueNode, IXamlLineInfo, XamlAstObjectNode, XamlAstTextNode,
    XamlAstXamlPropertyValueNode,
};
use crate::exceptions::XamlError;

use super::{
    MeScanner, MeScannerContext, MeScannerError, MeScannerParseException, MeScannerTypeName,
    MeScannerTypeResolver, MeTokenType,
};

pub struct SystemXamlMarkupExtensionParser;

struct Parser<'s, 'c, 'a> {
    li: &'s dyn IXamlLineInfo,
    scanner: MeScanner<'c, 'a>,
    type_resolver: MeScannerTypeResolver<'a>,
    current_type_stack: Vec<MeScannerTypeName>,
    depth: usize,
}

/// Markup extensions nested deeper than this are rejected instead of risking a stack overflow
/// (upstream has no such limit).
const MAX_EXTENSION_DEPTH: usize = 128;

fn unexpected_token(token: MeTokenType) -> MeScannerError {
    MeScannerParseException::new(format!("Unexpected token {token}")).into()
}

impl Parser<'_, '_, '_> {
    fn read_extension(&mut self) -> Result<Rc<dyn IXamlAstValueNode>, MeScannerError> {
        if self.scanner.token() != MeTokenType::Open {
            return Err(unexpected_token(self.scanner.token()));
        }

        self.scanner.read()?;
        if self.scanner.token() != MeTokenType::TypeName {
            return Err(unexpected_token(self.scanner.token()));
        }

        if self.current_type_stack.len() + self.depth >= MAX_EXTENSION_DEPTH {
            return Err(
                MeScannerParseException::new("Markup extensions are nested too deeply").into(),
            );
        }

        let ext_type = self.scanner.token_type().cloned().ok_or_else(|| {
            XamlError::internal(
                "NullReferenceException",
                "The scanner didn't produce a type name token",
            )
        })?;

        ext_type.type_reference.is_markup_extension.set(true);
        let previous_type =
            std::mem::replace(&mut self.scanner.context().current_type, ext_type.clone());
        self.current_type_stack.push(previous_type);
        let rv = XamlAstObjectNode::new(self.li, ext_type.type_reference.clone());

        loop {
            self.scanner.read()?;
            let token = self.scanner.token();
            if token == MeTokenType::Close {
                break;
            } else if token == MeTokenType::PropertyName {
                let prop = self.scanner.token_property().cloned().ok_or_else(|| {
                    XamlError::internal(
                        "NullReferenceException",
                        "The scanner didn't produce a property token",
                    )
                })?;
                self.scanner.read()?;
                if self.scanner.token() != MeTokenType::EqualSign {
                    return Err(unexpected_token(self.scanner.token()));
                }
                let prop_value = self.read()?;
                rv.children
                    .borrow_mut()
                    .push(XamlAstXamlPropertyValueNode::new(
                        self.li, prop, prop_value, true,
                    ));
            } else if token == MeTokenType::String
                || token == MeTokenType::QuotedMarkupExtension
                || token == MeTokenType::Open
            {
                if !rv.children.borrow().is_empty() {
                    return Err(MeScannerParseException::new(format!(
                        "Unexpected token after property list {token}"
                    ))
                    .into());
                }
                let argument = self.read_current()?;
                rv.arguments.borrow_mut().push(argument);
            } else if token == MeTokenType::Comma {
                continue;
            } else {
                return Err(unexpected_token(token));
            }
        }

        if let Some(previous_type) = self.current_type_stack.pop() {
            self.scanner.context().current_type = previous_type;
        }
        Ok(rv)
    }

    fn read(&mut self) -> Result<Rc<dyn IXamlAstValueNode>, MeScannerError> {
        self.scanner.read()?;
        self.read_current()
    }

    fn read_current(&mut self) -> Result<Rc<dyn IXamlAstValueNode>, MeScannerError> {
        match self.scanner.token() {
            MeTokenType::String => Ok(XamlAstTextNode::new(
                self.li,
                self.scanner.token_text(),
                true,
            )),
            MeTokenType::Open => self.read_extension(),
            MeTokenType::QuotedMarkupExtension => {
                let text = self.scanner.token_text().to_string();
                SystemXamlMarkupExtensionParser::parse_nested(
                    self.li,
                    &text,
                    self.type_resolver,
                    self.current_type_stack.len() + self.depth + 1,
                )
            }
            token => Err(unexpected_token(token)),
        }
    }
}

impl SystemXamlMarkupExtensionParser {
    pub fn parse(
        li: &dyn IXamlLineInfo,
        ext: &str,
        type_resolver: MeScannerTypeResolver<'_>,
    ) -> Result<Rc<dyn IXamlAstValueNode>, MeScannerError> {
        Self::parse_nested(li, ext, type_resolver, 0)
    }

    fn parse_nested(
        li: &dyn IXamlLineInfo,
        ext: &str,
        type_resolver: MeScannerTypeResolver<'_>,
        depth: usize,
    ) -> Result<Rc<dyn IXamlAstValueNode>, MeScannerError> {
        if depth >= MAX_EXTENSION_DEPTH {
            return Err(
                MeScannerParseException::new("Markup extensions are nested too deeply").into(),
            );
        }
        let mut ctx = MeScannerContext::new(type_resolver, li);
        let scanner = MeScanner::new(&mut ctx, ext, li.line(), li.position());
        let mut parser = Parser {
            li,
            scanner,
            type_resolver,
            current_type_stack: Vec::new(),
            depth,
        };
        parser.read()
    }
}
