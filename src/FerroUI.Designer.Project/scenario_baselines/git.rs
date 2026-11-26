use super::MAX_PNG_BYTES;
use crate::{require, ProjectError, Result};
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Read-only identity of the worktree and its selected review ancestor.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Repository {
    root: PathBuf,
    head: Option<String>,
    branch: Option<String>,
    ancestor: Option<String>,
}

impl Repository {
    pub fn open(workspace: &Path, reference: Option<&str>) -> Result<Option<Self>> {
        let root = match text(workspace, &["rev-parse", "--show-toplevel"]) {
            Ok(root) => Path::new(&root).canonicalize()?,
            Err(_) if reference.is_none() => return Ok(None),
            Err(error) => {
                return Err(ProjectError(format!(
                    "Git review needs a local worktree: {error}"
                )))
            }
        };
        let head = commit(&root, "HEAD").ok();
        let branch = text(workspace, &["symbolic-ref", "--quiet", "HEAD"]).ok();
        let ancestor = reference.map(|reference| {
            require(!reference.trim().is_empty(), "Choose a Git branch, tag, or commit")?;
            let head = head.as_deref().ok_or_else(|| ProjectError("Commit the current Git branch before comparing it".into()))?;
            let base = commit(workspace, reference).map_err(|_| ProjectError(format!("Git reference {reference:?} is unavailable locally")))?;
            let bases = text(workspace, &["merge-base", "--all", head, &base])
                .map_err(|_| ProjectError("The selected Git references have no available merge base. Check the repository history depth.".into()))?;
            require(object_id(&bases), "Git found multiple merge bases. Choose an unambiguous reference.")?;
            Ok::<_, ProjectError>(bases)
        }).transpose()?;
        Ok(Some(Self {
            root,
            head,
            branch,
            ancestor,
        }))
    }

    pub fn ancestor(&self) -> Option<&str> {
        self.ancestor.as_deref()
    }

    pub fn baseline(&self, path: &Path) -> Result<Option<Vec<u8>>> {
        let ancestor = self
            .ancestor()
            .ok_or_else(|| ProjectError("Choose a Git review reference".into()))?;
        let relative = path.strip_prefix(&self.root).map_err(|_| {
            ProjectError("Git baselines must belong to the selected worktree".into())
        })?;
        let relative = relative
            .to_str()
            .ok_or_else(|| ProjectError("Use a UTF-8 baseline path".into()))?;
        let relative = if cfg!(windows) {
            relative.replace('\\', "/")
        } else {
            relative.to_owned()
        };
        let entry = bytes(
            &self.root,
            &[
                "--literal-pathspecs",
                "ls-tree",
                "-z",
                ancestor,
                "--",
                &relative,
            ],
            32 * 1024,
        )?;
        if entry.is_empty() {
            return Ok(None);
        }
        let separator = entry
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(|| ProjectError("Git returned an invalid baseline path".into()))?;
        let (metadata, filename) = entry.split_at(separator);
        require(
            filename[1..].strip_suffix(&[0]) == Some(relative.as_bytes()),
            "Git returned an unexpected baseline path",
        )?;
        let metadata =
            std::str::from_utf8(metadata).map_err(|error| ProjectError(error.to_string()))?;
        let mut fields = metadata.split_ascii_whitespace();
        let mode = fields.next();
        let kind = fields.next();
        let oid = fields.next();
        require(
            matches!(mode, Some("100644" | "100755"))
                && kind == Some("blob")
                && oid.is_some_and(object_id),
            "Git baseline must be a regular PNG file",
        )?;
        bytes(
            &self.root,
            &["cat-file", "blob", oid.expect("validated Git object")],
            MAX_PNG_BYTES,
        )
        .map(Some)
    }
}

fn commit(directory: &Path, name: &str) -> Result<String> {
    let id = text(
        directory,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{name}^{{commit}}"),
        ],
    )?;
    require(object_id(&id), "Git returned an invalid commit identity")?;
    Ok(id)
}

fn object_id(input: &str) -> bool {
    matches!(input.len(), 40 | 64) && input.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn text(directory: &Path, args: &[&str]) -> Result<String> {
    let value = bytes(directory, args, 32 * 1024)?;
    String::from_utf8(value)
        .map(|value| value.trim_end_matches(['\r', '\n']).to_owned())
        .map_err(|error| ProjectError(error.to_string()))
}

fn bytes(directory: &Path, args: &[&str], limit: u64) -> Result<Vec<u8>> {
    let mut child = Command::new("git")
        .current_dir(directory)
        .arg("--no-replace-objects")
        .args(args)
        .env("GIT_NO_LAZY_FETCH", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut output = Vec::new();
    let reading = child
        .stdout
        .take()
        .expect("piped Git output")
        .take(limit + 1)
        .read_to_end(&mut output);
    if reading.is_err() || output.len() as u64 > limit {
        let _ = child.kill();
    }
    let status = child.wait()?;
    reading?;
    require(
        output.len() as u64 <= limit,
        "Git baseline exceeds its size limit",
    )?;
    require(
        status.success(),
        format!("Git {} exited with {status}", args[0]),
    )?;
    Ok(output)
}
