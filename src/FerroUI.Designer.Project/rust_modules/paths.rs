use std::path::{Path, PathBuf};
use syn::{Expr, ItemMod, Lit, Meta, ext::IdentExt};

pub(super) struct ModuleDirectory {
    file_directory: PathBuf,
    children: PathBuf,
    inline: bool,
}

pub(super) struct ModulePath {
    pub(super) path: PathBuf,
    pub(super) own_directory: bool,
}

impl ModuleDirectory {
    pub(super) fn file(path: &Path, own_directory: bool) -> Option<Self> {
        let directory = path.parent()?;
        let children = if own_directory {
            directory.to_owned()
        } else {
            directory.join(path.file_stem()?)
        };
        Some(Self {
            file_directory: directory.to_owned(),
            children,
            inline: false,
        })
    }

    pub(super) fn inline(&self, item: &ItemMod) -> Option<Self> {
        let children = match override_path(item)? {
            Some(path) => self.children.join(path),
            None => self.children.join(item.ident.unraw().to_string()),
        };
        Some(Self {
            file_directory: self.file_directory.clone(),
            children,
            inline: true,
        })
    }

    pub(super) fn files(&self, item: &ItemMod) -> Option<(ModulePath, Option<ModulePath>)> {
        if let Some(path) = override_path(item)? {
            let base = if self.inline {
                &self.children
            } else {
                &self.file_directory
            };
            return Some((
                ModulePath {
                    path: base.join(path),
                    own_directory: true,
                },
                None,
            ));
        }
        let name = item.ident.unraw().to_string();
        Some((
            ModulePath {
                path: self.children.join(format!("{name}.rs")),
                own_directory: false,
            },
            Some(ModulePath {
                path: self.children.join(name).join("mod.rs"),
                own_directory: true,
            }),
        ))
    }
}

fn override_path(item: &ItemMod) -> Option<Option<String>> {
    let mut paths = item
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("path"));
    let Some(path) = paths.next() else {
        return Some(None);
    };
    if paths.next().is_some() {
        return None;
    }
    let Meta::NameValue(path) = &path.meta else {
        return None;
    };
    let Expr::Lit(path) = &path.value else {
        return None;
    };
    let Lit::Str(path) = &path.lit else {
        return None;
    };
    Some(Some(path.value()))
}
