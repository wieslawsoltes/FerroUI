//! Retain the actual group membership and exported entry points for file refactors.
use crate::XamlGroup;
use ferroui_build_scan::{
    compiled_index::{CompiledXamlGroup, CompiledXamlMember},
    compiled_names::CompiledGroupKind,
    xaml_metadata::DocumentModel,
};
use std::{collections::BTreeMap, path::Path};

pub(super) fn group<'a>(
    group: &XamlGroup,
    kind: CompiledGroupKind,
    xaml_root: &Path,
    keys: impl IntoIterator<Item = &'a str>,
    documents: &[DocumentModel],
) -> CompiledXamlGroup {
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
                }
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Build, TypeSystem};
    use ferroui_build_scan::compiled_index::CompiledXamlIndex;
    use ferroui_build_scan::export::Export;
    use std::{env, fs};

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
        let build = Build::new(fixtures.clone(), out.clone(), "fixture", dependencies)
            .source_root("lib.rs")
            .type_system(TypeSystem::Model)
            .compile_group(group)
            .execute();
        assert!(build.errors.is_empty(), "{:?}", build.errors);
        let index: CompiledXamlIndex =
            serde_json::from_slice(&fs::read(out.join(CompiledXamlIndex::FILE_NAME)).unwrap())
                .unwrap();
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
