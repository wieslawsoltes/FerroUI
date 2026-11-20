//! A file move owned by a Studio history transaction.
use super::{created_file::same_file, file_path, read_source, require, ProjectWorkspace};
use crate::{Result, XamlFileMove};
use std::{fs, path::Path, sync::Arc};

#[derive(Clone)]
pub struct MovedSourceFile {
    pub from: String,
    pub to: String,
    bytes: Arc<str>,
    identity: fs::Metadata,
}

impl ProjectWorkspace {
    pub fn move_source_file(
        &mut self,
        from: &str,
        to: &str,
        expected_source: &str,
    ) -> Result<MovedSourceFile> {
        let movement = self.plan_xaml_move(from, to)?;
        let source = file_path(&movement.from)?;
        let identity = fs::symlink_metadata(&source)?;
        check_source(&source, &identity, expected_source)?;
        self.transfer_source(&movement, &identity, expected_source)?;
        Ok(MovedSourceFile {
            from: movement.from,
            to: movement.to,
            bytes: Arc::from(expected_source),
            identity,
        })
    }

    /// Validate physical identity, bytes and the vacant destination before replay.
    pub fn replay_source_move(
        &mut self,
        file: &mut MovedSourceFile,
        redo: bool,
        saved: &str,
    ) -> Result<()> {
        let (from, to) = if redo {
            (&file.from, &file.to)
        } else {
            (&file.to, &file.from)
        };
        let movement = self.plan_xaml_move(from, to)?;
        require(
            &movement.from == from && &movement.to == to,
            "The moved file or destination was redirected",
        )?;
        let identity = if saved == file.bytes.as_ref() {
            file.identity.clone()
        } else {
            // The workspace host can attest to its own newer saved snapshot.
            fs::symlink_metadata(file_path(from)?)?
        };
        self.transfer_source(&movement, &identity, saved)?;
        file.bytes = Arc::from(saved);
        file.identity = identity;
        Ok(())
    }

    fn transfer_source(
        &mut self,
        movement: &XamlFileMove,
        identity: &fs::Metadata,
        expected_source: &str,
    ) -> Result<()> {
        let source = file_path(&movement.from)?;
        let destination = file_path(&movement.to)?;
        require(
            !self.buffers.contains_key(&movement.to),
            "The move destination already has an editor buffer",
        )?;
        let mut staged = self.clone();
        staged.rekey_document(&movement.from, &movement.to, &destination)?;
        for path in staged.resource_paths.values_mut() {
            if path == &source {
                *path = destination.clone();
            }
        }
        check_source(&source, identity, expected_source)?;
        fs::hard_link(&source, &destination)?;
        let remove_origin = check_source(&source, identity, expected_source)
            .and_then(|()| check_identity(&destination, identity))
            .and_then(|()| fs::remove_file(&source).map_err(Into::into));
        if let Err(error) = remove_origin {
            if check_identity(&destination, identity).is_ok() {
                fs::remove_file(&destination)?;
            }
            return Err(error);
        }
        *self = staged;
        Ok(())
    }
}

fn check_source(path: &Path, identity: &fs::Metadata, expected: &str) -> Result<()> {
    check_identity(path, identity)?;
    require(
        read_source(path)? == expected,
        "The moved file changed on disk",
    )
}

fn check_identity(path: &Path, identity: &fs::Metadata) -> Result<()> {
    let actual = fs::symlink_metadata(path)?;
    require(
        actual.is_file() && same_file(identity, &actual),
        "The moved file changed on disk",
    )
}
