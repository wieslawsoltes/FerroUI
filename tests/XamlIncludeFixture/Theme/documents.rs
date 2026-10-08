//! The documents of the library: the documents of upstream's
//! `ResourceIncludeTests`, `StyleIncludeTests` and `MergeResourceIncludeTests`
//! that have the URI `ferres://Tests/..`, with the
//! documents those tests load with them in the same compilation.
//!
//! Upstream gives every test a group of its own, so two tests may give the same
//! URI to different documents. A crate holds one document per URI: the
//! documents of a later test (in upstream's order) whose URI an earlier test
//! already uses are under a folder named after the later test, and their
//! includes name them there. A document upstream gives no URI (the document
//! the test loads) is named after its test. A document of another assembly of
//! upstream (`ferres://Demo/..`, `Xaml/Style1.xaml` of the test assembly) is a
//! document of this library under a folder named after that assembly.
//!
//! The texts are upstream's, with the namespace and the scheme of the port.

/// The documents compiled into `compiled_xaml.rs`, by their path below `ferres://Tests/`.
pub const DOCUMENTS: &[(&str, &str)] = &[
    // ResourceIncludeTests.ResourceInclude_Loads_ResourceDictionary
    (
        "Resource.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
<SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
</ResourceDictionary>",
    ),
    // ResourceIncludeTests.Missing_ResourceKey_In_ResourceInclude_Does_Not_Cause_StackOverflow
    (
        "Missing_ResourceKey_In_ResourceInclude_Does_Not_Cause_StackOverflow/Resource.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
<StaticResource x:Key='brush' ResourceKey='missing' />
</ResourceDictionary>",
    ),
    // ResourceIncludeTests.ResourceInclude_Should_Be_Allowed_To_Have_Key_In_Custom_Container (`ferres://Demo/en-us.xaml` upstream)
    (
        "Demo/en-us.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <x:String x:Key='OkButton'>OK</x:String>
</ResourceDictionary>",
    ),
    // StyleIncludeTests.StyleInclude_Is_Built and StyleInclude_Is_Built_Resources: the
    // embedded document `Xaml/Style1.xaml` of upstream's test assembly.
    (
        "Xaml/Style1.xaml",
        r#"<Style xmlns="https://github.com/ferroui"
       xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Style.Resources>
    <Color x:Key="Red">Red</Color>
    <Color x:Key="Green">Green</Color>
    <Color x:Key="Blue">Blue</Color>
  </Style.Resources>
</Style>
"#,
    ),
    // StyleIncludeTests.StyleInclude_Is_Resolved_With_Two_Files and
    // Relative_Root_StyleInclude_Is_Resolved_With_Two_Files (the same document)
    (
        "Style.xaml",
        "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
    ),
    // StyleIncludeTests.Relative_Back_StyleInclude_Is_Resolved_With_Two_Files
    (
        "Subfolder/Style.xaml",
        "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
    ),
    (
        "Subfolder/Folder/Root.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='../Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
    ),
    // StyleIncludeTests.Relative_Root_StyleInclude_Is_Resolved_With_Two_Files (with `Style.xaml`)
    (
        "Folder/Root.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='/Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
    ),
    // StyleIncludeTests.Relative_StyleInclude_Is_Resolved_With_Two_Files
    (
        "Relative_StyleInclude_Is_Resolved_With_Two_Files/Folder/Style.xaml",
        "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
    ),
    (
        "Relative_StyleInclude_Is_Resolved_With_Two_Files/Folder/Root.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
    ),
    // StyleIncludeTests.Relative_Dot_Syntax_StyleInclude_Is_Resolved_With_Two_Files
    (
        "Relative_Dot_Syntax_StyleInclude_Is_Resolved_With_Two_Files/Folder/Style.xaml",
        "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
    ),
    (
        "Relative_Dot_Syntax_StyleInclude_Is_Resolved_With_Two_Files/Folder/Root.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='./Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
    ),
    // StyleIncludeTests.Missing_ResourceKey_In_StyleInclude_Does_Not_Cause_StackOverflow
    (
        "Missing_ResourceKey_In_StyleInclude_Does_Not_Cause_StackOverflow/Style.xaml",
        "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <StaticResource x:Key='brush' ResourceKey='missing' />
    </Style.Resources>
</Style>",
    ),
    // MergeResourceIncludeTests.MergeResourceInclude_Works_With_Single_Resource
    ("Resources.xaml", MERGE_SINGLE_RESOURCES),
    ("MergeResourceInclude_Works_With_Single_Resource.xaml", MERGE_SINGLE_ROOT),
    // MergeResourceIncludeTests.MergeResourceInclude_Is_Allowed_After_ResourceInclude
    (
        "Resources1.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush1'>Red</SolidColorBrush>
</ResourceDictionary>",
    ),
    (
        "Resources2.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush2'>Blue</SolidColorBrush>
</ResourceDictionary>",
    ),
    (
        "MergeResourceInclude_Is_Allowed_After_ResourceInclude.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <ResourceInclude Source='ferres://Tests/Resources2.xaml'/>
        <MergeResourceInclude Source='ferres://Tests/Resources1.xaml'/>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
    ),
    // MergeResourceIncludeTests.MergeResourceInclude_Works_With_Multiple_Resources
    (
        "MergeResourceInclude_Works_With_Multiple_Resources/Resources1.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush1'>Red</SolidColorBrush>
    <SolidColorBrush x:Key='brush2'>Blue</SolidColorBrush>
</ResourceDictionary>",
    ),
    (
        "MergeResourceInclude_Works_With_Multiple_Resources/Resources2.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush4'>Yellow</SolidColorBrush>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/MergeResourceInclude_Works_With_Multiple_Resources/Resources1_2.xaml'/>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
    ),
    (
        "MergeResourceInclude_Works_With_Multiple_Resources.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/MergeResourceInclude_Works_With_Multiple_Resources/Resources1.xaml'/>
        <MergeResourceInclude Source='ferres://Tests/MergeResourceInclude_Works_With_Multiple_Resources/Resources2.xaml'/>
    </ResourceDictionary.MergedDictionaries>
    <SolidColorBrush x:Key='brush5'>Black</SolidColorBrush>
    <SolidColorBrush x:Key='brush6'>White</SolidColorBrush>
</ResourceDictionary>",
    ),
    (
        "MergeResourceInclude_Works_With_Multiple_Resources/Resources1_2.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush3'>Green</SolidColorBrush>
</ResourceDictionary>",
    ),
    // MergeResourceIncludeTests.MergeResourceInclude_Works_With_ThemeDictionaries
    (
        "MergeResourceInclude_Works_With_ThemeDictionaries/Resources1.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Light'>
            <SolidColorBrush x:Key='brush1'>White</SolidColorBrush>
            <SolidColorBrush x:Key='brush2'>Black</SolidColorBrush>
        </ResourceDictionary>
        <ResourceDictionary x:Key='Dark'>
            <SolidColorBrush x:Key='brush1'>Black</SolidColorBrush>
            <SolidColorBrush x:Key='brush2'>White</SolidColorBrush>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
    ),
    (
        "MergeResourceInclude_Works_With_ThemeDictionaries/Resources2.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Light'>
            <SolidColorBrush x:Key='brush3'>Red</SolidColorBrush>
            <SolidColorBrush x:Key='brush4'>Blue</SolidColorBrush>
        </ResourceDictionary>
        <ResourceDictionary x:Key='Dark'>
            <SolidColorBrush x:Key='brush3'>Blue</SolidColorBrush>
            <SolidColorBrush x:Key='brush4'>Red</SolidColorBrush>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
    ),
    (
        "MergeResourceInclude_Works_With_ThemeDictionaries.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/MergeResourceInclude_Works_With_ThemeDictionaries/Resources1.xaml'/>
        <MergeResourceInclude Source='ferres://Tests/MergeResourceInclude_Works_With_ThemeDictionaries/Resources2.xaml'/>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
    ),
    // MergeResourceIncludeTests.MergeResourceInclude_Fails_With_ThemeDictionaries_Duplicate_Resources
    (
        "MergeResourceInclude_Fails_With_ThemeDictionaries_Duplicate_Resources/Resources1.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Light'>
            <SolidColorBrush x:Key='brush1'>White</SolidColorBrush>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
    ),
    (
        "MergeResourceInclude_Fails_With_ThemeDictionaries_Duplicate_Resources/Resources2.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Light'>
            <SolidColorBrush x:Key='brush1'>Black</SolidColorBrush>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
    ),
    (
        "MergeResourceInclude_Fails_With_ThemeDictionaries_Duplicate_Resources.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/MergeResourceInclude_Fails_With_ThemeDictionaries_Duplicate_Resources/Resources1.xaml'/>
        <MergeResourceInclude Source='ferres://Tests/MergeResourceInclude_Fails_With_ThemeDictionaries_Duplicate_Resources/Resources2.xaml'/>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
    ),
];

const MERGE_SINGLE_RESOURCES: &str = "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush2'>Red</SolidColorBrush>
</ResourceDictionary>";

const MERGE_SINGLE_ROOT: &str = "
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <ResourceDictionary>
            <SolidColorBrush x:Key='brush1'>Blue</SolidColorBrush>
            <ResourceDictionary.MergedDictionaries>
                <MergeResourceInclude Source='ferres://Tests/Resources.xaml'/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </UserControl.Resources>
</UserControl>";

/// The documents compiled with `CreateSourceInfo` into `compiled_xaml_source_info.rs`:
/// the case `createSourceInfo: true` of the theories, whose documents upstream loads
/// as one group (`MergeResourceInclude_Works_With_Single_Resource`).
pub const SOURCE_INFO_DOCUMENTS: &[(&str, &str)] = &[
    ("Resources.xaml", MERGE_SINGLE_RESOURCES),
    ("MergeResourceInclude_Works_With_Single_Resource.xaml", MERGE_SINGLE_ROOT),
];

/// The document of [`crate::StyleWithServiceProvider`] (upstream's embedded
/// `Xaml/StyleWithServiceProvider.xaml` of the test assembly), compiled into
/// `compiled_style_with_service_provider.rs`.
pub const STYLE_WITH_SERVICE_PROVIDER: (&str, &str) = (
    "/Xaml/StyleWithServiceProvider.xaml",
    r#"<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
       x:Class="XamlIncludeFixture.Theme.StyleWithServiceProvider">

</Style>
"#,
);
