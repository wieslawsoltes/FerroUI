//! Calculate the next Rust entry points from an actual XAML compiler output.
use super::{compiled_key, input_base, refers_to};
use crate::{require, LanguageError, Position, Result};
use ferroui_build_scan::{
    compiled_index::CompiledXamlGroup,
    compiled_names::{same_document_uri, CompiledGroupKind},
};
use ferroui_designer_project::{
    file_path, file_uri, relative_asset_path, relative_asset_uri, relative_file_path,
    CompiledXamlOutput, RustXamlReferences, XamlFileMove,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

pub use crate::RustSymbolRename as GeneratedRustRename;

pub(super) fn plan(
    output: &CompiledXamlOutput,
    movement: &XamlFileMove,
    roots: &[PathBuf],
    script: &str,
    script_path: &Path,
    references: &RustXamlReferences,
) -> Result<Vec<GeneratedRustRename>> {
    let mut renames = Vec::new();
    let destination = file_path(&movement.to)?;
    for group in output
        .index
        .groups
        .iter()
        .filter(|group| !group.file_backed)
    {
        let mut keys = BTreeMap::new();
        if let Some(root) = &group.discovery_root {
            if let Some((from, to)) = discovered_keys(group, movement, &destination, root)? {
                keys.insert(from, to);
            }
        } else {
            let mut mapped = BTreeSet::new();
            for entry in references
                .registrations
                .iter()
                .filter(|entry| entry.module == group.module)
            {
                let Some(base) = input_base(&movement.package_root, script_path, entry.anchor)
                else {
                    continue;
                };
                let key = group_key(group.kind, &entry.key)?;
                if !base.join(&entry.input).is_file() {
                    continue;
                }
                mapped.insert(key.clone());
                if !refers_to(base, &entry.input, &movement.from) {
                    continue;
                }
                if let Some(next) = compiled_key(
                    movement,
                    &destination,
                    roots,
                    script,
                    &entry.key,
                    entry.shares_input_path,
                )? {
                    keys.insert(key, group_key(group.kind, &next)?);
                }
            }
            for member in group
                .members
                .iter()
                .filter(|member| !mapped.contains(&member.key))
            {
                require(
                    compiled_key(movement, &destination, roots, script, &member.key, false)?
                        .is_none(),
                    format!(
                        "Cannot trace XAML key {} in Cargo group {} to a file input. Review the build's inputs or use a file-backed registration.",
                        member.key, group.module
                    ),
                )?;
            }
        }
        let names = plan_group(group, &keys)?;
        if names.is_empty() {
            continue;
        }
        let (file, source) = output.module_source(&group.module)?;
        renames.extend(declarations(&file_uri(&file)?, &source, names)?);
    }
    Ok(renames)
}

fn group_key(kind: CompiledGroupKind, key: &str) -> Result<String> {
    if kind == CompiledGroupKind::Documents {
        return Ok(key.to_owned());
    }
    relative_asset_path(key.trim_start_matches('/'))
        .and_then(|path| relative_asset_uri(&path))
        .ok_or_else(|| LanguageError(format!("Invalid XAML class document key {key}")))
}

fn discovered_keys(
    group: &CompiledXamlGroup,
    movement: &XamlFileMove,
    destination: &Path,
    root: &Path,
) -> Result<Option<(String, String)>> {
    let package = movement
        .package_root
        .canonicalize()
        .map_err(ferroui_designer_project::ProjectError::from)?;
    let from = file_path(&movement.from)?;
    let prior = relative_file_path(&package, &from)?;
    let prior = if group.kind == CompiledGroupKind::Class {
        relative_asset_uri(&prior)
            .ok_or_else(|| LanguageError("Invalid discovered class document path".into()))?
    } else {
        prior
    };
    if !group.members.iter().any(|member| member.key == prior) {
        return Ok(None);
    }
    let root_uri = file_uri(package.join(root))?;
    let root = file_path(&ferroui_designer_project::canonical_file_uri(&root_uri)?)?;
    require(
        destination
            .strip_prefix(root)
            .ok()
            .and_then(Path::parent)
            .is_some_and(|parent| {
                parent
                    .components()
                    .all(|component| !component.as_os_str().to_string_lossy().starts_with('.'))
            })
            && destination
                .extension()
                .is_some_and(|extension| extension == "xaml"),
        "The destination must be a .xaml file in the Cargo build's discovery root",
    )?;
    let next = relative_file_path(&package, destination)?;
    let next = if group.kind == CompiledGroupKind::Class {
        relative_asset_uri(&next)
            .ok_or_else(|| LanguageError("Invalid discovered class document path".into()))?
    } else {
        next
    };
    Ok(Some((prior, next)))
}

fn plan_group(
    group: &CompiledXamlGroup,
    replacements: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    if replacements.is_empty() {
        return Ok(BTreeMap::new());
    }
    let keys: Vec<_> = group
        .members
        .iter()
        .map(|member| member.key.as_str())
        .collect();
    let mut next: Vec<_> = keys
        .iter()
        .enumerate()
        .map(|(index, &key)| (index, replacements.get(key).map_or(key, String::as_str)))
        .collect();
    if keys.iter().copied().eq(next.iter().map(|(_, key)| *key)) {
        return Ok(BTreeMap::new());
    }
    let mut distinct = BTreeSet::new();
    require(
        next.iter().all(|(_, key)| distinct.insert(*key)),
        "The move would duplicate a compiled XAML key",
    )?;
    for (offset, (_, key)) in next
        .iter()
        .enumerate()
        .filter(|(_, (index, _))| group.members[*index].public)
    {
        require(
            !next[..offset].iter().any(|(other, existing)| {
                group.members[*other].public && same_document_uri(key, existing)
            }),
            format!(
                "The compiled XAML address {key} already exists in {}",
                group.module
            ),
        )?;
    }
    // The compiler walks automatic groups in path order. Class groups follow the
    // existing Include graph, which the move also rewrites to keep it connected.
    if group.discovery_root.is_some() && group.kind == CompiledGroupKind::Documents {
        next.sort_by(|(_, left), (_, right)| Path::new(left).cmp(Path::new(right)));
    }
    let before = group.kind.function_names(keys);
    let after = group.kind.function_names(next.iter().map(|(_, key)| *key));
    next.into_iter()
        .zip(after)
        .try_fold(BTreeMap::new(), |mut edits, ((index, _), new_name)| {
            let Some(old_name) = &group.members[index].symbol else {
                return Ok(edits);
            };
            if group.kind == CompiledGroupKind::Class && index == 0 {
                require(
                    old_name == "populate",
                    "Rebuild the package XAML schema to locate the class entry point",
                )?;
                return Ok(edits);
            }
            require(
                *old_name == before[index],
                "Rebuild the package XAML schema to verify its generated Rust names",
            )?;
            if *old_name != new_name {
                edits.insert(old_name.clone(), new_name);
            }
            Ok(edits)
        })
}

fn declarations(
    uri: &str,
    source: &str,
    mut names: BTreeMap<String, String>,
) -> Result<Vec<GeneratedRustRename>> {
    let mut result = Vec::with_capacity(names.len());
    for (line, text) in source.lines().enumerate() {
        let Some((signature, prefix)) =
            ["pub fn ", "pub(crate) fn "]
                .into_iter()
                .find_map(|prefix| {
                    text.strip_prefix(prefix)
                        .map(|signature| (signature, prefix))
                })
        else {
            continue;
        };
        let Some((name, _)) = signature.split_once('(') else {
            continue;
        };
        if let Some(new_name) = names.remove(name) {
            result.push(GeneratedRustRename {
                uri: uri.into(),
                position: Position {
                    line: line as u32,
                    character: prefix.len() as u32,
                },
                old_name: name.into(),
                new_name,
            });
        }
    }
    require(
        names.is_empty(),
        "Rebuild the package XAML schema to find the generated Rust declarations",
    )?;
    Ok(result)
}
