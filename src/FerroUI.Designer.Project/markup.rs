use crate::{ProjectError, Result, require};
use ferroui_build_scan::model::{AssemblyModel, RegisteredKind, TypeKind, TypeModel};
use ferroui_build_scan::model_set::ModelSet;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

#[path = "markup_symbols.rs"]
mod symbols;
pub use symbols::CompiledXamlOutput;

#[path = "markup_events.rs"]
mod events;
pub use events::MarkupEventSignature;

#[path = "markup_sources.rs"]
mod sources;
pub use sources::MarkupSource;

pub(crate) const MAX_DIRECTORIES: usize = 1024;
const MAX_FILES: usize = 256;
const MAX_BYTES: u64 = 64 * 1024 * 1024;
const CONTROL: &str = "::ferroui_controls::Control";
const TOP_LEVEL: &str = "::ferroui_controls::TopLevel";

/// Output directories reported by Cargo for one package build and its dependencies.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct BuildMetadata {
    pub manifest: PathBuf,
    pub directories: Vec<PathBuf>,
    #[serde(default)]
    pub package: Option<String>,
}

impl BuildMetadata {
    pub fn package_root(&self) -> Result<PathBuf> {
        require(
            self.manifest.is_absolute()
                && self
                    .manifest
                    .file_name()
                    .is_some_and(|name| name == "Cargo.toml"),
            "Expected an absolute Cargo manifest",
        )?;
        self.manifest
            .canonicalize()?
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| ProjectError("Missing package directory".into()))
    }

    pub fn catalog(&self) -> Result<MarkupCatalog> {
        MarkupCatalog::load(&self.directories)
    }
}

/// The versioned XAML registry emitted during a Cargo build. Loading metadata is
/// a bounded read of the reported outputs and never executes application code.
pub struct MarkupCatalog {
    models: ModelSet,
    directories: BTreeSet<PathBuf>,
    sources: sources::MarkupSources,
}

/// An element with a known XAML constructor and Control ancestry.
pub struct MarkupControl<'a> {
    pub assembly: &'a AssemblyModel,
    pub class: &'a TypeModel,
}

/// A styled property resolved from a build-generated Rust accessor.
pub struct MarkupAnimationProperty<'a> {
    pub name: &'a str,
    pub value_type: String,
    pub owner: MarkupControl<'a>,
}

impl MarkupControl<'_> {
    pub fn namespace(&self) -> String {
        format!(
            "clr-namespace:{};assembly={}",
            self.class.namespace, self.assembly.name
        )
    }
}

impl MarkupCatalog {
    pub fn load(directories: &[PathBuf]) -> Result<Self> {
        require(
            directories.len() <= MAX_DIRECTORIES,
            "Too many Cargo build output directories",
        )?;
        let mut paths = BTreeSet::new();
        for directory in directories.iter().filter(|path| path.is_absolute()) {
            let Ok(directory) = directory.canonicalize() else {
                continue;
            };
            let Ok(entries) = directory.read_dir() else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .is_some_and(|extension| extension == "xamlmeta")
                    && entry.file_type().is_ok_and(|kind| kind.is_file())
                {
                    paths.insert(path);
                    require(paths.len() <= MAX_FILES, "Too many XAML metadata files")?;
                }
            }
        }
        let mut models = Vec::new();
        let mut directories = BTreeSet::new();
        let mut sources = sources::MarkupSources::default();
        let mut source_budget = sources::INDEX_BYTES;
        let mut remaining = MAX_BYTES;
        for path in paths {
            let mut source = String::new();
            std::fs::File::open(&path)
                .and_then(|file| file.take(remaining + 1).read_to_string(&mut source))
                .map_err(|error| {
                    ProjectError(format!("Cannot read {}: {error}", path.display()))
                })?;
            require(
                source.len() as u64 <= remaining,
                "Cargo XAML metadata exceeds 64 MiB",
            )?;
            remaining -= source.len() as u64;
            let model = AssemblyModel::parse(&source).map_err(|error| {
                ProjectError(format!("Cannot read {}: {error}", path.display()))
            })?;
            if let Some(directory) = path.parent() {
                sources.read(directory, &model, &mut source_budget);
                directories.insert(directory.to_owned());
            }
            models.push(model);
        }
        Ok(Self {
            models: ModelSet::new(models),
            directories,
            sources,
        })
    }

    pub fn models(&self) -> &ModelSet {
        &self.models
    }

    pub fn animation_property(&self, accessor: &str) -> Option<MarkupAnimationProperty<'_>> {
        let (listed, property) = self.models.find_accessor(accessor)?;
        if property.kind != RegisteredKind::Styled || property.read_only {
            return None;
        }
        let path = property.owner.as_ref().unwrap_or(&listed.rust_path);
        let (assembly, class) = self.models.find_rust_type(&path.text)?;
        let name = self.models.name_of(property)?;
        if !element_name(name) || !element_name(&class.name) || class.namespace.is_empty()
            || !namespace_part(&class.namespace) || !namespace_part(&assembly.name)
        {
            return None;
        }
        Some(MarkupAnimationProperty {
            name,
            value_type: self.models.expanded(&property.value_type.text),
            owner: MarkupControl { assembly, class },
        })
    }

    pub fn controls(&self) -> impl Iterator<Item = MarkupControl<'_>> {
        self.models.models().iter().flat_map(move |assembly| {
            assembly.types.iter().filter_map(move |class| {
                (class.kind == TypeKind::Class
                    && class.object_model
                    && !class.unregistered
                    && class.generic.is_none()
                    && !class.namespace.is_empty()
                    && !assembly.name.is_empty()
                    && element_name(&class.name)
                    && namespace_part(&class.namespace)
                    && namespace_part(&assembly.name)
                    && constructible(class)
                    && self.derives_from(class, CONTROL))
                .then_some(MarkupControl { assembly, class })
            })
        })
    }

    /// A top-level control can be previewed as a document but cannot be inserted as a child.
    pub fn is_top_level(&self, class: &TypeModel) -> bool {
        self.derives_from(class, TOP_LEVEL)
    }

    fn derives_from(&self, class: &TypeModel, ancestor: &str) -> bool {
        let ancestor = self.models.canonical_path(ancestor);
        let mut base = Some(&class.rust_path);
        let mut seen = BTreeSet::new();
        while let Some(path) = base {
            let path = self.models.canonical_path(&path.text);
            if path == ancestor {
                return true;
            }
            if !seen.insert(path.clone()) {
                return false;
            }
            base = self
                .models
                .find_rust_type(&path)
                .and_then(|(_, class)| class.base.as_ref());
        }
        false
    }
}

fn element_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
        && chars.all(|c| c == '_' || c.is_alphanumeric())
}

fn namespace_part(value: &str) -> bool {
    !value
        .chars()
        .any(|character| character == ';' || character.is_control())
}

pub(crate) fn constructible(class: &TypeModel) -> bool {
    class.default_constructor.is_some()
        || class
            .constructors
            .iter()
            .any(|constructor| constructor.parameters.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_build_scan::model::{CallableModel, ExportModel, RustType};

    #[test]
    fn controls_follow_dependency_exports_and_skip_nonvisual_and_unregistered_types() {
        let control = TypeModel::new(
            "Control",
            TypeKind::Class,
            RustType::resolved("::ferroui_controls::control::Control"),
            "ferroui_controls::control",
        );
        let mut framework = AssemblyModel::new("FerroUI.Controls", "ferroui_controls");
        framework.exports.push(ExportModel {
            path: CONTROL.into(),
            declared: control.rust_path.text.clone(),
        });
        framework.types.push(control);

        let mut badge = TypeModel::new(
            "Badge",
            TypeKind::Class,
            RustType::resolved("::widgets::Badge"),
            "widgets",
        );
        badge.namespace = "Widgets.Controls".into();
        badge.object_model = true;
        badge.base = Some(RustType::resolved(CONTROL));
        badge.default_constructor = Some(CallableModel::default());
        let mut widgets = AssemblyModel::new("Widgets", "widgets");
        widgets.types.push(badge.clone());
        let mut shell = AssemblyModel::new("Shell", "shell");
        badge.name = "HiddenBadge".into();
        badge.rust_path = RustType::resolved("::shell::HiddenBadge");
        badge.unregistered = true;
        shell.types.push(badge.clone());
        badge.name = "BadgeSettings".into();
        badge.rust_path = RustType::resolved("::shell::BadgeSettings");
        badge.unregistered = false;
        badge.base = None;
        shell.types.push(badge);

        let catalog = MarkupCatalog {
            directories: BTreeSet::new(),
            models: ModelSet::new(vec![shell, widgets, framework]),
            sources: sources::MarkupSources::default(),
        };
        let found = catalog.controls().collect::<Vec<_>>();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].class.name, "Badge");
        assert_eq!(
            found[0].namespace(),
            "clr-namespace:Widgets.Controls;assembly=Widgets"
        );
    }
}
