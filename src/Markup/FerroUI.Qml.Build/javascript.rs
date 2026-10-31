//! Embed local JavaScript resources in the compiled component. Stable virtual
//! URLs preserve module identity through cycles without encoding host paths.
use ferroui_qml_compiler::{
    javascript::{inline_dependencies, parse_resource, visit_scripts, Dependency},
    Document, ScriptResource,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub(super) struct Resources {
    paths: BTreeMap<PathBuf, String>,
    embedded: BTreeMap<String, ScriptResource>,
    preload: std::collections::BTreeSet<String>,
}

impl Resources {
    pub(super) fn watched(&self) -> impl Iterator<Item = &PathBuf> {
        self.paths.keys()
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
            let key = self.load(&file, 0)?;
            self.preload.insert(key.clone());
            imports.insert(alias.clone(), key);
        }
        let origin = format!("https://qml.invalid/components/{index}/component.qml");
        visit_scripts(&mut document.root, &mut |script| {
            script.origin.clone_from(&origin);
            script.imports.clone_from(&imports);
            let dynamic =
                inline_dependencies(script).map_err(|e| format!("{}:{e}", path.display()))?;
            let mut edits = Vec::new();
            for dep in dynamic {
                let target = self.load(&local_path(path, &dep.specifier)?, 0)?;
                push_edit(&mut edits, dep, &target);
            }
            replace(&mut script.source, edits);
            Ok::<_, String>(())
        })?;
        Ok(document)
    }

    pub(super) fn into_embedded(mut self) -> BTreeMap<String, ScriptResource> {
        for key in self.preload {
            if let Some(resource) = self.embedded.get_mut(&key) {
                resource.preload = true;
            }
        }
        self.embedded
    }

    fn load(&mut self, path: &Path, depth: usize) -> Result<String, String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(existing) = self.paths.get(&path) {
            return Ok(existing.clone());
        }
        if depth >= 128 {
            return Err("JavaScript import nesting limit exceeded".into());
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !matches!(ext, "js" | "mjs") {
            return Err(format!(
                "unsupported JavaScript resource extension: {}",
                path.display()
            ));
        }
        let source = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let parsed =
            parse_resource(&source, ext == "mjs").map_err(|e| format!("{}:{e}", path.display()))?;
        let key = format!(
            "https://qml.invalid/modules/{}/module.{ext}",
            self.paths.len()
        );
        self.paths.insert(path.clone(), key.clone());
        let mut resource = parsed.resource;
        let mut edits = Vec::new();
        for dep in parsed.dependencies {
            let dependency = self.load(&local_path(&path, &dep.specifier)?, depth + 1)?;
            if let Some(alias) = &dep.namespace {
                resource.imports.insert(alias.clone(), dependency.clone());
            }
            resource.dependencies.insert(
                dep.namespace
                    .clone()
                    .unwrap_or_else(|| dep.specifier.clone()),
                dependency.clone(),
            );
            push_edit(&mut edits, dep, &dependency);
        }
        replace(&mut resource.source, edits);
        self.embedded.insert(key.clone(), resource);
        Ok(key)
    }
}

fn local_path(origin: &Path, specifier: &str) -> Result<PathBuf, String> {
    if specifier.is_empty() || specifier.contains([':', '?', '#']) || specifier.starts_with("//") {
        return Err(format!(
            "expected a local JavaScript resource path: {specifier}"
        ));
    }
    Ok(origin
        .parent()
        .ok_or("JavaScript resource has no containing directory")?
        .join(specifier))
}

fn push_edit(
    edits: &mut Vec<(ferroui_qml_compiler::SourceSpan, String)>,
    dep: Dependency,
    target: &str,
) {
    // Generated URLs contain only fixed URL characters, digits and the checked extension.
    if let Some(span) = dep.span {
        edits.push((span, format!("\"{target}\"")));
    }
}
fn replace(source: &mut String, mut edits: Vec<(ferroui_qml_compiler::SourceSpan, String)>) {
    edits.sort_by_key(|(s, _)| std::cmp::Reverse(s.start));
    for (range, value) in edits {
        source.replace_range(range.start..range.end, &value);
    }
}
