//! Resolve compiler symbol indexes against the selected Cargo package.
use super::MarkupCatalog;
use crate::{ProjectError, Result, require};
use ferroui_build_scan::compiled_index::CompiledXamlIndex;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_INDEX_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

pub struct CompiledXamlOutput {
    pub index: CompiledXamlIndex,
    directory: PathBuf,
}

impl MarkupCatalog {
    /// Older and manual XAML schemas can omit the authoring index. Dependencies
    /// and duplicate Cargo builds never stand in for the owning package.
    pub fn compiled_xaml_output(&self, manifest: &Path) -> Result<Option<CompiledXamlOutput>> {
        let mut result = None;
        let mut remaining = MAX_INDEX_BYTES;
        for directory in &self.directories {
            let path = directory.join(CompiledXamlIndex::FILE_NAME);
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() => metadata,
                Ok(_) => continue,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            require(
                metadata.len() <= remaining,
                "Compiled XAML indexes exceed 4 MiB",
            )?;
            let text = read_text(&path, remaining)?;
            remaining -= text.len() as u64;
            let index = CompiledXamlIndex::parse(&text).map_err(|error| {
                ProjectError(format!("Cannot read {}: {error}", path.display()))
            })?;
            if !index.manifest.is_absolute()
                || index.manifest.canonicalize().ok().as_deref() != Some(manifest)
            {
                continue;
            }
            require(
                index.is_supported(),
                "Rebuild the package XAML schema with this version of FerroUI",
            )?;
            require(
                result.is_none(),
                "Build the XAML schema for one Cargo target before moving this file",
            )?;
            result = Some(CompiledXamlOutput {
                index,
                directory: directory.clone(),
            });
        }
        Ok(result)
    }
}

impl CompiledXamlOutput {
    pub fn module_directory(&self) -> PathBuf {
        self.directory.join("xaml")
    }

    /// The generated source is a navigation target inside this build. Consumers
    /// must put any authoring edits in the package sources.
    pub fn module_source(&self, module: &str) -> Result<(PathBuf, String)> {
        require(
            syn::parse_str::<syn::Ident>(module).is_ok(),
            "Invalid compiled XAML module name",
        )?;
        let path = self.module_directory().join(format!("{module}.rs"));
        require(
            path.canonicalize().ok().as_ref() == Some(&path) && path.is_file(),
            "Rebuild the package XAML schema to restore its generated Rust module",
        )?;
        let text = read_text(&path, MAX_SOURCE_BYTES)?;
        Ok((path, text))
    }
}

fn read_text(path: &Path, limit: u64) -> Result<String> {
    let mut text = String::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut text)?;
    require(
        text.len() as u64 <= limit,
        "Compiled XAML output is too large",
    )?;
    Ok(text)
}
