// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::source_declarations::test_only;
use ferrodock_serializer_system_text_json_generators::{
    Compilation, CompilationError, Registration, TypeId,
};
use ferroui_build_scan::{model::RustType, scanner::Scan};
use quote::ToTokens;
use syn::{spanned::Spanned, visit::Visit};

/// One source registration before the original semantic generator's filtering.
#[derive(Clone, Debug)]
pub struct RustGenerationRegistration {
    /// Original logical Rust type, normalized by the shared scanner.
    pub type_: RustType,
    /// Source file, line and column of the registration macro.
    pub location: String,
}

/// Crate-scope source-generation attribute declarations discovered from Rust source.
#[derive(Clone, Debug, Default)]
pub struct RustGenerationAttributes {
    /// Whether the source activation macro is present.
    pub activated: bool,
    /// Source location of the activation marker, if any.
    pub activation_location: Option<String>,
    /// Registrations in declaration order, retaining duplicates for the semantic pass.
    pub registrations: Vec<RustGenerationRegistration>,
}

struct Reader<'a> {
    scan: &'a Scan,
    module: &'a str,
    file: &'a str,
    result: RustGenerationAttributes,
    error: Option<CompilationError>,
}
impl Reader<'_> {
    fn record(&mut self, mac: &syn::Macro, at_crate_scope: bool, attrs: &[syn::Attribute]) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.read(mac, at_crate_scope, attrs) {
            self.error = Some(error);
        }
    }
    fn read(
        &mut self,
        mac: &syn::Macro,
        at_crate_scope: bool,
        attrs: &[syn::Attribute],
    ) -> Result<(), CompilationError> {
        let path = mac.path.to_token_stream().to_string();
        let Some(path) = self.scan.normalise(self.module, &path) else {
            return Ok(());
        };
        if !path.is_resolved() {
            return Ok(());
        }
        let activation =
            path.text == "::ferrodock_serializer_system_text_json::dock_json_source_generation";
        let registration =
            path.text == "::ferrodock_serializer_system_text_json::dock_json_serializable";
        if !activation && !registration {
            return Ok(());
        }
        let start = mac.path.span().start();
        let location = format!("{}:{}:{}", self.file, start.line, start.column + 1);
        let fail = |message: &str| CompilationError(format!("{location}: {message}"));
        if !at_crate_scope || self.module != self.scan.model.crate_name {
            return Err(fail("Dock JSON assembly attributes require crate scope"));
        }
        if attrs
            .iter()
            .any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
        {
            return Err(fail(
                "Conditional Dock JSON attribute requires target cfg selection",
            ));
        }
        if activation {
            if !mac.tokens.is_empty() {
                return Err(fail("DockJsonSourceGeneration takes no arguments"));
            }
            if self.result.activated {
                return Err(fail(
                    "DockJsonSourceGeneration cannot be declared more than once",
                ));
            }
            self.result.activated = true;
            self.result.activation_location = Some(location);
        } else {
            let ty = syn::parse2::<syn::Type>(mac.tokens.clone())
                .map_err(|_| fail("DockJsonSerializable requires one Rust type"))?;
            let type_ = self
                .scan
                .normalise(self.module, &ty.to_token_stream().to_string())
                .ok_or_else(|| fail("Cannot normalize DockJsonSerializable type"))?;
            if !type_.is_resolved() {
                return Err(fail("Unresolved DockJsonSerializable type"));
            }
            self.result
                .registrations
                .push(RustGenerationRegistration { type_, location });
        }
        Ok(())
    }
}
impl<'ast> Visit<'ast> for Reader<'_> {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.record(mac, false, &[]);
    }
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if !test_only(&item.attrs) {
            syn::visit::visit_item_macro(self, item);
        }
    }
    fn visit_stmt_macro(&mut self, item: &'ast syn::StmtMacro) {
        if !test_only(&item.attrs) {
            syn::visit::visit_stmt_macro(self, item);
        }
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !test_only(&item.attrs) {
            syn::visit::visit_item_mod(self, item);
        }
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !test_only(&item.attrs) {
            syn::visit::visit_item_fn(self, item);
        }
    }
}

/// Reads the actual source markers using the shared scan's module/path resolution.
///
/// `source` is a crate-scope source file or literal included source fragment whose
/// imports and types belong to `module` in `scan`. Qualified markers and imports
/// renamed by that scan are recognized by their resolved identity. Only these
/// original assembly attributes are read; arbitrary macro bodies are not expanded.
/// Nested markers, duplicate activation and unselected cfg return source errors.
/// Test-only items (`cfg(test)` and `cfg(all(test, ...))`) follow the shared production scan.
pub fn parse_generation_attributes(
    scan: &Scan,
    module: &str,
    file: &str,
    source: &str,
) -> Result<RustGenerationAttributes, CompilationError> {
    if scan.normalise(module, "String").is_none() {
        return Err(CompilationError(format!(
            "{file}: unknown scanner module {module}"
        )));
    }
    let parsed = syn::parse_file(source)
        .map_err(|e| CompilationError(format!("{file}: invalid Rust registration source: {e}")))?;
    let mut reader = Reader {
        scan,
        module,
        file,
        result: RustGenerationAttributes::default(),
        error: None,
    };
    for item in &parsed.items {
        if let syn::Item::Macro(item) = item {
            if !test_only(&item.attrs) {
                reader.record(&item.mac, true, &item.attrs);
            }
        } else {
            reader.visit_item(item);
        }
    }
    if let Some(error) = reader.error {
        Err(error)
    } else {
        Ok(reader.result)
    }
}

/// Replaces a compilation's attribute input with discovered Rust declarations.
///
/// The target compiler must resolve every logical type to its semantic symbol;
/// missing facts are adapter errors, not fabricated invalid registrations. The
/// original semantic pass still owns validation, diagnostics and deduplication.
/// Resolution completes before the compilation is changed.
pub fn apply_generation_attributes(
    compilation: &mut Compilation,
    attributes: &RustGenerationAttributes,
    resolve: impl Fn(&RustType) -> Option<TypeId>,
) -> Result<(), CompilationError> {
    let mut registrations = Vec::with_capacity(attributes.registrations.len());
    for registration in &attributes.registrations {
        if !registration.type_.is_resolved() {
            return Err(CompilationError(format!(
                "{}: Unresolved registration type",
                registration.location
            )));
        }
        let id = resolve(&registration.type_).ok_or_else(|| {
            CompilationError(format!(
                "{}: Missing semantic symbol for {}",
                registration.location, registration.type_.text
            ))
        })?;
        let symbol = compilation.types.get(id).ok_or_else(|| {
            CompilationError(format!(
                "{}: Missing compiler type {id}",
                registration.location
            ))
        })?;
        registrations.push(Registration {
            type_id: Some(id),
            valid_type_constant: true,
            display_name: symbol.display_name.clone(),
            location: Some(registration.location.clone()),
        });
    }
    compilation.activated = attributes.activated;
    compilation.registrations = registrations;
    Ok(())
}
