//! Include explicit builder mappings in the same checked XAML and Rust review.
use super::{source_review::RustSnapshot, FileMoveDraft, LanguageService, ReviewedDocument};
use crate::{project_schema::ProjectSchema, require, LanguageError, Result};
use ferroui_build_scan::{authored_symbols::AuthoredMetadata, model::AssemblyModel};
use ferroui_designer_core::TextPatch;
use ferroui_designer_project::{
    file_uri,
    rsx_mapping::{rename_key, RsxBuilderMapping, PREFIX},
    MarkupControl, MarkupSource, RustXamlReferences,
};
use std::collections::BTreeMap;

pub(super) enum RenameKind {
    Property,
    Type { rename_rust: bool },
}

pub(super) struct MappingRename<'a> {
    pub registration: &'a MarkupSource,
    pub old: &'a str,
    pub new: &'a str,
    pub kind: RenameKind,
}

struct MappingEdit<'a> {
    key: &'a str,
    new_key: Option<String>,
    property: bool,
    builder: bool,
}

impl LanguageService {
    pub(super) fn review_rsx_mappings(
        &self,
        rename: MappingRename<'_>,
        drafts: &BTreeMap<String, &FileMoveDraft>,
        documents: &mut Vec<ReviewedDocument>,
    ) -> Result<()> {
        for project in self.package_schemas.values() {
            for assembly in project.catalog.models().models() {
                let edits = rename.in_assembly(project, assembly)?;
                if edits.is_empty() {
                    continue;
                }
                let declaration = project.catalog.authored_assembly(assembly).ok_or_else(|| {
                    LanguageError(format!(
                        "Rebuild the Cargo schema to locate the RSX mappings for {}",
                        assembly.name
                    ))
                })?;
                let file = declaration.file().ok_or_else(|| {
                    LanguageError(format!(
                        "The Rust assembly source for {} moved. Rebuild the Cargo schema",
                        assembly.name
                    ))
                })?;
                let uri = file_uri(file)?;
                require(
                    self.workspace_root(&uri).is_some(),
                    format!(
                        "Open the workspace containing the RSX mappings for {}",
                        assembly.name
                    ),
                )?;
                let source = RustSnapshot::read(&uri, drafts)?;
                let entries = declaration.metadata_in(&source.text).ok_or_else(|| {
                    LanguageError(format!(
                        "Repair the selected Rust assembly metadata before review: {uri}"
                    ))
                })?;
                require(
                    entries.iter().all(|entry| entry.key.is_some()),
                    format!("The keys in the selected assembly need literals or text constants from this source file before review: {uri}"),
                )?;
                let mut patches = Vec::new();
                for edit in edits {
                    rename.edits_in_source(
                        project,
                        assembly,
                        edit,
                        &source.text,
                        &entries,
                        &mut patches,
                    )?;
                }
                let document = source.merge(&uri, patches, documents)?;
                RustXamlReferences::parse(&document.after, false)?;
                documents.push(document);
            }
        }
        Ok(())
    }
}

impl MappingRename<'_> {
    fn in_assembly<'a>(
        &self,
        project: &ProjectSchema,
        assembly: &'a AssemblyModel,
    ) -> Result<Vec<MappingEdit<'a>>> {
        let mut edits = Vec::new();
        for (key, value) in &assembly.metadata {
            let Some(full_name) = key.strip_prefix(PREFIX) else {
                continue;
            };
            let Some(class) = assembly
                .types
                .iter()
                .find(|class| class.full_name() == full_name)
            else {
                continue;
            };
            let mut edit = MappingEdit {
                key,
                new_key: None,
                property: false,
                builder: false,
            };
            match self.kind {
                RenameKind::Property => {
                    let namespace = MarkupControl { assembly, class }.namespace();
                    let member =
                        project
                            .effective
                            .property(&class.name, Some(&namespace), self.old);
                    if member
                        .and_then(|member| member.authored)
                        .is_some_and(|source| source.same_registration(self.registration))
                    {
                        edit.property = RsxBuilderMapping::parse(value)?
                            .properties
                            .contains_key(self.old);
                    }
                }
                RenameKind::Type { rename_rust } => {
                    if project
                        .catalog
                        .authored_definition(assembly, class, None)
                        .is_some_and(|source| source.same_registration(self.registration))
                    {
                        let prefix = key.strip_suffix(self.old).ok_or_else(|| {
                            LanguageError(
                                "The published RSX type name changed before review".into(),
                            )
                        })?;
                        edit.new_key = Some(format!("{prefix}{}", self.new));
                    }
                    edit.builder = rename_rust
                        && RsxBuilderMapping::parse(value).is_ok_and(|mapping| {
                            self.renamed_builder(project, assembly, &mapping).is_some()
                        });
                }
            }
            if edit.new_key.is_none() && !edit.property && !edit.builder {
                continue;
            }
            require(
                assembly
                    .metadata
                    .iter()
                    .filter(|(name, _)| name == key)
                    .count()
                    == 1,
                format!("The Cargo schema has more than one mapping for {full_name}"),
            )?;
            if let Some(key) = &edit.new_key {
                require(
                    !assembly.metadata.iter().any(|(name, _)| name == key),
                    format!("The Cargo schema already has the RSX mapping {key}"),
                )?;
            }
            edits.push(edit);
        }
        Ok(edits)
    }

    fn edits_in_source(
        &self,
        project: &ProjectSchema,
        assembly: &AssemblyModel,
        edit: MappingEdit<'_>,
        source: &str,
        entries: &[AuthoredMetadata],
        patches: &mut Vec<TextPatch>,
    ) -> Result<()> {
        let mut matches = entries
            .iter()
            .filter(|entry| entry.key.as_ref().is_some_and(|key| key.value == edit.key));
        let entry = matches.next().ok_or_else(|| {
            LanguageError(format!(
                "The RSX mapping {} changed in Rust. Rebuild the project schema",
                edit.key
            ))
        })?;
        require(
            matches.next().is_none(),
            format!(
                "The current Rust assembly repeats the RSX mapping {}",
                edit.key
            ),
        )?;
        let inspect_builder = matches!(self.kind, RenameKind::Type { rename_rust: true })
            && (edit.builder || edit.new_key.is_some());
        if let Some(new_key) = edit.new_key {
            require(
                !entries
                    .iter()
                    .any(|entry| entry.key.as_ref().is_some_and(|key| key.value == new_key)),
                format!("The current Rust assembly already has the RSX mapping {new_key}"),
            )?;
            patches.push(rename_key(
                source,
                entry.key.as_ref().expect("matched key"),
                &new_key,
            )?);
        }
        if edit.property || inspect_builder {
            let value = entry.value.as_ref().ok_or_else(|| {
                LanguageError(format!(
                    "Use a literal or a text constant from the assembly source file for the RSX mapping {}",
                    edit.key
                ))
            })?;
            let mapping = RsxBuilderMapping::parse(&value.value)?;
            if edit.property {
                patches.push(mapping.rename_property(source, value, self.old, self.new)?);
            }
            if inspect_builder {
                if let Some(next) = self.renamed_builder(project, assembly, &mapping) {
                    patches.push(RsxBuilderMapping::rename_path(source, value, &next)?);
                } else {
                    require(
                        !edit.builder,
                        format!(
                            "The Rust builder of {} changed. Rebuild the project schema",
                            edit.key
                        ),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn renamed_builder(
        &self,
        project: &ProjectSchema,
        assembly: &AssemblyModel,
        mapping: &RsxBuilderMapping,
    ) -> Option<String> {
        mapping.renamed_builder(
            project.catalog.models(),
            &assembly.crate_name,
            self.registration.rust_type(),
            self.old,
            self.new,
        )
    }
}
