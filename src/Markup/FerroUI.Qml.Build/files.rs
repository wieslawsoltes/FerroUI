use crate::identified::{IdentifiedModules, Lookup};
use ferroui_qml_compiler::{parse, ComponentResolver, ComponentSource, Document, Import};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(crate) struct Files {
    project: PathBuf,
    package: Option<PathBuf>,
    component_entries: BTreeMap<String, PathBuf>,
    #[cfg(feature = "javascript")]
    resources: Option<crate::javascript::Resources>,
    modules: IdentifiedModules,
    documents: BTreeMap<PathBuf, Rc<Document>>,
    directories: BTreeSet<PathBuf>,
}

impl Files {
    pub(crate) fn new(
        roots: Vec<PathBuf>,
        plugins: BTreeSet<ferroui_qml_compiler::qmldir::StaticPlugin>,
    ) -> Self {
        Self {
            project: PathBuf::new(),
            package: None,
            component_entries: BTreeMap::new(),
            #[cfg(feature = "javascript")]
            resources: None,
            modules: IdentifiedModules::new(roots, plugins),
            documents: BTreeMap::new(),
            directories: BTreeSet::new(),
        }
    }

    pub(crate) fn set_component_entries(
        &mut self,
        input: &Path,
        entries: &BTreeSet<PathBuf>,
    ) -> Result<(), String> {
        let input = input
            .canonicalize()
            .map_err(|e| format!("{}: {e}", input.display()))?;
        let root = input.parent().ok_or("component needs a parent directory")?;
        self.project = root.to_owned();
        self.package = root
            .ancestors()
            .find(|p| p.join("Cargo.toml").is_file())
            .map(Path::to_owned);
        for entry in entries {
            if entry.is_absolute() || entry.has_root() {
                return Err(format!(
                    "QML entry must be project-relative: {}",
                    entry.display()
                ));
            }
            let target = root
                .join(entry)
                .canonicalize()
                .map_err(|e| format!("{}: {e}", entry.display()))?;
            let relative = target
                .strip_prefix(root)
                .map_err(|_| format!("QML entry escapes project: {}", entry.display()))?;
            if target.extension().and_then(|s| s.to_str()) != Some("qml") || !target.is_file() {
                return Err(format!("expected a local QML file: {}", entry.display()));
            }
            self.component_entries.insert(project_url(relative), target);
        }
        Ok(())
    }

    #[cfg(feature = "javascript")]
    pub(crate) fn with_javascript(
        &mut self,
        input: &Path,
        entries: &BTreeSet<PathBuf>,
        workers: &BTreeSet<PathBuf>,
    ) -> Result<(), String> {
        self.resources = Some(crate::javascript::Resources::new(input, entries, workers)?);
        Ok(())
    }

    #[cfg(feature = "javascript")]
    pub(crate) fn finish(&mut self, document: &mut Document) {
        if let Some(graph) = self.resources.take() {
            document.scripts = graph.into_embedded();
        }
    }

    pub(crate) fn load(&mut self, path: &Path) -> Result<ComponentSource, String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !self.documents.contains_key(&path) {
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            #[cfg(feature = "javascript")]
            let document = if let Some(resources) = &mut self.resources {
                let document = ferroui_qml_compiler::parse_external(&text)
                    .map_err(|e| format!("{}:{e}", path.display()))?;
                resources.attach(&path, self.documents.len(), document)?
            } else {
                parse(&text).map_err(|e| format!("{}:{e}", path.display()))?
            };
            #[cfg(not(feature = "javascript"))]
            let document = parse(&text).map_err(|e| format!("{}:{e}", path.display()))?;
            let mut document = document;
            if let Ok(path) = path.strip_prefix(&self.project) {
                document.root.set_source_url(&project_url(path));
            }
            {
                let root = &self.project;
                for relative in crate::loader_sources::sources(&document) {
                    // URLs outside the embedded application catalog retain runtime Error status.
                    if relative.is_empty()
                        || relative.contains([':', '?', '#', '\\'])
                        || relative.starts_with('/')
                    {
                        continue;
                    }
                    let file = path.parent().expect("document directory").join(relative);
                    if let Ok(target) = file.canonicalize() {
                        if target.is_file()
                            && target.extension().and_then(|e| e.to_str()) == Some("qml")
                        {
                            if let Ok(within) = target.strip_prefix(root) {
                                self.component_entries.insert(project_url(within), target);
                            }
                        }
                    } else if let Some(directory) = file
                        .parent()
                        .filter(|dir| dir.is_dir() && dir.starts_with(root))
                    {
                        self.directories.insert(directory.to_owned());
                    }
                }
            }
            self.documents.insert(path.clone(), Rc::new(document));
        }
        Ok(ComponentSource {
            key: path
                .to_str()
                .ok_or("component path is not valid UTF-8")?
                .to_owned(),
            document: Rc::clone(&self.documents[&path]),
        })
    }

    pub(crate) fn dependencies(&self) -> BTreeSet<PathBuf> {
        let paths = self
            .documents
            .keys()
            .chain(&self.directories)
            .cloned()
            .chain(self.modules.watched().map(Path::to_owned));
        #[cfg(feature = "javascript")]
        let paths = paths.chain(self.resources.iter().flat_map(|r| r.watched().cloned()));
        paths.collect()
    }
}

impl ComponentResolver for Files {
    fn type_key(&self, origin: &str) -> String {
        let (file, inline) = if self.documents.contains_key(Path::new(origin)) {
            (origin, "")
        } else {
            origin.rsplit_once('#').unwrap_or((origin, ""))
        };
        let path = Path::new(file);
        let key = self.modules.type_key(path).unwrap_or_else(|| {
            // Different entry directories in one Cargo package can each have a
            // Foo.qml. The package-relative name survives moving the checkout.
            if let Some(package) = &self.package {
                if let (Some(name), Ok(relative)) =
                    (package.file_name(), path.strip_prefix(package))
                {
                    return format!(
                        "cargo:{}/{}",
                        name.to_string_lossy(),
                        relative.to_string_lossy().replace('\\', "/")
                    );
                }
            }
            // Standalone compilers share the entry directory as their project base.
            let mut prefix = PathBuf::new();
            for ancestor in self.project.ancestors() {
                if let Ok(relative) = path.strip_prefix(ancestor) {
                    return project_url(&prefix.join(relative));
                }
                prefix.push("..");
            }
            file.to_owned()
        });
        if inline.is_empty() {
            key
        } else {
            format!("{key}#{inline}")
        }
    }

    fn next_component(&mut self) -> Result<Option<(String, ComponentSource)>, String> {
        self.component_entries
            .pop_first()
            .map(|(url, path)| self.load(&path).map(|source| (url, source)))
            .transpose()
    }

    fn native_imports(&mut self, from: &str, imports: &[Import]) -> Result<Vec<Import>, String> {
        let base = Path::new(from)
            .parent()
            .ok_or("component has no directory")?;
        let mut native: Vec<_> = self
            .modules
            .implicit_native(Path::new(from))
            .into_iter()
            .collect();
        for import in imports {
            if import.is_script() {
                #[cfg(feature = "javascript")]
                if self.resources.is_some() {
                    continue;
                }
                return Err(format!(
                    "JavaScript import '{}' requires the JavaScript compiler mode",
                    import.uri
                ));
            }
            if import.is_path {
                let dir = base.join(&import.uri);
                if !dir.is_dir() {
                    return Err(format!(
                        "expected a local QML import directory: {}",
                        dir.display()
                    ));
                }
                if let Some(native_import) = self.modules.directory_native(&dir, import)? {
                    native.push(native_import);
                }
                self.directories.insert(
                    dir.canonicalize()
                        .map_err(|err| format!("{}: {err}", dir.display()))?,
                );
            } else {
                native.extend(self.modules.native_imports(import)?);
            }
        }
        Ok(native)
    }

    fn singleton(
        &mut self,
        from: &str,
        imports: &[Import],
        ty: &str,
    ) -> Result<Option<ComponentSource>, String> {
        self.lookup(from, imports, ty, Lookup::Singleton)
    }

    fn resolve(
        &mut self,
        from: &str,
        imports: &[Import],
        ty: &str,
    ) -> Result<Option<ComponentSource>, String> {
        self.lookup(from, imports, ty, Lookup::Component)
    }
}

impl Files {
    fn lookup(
        &mut self,
        from: &str,
        imports: &[Import],
        ty: &str,
        kind: Lookup,
    ) -> Result<Option<ComponentSource>, String> {
        let base = Path::new(from)
            .parent()
            .ok_or("component has no directory")?;
        let (namespace, ty) = ty
            .split_once('.')
            .map_or((None, ty), |(ns, ty)| (Some(ns), ty));
        let visible = || {
            imports
                .iter()
                .filter(|i| !i.is_script() && i.alias.as_deref() == namespace)
        };
        // Identified module types have precedence over explicit and implicit
        // directory imports, regardless of the order of the import statements.
        for import in visible().rev().filter(|i| !i.is_path) {
            if let Some(file) = self.modules.component(import, ty, kind)? {
                return self.load(&file).map(Some);
            }
        }
        let mut candidates = BTreeSet::new();
        if namespace.is_none() {
            self.directories.insert(base.to_owned());
            if let Some(file) = self.modules.local_component(Path::new(from), ty, kind)? {
                candidates.insert(
                    file.canonicalize()
                        .map_err(|e| format!("{}: {e}", file.display()))?,
                );
            }
        }
        for import in visible().filter(|i| i.is_path) {
            if let Some(file) =
                self.modules
                    .directory_component(&base.join(&import.uri), ty, kind)?
            {
                candidates.insert(
                    file.canonicalize()
                        .map_err(|e| format!("{}: {e}", file.display()))?,
                );
            }
        }
        if candidates.len() > 1 {
            return Err(format!("ambiguous component '{ty}' in {candidates:?}"));
        }
        candidates.first().map(|path| self.load(path)).transpose()
    }
}

fn project_url(relative: &Path) -> String {
    use percent_encoding::{percent_encode, AsciiSet, NON_ALPHANUMERIC};
    const ENCODE: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'.')
        .remove(b'_')
        .remove(b'~');
    let mut url = String::from("https://qml.invalid/components/project");
    for part in relative {
        url.push('/');
        url.push_str(&percent_encode(part.as_encoded_bytes(), ENCODE).to_string());
    }
    url
}
