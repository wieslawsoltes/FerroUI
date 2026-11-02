//! Input shared by asset discovery, group caching and runtime compilation.
use std::{
    io::{self, Read},
    rc::Rc,
};
use xamlx::{ast::XamlDocument, exceptions::XamlResult, type_system::IXamlType};

use crate::{
    back_end::{may_include, syntax_includes},
    compiler_extensions::FerroXamlIlCompiler,
};

enum DocumentContent {
    Xml(String),
    #[cfg(feature = "portable-xaml")]
    Portable(Vec<u8>),
}

pub(crate) struct DocumentInput {
    content: DocumentContent,
    // Discovery may already have parsed this exact source. Consume that tree once
    // when resolving the root, which mutates type references for this load.
    syntax: Option<XamlDocument>,
}

impl DocumentInput {
    pub(crate) fn xml(text: String) -> Self {
        Self {
            content: DocumentContent::Xml(text),
            syntax: None,
        }
    }

    #[cfg(feature = "portable-xaml")]
    pub(crate) fn from_portable(bytes: &[u8]) -> Self {
        Self { content: DocumentContent::Portable(bytes.to_vec()), syntax: None }
    }

    pub(crate) fn read(stream: &mut dyn Read) -> io::Result<Self> {
        #[cfg(feature = "portable-xaml")]
        {
            let mut prefix = Vec::with_capacity(4);
            (&mut *stream).take(4).read_to_end(&mut prefix)?;
            if crate::binary_document::is_binary(&prefix) {
                let limit = crate::binary_document::MAX_DOCUMENT_BYTES;
                stream
                    .take((limit - prefix.len() + 1) as u64)
                    .read_to_end(&mut prefix)?;
                if prefix.len() > limit {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "FXD document exceeds size limit",
                    ));
                }
                return Ok(Self {
                    content: DocumentContent::Portable(prefix),
                    syntax: None,
                });
            }
            // The prefix may end in the middle of a UTF-8 code point.
            return Self::read_xml(&mut io::Cursor::new(prefix).chain(stream));
        }
        #[cfg(not(feature = "portable-xaml"))]
        Self::read_xml(stream)
    }

    fn read_xml(stream: &mut dyn Read) -> io::Result<Self> {
        let mut text = String::new();
        stream.read_to_string(&mut text)?;
        Ok(Self::xml(text))
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        match &self.content {
            DocumentContent::Xml(text) => text.as_bytes(),
            #[cfg(feature = "portable-xaml")]
            DocumentContent::Portable(bytes) => bytes,
        }
    }

    /// Admission cost for a successfully transformed source. Compression must not
    /// permit a large syntax image to bypass the group's bounded edit history.
    pub(crate) fn cache_weight(&self) -> usize {
        #[cfg(feature = "portable-xaml")]
        if let DocumentContent::Portable(bytes) = &self.content {
            return crate::binary_document::expanded_size(bytes).unwrap_or(usize::MAX);
        }
        self.bytes().len()
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub(crate) fn source_text(&self) -> Option<&str> {
        match &self.content {
            DocumentContent::Xml(text) => Some(text),
            #[cfg(feature = "portable-xaml")]
            DocumentContent::Portable(_) => None,
        }
    }

    pub(crate) fn into_text(self) -> io::Result<String> {
        match self.content {
            DocumentContent::Xml(text) => Ok(text),
            #[cfg(feature = "portable-xaml")]
            DocumentContent::Portable(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "this asset contains binary FXD syntax",
            )),
        }
    }

    fn syntax(&self) -> XamlResult<XamlDocument> {
        match &self.content {
            DocumentContent::Xml(text) => FerroXamlIlCompiler::parse_source_syntax(text),
            #[cfg(feature = "portable-xaml")]
            DocumentContent::Portable(bytes) => crate::binary_document::decode(bytes),
        }
    }

    pub(crate) fn include_sources(&mut self) -> XamlResult<Vec<String>> {
        match &self.content {
            DocumentContent::Xml(text) if !may_include(text) => return Ok(Vec::new()),
            _ => {}
        }
        let syntax = self.syntax()?;
        let sources = syntax_includes(&syntax)?;
        self.syntax = Some(syntax);
        Ok(sources)
    }

    pub(crate) fn parse(
        &mut self,
        compiler: &FerroXamlIlCompiler,
        root: Option<Rc<dyn IXamlType>>,
    ) -> XamlResult<XamlDocument> {
        let syntax = match self.syntax.take() {
            Some(syntax) => syntax,
            None => self.syntax()?,
        };
        compiler.resolve_document_root(syntax, root)
    }
}

#[cfg(all(test, feature = "portable-xaml"))]
mod tests {
    use super::*;
    #[test]
    fn cache_charges_the_expanded_document() {
        let text = "abcd".repeat(100_000);
        let xml = format!("<TextBlock xmlns='https://github.com/ferroui' Text='{text}'/>");
        let syntax = FerroXamlIlCompiler::parse_source_syntax(&xml).unwrap();
        let stored = crate::binary_document::encode_stored(&syntax).unwrap();
        let compressed = crate::binary_document::encode(&syntax).unwrap();
        assert!(compressed.len() < stored.len());
        let input = DocumentInput::read(&mut &compressed[..]).unwrap();
        assert_eq!(input.cache_weight(), stored.len());
    }
}
