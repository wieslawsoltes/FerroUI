pub mod binary_document;

// The FXD sources call the PR's parser entry point. Supply the current checkout's
// identical compatibility mappings without linking a second FerroUI compiler.
mod compiler_extensions {
    use std::collections::HashMap;
    use xamlx::{
        ast::XamlDocument, exceptions::XamlResult, parsers::XDocumentXamlParser, XamlNamespaces,
    };

    pub struct FerroXamlIlCompiler;
    impl FerroXamlIlCompiler {
        pub fn parse_source_syntax(xaml: &str) -> XamlResult<XamlDocument> {
            let dev = "https://github.com/ferroui/hot-reload";
            let mappings = HashMap::from([
                (
                    XamlNamespaces::BLEND2008.to_string(),
                    XamlNamespaces::BLEND2008.to_string(),
                ),
                (dev.to_owned(), dev.to_owned()),
            ]);
            XDocumentXamlParser::parse(xaml, Some(&mappings))
        }
    }
}

use binary_document::pack::{expand_solid, ThemePack};
use std::{collections::BTreeMap, marker::PhantomData, ops::Range};
use xamlx::ast::XamlDocument;

pub struct Archive<'a> {
    expanded: Vec<u8>,
    entries: BTreeMap<String, Range<usize>>,
    _source: PhantomData<&'a [u8]>,
}

impl<'a> Archive<'a> {
    pub fn open(image: &'a [u8]) -> Self {
        let expanded = expand_solid(image).unwrap();
        let index = ThemePack::open(&expanded).unwrap();
        let entries = index
            .paths()
            .map(|path| {
                let bytes = index.document(path).unwrap();
                let start = bytes.as_ptr() as usize - expanded.as_ptr() as usize;
                (path.to_owned(), start..start + bytes.len())
            })
            .collect();
        Self {
            expanded,
            entries,
            _source: PhantomData,
        }
    }

    pub fn document(&self, path: &str) -> XamlDocument {
        binary_document::decode(&self.expanded[self.entries[path].clone()]).unwrap()
    }

    pub fn verify(&self, root: &std::path::Path) -> usize {
        for path in self.entries.keys() {
            let xml = std::fs::read_to_string(root.join(path)).unwrap();
            let original =
                compiler_extensions::FerroXamlIlCompiler::parse_source_syntax(&xml).unwrap();
            let encode = binary_document::encode_stored;
            assert_eq!(
                encode(&original).unwrap(),
                encode(&self.document(path)).unwrap(),
                "{path}"
            );
        }
        self.entries.len()
    }
}
