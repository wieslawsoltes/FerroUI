//! Resolve Rust reference sites against the indexed Cargo packages and draft snapshots.
use super::{move_document, new_reference, FileMoveDocument, FileMoveDraft, LanguageService};
use crate::{require, LanguageError, LineIndex, Result};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_project::{
    canonical_file_uri, file_path, file_uri, relative_asset_uri, relative_file_path, resolve_uri,
    DocumentEntry, DocumentKind, ProjectDescriptor, ProjectWorkspace, RustPathAnchor, RustXamlPath,
    RustXamlReferences, Solution, XamlFileMove,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const MAX_DRAFT_BYTES: usize = 8 * 1024 * 1024;

impl LanguageService {
    pub(super) fn move_rust_paths(
        &self,
        movement: &XamlFileMove,
        drafts: &[FileMoveDraft],
    ) -> Result<Vec<FileMoveDocument>> {
        let drafts = rust_drafts(drafts)?;
        let files = rust_files(self);
        let from = file_path(&movement.from)?;
        let to = file_path(&movement.to)?;
        let mut builds = BTreeMap::new();
        let mut documents = Vec::new();
        for (entry, project, workspace) in files.values() {
            let version = drafts.get(&entry.uri).map_or(-1, |draft| draft.version);
            let saved;
            let source = if let Some(draft) = drafts.get(&entry.uri) {
                draft.source.as_str()
            } else {
                saved = workspace.read_source(&entry.uri)?;
                saved.as_str()
            };
            let package = project.authored_package_root(&entry.path);
            let build = is_cargo_build(entry, project, package, source, &mut builds)?;
            if !RustXamlReferences::may_contain_paths(source, build) {
                continue;
            }
            let references = match RustXamlReferences::parse(source, build) {
                Ok(references) => references,
                Err(_)
                    if from
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| source.contains(name)) =>
                {
                    return Err(LanguageError(format!(
                        "Repair the Rust draft that may reference this XAML file: {}",
                        entry.uri
                    )));
                }
                Err(_) => continue,
            };
            let roots = xaml_roots(package, &references);
            if references.discovers_xaml
                && roots
                    .iter()
                    .any(|root| from.starts_with(root) && !to.starts_with(root))
            {
                return Err(LanguageError(format!(
                    "The destination leaves the Cargo build's XAML discovery root: {}",
                    entry.uri
                )));
            }
            let context = RustMove {
                service: self,
                movement,
                source_file: entry,
                package,
                roots: &roots,
                destination: &to,
            };
            let mut patches = Vec::new();
            for site in &references.paths {
                if let Some(path) = context.replacement(site)? {
                    patches.push(TextPatch {
                        span: SourceSpan::new(site.range.start, site.range.end),
                        replacement: site.replacement(source, &path),
                    });
                }
            }
            patches.dedup_by(|later, earlier| {
                later.span == earlier.span && later.replacement == earlier.replacement
            });
            if !patches.is_empty() {
                let document =
                    move_document(&entry.uri, version, &LineIndex::new(source), patches)?;
                RustXamlReferences::parse(&document.after, build)?;
                documents.push(document);
            }
        }
        Ok(documents)
    }
}

fn rust_drafts(drafts: &[FileMoveDraft]) -> Result<BTreeMap<String, &FileMoveDraft>> {
    require(drafts.len() <= 256, "Too many Rust drafts for a file move")?;
    let mut snapshots = BTreeMap::new();
    let mut remaining = MAX_DRAFT_BYTES;
    for draft in drafts {
        require(
            draft.version >= 0 && draft.source.len() <= remaining,
            "Rust drafts for a file move exceed the source limit",
        )?;
        remaining -= draft.source.len();
        let uri = canonical_file_uri(&draft.uri)?;
        require(
            snapshots.insert(uri, draft).is_none(),
            "Multiple open buffers name the same Rust file",
        )?;
    }
    Ok(snapshots)
}

fn rust_files(
    service: &LanguageService,
) -> BTreeMap<&str, (&DocumentEntry, &ProjectDescriptor, &ProjectWorkspace)> {
    let mut files = BTreeMap::new();
    for workspace in service.workspaces.values() {
        for project in &workspace.solution.projects {
            for entry in project
                .documents
                .iter()
                .filter(|entry| entry.kind == DocumentKind::Rust)
            {
                let owner = files
                    .entry(entry.uri.as_str())
                    .or_insert((entry, project, workspace));
                if project.root.components().count() > owner.1.root.components().count() {
                    *owner = (entry, project, workspace);
                }
            }
        }
    }
    files
}

fn is_cargo_build(
    entry: &DocumentEntry,
    project: &ProjectDescriptor,
    package: &Path,
    source: &str,
    builds: &mut BTreeMap<PathBuf, Vec<PathBuf>>,
) -> Result<bool> {
    if package == project.root && !project.targets.is_empty() {
        return Ok(project.targets.iter().any(|target| {
            target.kind.iter().any(|kind| kind == "custom-build") && target.source == entry.path
        }));
    }
    if !source.contains("ferroui_build") && !source.contains("rerun-if-changed") {
        return Ok(false);
    }
    if !package.join("Cargo.toml").is_file() {
        return Ok(false);
    }
    if !builds.contains_key(package) {
        let solution = Solution::discover(package)?;
        let sources = solution
            .projects
            .iter()
            .filter(|project| project.root == package)
            .flat_map(|project| &project.targets)
            .filter(|target| target.kind.iter().any(|kind| kind == "custom-build"))
            .map(|target| target.source.clone())
            .collect();
        builds.insert(package.to_path_buf(), sources);
    }
    Ok(builds[package].contains(&entry.path))
}

fn xaml_roots(package: &Path, references: &RustXamlReferences) -> Vec<PathBuf> {
    if references.xaml_roots.is_empty() {
        return vec![package.to_path_buf()];
    }
    references
        .xaml_roots
        .iter()
        .filter_map(|directory| package.join(directory).canonicalize().ok())
        .collect()
}

struct RustMove<'a> {
    service: &'a LanguageService,
    movement: &'a XamlFileMove,
    source_file: &'a DocumentEntry,
    package: &'a Path,
    roots: &'a [PathBuf],
    destination: &'a Path,
}

impl RustMove<'_> {
    fn replacement(&self, site: &RustXamlPath) -> Result<Option<String>> {
        match site.anchor {
            RustPathAnchor::ResourceUri => self.resource(&site.path),
            RustPathAnchor::XamlRoot => self.key(site),
            anchor => {
                let base = self
                    .base(anchor)
                    .expect("file and Cargo references have a path base");
                if !self.refers_to_move(base, &site.path) {
                    return Ok(None);
                }
                if Path::new(&site.path).is_absolute() {
                    return Ok(Some(native_spelling(self.destination)?));
                }
                let relative = relative_file_path(base, self.destination)?;
                Ok(Some(retain_dot_prefix(&site.path, relative)))
            }
        }
    }

    fn key(&self, site: &RustXamlPath) -> Result<Option<String>> {
        if let Some((input, anchor)) = &site.input {
            if !self
                .base(*anchor)
                .is_some_and(|base| self.refers_to_move(base, input))
            {
                return Ok(None);
            }
        }
        for root in self.roots {
            let base = format!("{}/", file_uri(root)?);
            let resolved = resolve_uri(&base, site.path.trim_start_matches('/'))
                .and_then(|uri| canonical_file_uri(&uri));
            if resolved.ok().as_deref() != Some(self.movement.from.as_str()) {
                continue;
            }
            let relative = self.destination.strip_prefix(root).map_err(|_| {
                LanguageError(format!(
                    "The destination leaves the compiled XAML root: {}",
                    self.source_file.uri
                ))
            })?;
            let relative = native_spelling(relative)?;
            // The compiled loader compares the requested URI with the registered key.
            let relative = relative_asset_uri(&relative).ok_or_else(|| {
                LanguageError("The new compiled XAML key cannot be encoded".into())
            })?;
            return Ok(Some(if site.path.starts_with('/') {
                format!("/{relative}")
            } else {
                retain_dot_prefix(&site.path, relative)
            }));
        }
        Ok(None)
    }

    fn resource(&self, value: &str) -> Result<Option<String>> {
        let delimiter = if value.starts_with("resm:") {
            value.find('#')
        } else {
            value.find(['?', '#'])
        };
        let (uri, suffix) = value.split_at(delimiter.unwrap_or(value.len()));
        let owner = &self.source_file.uri;
        if self.service.resource_uri(owner, uri).ok().as_deref()
            != Some(self.movement.from.as_str())
        {
            return Ok(None);
        }
        let path = new_reference(uri, owner, &self.movement.to, self.movement)?;
        Ok(Some(format!("{path}{suffix}")))
    }

    fn base(&self, anchor: RustPathAnchor) -> Option<&Path> {
        match anchor {
            RustPathAnchor::SourceFile => self.source_file.path.parent(),
            RustPathAnchor::Package => Some(self.package),
            _ => None,
        }
    }

    fn refers_to_move(&self, base: &Path, path: &str) -> bool {
        base.join(path)
            .canonicalize()
            .ok()
            .and_then(|path| file_uri(path).ok())
            .as_deref()
            == Some(self.movement.from.as_str())
    }
}

fn native_spelling(path: &Path) -> Result<String> {
    path.to_str()
        .map(|path| path.replace(std::path::MAIN_SEPARATOR, "/"))
        .ok_or_else(|| LanguageError("The new path is not a UTF-8 filename".into()))
}

fn retain_dot_prefix(old: &str, relative: String) -> String {
    if old.starts_with("./") && !relative.starts_with("../") {
        format!("./{relative}")
    } else {
        relative
    }
}
