//! Input shared by asset discovery, group caching and runtime compilation.
use std::{
    io::{self, Read},
    rc::Rc,
};
use xamlx::{ast::XamlDocument, exceptions::XamlResult, type_system::IXamlType};

use crate::{back_end::include_sources, compiler_extensions::FerroXamlIlCompiler};

pub(crate) enum DocumentInput {
    Xml(String),
    #[cfg(feature = "portable-xaml")]
    Portable {
        bytes: Vec<u8>,
        syntax: Option<XamlDocument>,
    },
}

impl DocumentInput {
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
                return Ok(Self::Portable {
                    bytes: prefix,
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
        Ok(Self::Xml(text))
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        match self {
            Self::Xml(text) => text.as_bytes(),
            #[cfg(feature = "portable-xaml")]
            Self::Portable { bytes, .. } => bytes,
        }
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub(crate) fn source_text(&self) -> Option<&str> {
        match self {
            Self::Xml(text) => Some(text),
            #[cfg(feature = "portable-xaml")]
            Self::Portable { .. } => None,
        }
    }

    pub(crate) fn into_text(self) -> io::Result<String> {
        match self {
            Self::Xml(text) => Ok(text),
            #[cfg(feature = "portable-xaml")]
            Self::Portable { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the asset contains FXD syntax rather than XML source",
            )),
        }
    }

    pub(crate) fn include_sources(&mut self) -> XamlResult<Vec<String>> {
        match self {
            Self::Xml(text) => Ok(include_sources(text)),
            #[cfg(feature = "portable-xaml")]
            Self::Portable { bytes, syntax } => {
                let parsed = crate::binary_document::decode(bytes)?;
                let sources = syntax_includes(&parsed)?;
                *syntax = Some(parsed);
                Ok(sources)
            }
        }
    }

    pub(crate) fn parse(
        &mut self,
        compiler: &FerroXamlIlCompiler,
        root: Option<Rc<dyn IXamlType>>,
    ) -> XamlResult<XamlDocument> {
        match self {
            Self::Xml(text) => compiler.parse(text, root),
            #[cfg(feature = "portable-xaml")]
            Self::Portable { bytes, syntax } => {
                let parsed = match syntax.take() {
                    Some(parsed) => parsed,
                    None => crate::binary_document::decode(bytes)?,
                };
                compiler.resolve_document_root(parsed, root)
            }
        }
    }
}

#[cfg(feature = "portable-xaml")]
fn syntax_includes(document: &XamlDocument) -> XamlResult<Vec<String>> {
    use xamlx::ast::{
        visit_node, IXamlAstNode, IXamlAstVisitor, XamlAstNamePropertyReference,
        XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode, XamlAstXamlPropertyValueNode,
        XamlAstXmlTypeReference,
    };
    fn literal_source(object: &XamlAstObjectNode) -> Option<String> {
        let ty = object.type_.borrow().cast::<XamlAstXmlTypeReference>()?;
        if !matches!(
            ty.name().as_str(),
            "MergeResourceInclude" | "ResourceInclude" | "StyleInclude"
        ) {
            return None;
        }
        object.children.borrow().iter().find_map(|child| {
            let property = child.cast::<XamlAstXamlPropertyValueNode>()?;
            let member = property.property().cast::<XamlAstNamePropertyReference>()?;
            if member.name() != "Source"
                || !member
                    .declaring_type
                    .borrow()
                    .equals(&**object.type_.borrow())
            {
                return None;
            }
            let values = property.values.borrow();
            let [value] = values.as_slice() else {
                return None;
            };
            let source = value.cast::<XamlAstTextNode>()?.text();
            (!source.is_empty()).then_some(source)
        })
    }

    struct Includes(Vec<String>);
    impl IXamlAstVisitor for Includes {
        fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
            if let Some(source) = node
                .cast::<XamlAstObjectNode>()
                .and_then(|object| literal_source(&object))
            {
                self.0.push(source);
            }
            Ok(node)
        }
        fn push(&mut self, _: Rc<dyn IXamlAstNode>) {}
        fn pop(&mut self) {}
    }
    let mut includes = Includes(Vec::new());
    visit_node(&document.root()?, &mut includes)?;
    Ok(includes.0)
}
