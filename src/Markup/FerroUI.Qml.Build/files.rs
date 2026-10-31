use crate::identified::IdentifiedModules;
use ferroui_qml_compiler::{parse, ComponentResolver, ComponentSource, Document, Import};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(crate) struct Files {
    modules: IdentifiedModules,
    documents: BTreeMap<PathBuf, Rc<Document>>,
    directories: BTreeSet<PathBuf>,
}

impl Files {
    pub(crate) fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            modules: IdentifiedModules::new(roots),
            documents: BTreeMap::new(),
            directories: BTreeSet::new(),
        }
    }

    pub(crate) fn load(&mut self, path: &Path) -> Result<ComponentSource, String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !self.documents.contains_key(&path) {
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let document = parse(&text).map_err(|e| format!("{}:{e}", path.display()))?;
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
        self.documents
            .keys()
            .chain(&self.directories)
            .cloned()
            .chain(self.modules.watched().map(Path::to_owned))
            .collect()
    }
}

impl ComponentResolver for Files {
    fn native_imports(&mut self, from: &str, imports: &[Import]) -> Result<Vec<Import>, String> {
        let base = Path::new(from)
            .parent()
            .ok_or("component has no directory")?;
        let mut native = Vec::new();
        for import in imports {
            if import.is_path {
                let dir = base.join(&import.uri);
                if !dir.is_dir() {
                    return Err(format!(
                        "expected a local QML import directory: {}",
                        dir.display()
                    ));
                }
                self.directories.insert(dir);
            } else {
                native.extend(self.modules.native_imports(import)?);
            }
        }
        Ok(native)
    }

    fn resolve(
        &mut self,
        from: &str,
        imports: &[Import],
        ty: &str,
    ) -> Result<Option<ComponentSource>, String> {
        let base = Path::new(from)
            .parent()
            .ok_or("component has no directory")?;
        let (namespace, ty) = ty
            .split_once('.')
            .map_or((None, ty), |(ns, ty)| (Some(ns), ty));
        let visible = || imports.iter().filter(|i| i.alias.as_deref() == namespace);
        // Identified module types have precedence over explicit and implicit
        // directory imports, regardless of the order of the import statements.
        for import in visible().rev().filter(|i| !i.is_path) {
            if let Some(file) = self.modules.component(import, ty)? {
                return self.load(&file).map(Some);
            }
        }
        let mut candidates = BTreeSet::new();
        if namespace.is_none() {
            self.directories.insert(base.to_owned());
            if let Some(file) = self.modules.local_component(Path::new(from), ty)? {
                candidates.insert(
                    file.canonicalize()
                        .map_err(|e| format!("{}: {e}", file.display()))?,
                );
            }
        }
        for import in visible().filter(|i| i.is_path) {
            if let Some(file) = self
                .modules
                .directory_component(&base.join(&import.uri), ty)?
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
