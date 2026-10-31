use crate::{LanguageService, Location};
use std::collections::BTreeSet;

const URI: &str = "untitled:/View.xaml";
const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

fn service(source: &str) -> LanguageService {
    let mut service = LanguageService::default();
    service.open(URI, source, 1).unwrap();
    service
}

fn positions(service: &LanguageService, locations: &[Location]) -> BTreeSet<usize> {
    locations
        .iter()
        .map(|location| {
            service
                .index(&location.uri)
                .unwrap()
                .span(location.range)
                .unwrap()
                .start
        })
        .collect()
}

fn declaration(source: &str, name: &str) -> usize {
    let marker = source.find(&format!("Name='{name}'")).unwrap();
    source[..marker].rfind("Accent").unwrap()
}

#[test]
fn theme_resources_keep_all_variant_definitions_and_reject_partial_family_renames() {
    let source = format!(
        "<Grid {NS}><Grid.Resources><ResourceDictionary><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Light'><Color x:Key='Accent' Name='LightBrush'/></ResourceDictionary><ResourceDictionary x:Key='Dark'><Color x:Key='Accent' Name='DarkBrush'/></ResourceDictionary></ResourceDictionary.ThemeDictionaries></ResourceDictionary></Grid.Resources><Button Background='{{DynamicResource Accent}}'/></Grid>"
    );
    let service = service(&source);
    let reference = source.rfind("Accent").unwrap();
    let light = declaration(&source, "LightBrush");
    let dark = declaration(&source, "DarkBrush");
    assert_eq!(
        positions(&service, &service.definitions(URI, reference).unwrap()),
        BTreeSet::from([light, dark])
    );
    assert!(!service.diagnostics(URI).unwrap().iter().any(|diagnostic| {
        matches!(
            diagnostic.code.as_str(),
            "unresolved-reference" | "duplicate-symbol"
        )
    }));
    assert!(service.rename(URI, reference, "Primary").is_err());
    assert!(service.rename(URI, light, "Primary").is_err());
    assert_eq!(service.document(URI).unwrap().source(), source);
    assert_eq!(
        positions(&service, &service.references(URI, light).unwrap()),
        BTreeSet::from([light, reference])
    );
}

#[test]
fn theme_lookup_preserves_local_default_merged_and_ancestor_precedence() {
    for (local, default, merged, expected) in [
        (true, true, true, vec!["Local"]),
        (false, true, true, vec!["LightBrush", "DefaultBrush"]),
        (false, false, true, vec!["LightBrush", "LastMerged"]),
        (false, false, false, vec!["LightBrush", "Outer"]),
    ] {
        let local = if local {
            "<Color x:Key='Accent' Name='Local'/>"
        } else {
            ""
        };
        let default = if default {
            "<ResourceDictionary x:Key=' Default '><Color x:Key='Accent' Name='DefaultBrush'/></ResourceDictionary>"
        } else {
            ""
        };
        let merged = if merged {
            "<ResourceDictionary.MergedDictionaries><ResourceDictionary><Color x:Key='Accent' Name='FirstMerged'/></ResourceDictionary><ResourceDictionary><Color x:Key='Accent' Name='LastMerged'/></ResourceDictionary></ResourceDictionary.MergedDictionaries>"
        } else {
            ""
        };
        let source = format!(
            "<Grid {NS}><Grid.Resources><Color x:Key='Accent' Name='Outer'/></Grid.Resources><Border><Border.Resources><ResourceDictionary>{local}{merged}<ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Light'><Color x:Key='Accent' Name='LightBrush'/></ResourceDictionary>{default}</ResourceDictionary.ThemeDictionaries></ResourceDictionary></Border.Resources><Button Background='{{DynamicResource Accent}}'/></Border></Grid>"
        );
        let service = service(&source);
        let actual = service
            .definitions(URI, source.rfind("Accent").unwrap())
            .unwrap();
        assert_eq!(
            positions(&service, &actual),
            expected
                .iter()
                .map(|name| declaration(&source, name))
                .collect(),
            "Theme alternatives must preserve native resource precedence: {expected:?}"
        );
    }
}

#[test]
fn theme_dictionary_aliases_and_static_default_keys_respect_native_namespace_identity() {
    let source = format!(
        "<Grid {NS} xmlns:r='using:FerroUI.Controls' xmlns:s='using:FerroUI.Styling' xmlns:app='using:Application'><Grid.Resources><r:ResourceDictionary><r:ResourceDictionary.ThemeDictionaries><r:ResourceDictionary x:Key='{{x:Static s:ThemeVariant.Default}}'><Color x:Key='Accent' Name='DefaultBrush'/></r:ResourceDictionary><r:ResourceDictionary x:Key='Dark'/></r:ResourceDictionary.ThemeDictionaries><app:ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Foreign'><Color x:Key='Hidden'/></ResourceDictionary></app:ResourceDictionary.ThemeDictionaries><r:ResourceDictionary.MergedDictionaries><ResourceDictionary><Color x:Key='Accent' Name='HiddenMerge'/></ResourceDictionary></r:ResourceDictionary.MergedDictionaries></r:ResourceDictionary></Grid.Resources><Button Background='{{DynamicResource Accent}}' Tag='{{StaticResource Hidden}}' Content='{{StaticResource Dark}}'/></Grid>"
    );
    let service = service(&source);
    assert_eq!(
        positions(
            &service,
            &service
                .definitions(URI, source.rfind("Accent").unwrap())
                .unwrap()
        ),
        BTreeSet::from([declaration(&source, "DefaultBrush")])
    );
    assert!(service
        .definitions(URI, source.rfind("Hidden").unwrap())
        .unwrap()
        .is_empty());
    assert!(service
        .definitions(URI, source.rfind("Dark").unwrap())
        .unwrap()
        .is_empty());
    let diagnostics = service.diagnostics(URI).unwrap();
    let unresolved: BTreeSet<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "unresolved-reference")
        .map(|diagnostic| &source[diagnostic.span.range()])
        .collect();
    assert_eq!(unresolved, BTreeSet::from(["Hidden", "Dark"]));
}

#[test]
fn imported_theme_resources_keep_private_view_scopes_and_terminate_cycles() {
    let view = format!(
        "<Grid {NS}><Grid.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='A.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Grid.Resources><Button Background='{{DynamicResource Accent}}' Tag='{{StaticResource Private}}'/></Grid>"
    );
    let resources = format!(
        "<ResourceDictionary {NS}><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Light'><Color x:Key='Accent'/></ResourceDictionary><ResourceDictionary x:Key='Default'><Color x:Key='Accent'/></ResourceDictionary></ResourceDictionary.ThemeDictionaries></ResourceDictionary>"
    );
    let private = format!(
        "<Grid {NS}><Grid.Resources><ResourceDictionary><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Default'><Color x:Key='Private'/></ResourceDictionary></ResourceDictionary.ThemeDictionaries></ResourceDictionary></Grid.Resources></Grid>"
    );
    let mut service = service(&view);
    service.open("untitled:/A.xaml", &resources, 1).unwrap();
    service.open("untitled:/Private.xaml", &private, 1).unwrap();
    let definitions = service
        .definitions(URI, view.find("Accent").unwrap())
        .unwrap();
    assert_eq!(definitions.len(), 2);
    assert!(definitions
        .iter()
        .all(|location| location.uri == "untitled:/A.xaml"));
    assert!(service
        .definitions(URI, view.find("Private").unwrap())
        .unwrap()
        .is_empty());
    service.replace("untitled:/A.xaml", format!(
        "<ResourceDictionary {NS}><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Default'><ResourceDictionary.MergedDictionaries><ResourceInclude Source='B.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary></ResourceDictionary.ThemeDictionaries></ResourceDictionary>"
    ), 2).unwrap();
    service.open("untitled:/B.xaml", "<ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='A.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>", 1).unwrap();
    assert!(service
        .definitions(URI, view.find("Accent").unwrap())
        .unwrap()
        .is_empty());
}

#[test]
fn resource_includes_are_theme_alternatives_and_protect_imported_family_renames() {
    let view = format!(
        "<Grid {NS}><Grid.Resources><ResourceDictionary><ResourceDictionary.ThemeDictionaries><ResourceInclude x:Key='Light' Source='Light.xaml'/><ResourceInclude x:Key='Dark' Source='Dark.xaml'/></ResourceDictionary.ThemeDictionaries></ResourceDictionary></Grid.Resources><Button Background='{{DynamicResource Accent}}' Tag='{{StaticResource Light}}'/></Grid>"
    );
    let dictionary =
        format!("<ResourceDictionary {NS}><Color x:Key='Accent'/></ResourceDictionary>");
    let mut service = service(&view);
    service
        .open("untitled:/Light.xaml", &dictionary, 1)
        .unwrap();
    service.open("untitled:/Dark.xaml", &dictionary, 1).unwrap();
    let reference = view.rfind("Accent").unwrap();
    let definitions = service.definitions(URI, reference).unwrap();
    assert_eq!(
        definitions
            .iter()
            .map(|location| location.uri.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["untitled:/Light.xaml", "untitled:/Dark.xaml"])
    );
    assert!(service
        .definitions(URI, view.rfind("Light").unwrap())
        .unwrap()
        .is_empty());
    assert!(service.rename(URI, reference, "Primary").is_err());
    assert!(service
        .rename(
            "untitled:/Light.xaml",
            dictionary.find("Accent").unwrap(),
            "Primary"
        )
        .is_err());
    assert!(service
        .rename(
            "untitled:/Dark.xaml",
            dictionary.find("Accent").unwrap(),
            "Primary"
        )
        .is_err());
    assert_eq!(
        service.document("untitled:/Light.xaml").unwrap().source(),
        dictionary
    );
    assert_eq!(
        service.document("untitled:/Dark.xaml").unwrap().source(),
        dictionary
    );
}

#[test]
fn preview_stage_sample_is_accepted_without_suppressing_theme_resource_diagnostics() {
    let source = include_str!("../../docs/designer/samples/PreviewStages.xaml");
    let service = service(source);
    let diagnostics = service.diagnostics(URI).unwrap();
    assert!(
        diagnostics.iter().all(|diagnostic| diagnostic.severity > 2),
        "{diagnostics:?}"
    );
}
