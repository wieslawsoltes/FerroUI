//! The export plan of the compiled markup and the assets a crate registers (#857).
//!
//! Not from upstream. `compiled_markup::register()` registers the loader table of the crate and
//! the table of its assets. Without a plan ([`ExportPlan::all`], the default), the loader table
//! answers every public document of every exported group, and the asset table holds every
//! embedded asset: a load by URI can reach any of them, so every build function and every payload
//! is linked. An application that loads a known set of documents by URI and reads a known set of
//! assets states that set ([`ExportPlan::closed`]): the loader table answers only the listed
//! documents, each through its own build function (or the loader of its class), and the asset
//! table holds only the listed assets. What the plan leaves out is not referenced by the
//! registration, so the linker removes it unless code names it.
//!
//! What stays: URIs are compared as before, without regard to case (`rt::uri_equals`); a
//! document is built with a root service provider over the one given, as the loader of its group
//! builds it; the checked-in loader tables come first; construction stays lazy. A listed URI that
//! is not a public document of the crate, and a listed asset the crate does not embed, are errors
//! of the build.
//!
//! The build writes `$OUT_DIR/xaml/exports.txt` in either mode: every compiled document with its
//! module, whether it is public and whether the loader table answers it, and every embedded asset
//! with its size in bytes and whether the asset table holds it.

use std::path::PathBuf;

use ferroui_build_scan::{packaged_assets::AssetDeclaration, xaml_metadata::DocumentModel};

/// What `compiled_markup::register()` of a crate registers (#857).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportPlan {
    /// `None`: every public document and every asset (the default). `Some`: the closed set.
    closed: Option<ClosedPlan>,
}

#[derive(Clone, Debug, Default)]
struct ClosedPlan {
    documents: Vec<String>,
    assets: Vec<AssetDeclaration>,
}

impl PartialEq for ClosedPlan {
    fn eq(&self, other: &Self) -> bool {
        self.documents == other.documents && self.assets.iter().map(|site| &site.value)
            .eq(other.assets.iter().map(|site| &site.value))
    }
}

impl Eq for ClosedPlan {}

impl ExportPlan {
    /// Every public document of the exported groups and every embedded asset: what a crate
    /// registers without a plan.
    pub fn all() -> Self {
        Self::default()
    }

    /// No document and no asset until [`document`](Self::document) and
    /// [`asset`](Self::asset) add them.
    pub fn closed() -> Self {
        Self { closed: Some(ClosedPlan::default()) }
    }

    /// Adds the document with the URI `uri` (`ferres://<assembly>/<path>`) to a closed plan; a
    /// plan of every document already has it.
    pub fn document(mut self, uri: &str) -> Self {
        if let Some(closed) = &mut self.closed {
            closed.documents.push(uri.to_string());
        }
        self
    }

    /// Adds the asset with the rooted path `path` (`/Assets/logo.png`) to a closed plan; a plan
    /// of every asset already has it.
    #[track_caller]
    pub fn asset(mut self, path: &str) -> Self {
        if let Some(closed) = &mut self.closed {
            closed.assets.push(AssetDeclaration::at(path, std::panic::Location::caller()));
        }
        self
    }

    /// Whether the plan is closed.
    pub fn is_closed(&self) -> bool {
        self.closed.is_some()
    }

    pub(crate) fn asset_registrations(&self) -> Option<Vec<AssetDeclaration>> {
        self.closed.as_ref().map(|closed| closed.assets.clone())
    }
}

/// The comparison of URIs and rooted paths of the loader: without regard to case.
fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// How the loader table of a closed plan answers one document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// Through the build function of the document (`super::<module>::<function>`).
    Build(String),
    /// Through the loader of the module of the class of the document.
    Module(String),
}

/// The plan applied to what the crate compiled and embeds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Applied {
    /// `None`: the loader table of every exported module. `Some`: the listed documents, each
    /// with its URI and how it is answered, in the order of the plan.
    pub(crate) documents: Option<Vec<(String, Answer)>>,
    /// The assets the asset table holds.
    pub(crate) assets: Vec<(String, PathBuf)>,
}

/// The module of `path` (`::<crate>::<module>::<item>`) of the crate `crate_name`.
fn module_of<'p>(path: &'p str, crate_name: &str) -> Option<(&'p str, &'p str)> {
    let rest = path.strip_prefix(&format!("::{crate_name}::"))?;
    rest.split_once("::")
}

/// `plan` applied to the documents the crate compiled (`documents`, from its `.xamlmeta`; the
/// modules that are exported are `exported`) and the assets it embeds. The errors name what the
/// plan lists and the crate does not have.
pub(crate) fn apply(
    plan: &ExportPlan,
    crate_name: &str,
    documents: &[DocumentModel],
    exported: &[String],
    assets: &[(String, PathBuf)],
) -> Result<Applied, Vec<String>> {
    let Some(closed) = &plan.closed else {
        return Ok(Applied { documents: None, assets: assets.to_vec() });
    };
    let mut errors = Vec::new();
    let mut answers = Vec::new();
    for uri in &closed.documents {
        let found = documents.iter().find(|document| same(&document.uri, uri) && document.public);
        let answer = found.and_then(|document| {
            if let Some((module, function)) = document.build_path.as_deref().and_then(|path| module_of(path, crate_name)) {
                return exported.iter().any(|known| known == module).then(|| Answer::Build(format!("super::{module}::{function}")));
            }
            let (module, _) = document.populate_path.as_deref().and_then(|path| module_of(path, crate_name))?;
            exported.iter().any(|known| known == module).then(|| Answer::Module(module.to_string()))
        });
        match answer {
            Some(answer) => answers.push((found.map_or_else(|| uri.clone(), |document| document.uri.clone()), answer)),
            None => errors.push(format!(
                "the export plan lists the document `{uri}`, which is not a public document of an exported group of the crate"
            )),
        }
    }
    let mut kept = Vec::new();
    for declaration in &closed.assets {
        let path = &declaration.value;
        match assets.iter().find(|(rooted, _)| same(rooted, path)) {
            Some(asset) => {
                if !kept.contains(asset) {
                    kept.push(asset.clone());
                }
            }
            None => errors.push(format!("the export plan lists the asset `{path}`, which the crate does not embed")),
        }
    }
    kept.sort();
    match errors.is_empty() {
        true => Ok(Applied { documents: Some(answers), assets: kept }),
        false => Err(errors),
    }
}

/// The body of `try_load` of the crate file for the documents of a closed plan: one branch for
/// each document, after the checked-in tables.
pub(crate) fn loader_branches(documents: &[(String, Answer)], literal: impl Fn(&str) -> String) -> String {
    let mut text = String::new();
    for (uri, answer) in documents {
        text.push_str(&format!("        if ::ferroui_markup_xaml::xaml_il::runtime::compiled::uri_equals(uri, {}, \"\") {{\n", literal(uri)));
        match answer {
            Answer::Build(function) => {
                text.push_str("            let provider = ::ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v3(\n");
                text.push_str("                service_provider.cloned(),\n");
                text.push_str("            );\n");
                text.push_str(&format!("            let root = {function}(::core::option::Option::Some(provider))?;\n"));
                text.push_str(
                    "            return ::core::result::Result::Ok(::core::option::Option::Some(::std::rc::Rc::new(root) as ::ferroui_base::BoxedValue));\n",
                );
            }
            Answer::Module(module) => {
                text.push_str(&format!("            return super::{module}::try_load(service_provider, uri);\n"));
            }
        }
        text.push_str("        }\n");
    }
    text
}

/// The text of `exports.txt`: the documents and the assets of the crate and what the plan does
/// with them.
pub(crate) fn report(plan: &ExportPlan, documents: &[DocumentModel], applied: &Applied, assets: &[(String, PathBuf)]) -> String {
    let mut text = String::new();
    let mode = match plan.is_closed() {
        true => "closed",
        false => "all",
    };
    text.push_str(&format!("# The export plan of the crate (#857): {mode}.\n"));
    text.push_str("# document <uri> <public|internal> <answered|not answered> <build or populate path>\n");
    for document in documents {
        let answered = match &applied.documents {
            None => document.public,
            Some(listed) => listed.iter().any(|(uri, _)| same(uri, &document.uri)),
        };
        let path = document.build_path.as_deref().or(document.populate_path.as_deref()).unwrap_or("-");
        text.push_str(&format!(
            "document {} {} {} {path}\n",
            document.uri,
            if document.public { "public" } else { "internal" },
            if answered { "answered" } else { "not answered" }
        ));
    }
    text.push_str("# asset <rooted path> <bytes> <registered|left out>\n");
    let mut total = (0u64, 0u64);
    for (rooted, path) in assets {
        let bytes = std::fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0);
        let registered = applied.assets.iter().any(|(kept, _)| kept == rooted);
        total.0 += bytes;
        if registered {
            total.1 += bytes;
        }
        text.push_str(&format!("asset {rooted} {bytes} {}\n", if registered { "registered" } else { "left out" }));
    }
    text.push_str(&format!("# assets: {} bytes embedded, {} bytes registered\n", total.0, total.1));
    text
}

#[cfg(test)]
#[path = "export_plan_tests.rs"]
mod tests;
