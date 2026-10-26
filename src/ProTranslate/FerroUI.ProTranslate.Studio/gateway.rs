use crate::StudioError;
use ferroui_protranslate_core::Culture;
use ferroui_protranslate_formats::{
    TranslationCatalog, TranslationCatalogConverter, TranslationExportResult,
    TranslationFileFormat, TranslationFormatDiagnostic, TranslationFormatOptions,
};

/// A successful import retains the complete normalized document and diagnostics.
#[derive(Clone, Debug)]
pub struct ImportedCatalog {
    pub catalog: TranslationCatalog,
    pub diagnostics: Vec<TranslationFormatDiagnostic>,
    pub file_name: String,
    pub format: TranslationFileFormat,
    pub target_culture: String,
}

/// Stateless gateway shared by file, clipboard and web hosts.
pub struct CatalogGateway;

impl CatalogGateway {
    pub fn import_from_storage(
        storage: &dyn crate::StudioStorage,
        path: &str,
        format: TranslationFileFormat,
        options: &TranslationFormatOptions,
        target_culture: &str,
    ) -> Result<ImportedCatalog, StudioError> {
        let bytes = storage
            .read(path)?
            .ok_or_else(|| StudioError::Storage(format!("Catalog '{path}' does not exist")))?;
        let content = std::str::from_utf8(&bytes)
            .map_err(|error| StudioError::InvalidInput(error.to_string()))?;
        Self::import(content, format, options, path, target_culture)
    }

    pub fn export_to_storage(
        storage: &dyn crate::StudioStorage,
        path: &str,
        catalog: &TranslationCatalog,
        format: TranslationFileFormat,
        options: &TranslationFormatOptions,
    ) -> Result<TranslationExportResult, StudioError> {
        let result = Self::export(catalog, format, options)?;
        storage.write_atomic(path, result.content.as_bytes())?;
        Ok(result)
    }

    pub fn import(
        content: &str,
        format: TranslationFileFormat,
        options: &TranslationFormatOptions,
        file_name: &str,
        target_culture: &str,
    ) -> Result<ImportedCatalog, StudioError> {
        let target_culture = normalize_culture(target_culture)?;
        let result = TranslationCatalogConverter::import(content, format, options, Some(file_name));
        if result.has_errors() {
            return Err(StudioError::Format(result.diagnostics));
        }
        Ok(ImportedCatalog {
            catalog: result.catalog,
            diagnostics: result.diagnostics,
            file_name: file_name.to_owned(),
            format,
            target_culture,
        })
    }

    pub fn export(
        catalog: &TranslationCatalog,
        format: TranslationFileFormat,
        options: &TranslationFormatOptions,
    ) -> Result<TranslationExportResult, StudioError> {
        let result = TranslationCatalogConverter::export(catalog, format, options);
        if result.has_errors() {
            Err(StudioError::Format(result.diagnostics))
        } else {
            Ok(result)
        }
    }
}

pub(crate) fn normalize_culture(value: &str) -> Result<String, StudioError> {
    Culture::parse(value)
        .map(|culture| culture.name().to_owned())
        .map_err(|error| StudioError::InvalidInput(error.to_string()))
}
