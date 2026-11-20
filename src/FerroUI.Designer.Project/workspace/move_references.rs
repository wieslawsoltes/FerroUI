//! Saved design files participate in the same filesystem operation as the moved source.
use super::{file_path, file_uri, moved_file::check_source, require, storage, ProjectWorkspace};
use crate::{ProjectError, Result, SavedDesignFile};
use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

/// An exact saved reference replacement from an accepted file move review.
pub struct SavedReferenceEdit {
    pub uri: String,
    pub before: String,
    pub after: String,
}

#[derive(Clone)]
pub(super) struct SavedReference {
    edit: Arc<SavedReferenceEdit>,
    identity: fs::Metadata,
}

impl SavedReference {
    pub(super) fn capture(
        workspace: &ProjectWorkspace,
        edits: Vec<SavedReferenceEdit>,
    ) -> Result<Vec<Self>> {
        let mut seen = BTreeSet::new();
        edits
            .into_iter()
            .map(|edit| {
                require(
                    seen.insert(edit.uri.clone()),
                    "Duplicate saved reference in the move",
                )?;
                let path = checked(workspace, &edit.uri)?;
                let identity = fs::symlink_metadata(&path)?;
                check_source(&path, &identity, &edit.before)?;
                Ok(Self {
                    edit: Arc::new(edit),
                    identity,
                })
            })
            .collect()
    }

    pub(super) fn prepare(
        &self,
        workspace: &ProjectWorkspace,
        forward: bool,
    ) -> Result<PreparedReference> {
        let path = checked(workspace, &self.edit.uri)?;
        let before = if forward {
            &self.edit.before
        } else {
            &self.edit.after
        };
        check_source(&path, &self.identity, before)
            .map_err(|error| ProjectError(format!("{}: {error}", path.display())))?;
        let backup = storage::backup_file(&path)?;
        let prepared = PreparedReference {
            path,
            backup: Some(backup),
            reference: self.clone(),
            forward,
            installed: None,
        };
        check_source(prepared.backup.as_ref().unwrap(), &self.identity, before)?;
        Ok(prepared)
    }

    pub(super) fn accept(&mut self, prepared: &PreparedReference) {
        self.identity = prepared
            .installed
            .as_ref()
            .expect("reference was installed")
            .clone();
    }
}

pub(super) struct PreparedReference {
    path: PathBuf,
    backup: Option<PathBuf>,
    reference: SavedReference,
    forward: bool,
    installed: Option<fs::Metadata>,
}

impl PreparedReference {
    pub(super) fn install(&mut self) -> Result<()> {
        let (before, after) = self.contents();
        let identity = storage::atomic_write_checked(&self.path, after, || {
            check_source(&self.path, &self.reference.identity, before)
        })
        .map_err(|error| ProjectError(format!("{}: {error}", self.path.display())))?;
        self.installed = Some(identity);
        Ok(())
    }

    pub(super) fn verify(&self) -> Result<()> {
        let (_, after) = self.contents();
        check_source(
            &self.path,
            self.installed.as_ref().expect("reference was installed"),
            after,
        )
        .map_err(|error| ProjectError(format!("{}: {error}", self.path.display())))
    }

    /// Restore the original inode so an interrupted history step can be retried.
    /// An external edit takes priority over rollback, and the backup path is
    /// retained if the original cannot be restored safely.
    pub(super) fn restore(&mut self) -> Result<()> {
        let Some(installed) = &self.installed else {
            return Ok(());
        };
        let backup = self
            .backup
            .take()
            .expect("an installed reference has a backup");
        let (before, after) = self.contents();
        let restore = check_source(&backup, &self.reference.identity, before)
            .and_then(|()| check_source(&self.path, installed, after))
            .and_then(|()| fs::rename(&backup, &self.path).map_err(Into::into));
        restore.map_err(|error| {
            ProjectError(format!(
                "Could not restore {}. Original saved at {}. {error}",
                self.path.display(),
                backup.display()
            ))
        })
    }

    fn contents(&self) -> (&str, &str) {
        let edit = &self.reference.edit;
        if self.forward {
            (&edit.before, &edit.after)
        } else {
            (&edit.after, &edit.before)
        }
    }
}

impl Drop for PreparedReference {
    fn drop(&mut self) {
        if let Some(backup) = self.backup.take() {
            let _ = fs::remove_file(backup);
        }
    }
}

fn checked(workspace: &ProjectWorkspace, uri: &str) -> Result<PathBuf> {
    let input = file_path(uri)?;
    let path = workspace.checked_path(&input)?;
    require(
        file_uri(&path)? == uri
            && SavedDesignFile::for_path(&path).is_some()
            && workspace.buffer(uri).is_none(),
        "The saved reference changed location or has an open source buffer",
    )?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_move_reference_rollback_restores_the_snapshot_identity_for_a_retry() {
        let directory =
            std::env::temp_dir().join(format!("ferroui-reference-rollback-{}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let directory = directory.canonicalize().unwrap();
        let stage = directory.join("Board.ferrostages");
        let scenario = directory.join("Review.ferroui-scenario.json");
        let original = r#"{"document":"Old.xaml"}"#;
        let changed = r#"{"document":"New.xaml"}"#;
        for path in [&stage, &scenario] {
            fs::write(path, original).unwrap();
        }
        let workspace = ProjectWorkspace::new(crate::Solution::discover(&directory).unwrap());
        let references = SavedReference::capture(
            &workspace,
            [&stage, &scenario]
                .into_iter()
                .map(|path| SavedReferenceEdit {
                    uri: file_uri(path).unwrap(),
                    before: original.into(),
                    after: changed.into(),
                })
                .collect(),
        )
        .unwrap();
        let mut first = references[0].prepare(&workspace, true).unwrap();
        let mut next = references[1].prepare(&workspace, true).unwrap();
        first.install().unwrap();
        fs::write(&scenario, "External edit during the move").unwrap();
        assert!(next.install().is_err());
        first.restore().unwrap();
        assert_eq!(fs::read_to_string(&stage).unwrap(), original);
        assert_eq!(
            fs::read_to_string(&scenario).unwrap(),
            "External edit during the move"
        );
        assert!(references[0].prepare(&workspace, true).is_ok());
        drop((first, next));
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
        fs::remove_dir_all(directory).unwrap();
    }
}
