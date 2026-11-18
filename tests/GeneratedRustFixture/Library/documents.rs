//! The documents of the classes of the library, by their path below `ferres://GeneratedRust.Library/`.
//! The document of each class merges a dictionary of its own (`MergeResourceInclude`), so the
//! compiled markup of each class is partitioned into units, whether the class can be constructed
//! from some of them or not.

/// The application class [`crate::App`]: an ordinary class with `new()` only.
pub const APP: &[(&str, &str)] = &[
    (
        "App.xaml",
        "<Application xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             x:Class='GeneratedRust.Library.App'>
  <Application.Resources>
    <ResourceDictionary>
      <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='/AppResources.xaml'/>
      </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
  </Application.Resources>
</Application>",
    ),
    (
        "AppResources.xaml",
        "<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                    x:ClassModifier='internal'>
  <SolidColorBrush x:Key='AppAccentBrush'>#ff102030</SolidColorBrush>
  <x:Double x:Key='AppSpacing'>4</x:Double>
</ResourceDictionary>",
    ),
];

/// The theme `GeneratedRust.Library.A_B.PartsTheme`, constructed from selected units
/// (`with_parts`).
pub const PARTS_THEME: &[(&str, &str)] = &[
    (
        "A_B/PartsTheme.xaml",
        "<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        x:Class='GeneratedRust.Library.A_B.PartsTheme'>
  <Styles.Resources>
    <ResourceDictionary>
      <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='/A_B/PartsThemeResources.xaml'/>
      </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
  </Styles.Resources>
</Styles>",
    ),
    (
        "A_B/PartsThemeResources.xaml",
        "<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                    x:ClassModifier='internal'>
  <SolidColorBrush x:Key='PartsThemeBrush'>#ff405060</SolidColorBrush>
</ResourceDictionary>",
    ),
];

/// The theme `GeneratedRust.Library.A.B.Partstheme`, constructed from selected units
/// (`with_parts`): its full name differs from the one of [`PARTS_THEME`] in case and punctuation
/// only, so both give the same established name of a parts table.
pub const PARTS_THEME_LOWER: &[(&str, &str)] = &[
    (
        "A/B/Partstheme.xaml",
        "<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        x:Class='GeneratedRust.Library.A.B.Partstheme'>
  <Styles.Resources>
    <ResourceDictionary>
      <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='/A/B/PartsthemeResources.xaml'/>
      </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
  </Styles.Resources>
</Styles>",
    ),
    (
        "A/B/PartsthemeResources.xaml",
        "<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                    x:ClassModifier='internal'>
  <SolidColorBrush x:Key='PartsthemeBrush'>#ff708090</SolidColorBrush>
</ResourceDictionary>",
    ),
];

/// The theme `GeneratedRust.Library.RenamedTheme`: partitioned into units, constructed from some of
/// them by a function of another name (`from_parts`), so other crates construct it with `new()`.
pub const RENAMED_THEME: &[(&str, &str)] = &[
    (
        "RenamedTheme.xaml",
        "<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        x:Class='GeneratedRust.Library.RenamedTheme'>
  <Styles.Resources>
    <ResourceDictionary>
      <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='/RenamedThemeResources.xaml'/>
      </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
  </Styles.Resources>
</Styles>",
    ),
    (
        "RenamedThemeResources.xaml",
        "<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                    x:ClassModifier='internal'>
  <SolidColorBrush x:Key='RenamedThemeBrush'>#ffa0b0c0</SolidColorBrush>
</ResourceDictionary>",
    ),
];
