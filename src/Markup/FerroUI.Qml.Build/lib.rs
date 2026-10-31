//! Build QML into a Rust module with a `compiled_qml()` entry point.
//! The generated code requires `ferroui-qml` as an application dependency.
mod emit;
mod files;
mod identified;
mod native;

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

#[derive(Default)]
pub struct Compiler {
    import_paths: Vec<PathBuf>,
}

/// Rust source and the exact files and import directories to watch for changes.
pub struct Output {
    pub rust: String,
    pub dependencies: BTreeSet<PathBuf>,
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a directory containing URI modules such as `Acme/Controls/qmldir`.
    pub fn import_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.import_paths.push(path.into());
        self
    }

    pub fn compile(&self, input: impl AsRef<Path>) -> Result<Output, BuildError> {
        let mut files = files::Files::new(self.import_paths.clone());
        let source = files.load(input.as_ref()).map_err(BuildError)?;
        let document = ferroui_qml_compiler::link(source, &mut files)
            .map_err(|e| BuildError(e.to_string()))?;
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
