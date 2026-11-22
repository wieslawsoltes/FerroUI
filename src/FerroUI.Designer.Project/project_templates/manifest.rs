use super::{Request, Template, PUBLISHED_VERSION, RUST_VERSION};
use crate::{require, ProjectError, Result};
use std::{fmt::Write, path::Path};

const BASE: (&str, &str) = ("ferroui-base", "src/FerroUI.Base");
const VIEW: (&str, &str) = ("ferroui-view", "src/FerroUI.View");
const THEME: (&str, &str) = ("ferroui-themes-winui", "src/FerroUI.Themes.WinUI");
const CONTROLS: (&str, &str) = ("ferroui-controls", "src/FerroUI.Controls");
const XAML: (&str, &str) = ("ferroui-markup-xaml", "src/Markup/FerroUI.Markup.Xaml");
const BUILD: (&str, &str) = ("ferroui-build", "src/FerroUI.Build.Tasks");
const RELOAD: (&str, &str) = ("ferroui-hot-reload", "src/FerroUI.HotReload");
const SKIA_PATCH: (&str, &str) = ("skia-bindings", "external/rust-skia-bindings");

pub(super) fn render(request: &Request) -> Result<(String, bool)> {
    let checkout = request
        .checkout
        .as_ref()
        .map(|path| path.canonicalize())
        .transpose()?;
    if let Some(root) = checkout.as_ref() {
        require(
            root.join("Cargo.toml").is_file(),
            "Choose the root of a FerroUI source checkout",
        )?;
    }
    let mut text = format!(
        "[package]\nname = {:?}\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = {:?}\npublish = false\n\n[workspace]\n\n[dependencies]\n",
        request.name, RUST_VERSION,
    );
    let root = checkout.as_deref();
    let xaml = request.template == Template::DesktopXaml;
    let managed_xaml = xaml
        && root.is_some_and(|root| root.join("src/FerroUI.ReloadProtocol/Cargo.toml").is_file());
    dependency(
        &mut text,
        root,
        BASE,
        if xaml { &["markup-functions"] } else { &[] },
    )?;
    dependency(
        &mut text,
        root,
        VIEW,
        if xaml {
            &["desktop"]
        } else {
            &["desktop", "hot-reload"]
        },
    )?;
    dependency(&mut text, root, THEME, &[])?;
    if xaml {
        dependency(&mut text, root, CONTROLS, &[])?;
        dependency(&mut text, root, XAML, &[])?;
        if managed_xaml {
            dependency(&mut text, root, RELOAD, &[])?;
        }
    }
    text.push_str("\n[build-dependencies]\n");
    dependency(&mut text, root, BUILD, &[])?;
    if root.is_some_and(|root| root.join(SKIA_PATCH.1).join("Cargo.toml").is_file()) {
        // Cargo only applies patch tables from the consuming workspace.
        text.push_str("\n[patch.crates-io]\n");
        dependency(&mut text, root, SKIA_PATCH, &[])?;
    }
    Ok((text, managed_xaml))
}

fn dependency(
    manifest: &mut String,
    checkout: Option<&Path>,
    (name, relative): (&str, &str),
    features: &[&str],
) -> Result<()> {
    let source = if let Some(root) = checkout {
        let directory = root.join(relative);
        require(
            directory.join("Cargo.toml").is_file(),
            format!("The FerroUI checkout is missing {name}"),
        )?;
        let physical = directory.canonicalize()?;
        require(
            physical.starts_with(root),
            format!("The {name} dependency resolves outside the checkout"),
        )?;
        let path = physical
            .to_str()
            .ok_or_else(|| ProjectError("FerroUI checkout paths must be UTF-8".into()))?;
        format!("path = {}", serde_json::to_string(path)?)
    } else {
        format!("version = \"={PUBLISHED_VERSION}\"")
    };
    let modes = if features.is_empty() {
        String::new()
    } else {
        format!(", features = {features:?}")
    };
    writeln!(manifest, "{name} = {{ {source}{modes} }}").expect("writing to String");
    Ok(())
}
