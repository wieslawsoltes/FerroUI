//! Materialize mapped Rust colors as native resources for the isolated design.
use super::{INDENT, RsxStyle, resource_key_attribute};
use ferroui_designer_tools::component_palette::PaletteLayer;
use ferroui_designer_tools::theme_variants::HighContrastProvider;
use std::fmt::Write;

pub(super) fn write(source: &mut String, style: &RsxStyle, pad: &str) {
    let mut layers = PaletteLayer::ALL
        .into_iter()
        .filter_map(|layer| {
            let entries: Vec<_> = style
                .resource_entries(layer)
                .filter_map(|entry| Some((resource_key_attribute(&entry.key), entry.color()?)))
                .collect();
            (!entries.is_empty()).then_some((layer, entries))
        })
        .peekable();
    if layers.peek().is_none() {
        return;
    }
    let _ = write!(
        source,
        "\n{pad}{INDENT}<Style.Resources>\n{pad}{INDENT}{INDENT}<ResourceDictionary xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\">"
    );
    let dictionary = format!("{pad}{INDENT}{INDENT}");
    if layers
        .peek()
        .is_some_and(|(layer, _)| *layer == PaletteLayer::Shared)
    {
        let (_, shared) = layers.next().expect("shared layer");
        colors(source, &dictionary, &shared);
    }
    if layers.peek().is_some() {
        let _ = write!(
            source,
            "\n{dictionary}{INDENT}<ResourceDictionary.ThemeDictionaries>"
        );
        let themes = format!("{dictionary}{INDENT}{INDENT}");
        for (layer, entries) in layers {
            let _ = write!(source, "\n{themes}<ResourceDictionary");
            if layer == PaletteLayer::HighContrast {
                let provider = HighContrastProvider::WinUi;
                let _ = write!(
                    source,
                    " xmlns:rsxContrast=\"using:{}\" x:Key=\"{{x:Static rsxContrast:{}.HighContrast}}\">",
                    provider.assembly(),
                    provider.theme_class()
                );
            } else {
                let _ = write!(source, " x:Key=\"{}\">", layer.title());
            }
            colors(source, &themes, &entries);
            let _ = write!(source, "\n{themes}</ResourceDictionary>");
        }
        let _ = write!(
            source,
            "\n{dictionary}{INDENT}</ResourceDictionary.ThemeDictionaries>"
        );
    }
    let _ = write!(
        source,
        "\n{dictionary}</ResourceDictionary>\n{pad}{INDENT}</Style.Resources>"
    );
}

fn colors(source: &mut String, pad: &str, colors: &[(String, String)]) {
    for (key, color) in colors {
        let _ = write!(
            source,
            "\n{pad}{INDENT}<Color x:Key=\"{key}\">{color}</Color>"
        );
    }
}
