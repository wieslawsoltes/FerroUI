//! Hot reload of the literals of `rsx!` (feature `hot-reload`).
//!
//! `rsx!` passes each literal it can change at run time (the text of a child,
//! a number, a text or a character given to an attribute) through [`lit`],
//! with the place of the macro and the number of the literal among the
//! literal tokens of the macro. Without the feature `lit` returns the literal
//! and costs nothing.
//!
//! With the feature, [`watch`] (which `launch` calls) checks the source files
//! that hold `rsx!` literals several times a second. When a file changes, its
//! macros are read again (`proc-macro2` with the locations of tokens): a
//! macro whose tokens are those of the build apart from literals gets the new
//! literals, and the components that read them run again, so the
//! reconciler patches the controls and keeps their state. A macro whose
//! structure changed is reported ([`Diagnostic::NeedsRebuild`]): it needs a
//! rebuild. Only components that run again (`rerender`,
//! `#[component(rerender)]`) and children that follow state run again; a
//! component that runs once keeps what it built.
//!
//! [`Diagnostic::NeedsRebuild`]: crate::Diagnostic::NeedsRebuild

/// Where an `rsx!` is: what the macro writes for each of its literals.
#[derive(Clone, Copy, Debug)]
pub struct Site {
    /// `file!()` of the macro: relative to the root of the workspace.
    pub file: &'static str,
    /// `line!()` of the macro.
    pub line: u32,
    /// `column!()` of the macro.
    pub column: u32,
    /// `env!("CARGO_MANIFEST_DIR")` of the crate of the macro, to find the
    /// file.
    pub manifest: &'static str,
}

/// A literal that hot reload can give another value: what the text of the
/// literal in the source parses to.
pub trait HotLiteral: Sized + 'static {
    /// The value of the literal written as `text` in the source, if it is
    /// one of this type.
    fn from_literal(text: &str) -> Option<Self>;
}

/// A literal of an `rsx!`: `value`, or the value the source gives it now
/// when hot reload is on and the literal changed.
#[inline(always)]
pub fn lit<T: HotLiteral>(site: Site, index: u32, value: T) -> T {
    #[cfg(feature = "hot-reload")]
    {
        if let Some(text) = reload::current(site, index) {
            if let Some(changed) = T::from_literal(&text) {
                return changed;
            }
        }
    }
    let _ = (site, index);
    value
}

/// Starts checking the source files of the `rsx!` literals for changes (on
/// the dispatcher of the thread); `launch` calls it. Without the feature
/// `hot-reload` it does nothing.
pub fn watch() {
    #[cfg(feature = "hot-reload")]
    reload::watch();
}

/// Checks the source files for changes now and applies them; what [`watch`]
/// runs on its timer.
pub fn poll() {
    #[cfg(feature = "hot-reload")]
    reload::poll();
}

/// Applies `source` as the new text of `file` (its `file!()` path), as if
/// the file had been saved with it: for tests and tools that hold the source
/// themselves.
pub fn apply_source(file: &str, source: &str) {
    #[cfg(feature = "hot-reload")]
    reload::apply(file, source);
    let _ = (file, source);
}

macro_rules! hot_number {
    ($($t:ty),*) => {$(
        impl HotLiteral for $t {
            fn from_literal(text: &str) -> Option<Self> {
                #[cfg(feature = "hot-reload")]
                {
                    return match syn::parse_str::<syn::Lit>(text).ok()? {
                        syn::Lit::Int(int) => int.base10_parse::<$t>().ok(),
                        syn::Lit::Float(float) => float.base10_parse::<$t>().ok(),
                        _ => None,
                    };
                }
                #[allow(unreachable_code)]
                {
                    let _ = text;
                    None
                }
            }
        }
    )*};
}

hot_number!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

impl HotLiteral for &'static str {
    fn from_literal(text: &str) -> Option<Self> {
        #[cfg(feature = "hot-reload")]
        {
            let value = syn::parse_str::<syn::LitStr>(text).ok()?.value();
            // A literal is static; a changed one lives as long as the program
            // (hot reload is for development).
            return Some(Box::leak(value.into_boxed_str()));
        }
        #[allow(unreachable_code)]
        {
            let _ = text;
            None
        }
    }
}

impl HotLiteral for char {
    fn from_literal(text: &str) -> Option<Self> {
        #[cfg(feature = "hot-reload")]
        {
            return syn::parse_str::<syn::LitChar>(text).ok().map(|lit| lit.value());
        }
        #[allow(unreachable_code)]
        {
            let _ = text;
            None
        }
    }
}

#[cfg(feature = "hot-reload")]
mod reload {
    use super::Site;
    use crate::diagnostics::{report, Diagnostic};
    use crate::reactive::{signal, Signal};
    use proc_macro2::{Delimiter, TokenStream, TokenTree};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;

    /// An `rsx!` of a file: where it starts, its literal tokens and the
    /// shape of its other tokens.
    #[derive(Clone, PartialEq, Debug)]
    struct Invocation {
        line: u32,
        column: u32,
        literals: Vec<String>,
        shape: String,
    }

    struct FileState {
        path: Option<PathBuf>,
        modified: Option<SystemTime>,
        /// The macros as the build saw them (the first read of the file).
        built: Vec<Invocation>,
        /// The literals that differ from the build: (line, column, index) of
        /// the build to the text in the source now.
        changed: HashMap<(u32, u32, u32), String>,
    }

    ferroui_base::ferro_thread_local! {
        static FILES: RefCell<HashMap<&'static str, FileState>> = RefCell::new(HashMap::new());
        static VERSION: RefCell<Option<Signal<u64>>> = RefCell::new(None);
        static WATCHING: std::cell::Cell<bool> = std::cell::Cell::new(false);
    }

    fn version() -> Signal<u64> {
        VERSION.with(|version| {
            *version.borrow_mut().get_or_insert_with(|| {
                // Under an owner of its own, which nothing disposes: it lives
                // as long as the thread, whatever runs when it is first read.
                crate::reactive::Owner::new_root().with(|| signal(0_u64))
            })
        })
    }

    pub(super) fn current(site: Site, index: u32) -> Option<String> {
        // Follow the changes of the sources: a component that read a literal
        // runs again when one changes.
        version().get();
        FILES.with(|files| {
            let mut files = files.borrow_mut();
            let state = files.entry(site.file).or_insert_with(|| register(site));
            state.changed.get(&(site.line, site.column, index)).cloned()
        })
    }

    fn register(site: Site) -> FileState {
        let path = resolve(site.file, site.manifest);
        let (modified, built) = match &path {
            Some(path) => {
                let modified = std::fs::metadata(path).and_then(|meta| meta.modified()).ok();
                let built = std::fs::read_to_string(path).map(|text| scan(&text)).unwrap_or_default();
                (modified, built)
            }
            None => (None, Vec::new()),
        };
        FileState { path, modified, built, changed: HashMap::new() }
    }

    /// The file of `file!()`: relative to the root of the workspace, which
    /// is an ancestor of the directory of the crate.
    fn resolve(file: &str, manifest: &str) -> Option<PathBuf> {
        let file = Path::new(file);
        if file.is_absolute() {
            return file.exists().then(|| file.to_path_buf());
        }
        Path::new(manifest).ancestors().map(|dir| dir.join(file)).find(|candidate| candidate.exists())
    }

    pub(super) fn watch() {
        if WATCHING.with(|watching| watching.replace(true)) {
            return;
        }
        let _timer = ferroui_base::threading::DispatcherTimer::run(
            || {
                poll();
                true
            },
            std::time::Duration::from_millis(300),
            ferroui_base::threading::DispatcherPriority::BACKGROUND,
        );
    }

    pub(super) fn poll() {
        let changed: Vec<(&'static str, String)> = FILES.with(|files| {
            let mut files = files.borrow_mut();
            let mut changed = Vec::new();
            for (file, state) in files.iter_mut() {
                let Some(path) = &state.path else { continue };
                let modified = std::fs::metadata(path).and_then(|meta| meta.modified()).ok();
                if modified != state.modified {
                    state.modified = modified;
                    if let Ok(text) = std::fs::read_to_string(path) {
                        changed.push((*file, text));
                    }
                }
            }
            changed
        });
        for (file, text) in changed {
            apply(file, &text);
        }
    }

    pub(super) fn apply(file: &str, source: &str) {
        let now = scan(source);
        let updated = FILES.with(|files| {
            let mut files = files.borrow_mut();
            let Some(state) = files.iter_mut().find(|(name, _)| **name == file).map(|(_, state)| state) else {
                return false;
            };
            let mut changed = HashMap::new();
            let same_count = now.len() == state.built.len();
            for (index, built) in state.built.iter().enumerate() {
                match now.get(index) {
                    Some(current) if same_count && current.shape == built.shape && current.literals.len() == built.literals.len() => {
                        for (number, (old, new)) in built.literals.iter().zip(&current.literals).enumerate() {
                            if old != new {
                                changed.insert((built.line, built.column, number as u32), new.clone());
                            }
                        }
                    }
                    _ => report(Diagnostic::NeedsRebuild { file: file.to_string(), line: built.line }),
                }
            }
            if changed == state.changed {
                return false;
            }
            report(Diagnostic::HotReloaded { file: file.to_string(), literals: changed.len() });
            state.changed = changed;
            true
        });
        if updated {
            version().update(|version| *version += 1);
        }
    }

    /// The `rsx!` invocations of a source file, in order, with nested ones.
    fn scan(source: &str) -> Vec<Invocation> {
        let Ok(tokens) = source.parse::<TokenStream>() else { return Vec::new() };
        let mut found = Vec::new();
        find(tokens, &mut found);
        found
    }

    fn find(tokens: TokenStream, found: &mut Vec<Invocation>) {
        let tokens: Vec<TokenTree> = tokens.into_iter().collect();
        for (at, token) in tokens.iter().enumerate() {
            if let TokenTree::Group(group) = token {
                // `rsx ! { .. }`, perhaps with a path before `rsx`.
                let is_rsx = at >= 2
                    && matches!(&tokens[at - 1], TokenTree::Punct(p) if p.as_char() == '!')
                    && matches!(&tokens[at - 2], TokenTree::Ident(ident) if ident == "rsx")
                    && group.delimiter() != Delimiter::None;
                if is_rsx {
                    let start = path_start(&tokens, at - 2);
                    let location = tokens[start].span().start();
                    let mut literals = Vec::new();
                    let mut shape = String::new();
                    describe(group.stream(), &mut literals, &mut shape);
                    found.push(Invocation { line: location.line as u32, column: location.column as u32 + 1, literals, shape });
                }
                find(group.stream(), found);
            }
        }
    }

    /// The first token of the path that ends with the token at `at`
    /// (`ferroui_view::rsx`).
    fn path_start(tokens: &[TokenTree], mut at: usize) -> usize {
        while at >= 3
            && matches!(&tokens[at - 1], TokenTree::Punct(p) if p.as_char() == ':')
            && matches!(&tokens[at - 2], TokenTree::Punct(p) if p.as_char() == ':')
            && matches!(&tokens[at - 3], TokenTree::Ident(_))
        {
            at -= 3;
        }
        at
    }

    fn describe(tokens: TokenStream, literals: &mut Vec<String>, shape: &mut String) {
        for token in tokens {
            match token {
                TokenTree::Literal(literal) => {
                    literals.push(literal.to_string());
                    shape.push_str("$ ");
                }
                TokenTree::Group(group) => {
                    let (open, close) = match group.delimiter() {
                        Delimiter::Brace => ("{", "}"),
                        Delimiter::Bracket => ("[", "]"),
                        Delimiter::Parenthesis => ("(", ")"),
                        Delimiter::None => ("", ""),
                    };
                    shape.push_str(open);
                    describe(group.stream(), literals, shape);
                    shape.push_str(close);
                    shape.push(' ');
                }
                other => {
                    shape.push_str(&other.to_string());
                    shape.push(' ');
                }
            }
        }
    }
}
