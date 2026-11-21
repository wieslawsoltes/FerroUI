use super::*;

const NS: &str =
    r#"xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml""#;

struct Includes(Document);
impl ComponentResourceSources for Includes {
    fn include(&self, _: &str, _: &Document, node: &Node) -> Option<IncludedResourceDocument<'_>> {
        (node.attribute("Source") == Some("Colors.xaml")).then_some(IncludedResourceDocument {
            address: "ferrores://sample/Themes/Colors.xaml",
            uri: "file:///local/Theme%20Colors.xaml",
            document: &self.0,
        })
    }
}

#[test]
fn palette_resolves_static_include_mounts_and_dynamic_hosts_independently() {
    let dictionary = Includes(Document::new(format!(r#"
<ResourceDictionary {NS}>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key="Default"><Color x:Key="Mapped">Red</Color></ResourceDictionary>
        <ResourceDictionary x:Key="Dark"><Color x:Key="Mapped">Blue</Color></ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
    <SolidColorBrush x:Key="Fixed"><SolidColorBrush.Color><StaticResource ResourceKey="Mapped"/></SolidColorBrush.Color></SolidColorBrush>
    <SolidColorBrush x:Key="Live" Color="{{DynamicResource Accent}}"/>
    <SolidColorBrush x:Key="FromParent" Color="{{StaticResource ShellInk}}"/>
    <StaticResource x:Key="Camera"><StaticResource.ResourceKey>Fixed</StaticResource.ResourceKey></StaticResource>
</ResourceDictionary>"#)).unwrap());
    let document = Document::new(format!(r#"
<Grid {NS}>
    <Grid.Resources><ResourceDictionary>
        <ResourceDictionary.ThemeDictionaries>
            <ResourceDictionary x:Key="Default"><Color x:Key="Accent">Gray</Color></ResourceDictionary>
            <ResourceDictionary x:Key="Light"><Color x:Key="Accent">Teal</Color></ResourceDictionary>
            <ResourceInclude x:Key="Dark" Source="Colors.xaml"/>
        </ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary.MergedDictionaries><ResourceInclude Source="Colors.xaml"/></ResourceDictionary.MergedDictionaries>
        <SolidColorBrush x:Key="Chroma" Color="{{StaticResource Accent}}"/>
        <StaticResource x:Key="Stable" ResourceKey="Chroma"/>
        <Color x:Key="ShellInk">Purple</Color>
    </ResourceDictionary></Grid.Resources>
    <Border Name="Consumer"><Border.Resources><Color x:Key="Accent">Orange</Color></Border.Resources></Border>
</Grid>"#)).unwrap();
    let root = document.ast().root();
    let consumer = document.ast().find_name("Consumer").unwrap();
    let dark = ComponentResourceContext {
        theme: Some("Dark".into()),
        ..ComponentResourceContext::default()
    };
    let light = ComponentResourceContext {
        theme: Some("Light".into()),
        ..ComponentResourceContext::default()
    };
    let resolve = |at, key, context| {
        ResourceValue::lookup(
            &document,
            "file:///View.xaml",
            Some(&dictionary),
            at,
            key,
            context,
        )
        .unwrap()
        .unwrap()
    };
    let color = |at, key, context| {
        let trace = resolve(at, key, Some(context)).color_trace(key, context);
        assert!(trace.issue.is_none(), "{key}: {:?}", trace.issue);
        let terminal = &trace.steps.last().unwrap().value;
        let color = palette_color(terminal.document(), terminal.location().node)
            .unwrap()
            .unwrap();
        (terminal.location(), color.value.unwrap())
    };
    assert_eq!(color(root, "Camera", &light).1, "Red");
    assert_eq!(color(root, "Camera", &dark).1, "Blue");
    assert_eq!(
        color(root, "Camera", &dark).0.uri,
        Some("file:///local/Theme%20Colors.xaml")
    );
    assert_eq!(color(consumer, "Live", &dark).1, "Gray");
    assert_eq!(color(consumer, "Live", &light).1, "Teal");
    assert_eq!(color(consumer, "Stable", &dark).1, "Gray");
    assert_eq!(color(consumer, "FromParent", &light).1, "Purple");
    assert_eq!(color(consumer, "FromParent", &dark).0.uri, None);
    let brush = resolve(root, "Stable", None)
        .alias_target("Stable", None)
        .unwrap();
    assert_eq!(
        brush
            .document()
            .ast()
            .node(brush.location().node)
            .unwrap()
            .local_name(),
        "SolidColorBrush"
    );
}

#[test]
fn palette_reports_cycles_and_opaque_resource_values_without_unbounded_tracing() {
    let document = Document::new(format!(r#"
<Grid {NS}><Grid.Resources><ResourceDictionary>
    <StaticResource x:Key="One" ResourceKey="Two"/>
    <StaticResource x:Key="Two" ResourceKey="One"/>
    <SolidColorBrush x:Key="Unverified" Color="{{DynamicResource External}}"/>
    <ResourceDictionary.MergedDictionaries><ResourceInclude Source="Opaque.xaml"/></ResourceDictionary.MergedDictionaries>
</ResourceDictionary></Grid.Resources></Grid>"#)).unwrap();
    let context = ComponentResourceContext::default();
    let error = |key| {
        ResourceValue::lookup(
            &document,
            "",
            None,
            document.ast().root(),
            key,
            Some(&context),
        )
        .unwrap()
        .unwrap()
        .color_trace(key, &context)
        .issue
        .unwrap()
    };
    assert!(error("One").contains("cycle"));
    assert!(error("Unverified").contains("no unambiguous authored value"));
}
