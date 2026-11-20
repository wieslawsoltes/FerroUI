//! Resolve Rust reference sites against the indexed Cargo packages and draft snapshots.
use super::{move_document, new_reference, FileMoveDocument, FileMoveDraft, LanguageService};
use crate::{LanguageError, LineIndex, Result};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_project::{
    canonical_file_uri, file_path, file_uri, relative_asset_uri, relative_file_path, resolve_uri,
    DocumentEntry, DocumentKind, ProjectDescriptor, ProjectWorkspace, RustPathAnchor, RustXamlPath,
    RustXamlReferences, Solution, XamlFileMove,
};
use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

#[path = "file_move_compiled.rs"]
mod compiled;
#[path = "file_move_inputs.rs"]
mod inputs;
pub(super) use compiled::CompiledAddresses;
pub use compiled::GeneratedRustRename;

impl LanguageService {
    pub(super) fn move_rust_paths(
        &self,
        movement: &XamlFileMove,
        drafts: &BTreeMap<String, &FileMoveDraft>,
        addresses: &CompiledAddresses,
        review: &mut impl Hasher,
    ) -> Result<Vec<FileMoveDocument>> {
        let files = rust_files(self);
        let from = file_path(&movement.from)?;
        let to = file_path(&movement.to)?;
        let mut builds = BTreeMap::new();
        let mut documents = Vec::new();
        for (entry, project, workspace) in files.values() {
            if addresses.is_generated(&entry.path) {
                continue;
            }
            let version = drafts.get(&entry.uri).map_or(-1, |draft| draft.version);
            let cached_build = addresses.build_for(&entry.uri);
            let saved;
            let source = if let Some(snapshot) = cached_build {
                snapshot.source.as_str()
            } else if let Some(draft) = drafts.get(&entry.uri) {
                draft.source.as_str()
            } else {
                saved = workspace.read_source(&entry.uri)?;
                saved.as_str()
            };
            if !addresses.generated_rust.is_empty() {
                // Bind asynchronously resolved Rust references to these exact buffers.
                (&entry.uri, version, source).hash(review);
            }
            let package = if cached_build.is_some() {
                movement.package_root.as_path()
            } else {
                project.authored_package_root(&entry.path)
            };
            let build = cached_build.is_some()
                || is_cargo_build(entry, project, package, source, &mut builds)?;
            let compiled_inputs = addresses.file_inputs.get(&entry.uri);
            let build_sources = build || compiled_inputs.is_some();
            if !RustXamlReferences::may_contain_paths(source, build_sources) {
                continue;
            }
            let parsed;
            let references = if let Some(snapshot) = cached_build {
                &snapshot.references
            } else {
                parsed = match RustXamlReferences::parse(source, build_sources) {
                    Ok(references) => references,
                    Err(_)
                        if compiled_inputs.is_some()
                            || from
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
                &parsed
            };
            if cached_build.is_some()
                && !addresses.has_compiler_index
                && (references.uses_file_documents
                    || references
                        .file_documents
                        .iter()
                        .any(|input| refers_to(package, &input.site.path, &movement.from)))
            {
                return Err(LanguageError(
                    "Build the project XAML schema to resolve its file-backed registrations".into(),
                ));
            }
            let file_paths = match compiled_inputs {
                Some(inputs) => inputs::paths(inputs, source, references, movement)?,
                None => Vec::new(),
            };
            let roots = xaml_roots(package, references);
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
                addresses,
                source_file: entry,
                package,
                roots: &roots,
                destination: &to,
            };
            let mut patches = Vec::new();
            for site in references.paths.iter().chain(file_paths) {
                if let Some(path) = context.replacement(site)? {
                    patches.push(TextPatch {
                        span: SourceSpan::new(site.range.start, site.range.end),
                        replacement: site.replacement(source, &path),
                    });
                }
            }
            patches.sort_by_key(|patch| (patch.span.start, patch.span.end));
            patches.dedup_by(|later, earlier| {
                later.span == earlier.span && later.replacement == earlier.replacement
            });
            if !patches.is_empty() {
                let document =
                    move_document(&entry.uri, version, &LineIndex::new(source), patches)?;
                RustXamlReferences::parse(&document.after, build_sources)?;
                documents.push(document);
            }
        }
        Ok(documents)
    }
}

pub(super) fn rust_files(
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
    addresses: &'a CompiledAddresses,
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
        let Some((input, anchor)) = &site.input else {
            // Discovery scans below the configured root, and selects the class
            // by its raw path relative to the manifest.
            return if self.refers_to_move(self.package, &site.path)
                && !self.addresses.preserves_class_key(&site.path)?
            {
                Ok(Some(relative_file_path(self.package, self.destination)?))
            } else {
                Ok(None)
            };
        };
        if !self
            .base(*anchor)
            .is_some_and(|base| self.refers_to_move(base, input))
        {
            return Ok(None);
        }
        let next = compiled_key(
            self.movement,
            self.destination,
            self.roots,
            &self.source_file.uri,
            &site.path,
            site.shares_input_path,
        )?;
        if site.shares_input_path && next.is_some() {
            let path =
                retain_dot_prefix(input, relative_file_path(self.package, self.destination)?);
            if next.as_ref() == Some(&path) {
                return Ok(None);
            }
        }
        Ok(next)
    }

    fn resource(&self, value: &str) -> Result<Option<String>> {
        let delimiter = if value.starts_with("resm:") {
            value.find('#')
        } else {
            value.find(['?', '#'])
        };
        let (uri, suffix) = value.split_at(delimiter.unwrap_or(value.len()));
        let owner = &self.source_file.uri;
        if let Some(next) = self.addresses.replacement(self.service, owner, uri) {
            return Ok((next != uri).then(|| format!("{next}{suffix}")));
        }
        if self.service.resource_uri(owner, uri).ok().as_deref()
            != Some(self.movement.from.as_str())
        {
            return Ok(None);
        }
        let path = new_reference(uri, owner, &self.movement.to, self.movement)?;
        Ok(Some(format!("{path}{suffix}")))
    }

    fn base(&self, anchor: RustPathAnchor) -> Option<&Path> {
        input_base(self.package, &self.source_file.path, anchor)
    }

    fn refers_to_move(&self, base: &Path, path: &str) -> bool {
        refers_to(base, path, &self.movement.from)
    }
}

fn compiled_key(
    movement: &XamlFileMove,
    destination: &Path,
    roots: &[PathBuf],
    script: &str,
    key: &str,
    shares_input_path: bool,
) -> Result<Option<String>> {
    if shares_input_path {
        let relative = relative_file_path(&movement.package_root, destination)?;
        return relative_asset_uri(&relative)
            .map(Some)
            .ok_or_else(|| LanguageError("The new compiled XAML key cannot be encoded".into()));
    }
    for root in roots {
        let base = format!("{}/", file_uri(root)?);
        let resolved = resolve_uri(&base, key.trim_start_matches('/'))
            .and_then(|uri| canonical_file_uri(&uri));
        if resolved.ok().as_deref() != Some(movement.from.as_str()) {
            continue;
        }
        let relative = destination.strip_prefix(root).map_err(|_| {
            LanguageError(format!(
                "The destination leaves the compiled XAML root: {script}"
            ))
        })?;
        let relative = native_spelling(relative)?;
        // The compiled loader compares the requested URI with the registered key.
        let relative = relative_asset_uri(&relative)
            .ok_or_else(|| LanguageError("The new compiled XAML key cannot be encoded".into()))?;
        return Ok(Some(if key.starts_with('/') {
            format!("/{relative}")
        } else {
            retain_dot_prefix(key, relative)
        }));
    }
    Ok(None)
}

fn input_base<'a>(package: &'a Path, script: &'a Path, anchor: RustPathAnchor) -> Option<&'a Path> {
    match anchor {
        RustPathAnchor::SourceFile => script.parent(),
        RustPathAnchor::Package => Some(package),
        _ => None,
    }
}

fn refers_to(base: &Path, path: &str, uri: &str) -> bool {
    base.join(path)
        .canonicalize()
        .ok()
        .and_then(|path| file_uri(path).ok())
        .as_deref()
        == Some(uri)
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
