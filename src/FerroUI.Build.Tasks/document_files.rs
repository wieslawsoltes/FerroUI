use ferroui_build_scan::compiled_index::CompiledXamlInput;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// A compiled XAML input with a stable key and a separate physical path.
#[derive(Clone, Copy, Debug)]
pub struct XamlDocumentFile<'a> {
    key: &'a str,
    path: &'a str,
    rust_file: &'static str,
    line: u32,
    column: u32,
}

impl<'a> XamlDocumentFile<'a> {
    #[doc(hidden)]
    pub fn at(
        key: &'a str,
        path: &'a str,
        rust_file: &'static str,
        line: u32,
        column: u32,
    ) -> Self {
        Self {
            key,
            path,
            rust_file,
            line,
            column,
        }
    }

    /// The logical key can be used to select class roots or assemble groups.
    pub fn key(self) -> &'a str {
        self.key
    }
}

/// Declare a compiled XAML input for [`crate::XamlGroup::document_files`].
/// The key is its address within the assembly. The path is relative to the
/// Cargo package directory unless absolute. Use a path literal for IDE moves.
#[macro_export]
macro_rules! xaml_document_file {
    ($key:expr, $path:expr $(,)?) => {
        $crate::XamlDocumentFile::at(
            $key,
            $path,
            ::core::file!(),
            ::core::line!(),
            ::core::column!(),
        )
    };
}

pub(super) enum GroupDocuments {
    Inline(Vec<(String, String)>),
    Files(Vec<(String, CompiledXamlInput)>),
}

impl GroupDocuments {
    pub(super) fn from_files(files: &[XamlDocumentFile<'_>]) -> Self {
        Self::Files(
            files
                .iter()
                .map(|file| {
                    (
                        file.key.to_owned(),
                        CompiledXamlInput {
                            path: file.path.to_owned(),
                            rust_file: file.rust_file.into(),
                            line: file.line,
                            column: file.column,
                        },
                    )
                })
                .collect(),
        )
    }
}

#[derive(Default)]
pub(super) struct DocumentFiles {
    contents: BTreeMap<PathBuf, Option<String>>,
}

impl DocumentFiles {
    pub(super) fn read(
        &mut self,
        package: &Path,
        files: &[(String, CompiledXamlInput)],
        lines: &mut Vec<String>,
        errors: &mut Vec<String>,
    ) -> Option<Vec<(String, String)>> {
        let mut documents = Vec::with_capacity(files.len());
        for (key, input) in files {
            let path = package.join(&input.path);
            let contents = self.contents.entry(path.clone()).or_insert_with(|| {
                lines.push(format!("cargo::rerun-if-changed={}", path.display()));
                match fs::read_to_string(&path) {
                    Ok(text) => Some(text),
                    Err(error) => {
                        errors.push(format!("{}: {error}", path.display()));
                        None
                    }
                }
            });
            if let Some(text) = contents {
                documents.push((key.clone(), text.clone()));
            }
        }
        (documents.len() == files.len()).then_some(documents)
    }
}
