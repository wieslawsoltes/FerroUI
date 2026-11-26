use crate::{AttachedProperty, FerroObject, FerroProperty, StyledPropertyOptions, TypeInfo};
use std::path::{Path, PathBuf};

// Private owner for the typed attachment. It has no global class registration.
static SOURCE_OWNER: TypeInfo =
    TypeInfo::new("RsxSourceInfo", None).with_module_path(module_path!());

/// A location in the source used by the current Rust view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsxSourceLocation {
    /// One-based line and Unicode scalar column of the control's Rust type path.
    pub element: (usize, usize),
    /// One-based line and Unicode scalar column of its RSX invocation.
    pub invocation: (usize, usize),
    /// Preorder index among authored elements in that invocation.
    pub index: u32,
    /// Fingerprint of the compiled or accepted source bytes.
    pub revision: Option<u64>,
}

/// The source provenance carried by an authored native control.
#[derive(Clone, Copy, Debug)]
pub struct RsxSourceInfo {
    /// The compiler's display path, which may be relative to a Cargo workspace.
    pub file: &'static str,
    /// `CARGO_MANIFEST_DIR` of the compiling package.
    pub manifest: &'static str,
    /// Source compiled into the application.
    pub compiled: RsxSourceLocation,
    resolve: fn(Self) -> RsxSourceLocation,
}

crate::styled_value!(RsxSourceInfo);

impl PartialEq for RsxSourceInfo {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
            && self.manifest == other.manifest
            && self.compiled == other.compiled
    }
}

impl RsxSourceInfo {
    #[doc(hidden)]
    pub fn new(file: &'static str, manifest: &'static str, compiled: RsxSourceLocation) -> Self {
        Self {
            file,
            manifest,
            compiled,
            resolve: |source| source.compiled,
        }
    }

    #[doc(hidden)]
    pub fn with_resolver(mut self, resolve: fn(Self) -> RsxSourceLocation) -> Self {
        self.resolve = resolve;
        self
    }

    #[doc(hidden)]
    pub fn location_property() -> &'static AttachedProperty<Option<RsxSourceInfo>> {
        crate::ferro_thread_local! {
            static PROPERTY: &'static AttachedProperty<Option<RsxSourceInfo>> =
                FerroProperty::register_attached_with_core(
                    &SOURCE_OWNER,
                    FerroObject::TYPE,
                    "RsxSourceLocation",
                    StyledPropertyOptions::new(None),
                );
        }
        PROPERTY.with(|property| *property)
    }

    /// The source recorded for this object by the declarative view reconciler.
    pub fn get(object: &FerroObject) -> Option<Self> {
        object.get_value(Self::location_property())
    }

    /// Resolve the location in the compiled or most recently accepted source.
    pub fn location(self) -> RsxSourceLocation {
        (self.resolve)(self)
    }

    /// Resolve the authored source to an absolute file URI when it is available.
    pub fn source_uri(&self) -> Option<url::Url> {
        #[cfg(any(unix, windows, target_os = "redox", target_os = "wasi", target_os = "hermit"))]
        {
            url::Url::from_file_path(Self::resolve_file(self.file, self.manifest)?).ok()
        }
        #[cfg(not(any(unix, windows, target_os = "redox", target_os = "wasi", target_os = "hermit")))]
        {
            None
        }
    }

    /// Resolve a compiler path through the package or an enclosing workspace.
    pub fn resolve_file(file: &str, manifest: &str) -> Option<PathBuf> {
        let file = Path::new(file);
        if file.is_absolute() {
            return file.is_file().then(|| file.to_path_buf());
        }
        Path::new(manifest)
            .ancestors()
            .map(|dir| dir.join(file))
            .find(|path| path.is_file())
    }
}
