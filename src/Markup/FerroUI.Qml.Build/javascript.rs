//! Embed local JavaScript resources in the compiled component. Stable virtual
//! URLs preserve module identity through cycles without encoding host paths.
use ferroui_qml_compiler::{
    javascript::visit_scripts, script_resource, Binding, Document, Expr, Member, MemberKind,
    Object, ScriptKind, ScriptResource,
};
use percent_encoding::{percent_encode, AsciiSet, NON_ALPHANUMERIC};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum LoadKind {
    Classic,
    Module,
}

pub(super) struct Resources {
    project: PathBuf,
    paths: BTreeMap<(PathBuf, LoadKind), String>,
    embedded: BTreeMap<String, ScriptResource>,
    preload: BTreeSet<String>,
}

impl Resources {
    pub(super) fn new(
        input: &Path,
        entries: &BTreeSet<PathBuf>,
        workers: &BTreeSet<PathBuf>,
    ) -> Result<Self, String> {
        let input = input
            .canonicalize()
            .map_err(|err| format!("{}: {err}", input.display()))?;
        let project = input
            .parent()
            .ok_or("entry QML file has no parent directory")?
            .to_owned();
        let mut resources = Self {
            project,
            paths: BTreeMap::new(),
            embedded: BTreeMap::new(),
            preload: BTreeSet::new(),
        };
        for (entry, kind) in entries
            .iter()
            .map(|p| (p, LoadKind::Module))
            .chain(workers.iter().map(|p| (p, LoadKind::Classic)))
        {
            if entry.is_absolute()
                || entry.has_root()
                || matches!(
                    entry.components().next(),
                    Some(std::path::Component::Prefix(_))
                )
            {
                return Err(format!(
                    "JavaScript entry must be relative to the QML project: {}",
                    entry.display()
                ));
            }
            let file = resources
                .project
                .join(entry)
                .canonicalize()
                .map_err(|err| format!("{}: {err}", entry.display()))?;
            if !file.starts_with(&resources.project) {
                return Err(format!(
                    "JavaScript entry escapes the QML project: {}",
                    entry.display()
                ));
            }
            if kind == LoadKind::Classic {
                resources.load_worker(&file)?;
            } else {
                resources.load(&file, 0, kind)?;
            }
        }
        Ok(resources)
    }

    pub(super) fn watched(&self) -> impl Iterator<Item = &PathBuf> {
        self.paths.keys().map(|(path, _)| path)
    }

    pub(super) fn attach(
        &mut self,
        path: &Path,
        index: usize,
        mut document: Document,
    ) -> Result<Document, String> {
        let mut imports = BTreeMap::new();
        for import in document.imports.iter().filter(|i| i.is_script()) {
            let alias = import
                .alias
                .as_ref()
                .ok_or("JavaScript imports require a namespace")?;
            let file = local_path(path, &import.uri)?;
            let key = self.load(&file, 0, LoadKind::Classic)?;
            self.preload.insert(key.clone());
            imports.insert(alias.clone(), key);
        }
        let origin = resource_url("components", index, path, &self.project)?;
        let imports = std::sync::Arc::new(imports);
        self.worker_sources(path, &mut document.root)?;
        visit_scripts(&mut document.root, &mut |script| {
            script_resource::attach(script, &origin, &imports, |specifier| {
                self.load(&local_path(path, specifier)?, 0, LoadKind::Module)
            })
        })?;
        Ok(document)
    }

    /// A worker has an independent global scope. Discover its literal file
    /// graph without initializing the script in the importing QML realm.
    fn worker_sources(&mut self, file: &Path, object: &mut Object) -> Result<(), String> {
        let worker = object.type_name.rsplit('.').next() == Some("WorkerScript");
        self.worker_members(file, &mut object.members, worker)
    }

    fn worker_members(
        &mut self,
        file: &Path,
        members: &mut [Member],
        worker: bool,
    ) -> Result<(), String> {
        for member in members {
            match &mut member.kind {
                MemberKind::Binding {
                    name,
                    value: Binding::Expr(Expr::String(source)),
                } if worker && name == "source" && !source.is_empty() => {
                    let local = local_path(file, source)?;
                    let url = self.load_worker(&local)?;
                    *source = url;
                }
                MemberKind::Child(child)
                | MemberKind::Component { object: child, .. }
                | MemberKind::OnProperty { object: child, .. } => {
                    self.worker_sources(file, child)?
                }
                MemberKind::Group { members, .. } => self.worker_members(file, members, false)?,
                MemberKind::Property {
                    value: Some(binding),
                    ..
                }
                | MemberKind::Binding { value: binding, .. } => match binding {
                    Binding::Object(child) => self.worker_sources(file, child)?,
                    Binding::Objects(children) => {
                        for child in children {
                            self.worker_sources(file, child)?;
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(())
    }

    fn load_worker(&mut self, file: &Path) -> Result<String, String> {
        let url = self.load(file, 0, LoadKind::Classic)?;
        let resource = &self.embedded[&url];
        if resource.kind != ScriptKind::Module
            && (resource.kind == ScriptKind::Library || !resource.imports.is_empty())
        {
            return Err(format!(
                "{}: plain WorkerScript files cannot use .import or .pragma library",
                file.display()
            ));
        }
        Ok(url)
    }

    pub(super) fn into_embedded(mut self) -> BTreeMap<String, ScriptResource> {
        for key in self.preload {
            if let Some(resource) = self.embedded.get_mut(&key) {
                resource.preload = true;
            }
        }
        self.embedded
    }

    fn load(&mut self, path: &Path, depth: usize, kind: LoadKind) -> Result<String, String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let kind = if ext == "mjs" { LoadKind::Module } else { kind };
        let identity = (path.clone(), kind);
        if let Some(existing) = self.paths.get(&identity) {
            return Ok(existing.clone());
        }
        if depth >= 128 {
            return Err("JavaScript import nesting limit exceeded".into());
        }

        if !matches!(ext, "js" | "mjs") {
            return Err(format!(
                "unsupported JavaScript resource extension: {}",
                path.display()
            ));
        }
        let source = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let directory = match kind {
            LoadKind::Module => "modules",
            LoadKind::Classic => "modules/classic",
        };
        let key = resource_url(directory, self.paths.len(), &path, &self.project)?;
        self.paths.insert(identity, key.clone());
        let resource = script_resource::compile(
            &key,
            &source,
            kind == LoadKind::Module,
            |specifier, module| {
                self.load(
                    &local_path(&path, specifier)?,
                    depth + 1,
                    if module {
                        LoadKind::Module
                    } else {
                        LoadKind::Classic
                    },
                )
            },
        )
        .map_err(|e| format!("{}:{e}", path.display()))?;
        self.embedded.insert(key.clone(), resource);
        Ok(key)
    }
}

/// Preserve directory relationships inside the project, so dynamic import()
/// resolves relative to its actual importer. External, statically linked files
/// retain opaque identities without exposing their host directory names.
fn resource_url(
    directory: &str,
    index: usize,
    path: &Path,
    project: &Path,
) -> Result<String, String> {
    const ENCODE: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'.')
        .remove(b'_')
        .remove(b'~');
    if let Ok(relative) = path.strip_prefix(project) {
        let mut url = format!("https://qml.invalid/{directory}/project");
        for segment in relative {
            url.push('/');
            url.push_str(&percent_encode(segment.as_encoded_bytes(), ENCODE).to_string());
        }
        return Ok(url);
    }
    let name = path
        .file_name()
        .ok_or("JavaScript resource has no file name")?;
    let segment = percent_encode(name.as_encoded_bytes(), ENCODE);
    Ok(format!("https://qml.invalid/{directory}/{index}/{segment}"))
}

fn local_path(origin: &Path, specifier: &str) -> Result<PathBuf, String> {
    if specifier.is_empty()
        || specifier.starts_with('/')
        || specifier.contains([':', '?', '#', '\\'])
        || specifier.chars().any(char::is_control)
    {
        return Err(format!(
            "expected a relative JavaScript resource path: {specifier}"
        ));
    }
    Ok(origin
        .parent()
        .ok_or("JavaScript resource has no containing directory")?
        .join(specifier))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_use_relative_url_paths_on_every_platform() {
        let origin = Path::new("app").join("View.qml");
        for path in [
            "/absolute.mjs",
            "file:///abs.mjs",
            "C:/file.js",
            r"\\host\share\file.js",
            r"dir\name.mjs",
            "./file?x",
            "./file#id",
        ] {
            assert!(local_path(&origin, path).is_err(), "{path}");
        }
        assert_eq!(
            local_path(&origin, "../lib/math.mjs").unwrap(),
            Path::new("app").join("../lib/math.mjs")
        );
    }
}
