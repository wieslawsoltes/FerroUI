use crate::{
    file_path, file_uri, relative_asset_uri, require, ProjectError, ProjectWorkspace, Result,
};
use std::{
    io,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XamlFileMove {
    pub from: String,
    pub to: String,
    pub package_root: PathBuf,
}

impl ProjectWorkspace {
    /// Plan a physical move within one authored package. Open buffers can still
    /// use aliases to the workspace directory.
    pub fn plan_xaml_move(&self, from: &str, to: &str) -> Result<XamlFileMove> {
        let input = file_path(from)?;
        let source = self.resolve_path(from)?;
        require(
            std::fs::symlink_metadata(&input)?.is_file(),
            "Choose the original XAML file, rather than a file alias",
        )?;
        let target = self.resolve_path(to)?;
        require(
            target.parent().is_some_and(Path::is_dir),
            "Choose an existing destination directory",
        )?;
        require(
            is_xaml(&source) && is_xaml(&target),
            "Choose a .xaml filename",
        )?;
        match std::fs::symlink_metadata(&target) {
            Ok(_) => return Err(ProjectError("The move destination already exists".into())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let package = |path: &Path| {
            self.solution
                .projects
                .iter()
                .filter(|project| path.starts_with(&project.root))
                .max_by_key(|project| project.root.components().count())
        };
        let owner =
            package(&source).ok_or_else(|| ProjectError("Choose a project XAML file".into()))?;
        let root = owner.authored_package_root(&source).to_path_buf();
        require(
            package(&target)
                .is_some_and(|destination| destination.authored_package_root(&target) == root),
            "Choose a destination in the same Cargo package or loose-file project",
        )?;
        require(
            !target
                .strip_prefix(&root)
                .unwrap()
                .components()
                .any(|part| {
                    matches!(
                        part.as_os_str().to_str(),
                        Some(".git" | "target" | "node_modules" | "bin" | "obj")
                    )
                }),
            "Choose a destination in the authored project tree",
        )?;
        Ok(XamlFileMove {
            from: file_uri(source)?,
            to: file_uri(target)?,
            package_root: root,
        })
    }
}

fn is_xaml(path: &Path) -> bool {
    path.extension()
        .is_some_and(|suffix| suffix.eq_ignore_ascii_case("xaml"))
}

/// Encode an existing lexical file identity relative to a document's directory.
/// A different filesystem root has no document-relative spelling.
pub fn relative_file_reference(from: &str, to: &str) -> Result<String> {
    let source = file_path(from)?;
    let parent = source
        .parent()
        .ok_or_else(|| ProjectError("Document has no parent directory".into()))?;
    relative_asset_uri(&relative_file_path(parent, &file_path(to)?)?)
        .ok_or_else(|| ProjectError("Cannot represent a relative file URI".into()))
}

/// Spell a native path from a base directory, with portable slash separators.
pub fn relative_file_path(base: &Path, target: &Path) -> Result<String> {
    let mut start = base.components().peekable();
    let mut end = target.components().peekable();
    while start.peek().is_some() && start.peek() == end.peek() {
        start.next();
        end.next();
    }
    let mut path = Vec::new();
    for part in start {
        require(
            matches!(part, Component::Normal(_)),
            "Files have different filesystem roots",
        )?;
        path.push("..");
    }
    for part in end {
        let name = match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        }
        .ok_or_else(|| ProjectError("Cannot represent a relative file URI".into()))?;
        path.push(name);
    }
    Ok(path.join("/"))
}
