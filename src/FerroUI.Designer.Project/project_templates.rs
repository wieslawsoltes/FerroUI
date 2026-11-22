//! Review and create a standalone FerroUI application without running its build.
use crate::{require, ProjectError, Result};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};

#[path = "project_templates/manifest.rs"]
mod manifest;
#[path = "project_templates/publish.rs"]
mod publish;

pub const PUBLISHED_VERSION: &str = "0.5.0";
pub const RUST_VERSION: &str = env!("CARGO_PKG_RUST_VERSION");

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Template {
    DesktopRsx,
    DesktopXaml,
}

impl Template {
    pub const ALL: [Self; 2] = [Self::DesktopRsx, Self::DesktopXaml];
    pub fn label(self) -> &'static str {
        match self {
            Self::DesktopRsx => "Desktop app with RSX",
            Self::DesktopXaml => "Desktop app with XAML",
        }
    }
    pub fn entry(self) -> &'static str {
        match self {
            Self::DesktopRsx => "src/view.rs",
            Self::DesktopXaml => "Views/MainView.xaml",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::DesktopRsx => "An editable task counter with a Rust component and a signal.",
            Self::DesktopXaml => "A compiled XAML view with a live text binding.",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub template: Template,
    pub name: String,
    /// Omit to use the tested published release.
    pub checkout: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub path: &'static str,
    pub source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub root: PathBuf,
    pub entry: &'static str,
    pub manifest: &'static str,
    pub files: Vec<File>,
    pub review: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Created {
    pub root: PathBuf,
    pub entry: PathBuf,
    pub manifest: PathBuf,
}

/// The same catalog supplies the editor and both MCP hosts.
pub fn catalog() -> serde_json::Value {
    serde_json::json!({
        "publishedVersion": PUBLISHED_VERSION,
        "rustVersion": RUST_VERSION,
        "templates": Template::ALL.map(|template| serde_json::json!({
            "id": template, "label": template.label(), "description": template.description(),
            "entry": template.entry()
        }))
    })
}

pub fn plan(parent: &Path, request: &Request) -> Result<Plan> {
    validate_name(&request.name)?;
    let parent = parent.canonicalize()?;
    require(parent.is_dir(), "Choose an existing project parent folder")?;
    let root = parent.join(&request.name);
    match root.symlink_metadata() {
        Ok(_) => {
            return Err(ProjectError(format!(
                "Project folder already exists: {}",
                root.display()
            )))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let (manifest, managed_xaml) = manifest::render(request)?;
    let entry = request.template.entry();
    let title = request
        .name
        .split(['-', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| format!("{}{}", word[..1].to_ascii_uppercase(), &word[1..]))
        .collect::<Vec<_>>()
        .join(" ");
    let (main, build, view) = match request.template {
        Template::DesktopRsx => (
            include_str!("project_templates/files/rsx-main.rs.in"),
            include_str!("project_templates/files/rsx-build.rs.in"),
            include_str!("project_templates/files/rsx-view.rs.in"),
        ),
        Template::DesktopXaml => (
            include_str!("project_templates/files/xaml-main.rs.in"),
            include_str!("project_templates/files/xaml-build.rs.in"),
            include_str!("project_templates/files/xaml-view.xaml.in"),
        ),
    };
    let file = |path, source: &str| File {
        path,
        source: source
            .replace("__APP_TITLE__", &title)
            .replace("__ENTRY__", entry)
            .replace(
                "__RELOAD_MODULE__",
                if managed_xaml { "mod reload;\n\n" } else { "" },
            )
            .replace(
                "__RELOAD_HOOK__",
                if managed_xaml {
                    "reload::start"
                } else {
                    "|_| {}"
                },
            )
            .replace(
                "__RELOAD_MANIFEST__",
                if managed_xaml {
                    "\n        .hot_reload_manifest(true)"
                } else {
                    ""
                },
            ),
    };
    let mut files = vec![
        file(".gitignore", "/target\n"),
        file("build.rs", build),
        file(
            "README.md",
            include_str!("project_templates/files/readme.md.in"),
        ),
        file("src/main.rs", main),
        file(entry, view),
    ];
    if managed_xaml {
        files.push(file(
            "src/reload.rs",
            include_str!("project_templates/files/xaml-reload.rs.in"),
        ));
    }
    files.push(File {
        path: "Cargo.toml",
        source: manifest,
    });
    let review = format!(
        "{:016x}",
        ferroui_reload_protocol::fingerprint(&serde_json::to_vec(&(&root, &files))?,)
    );
    Ok(Plan {
        root,
        entry,
        manifest: "Cargo.toml",
        files,
        review,
    })
}

/// Replan immediately before publication. A matching review authorizes exactly
/// these bytes and destination.
pub fn create(parent: &Path, request: &Request, expected_review: &str) -> Result<Created> {
    let plan = plan(parent, request)?;
    require(
        plan.review == expected_review,
        "The project plan changed. Review the current files before creating it.",
    )?;
    publish::create(&plan.root, &plan.files)?;
    Ok(Created {
        entry: plan.root.join(plan.entry),
        manifest: plan.root.join(plan.manifest),
        root: plan.root,
    })
}

fn validate_name(name: &str) -> Result<()> {
    require(
        name.len() <= 64
            && name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            && name.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
            }),
        "Use up to 64 lowercase letters, digits, hyphens or underscores, starting with a letter.",
    )?;
    let crate_name = name.replace('-', "_");
    require(
        syn::parse_str::<syn::Ident>(&crate_name).is_ok()
            && !matches!(
                name,
                "std" | "core" | "alloc" | "test" | "aux" | "con" | "nul" | "prn"
            )
            && !["com", "lpt"].iter().any(|prefix| {
                name.strip_prefix(prefix).is_some_and(|suffix| {
                    suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_digit()
                })
            }),
        "Choose a project name available as a Rust crate and folder name.",
    )
}

#[cfg(test)]
#[path = "project_templates/tests.rs"]
mod tests;
