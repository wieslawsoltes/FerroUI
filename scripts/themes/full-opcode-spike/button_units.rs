// Build script for the selected WinUI probe. Let the production theme compiler compute the
// transitive resource, constructed-type and type-inheritance closure of the Button root.
use ferroui_build::theme_roots::{kept_units, Roots, TypeGraph};
use ferroui_build_scan::{model::AssemblyModel, xaml_metadata::XamlMetadata};
use std::{env, fs, path::PathBuf};

fn main() {
    let path = env::var_os("DEP_FERROUI_THEMES_WINUI_XAML_XAMLMETA").expect("WinUI metadata");
    let models = XamlMetadata::read(path).unwrap().into_iter().map(AssemblyModel::from_metadata).collect::<Vec<_>>();
    let graph = TypeGraph::from_models(&models);
    let theme = models.iter().flat_map(|m| &m.theme_units).find(|t| t.class.ends_with(".WinUITheme")).unwrap();
    let mut roots = Roots::default();
    roots.names.insert("Button".to_owned());
    let kept = kept_units(theme, &roots, &graph, false);
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let names = kept.iter().map(|i| theme.units.iter().find(|u| u.index == *i).unwrap().name.clone()).collect::<Vec<_>>();
    assert!(names.iter().any(|n| n == "Controls/Button.xaml"));
    let refs = kept.into_iter().map(|i| format!("&xaml::THEME_UNIT_{i}")).collect::<Vec<_>>().join(", ");
    fs::write(output.join("button_units.rs"), format!("&[{refs}]")).unwrap();
    let calendar = names.iter().any(|name| name == "Controls/Calendar.xaml");
    fs::write(output.join("expect-calendar.rs"), calendar.to_string()).unwrap();
    fs::write(output.join("button-units.txt"), names.join("\n")).unwrap();
}
