// Generated from pinned Core and control enum sources. Consumed only by the native metadata exporter.
pub struct EnumDeclaration {
    pub name: &'static str,
    pub namespace: &'static str,
    pub rust_type: &'static str,
    pub members: &'static [(&'static str, i32)],
}
pub const ENUMS: &[EnumDeclaration] = &[
    EnumDeclaration {
        name: "RibbonItemSize",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonItemSize",
        members: &[("Large", 0), ("Medium", 1), ("Small", 2)],
    },
    EnumDeclaration {
        name: "RibbonGroupState",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonGroupState",
        members: &[("Large", 0), ("Medium", 1), ("Small", 2), ("Collapsed", 3)],
    },
    EnumDeclaration {
        name: "RibbonDisplayMode",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonDisplayMode",
        members: &[("Classic", 0), ("Simplified", 1)],
    },
    EnumDeclaration {
        name: "RibbonVisibilityMode",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonVisibilityMode",
        members: &[
            ("AlwaysShow", 0),
            ("TabsOnly", 1),
            ("FullScreen", 2),
            ("PanelButtons", 3),
            ("PanelTitles", 4),
        ],
    },
    EnumDeclaration {
        name: "RibbonMinimizeBehavior",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonMinimizeBehavior",
        members: &[
            ("Tabs", 0),
            ("PanelTitles", 1),
            ("PanelButtons", 2),
            ("CycleAll", 3),
        ],
    },
    EnumDeclaration {
        name: "RibbonQuickAccessPosition",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonQuickAccessPosition",
        members: &[("AboveRibbon", 0), ("BelowRibbon", 1)],
    },
    EnumDeclaration {
        name: "RibbonDensity",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonDensity",
        members: &[("Comfortable", 0), ("Compact", 1), ("Touch", 2)],
    },
    EnumDeclaration {
        name: "RibbonSimplifiedVisibility",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonSimplifiedVisibility",
        members: &[("Auto", 0), ("Pinned", 1), ("Overflow", 2), ("Hidden", 3)],
    },
    EnumDeclaration {
        name: "RibbonGroupItemsLayout",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonGroupItemsLayout",
        members: &[("Columns", 0), ("Rows", 1)],
    },
    EnumDeclaration {
        name: "RibbonContextualActivation",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonContextualActivation",
        members: &[("None", 0), ("SelectOnShow", 1)],
    },
    EnumDeclaration {
        name: "RibbonMergeAction",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonMergeAction",
        members: &[("Merge", 0), ("Replace", 1), ("Remove", 2), ("Add", 3)],
    },
    EnumDeclaration {
        name: "RibbonBackstagePlacement",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonBackstagePlacement",
        members: &[("Top", 0), ("Bottom", 1)],
    },
    EnumDeclaration {
        name: "RibbonToolBarOrientation",
        namespace: "FerroRibbon",
        rust_type: "ferroribbon_core::RibbonToolBarOrientation",
        members: &[("Horizontal", 0), ("Vertical", 1)],
    },
    EnumDeclaration {
        name: "RibbonChromeStyle",
        namespace: "FerroRibbon.Theming",
        rust_type: "ferroribbon_core::theming::RibbonChromeStyle",
        members: &[("Colorful", 0), ("Neutral", 1)],
    },
    EnumDeclaration {
        name: "RibbonThemeStyle",
        namespace: "FerroRibbon.Theming",
        rust_type: "ferroribbon_core::theming::RibbonThemeStyle",
        members: &[("Office", 0), ("Cad", 1)],
    },
    EnumDeclaration {
        name: "RibbonReductionStrategy",
        namespace: "FerroRibbon.Layout",
        rust_type: "ferroribbon_core::layout::RibbonReductionStrategy",
        members: &[("Stepwise", 0), ("GroupByGroup", 1)],
    },
    EnumDeclaration {
        name: "RibbonPanelPresentation",
        namespace: "FerroRibbon.Controls",
        rust_type: "ferroribbon::controls::RibbonPanelPresentation",
        members: &[("Full", 0), ("Buttons", 1), ("Titles", 2)],
    },
    EnumDeclaration {
        name: "RibbonIconKind",
        namespace: "FerroRibbon.Model",
        rust_type: "ferroribbon_core::model::RibbonIconKind",
        members: &[("Glyph", 0), ("Path", 1), ("Image", 2), ("Text", 3)],
    },
    EnumDeclaration {
        name: "Visibility",
        namespace: "FerroRibbon.Compatibility",
        rust_type: "ferroribbon::compat::Visibility",
        members: &[("Visible", 0), ("Collapsed", 1)],
    },
];
