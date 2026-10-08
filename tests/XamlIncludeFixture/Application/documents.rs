//! The documents of the application: the documents upstream's
//! `ResourceIncludeTests` and `StyleIncludeTests` load (upstream gives them no
//! URI; here each is named after its test) that include documents of other
//! assemblies, and the documents of
//! `NonLatin_StyleInclude_Is_Resolved_With_Two_Files`, whose assembly
//! `アセンブリ` this crate is. The documents with the URI `ferres://Tests/..`
//! are documents of the crate `xaml-include-fixture-theme` (the assembly
//! `Tests`), and an include of one is an include across crates; where that
//! crate holds a document under another URI than upstream's (see its
//! `documents.rs`), the include names it there.
//!
//! The texts are upstream's, with the namespace and the scheme of the port.

/// The documents compiled into `compiled_xaml.rs`, by their path below `ferres://アセンブリ/`.
pub const DOCUMENTS: &[(&str, &str)] = &[
    // ResourceIncludeTests.ResourceInclude_Loads_ResourceDictionary
    ("ResourceInclude_Loads_ResourceDictionary.xaml", RESOURCE_INCLUDE_LOADS_RESOURCE_DICTIONARY),
    // ResourceIncludeTests.Missing_ResourceKey_In_ResourceInclude_Does_Not_Cause_StackOverflow
    (
        "Missing_ResourceKey_In_ResourceInclude_Does_Not_Cause_StackOverflow.xaml",
        "
<Application xmlns='https://github.com/ferroui'
         xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
<Application.Resources>
    <ResourceDictionary>
        <ResourceDictionary.MergedDictionaries>
            <ResourceInclude Source='ferres://Tests/Missing_ResourceKey_In_ResourceInclude_Does_Not_Cause_StackOverflow/Resource.xaml'/>
        </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
</Application.Resources>
</Application>",
    ),
    // ResourceIncludeTests.ResourceInclude_Should_Be_Allowed_To_Have_Key_In_Custom_Container
    (
        "ResourceInclude_Should_Be_Allowed_To_Have_Key_In_Custom_Container.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                    xmlns:local='clr-namespace:XamlIncludeFixture.Application;assembly=\u{30a2}\u{30bb}\u{30f3}\u{30d6}\u{30ea}'>
    <ResourceDictionary.MergedDictionaries>
        <local:LocaleCollection>
            <ResourceInclude Source='ferres://Tests/Demo/en-us.xaml' x:Key='English' />
        </local:LocaleCollection>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
    ),
    // StyleIncludeTests.StyleInclude_Is_Built
    (
        "StyleInclude_Is_Built.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'>
    <ContentControl.Styles>
        <StyleInclude Source='ferres://Tests/Xaml/Style1.xaml'/>
    </ContentControl.Styles>
</ContentControl>",
    ),
    // StyleIncludeTests.StyleInclude_Is_Built_Resources
    (
        "StyleInclude_Is_Built_Resources.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='ferres://Tests/Xaml/Style1.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
    ),
    // StyleIncludeTests.StyleInclude_Is_Resolved_With_Two_Files
    (
        "StyleInclude_Is_Resolved_With_Two_Files.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='ferres://Tests/Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
    ),
    // StyleIncludeTests.NonLatin_StyleInclude_Is_Resolved_With_Two_Files
    (
        "\u{30b9}\u{30bf}\u{30a4}\u{30eb}.xaml",
        "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
    ),
    (
        "NonLatin_StyleInclude_Is_Resolved_With_Two_Files.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='ferres://\u{30a2}\u{30bb}\u{30f3}\u{30d6}\u{30ea}/\u{30b9}\u{30bf}\u{30a4}\u{30eb}.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
    ),
    // StyleIncludeTests.Missing_ResourceKey_In_StyleInclude_Does_Not_Cause_StackOverflow
    (
        "Missing_ResourceKey_In_StyleInclude_Does_Not_Cause_StackOverflow.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Styles>
        <StyleInclude Source='ferres://Tests/Missing_ResourceKey_In_StyleInclude_Does_Not_Cause_StackOverflow/Style.xaml'/>
    </ContentControl.Styles>
</ContentControl>",
    ),
    // StyleIncludeTests.StyleInclude_Should_Be_Replaced_With_Direct_Call
    (
        "StyleInclude_Should_Be_Replaced_With_Direct_Call.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:themes='clr-namespace:FerroUI.Themes.Simple;assembly=FerroUI.Themes.Simple'>
    <ContentControl.Styles>
        <themes:SimpleTheme />
        <StyleInclude Source='ferres://FerroUI.Themes.Simple/SimpleTheme.xaml'/>
    </ContentControl.Styles>
</ContentControl>",
    ),
    // Not from upstream: the class document of the Fluent theme, included from another crate.
    (
        "Fluent_Theme_Is_Included_Across_Crates.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'>
    <ContentControl.Styles>
        <StyleInclude Source='ferres://FerroUI.Themes.Fluent/FluentTheme.xaml'/>
    </ContentControl.Styles>
</ContentControl>",
    ),
    // StyleIncludeTests.Style_Inside_Resources_Should_Produce_Warning
    ("Style_Inside_Resources_Should_Produce_Warning.xaml", STYLE_INSIDE_RESOURCES_SHOULD_PRODUCE_WARNING),
];

const RESOURCE_INCLUDE_LOADS_RESOURCE_DICTIONARY: &str = "
<UserControl xmlns='https://github.com/ferroui'
         xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
<UserControl.Resources>
    <ResourceDictionary>
        <ResourceDictionary.MergedDictionaries>
            <ResourceInclude Source='ferres://Tests/Resource.xaml'/>
        </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
</UserControl.Resources>

<Border Name='border' Background='{StaticResource brush}'/>
</UserControl>";

/// The document of `Style_Inside_Resources_Should_Produce_Warning`, which the test
/// also compiles to read the warning.
pub const STYLE_INSIDE_RESOURCES_SHOULD_PRODUCE_WARNING: &str = "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:themes='clr-namespace:FerroUI.Themes.Simple;assembly=FerroUI.Themes.Simple'>
    <ContentControl.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <themes:SimpleTheme />
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </ContentControl.Resources>
</ContentControl>";

/// The documents compiled with `CreateSourceInfo` into `compiled_xaml_source_info.rs`:
/// the case `createSourceInfo: true` of `ResourceInclude_Loads_ResourceDictionary`.
pub const SOURCE_INFO_DOCUMENTS: &[(&str, &str)] =
    &[("ResourceInclude_Loads_ResourceDictionary.xaml", RESOURCE_INCLUDE_LOADS_RESOURCE_DICTIONARY)];
