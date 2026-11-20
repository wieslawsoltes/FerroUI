//! Keep explicit Cargo registrations and their packaged URI consumers in one move plan.
use super::{compiled_key, input_base, inputs, refers_to, xaml_roots};
use crate::{require, FileMoveDraft, LanguageError, LanguageService, Result};
use ferroui_build_scan::{
    compiled_index::CompiledXamlInput, compiled_names::CompiledGroupKind, model::AssemblyModel,
};
use ferroui_designer_project::{
    canonical_file_uri, compiled_resource_key as address_key, file_path, file_uri,
    relative_asset_path, RustXamlReferences, RustXamlRegistration, XamlFileMove,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[path = "file_move_symbols.rs"]
mod symbols;
pub use symbols::GeneratedRustRename;

#[derive(Default)]
pub(crate) struct CompiledAddresses {
    /// A registered absolute address is authoritative even if it keeps its spelling.
    routes: BTreeMap<String, Option<String>>,
    workspace: Option<PathBuf>,
    build: Option<BuildSnapshot>,
    pub(crate) generated_rust: Vec<GeneratedRustRename>,
    generated_directory: Option<PathBuf>,
    pub(super) file_inputs: BTreeMap<String, Vec<CompiledXamlInput>>,
    pub(super) has_compiler_index: bool,
    file_class_keys: BTreeSet<String>,
    discovered_class_keys: BTreeSet<String>,
}

pub(super) struct BuildSnapshot {
    uri: String,
    pub(super) source: String,
    pub(super) references: RustXamlReferences,
}

impl CompiledAddresses {
    pub(crate) fn prepare(
        service: &LanguageService,
        movement: &XamlFileMove,
        drafts: &BTreeMap<String, &FileMoveDraft>,
    ) -> Result<Self> {
        let package = &movement.package_root;
        if !package.join("Cargo.toml").is_file() {
            return Ok(Self::default());
        }
        let Some((name, targets)) = service.cargo_package_targets(package)? else {
            return Ok(Self::default());
        };
        let Some(build) = targets
            .iter()
            .find(|target| target.kind.iter().any(|kind| kind == "custom-build"))
        else {
            return Ok(Self::default());
        };
        let script = canonical_file_uri(&file_uri(&build.source)?)?;
        let Some(root) = service.workspace_root(&script) else {
            return Ok(Self::default());
        };
        let script_path = file_path(&script)?;
        let source = if let Some(draft) = drafts.get(&script) {
            draft.source.clone()
        } else if script_path.is_file() {
            service.workspaces[root].read_source(&script)?
        } else {
            return Ok(Self::default());
        };
        let references = RustXamlReferences::parse(&source, true);
        let destination = file_path(&movement.to)?;
        let mut addresses = Self {
            workspace: service.workspace_root(&movement.from).cloned(),
            ..Self::default()
        };
        let crate_name = name.replace('-', "_");
        let schema = service.project_schema_for(&movement.from);
        let catalog = schema
            .into_iter()
            .flat_map(|(_, project)| project.catalog.models().models())
            .filter(|model| model.crate_name == crate_name)
            .collect::<Vec<_>>();
        let assembly = catalog
            .first()
            .map_or(crate_name.as_str(), |model| model.name.as_str());
        require(
            catalog.iter().all(|model| model.name == assembly),
            "Rebuild the package XAML schema to resolve conflicting assembly names",
        )?;
        let roots = references
            .as_ref()
            .ok()
            .map(|references| xaml_roots(package, references))
            .unwrap_or_default();
        for registration in references
            .iter()
            .flat_map(|references| &references.registrations)
            .filter(|item| item.exported)
        {
            let Some(base) = input_base(package, &script_path, registration.anchor) else {
                continue;
            };
            if !base.join(&registration.input).is_file() {
                continue;
            }
            let previous = absolute(assembly, &registration.key);
            let Some(key) = address_key(&previous) else {
                continue;
            };
            if known_private(&catalog, registration, &key) {
                continue;
            }
            let next = if refers_to(base, &registration.input, &movement.from) {
                compiled_key(movement, &destination, &roots, &script, &registration.key)?
                    .map(|key| absolute(assembly, &key))
            } else {
                None
            };
            addresses.insert_route(key, &previous, next)?;
        }
        if let Some((_, project)) = schema {
            let manifest = package
                .join("Cargo.toml")
                .canonicalize()
                .map_err(ferroui_designer_project::ProjectError::from)?;
            if let Some(output) = project.catalog.compiled_xaml_output(&manifest)? {
                for group in &output.index.groups {
                    if group.kind != CompiledGroupKind::Class {
                        continue;
                    }
                    let Some(key) = group
                        .members
                        .first()
                        .and_then(|member| class_key(&member.key))
                    else {
                        continue;
                    };
                    if group.file_backed {
                        addresses.file_class_keys.insert(key);
                    } else if group.discovery_root.is_some() {
                        addresses.discovered_class_keys.insert(key);
                    }
                }
                let file_modules: BTreeSet<_> = output
                    .index
                    .groups
                    .iter()
                    .filter(|group| group.file_backed)
                    .map(|group| group.module.as_str())
                    .collect();
                addresses.has_compiler_index = !file_modules.is_empty();
                addresses.generated_directory = Some(output.module_directory());
                if let Ok(references) = &references {
                    addresses.generated_rust = symbols::plan(
                        &output,
                        movement,
                        &roots,
                        &script,
                        &script_path,
                        references,
                    )?;
                }
                for document in catalog.iter().flat_map(|model| &model.documents) {
                    if document.public
                        && document
                            .build_path
                            .iter()
                            .chain(&document.populate_path)
                            .filter_map(|path| path.rsplit("::").nth(1))
                            .any(|module| file_modules.contains(module))
                    {
                        if let Some(key) = address_key(&document.uri) {
                            addresses.insert_route(key, &document.uri, None)?;
                        }
                    }
                }
                addresses.file_inputs = inputs::by_source(
                    service,
                    movement,
                    output.index.file_inputs,
                    &project.resources,
                )?;
            }
        }
        if let Ok(references) = references {
            addresses.build = Some(BuildSnapshot {
                uri: script,
                source,
                references,
            });
        }
        addresses.check_destinations()?;
        Ok(addresses)
    }

    pub(crate) fn moves_file_input(&self) -> bool {
        !self.file_inputs.is_empty()
    }

    pub(super) fn preserves_class_key(&self, key: &str) -> Result<bool> {
        let Some(key) = class_key(key).filter(|key| self.file_class_keys.contains(key)) else {
            return Ok(false);
        };
        require(
            !self.discovered_class_keys.contains(&key),
            format!("The class selector {key} is shared by file-backed and discovered groups. Select the file-backed class with the registration's key() method."),
        )?;
        Ok(true)
    }

    fn insert_route(&mut self, key: String, uri: &str, next: Option<String>) -> Result<()> {
        if let Some(old) = self.routes.insert(key, next.clone()) {
            require(
                old == next,
                format!("The registered XAML address {uri} is ambiguous"),
            )?;
        }
        Ok(())
    }

    pub(super) fn build_for(&self, uri: &str) -> Option<&BuildSnapshot> {
        self.build.as_ref().filter(|snapshot| snapshot.uri == uri)
    }

    pub(super) fn is_generated(&self, file: &std::path::Path) -> bool {
        self.generated_directory
            .as_ref()
            .is_some_and(|directory| file.starts_with(directory))
    }

    fn check_destinations(&self) -> Result<()> {
        let mut planned = BTreeSet::new();
        for next in self.routes.values().flatten() {
            let key = address_key(next)
                .ok_or_else(|| LanguageError("Invalid compiled XAML address".into()))?;
            require(
                !self.routes.get(&key).is_some_and(Option::is_none) && planned.insert(key),
                format!("The registered XAML address {next} already exists"),
            )?;
        }
        Ok(())
    }

    pub(crate) fn replacement<'a>(
        &'a self,
        service: &LanguageService,
        owner: &str,
        uri: &'a str,
    ) -> Option<&'a str> {
        if self.routes.is_empty() {
            return None;
        }
        let next = self.routes.get(&address_key(uri)?)?;
        let assembly = uri.strip_prefix("ferres://")?.split_once('/')?.0;
        if self.workspace.as_ref() != service.workspace_root(owner)
            && owns_assembly(service, owner, assembly)
        {
            return Some(uri);
        }
        Some(next.as_deref().unwrap_or(uri))
    }
}

fn class_key(key: &str) -> Option<String> {
    relative_asset_path(key.trim_start_matches('/'))
}

fn owns_assembly(service: &LanguageService, owner: &str, assembly: &str) -> bool {
    let Some(root) = service.workspace_root(owner) else {
        return false;
    };
    service.workspaces[root]
        .solution
        .projects
        .iter()
        .any(|project| {
            let crate_name = project.name.replace('-', "_");
            if let Some(schema) = service.package_schemas.get(&project.root) {
                let mut owned = schema
                    .catalog
                    .models()
                    .models()
                    .iter()
                    .filter(|model| model.crate_name == crate_name)
                    .peekable();
                if owned.peek().is_some() {
                    return owned.any(|model| model.name.eq_ignore_ascii_case(assembly));
                }
            }
            project.name.eq_ignore_ascii_case(assembly)
                || crate_name.eq_ignore_ascii_case(assembly)
                || project
                    .targets
                    .iter()
                    .any(|target| target.name.eq_ignore_ascii_case(assembly))
        })
}

fn known_private(
    models: &[&AssemblyModel],
    registration: &RustXamlRegistration,
    key: &str,
) -> bool {
    models.iter().any(|model| {
        let module = format!("::{}::{}::", model.crate_name, registration.module);
        model.documents.iter().any(|doc| {
            !doc.public
                && address_key(&doc.uri).as_deref() == Some(key)
                && doc
                    .build_path
                    .iter()
                    .chain(doc.populate_path.iter())
                    .any(|path| path.starts_with(&module))
        })
    })
}

fn absolute(assembly: &str, key: &str) -> String {
    format!("ferres://{assembly}/{}", key.trim_start_matches('/'))
}
