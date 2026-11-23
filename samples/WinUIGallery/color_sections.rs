//! The sections of the Color page. The groups, the brush names and their usage notes are those of
//! `WinUIGallery/Samples/Color/brushes.json` at the pinned commit 14061eea (see NOTICE.md), in the
//! order of the source's six section pages. `key` is the resource of the WinUI theme a name is
//! drawn with; `None` for the two Mica brushes, which the theme does not define.

/// A brush of a group.
pub(crate) struct ColorBrushInfo {
    pub(crate) name: &'static str,
    pub(crate) usage: &'static str,
    pub(crate) key: Option<&'static str>,
}

/// A group of brushes with what it is for.
pub(crate) struct ColorGroup {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) brushes: &'static [ColorBrushInfo],
}

/// A section of the page.
pub(crate) struct ColorSection {
    pub(crate) title: &'static str,
    pub(crate) groups: &'static [ColorGroup],
}

pub(crate) static COLOR_SECTIONS: &[ColorSection] = &[
    ColorSection {
        title: "Text",
        groups: &[
            ColorGroup {
                name: r#"Text"#,
                description: r#"For UI labels and static text"#,
                brushes: &[
                    ColorBrushInfo { name: "TextFillColorPrimaryBrush", usage: r#"Rest or Hover"#, key: Some("TextFillColorPrimaryBrush") },
                    ColorBrushInfo { name: "TextFillColorSecondaryBrush", usage: r#"Rest or Hover"#, key: Some("TextFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "TextFillColorTertiaryBrush", usage: r#"Pressed only (not accessible)"#, key: Some("TextFillColorTertiaryBrush") },
                    ColorBrushInfo { name: "TextFillColorDisabledBrush", usage: r#"Disabled only (not accessible)"#, key: Some("TextFillColorDisabledBrush") },
                ],
            },
            ColorGroup {
                name: r#"Accent Text"#,
                description: r#"Recommended for links"#,
                brushes: &[
                    ColorBrushInfo { name: "AccentTextFillColorPrimaryBrush", usage: r#"Rest or Hover"#, key: Some("AccentTextFillColorPrimaryBrush") },
                    ColorBrushInfo { name: "AccentTextFillColorSecondaryBrush", usage: r#"Rest or Hover"#, key: Some("AccentTextFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "AccentTextFillColorTertiaryBrush", usage: r#"Pressed only (not accessible)"#, key: Some("AccentTextFillColorTertiaryBrush") },
                    ColorBrushInfo { name: "AccentTextFillColorDisabledBrush", usage: r#"Disabled only (not accessible)"#, key: Some("AccentTextFillColorDisabledBrush") },
                ],
            },
            ColorGroup {
                name: r#"Text On Accent"#,
                description: r#"Used for text on accent colored controls or fills"#,
                brushes: &[
                    ColorBrushInfo { name: "TextOnAccentFillColorPrimaryBrush", usage: r#"Rest or Hover"#, key: Some("TextOnAccentFillColorPrimaryBrush") },
                    ColorBrushInfo { name: "TextOnAccentFillColorSecondaryBrush", usage: r#"Pressed only (not accessible)"#, key: Some("TextOnAccentFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "TextOnAccentFillColorDisabledBrush", usage: r#"Disabled only (not accessible)"#, key: Some("TextOnAccentFillColorDisabledBrush") },
                    ColorBrushInfo { name: "TextOnAccentFillColorSelectedTextBrush", usage: r#"For highlighted text in text entry experiences"#, key: Some("TextOnAccentFillColorSelectedTextBrush") },
                ],
            },
        ],
    },
    ColorSection {
        title: "Fill",
        groups: &[
            ColorGroup {
                name: r#"Control Fill"#,
                description: r#"Fill used for standard controls"#,
                brushes: &[
                    ColorBrushInfo { name: "ControlFillColorDefaultBrush", usage: r#"Rest"#, key: Some("ControlFillColorDefaultBrush") },
                    ColorBrushInfo { name: "ControlFillColorSecondaryBrush", usage: r#"Hover"#, key: Some("ControlFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "ControlFillColorTertiaryBrush", usage: r#"Pressed"#, key: Some("ControlFillColorTertiaryBrush") },
                    ColorBrushInfo { name: "ControlFillColorQuarternaryBrush", usage: r#"Rest (Pill Button control)"#, key: Some("ControlFillColorQuarternaryBrush") },
                    ColorBrushInfo { name: "ControlFillColorDisabledBrush", usage: r#"Disabled"#, key: Some("ControlFillColorDisabledBrush") },
                    ColorBrushInfo { name: "ControlFillColorTransparentBrush", usage: r#"Rest"#, key: Some("ControlFillColorTransparentBrush") },
                    ColorBrushInfo { name: "ControlFillColorInputActiveBrush", usage: r#"Active/focused text input fields"#, key: Some("ControlFillColorInputActiveBrush") },
                ],
            },
            ColorGroup {
                name: r#"Control Alt Fill"#,
                description: r#"Fill used for the 'off' states of toggle controls"#,
                brushes: &[
                    ColorBrushInfo { name: "ControlAltFillColorTransparentBrush", usage: r#"Transparent"#, key: Some("ControlAltFillColorTransparentBrush") },
                    ColorBrushInfo { name: "ControlAltFillColorSecondaryBrush", usage: r#"Rest"#, key: Some("ControlAltFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "ControlAltFillColorTertiaryBrush", usage: r#"Hover"#, key: Some("ControlAltFillColorTertiaryBrush") },
                    ColorBrushInfo { name: "ControlAltFillColorQuarternaryBrush", usage: r#"Pressed"#, key: Some("ControlAltFillColorQuarternaryBrush") },
                    ColorBrushInfo { name: "ControlAltFillColorDisabledBrush", usage: r#"Disabled"#, key: Some("ControlAltFillColorDisabledBrush") },
                ],
            },
            ColorGroup {
                name: r#"Neutral Solid"#,
                description: r#"Fills used for Slider thumb control to cover the track beneath it"#,
                brushes: &[
                    ColorBrushInfo { name: "ControlSolidFillColorDefaultBrush", usage: r#"Rest"#, key: Some("ControlSolidFillColorDefaultBrush") },
                ],
            },
            ColorGroup {
                name: r#"Neutral Strong"#,
                description: r#"Used for controls that must meet contrast ratio requirements of 3:1"#,
                brushes: &[
                    ColorBrushInfo { name: "ControlStrongFillColorDefaultBrush", usage: r#"Rest or hover"#, key: Some("ControlStrongFillColorDefaultBrush") },
                    ColorBrushInfo { name: "ControlStrongFillColorDisabledBrush", usage: r#"Disabled only (not accessible)"#, key: Some("ControlStrongFillColorDisabledBrush") },
                ],
            },
            ColorGroup {
                name: r#"Subtle Fill"#,
                description: r#"Used for list items and fills that are transparent at rest and appear upon interaction"#,
                brushes: &[
                    ColorBrushInfo { name: "SubtleFillColorTransparentBrush", usage: r#"Rest"#, key: Some("SubtleFillColorTransparentBrush") },
                    ColorBrushInfo { name: "SubtleFillColorSecondaryBrush", usage: r#"Hover"#, key: Some("SubtleFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "SubtleFillColorTertiaryBrush", usage: r#"Pressed"#, key: Some("SubtleFillColorTertiaryBrush") },
                    ColorBrushInfo { name: "SubtleFillColorDisabledBrush", usage: r#"Disabled only (not accessible)"#, key: Some("SubtleFillColorDisabledBrush") },
                ],
            },
            ColorGroup {
                name: r#"Control On Image Fill"#,
                description: r#"Used for controls living on top of imagery"#,
                brushes: &[
                    ColorBrushInfo { name: "ControlOnImageFillColorDefaultBrush", usage: r#"Rest"#, key: Some("ControlOnImageFillColorDefaultBrush") },
                    ColorBrushInfo { name: "ControlOnImageFillColorSecondaryBrush", usage: r#"Hover"#, key: Some("ControlOnImageFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "ControlOnImageFillColorTertiaryBrush", usage: r#"Pressed"#, key: Some("ControlOnImageFillColorTertiaryBrush") },
                    ColorBrushInfo { name: "ControlOnImageFillColorDisabledBrush", usage: r#"Disabled only (not accessible)"#, key: Some("ControlOnImageFillColorDisabledBrush") },
                ],
            },
            ColorGroup {
                name: r#"Accent Fill"#,
                description: r#"Used for accent fills on controls"#,
                brushes: &[
                    ColorBrushInfo { name: "AccentFillColorDefaultBrush", usage: r#"Rest"#, key: Some("AccentFillColorDefaultBrush") },
                    ColorBrushInfo { name: "AccentFillColorSecondaryBrush", usage: r#"Hover"#, key: Some("AccentFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "AccentFillColorTertiaryBrush", usage: r#"Pressed"#, key: Some("AccentFillColorTertiaryBrush") },
                    ColorBrushInfo { name: "AccentFillColorDisabledBrush", usage: r#"Disabled"#, key: Some("AccentFillColorDisabledBrush") },
                    ColorBrushInfo { name: "AccentFillColorSelectedTextBackgroundBrush", usage: r#"Highlighted/selected text background"#, key: Some("AccentFillColorSelectedTextBackgroundBrush") },
                ],
            },
        ],
    },
    ColorSection {
        title: "Stroke",
        groups: &[
            ColorGroup {
                name: r#"Card Stroke"#,
                description: r#"Used for card and layer colors"#,
                brushes: &[
                    ColorBrushInfo { name: "CardStrokeColorDefaultBrush", usage: r#"Card layer and strokes"#, key: Some("CardStrokeColorDefaultBrush") },
                    ColorBrushInfo { name: "CardStrokeColorDefaultSolidBrush", usage: r#"Solid equivalent of Card Stroke Default. Used in command bar for expanded states"#, key: Some("CardStrokeColorDefaultSolidBrush") },
                ],
            },
            ColorGroup {
                name: r#"Control Elevation"#,
                description: r#"Used for standard control strokes and stroke states"#,
                brushes: &[
                    ColorBrushInfo { name: "ControlElevationBorderBrush", usage: r#"Rest"#, key: Some("ControlElevationBorderBrush") },
                    ColorBrushInfo { name: "CircleElevationBorderBrush", usage: r#"Rest"#, key: Some("CircleElevationBorderBrush") },
                    ColorBrushInfo { name: "TextControlElevationBorderBrush", usage: r#"Rest"#, key: Some("TextControlElevationBorderBrush") },
                    ColorBrushInfo { name: "TextControlElevationBorderFocusedBrush", usage: r#"Active text fields"#, key: Some("TextControlElevationBorderFocusedBrush") },
                    ColorBrushInfo { name: "AccentControlElevationBorderBrush", usage: r#"Rest"#, key: Some("AccentControlElevationBorderBrush") },
                ],
            },
            ColorGroup {
                name: r#"Control Stroke"#,
                description: r#"Used for gradient stops in elevation borders and for control states"#,
                brushes: &[
                    ColorBrushInfo { name: "ControlStrokeColorDefaultBrush", usage: r#"Used in Control Elevation Brushes. Pressed or Disabled"#, key: Some("ControlStrokeColorDefaultBrush") },
                    ColorBrushInfo { name: "ControlStrokeColorSecondaryBrush", usage: r#"Used in Control Elevation Brushes"#, key: Some("ControlStrokeColorSecondaryBrush") },
                    ColorBrushInfo { name: "ControlStrokeColorOnAccentDefaultBrush", usage: r#"Used in Control Elevation Brushes. Pressed or Disabled"#, key: Some("ControlStrokeColorOnAccentDefaultBrush") },
                    ColorBrushInfo { name: "ControlStrokeColorOnAccentSecondaryBrush", usage: r#"Used in Control Elevation Brushes"#, key: Some("ControlStrokeColorOnAccentSecondaryBrush") },
                    ColorBrushInfo { name: "ControlStrokeColorOnAccentTertiaryBrush", usage: r#"Linework on Accent controls, ie: dividers"#, key: Some("ControlStrokeColorOnAccentTertiaryBrush") },
                    ColorBrushInfo { name: "ControlStrokeColorOnAccentDisabledBrush", usage: r#"Disabled"#, key: Some("ControlStrokeColorOnAccentDisabledBrush") },
                    ColorBrushInfo { name: "ControlStrokeColorForStrongFillWhenOnImageBrush", usage: r#"When used with a strong fill color, ensures a 3:1 contrast on any background"#, key: Some("ControlStrokeColorForStrongFillWhenOnImageBrush") },
                ],
            },
            ColorGroup {
                name: r#"Control Strong Stroke"#,
                description: r#"Used for control strokes that must meet contrast ratio requirements of 3:1"#,
                brushes: &[
                    ColorBrushInfo { name: "ControlStrongStrokeColorDefaultBrush", usage: r#"3:1 control border"#, key: Some("ControlStrongStrokeColorDefaultBrush") },
                    ColorBrushInfo { name: "ControlStrongStrokeColorDisabledBrush", usage: r#"Disabled"#, key: Some("ControlStrongStrokeColorDisabledBrush") },
                ],
            },
            ColorGroup {
                name: r#"Surface Stroke"#,
                description: r#"Used for strokes on background surfaces, ie: flyouts, windows, dialogs"#,
                brushes: &[
                    ColorBrushInfo { name: "SurfaceStrokeColorDefaultBrush", usage: r#"Window and dialog borders, theme inverse"#, key: Some("SurfaceStrokeColorDefaultBrush") },
                    ColorBrushInfo { name: "SurfaceStrokeColorFlyoutBrush", usage: r#"Control flyouts, always dark"#, key: Some("SurfaceStrokeColorFlyoutBrush") },
                ],
            },
            ColorGroup {
                name: r#"Divider Stroke"#,
                description: r#"Used for divider and graphic lines"#,
                brushes: &[
                    ColorBrushInfo { name: "DividerStrokeColorDefaultBrush", usage: r#"Content dividers"#, key: Some("DividerStrokeColorDefaultBrush") },
                ],
            },
            ColorGroup {
                name: r#"Focus Stroke"#,
                description: r#"Used for focus indicator lines. Theme inverse; dark in light theme and light in dark theme"#,
                brushes: &[
                    ColorBrushInfo { name: "FocusStrokeColorOuterBrush", usage: r#"Outer stroke color"#, key: Some("FocusStrokeColorOuterBrush") },
                    ColorBrushInfo { name: "FocusStrokeColorInnerBrush", usage: r#"Inner stroke color"#, key: Some("FocusStrokeColorInnerBrush") },
                ],
            },
        ],
    },
    ColorSection {
        title: "Background",
        groups: &[
            ColorGroup {
                name: r#"Card Background"#,
                description: r#"Used to create cards - content blocks that live on page and layer backgrounds"#,
                brushes: &[
                    ColorBrushInfo { name: "CardBackgroundFillColorDefaultBrush", usage: r#"Default card color"#, key: Some("CardBackgroundFillColorDefaultBrush") },
                    ColorBrushInfo { name: "CardBackgroundFillColorSecondaryBrush", usage: r#"Alternate card color: slightly darker"#, key: Some("CardBackgroundFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "CardBackgroundTertiaryBrush", usage: r#"Default card hover and pressed color"#, key: Some("CardBackgroundFillColorTertiaryBrush") },
                ],
            },
            ColorGroup {
                name: r#"Smoke Background"#,
                description: r#"Used over windows and desktop to block them out as inaccessible"#,
                brushes: &[
                    ColorBrushInfo { name: "SmokeFillColorDefaultBrush", usage: r#"Dims the background behind dialogs"#, key: Some("SmokeFillColorDefaultBrush") },
                ],
            },
            ColorGroup {
                name: r#"Layer"#,
                description: r#"Used on background colors of any material to create layering"#,
                brushes: &[
                    ColorBrushInfo { name: "LayerFillColorDefaultBrush", usage: r#"Content layer color"#, key: Some("LayerFillColorDefaultBrush") },
                    ColorBrushInfo { name: "LayerFillColorAltBrush", usage: r#"Alternate content layer color"#, key: Some("LayerFillColorAltBrush") },
                ],
            },
            ColorGroup {
                name: r#"Layer on Acrylic"#,
                description: r#"Used on background colors of any material to create layering"#,
                brushes: &[
                    ColorBrushInfo { name: "LayerOnAcrylicFillColorDefaultBrush", usage: r#"Content layer color on acrylic surfaces"#, key: Some("LayerOnAcrylicFillColorDefaultBrush") },
                ],
            },
            ColorGroup {
                name: r#"Layer on Mica Base Alt"#,
                description: r#"Used for fills on Tab control"#,
                brushes: &[
                    ColorBrushInfo { name: "LayerOnMicaBaseAltFillColorDefaultBrush", usage: r#"Active Tab Rest, Content layer"#, key: Some("LayerOnMicaBaseAltFillColorDefaultBrush") },
                    ColorBrushInfo { name: "LayerOnMicaBaseAltFillColorSecondaryBrush", usage: r#"Inactive Tab Hover"#, key: Some("LayerOnMicaBaseAltFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "LayerOnMicaBaseAltFillColorTertiaryBrush", usage: r#"Active Tab Drag"#, key: Some("LayerOnMicaBaseAltFillColorTertiaryBrush") },
                    ColorBrushInfo { name: "LayerOnMicaBaseAltFillColorTransparentBrush", usage: r#"Inactive Tab Rest"#, key: Some("LayerOnMicaBaseAltFillColorTransparentBrush") },
                ],
            },
            ColorGroup {
                name: r#"Solid Background"#,
                description: r#"Solid background colors to place layers, cards or controls on"#,
                brushes: &[
                    ColorBrushInfo { name: "SolidBackgroundFillColorBaseBrush", usage: r#"Used for the bottom most layer of an experience"#, key: Some("SolidBackgroundFillColorBaseBrush") },
                    ColorBrushInfo { name: "SolidBackgroundFillColorBaseAltBrush", usage: r#"Used for the bottom most layer of an experience"#, key: Some("SolidBackgroundFillColorBaseAltBrush") },
                    ColorBrushInfo { name: "SolidBackgroundFillColorSecondaryBrush", usage: r#"Alternate base color for those who need a darker background color"#, key: Some("SolidBackgroundFillColorSecondaryBrush") },
                    ColorBrushInfo { name: "SolidBackgroundFillColorTertiaryBrush", usage: r#"Content layer color"#, key: Some("SolidBackgroundFillColorTertiaryBrush") },
                    ColorBrushInfo { name: "SolidBackgroundFillColorQuarternaryBrush", usage: r#"Alt content layer color"#, key: Some("SolidBackgroundFillColorQuarternaryBrush") },
                    ColorBrushInfo { name: "SolidBackgroundFillColorQuinaryBrush", usage: r#"Used for solid default card colors"#, key: Some("SolidBackgroundFillColorQuinaryBrush") },
                    ColorBrushInfo { name: "SolidBackgroundFillColorSenaryBrush", usage: r#"Used for solid default card color"#, key: Some("SolidBackgroundFillColorSenaryBrush") },
                ],
            },
            ColorGroup {
                name: r#"Mica Background"#,
                description: r#"Mica background colors to place layers, cards, or controls on"#,
                brushes: &[
                    ColorBrushInfo { name: "MicaBackgroundFillColorBaseBrush", usage: r#"Used for the bottom most layer of an experience"#, key: None },
                    ColorBrushInfo { name: "MicaBackgroundFillColorBaseAltBrush", usage: r#"Default tab band background color"#, key: None },
                ],
            },
            ColorGroup {
                name: r#"Acrylic Background"#,
                description: r#"Acrylic background colors to place layers, cards, or controls on"#,
                brushes: &[
                    ColorBrushInfo { name: "AcrylicBackgroundFillColorBaseBrush", usage: r#"Used for the bottom most layer of an acrylic surface only when the surface will use layers"#, key: Some("AcrylicBackgroundFillColorBaseBrush") },
                    ColorBrushInfo { name: "AcrylicBackgroundFillColorDefaultBrush", usage: r#"Default acrylic recipe used for control flyouts and surfaces that live within the context of an app"#, key: Some("AcrylicBackgroundFillColorDefaultBrush") },
                ],
            },
            ColorGroup {
                name: r#"Accent Acrylic Background"#,
                description: r#"Acrylic background colors to place layers, cards, or controls on"#,
                brushes: &[
                    ColorBrushInfo { name: "AccentAcrylicBackgroundFillColorBaseBrush", usage: r#"Used for the bottom most layer of an acrylic surface only when the surface will use layers"#, key: Some("AccentAcrylicBackgroundFillColorBaseBrush") },
                    ColorBrushInfo { name: "AccentAcrylicBackgroundFillColorDefaultBrush", usage: r#"Default acrylic recipe used for control flyouts and surfaces that live within the context of an app"#, key: Some("AccentAcrylicBackgroundFillColorDefaultBrush") },
                ],
            },
        ],
    },
    ColorSection {
        title: "Signal",
        groups: &[
            ColorGroup {
                name: r#"System"#,
                description: r#"Used for status indicators and signal colors"#,
                brushes: &[
                    ColorBrushInfo { name: "SystemFillColorSuccessBrush", usage: r#"Badge"#, key: Some("SystemFillColorSuccessBrush") },
                    ColorBrushInfo { name: "SystemFillColorCautionBrush", usage: r#"Badge"#, key: Some("SystemFillColorCautionBrush") },
                    ColorBrushInfo { name: "SystemFillColorCriticalBrush", usage: r#"Badge"#, key: Some("SystemFillColorCriticalBrush") },
                    ColorBrushInfo { name: "SystemFillColorSuccessBackgroundBrush", usage: r#"Infobar Background"#, key: Some("SystemFillColorSuccessBackgroundBrush") },
                    ColorBrushInfo { name: "SystemFillColorCautionBackgroundBrush", usage: r#"Infobar Background"#, key: Some("SystemFillColorCautionBackgroundBrush") },
                    ColorBrushInfo { name: "SystemFillColorCriticalBackgroundBrush", usage: r#"Infobar Background"#, key: Some("SystemFillColorCriticalBackgroundBrush") },
                    ColorBrushInfo { name: "SystemFillColorAttentionBrush", usage: r#"Badge"#, key: Some("SystemFillColorAttentionBrush") },
                    ColorBrushInfo { name: "SystemFillColorNeutralBrush", usage: r#"Badge"#, key: Some("SystemFillColorNeutralBrush") },
                    ColorBrushInfo { name: "SystemFillColorSolidNeutralBrush", usage: r#"Neutral badges over content"#, key: Some("SystemFillColorSolidNeutralBrush") },
                    ColorBrushInfo { name: "SystemFillColorAttentionBackgroundBrush", usage: r#"Infobar Background"#, key: Some("SystemFillColorAttentionBackgroundBrush") },
                    ColorBrushInfo { name: "SystemFillColorNeutralBackgroundBrush", usage: r#"Infobar Background"#, key: Some("SystemFillColorNeutralBackgroundBrush") },
                    ColorBrushInfo { name: "SystemFillColorSolidNeutralBackgroundBrush", usage: r#"Neutral badges over content"#, key: Some("SystemFillColorSolidNeutralBackgroundBrush") },
                    ColorBrushInfo { name: "SystemFillColorSolidAttentionBackgroundBrush", usage: r#"Solid attention background"#, key: Some("SystemFillColorSolidAttentionBackgroundBrush") },
                ],
            },
        ],
    },
    ColorSection {
        title: "High Contrast",
        groups: &[
            ColorGroup {
                name: r#"High Contrast"#,
                description: r#"System color brushes used in high contrast themes"#,
                brushes: &[
                    ColorBrushInfo { name: "SystemColorWindowTextColor", usage: r#"Foreground / Text color for Headings, body copy, lists, placeholder text, app and window borders, any UI that can't be interacted with"#, key: Some("SystemColorWindowTextColorBrush") },
                    ColorBrushInfo { name: "SystemColorWindowColor", usage: r#"Background of pages, panes, popups, and windows"#, key: Some("SystemColorWindowColorBrush") },
                    ColorBrushInfo { name: "SystemColorHighlightTextColor", usage: r#"Foreground color for text or UI that is selected, interacted with (hover, pressed), or in progress"#, key: Some("SystemColorHighlightTextColorBrush") },
                    ColorBrushInfo { name: "SystemColorHighlightColor", usage: r#"Background or accent color for UI that is selected, interacted with (hover, pressed), or in progress"#, key: Some("SystemColorHighlightColorBrush") },
                    ColorBrushInfo { name: "SystemColorButtonTextColor", usage: r#"Foreground color for buttons and any UI that can be interacted with"#, key: Some("SystemColorButtonTextColorBrush") },
                    ColorBrushInfo { name: "SystemColorButtonFaceColor", usage: r#"Background color for buttons and any UI that can be interacted with"#, key: Some("SystemColorButtonFaceColorBrush") },
                    ColorBrushInfo { name: "SystemColorHotlightColor", usage: r#"Foreground / Text color for hyperlink text"#, key: Some("SystemColorHotlightColorBrush") },
                    ColorBrushInfo { name: "SystemColorGrayTextColor", usage: r#"Foreground / Text color for Inactive (disabled) UI"#, key: Some("SystemColorGrayTextColorBrush") },
                ],
            },
        ],
    },
];
