//! The documents of the consumer, by their path below `ferres://GeneratedRust.Consumer/`.

/// The documents compiled into `compiled_xaml.rs`, as one group.
pub const DOCUMENTS: &[(&str, &str)] = &[
    // An application class of another crate whose markup is partitioned into units and that has
    // `new()` only: constructed with `new()`.
    (
        "Application.xaml",
        "<local:App xmlns:local='clr-namespace:GeneratedRust.Library;assembly=GeneratedRust.Library'/>",
    ),
    // Themes of another crate: two whose full names give the same established name of a parts
    // table (`with_parts` over two tables), and one whose constructor from units has another name
    // (`new()`).
    (
        "Themes.xaml",
        "<Styles xmlns='https://github.com/ferroui'
        xmlns:ab='clr-namespace:GeneratedRust.Library.A_B;assembly=GeneratedRust.Library'
        xmlns:lower='clr-namespace:GeneratedRust.Library.A.B;assembly=GeneratedRust.Library'
        xmlns:library='clr-namespace:GeneratedRust.Library;assembly=GeneratedRust.Library'>
  <ab:PartsTheme/>
  <lower:Partstheme/>
  <library:RenamedTheme/>
</Styles>",
    ),
    // Every theme of the framework: constructed from the units the crate keeps, each over the
    // parts table of its established name.
    (
        "BuiltInThemes.xaml",
        "<Styles xmlns='https://github.com/ferroui'
        xmlns:fluent='clr-namespace:FerroUI.Themes.Fluent;assembly=FerroUI.Themes.Fluent'
        xmlns:macos='clr-namespace:FerroUI.Themes.MacOS;assembly=FerroUI.Themes.MacOS'
        xmlns:material='clr-namespace:FerroUI.Themes.Material;assembly=FerroUI.Themes.Material'
        xmlns:simple='clr-namespace:FerroUI.Themes.Simple;assembly=FerroUI.Themes.Simple'
        xmlns:winui='clr-namespace:FerroUI.Themes.WinUI;assembly=FerroUI.Themes.WinUI'>
  <fluent:FluentTheme/>
  <macos:MacOSTheme/>
  <material:MaterialTheme/>
  <simple:SimpleTheme/>
  <winui:WinUITheme/>
</Styles>",
    ),
];

/// The class [`crate::CollisionStyles`] and the documents it includes: `Styles/A-B.xaml` and
/// `Styles/A_B.xaml` are public documents whose established build functions are one name.
pub const COLLISION_STYLES: &[(&str, &str)] = &[
    (
        "CollisionStyles.xaml",
        "<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        x:Class='GeneratedRust.Consumer.CollisionStyles'>
  <StyleInclude Source='/Styles/A-B.xaml'/>
  <StyleInclude Source='/Styles/A_B.xaml'/>
</Styles>",
    ),
    ("Styles/A-B.xaml", "<Styles xmlns='https://github.com/ferroui'/>"),
    ("Styles/A_B.xaml", "<Styles xmlns='https://github.com/ferroui'/>"),
];
