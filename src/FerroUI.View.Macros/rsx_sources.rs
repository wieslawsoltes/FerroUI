use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use syn::{spanned::Spanned, Path};

pub(super) struct RsxSources {
    file: String,
    invocation: (usize, usize),
    revision: Option<u64>,
    next: u32,
}

impl RsxSources {
    pub(super) fn new() -> Self {
        let call = Span::call_site().unwrap();
        Self {
            file: call.file(),
            invocation: (call.line(), call.column()),
            revision: call.local_file().and_then(fingerprint),
            next: 0,
        }
    }

    pub(super) fn next(&mut self) -> u32 {
        let index = self.next;
        self.next += 1;
        index
    }

    pub(super) fn locate(&self, path: &Path, build: TokenStream, index: u32) -> TokenStream {
        let span = path.span().unwrap();
        let file = span.file();
        let line = span.line();
        let column = span.column();
        if file != self.file || file.starts_with('<') || line == 0 || column == 0 {
            return build;
        }
        let (invocation_line, invocation_column) = self.invocation;
        let revision = self.revision.map_or_else(
            || quote!(::core::option::Option::None),
            |revision| quote!(::core::option::Option::Some(#revision)),
        );
        quote! {
            <#path as ::ferroui_view::Builder>::with_editor_source(#build, ::ferroui_view::RsxSourceInfo::new(
                #file,
                match ::core::option_env!("CARGO_MANIFEST_DIR") { Some(dir) => dir, None => "" },
                ::ferroui_view::RsxSourceLocation {
                    element: (#line, #column),
                    invocation: (#invocation_line, #invocation_column),
                    index: #index,
                    revision: #revision,
                },
            ))
        }
    }
}

fn fingerprint(path: PathBuf) -> Option<u64> {
    // Expansion reads the compiler's real path once. Only a digest and the
    // compiler's possibly remapped display path are emitted into the app.
    static REVISIONS: OnceLock<Mutex<HashMap<PathBuf, Option<u64>>>> = OnceLock::new();
    let mut revisions = REVISIONS.get_or_init(Mutex::default).lock().ok()?;
    *revisions.entry(path.clone()).or_insert_with(|| {
        std::fs::read(path)
            .ok()
            .map(|bytes| ferroui_reload_protocol::fingerprint(&bytes))
    })
}
