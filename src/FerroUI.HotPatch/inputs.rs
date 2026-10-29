//! Capture explicit build-input trees before admission, and stage their exact bytes.
//!
//! This is not Cargo dependency discovery. The caller must include every local path
//! dependency inside the root and supply compiler/configuration/environment inputs.
//! Builds must place output outside the staged tree. Network inputs, generated inputs
//! outside the tree, and mutable external dependencies require restart.
use crate::SourceSnapshot;
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Component, Path, PathBuf},
};

/// An owned, byte-exact revision. Rust sources remain text for function admission;
/// every other input is encoded so binary assets are covered without lossy decoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildInputs {
    files: BTreeMap<String, Vec<u8>>,
    #[cfg(unix)]
    modes: BTreeMap<String, u32>,
    context: BTreeMap<String, String>,
}

impl BuildInputs {
    /// Read all regular files recursively. Exclusions are explicit root-relative
    /// directories, normally `.git` and the build output directory. Never exclude
    /// a dependency or compiler input. Symlinks are rejected instead of silently
    /// overlooking external inputs or making the staged build resolve differently.
    /// Two identical reads reject ordinary edits during capture; the staged bytes,
    /// rather than the mutable source directory, must be used for compilation.
    pub fn capture(
        root: &Path,
        excluded_directories: &[PathBuf],
        context: BTreeMap<String, String>,
    ) -> io::Result<Self> {
        for path in excluded_directories {
            if path.as_os_str().is_empty()
                || path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(invalid(
                    "Exclusions must be nonempty relative directory paths",
                ));
            }
        }
        if fs::symlink_metadata(root)?.file_type().is_symlink() || !root.is_dir() {
            return Err(invalid("Input root must be a real directory"));
        }
        let read = || {
            let mut files = BTreeMap::new();
            let mut modes = BTreeMap::new();
            collect(
                root,
                Path::new(""),
                excluded_directories,
                &mut files,
                &mut modes,
            )?;
            Ok::<_, io::Error>((files, modes))
        };
        let captured = read()?;
        if captured != read()? {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Build inputs changed during capture",
            ));
        }
        Ok(Self {
            files: captured.0,
            #[cfg(unix)]
            modes: captured.1,
            context,
        })
    }

    /// Exact comparison inputs for `PatchSupervisor`. Context keys live outside the
    /// filesystem namespace and must include toolchain identity and relevant env.
    pub fn snapshot(&self) -> io::Result<SourceSnapshot> {
        let mut result = BTreeMap::new();
        for (name, bytes) in &self.files {
            let value = if name.ends_with(".rs") {
                String::from_utf8(bytes.clone()).map_err(|_| invalid("Rust input is not UTF-8"))?
            } else {
                let mut encoded = String::with_capacity(bytes.len() * 2);
                const HEX: &[u8] = b"0123456789abcdef";
                for byte in bytes {
                    encoded.push(HEX[(byte >> 4) as usize] as char);
                    encoded.push(HEX[(byte & 15) as usize] as char);
                }
                encoded
            };
            result.insert(name.clone(), value);
        }
        for (name, value) in &self.context {
            result.insert(format!("@context/{name}"), value.clone());
        }
        #[cfg(unix)]
        for (name, mode) in &self.modes {
            result.insert(format!("@permissions/{name}"), mode.to_string());
        }
        Ok(result)
    }

    /// Create an isolated source tree from the captured bytes, never by rereading
    /// live sources. Keep this object alive for the entire build and verification.
    pub fn stage(&self) -> io::Result<StagedInputs> {
        let directory = tempfile::tempdir()?;
        for (name, bytes) in &self.files {
            let path = directory.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, bytes)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(self.modes[name]))?;
            }
        }
        Ok(StagedInputs {
            directory,
            inputs: self.clone(),
        })
    }
}

/// An isolated build tree. Build scripts can still write to it: verification fails
/// closed if they do. This is source isolation, not a sandbox for arbitrary builds.
pub struct StagedInputs {
    directory: tempfile::TempDir,
    inputs: BuildInputs,
}
impl StagedInputs {
    pub fn path(&self) -> &Path {
        self.directory.path()
    }
    pub fn inputs(&self) -> &BuildInputs {
        &self.inputs
    }
    pub fn verify(&self) -> io::Result<()> {
        let current = BuildInputs::capture(self.path(), &[], self.inputs.context.clone())?;
        if current != self.inputs {
            return Err(invalid("Staged build inputs changed; discard the artifact"));
        }
        Ok(())
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn collect(
    root: &Path,
    relative: &Path,
    excluded: &[PathBuf],
    files: &mut BTreeMap<String, Vec<u8>>,
    modes: &mut BTreeMap<String, u32>,
) -> io::Result<()> {
    for entry in fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let path = relative.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() && excluded.contains(&path) {
            continue;
        }
        if kind.is_symlink() {
            return Err(invalid(
                "Symlink build inputs require an explicit resolved input tree",
            ));
        }
        if kind.is_dir() {
            collect(root, &path, excluded, files, modes)?;
        } else if kind.is_file() {
            let name = path
                .components()
                .map(|part| {
                    part.as_os_str()
                        .to_str()
                        .ok_or_else(|| invalid("Input path is not UTF-8"))
                })
                .collect::<io::Result<Vec<_>>>()?
                .join("/");
            // Reserve context keys and reject names whose meaning differs on Windows.
            if name.starts_with("@context/")
                || name.starts_with("@permissions/")
                || name.contains(['\\', ':'])
            {
                return Err(invalid(
                    "Input path is not portable or uses the context namespace",
                ));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                modes.insert(name.clone(), entry.metadata()?.permissions().mode() & 0o777);
            }
            files.insert(name, fs::read(entry.path())?);
        } else {
            return Err(invalid("Build inputs must be regular files or directories"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PatchSupervisor;
    use std::collections::BTreeSet;
    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("main.rs"), "fn click() { let x = 1; }").unwrap();
        fs::write(root.path().join("Cargo.toml"), "manifest").unwrap();
        fs::write(root.path().join("asset.bin"), [0, 255]).unwrap();
        root
    }
    fn capture(root: &Path) -> BuildInputs {
        BuildInputs::capture(
            root,
            &[],
            BTreeMap::from([("rustc".into(), "1.93.0".into())]),
        )
        .unwrap()
    }
    #[test]
    fn admitted_revision_is_isolated_from_later_saves_and_detects_build_writes() {
        let root = fixture();
        let baseline = capture(root.path());
        let supervisor = PatchSupervisor::new(
            baseline.snapshot().unwrap(),
            BTreeMap::from([("main.rs".into(), BTreeSet::from(["click".into()]))]),
        );
        fs::write(root.path().join("main.rs"), "fn click() { let x = 2; }").unwrap();
        let admitted = capture(root.path());
        assert!(supervisor.admit(admitted.snapshot().unwrap()).is_ok());
        fs::write(root.path().join("main.rs"), "struct Incompatible;").unwrap();
        let staged = admitted.stage().unwrap();
        assert_eq!(
            fs::read_to_string(staged.path().join("main.rs")).unwrap(),
            "fn click() { let x = 2; }"
        );
        staged.verify().unwrap();
        fs::write(staged.path().join("asset.bin"), [0, 254]).unwrap();
        assert!(staged.verify().is_err());
    }
    #[test]
    fn dependencies_binary_assets_and_configuration_require_restart() {
        let root = fixture();
        let before = capture(root.path());
        let supervisor = PatchSupervisor::new(before.snapshot().unwrap(), BTreeMap::new());
        fs::write(root.path().join("asset.bin"), [0, 254]).unwrap();
        assert!(supervisor
            .admit(capture(root.path()).snapshot().unwrap())
            .unwrap_err()
            .contains("asset.bin"));
        let mut changed = before.clone();
        changed.context.insert("rustc".into(), "other".into());
        assert!(supervisor
            .admit(changed.snapshot().unwrap())
            .unwrap_err()
            .contains("@context/rustc"));
        fs::write(root.path().join("new.rs"), "fn new() {}").unwrap();
        assert!(supervisor
            .admit(capture(root.path()).snapshot().unwrap())
            .unwrap_err()
            .contains("input set"));
    }
    #[test]
    fn exclusions_are_explicit_directories_and_cannot_escape() {
        let root = fixture();
        fs::create_dir(root.path().join("target")).unwrap();
        fs::write(root.path().join("target/output"), "output").unwrap();
        let inputs =
            BuildInputs::capture(root.path(), &["target".into()], BTreeMap::new()).unwrap();
        assert!(!inputs.snapshot().unwrap().contains_key("target/output"));
        assert!(
            BuildInputs::capture(root.path(), &["../external".into()], BTreeMap::new()).is_err()
        );
        let stage = inputs.stage().unwrap();
        fs::write(stage.path().join("generated.rs"), "generated").unwrap();
        assert!(stage.verify().is_err());
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_fail_closed() {
        let root = fixture();
        std::os::unix::fs::symlink("main.rs", root.path().join("linked.rs")).unwrap();
        assert!(BuildInputs::capture(root.path(), &[], BTreeMap::new()).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn executable_permissions_are_preserved_and_admitted_as_build_inputs() {
        use std::os::unix::fs::PermissionsExt;
        let root = fixture();
        let path = root.path().join("build.sh");
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let before = capture(root.path());
        let staged = before.stage().unwrap();
        assert_eq!(
            fs::metadata(staged.path().join("build.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        staged.verify().unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let supervisor = PatchSupervisor::new(before.snapshot().unwrap(), BTreeMap::new());
        assert!(supervisor
            .admit(capture(root.path()).snapshot().unwrap())
            .unwrap_err()
            .contains("@permissions/build.sh"));
    }
}
