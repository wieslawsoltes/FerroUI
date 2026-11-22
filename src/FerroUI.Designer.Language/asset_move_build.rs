//! Map the executed Cargo asset declarations to authored Rust text.

use super::rust_files;
use crate::{require, LanguageError, LanguageService, Result};
use ferroui_build_scan::packaged_assets::AssetDeclaration;
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_project::{
    rust_source_offset, ProjectFileMove, PublishedAssetMove, RustAssetReferences,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct CargoAssetEdit<'a> {
    declaration: &'a AssetDeclaration,
    replacement: Option<&'a str>,
}

pub(super) fn build_inputs<'a>(
    service: &LanguageService,
    movement: &ProjectFileMove,
    builds: &'a BTreeSet<PublishedAssetMove>,
) -> Result<BTreeMap<String, BTreeSet<CargoAssetEdit<'a>>>> {
    if builds.is_empty() {
        return Ok(BTreeMap::new());
    }
    let package = movement.package_root.as_path();
    let files = rust_files(service);
    let mut sources = BTreeMap::new();
    for build in builds {
        let directories = build.directories.iter().map(|declaration| CargoAssetEdit {
            declaration,
            replacement: None,
        });
        let registrations = build
            .registrations
            .iter()
            .map(|declaration| CargoAssetEdit {
                declaration,
                replacement: Some(&build.to_key),
            });
        for input in directories.chain(registrations) {
            let candidates: BTreeSet<_> = package
                .ancestors()
                .filter_map(|base| base.join(&input.declaration.rust_file).canonicalize().ok())
                .filter(|path| path.starts_with(package))
                .collect();
            let mut matches = files.values().filter(|(entry, project, _)| {
                candidates.contains(&entry.path)
                    && project.authored_package_root(&entry.path) == package
            });
            let uri = match (matches.next(), matches.next()) {
                (Some((entry, _, _)), None) => entry.uri.clone(),
                _ => return Err(unmapped(&input, package)),
            };
            sources
                .entry(uri)
                .or_insert_with(BTreeSet::new)
                .insert(input);
        }
    }
    Ok(sources)
}

pub(super) fn build_patches(
    inputs: Option<&BTreeSet<CargoAssetEdit<'_>>>,
    syntax: &RustAssetReferences,
    source: &str,
    package: &Path,
) -> Result<Vec<TextPatch>> {
    let mut patches = Vec::new();
    for input in inputs.into_iter().flatten() {
        let declaration = input.declaration;
        let offset = rust_source_offset(source, declaration.line, declaration.column)
            .ok_or_else(|| unmapped(input, package))?;
        let name = if input.replacement.is_some() {
            "asset"
        } else {
            "embed_assets"
        };
        let mut candidates = syntax
            .build_calls
            .iter()
            .filter(|call| call.name == name && call.callee.contains(&offset))
            .flat_map(|call| &call.operands)
            .filter(|path| path.path == declaration.value);
        let site = candidates.next().ok_or_else(|| unmapped(input, package))?;
        require(
            candidates.next().is_none(),
            "The Cargo asset declaration is ambiguous",
        )?;
        if let Some(replacement) = input.replacement {
            patches.push(TextPatch {
                span: SourceSpan::new(site.range.start, site.range.end),
                replacement: site.replacement(source, replacement),
            });
        }
    }
    Ok(patches)
}

fn unmapped(input: &CargoAssetEdit<'_>, package: &Path) -> LanguageError {
    let site = input.declaration;
    let method = if input.replacement.is_some() {
        "ExportPlan::asset"
    } else {
        "Build::embed_assets"
    };
    LanguageError(format!("Cannot review the {method} declaration for {} at {}:{} in {}. Use the authored path in the builder and rebuild the project XAML schema.",
        site.value, site.rust_file.display(), site.line, package.display()))
}
