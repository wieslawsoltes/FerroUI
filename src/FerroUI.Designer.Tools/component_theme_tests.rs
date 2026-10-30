use super::*;

fn document(body: &str) -> Document {
    Document::new(format!(
        r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">{body}</Grid>"#,
    ))
    .unwrap()
}
fn named(document: &Document, name: &str) -> NodeId {
    document.ast().find_name(name).unwrap()
}
fn context(theme: &str, inherited: &[&str]) -> ComponentResourceContext {
    ComponentResourceContext {
        theme: Some(theme.into()),
        inherited_themes: inherited.iter().map(|value| (*value).into()).collect(),
    }
}

#[test]
fn explicit_theme_lookup_follows_runtime_local_selected_inherited_default_merge_order() {
    let d = document(
        r#"<Grid.Resources><Color x:Key="Ancestor" Name="Outer">Gray</Color></Grid.Resources>
<Border Name="Scope"><Border.Resources><ResourceDictionary>
<Color x:Key="Local" Name="Local">Black</Color>
<ResourceDictionary.ThemeDictionaries>
<ResourceDictionary x:Key="Custom"><Color x:Key="Local">Wrong</Color><Color x:Key="Selected" Name="Selected">Red</Color></ResourceDictionary>
<ResourceDictionary x:Key="Dark"><Color x:Key="Selected">Wrong</Color><Color x:Key="Inherited" Name="Inherited">Green</Color></ResourceDictionary>
<ResourceDictionary x:Key="Default"><Color x:Key="Selected">Wrong</Color><Color x:Key="Inherited">Wrong</Color><Color x:Key="Fallback" Name="Fallback">Blue</Color></ResourceDictionary>
</ResourceDictionary.ThemeDictionaries>
<ResourceDictionary.MergedDictionaries>
<ResourceDictionary><Color x:Key="Merged" Name="FirstMerge">Red</Color></ResourceDictionary>
<ResourceDictionary><Color x:Key="Fallback">Wrong</Color><Color x:Key="Merged" Name="LastMerge">Green</Color></ResourceDictionary>
</ResourceDictionary.MergedDictionaries>
</ResourceDictionary></Border.Resources><Button Name="Target"/></Border>"#,
    );
    let target = named(&d, "Target");
    let theme = context("Custom", &["Dark"]);
    for (key, expected) in [
        ("Local", "Local"),
        ("Selected", "Selected"),
        ("Inherited", "Inherited"),
        ("Fallback", "Fallback"),
        ("Merged", "LastMerge"),
        ("Ancestor", "Outer"),
    ] {
        assert_eq!(
            resolve_component_resource_in_context(&d, target, key, &theme),
            Some(named(&d, expected)),
            "key {key}"
        );
    }
    assert_eq!(
        resolve_component_resource_in_context(&d, target, "Inherited", &context("Custom", &[])),
        Some(
            d.ast()
                .elements()
                .find(|node| literal_key(node).as_deref() == Some("Inherited")
                    && node.xaml_name().is_none())
                .unwrap()
                .id
        )
    );
}

#[test]
fn unknown_theme_reports_all_candidates_and_does_not_guess_a_default_or_ancestor() {
    let d = document(
        r#"<Grid.Resources><ResourceDictionary><Color x:Key="Stable" Name="Local">Red</Color>
<ResourceDictionary.ThemeDictionaries>
<ResourceDictionary x:Key="Light"><Color x:Key="Accent" Name="LightAccent">White</Color><Color x:Key="Stable">Wrong</Color></ResourceDictionary>
<ResourceDictionary x:Key="Dark"><Color x:Key="Accent" Name="DarkAccent">Black</Color></ResourceDictionary>
<ResourceDictionary x:Key="Default"><Color x:Key="Accent" Name="DefaultAccent">Gray</Color><Color x:Key="Common" Name="Common">Blue</Color></ResourceDictionary>
</ResourceDictionary.ThemeDictionaries></ResourceDictionary></Grid.Resources><Button Name="Target"/>"#,
    );
    let target = named(&d, "Target");
    let resolution = component_resource_resolution(&d, target, "Accent", None).unwrap();
    assert_eq!(
        resolution.candidates.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([
            named(&d, "LightAccent"),
            named(&d, "DarkAccent"),
            named(&d, "DefaultAccent"),
        ])
    );
    assert!(!resolution.may_be_missing);
    assert!(!resolution.external);
    assert_eq!(resolve_component_resource(&d, target, "Accent"), None);
    assert_eq!(
        resolve_component_resource(&d, target, "Stable"),
        Some(named(&d, "Local"))
    );
    assert_eq!(
        resolve_component_resource(&d, target, "Common"),
        Some(named(&d, "Common"))
    );
    assert_eq!(
        resolve_component_resource_in_context(
            &d,
            target,
            "Accent",
            &ComponentResourceContext::default()
        ),
        Some(named(&d, "DefaultAccent"))
    );
    assert_eq!(
        resolve_component_resource_in_context(&d, target, "Accent", &context("Light", &[])),
        Some(named(&d, "LightAccent"))
    );
}

#[test]
fn themed_resource_usages_protect_rename_and_deletion_even_when_no_unique_target_exists() {
    let d = document(
        r#"<Grid.Resources><ResourceDictionary><ResourceDictionary.ThemeDictionaries>
<ResourceDictionary x:Key="Light"><Color x:Key="Accent" Name="LightAccent">White</Color></ResourceDictionary>
<ResourceDictionary x:Key="Dark"><Color x:Key="Accent" Name="DarkAccent">Black</Color></ResourceDictionary>
</ResourceDictionary.ThemeDictionaries></ResourceDictionary></Grid.Resources><Button Name="Target" Background="{DynamicResource Accent}"/>"#,
    );
    let source = d.source().to_owned();
    for name in ["LightAccent", "DarkAccent"] {
        let resource = named(&d, name);
        let references = component_resource_references(&d, resource).unwrap();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].node, named(&d, "Target"));
        assert!(references[0].dynamic);
        assert_eq!(
            ComponentCatalog::from_document(&d)
                .unwrap()
                .entries
                .into_iter()
                .find(|entry| entry.node == resource)
                .unwrap()
                .usages,
            1
        );
        assert!(plan_component_action(&d, &ComponentAction::RemoveResource { resource }).is_err());
        let error = plan_component_action(
            &d,
            &ComponentAction::RenameResource {
                resource,
                key: "Primary".into(),
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("theme variants"), "{error}");
    }
    assert_eq!(d.source(), source);
    let result = component_resource_resolution(&d, named(&d, "Target"), "Accent", None).unwrap();
    assert!(result.may_be_missing);
}

#[test]
fn local_overrides_do_not_report_unreachable_theme_resources_as_used() {
    let mut d = document(
        r#"<Grid.Resources><ResourceDictionary><Color x:Key="Accent" Name="LocalAccent">Red</Color>
<ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key="Light"><Color x:Key="Accent" Name="HiddenAccent">White</Color></ResourceDictionary></ResourceDictionary.ThemeDictionaries>
</ResourceDictionary></Grid.Resources><Button Name="Target" Background="{DynamicResource Accent}"/>"#,
    );
    let hidden = named(&d, "HiddenAccent");
    assert!(
        component_resource_references(&d, hidden)
            .unwrap()
            .is_empty()
    );
    let local = named(&d, "LocalAccent");
    let edits = plan_component_action(
        &d,
        &ComponentAction::RenameResource {
            resource: local,
            key: "Primary".into(),
        },
    )
    .unwrap();
    d.apply("Rename overriding resource", edits).unwrap();
    assert_eq!(component_resource_references(&d, local).unwrap().len(), 1);
    assert!(
        component_resource_references(&d, hidden)
            .unwrap()
            .is_empty()
    );
    let edits =
        plan_component_action(&d, &ComponentAction::RemoveResource { resource: hidden }).unwrap();
    d.apply("Remove unused resource", edits).unwrap();
    assert!(d.ast().node(hidden).is_none());
}

#[test]
fn external_theme_and_merged_providers_keep_possible_fallback_resources_protected() {
    for provider in [
        r#"<ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key="Light" Source="External.xaml"/></ResourceDictionary.ThemeDictionaries>"#,
        r#"<ResourceDictionary.MergedDictionaries><ResourceInclude Source="External.xaml"/></ResourceDictionary.MergedDictionaries>"#,
    ] {
        let d = document(&format!(
            r#"<Grid.Resources><Color x:Key="Accent" Name="Fallback">Red</Color></Grid.Resources><Border><Border.Resources><ResourceDictionary>{provider}</ResourceDictionary></Border.Resources><Button Name="Target" Background="{{StaticResource Accent}}"/></Border>"#,
        ));
        let target = named(&d, "Target");
        let fallback = named(&d, "Fallback");
        let resolution = component_resource_resolution(&d, target, "Accent", None).unwrap();
        assert!(resolution.external);
        assert_eq!(resolution.candidates, [fallback]);
        assert_eq!(resolution.resolved(), None);
        assert_eq!(
            component_resource_references(&d, fallback).unwrap().len(),
            1
        );
        assert!(
            plan_component_action(&d, &ComponentAction::RemoveResource { resource: fallback })
                .is_err()
        );
        assert!(
            plan_component_action(
                &d,
                &ComponentAction::RenameResource {
                    resource: fallback,
                    key: "New".into()
                }
            )
            .is_err()
        );
    }
}

#[test]
fn qualified_theme_members_and_builtin_static_keys_retain_framework_identity() {
    let d = document(
        r#"<Grid.Resources><ResourceDictionary xmlns:r="clr-namespace:FerroUI.Controls;assembly=FerroUI.Base" xmlns:s="using:FerroUI.Styling" xmlns:app="urn:application">
<r:ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key="{x:Static s:ThemeVariant.Light}"><Color x:Key="Accent" Name="Light">White</Color></ResourceDictionary><ResourceDictionary x:Key="Default"><Color x:Key="Accent" Name="Default">Gray</Color></ResourceDictionary></r:ResourceDictionary.ThemeDictionaries>
<app:ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key="Light"><Color x:Key="Accent" Name="Foreign">Wrong</Color></ResourceDictionary></app:ResourceDictionary.ThemeDictionaries>
</ResourceDictionary></Grid.Resources><Button Name="Target"/>"#,
    );
    let target = named(&d, "Target");
    assert_eq!(
        resolve_component_resource_in_context(&d, target, "Accent", &context("Light", &[])),
        Some(named(&d, "Light"))
    );
    let result = component_resource_resolution(&d, target, "Accent", None).unwrap();
    assert_eq!(result.candidates.len(), 2);
    assert!(!result.candidates.contains(&named(&d, "Foreign")));
    assert!(!result.external);
}

#[test]
fn nested_theme_inheritance_is_included_in_conservative_candidates() {
    let d = document(
        r#"<Grid.Resources><ResourceDictionary><ResourceDictionary.ThemeDictionaries>
<ResourceDictionary x:Key="Custom"><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key="Dark"><Color x:Key="Accent" Name="Nested">Black</Color></ResourceDictionary></ResourceDictionary.ThemeDictionaries></ResourceDictionary>
<ResourceDictionary x:Key="Default"><Color x:Key="Accent" Name="Fallback">Gray</Color></ResourceDictionary>
</ResourceDictionary.ThemeDictionaries></ResourceDictionary></Grid.Resources><Button Name="Target" Background="{DynamicResource Accent}"/>"#,
    );
    let target = named(&d, "Target");
    assert_eq!(
        resolve_component_resource_in_context(&d, target, "Accent", &context("Custom", &["Dark"])),
        Some(named(&d, "Nested"))
    );
    assert_eq!(
        resolve_component_resource_in_context(&d, target, "Accent", &context("Custom", &[])),
        Some(named(&d, "Fallback"))
    );
    let result = component_resource_resolution(&d, target, "Accent", None).unwrap();
    assert_eq!(
        result.candidates.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([named(&d, "Nested"), named(&d, "Fallback")])
    );
    assert_eq!(
        component_resource_references(&d, named(&d, "Nested"))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn unsupported_theme_keys_and_invalid_inheritance_do_not_produce_false_certainty() {
    let d = document(
        r#"<Grid.Resources><ResourceDictionary><ResourceDictionary.ThemeDictionaries><ResourceDictionary xmlns:app="urn:application" x:Key="{app:CurrentTheme}"><Color x:Key="Accent" Name="Possible">Blue</Color></ResourceDictionary></ResourceDictionary.ThemeDictionaries></ResourceDictionary></Grid.Resources><Button Name="Target"/>"#,
    );
    let target = named(&d, "Target");
    let resolution =
        component_resource_resolution(&d, target, "Accent", Some(&context("Light", &[]))).unwrap();
    assert!(resolution.external);
    assert_eq!(resolution.candidates, [named(&d, "Possible")]);
    assert_eq!(resolution.resolved(), None);
    for invalid in [
        context("Light", &["Light"]),
        context("Light", &["Default"]),
        context("Default", &["Dark"]),
    ] {
        assert!(component_resource_resolution(&d, target, "Accent", Some(&invalid)).is_err());
    }
}

#[test]
fn active_theme_picker_and_apply_plan_share_visibility_without_weakening_structural_guards() {
    let mut d = document(
        r#"<Grid.Resources><ResourceDictionary><ResourceDictionary.ThemeDictionaries>
<ResourceDictionary x:Key="Light"><Color x:Key="Accent" Name="LightAccent">White</Color></ResourceDictionary>
<ResourceDictionary x:Key="Dark"><Color x:Key="Accent" Name="DarkAccent">Black</Color></ResourceDictionary>
</ResourceDictionary.ThemeDictionaries></ResourceDictionary></Grid.Resources><Button Name="Target"/>"#,
    );
    let target = named(&d, "Target");
    let light_resource = named(&d, "LightAccent");
    let light = context("Light", &[]);
    let dark = context("Dark", &[]);
    assert!(visible_component_resources(&d, target).unwrap().is_empty());
    let visible = visible_component_resources_in_context(&d, target, &light).unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].node, light_resource);
    let apply = ComponentAction::ApplyResource {
        resource: light_resource,
        targets: vec![target],
        property: "Background".into(),
        dynamic: true,
    };
    assert!(plan_component_action(&d, &apply).is_err());
    assert!(plan_component_action_in_context(&d, &apply, &dark).is_err());
    let original = d.source().to_owned();
    let edits = plan_component_action_in_context(&d, &apply, &light).unwrap();
    d.apply("Apply active theme resource", edits).unwrap();
    assert_eq!(
        d.ast().node(target).unwrap().attribute("Background"),
        Some("{DynamicResource 'Accent'}")
    );
    assert!(
        plan_component_action_in_context(
            &d,
            &ComponentAction::RenameResource {
                resource: light_resource,
                key: "New".into()
            },
            &light
        )
        .is_err()
    );
    assert!(
        plan_component_action_in_context(
            &d,
            &ComponentAction::RemoveResource {
                resource: light_resource
            },
            &light
        )
        .is_err()
    );
    d.undo().unwrap();
    assert_eq!(d.source(), original);
}

#[test]
fn resource_deletion_does_not_treat_containers_or_implicit_type_keys_as_unused_entries() {
    let d = document(
        r#"<Grid.Resources><ResourceDictionary Name="Container"><ControlTheme x:Key="{x:Type Button}" Name="Implicit" TargetType="Button"/><Color x:Key="Accent" Name="Accent">Red</Color></ResourceDictionary></Grid.Resources><Button Background="{StaticResource Accent}"/>"#,
    );
    for name in ["Container", "Implicit"] {
        assert!(
            plan_component_action(
                &d,
                &ComponentAction::RemoveResource {
                    resource: named(&d, name)
                }
            )
            .is_err()
        );
    }
    assert_eq!(
        component_resource_references(&d, named(&d, "Accent"))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn duplicate_theme_containers_are_ambiguous_instead_of_hiding_their_possible_references() {
    let d = document(
        r#"<Grid.Resources><ResourceDictionary>
<ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key="Light"><Color x:Key="Accent" Name="First">White</Color></ResourceDictionary></ResourceDictionary.ThemeDictionaries>
<ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key="Light"><Color x:Key="Accent" Name="Second">Blue</Color></ResourceDictionary></ResourceDictionary.ThemeDictionaries>
</ResourceDictionary></Grid.Resources><Button Name="Target" Background="{DynamicResource Accent}"/>"#,
    );
    let result = component_resource_resolution(
        &d,
        named(&d, "Target"),
        "Accent",
        Some(&context("Light", &[])),
    )
    .unwrap();
    assert!(result.external);
    assert_eq!(result.candidates.len(), 2);
    assert_eq!(result.resolved(), None);
    for name in ["First", "Second"] {
        assert_eq!(
            component_resource_references(&d, named(&d, name))
                .unwrap()
                .len(),
            1
        );
        assert!(
            plan_component_action(
                &d,
                &ComponentAction::RemoveResource {
                    resource: named(&d, name)
                }
            )
            .is_err()
        );
    }
}
