use crate::{LanguageError, Result};
use ferroui_build_scan::model::TypeModel;
use ferroui_build_scan::scanner::{ClassRegistration, ScanOptions, class_registration, scan_crate};
use ferroui_designer_project::{CargoTarget, file_uri};
use std::path::Path;

pub(crate) struct HandlerSource {
    pub uri: String,
    module: Vec<String>,
    class: String,
    namespace: Option<String>,
}

impl HandlerSource {
    pub fn locate(
        root: &Path,
        class: &TypeModel,
        crate_name: &str,
        targets: &[CargoTarget],
    ) -> Result<Self> {
        let mut result = None;
        for target in targets.iter().filter(|target| {
            target.kind.iter().any(|kind| {
                matches!(
                    kind.as_str(),
                    "bin" | "lib" | "rlib" | "dylib" | "cdylib" | "staticlib"
                )
            })
        }) {
            let Ok(root_file) = target.source.canonicalize() else {
                continue;
            };
            if !root_file.starts_with(root) {
                continue;
            }
            let scan = scan_crate(&ScanOptions::new(crate_name, root_file));
            let Some(scanned) = scan.model.find_rust_type(&class.rust_path.text) else {
                continue;
            };
            if scanned.module != class.module
                || scanned.explicit_namespace != class.explicit_namespace
            {
                continue;
            }
            // Inline modules are owned by their closest file-level module. Pick
            // the exact authored location even with #[path] and nested modules.
            let Some(file) = scan
                .files
                .iter()
                .filter(|file| {
                    class.module == file.module
                        || class
                            .module
                            .strip_prefix(&file.module)
                            .is_some_and(|suffix| suffix.starts_with("::"))
                })
                .max_by_key(|file| file.module.len())
            else {
                continue;
            };
            let Ok(path) = file.path.canonicalize() else {
                continue;
            };
            if !path.starts_with(root) || path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let candidate = Self {
                uri: file_uri(&path)?,
                module: class.module[file.module.len()..]
                    .split("::")
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect(),
                class: class.name.clone(),
                namespace: class.explicit_namespace.clone(),
            };
            if let Some(previous) = &result {
                let previous: &Self = previous;
                if previous.uri != candidate.uri || previous.module != candidate.module {
                    return Err(LanguageError(
                        "Several Cargo targets declare this code-behind class".into(),
                    ));
                }
            }
            result = Some(candidate);
        }
        result.ok_or_else(|| {
            LanguageError(
                "Build this Cargo package to refresh the authored code-behind location".into(),
            )
        })
    }

    pub fn registration(&self, source: &str) -> Result<ClassRegistration> {
        let module = self.module.iter().map(String::as_str).collect::<Vec<_>>();
        let registration = class_registration(source, &module, &self.class)
            .map_err(LanguageError)?
            .ok_or_else(|| {
                LanguageError(
                    "The owner needs a directly authored ferro_class_info! registration".into(),
                )
            })?;
        if registration.namespace != self.namespace {
            return Err(LanguageError(
                "The registered namespace changed. Refresh the Cargo schema".into(),
            ));
        }
        Ok(registration)
    }
}
