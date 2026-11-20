//! Publish and reverse new files owned by a checked editor transaction.
use super::{Buffer, ProjectWorkspace, file_path, file_uri, read_source, require, storage};
use crate::Result;
use ferroui_designer_core::Document;
use std::{fs, sync::Arc};

#[derive(Clone)]
pub struct CreatedSourceFile {
    uri: String,
    source: Arc<str>,
    identity: fs::Metadata,
}

impl CreatedSourceFile {
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn source(&self) -> &str {
        &self.source
    }
}

impl ProjectWorkspace {
    /// Create a new valid XAML file without replacing existing paths or editor drafts.
    pub fn publish_source_file(&mut self, uri: &str, source: &str) -> Result<CreatedSourceFile> {
        let path = self.checked_path(&file_path(uri)?)?;
        require(
            path.extension()
                .is_some_and(|suffix| suffix.eq_ignore_ascii_case("xaml")),
            "Choose a new XAML source file",
        )?;
        let uri = file_uri(&path)?;
        require(
            !self.buffers.contains_key(&uri),
            "The dictionary destination is already open",
        )?;
        require(source.len() <= 16 * 1024 * 1024, "Document exceeds 16 MiB")?;
        let document = Document::new(source)?;
        let identity = storage::atomic_create_with_metadata(&path, source)?;
        self.buffers.insert(
            uri.clone(),
            Buffer {
                uri: uri.clone(),
                path: Some(path),
                document,
                saved_source: Some(source.into()),
                version: 0,
            },
        );
        self.add_entry(&uri)?;
        Ok(CreatedSourceFile {
            uri,
            source: Arc::from(source),
            identity,
        })
    }

    /// Reverse an owned creation only while its original file and contents match.
    pub fn undo_source_file(&mut self, file: &CreatedSourceFile) -> Result<()> {
        let path = file_path(&file.uri)?;
        require(
            file_uri(self.checked_path(&path)?)? == file.uri,
            "The created file was moved or replaced",
        )?;
        require(
            self.buffer(&file.uri)
                .is_none_or(|buffer| buffer.source() == file.source()),
            "Undo newer edits in the extracted dictionary first",
        )?;
        let check = || -> Result<()> {
            let meta = fs::symlink_metadata(&path)?;
            require(
                meta.is_file() && same_file(&file.identity, &meta),
                "The extracted dictionary changed on disk",
            )
        };
        check()?;
        require(
            read_source(&path)? == file.source(),
            "The extracted dictionary changed on disk",
        )?;
        check()?;
        fs::remove_file(&path)?;
        self.buffers.remove(&file.uri);
        if self.active.as_deref() == Some(&file.uri) {
            self.active = self.buffers.keys().next().cloned();
        }
        for project in &mut self.solution.projects {
            project
                .documents
                .retain(|document| document.uri != file.uri);
        }
        Ok(())
    }
}

fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (left.dev(), left.ino()) != (right.dev(), right.ino()) {
            return false;
        }
    }
    #[cfg(not(unix))]
    if left.created().ok() != right.created().ok() {
        return false;
    }
    left.len() == right.len() && left.modified().ok() == right.modified().ok()
}
