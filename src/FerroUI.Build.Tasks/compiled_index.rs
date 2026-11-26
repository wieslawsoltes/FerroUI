//! Retain the actual group membership and exported entry points for file refactors.
use crate::{GroupDocuments, XamlGroup};
use ferroui_base::utilities::UriRef;
use ferroui_build_scan::{
    compiled_index::{CompiledXamlGroup, CompiledXamlMember},
    compiled_names::CompiledGroupKind,
    xaml_metadata::DocumentModel,
};
use std::{collections::BTreeMap, path::Path};

pub(super) fn group<'a>(
    group: &XamlGroup,
    kind: CompiledGroupKind,
    root_uri: &str,
    xaml_root: &Path,
    keys: impl IntoIterator<Item = &'a str>,
    documents: &[DocumentModel],
) -> CompiledXamlGroup {
    let mut sources = BTreeMap::new();
    if let Some(GroupDocuments::Files(files)) = &group.documents {
        for (key, input) in files {
            let name = match kind {
                CompiledGroupKind::Documents => key.clone(),
                CompiledGroupKind::Class => {
                    let Ok(uri) = UriRef::parse_absolute(&format!("{root_uri}{key}")) else {
                        continue;
                    };
                    uri.path().trim_start_matches('/').to_owned()
                }
            };
            sources.insert(name, &input.path);
        }
    }
    let symbols: BTreeMap<_, _> = documents
        .iter()
        .filter_map(|document| {
            let (_, uri) = document.uri.split_once("://")?;
            let (_, key) = uri.split_once('/')?;
            let path = document
                .build_path
                .as_ref()
                .or(document.populate_path.as_ref())?;
            Some((key, (path.rsplit_once("::")?.1, document.public)))
        })
        .collect();
    CompiledXamlGroup {
        module: group.module.clone(),
        kind,
        discovery_root: group.documents.is_none().then(|| xaml_root.to_owned()),
        file_backed: matches!(group.documents, Some(GroupDocuments::Files(_))),
        members: keys
            .into_iter()
            .map(|key| {
                let (symbol, public) = symbols.get(key).map_or((None, false), |(name, public)| {
                    (Some(name.to_string()), *public)
                });
                CompiledXamlMember {
                    key: key.to_owned(),
                    symbol,
                    public,
                    source_path: sources.get(key).map(|path| (*path).clone()),
                }
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{xaml_document_file, Build, TypeSystem};
    use ferroui_build_scan::compiled_index::CompiledXamlIndex;
    use ferroui_build_scan::export::Export;
    use std::{env, fs};

    #[test]
    fn class_members_record_the_source_for_a_normalized_key() {
        let group = XamlGroup::new("page").document_files(&[xaml_document_file!(
            "Published/Page with spaces.xaml",
            "Sources/Page.xaml"
        )]);
        let index = super::group(
            &group,
            CompiledGroupKind::Class,
            "ferres://My.App/",
            Path::new("."),
            ["Published/Page%20with%20spaces.xaml"],
            &[],
        );
        assert_eq!(
            index.members[0].source_path.as_deref(),
            Some("Sources/Page.xaml")
        );
    }

    #[test]
    fn cargo_build_indexes_private_generated_functions() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
            .with_file_name("FerroUI.Build.Scan")
            .join("tests/fixtures/scanner");
        let out = env::temp_dir().join(format!("ferroui-build-symbols-{}", std::process::id()));
        let sources = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut dependencies = Vec::new();
        for (source, package) in [
            ("FerroUI.Base", "ferroui_base"),
            ("FerroUI.Controls", "ferroui_controls"),
            ("Markup/FerroUI.Markup.Xaml", "ferroui_markup_xaml"),
        ] {
            let output = out.join(package);
            let outcome = Export::new(
                sources.join(source),
                output.clone(),
                package,
                dependencies.clone(),
            )
            .source_root("lib.rs")
            .execute();
            assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
            dependencies.push(output.join(format!("{package}.xamlmeta")));
        }
        let document = "<Border xmlns='https://github.com/ferroui' />";
        let group = XamlGroup::new("views")
            .documents(&[("A-B.xaml", document), ("A_B.xaml", document)])
            .internal();
        let old_path = out.join("Original.xaml");
        let new_path = out.join("Renamed Card.xaml");
        fs::write(&old_path, document).unwrap();
        let key = "Published/Accent.xaml";
        let compile = |file| {
            let mut build = Build::new(
                fixtures.clone(),
                out.clone(),
                "fixture",
                dependencies.clone(),
            )
            .source_root("lib.rs")
            .type_system(TypeSystem::Model);
            for module in ["styles", "internal_styles"] {
                let group = XamlGroup::new(module).document_files(&[file]);
                build = build.compile_group(if module == "styles" {
                    group
                } else {
                    group.internal()
                });
            }
            build.execute()
        };
        let input_line = line!() + 1;
        let file = /* 🎨 */ xaml_document_file!(key, old_path.to_str().unwrap());
        let backed = compile(file);
        assert!(backed.errors.is_empty(), "{:?}", backed.errors);
        let read_index = || {
            CompiledXamlIndex::parse(
                &fs::read_to_string(out.join(CompiledXamlIndex::FILE_NAME)).unwrap(),
            )
            .unwrap()
        };
        let before = read_index();
        let [input] = before.file_inputs.as_slice() else {
            panic!("shared input should be recorded once");
        };
        assert_eq!(input.path, old_path.to_str().unwrap());
        assert_eq!(input.rust_file, Path::new(file!()));
        assert_eq!(input.line, input_line);
        let line = include_str!("compiled_index.rs")
            .lines()
            .nth(input_line as usize - 1)
            .unwrap();
        assert!(line
            .chars()
            .skip(input.column as usize - 1)
            .collect::<String>()
            .starts_with("xaml_document_file!"));
        assert!(before
            .groups
            .iter()
            .all(|group| group.file_backed && group.members[0].key == key));
        assert!(before
            .groups
            .iter()
            .all(|group| group.members[0].source_path.as_deref() == old_path.to_str()));
        let old_input = format!("cargo::rerun-if-changed={}", old_path.display());
        assert_eq!(
            backed
                .lines
                .iter()
                .filter(|line| **line == old_input)
                .count(),
            1
        );
        let generated = fs::read(out.join("xaml/styles.rs")).unwrap();
        let metadata_path = out.join("fixture.xamlmeta");
        let metadata = fs::read_to_string(&metadata_path).unwrap();
        let assembly = ferroui_build_scan::xaml_metadata::XamlMetadata::parse(&metadata).unwrap();
        assert_eq!(
            assembly.documents[0].uri,
            format!("ferres://{}/{key}", assembly.name)
        );

        fs::rename(&old_path, &new_path).unwrap();
        let rebuilt = compile(xaml_document_file!(key, new_path.to_str().unwrap()));
        assert!(rebuilt.errors.is_empty(), "{:?}", rebuilt.errors);
        let current = read_index();
        assert_eq!(current.file_inputs[0].path, new_path.to_str().unwrap());
        assert_eq!(
            current.groups[0].members[0].source_path.as_deref(),
            new_path.to_str()
        );
        assert!(!rebuilt.lines.contains(&old_input));
        assert_eq!(fs::read(out.join("xaml/styles.rs")).unwrap(), generated);
        assert_eq!(fs::read_to_string(&metadata_path).unwrap(), metadata);

        let build = Build::new(fixtures.clone(), out.clone(), "fixture", dependencies)
            .source_root("lib.rs")
            .type_system(TypeSystem::Model)
            .compile_group(group)
            .execute();
        assert!(build.errors.is_empty(), "{:?}", build.errors);
        let index = read_index();
        assert!(index.is_supported());
        assert_eq!(index.manifest, fixtures.join("Cargo.toml"));
        let [group] = index.groups.as_slice() else {
            panic!("expected one compiled group");
        };
        assert_eq!(group.module, "views");
        let generated = fs::read_to_string(out.join("xaml/views.rs")).unwrap();
        assert_eq!(group.members.len(), 2);
        let expected = CompiledGroupKind::Documents.function_names(["A-B.xaml", "A_B.xaml"]);
        for (entry, name) in group.members.iter().zip(expected) {
            assert_eq!(entry.symbol.as_deref(), Some(name.as_str()));
            assert!(
                generated.contains(&format!("pub fn {name}(")),
                "{name} missing"
            );
        }
        let assembly = ferroui_build_scan::xaml_metadata::XamlMetadata::parse(
            &fs::read_to_string(out.join("fixture.xamlmeta")).unwrap(),
        )
        .unwrap();
        assert!(assembly.documents.is_empty());
        fs::remove_dir_all(out).unwrap();
    }
}
