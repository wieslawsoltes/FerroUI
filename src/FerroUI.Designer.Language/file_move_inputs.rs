//! Resolve the actual inputs of computed Cargo groups back to Rust literals.
use super::{refers_to, rust_files};
use crate::{LanguageError, LanguageService, Result};
use ferroui_build_scan::compiled_index::CompiledXamlInput;
use ferroui_designer_project::{CompiledResources, RustXamlPath, RustXamlReferences, XamlFileMove};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(super) fn by_source(
    service: &LanguageService,
    movement: &XamlFileMove,
    inputs: Vec<CompiledXamlInput>,
    resources: &CompiledResources,
) -> Result<BTreeMap<String, Vec<CompiledXamlInput>>> {
    let package = movement.package_root.as_path();
    let mut result = BTreeMap::<_, Vec<_>>::new();
    let affected =
        inputs
            .into_iter()
            .filter(|input| match resources.current_input_source(&input.path) {
                Some(source) => source == movement.from,
                None => refers_to(package, &input.path, &movement.from),
            });
    let mut files = None;
    for input in affected {
        let files = files.get_or_insert_with(|| rust_files(service));
        let candidates: BTreeSet<_> = package
            .ancestors()
            .filter_map(|base| base.join(&input.rust_file).canonicalize().ok())
            .filter(|path| path.starts_with(package))
            .collect();
        let mut sources = files.values().filter(|(entry, project, _)| {
            candidates.contains(&entry.path)
                && project.authored_package_root(&entry.path) == package
        });
        let entry = match (sources.next(), sources.next()) {
            (Some((entry, _, _)), None) => entry,
            _ => return Err(unmapped(&input, package)),
        };
        result.entry(entry.uri.clone()).or_default().push(input);
    }
    Ok(result)
}

pub(super) fn paths<'a>(
    inputs: &[CompiledXamlInput],
    source: &str,
    references: &'a RustXamlReferences,
    movement: &XamlFileMove,
) -> Result<Vec<&'a RustXamlPath>> {
    let package = &movement.package_root;
    inputs
        .iter()
        .map(|input| {
            let offset = call_offset(source, input).ok_or_else(|| unmapped(input, package))?;
            references
                .file_documents
                .iter()
                .find(|candidate| {
                    candidate.callee.contains(&offset)
                        && refers_to(package, &candidate.site.path, &movement.from)
                })
                .map(|candidate| &candidate.site)
                .ok_or_else(|| unmapped(input, package))
        })
        .collect()
}

fn call_offset(source: &str, input: &CompiledXamlInput) -> Option<usize> {
    let body = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut offset = source.len() - body.len();
    let mut lines = body.split_inclusive('\n');
    for _ in 0..input.line.checked_sub(1)? {
        offset += lines.next()?.len();
    }
    let (local, _) = lines
        .next()?
        .char_indices()
        .nth(input.column.checked_sub(1)? as usize)?;
    Some(offset + local)
}

fn unmapped(input: &CompiledXamlInput, package: &Path) -> LanguageError {
    LanguageError(format!(
        "Cannot edit the compiler's XAML input {} at {}:{} in {}. Use a literal file path in xaml_document_file! and rebuild the project XAML schema.",
        input.path,
        input.rust_file.display(),
        input.line,
        package.display(),
    ))
}
