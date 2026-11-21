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

#[inline]
pub(crate) fn record_source_locations() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    cfg!(debug_assertions) && *ENABLED.get_or_init(|| {
        ["FERROUI_EDITOR_RELOAD", "FERROUI_EDITOR_SOURCE"]
            .iter()
            .any(|name| std::env::var_os(name).as_deref() == Some("1".as_ref()))
    })
}

pub(crate) fn source_location(editor_source: crate::RsxSourceInfo) -> crate::RsxSourceLocation {
    #[cfg(feature = "hot-reload")]
    if ferroui_reload_protocol::enabled() {
        return reload::source_location(editor_source);
    }
    editor_source.compiled
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

/// Report that a custom RSX application opened its main window. Call before
/// showing the window and call [`watch`] on the same dispatcher. `launch`
/// registers both automatically.
pub fn notify_editor_on_open(window: &ferroui_base::Ref<ferroui_controls::Window>) {
    #[cfg(feature = "hot-reload")]
    editor_frames::notify_on_open(window);
    #[cfg(not(feature = "hot-reload"))]
    let _ = window;
}

#[cfg(feature = "hot-reload")]
#[path = "hot_editor_frames.rs"]
mod editor_frames;

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
#[path = "hot_source.rs"]
mod source;

#[cfg(feature = "hot-reload")]
mod reload {
    use super::Site;
    use super::source::{Locations, Source};
    use crate::diagnostics::{report, Diagnostic};
    use crate::reactive::{signal, Signal};
    use ferroui_reload_protocol::{self as protocol, Language, Status};
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;
    use std::time::SystemTime;

    type LiteralId = (u32, u32, u32);

    struct FileState {
        path: Option<PathBuf>,
        manifest: &'static str,
        modified: Option<SystemTime>,
        /// The macros as the build saw them (the first read of the file).
        built: Option<Source>,
        accepted: Option<Locations>,
        /// The literals that differ from the build: (line, column, index) of
        /// the build to the text in the source now.
        changed: HashMap<LiteralId, String>,
        awaiting_reads: HashSet<LiteralId>,
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
            let literal = (site.line, site.column, index);
            state.awaiting_reads.remove(&literal);
            state.changed.get(&literal).cloned()
        })
    }

    fn register(site: Site) -> FileState {
        let path = crate::RsxSourceInfo::resolve_file(site.file, site.manifest);
        let (modified, built) = match &path {
            Some(path) => {
                let modified = std::fs::metadata(path).and_then(|meta| meta.modified()).ok();
                let built = std::fs::read_to_string(path).ok().and_then(|text| Source::parse(&text, protocol::enabled()).ok());
                (modified, built)
            }
            None => (None, None),
        };
        FileState { path, manifest: site.manifest, modified, built, accepted: None,
            changed: HashMap::new(), awaiting_reads: HashSet::new() }
    }

    pub(super) fn source_location(info: crate::RsxSourceInfo) -> crate::RsxSourceLocation {
        let located = FILES.with(|files| {
            let files = files.borrow();
            let state = files.get(info.file)?;
            if state.manifest != info.manifest {
                return None;
            }
            state.accepted.as_ref()?.resolve(state.built.as_ref()?, info.compiled)
        });
        located.unwrap_or(info.compiled)
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
        )
        .detach();
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
        let managed = protocol::enabled();
        let now = Source::parse(source, managed);
        let result = FILES.with(|files| {
            let mut files = files.borrow_mut();
            let state = files.get_mut(file)?;
            let built = state.built.as_ref()?;
            let now = match now {
                Ok(now) => now,
                Err(error) => {
                    if let Some(path) = &state.path {
                        protocol::source(Language::Rsx, Status::Failed, path, source, Some(&error));
                    }
                    return None;
                }
            };
            if managed && built.signature != now.signature {
                if let Some(path) = &state.path {
                    protocol::source(Language::Rsx, Status::Rebuild, path, source, Some("Rust code or RSX structure changed"));
                }
                return None;
            }
            let mut changed = HashMap::new();
            let same_count = now.rsx.len() == built.rsx.len();
            for (index, built) in built.rsx.iter().enumerate() {
                match now.rsx.get(index) {
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
            let updated = changed != state.changed;
            if updated {
                if managed {
                    state.awaiting_reads = changed.keys().chain(state.changed.keys()).copied()
                        .filter(|key| changed.get(key) != state.changed.get(key)).collect();
                }
                report(Diagnostic::HotReloaded { file: file.to_string(), literals: changed.len() });
                state.changed = changed;
            }
            Some((updated, state.path.clone(), now))
        });
        let Some((updated, path, now)) = result else { return; };
        if updated {
            version().update(|version| *version += 1);
        }
        if managed {
            if let Some(path) = path {
                let visible = FILES.with(|files| files.borrow_mut().get_mut(file).is_some_and(|state| {
                    let visible = std::mem::take(&mut state.awaiting_reads).is_empty();
                    if visible {
                        state.accepted = Some(now.into_locations());
                    }
                    visible
                }));
                if visible {
                    super::editor_frames::applied(&path, source);
                } else {
                    protocol::source(Language::Rsx, Status::Rebuild, &path, source,
                        Some("The changed RSX literals were not reread by a mounted reactive view"));
                }
            }
        }
    }
}
