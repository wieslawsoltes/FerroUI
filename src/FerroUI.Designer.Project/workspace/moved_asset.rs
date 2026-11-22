//! Move a binary project asset without buffering its contents in editor history.

use super::{created_file::same_file, file_path, require, ProjectWorkspace};
use crate::{ProjectError, ProjectFileMove, Result};
use std::{
    fs,
    hash::{DefaultHasher, Hasher},
    io::{self, Read},
    path::Path,
};

#[derive(Clone)]
pub struct MovedAssetFile {
    pub from: String,
    pub to: String,
    digest: String,
    identity: fs::Metadata,
}

/// Review and history use the same bounded-memory fingerprint for asset bytes.
pub fn asset_digest(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = DefaultHasher::new();
    let mut bytes = [0; 32 * 1024];
    loop {
        let count = file.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        hash.write(&bytes[..count]);
    }
    Ok(format!("{:016x}", hash.finish()))
}

impl ProjectWorkspace {
    pub fn move_asset_file(
        &mut self,
        from: &str,
        to: &str,
        expected_digest: &str,
    ) -> Result<MovedAssetFile> {
        let movement = self.plan_asset_move(from, to)?;
        let identity = fs::symlink_metadata(file_path(&movement.from)?)?;
        self.transfer_asset(&movement, &identity, expected_digest)?;
        Ok(MovedAssetFile {
            from: movement.from,
            to: movement.to,
            digest: expected_digest.into(),
            identity,
        })
    }

    pub fn replay_asset_move(&mut self, file: &MovedAssetFile, redo: bool) -> Result<()> {
        let (from, to) = if redo {
            (&file.from, &file.to)
        } else {
            (&file.to, &file.from)
        };
        let movement = self.plan_asset_move(from, to)?;
        require(
            movement.from == *from && movement.to == *to,
            "The asset path was redirected",
        )?;
        self.transfer_asset(&movement, &file.identity, &file.digest)
    }

    fn transfer_asset(
        &mut self,
        movement: &ProjectFileMove,
        identity: &fs::Metadata,
        digest: &str,
    ) -> Result<()> {
        let from = file_path(&movement.from)?;
        let to = file_path(&movement.to)?;
        check_asset(&from, identity, digest)?;
        fs::hard_link(&from, &to)?;
        let result = (|| {
            check_asset(&from, identity, digest)?;
            require(
                asset_identity(&to, identity),
                "The asset move destination changed",
            )?;
            Ok(fs::remove_file(&from)?)
        })();
        if let Err(error) = result {
            if asset_identity(&to, identity) {
                if !asset_identity(&from, identity) {
                    return Err(ProjectError(format!(
                        "{error}. The original asset is held at {}",
                        to.display()
                    )));
                }
                fs::remove_file(&to)?;
            }
            return Err(error);
        }
        for path in self.resource_paths.values_mut() {
            if path == &from {
                *path = to.clone();
            }
        }
        Ok(())
    }
}

fn asset_identity(path: &Path, identity: &fs::Metadata) -> bool {
    fs::symlink_metadata(path).is_ok_and(|actual| actual.is_file() && same_file(identity, &actual))
}

fn check_asset(path: &Path, identity: &fs::Metadata, digest: &str) -> Result<()> {
    require(
        asset_identity(path, identity) && asset_digest(path)? == digest,
        "The asset changed on disk",
    )
}
