use ferroui_base::metadata::FERRO_XML_NAMESPACE;
use crate::*;
use roxmltree::{Document, Node};

const XAML_NAMESPACE: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
const CONTROLS_NAMESPACE: &str = "using:FerroUI.Controls";
const PRIMITIVES_NAMESPACE: &str = "using:FerroUI.Controls.Primitives";
const NAVIGATION_NAMESPACE: &str = "using:FerroUI.Navigation";

fn source(body: &str) -> String {
    format!("<Grid xmlns='{WINUI_PRESENTATION_NAMESPACE}' xmlns:x='{XAML_NAMESPACE}'>{body}</Grid>")
}

fn migrate(body: &str) -> WinUIXamlMigration {
    migrate_winui_xaml(&source(body), &WinUIXamlOptions::default()).unwrap()
}

#[test]
fn namespaces_change_only_declarations_not_comments_literals_cdata_or_text() {
    let body = format!("<!-- {WINUI_PRESENTATION_NAMESPACE} --><TextBlock Text='{WINUI_PRESENTATION_NAMESPACE}'/><TextBlock><![CDATA[{WINUI_PRESENTATION_NAMESPACE}]]></TextBlock>");
    let original = source(&body);
    let result = migrate_winui_xaml(&original, &WinUIXamlOptions::default()).unwrap();
    assert!(!result.has_errors());
    assert_eq!(result.xaml.matches(FERRO_XML_NAMESPACE).count(), 1);
    assert_eq!(result.xaml.matches(WINUI_PRESENTATION_NAMESPACE).count(), 3);
    assert_eq!(result.edits.len(), 1);
    assert_eq!(
        &original[result.edits[0].span.bytes.clone()],
        WINUI_PRESENTATION_NAMESPACE
    );
}

#[test]
fn native_documents_are_identity_and_migration_is_idempotent() {
    let original = "<TextBox xmlns='https://github.com/ferroui' Text='{Binding Name}'/>";
    let result = migrate_winui_xaml(original, &WinUIXamlOptions::default()).unwrap();
    assert_eq!(result.xaml, original);
    assert!(result.edits.is_empty());
    let once = migrate("<ToggleSwitch IsOn='True'/><Button Visibility='Collapsed'/>")
        .into_xaml()
        .unwrap();
    let twice = migrate_winui_xaml(&once, &WinUIXamlOptions::default()).unwrap();
    assert_eq!(twice.xaml, once);
    assert!(twice.edits.is_empty());
}

#[test]
fn source_page_maps_to_native_frame_page_with_balanced_property_elements() {
    let result = migrate("<Page><Page.Content><Frame/></Page.Content></Page>");
    assert!(!result.has_errors());
    let document = Document::parse(&result.xaml).unwrap();
    let elements: Vec<_> = document.descendants().filter(Node::is_element).collect();
    for element in &elements[1..] {
        assert_eq!(element.tag_name().namespace(), Some(CONTROLS_NAMESPACE));
    }
    assert_eq!(elements[1].tag_name().name(), "FramePage");
    assert_eq!(elements[2].tag_name().name(), "FramePage.Content");
    assert_eq!(elements[3].tag_name().name(), "Frame");
}

#[test]
fn inline_color_picker_maps_to_color_view_and_preserves_property_element_owners() {
    let input = source("<ColorPicker Color='Red' IsColorSliderVisible='False' IsColorChannelTextInputVisible='True'><ColorPicker.ColorSpectrumShape>Ring</ColorPicker.ColorSpectrumShape></ColorPicker><Grid Tag='{x:Type ColorPicker}'/>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let document = Document::parse(&result.xaml).unwrap();
    let picker = document.descendants().find(|node| node.has_tag_name((CONTROLS_NAMESPACE, "ColorView"))).unwrap();
    assert_eq!(picker.attribute("Color"), Some("Red"));
    assert_eq!(picker.attribute("IsColorSpectrumSliderVisible"), Some("False"));
    assert_eq!(picker.attribute("IsComponentTextInputVisible"), Some("True"));
    assert!(picker.children().any(|node| node.has_tag_name((CONTROLS_NAMESPACE, "ColorView.ColorSpectrumShape"))));
    assert!(result.xaml.contains(":ColorView}"));
    let notice = result.diagnostics.iter().find(|diagnostic| diagnostic.code == "WXAML2006").unwrap();
    assert_eq!(&input[notice.span.bytes.clone()], "ColorPicker");
    let repeated = migrate_winui_xaml(&result.xaml, &WinUIXamlOptions::default()).unwrap();
    assert_eq!(repeated.xaml, result.xaml);
    assert!(repeated.edits.is_empty() && repeated.diagnostics.is_empty());
}

#[test]
fn color_picker_member_renames_apply_to_property_elements_and_explicit_overrides() {
    for (prefix, declaration) in [
        ("", ""),
        ("w:", " xmlns:w='using:Microsoft.UI.Xaml.Controls'"),
    ] {
        let input = source(&format!("<{prefix}ColorPicker{declaration}><{prefix}ColorPicker.IsColorSliderVisible>False</{prefix}ColorPicker.IsColorSliderVisible><{prefix}ColorPicker.IsColorChannelTextInputVisible>True</{prefix}ColorPicker.IsColorChannelTextInputVisible></{prefix}ColorPicker>"));
        let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        let document = Document::parse(&result.xaml).unwrap();
        for (member, text) in [
            ("ColorView.IsColorSpectrumSliderVisible", "False"),
            ("ColorView.IsComponentTextInputVisible", "True"),
        ] {
            let element = document.descendants().find(|node| node.has_tag_name((CONTROLS_NAMESPACE, member))).unwrap();
            assert_eq!(element.text(), Some(text));
        }
        let mut options = WinUIXamlOptions::default();
        options.members.push(WinUIXamlMemberMapping::new(
            if prefix.is_empty() { WINUI_PRESENTATION_NAMESPACE } else { "using:Microsoft.UI.Xaml.Controls" },
            "ColorPicker", "IsColorSliderVisible", "ShowSourceSlider",
        ));
        let mapped = migrate_winui_xaml(&input, &options).unwrap();
        assert!(!mapped.has_errors());
        let document = Document::parse(&mapped.xaml).unwrap();
        assert!(document.descendants().any(|node| node.has_tag_name((CONTROLS_NAMESPACE, "ColorView.ShowSourceSlider"))));
        assert!(!document.descendants().any(|node| node.has_tag_name((CONTROLS_NAMESPACE, "ColorView.IsColorSpectrumSliderVisible"))));
    }
}

#[test]
fn native_hosts_resolve_source_type_references_without_importing_binding_or_extension_semantics() {
    let input = format!(r#"<Grid xmlns="{FERRO_XML_NAMESPACE}" xmlns:w="using:Microsoft.UI.Xaml.Controls" xmlns:x="{XAML_NAMESPACE}" xmlns:app="using:App.Markup">
        <ControlTemplate TargetType="w:ColorPicker" />
        <TextBlock x:DataType="w:ColorPicker" />
        <Grid Tag="{{x:Type w:ColorPicker}}" />
        <TextBox Text="{{Binding Name}}" Tag="{{Binding Item, ConverterParameter={{x:Type w:ColorPicker}}}}" />
        <Grid Tag="{{app:Resolve Value={{x:Type w:ColorPicker}}, Literal='{{x:Type w:ColorPicker}}'}}" />
        <Grid Tag="{{app:Type w:ColorPicker}}" />
        <Grid Tag="{{}}{{x:Type w:ColorPicker}}" />
        <Grid xmlns:w="using:Other.Controls" Tag="{{x:Type w:ColorPicker}}" />
        <Binding Path="Name" />
        <TextBox Text="{{Binding Name, Mode=Default}}" />
    </Grid>"#);
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let document = Document::parse(&result.xaml).unwrap();
    let children: Vec<_> = document.root_element().children().filter(Node::is_element).collect();
    let target = children[0].attribute("TargetType").unwrap();
    let (prefix, name) = target.split_once(':').unwrap();
    assert_eq!(name, "ColorView");
    assert_eq!(children[0].lookup_namespace_uri(Some(prefix)), Some(CONTROLS_NAMESPACE));
    assert_eq!(children[1].attribute((XAML_NAMESPACE, "DataType")), Some(target));
    assert_eq!(children[2].attribute("Tag"), Some(format!("{{x:Type {target}}}").as_str()));
    assert_eq!(children[3].attribute("Text"), Some("{Binding Name}"));
    assert_eq!(children[3].attribute("Tag"), Some(format!("{{Binding Item, ConverterParameter={{x:Type {target}}}}}").as_str()));
    assert_eq!(children[4].attribute("Tag"), Some(format!("{{app:Resolve Value={{x:Type {target}}}, Literal='{{x:Type w:ColorPicker}}'}}").as_str()));
    assert_eq!(children[5].attribute("Tag"), Some("{app:Type w:ColorPicker}"));
    assert_eq!(children[6].attribute("Tag"), Some("{}{x:Type w:ColorPicker}"));
    assert_eq!(children[7].attribute("Tag"), Some("{x:Type w:ColorPicker}"));
    assert_eq!(children[7].lookup_namespace_uri(Some("w")), Some("using:Other.Controls"));
    assert_eq!(children[8].attribute("Mode"), None);
    assert_eq!(children[9].attribute("Text"), Some("{Binding Name, Mode=Default}"));
    let repeated = migrate_winui_xaml(&result.xaml, &WinUIXamlOptions::default()).unwrap();
    assert_eq!(repeated.xaml, result.xaml);
    assert!(repeated.edits.is_empty() && repeated.diagnostics.is_empty());
}

#[test]
fn native_host_type_references_obey_explicit_namespace_and_type_policies() {
    let input = format!("<ControlTemplate xmlns='{FERRO_XML_NAMESPACE}' xmlns:w='using:Microsoft.UI.Xaml.Controls' xmlns:x='{XAML_NAMESPACE}' TargetType='w:ColorPicker'><Grid Tag='{{x:Type w:ColorPicker}}'/></ControlTemplate>");
    for override_type in [false, true] {
        let mut options = WinUIXamlOptions::default();
        options.namespaces.push(WinUIXamlNamespaceMapping::new(
            "using:Microsoft.UI.Xaml.Controls", "using:App.Controls",
        ));
        if override_type {
            options.types.push(WinUIXamlTypeMapping::new(
                "using:Microsoft.UI.Xaml.Controls", "ColorPicker", "using:App.Palettes", "Palette",
            ));
        }
        let result = migrate_winui_xaml(&input, &options).unwrap();
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        let document = Document::parse(&result.xaml).unwrap();
        let node = document.root_element();
        let target = node.attribute("TargetType").unwrap();
        let (prefix, name) = target.split_once(':').unwrap();
        assert_eq!(name, if override_type { "Palette" } else { "ColorPicker" });
        assert_eq!(node.lookup_namespace_uri(Some(prefix)), Some(if override_type { "using:App.Palettes" } else { "using:App.Controls" }));
        assert_eq!(node.first_element_child().unwrap().attribute("Tag"), Some(format!("{{x:Type {target}}}").as_str()));
        assert!(!result.xaml.contains("ColorView"));
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn type_reference_scans_reject_excessive_nested_extensions_in_native_hosts() {
    let mut value = "{x:Type w:ColorPicker}".to_string();
    for _ in 0..80 {
        value = format!("{{Binding Item, ConverterParameter={value}}}");
    }
    let input = format!("<Grid xmlns='{FERRO_XML_NAMESPACE}' xmlns:w='using:Microsoft.UI.Xaml.Controls' xmlns:x='{XAML_NAMESPACE}' Tag='{value}'/>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert!(result.has_errors());
    assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "WXAML0002"));
    assert!(result.into_xaml().is_err());
}

#[test]
fn color_picker_alpha_and_editor_contracts_require_explicit_adaptation() {
    for member in ["IsAlphaEnabled", "IsMoreButtonVisible", "IsAlphaSliderVisible", "IsAlphaTextInputVisible"] {
        for body in [
            format!("<ColorPicker {member}='False'/>"),
            format!("<ColorPicker><ColorPicker.{member}>False</ColorPicker.{member}></ColorPicker>"),
            format!("<c:ColorPicker xmlns:c='using:Microsoft.UI.Xaml.Controls' {member}='True'/>"),
        ] {
            let input = source(&body);
            let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
            let errors: Vec<_> = result.diagnostics.iter().filter(|diagnostic| diagnostic.code == "WXAML1012").collect();
            assert_eq!(errors.len(), 1, "{body}: {:?}", result.diagnostics);
            assert!(result.has_errors());
            assert!(input[errors[0].span.bytes.clone()].contains(member));
            assert!(result.xaml.contains(member), "Unsupported input must remain visible");
        }
    }
}

#[test]
fn color_picker_explicit_host_policies_take_precedence_over_inline_adaptation() {
    let input = "<c:ColorPicker xmlns:c='using:Microsoft.UI.Xaml.Controls' IsMoreButtonVisible='True' IsColorSliderVisible='False'/>";
    let mut options = WinUIXamlOptions::default();
    options.types.push(WinUIXamlTypeMapping::new("using:Microsoft.UI.Xaml.Controls", "ColorPicker", "using:App.Controls", "Palette"));
    let result = migrate_winui_xaml(input, &options).unwrap();
    assert!(!result.has_errors());
    assert!(result.xaml.contains(":Palette"));
    assert!(result.xaml.contains("IsColorSliderVisible='False'"));
    assert!(result.diagnostics.is_empty());

    let mut options = WinUIXamlOptions::default();
    options.namespaces.push(WinUIXamlNamespaceMapping::new("using:Microsoft.UI.Xaml.Controls", "using:App.Controls"));
    let result = migrate_winui_xaml(input, &options).unwrap();
    assert!(!result.has_errors());
    assert!(result.xaml.contains("using:App.Controls"));
    assert!(!result.xaml.contains("ColorView"));

    let mut options = WinUIXamlOptions::default();
    options.members.push(WinUIXamlMemberMapping::new("using:Microsoft.UI.Xaml.Controls", "ColorPicker", "IsAlphaEnabled", "IsAlphaEnabled"));
    let result = migrate_winui_xaml("<c:ColorPicker xmlns:c='using:Microsoft.UI.Xaml.Controls' IsAlphaEnabled='False'/>", &options).unwrap();
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "WXAML2006"));
}

#[test]
fn native_color_picker_and_color_view_keep_their_existing_meanings() {
    for name in ["ColorPicker", "ColorView"] {
        let input = format!("<{name} xmlns='{FERRO_XML_NAMESPACE}' IsAlphaEnabled='False'/>");
        let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
        assert_eq!(result.xaml, input);
        assert!(result.edits.is_empty() && result.diagnostics.is_empty());
    }
}

#[test]
fn implemented_controls_use_the_native_controls_namespace() {
    let result = migrate(
        "<NumberBox Value='12'/><RadioButtons/><FontIcon Glyph='A'/><SymbolIcon Symbol='Accept'/><InfoBar/><FontIconSource/><SymbolIconSource/><PathIconSource/><AutoSuggestBox/>",
    );
    assert!(!result.has_errors());
    let doc = Document::parse(&result.xaml).unwrap();
    for element in doc.descendants().filter(Node::is_element).skip(1) {
        assert_eq!(element.tag_name().namespace(), Some(CONTROLS_NAMESPACE));
    }
}

#[test]
fn feedback_controls_and_item_descriptions_migrate_without_global_type_replacement() {
    let result = migrate("<InfoBadge Value='7'><InfoBadge.IconSource><FontIconSource Glyph='A'/></InfoBadge.IconSource></InfoBadge><ProgressRing IsIndeterminate='False' Value='42'/><RatingControl PlaceholderValue='2.5'><RatingControl.ItemInfo><RatingItemFontInfo Glyph='B' UnsetGlyph='C'/></RatingControl.ItemInfo></RatingControl><RatingItemImageInfo/><RatingItemInfo/><InfoBadgeTemplateSettings/><ProgressRingTemplateSettings/>");
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let document = Document::parse(&result.xaml).unwrap();
    for element in document.descendants().filter(Node::is_element).skip(1) {
        let expected = match element.tag_name().name() {
            "InfoBadgeTemplateSettings" | "ProgressRingTemplateSettings" => PRIMITIVES_NAMESPACE,
            _ => CONTROLS_NAMESPACE,
        };
        assert_eq!(element.tag_name().namespace(), Some(expected));
    }
    assert_eq!(migrate_winui_xaml(&result.xaml, &WinUIXamlOptions::default()).unwrap().xaml, result.xaml);
}

#[test]
fn frame_navigation_and_primitive_type_references_use_their_native_namespaces() {
    for (source_namespace, name, target_namespace) in [
        ("using:Microsoft.UI.Xaml.Navigation", "NavigationEventArgs", NAVIGATION_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Navigation", "NavigatingCancelEventArgs", NAVIGATION_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Navigation", "NavigationFailedEventArgs", NAVIGATION_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Navigation", "NavigationMode", NAVIGATION_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Navigation", "NavigationCacheMode", NAVIGATION_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Navigation", "FrameNavigationOptions", NAVIGATION_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Navigation", "PageStackEntry", NAVIGATION_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Controls.Primitives", "InfoBarPanel", PRIMITIVES_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Controls.Primitives", "InfoBarTemplateSettings", PRIMITIVES_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Controls.Primitives", "InfoBadgeTemplateSettings", PRIMITIVES_NAMESPACE),
        ("using:Microsoft.UI.Xaml.Controls.Primitives", "ProgressRingTemplateSettings", PRIMITIVES_NAMESPACE),
    ] {
        let result = migrate(&format!("<Grid xmlns:t='{source_namespace}' Tag='{{x:Type t:{name}}}'/>"));
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        let document = Document::parse(&result.xaml).unwrap();
        let node = document.root_element().first_element_child().unwrap();
        let literal = node.attribute("Tag").unwrap().strip_prefix("{x:Type ").unwrap().strip_suffix('}').unwrap();
        let (prefix, local) = literal.split_once(':').unwrap();
        assert_eq!(local, name);
        assert_eq!(node.lookup_namespace_uri(Some(prefix)), Some(target_namespace));
    }
}

#[test]
fn navigation_view_item_collections_and_badges_keep_their_property_element_owners() {
    let result = migrate("<NavigationView PaneDisplayMode='Left'><NavigationView.MenuItems><NavigationViewItem Content='Inbox'><NavigationViewItem.InfoBadge><InfoBadge Value='5'/></NavigationViewItem.InfoBadge><NavigationViewItem.MenuItems><NavigationViewItemHeader Content='Folders'/><NavigationViewItemSeparator/><NavigationViewItem Content='Archive'/></NavigationViewItem.MenuItems></NavigationViewItem></NavigationView.MenuItems><NavigationView.FooterMenuItems><NavigationViewItem Content='Help'/></NavigationView.FooterMenuItems><Frame/></NavigationView>");
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let document = Document::parse(&result.xaml).unwrap();
    for node in document.descendants().filter(Node::is_element).skip(1) {
        assert_eq!(node.tag_name().namespace(), Some(CONTROLS_NAMESPACE));
    }
    for member in ["NavigationView.MenuItems", "NavigationView.FooterMenuItems", "NavigationViewItem.MenuItems", "NavigationViewItem.InfoBadge"] {
        assert!(document.descendants().any(|node| node.has_tag_name((CONTROLS_NAMESPACE, member))));
    }
    assert_eq!(migrate_winui_xaml(&result.xaml, &WinUIXamlOptions::default()).unwrap().xaml, result.xaml);
    for name in [
        "AutoSuggestBoxAutomationPeer", "InfoBarAutomationPeer",
        "NavigationViewAutomationPeer", "NavigationViewItemAutomationPeer",
        "NumberBoxAutomationPeer", "ProgressRingAutomationPeer",
        "RadioButtonsAutomationPeer", "RatingControlAutomationPeer",
    ] {
        let result = migrate(&format!("<Grid xmlns:a='using:Microsoft.UI.Xaml.Automation.Peers' Tag='{{x:Type a:{name}}}'/>"));
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        let document = Document::parse(&result.xaml).unwrap();
        let node = document.root_element().first_element_child().unwrap();
        let literal = node.attribute("Tag").unwrap().strip_prefix("{x:Type ").unwrap().strip_suffix('}').unwrap();
        let (prefix, local) = literal.split_once(':').unwrap();
        assert_eq!(local, name);
        assert_eq!(node.lookup_namespace_uri(Some(prefix)), Some("using:FerroUI.Automation.Peers"));
    }

}

#[test]
fn symbol_icon_literals_expand_for_empty_and_content_elements_without_touching_original_text() {
    let input = source("<!-- 海 Icon='Home' café --><NavigationViewItem Icon = 'Home' Content='A &amp; B — café'/><NavigationViewItem Icon='Mail' Tag='keep&gt;value'><NavigationViewItem.InfoBadge><InfoBadge Value='5'/></NavigationViewItem.InfoBadge>Inbox</NavigationViewItem><AutoSuggestBox QueryIcon='Find'/>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert!(result.xaml.contains("<!-- 海 Icon='Home' café -->"));
    assert!(result.xaml.contains("Content='A &amp; B — café'"));
    assert!(result.xaml.contains("Tag='keep&gt;value'"));
    let document = Document::parse(&result.xaml).unwrap();
    let icons: Vec<_> = document.descendants().filter(|node| node.has_tag_name((CONTROLS_NAMESPACE, "SymbolIcon"))).collect();
    assert_eq!(icons.iter().map(|node| node.attribute("Symbol").unwrap()).collect::<Vec<_>>(), ["Home", "Mail", "Find"]);
    assert_eq!(icons[0].parent_element().unwrap().tag_name().name(), "NavigationViewItem.Icon");
    assert_eq!(icons[2].parent_element().unwrap().tag_name().name(), "AutoSuggestBox.QueryIcon");
    assert_eq!(result.diagnostics.iter().filter(|diagnostic| diagnostic.code == "WXAML2005").count(), 3);
    assert_eq!(result.diagnostics.iter().filter(|diagnostic| diagnostic.code == "WXAML2005")
        .map(|diagnostic| &input[diagnostic.span.bytes.clone()]).collect::<Vec<_>>(), ["Home", "Mail", "Find"]);
    for edit in &result.edits {
        assert!(input.is_char_boundary(edit.span.bytes.start));
        assert!(input.is_char_boundary(edit.span.bytes.end));
    }
    assert!(migrate_winui_xaml(&result.xaml, &WinUIXamlOptions::default()).unwrap().edits.is_empty());
}

#[test]
fn symbolic_icons_respect_explicit_namespace_prefixes_resources_and_custom_type_policies() {
    let input = format!("<c:NavigationViewItem xmlns:c='using:Microsoft.UI.Xaml.Controls' xmlns:x='{XAML_NAMESPACE}' Icon='Play' Content='menu'/>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let document = Document::parse(&result.xaml).unwrap();
    assert_eq!(document.root_element().tag_name().namespace(), Some(CONTROLS_NAMESPACE));
    assert!(document.descendants().any(|node| node.has_tag_name((CONTROLS_NAMESPACE, "SymbolIcon")) && node.attribute("Symbol") == Some("Play")));

    let source = source("<NavigationViewItem Icon='{StaticResource SharedIcon}'/><AutoSuggestBox QueryIcon='{Binding SearchIcon}'/>");
    let result = migrate_winui_xaml(&source, &WinUIXamlOptions::default()).unwrap();
    assert!(result.xaml.contains("Icon='{StaticResource SharedIcon}'"));
    assert!(result.xaml.contains("QueryIcon='{Binding SearchIcon, Mode=OneWay}'"));
    assert!(!result.diagnostics.iter().any(|diagnostic| diagnostic.code == "WXAML2005"));

    let mut options = WinUIXamlOptions::default();
    options.types.push(WinUIXamlTypeMapping::new("using:Microsoft.UI.Xaml.Controls", "NavigationViewItem", "using:My.Controls", "NavigationEntry"));
    let result = migrate_winui_xaml(&input, &options).unwrap();
    assert!(result.xaml.contains("Icon='Play'"));
    assert!(!result.diagnostics.iter().any(|diagnostic| diagnostic.code == "WXAML2005"));

    let mut options = WinUIXamlOptions::default();
    options.types.push(WinUIXamlTypeMapping::new("using:Microsoft.UI.Xaml.Controls", "SymbolIcon", "using:My.Controls", "CustomIcon"));
    options.members.push(WinUIXamlMemberMapping::new("using:Microsoft.UI.Xaml.Controls", "SymbolIcon", "Symbol", "Glyph"));
    let result = migrate_winui_xaml(&input, &options).unwrap();
    let document = Document::parse(&result.xaml).unwrap();
    let custom = document.descendants().find(|node| node.has_tag_name(("using:My.Controls", "CustomIcon"))).unwrap();
    assert_eq!(custom.attribute("Glyph"), Some("Play"));
    assert!(custom.attribute("Symbol").is_none());

    let mut options = WinUIXamlOptions::default();
    options.members.push(WinUIXamlMemberMapping::new("using:Microsoft.UI.Xaml.Controls", "NavigationViewItem", "Icon", "Decoration"));
    let result = migrate_winui_xaml(&input, &options).unwrap();
    assert!(result.xaml.contains("Decoration='Play'"));
    assert!(!result.diagnostics.iter().any(|diagnostic| diagnostic.code == "WXAML2005"));

    let native = "<w:NavigationViewItem xmlns:w='using:FerroUI.Controls' Icon='Mail'/>";
    assert!(migrate_winui_xaml(native, &WinUIXamlOptions::default()).unwrap().edits.is_empty());
}

#[test]
fn info_badge_style_keys_are_audited_only_when_the_winui_theme_policy_is_selected() {
    for severity in ["Attention", "Informational", "Success", "Caution", "Critical"] {
        for kind in ["Dot", "Value", "Icon"] {
            let input = source(&format!("<InfoBadge Style='{{StaticResource {severity}{kind}InfoBadgeStyle}}'/>"));
            assert!(migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap().has_errors());
            let mapped = migrate_winui_xaml(&input, &WinUIXamlOptions::for_winui_theme()).unwrap();
            assert!(!mapped.has_errors(), "{:?}", mapped.diagnostics);
            assert!(mapped.xaml.contains(&format!("Theme='{{StaticResource {severity}{kind}InfoBadgeStyle}}'")));
        }
    }
}

#[test]
fn feedback_control_xaml_constructs_live_native_contracts_and_item_info() {
    use ferroui_base::Ref;
    use ferroui_controls::{FontIconSource, InfoBadge, ProgressRing, RatingControl, RatingItemFontInfo};
    use ferroui_controls::Control;
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    let input = source("<InfoBadge x:Name='Badge' Value='7'><InfoBadge.IconSource><FontIconSource Glyph='A'/></InfoBadge.IconSource></InfoBadge><ProgressRing x:Name='Progress' IsIndeterminate='False' Maximum='200' Value='42'/><RatingControl x:Name='Rating' Caption='Ratings' PlaceholderValue='2.5'><RatingControl.ItemInfo><RatingItemFontInfo Glyph='B' UnsetGlyph='C'/></RatingControl.ItemInfo></RatingControl>");
    let root: Ref<Control> = crate::parse_winui_as(&input, None, &WinUIXamlOptions::default()).unwrap();
    let badge = root.get_control::<InfoBadge>("Badge");
    assert_eq!(badge.value(), 7);
    assert_eq!(badge.icon_source().unwrap().cast::<FontIconSource>().unwrap().glyph(), "A");
    let ring = root.get_control::<ProgressRing>("Progress");
    assert_eq!((ring.minimum(), ring.maximum(), ring.value()), (0.0, 200.0, 42.0));
    assert!(!ring.is_indeterminate());
    let rating = root.get_control::<RatingControl>("Rating");
    assert_eq!(rating.placeholder_value(), 2.5);
    let info = rating.item_info().unwrap().cast::<RatingItemFontInfo>().unwrap();
    assert_eq!(info.glyph(), "B");
    assert_eq!(info.unset_glyph(), "C");
}

#[test]
fn info_bar_primitives_and_close_button_theme_require_explicit_mappings() {
    let input = format!("<Grid xmlns='{WINUI_PRESENTATION_NAMESPACE}' xmlns:p='using:Microsoft.UI.Xaml.Controls.Primitives'><p:InfoBarPanel/><InfoBar CloseButtonStyle='{{StaticResource Custom}}'/></Grid>");
    let rejected = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert!(rejected.has_errors());
    let mut options = WinUIXamlOptions::default();
    options.style_resources.push("Custom".into());
    let result = migrate_winui_xaml(&input, &options).unwrap();
    assert!(!result.has_errors());
    let doc = Document::parse(&result.xaml).unwrap();
    let panel = doc
        .descendants()
        .find(|node| node.tag_name().name() == "InfoBarPanel")
        .unwrap();
    assert_eq!(panel.tag_name().namespace(), Some(PRIMITIVES_NAMESPACE));
}

#[test]
fn generated_prefix_never_collides_with_a_nested_shadowing_declaration() {
    let result =
        migrate("<Page xmlns:fui0='using:Custom'><Frame xmlns:fui1='using:Another'/></Page>");
    let document = Document::parse(&result.xaml).unwrap();
    assert!(result
        .xaml
        .contains("xmlns:fui2=\"using:FerroUI.Controls\""));
    for element in document.descendants().filter(Node::is_element).skip(1) {
        assert_eq!(element.tag_name().namespace(), Some(CONTROLS_NAMESPACE));
    }
}

#[test]
fn literal_visibility_and_toggle_state_map_but_binding_visibility_fails() {
    let result = migrate("<ToggleSwitch IsOn='False'/><Button Visibility='Visible'/><Button Visibility='{Binding State}'/>");
    assert!(result.xaml.contains("IsChecked='False'"));
    assert!(result.xaml.contains("IsVisible='True'"));
    assert!(result.diagnostics.iter().any(|d| d.code == "WXAML1003"));
    assert!(result.into_xaml().is_err());
}

#[test]
fn implicit_winui_binding_modes_are_one_way_even_for_two_way_target_properties() {
    let result = migrate("<TextBox Text='{Binding Name}'/><TextBox Text='{Binding}'/><Slider Value='{Binding Level, Mode=TwoWay}'/>");
    assert!(result.xaml.contains("{Binding Name, Mode=OneWay}"));
    assert!(result.xaml.contains("{Binding Mode=OneWay}"));
    assert!(result.xaml.contains("{Binding Level, Mode=TwoWay}"));
    assert!(!result.has_errors());
}

#[test]
fn binding_modes_inside_nested_extensions_or_quoted_values_do_not_count() {
    let result = migrate("<TextBox Text=\"{Binding Name, FallbackValue='Mode=TwoWay,{text}', Converter={StaticResource Converter}}\"/>");
    assert!(result
        .xaml
        .contains("Converter={StaticResource Converter}, Mode=OneWay}"));
    assert!(!result.has_errors());
}

#[test]
fn xbind_reports_root_source_semantics_even_with_an_alternative_xaml_prefix() {
    let input = format!("<Grid xmlns='{WINUI_PRESENTATION_NAMESPACE}' xmlns:language='{XAML_NAMESPACE}'>\n  <TextBlock Text='{{language:Bind Name}}'/>\n</Grid>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    let error = result
        .diagnostics
        .iter()
        .find(|d| d.code == "WXAML1007")
        .unwrap();
    assert_eq!(error.span.line, 2);
    assert_eq!(&input[error.span.bytes.clone()], "{language:Bind Name}");
    assert!(result
        .into_xaml()
        .unwrap_err()
        .to_string()
        .contains("WXAML1007 at 2:"));
}

#[test]
fn escaped_markup_stays_literal() {
    let result = migrate("<TextBlock Text='{}{x:Bind Name}'/>");
    assert!(!result.has_errors());
    assert!(result.xaml.contains("{}{x:Bind Name}"));
}

#[test]
fn theme_resources_require_explicit_policy_and_nested_lookups_are_recorded() {
    let input = source("<TextBlock Foreground='{ThemeResource TextFillColorPrimaryBrush}' Text='{Binding Name, FallbackValue={ThemeResource MissingName}}'/>");
    let rejected = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert_eq!(
        rejected
            .diagnostics
            .iter()
            .filter(|d| d.code == "WXAML1008")
            .count(),
        2
    );
    let adapted = migrate_winui_xaml(&input, &WinUIXamlOptions::for_winui_theme()).unwrap();
    assert!(!adapted.has_errors());
    assert_eq!(adapted.xaml.matches("DynamicResource").count(), 2);
    assert_eq!(
        adapted
            .diagnostics
            .iter()
            .filter(|d| d.code == "WXAML2001")
            .count(),
        2
    );
}

#[test]
fn only_audited_style_resources_are_control_themes() {
    let input = source("<Button Style='{StaticResource AccentButtonStyle}'/><Button Style='{StaticResource AppStyle}'/>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::for_winui_theme()).unwrap();
    assert!(result
        .xaml
        .contains("Theme='{StaticResource AccentButtonStyle}'"));
    assert!(result.diagnostics.iter().any(|d| d.code == "WXAML1005"));
    assert!(migrate("<Style TargetType='Button'/>").has_errors());
}

#[test]
fn unknown_state_lifecycle_and_localization_contracts_are_not_discarded() {
    let result = migrate("<Button x:Load='False' x:Uid='SaveButton'/><VisualStateManager.VisualStateGroups><VisualStateGroup/></VisualStateManager.VisualStateGroups>");
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "WXAML1002")
            .count(),
        2
    );
    assert!(result.diagnostics.iter().any(|d| d.code == "WXAML1001"));
    assert!(result.xaml.contains("x:Uid='SaveButton'"));
}

#[test]
fn custom_type_member_and_namespace_mappings_are_explicit_and_local() {
    let input = source("<app:Thing xmlns:app='using:Original.Controls' Caption='Hello'><app:Thing.Content><TextBlock/></app:Thing.Content></app:Thing>");
    let mut options = WinUIXamlOptions::default();
    options.types.push(WinUIXamlTypeMapping::new(
        "using:Original.Controls",
        "Thing",
        "using:Port.Controls",
        "Widget",
    ));
    options.members.push(WinUIXamlMemberMapping::new(
        "using:Original.Controls",
        "Thing",
        "Caption",
        "Header",
    ));
    let result = migrate_winui_xaml(&input, &options).unwrap();
    assert!(!result.has_errors());
    let doc = Document::parse(&result.xaml).unwrap();
    let widget = doc
        .descendants()
        .find(|n| n.has_tag_name(("using:Port.Controls", "Widget")))
        .unwrap();
    assert_eq!(widget.attribute("Header"), Some("Hello"));
    assert!(doc
        .descendants()
        .any(|n| n.has_tag_name(("using:Port.Controls", "Widget.Content"))));
    // Mapping application classes does not mutate the global type registry or the next call.
    assert!(migrate_winui_xaml(&input, &WinUIXamlOptions::default())
        .unwrap()
        .xaml
        .contains("app:Thing"));
}

#[test]
fn custom_namespace_maps_update_class_identity_without_rewriting_user_text() {
    let input = format!("<UserControl xmlns='{WINUI_PRESENTATION_NAMESPACE}' xmlns:x='{XAML_NAMESPACE}' xmlns:app='using:Original.Pages' x:Class='Original.Pages.Example'><TextBlock Text='Original.Pages.Example'/></UserControl>");
    let mut options = WinUIXamlOptions::default();
    options.namespaces.push(WinUIXamlNamespaceMapping::new(
        "using:Original.Pages",
        "using:Port.Pages",
    ));
    let result = migrate_winui_xaml(&input, &options).unwrap();
    assert!(result.xaml.contains("x:Class='Port.Pages.Example'"));
    assert!(result.xaml.contains("Text='Original.Pages.Example'"));
}

#[test]
fn asset_uris_require_an_explicit_base_and_unrelated_strings_are_not_rewritten() {
    let input = source(
        "<Image Source='ms-appx:///Assets/a.png'/><TextBlock Text='ms-appx:///Assets/a.png'/>",
    );
    assert!(migrate_winui_xaml(&input, &WinUIXamlOptions::default())
        .unwrap()
        .has_errors());
    let mut options = WinUIXamlOptions::default();
    options.asset_base_uri = Some("ferres://Gallery/".into());
    let result = migrate_winui_xaml(&input, &options).unwrap();
    assert!(result
        .xaml
        .contains("Source='ferres://Gallery/Assets/a.png'"));
    assert!(result.xaml.contains("Text='ms-appx:///Assets/a.png'"));
}

#[test]
fn malformed_xml_duplicate_policy_and_invalid_names_are_errors_not_panics() {
    assert!(migrate_winui_xaml("<Grid>", &WinUIXamlOptions::default()).is_err());
    assert!(migrate_winui_xaml(
        "<!DOCTYPE a [<!ENTITY x 'v'>]><a>&x;</a>",
        &WinUIXamlOptions::default()
    )
    .is_err());
    let mut options = WinUIXamlOptions::default();
    options
        .namespaces
        .push(WinUIXamlNamespaceMapping::new("a", "b"));
    options
        .namespaces
        .push(WinUIXamlNamespaceMapping::new("a", "c"));
    assert!(migrate_winui_xaml("<a/>", &options).is_err());
    options.namespaces.clear();
    options
        .types
        .push(WinUIXamlTypeMapping::new("a", "A", "b", "x:Injected"));
    assert!(migrate_winui_xaml("<a/>", &options).is_err());
}

#[test]
fn all_edits_replay_exactly_and_refer_to_original_source() {
    let input = source("\n<ToggleSwitch IsOn='True' Visibility='Collapsed'/><Page/>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    let mut replayed = input.clone();
    for edit in result.edits.iter().rev() {
        replayed.replace_range(edit.span.bytes.clone(), &edit.replacement);
    }
    assert_eq!(replayed, result.xaml);
    assert!(result.edits.iter().any(|edit| edit.span.line == 2));
}

#[test]
fn excessive_extension_nesting_is_a_diagnostic() {
    let mut value = "{StaticResource End}".to_string();
    for _ in 0..80 {
        value = format!("{{Binding X, FallbackValue={value}}}");
    }
    let result = migrate(&format!("<TextBlock Text='{value}'/>"));
    assert!(result.diagnostics.iter().any(|d| d.code == "WXAML0002"));
}

#[test]
fn object_element_binding_defaults_and_explicit_modes_are_preserved() {
    let result = migrate("<TextBox><TextBox.Text><Binding Path='Name'/></TextBox.Text></TextBox><Binding Mode='TwoWay'/>");
    let doc = Document::parse(&result.xaml).unwrap();
    let modes: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name((FERRO_XML_NAMESPACE, "Binding")))
        .map(|node| node.attribute("Mode").unwrap())
        .collect();
    assert_eq!(modes, vec!["OneWay", "TwoWay"]);
}

#[test]
fn aliases_resolve_type_references_in_templates_directives_and_type_extensions() {
    let result = migrate("<ControlTemplate TargetType='Page'><TextBlock x:DataType='Page' Tag='{x:Type Frame}'/></ControlTemplate>");
    let doc = Document::parse(&result.xaml).unwrap();
    let template = doc
        .descendants()
        .find(|n| n.tag_name().name() == "ControlTemplate")
        .unwrap();
    let target = template.attribute("TargetType").unwrap();
    let (prefix, local) = target.split_once(':').unwrap();
    assert_eq!(local, "FramePage");
    assert_eq!(
        template.lookup_namespace_uri(Some(prefix)),
        Some(CONTROLS_NAMESPACE)
    );
    assert!(result.xaml.contains(":Frame}"));
}

#[test]
fn nested_default_namespaces_do_not_leak_mappings_into_other_dialects() {
    let result = migrate("<Button Visibility='Collapsed'/><foreign xmlns='urn:Other' Visibility='Collapsed' Text='{Binding Name}'><Style/></foreign>");
    assert!(!result.has_errors());
    let doc = Document::parse(&result.xaml).unwrap();
    let foreign = doc
        .descendants()
        .find(|n| n.has_tag_name(("urn:Other", "foreign")))
        .unwrap();
    assert_eq!(foreign.attribute("Visibility"), Some("Collapsed"));
    assert_eq!(foreign.attribute("Text"), Some("{Binding Name}"));
}

#[test]
fn explicit_framework_prefixes_are_resolved_in_attached_members_and_extensions() {
    let input = source("<Button xmlns:ui='using:Microsoft.UI.Xaml.Controls' ui:Grid.Row='2' Foreground='{ui:ThemeResource Brush}'/>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::for_winui_theme()).unwrap();
    assert!(!result.has_errors());
    assert!(result.xaml.contains("ui:Grid.Row='2'"));
    assert!(result.xaml.contains(":DynamicResource Brush}"));
    Document::parse(&result.xaml).unwrap();
}

#[test]
fn event_handlers_and_entity_spelling_stay_exactly_as_authored() {
    let result = migrate("<Button Click='OnClick' Content='A &amp; B &quot;quoted&quot;'/>");
    assert!(result.xaml.contains("Click='OnClick'"));
    assert!(result
        .xaml
        .contains("Content='A &amp; B &quot;quoted&quot;'"));
}

#[test]
fn ignored_design_attributes_and_subtrees_remain_verbatim() {
    let input = source("<Grid xmlns:mc='http://schemas.openxmlformats.org/markup-compatibility/2006' xmlns:d='http://schemas.microsoft.com/expression/blend/2008' mc:Ignorable='d' d:Text='{x:Bind DesignerOnly}'><d:Sample><Button Content='{ThemeResource Ignored}'/></d:Sample><TextBox PlaceholderText='Keep the actual registered property'/></Grid>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert!(!result.has_errors());
    assert!(result.xaml.contains("d:Text='{x:Bind DesignerOnly}'"));
    assert!(result
        .xaml
        .contains("<Button Content='{ThemeResource Ignored}'/>"));
    assert!(result
        .xaml
        .contains("PlaceholderText='Keep the actual registered property'"));
}

#[test]
fn shadowing_an_ignorable_prefix_does_not_ignore_a_different_namespace() {
    let input = source("<Grid xmlns:mc='http://schemas.openxmlformats.org/markup-compatibility/2006' xmlns:d='urn:Designer' mc:Ignorable='d'><Button xmlns:d='urn:Runtime' d:Unknown='{x:Bind StillRuntime}'/></Grid>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "WXAML1007"));
}

#[test]
fn unsupported_features_in_unused_resources_still_report_original_locations() {
    let input = source("<Grid.Resources>\n<DataTemplate x:Key='Unused'><TextBlock Text='{x:Bind Name}'/></DataTemplate>\n</Grid.Resources>");
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "WXAML1007")
        .unwrap();
    assert_eq!(diagnostic.span.line, 2);
    assert_eq!(&input[diagnostic.span.bytes.clone()], "{x:Bind Name}");
}

#[test]
fn long_attribute_whitespace_uses_true_source_ranges() {
    let input = source(&format!(
        "<Button Visibility{}={} 'Collapsed'/>",
        " ".repeat(500),
        " ".repeat(500)
    ));
    let result = migrate_winui_xaml(&input, &WinUIXamlOptions::default()).unwrap();
    assert!(!result.has_errors());
    let doc = Document::parse(&result.xaml).unwrap();
    assert!(doc
        .descendants()
        .any(|node| node.attribute("IsVisible") == Some("False")));
}

#[test]
fn opt_in_runtime_loader_creates_real_controls_and_preserves_normal_loader_dialect() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    let source = format!(
        "<Button xmlns='{WINUI_PRESENTATION_NAMESPACE}' Visibility='Collapsed' Content='Live'/>"
    );
    assert!(FerroRuntimeXamlLoader::parse(&source, None).is_err());
    let result =
        crate::parse_winui(&source, None, &WinUIXamlOptions::default()).unwrap();
    let object = ValueTypes::as_object(&*result).unwrap();
    assert_eq!(object.get_type().full_name(), "FerroUI.Controls.Button");
    assert!(!object.get_value(ferroui_base::Visual::is_visible_property()));
    // The opt-in call does not register the source namespace as a global alias.
    assert!(FerroRuntimeXamlLoader::parse(&source, None).is_err());
    let native = "<Button xmlns='https://github.com/ferroui' IsVisible='True'/>";
    assert!(FerroRuntimeXamlLoader::parse(native, None).is_ok());
}

#[test]
fn runtime_rejects_semantic_diagnostics_before_instantiating_unknown_controls() {
    let input = source("<Unknown Text='{x:Bind Name}'/>");
    let error = crate::parse_winui(
        &input,
        None,
        &WinUIXamlOptions::default(),
    )
    .err()
    .unwrap();
    assert!(error.message().contains("WXAML1007"));
}

#[test]
fn fresh_root_deferred_resource_error_is_returned_and_default_loader_still_throws() {
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    let input = source("<Grid.Resources><ControlTheme x:Key='Example' TargetType='Button' BasedOn='{StaticResource MissingTheme}'/></Grid.Resources><Button Theme='{StaticResource Example}'/>");
    let error = crate::parse_winui(&input, None, &WinUIXamlOptions::default())
        .unwrap_err();
    assert!(error.to_string().contains("MissingTheme"));
    let native = migrate_winui_xaml(&input, &WinUIXamlOptions::default())
        .unwrap()
        .into_xaml()
        .unwrap();
    assert!(
        catch_unwind(AssertUnwindSafe(|| FerroRuntimeXamlLoader::parse(
            &native, None
        )))
        .is_err()
    );
}

fn xbind_options() -> WinUIXamlOptions {
    WinUIXamlOptions {
        x_bind: WinUIXBindPolicy::PropertyPathsWithExplicitLifecycle,
        ..Default::default()
    }
}

#[test]
fn property_path_xbind_is_a_separate_extension_with_original_location_and_explicit_scope() {
    let input = source("\n<TextBox Text='{x:Bind ViewModel.Name, Mode=TwoWay}'/>");
    let result = migrate_winui_xaml(&input, &xbind_options()).unwrap();
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let doc = Document::parse(&result.xaml).unwrap();
    let value = doc
        .descendants()
        .find(|n| n.has_tag_name("TextBox"))
        .unwrap()
        .attribute("Text")
        .unwrap();
    assert!(value.contains(":WinUIBind Path=ViewModel.Name, Mode=TwoWay, Scope=Root"));
    assert!(value.contains("SourceLine=2"));
    assert!(!value.contains("UpdateSourceTrigger="));
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "WXAML2004" && d.span.line == 2));
    assert!(result.xaml.contains("using:FerroUI.Markup.Xaml.WinUI"));
    let twice = migrate_winui_xaml(&result.xaml, &xbind_options()).unwrap();
    assert_eq!(twice.xaml, result.xaml);
    assert!(twice.edits.is_empty());
}

#[test]
fn property_path_policy_still_rejects_functions_events_casts_indexers_and_unknown_parameters() {
    for body in [
        "<TextBlock Text='{x:Bind Format(Name)}'/>",
        "<TextBlock Text='{x:Bind Items[0].Name}'/>",
        "<TextBlock Text='{x:Bind ((Model)Data).Name}'/>",
        "<Button Click='{x:Bind OnClick}'/>",
        "<TextBox Loaded='{x:Bind OnLoaded}'/>",
        "<TextBlock Text='{x:Bind Name, Converter={StaticResource Converter}}'/>",
        "<TextBox Text='{x:Bind Name, Mode=Default}'/>",
        "<TextBox Text='{x:Bind Name, UpdateSourceTrigger=PropertyChanged}'/>",
        "<TextBox Text='{x:Bind Name, Mode=TwoWay, UpdateSourceTrigger=Explicit}'/>",
        "<TextBlock Text='{x:Bind Name, Mode=OneTime, Mode=OneWay}'/>",
        "<TextBlock Text='{Binding Name, FallbackValue={x:Bind Other}}'/>",
    ] {
        let input = source(body);
        let result = migrate_winui_xaml(&input, &xbind_options()).unwrap();
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|d| d.code == "WXAML1007")
            .expect(body);
        assert!(input[diagnostic.span.bytes.clone()].contains("{x:Bind"));
        assert!(result.into_xaml().is_err(), "{body}");
    }
}

#[test]
fn event_rejection_resolves_the_declaring_namespace_and_inherited_members() {
    let input = format!("<Grid xmlns='{WINUI_PRESENTATION_NAMESPACE}' xmlns:x='{XAML_NAMESPACE}' xmlns:ui='using:Microsoft.UI.Xaml.Controls' xmlns:custom='using:MyControls'><ui:Button Click='{{x:Bind ClickHandler}}'/><custom:Button Click='{{x:Bind Label}}'/></Grid>");
    let mut options = xbind_options();
    options.namespaces.push(WinUIXamlNamespaceMapping::new(
        "using:MyControls",
        "using:PortedControls",
    ));
    let result = migrate_winui_xaml(&input, &options).unwrap();
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "WXAML1007")
            .count(),
        1
    );
    assert!(result.xaml.contains("Click='{fui"));
}

#[test]
fn typed_template_scope_captures_its_type_namespace_even_when_the_prefix_is_shadowed() {
    let input = format!("<DataTemplate xmlns='{WINUI_PRESENTATION_NAMESPACE}' xmlns:x='{XAML_NAMESPACE}' xmlns:item='using:OriginalModels' x:DataType='item:Person'><TextBlock xmlns:item='using:OtherModels' Text='{{x:Bind Name}}'/></DataTemplate>");
    let mut options = xbind_options();
    options.namespaces.push(WinUIXamlNamespaceMapping::new(
        "using:OriginalModels",
        "using:PortedModels",
    ));
    let result = migrate_winui_xaml(&input, &options).unwrap();
    assert!(!result.has_errors());
    let doc = Document::parse(&result.xaml).unwrap();
    let text = doc
        .descendants()
        .find(|node| node.has_tag_name("TextBlock"))
        .unwrap();
    let binding = text.attribute("Text").unwrap();
    assert!(binding.contains("Scope=Template"));
    assert!(binding.contains("Mode=OneTime"));
    let type_ = binding
        .split("Type ")
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();
    let (prefix, name) = type_.split_once(':').unwrap();
    assert_eq!(name, "Person");
    assert_eq!(
        text.lookup_namespace_uri(Some(prefix)),
        Some("using:PortedModels")
    );
    for body in [
        "<DataTemplate><TextBlock Text='{x:Bind Name}'/></DataTemplate>",
        "<ControlTemplate><TextBlock Text='{x:Bind Name}'/></ControlTemplate>",
    ] {
        assert!(migrate_winui_xaml(&source(body), &xbind_options())
            .unwrap()
            .has_errors());
    }
}

#[test]
fn migrated_xbind_stays_dormant_until_the_host_initializes_the_document() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_base::data::BindingPriority;
    use ferroui_base::{BoxedValue, FerroPropertyRegistry};
    use crate::WinUIBindings;
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    use std::rc::Rc;
    let input = format!("<TextBlock xmlns='{WINUI_PRESENTATION_NAMESPACE}' xmlns:x='{XAML_NAMESPACE}' Tag='initial' Text='{{x:Bind Tag}}'/>");
    let value = crate::parse_winui(&input, None, &xbind_options()).unwrap();
    let root = ValueTypes::as_object(&*value).unwrap();
    let text = FerroPropertyRegistry::instance()
        .find_registered_for(&root, "Text")
        .unwrap();
    let tag = FerroPropertyRegistry::instance()
        .find_registered_for(&root, "Tag")
        .unwrap();
    assert!(ValueTypes::normalize(root.get_value_untyped(text)).is_none());
    let late: Option<BoxedValue> = Some(Rc::new("after code-behind".to_string()));
    root.set_value_untyped(tag, &late, BindingPriority::LocalValue);
    let group = WinUIBindings::get_for_root(&root).unwrap();
    assert!(!group.is_initialized());
    group.initialize().unwrap();
    let text_value = ValueTypes::normalize(root.get_value_untyped(text)).unwrap();
    assert_eq!(
        text_value.downcast_ref::<String>().map(String::as_str),
        Some("after code-behind")
    );
}

#[test]
fn migrated_template_xbind_uses_each_deferred_instance_and_typed_item() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_base::FerroPropertyRegistry;
    use ferroui_markup_xaml::templates::DataTemplate;
    use crate::WinUIBindings;
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    use std::rc::Rc;
    let input = format!("<DataTemplate xmlns='{WINUI_PRESENTATION_NAMESPACE}' xmlns:x='{XAML_NAMESPACE}' x:DataType='Button'><TextBlock Text='{{x:Bind Content}}'/></DataTemplate>");
    let template: Rc<DataTemplate> =
        crate::parse_winui_as(&input, None, &xbind_options()).unwrap();
    let first = template.build(None).unwrap();
    let second = template.build(None).unwrap();
    let first_item = FerroRuntimeXamlLoader::parse(
        "<Button xmlns='https://github.com/ferroui' Content='first'/>",
        None,
    )
    .unwrap();
    let second_item = FerroRuntimeXamlLoader::parse(
        "<Button xmlns='https://github.com/ferroui' Content='second'/>",
        None,
    )
    .unwrap();
    first.set_data_context(Some(first_item));
    second.set_data_context(Some(second_item));
    WinUIBindings::get_for_root(&first)
        .unwrap()
        .initialize()
        .unwrap();
    WinUIBindings::get_for_root(&second)
        .unwrap()
        .initialize()
        .unwrap();
    let text = FerroPropertyRegistry::instance()
        .find_registered_for(&first, "Text")
        .unwrap();
    let read = |value: ferroui_base::BoxedValue| {
        ValueTypes::normalize(value)
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .clone()
    };
    assert_eq!(read(first.get_value_untyped(text)), "first");
    assert_eq!(read(second.get_value_untyped(text)), "second");
    assert!(!Rc::ptr_eq(
        &WinUIBindings::get_for_root(&first).unwrap(),
        &WinUIBindings::get_for_root(&second).unwrap()
    ));
}
