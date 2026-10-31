//! Build QML into a Rust module with a `compiled_qml()` entry point.
//! The generated code requires `ferroui-qml` as an application dependency.
mod emit;
mod files;
mod identified;
mod native;
mod scripts;

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct BuildError(String);
impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for BuildError {}

type Validate<'a> = Box<dyn Fn(&ferroui_qml_compiler::Document) -> Result<(), String> + 'a>;

#[derive(Default)]
pub struct Compiler<'a> {
    validate: Option<Validate<'a>>,
    import_paths: Vec<PathBuf>,
}

/// Rust source and the exact files and import directories to watch for changes.
pub struct Output {
    pub rust: String,
    pub dependencies: BTreeSet<PathBuf>,
}

impl<'a> Compiler<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a directory containing URI modules such as `Acme/Controls/qmldir`.
    pub fn import_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.import_paths.push(path.into());
        self
    }

    /// Apply the host's type registry during compilation, before emitting Rust.
    /// A build script can pass `|doc| registry.check(doc)` here.
    pub fn check_with<E: fmt::Display>(
        mut self,
        validate: impl Fn(&ferroui_qml_compiler::Document) -> Result<(), E> + 'a,
    ) -> Self {
        self.validate = Some(Box::new(move |doc| {
            validate(doc).map_err(|e| e.to_string())
        }));
        self
    }

    pub fn compile(&self, input: impl AsRef<Path>) -> Result<Output, BuildError> {
        let mut files = files::Files::new(self.import_paths.clone());
        let source = files.load(input.as_ref()).map_err(BuildError)?;
        let document = ferroui_qml_compiler::link(source, &mut files)
            .map_err(|e| BuildError(e.to_string()))?;
        if let Some(validate) = &self.validate {
            validate(&document)
                .map_err(|reason| BuildError(format!("{}: {reason}", input.as_ref().display())))?;
        }
        Ok(Output {
            rust: emit::module(document).map_err(BuildError)?,
            dependencies: files.dependencies(),
        })
    }

    /// Compile a component for `include!` and register its dependencies with Cargo.
    pub fn compile_file(
        &self,
        input: impl AsRef<Path>,
        output_name: impl AsRef<Path>,
    ) -> Result<(), BuildError> {
        let output_name = output_name.as_ref();
        if output_name.file_name() != Some(output_name.as_os_str()) {
            return Err(BuildError(
                "generated module name must be a single filename".into(),
            ));
        }
        let out_dir = std::env::var_os("OUT_DIR").ok_or_else(|| {
            BuildError("OUT_DIR is missing; call compile_file from a Cargo build script".into())
        })?;
        let output = self.compile(input)?;
        for path in &output.dependencies {
            println!("cargo:rerun-if-changed={}", path.display());
        }
        let path = PathBuf::from(out_dir).join(output_name);
        if std::fs::read(&path).ok().as_deref() != Some(output.rust.as_bytes()) {
            std::fs::write(&path, output.rust)
                .map_err(|e| BuildError(format!("{}: {e}", path.display())))?;
        }
        Ok(())
    }
}

pub fn compile_file(
    input: impl AsRef<Path>,
    output_name: impl AsRef<Path>,
) -> Result<(), BuildError> {
    Compiler::new().compile_file(input, output_name)
}
