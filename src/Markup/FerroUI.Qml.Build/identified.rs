//! Resolve local identified modules and record every input used by Cargo.
//! A manifest controls the public type names even when additional .qml files
//! happen to be present in the directory.
use ferroui_qml_compiler::{
    qmldir::{
        parse_directory_manifest, ExportKind, ImportVersion, ModuleDefinition, RequestedVersion,
        Version,
    },
    Import,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::rc::Rc;

type Result<T> = std::result::Result<T, String>;

#[derive(Clone)]
struct Module {
    dir: PathBuf,
    manifest: Rc<ModuleDefinition>,
    version: Option<Version>,
}

impl Module {
    fn selected(&self) -> RequestedVersion {
        self.version
            .map(RequestedVersion::Exact)
            .unwrap_or_default()
    }
}

enum Dependency {
    Module(Module),
    Native(Import),
}

pub(crate) struct IdentifiedModules {
    roots: Vec<PathBuf>,
    manifests: BTreeMap<PathBuf, Option<Rc<ModuleDefinition>>>,
    /// An exported source may have a filename unrelated to its public type name.
    /// Its sibling lookup inherits the originating manifest and version.
    origins: BTreeMap<PathBuf, Module>,
    watched: BTreeSet<PathBuf>,
}

impl IdentifiedModules {
    pub(crate) fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            manifests: BTreeMap::new(),
            origins: BTreeMap::new(),
            watched: BTreeSet::new(),
        }
    }

    pub(crate) fn watched(&self) -> impl Iterator<Item = &Path> {
        self.watched.iter().map(PathBuf::as_path)
    }

    fn manifest(&mut self, dir: &Path) -> Result<Option<Rc<ModuleDefinition>>> {
        let path = dir.join("qmldir");
        if let Some(cached) = self.manifests.get(&path) {
            return Ok(cached.clone());
        }
        let loaded = match fs::read_to_string(&path) {
            Ok(source) => {
                let definition = parse_directory_manifest(&source)
                    .map_err(|error| format!("{}:{error}", path.display()))?;
                self.watched.insert(path.clone());
                Some(Rc::new(definition))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                if let Some(parent) = dir.ancestors().find(|candidate| candidate.is_dir()) {
                    self.watched.insert(parent.to_owned());
                }
                None
            }
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        self.manifests.insert(path, loaded.clone());
        Ok(loaded)
    }

    fn select(
        &self,
        dir: &Path,
        manifest: Rc<ModuleDefinition>,
        request: RequestedVersion,
    ) -> Result<Module> {
        if let Some(plugin) = manifest.plugins.iter().find(|p| !p.optional) {
            return Err(format!(
                "QML module '{}' requires unsupported native Qt plugin '{}'",
                manifest.uri, plugin.name
            ));
        }
        let version = manifest.select(request)?.version;
        Ok(Module {
            dir: dir.to_owned(),
            manifest,
            version,
        })
    }

    fn locate(&mut self, uri: &str, request: RequestedVersion) -> Result<Option<Module>> {
        let mut relative = PathBuf::new();
        for segment in uri.split('.') {
            if segment.is_empty()
                || !segment
                    .chars()
                    .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
            {
                return Err(format!("invalid module URI '{uri}'"));
            }
            relative.push(segment);
        }
        let mut candidates = Vec::new();
        if let RequestedVersion::Exact(v) = request {
            candidates.push(relative.with_extension(v.to_string()));
        }
        match request {
            RequestedVersion::Major(major) | RequestedVersion::Exact(Version { major, .. }) => {
                candidates.push(relative.with_extension(major.to_string()))
            }
            RequestedVersion::Latest => (),
        }
        candidates.push(relative);
        for i in 0..self.roots.len() {
            for suffix in &candidates {
                let dir = self.roots[i].join(suffix);
                if let Some(manifest) = self.manifest(&dir)? {
                    if manifest.uri != uri {
                        return Err(format!(
                            "{} declares module '{}' but '{uri}' was requested",
                            dir.join("qmldir").display(),
                            manifest.uri
                        ));
                    }
                    return self.select(&dir, manifest, request).map(Some);
                }
            }
        }
        Ok(None)
    }

    fn closure(&mut self, import: &Import) -> Result<Vec<Dependency>> {
        let mut result = Vec::new();
        self.visit(
            import,
            &mut result,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
        )?;
        Ok(result)
    }

    fn visit(
        &mut self,
        import: &Import,
        result: &mut Vec<Dependency>,
        seen: &mut BTreeSet<(String, Option<String>)>,
        stack: &mut BTreeSet<String>,
    ) -> Result<()> {
        if stack.len() >= 64 {
            return Err("qmldir import nesting limit exceeded".into());
        }
        if stack.contains(&import.uri) {
            return Err(format!("cyclic qmldir import involving '{}'", import.uri));
        }
        if !seen.insert((import.uri.clone(), import.version.clone())) {
            return Ok(());
        }
        let request = import
            .version
            .as_deref()
            .map(str::parse)
            .transpose()?
            .unwrap_or_default();
        let Some(module) = self.locate(&import.uri, request)? else {
            result.push(Dependency::Native(import.clone()));
            return Ok(());
        };
        stack.insert(import.uri.clone());
        for dependency in module
            .manifest
            .imports
            .iter()
            .filter(|item| !item.optional && !item.default)
        {
            let version = match dependency.version {
                ImportVersion::Inherit => import.version.clone(),
                ImportVersion::Request(RequestedVersion::Latest) => None,
                ImportVersion::Request(RequestedVersion::Major(v)) => Some(v.to_string()),
                ImportVersion::Request(RequestedVersion::Exact(v)) => Some(v.to_string()),
            };
            let item = Import {
                uri: dependency.uri.clone(),
                version,
                ..import.clone()
            };
            self.visit(&item, result, seen, stack)?;
        }
        stack.remove(&import.uri);
        result.push(Dependency::Module(module));
        Ok(())
    }

    pub(crate) fn native_imports(&mut self, import: &Import) -> Result<Vec<Import>> {
        Ok(self
            .closure(import)?
            .into_iter()
            .filter_map(|dependency| match dependency {
                Dependency::Native(import) => Some(import),
                Dependency::Module(_) => None,
            })
            .collect())
    }

    pub(crate) fn component(&mut self, import: &Import, ty: &str) -> Result<Option<PathBuf>> {
        for dependency in self.closure(import)?.into_iter().rev() {
            if let Dependency::Module(module) = dependency {
                if let Some(path) = self.exported(&module, ty, false)? {
                    return Ok(Some(path));
                }
            }
        }
        Ok(None)
    }

    fn exported(&mut self, module: &Module, ty: &str, private: bool) -> Result<Option<PathBuf>> {
        let mut file = if private {
            module.manifest.internal.get(ty).map(String::as_str)
        } else {
            None
        };
        if file.is_none() {
            let Some(export) = module.manifest.select(module.selected())?.get(ty) else {
                return Ok(None);
            };
            match export.kind {
                ExportKind::Object => file = Some(&export.file),
                ExportKind::Singleton => {
                    return Err(format!(
                        "cannot construct singleton '{}.{ty}'",
                        module.manifest.uri
                    ))
                }
                ExportKind::Script => {
                    return Err(format!(
                        "script '{}.{ty}' is not a constructible object",
                        module.manifest.uri
                    ))
                }
            }
        }
        let dir = match &module.manifest.prefer {
            None => module.dir.clone(),
            Some(path) if !path.contains(':') => module.dir.join(path),
            Some(path) => {
                return Err(format!(
                    "resource lookup for qmldir prefer path '{path}' is unsupported"
                ))
            }
        };
        let path = dir.join(file.expect("internal or public source"));
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if let Some(origin) = self.origins.get(&canonical) {
            if origin.dir != module.dir || origin.version != module.version {
                return Err(format!(
                    "{} was selected with conflicting module versions or origins",
                    canonical.display()
                ));
            }
        } else {
            self.origins.insert(canonical.clone(), module.clone());
        }
        Ok(Some(canonical))
    }

    /// Public, explicit directory import.
    pub(crate) fn directory_component(&mut self, dir: &Path, ty: &str) -> Result<Option<PathBuf>> {
        self.directory(dir, ty, false)
    }

    /// The component's own directory may additionally resolve private types.
    pub(crate) fn local_component(&mut self, from: &Path, ty: &str) -> Result<Option<PathBuf>> {
        if let Some(module) = self.origins.get(from).cloned() {
            return self.exported(&module, ty, true);
        }
        let dir = from
            .parent()
            .ok_or_else(|| format!("component path '{}' has no parent", from.display()))?;
        self.directory(dir, ty, true)
    }

    fn directory(&mut self, dir: &Path, ty: &str, private: bool) -> Result<Option<PathBuf>> {
        if !dir.is_dir() {
            return Err(format!(
                "QML import directory '{}' does not exist",
                dir.display()
            ));
        }
        let canonical = dir
            .canonicalize()
            .map_err(|error| format!("{}: {error}", dir.display()))?;
        if let Some(manifest) = self.manifest(&canonical)? {
            let module = self.select(&canonical, manifest, RequestedVersion::Latest)?;
            return self.exported(&module, ty, private);
        }
        if !ty.chars().next().is_some_and(char::is_uppercase)
            || !ty.chars().all(|ch| ch == '_' || ch.is_alphanumeric())
        {
            return Ok(None);
        }
        let path = canonical.join(format!("{ty}.qml"));
        Ok(path.is_file().then_some(path))
    }
}
