// Generated from unchanged RibbonThemeResources.cs. Do not edit.
use super::*;
pub const BRUSH_KEYS: &[&str] = &[
    "RibbonChromeBackgroundBrush",
    "RibbonChromeForegroundBrush",
    "RibbonCommandBarBackgroundBrush",
    "RibbonCommandBarBorderBrush",
    "RibbonForegroundBrush",
    "RibbonSecondaryForegroundBrush",
    "RibbonDisabledForegroundBrush",
    "RibbonIconBrush",
    "RibbonAccentBrush",
    "RibbonAccentForegroundBrush",
    "RibbonAccentSubtleBrush",
    "RibbonAccentSubtleStrongBrush",
    "RibbonAccentTextBrush",
    "RibbonItemHoverBrush",
    "RibbonItemPressedBrush",
    "RibbonItemCheckedBrush",
    "RibbonItemCheckedHoverBrush",
    "RibbonItemCheckedBorderBrush",
    "RibbonItemBorderHoverBrush",
    "RibbonSeparatorBrush",
    "RibbonTabForegroundBrush",
    "RibbonTabHoverBrush",
    "RibbonTabSelectedForegroundBrush",
    "RibbonTabIndicatorBrush",
    "RibbonPopupBackgroundBrush",
    "RibbonPopupBorderBrush",
    "RibbonInputBackgroundBrush",
    "RibbonInputBorderBrush",
    "RibbonInputHoverBorderBrush",
    "RibbonInputFocusBorderBrush",
    "RibbonKeyTipBackgroundBrush",
    "RibbonKeyTipForegroundBrush",
    "RibbonKeyTipBorderBrush",
    "RibbonTitleBarBackgroundBrush",
    "RibbonTitleBarForegroundBrush",
    "RibbonTitleBarHoverBrush",
    "RibbonTitleBarIconBrush",
    "RibbonBackstagePaneBackgroundBrush",
    "RibbonBackstagePaneForegroundBrush",
    "RibbonBackstagePaneHoverBrush",
    "RibbonBackstagePaneSelectedBrush",
    "RibbonBackstageContentBackgroundBrush",
    "RibbonGalleryItemBorderBrush",
    "RibbonGalleryItemSelectedBorderBrush",
    "RibbonScreenTipBackgroundBrush",
    "RibbonScreenTipBorderBrush",
    "RibbonFocusBrush",
    "RibbonWindowBackgroundBrush",
    "RibbonStatusBarBackgroundBrush",
    "RibbonStatusBarForegroundBrush",
    "RibbonToolBarBackgroundBrush",
    "RibbonSearchBackgroundBrush",
    "RibbonSearchBorderBrush",
    "RibbonSwatchBorderBrush",
    "RibbonShadowBrush",
    "RibbonScrollButtonBackgroundBrush",
    "RibbonGroupCaptionBackgroundBrush",
    "RibbonGroupCaptionForegroundBrush",
    "RibbonTabSelectedBackgroundBrush",
    "RibbonFloatingPanelBarBrush",
];
pub const SHAPE_KEYS: &[&str] = &[
    "RibbonControlCornerRadius",
    "RibbonCommandBarCornerRadius",
    "RibbonPopupCornerRadius",
    "RibbonTabCornerRadius",
    "RibbonGroupCaptionCornerRadius",
    "RibbonCommandBarMargin",
    "RibbonCommandBarBorderThickness",
    "RibbonTabMargin",
    "RibbonTabRowPadding",
    "RibbonGroupCaptionMargin",
];
impl RibbonThemeResources {
    pub(super) fn apply_light(&self, palette: &RibbonThemePalette, chrome: RibbonChromeStyle) {
        let accent = palette.get_accent(false);
        let colorful = chrome == RibbonChromeStyle::Colorful;
        self.set_brush(&self.light, "RibbonWindowBackgroundBrush", c("#F5F5F5"));
        self.set_brush(
            &self.light,
            "RibbonChromeBackgroundBrush",
            if colorful { accent } else { c("#F5F5F5") },
        );
        self.set_brush(
            &self.light,
            "RibbonChromeForegroundBrush",
            if colorful {
                palette.get_on_accent(false)
            } else {
                c("#242424")
            },
        );
        self.set_brush(&self.light, "RibbonCommandBarBackgroundBrush", c("#FFFFFF"));
        self.set_brush(&self.light, "RibbonCommandBarBorderBrush", c("#E0E0E0"));
        self.set_brush(&self.light, "RibbonForegroundBrush", c("#242424"));
        self.set_brush(&self.light, "RibbonSecondaryForegroundBrush", c("#616161"));
        self.set_brush(&self.light, "RibbonDisabledForegroundBrush", c("#BDBDBD"));
        self.set_brush(&self.light, "RibbonIconBrush", c("#424242"));
        self.set_brush(&self.light, "RibbonAccentBrush", accent);
        self.set_brush(
            &self.light,
            "RibbonAccentForegroundBrush",
            palette.get_on_accent(false),
        );
        self.set_brush(
            &self.light,
            "RibbonAccentSubtleBrush",
            palette.get_accent_subtle(false),
        );
        self.set_brush(
            &self.light,
            "RibbonAccentSubtleStrongBrush",
            palette.get_accent_subtle_strong(false),
        );
        self.set_brush(&self.light, "RibbonAccentTextBrush", accent.darken(0.1));
        self.set_brush(&self.light, "RibbonItemHoverBrush", c("#F0F0F0"));
        self.set_brush(&self.light, "RibbonItemPressedBrush", c("#E0E0E0"));
        self.set_brush(
            &self.light,
            "RibbonItemCheckedBrush",
            palette.get_accent_subtle(false),
        );
        self.set_brush(
            &self.light,
            "RibbonItemCheckedHoverBrush",
            palette.get_accent_subtle_strong(false),
        );
        self.set_brush(
            &self.light,
            "RibbonItemCheckedBorderBrush",
            accent.lighten(0.55),
        );
        self.set_brush(&self.light, "RibbonItemBorderHoverBrush", c("#00000000"));
        self.set_brush(&self.light, "RibbonSeparatorBrush", c("#E0E0E0"));
        self.set_brush(
            &self.light,
            "RibbonTabForegroundBrush",
            if colorful {
                palette.get_on_accent(false)
            } else {
                c("#424242")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonTabHoverBrush",
            if colorful {
                accent.lighten(0.15)
            } else {
                c("#E8E8E8")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonTabSelectedForegroundBrush",
            if colorful {
                palette.get_on_accent(false)
            } else {
                accent
            },
        );
        self.set_brush(
            &self.light,
            "RibbonTabIndicatorBrush",
            if colorful {
                palette.get_on_accent(false)
            } else {
                accent
            },
        );
        self.set_brush(&self.light, "RibbonPopupBackgroundBrush", c("#FFFFFF"));
        self.set_brush(&self.light, "RibbonPopupBorderBrush", c("#D1D1D1"));
        self.set_brush(&self.light, "RibbonInputBackgroundBrush", c("#FFFFFF"));
        self.set_brush(&self.light, "RibbonInputBorderBrush", c("#D1D1D1"));
        self.set_brush(&self.light, "RibbonInputHoverBorderBrush", c("#A6A6A6"));
        self.set_brush(&self.light, "RibbonInputFocusBorderBrush", accent);
        self.set_brush(&self.light, "RibbonKeyTipBackgroundBrush", c("#3B3B3B"));
        self.set_brush(&self.light, "RibbonKeyTipForegroundBrush", c("#FFFFFF"));
        self.set_brush(&self.light, "RibbonKeyTipBorderBrush", c("#1F1F1F"));
        self.set_brush(
            &self.light,
            "RibbonTitleBarBackgroundBrush",
            if colorful { accent } else { c("#F5F5F5") },
        );
        self.set_brush(
            &self.light,
            "RibbonTitleBarForegroundBrush",
            if colorful {
                palette.get_on_accent(false)
            } else {
                c("#242424")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonTitleBarHoverBrush",
            if colorful {
                accent.lighten(0.15)
            } else {
                c("#E8E8E8")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonTitleBarIconBrush",
            if colorful {
                palette.get_on_accent(false)
            } else {
                accent
            },
        );
        self.set_brush(&self.light, "RibbonBackstagePaneBackgroundBrush", accent);
        self.set_brush(
            &self.light,
            "RibbonBackstagePaneForegroundBrush",
            palette.get_on_accent(false),
        );
        self.set_brush(
            &self.light,
            "RibbonBackstagePaneHoverBrush",
            accent.lighten(0.15),
        );
        self.set_brush(
            &self.light,
            "RibbonBackstagePaneSelectedBrush",
            accent.darken(0.25),
        );
        self.set_brush(
            &self.light,
            "RibbonBackstageContentBackgroundBrush",
            c("#FFFFFF"),
        );
        self.set_brush(&self.light, "RibbonGalleryItemBorderBrush", c("#E0E0E0"));
        self.set_brush(&self.light, "RibbonGalleryItemSelectedBorderBrush", accent);
        self.set_brush(&self.light, "RibbonScreenTipBackgroundBrush", c("#FFFFFF"));
        self.set_brush(&self.light, "RibbonScreenTipBorderBrush", c("#C7C7C7"));
        self.set_brush(&self.light, "RibbonFocusBrush", c("#000000"));
        self.set_brush(&self.light, "RibbonStatusBarBackgroundBrush", c("#F5F5F5"));
        self.set_brush(&self.light, "RibbonStatusBarForegroundBrush", c("#424242"));
        self.set_brush(&self.light, "RibbonToolBarBackgroundBrush", c("#FAFAFA"));
        self.set_brush(
            &self.light,
            "RibbonSearchBackgroundBrush",
            if colorful {
                accent.lighten(0.2)
            } else {
                c("#FFFFFF")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonSearchBorderBrush",
            if colorful {
                accent.lighten(0.3)
            } else {
                c("#D1D1D1")
            },
        );
        self.set_brush(&self.light, "RibbonSwatchBorderBrush", c("#33000000"));
        self.set_brush(&self.light, "RibbonShadowBrush", c("#1A000000"));
        self.set_brush(
            &self.light,
            "RibbonScrollButtonBackgroundBrush",
            c("#F2FFFFFF"),
        );
        self.set_brush(
            &self.light,
            "RibbonGroupCaptionBackgroundBrush",
            c("#00FFFFFF"),
        );
        self.set_brush(
            &self.light,
            "RibbonGroupCaptionForegroundBrush",
            c("#616161"),
        );
        self.set_brush(
            &self.light,
            "RibbonTabSelectedBackgroundBrush",
            c("#00FFFFFF"),
        );
        self.set_brush(&self.light, "RibbonFloatingPanelBarBrush", c("#F0F0F0"));
    }
    pub(super) fn apply_dark(&self, palette: &RibbonThemePalette, chrome: RibbonChromeStyle) {
        let accent = palette.get_accent(true);
        let colorful = chrome == RibbonChromeStyle::Colorful;
        let base_accent = palette.get_accent(false);
        self.set_brush(&self.dark, "RibbonWindowBackgroundBrush", c("#1F1F1F"));
        self.set_brush(
            &self.dark,
            "RibbonChromeBackgroundBrush",
            if colorful {
                base_accent.darken(0.35)
            } else {
                c("#1F1F1F")
            },
        );
        self.set_brush(&self.dark, "RibbonChromeForegroundBrush", c("#FFFFFF"));
        self.set_brush(&self.dark, "RibbonCommandBarBackgroundBrush", c("#292929"));
        self.set_brush(&self.dark, "RibbonCommandBarBorderBrush", c("#3D3D3D"));
        self.set_brush(&self.dark, "RibbonForegroundBrush", c("#FFFFFF"));
        self.set_brush(&self.dark, "RibbonSecondaryForegroundBrush", c("#C7C7C7"));
        self.set_brush(&self.dark, "RibbonDisabledForegroundBrush", c("#6E6E6E"));
        self.set_brush(&self.dark, "RibbonIconBrush", c("#E0E0E0"));
        self.set_brush(&self.dark, "RibbonAccentBrush", accent);
        self.set_brush(
            &self.dark,
            "RibbonAccentForegroundBrush",
            palette.get_on_accent(true),
        );
        self.set_brush(
            &self.dark,
            "RibbonAccentSubtleBrush",
            palette.get_accent_subtle(true),
        );
        self.set_brush(
            &self.dark,
            "RibbonAccentSubtleStrongBrush",
            palette.get_accent_subtle_strong(true),
        );
        self.set_brush(&self.dark, "RibbonAccentTextBrush", accent);
        self.set_brush(&self.dark, "RibbonItemHoverBrush", c("#383838"));
        self.set_brush(&self.dark, "RibbonItemPressedBrush", c("#454545"));
        self.set_brush(
            &self.dark,
            "RibbonItemCheckedBrush",
            palette.get_accent_subtle(true),
        );
        self.set_brush(
            &self.dark,
            "RibbonItemCheckedHoverBrush",
            palette.get_accent_subtle_strong(true),
        );
        self.set_brush(
            &self.dark,
            "RibbonItemCheckedBorderBrush",
            accent.with_alpha(0x88),
        );
        self.set_brush(&self.dark, "RibbonItemBorderHoverBrush", c("#00000000"));
        self.set_brush(&self.dark, "RibbonSeparatorBrush", c("#454545"));
        self.set_brush(&self.dark, "RibbonTabForegroundBrush", c("#D6D6D6"));
        self.set_brush(&self.dark, "RibbonTabHoverBrush", c("#2E2E2E"));
        self.set_brush(&self.dark, "RibbonTabSelectedForegroundBrush", accent);
        self.set_brush(&self.dark, "RibbonTabIndicatorBrush", accent);
        self.set_brush(&self.dark, "RibbonPopupBackgroundBrush", c("#2B2B2B"));
        self.set_brush(&self.dark, "RibbonPopupBorderBrush", c("#474747"));
        self.set_brush(&self.dark, "RibbonInputBackgroundBrush", c("#1F1F1F"));
        self.set_brush(&self.dark, "RibbonInputBorderBrush", c("#5C5C5C"));
        self.set_brush(&self.dark, "RibbonInputHoverBorderBrush", c("#8A8A8A"));
        self.set_brush(&self.dark, "RibbonInputFocusBorderBrush", accent);
        self.set_brush(&self.dark, "RibbonKeyTipBackgroundBrush", c("#F0F0F0"));
        self.set_brush(&self.dark, "RibbonKeyTipForegroundBrush", c("#1F1F1F"));
        self.set_brush(&self.dark, "RibbonKeyTipBorderBrush", c("#FFFFFF"));
        self.set_brush(
            &self.dark,
            "RibbonTitleBarBackgroundBrush",
            if colorful {
                base_accent.darken(0.35)
            } else {
                c("#1F1F1F")
            },
        );
        self.set_brush(&self.dark, "RibbonTitleBarForegroundBrush", c("#FFFFFF"));
        self.set_brush(&self.dark, "RibbonTitleBarHoverBrush", c("#383838"));
        self.set_brush(
            &self.dark,
            "RibbonTitleBarIconBrush",
            if colorful { c("#FFFFFF") } else { accent },
        );
        self.set_brush(
            &self.dark,
            "RibbonBackstagePaneBackgroundBrush",
            c("#141414"),
        );
        self.set_brush(
            &self.dark,
            "RibbonBackstagePaneForegroundBrush",
            c("#FFFFFF"),
        );
        self.set_brush(&self.dark, "RibbonBackstagePaneHoverBrush", c("#2E2E2E"));
        self.set_brush(
            &self.dark,
            "RibbonBackstagePaneSelectedBrush",
            accent.with_alpha(0x55),
        );
        self.set_brush(
            &self.dark,
            "RibbonBackstageContentBackgroundBrush",
            c("#1F1F1F"),
        );
        self.set_brush(&self.dark, "RibbonGalleryItemBorderBrush", c("#454545"));
        self.set_brush(&self.dark, "RibbonGalleryItemSelectedBorderBrush", accent);
        self.set_brush(&self.dark, "RibbonScreenTipBackgroundBrush", c("#2B2B2B"));
        self.set_brush(&self.dark, "RibbonScreenTipBorderBrush", c("#5C5C5C"));
        self.set_brush(&self.dark, "RibbonFocusBrush", c("#FFFFFF"));
        self.set_brush(&self.dark, "RibbonStatusBarBackgroundBrush", c("#1F1F1F"));
        self.set_brush(&self.dark, "RibbonStatusBarForegroundBrush", c("#D6D6D6"));
        self.set_brush(&self.dark, "RibbonToolBarBackgroundBrush", c("#262626"));
        self.set_brush(&self.dark, "RibbonSearchBackgroundBrush", c("#2E2E2E"));
        self.set_brush(&self.dark, "RibbonSearchBorderBrush", c("#474747"));
        self.set_brush(&self.dark, "RibbonSwatchBorderBrush", c("#44FFFFFF"));
        self.set_brush(&self.dark, "RibbonShadowBrush", c("#66000000"));
        self.set_brush(
            &self.dark,
            "RibbonScrollButtonBackgroundBrush",
            c("#F2292929"),
        );
        self.set_brush(
            &self.dark,
            "RibbonGroupCaptionBackgroundBrush",
            c("#00000000"),
        );
        self.set_brush(
            &self.dark,
            "RibbonGroupCaptionForegroundBrush",
            c("#C7C7C7"),
        );
        self.set_brush(
            &self.dark,
            "RibbonTabSelectedBackgroundBrush",
            c("#00000000"),
        );
        self.set_brush(&self.dark, "RibbonFloatingPanelBarBrush", c("#333333"));
    }
    pub(super) fn apply_cad_light(&self, palette: &RibbonThemePalette, chrome: RibbonChromeStyle) {
        let accent = palette.get_accent(false);
        let colorful = chrome == RibbonChromeStyle::Colorful;
        self.set_brush(&self.light, "RibbonWindowBackgroundBrush", c("#D5D8DD"));
        self.set_brush(
            &self.light,
            "RibbonChromeBackgroundBrush",
            if colorful { accent } else { c("#DADDE2") },
        );
        self.set_brush(
            &self.light,
            "RibbonChromeForegroundBrush",
            if colorful {
                palette.get_on_accent(false)
            } else {
                c("#1F242B")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonTitleBarBackgroundBrush",
            if colorful { accent } else { c("#CDD1D7") },
        );
        self.set_brush(
            &self.light,
            "RibbonTitleBarForegroundBrush",
            if colorful {
                palette.get_on_accent(false)
            } else {
                c("#1F242B")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonTitleBarHoverBrush",
            if colorful {
                accent.lighten(0.15)
            } else {
                c("#BFC5CD")
            },
        );
        self.set_brush(&self.light, "RibbonCommandBarBackgroundBrush", c("#F0F1F3"));
        self.set_brush(&self.light, "RibbonCommandBarBorderBrush", c("#B7BEC8"));
        self.set_brush(&self.light, "RibbonForegroundBrush", c("#1F242B"));
        self.set_brush(&self.light, "RibbonSecondaryForegroundBrush", c("#4E5866"));
        self.set_brush(&self.light, "RibbonDisabledForegroundBrush", c("#9AA3AF"));
        self.set_brush(&self.light, "RibbonIconBrush", c("#2F3742"));
        self.set_brush(&self.light, "RibbonItemHoverBrush", c("#DDE3EA"));
        self.set_brush(&self.light, "RibbonItemPressedBrush", c("#CBD3DD"));
        self.set_brush(&self.light, "RibbonItemBorderHoverBrush", c("#A9B4C2"));
        self.set_brush(&self.light, "RibbonSeparatorBrush", c("#C7CCD3"));
        self.set_brush(
            &self.light,
            "RibbonTabForegroundBrush",
            if colorful {
                palette.get_on_accent(false)
            } else {
                c("#2F3742")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonTabHoverBrush",
            if colorful {
                accent.lighten(0.15)
            } else {
                c("#E3E6EA")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonTabSelectedForegroundBrush",
            c("#000000"),
        );
        self.set_brush(
            &self.light,
            "RibbonTabSelectedBackgroundBrush",
            c("#F0F1F3"),
        );
        self.set_brush(&self.light, "RibbonTabIndicatorBrush", c("#00000000"));
        self.set_brush(&self.light, "RibbonPopupBackgroundBrush", c("#F7F8F9"));
        self.set_brush(&self.light, "RibbonPopupBorderBrush", c("#A9B4C2"));
        self.set_brush(&self.light, "RibbonInputBorderBrush", c("#B7BEC8"));
        self.set_brush(&self.light, "RibbonInputHoverBorderBrush", c("#8C97A5"));
        self.set_brush(&self.light, "RibbonInputFocusBorderBrush", accent);
        self.set_brush(
            &self.light,
            "RibbonBackstageContentBackgroundBrush",
            c("#F0F1F3"),
        );
        self.set_brush(&self.light, "RibbonGalleryItemBorderBrush", c("#C7CCD3"));
        self.set_brush(&self.light, "RibbonScreenTipBackgroundBrush", c("#F7F8F9"));
        self.set_brush(&self.light, "RibbonScreenTipBorderBrush", c("#A9B4C2"));
        self.set_brush(&self.light, "RibbonStatusBarBackgroundBrush", c("#D5D8DD"));
        self.set_brush(&self.light, "RibbonStatusBarForegroundBrush", c("#2F3742"));
        self.set_brush(&self.light, "RibbonToolBarBackgroundBrush", c("#F0F1F3"));
        self.set_brush(
            &self.light,
            "RibbonSearchBackgroundBrush",
            if colorful {
                accent.lighten(0.2)
            } else {
                c("#FFFFFF")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonSearchBorderBrush",
            if colorful {
                accent.lighten(0.3)
            } else {
                c("#B7BEC8")
            },
        );
        self.set_brush(
            &self.light,
            "RibbonScrollButtonBackgroundBrush",
            c("#F2F0F1F3"),
        );
        self.set_brush(
            &self.light,
            "RibbonGroupCaptionBackgroundBrush",
            c("#DCE0E5"),
        );
        self.set_brush(
            &self.light,
            "RibbonGroupCaptionForegroundBrush",
            c("#3F4855"),
        );
        self.set_brush(&self.light, "RibbonFloatingPanelBarBrush", c("#DCE0E5"));
    }
    pub(super) fn apply_cad_dark(&self, palette: &RibbonThemePalette, chrome: RibbonChromeStyle) {
        let accent = palette.get_accent(true);
        let colorful = chrome == RibbonChromeStyle::Colorful;
        self.set_brush(&self.dark, "RibbonWindowBackgroundBrush", c("#2B313B"));
        self.set_brush(
            &self.dark,
            "RibbonChromeBackgroundBrush",
            if colorful {
                palette.get_accent(false).darken(0.35)
            } else {
                c("#2B313B")
            },
        );
        self.set_brush(&self.dark, "RibbonChromeForegroundBrush", c("#E1E6EC"));
        self.set_brush(
            &self.dark,
            "RibbonTitleBarBackgroundBrush",
            if colorful {
                palette.get_accent(false).darken(0.35)
            } else {
                c("#252A33")
            },
        );
        self.set_brush(&self.dark, "RibbonTitleBarForegroundBrush", c("#E1E6EC"));
        self.set_brush(&self.dark, "RibbonTitleBarHoverBrush", c("#3B4453"));
        self.set_brush(&self.dark, "RibbonCommandBarBackgroundBrush", c("#3B4453"));
        self.set_brush(&self.dark, "RibbonCommandBarBorderBrush", c("#252A33"));
        self.set_brush(&self.dark, "RibbonForegroundBrush", c("#E1E6EC"));
        self.set_brush(&self.dark, "RibbonSecondaryForegroundBrush", c("#AEB8C4"));
        self.set_brush(&self.dark, "RibbonDisabledForegroundBrush", c("#6C7787"));
        self.set_brush(&self.dark, "RibbonIconBrush", c("#D8DEE6"));
        self.set_brush(&self.dark, "RibbonItemHoverBrush", c("#4A5568"));
        self.set_brush(&self.dark, "RibbonItemPressedBrush", c("#56637A"));
        self.set_brush(&self.dark, "RibbonItemBorderHoverBrush", c("#5F6E86"));
        self.set_brush(&self.dark, "RibbonSeparatorBrush", c("#2F3641"));
        self.set_brush(&self.dark, "RibbonTabForegroundBrush", c("#C9D1DB"));
        self.set_brush(&self.dark, "RibbonTabHoverBrush", c("#343C48"));
        self.set_brush(&self.dark, "RibbonTabSelectedForegroundBrush", c("#FFFFFF"));
        self.set_brush(&self.dark, "RibbonTabSelectedBackgroundBrush", c("#3B4453"));
        self.set_brush(&self.dark, "RibbonTabIndicatorBrush", c("#00000000"));
        self.set_brush(&self.dark, "RibbonPopupBackgroundBrush", c("#3B4453"));
        self.set_brush(&self.dark, "RibbonPopupBorderBrush", c("#252A33"));
        self.set_brush(&self.dark, "RibbonInputBackgroundBrush", c("#2B313B"));
        self.set_brush(&self.dark, "RibbonInputBorderBrush", c("#56637A"));
        self.set_brush(&self.dark, "RibbonInputHoverBorderBrush", c("#7A889E"));
        self.set_brush(&self.dark, "RibbonInputFocusBorderBrush", accent);
        self.set_brush(&self.dark, "RibbonKeyTipBackgroundBrush", c("#E8EDF2"));
        self.set_brush(&self.dark, "RibbonKeyTipForegroundBrush", c("#1B2027"));
        self.set_brush(&self.dark, "RibbonKeyTipBorderBrush", c("#FFFFFF"));
        self.set_brush(
            &self.dark,
            "RibbonBackstagePaneBackgroundBrush",
            c("#252A33"),
        );
        self.set_brush(&self.dark, "RibbonBackstagePaneHoverBrush", c("#343C48"));
        self.set_brush(
            &self.dark,
            "RibbonBackstageContentBackgroundBrush",
            c("#2B313B"),
        );
        self.set_brush(&self.dark, "RibbonGalleryItemBorderBrush", c("#4A5568"));
        self.set_brush(&self.dark, "RibbonScreenTipBackgroundBrush", c("#3B4453"));
        self.set_brush(&self.dark, "RibbonScreenTipBorderBrush", c("#252A33"));
        self.set_brush(&self.dark, "RibbonStatusBarBackgroundBrush", c("#2B313B"));
        self.set_brush(&self.dark, "RibbonStatusBarForegroundBrush", c("#C9D1DB"));
        self.set_brush(&self.dark, "RibbonToolBarBackgroundBrush", c("#3B4453"));
        self.set_brush(&self.dark, "RibbonSearchBackgroundBrush", c("#2B313B"));
        self.set_brush(&self.dark, "RibbonSearchBorderBrush", c("#4A5568"));
        self.set_brush(
            &self.dark,
            "RibbonScrollButtonBackgroundBrush",
            c("#F23B4453"),
        );
        self.set_brush(
            &self.dark,
            "RibbonGroupCaptionBackgroundBrush",
            c("#323A47"),
        );
        self.set_brush(
            &self.dark,
            "RibbonGroupCaptionForegroundBrush",
            c("#B8C2CE"),
        );
        self.set_brush(&self.dark, "RibbonFloatingPanelBarBrush", c("#2F3641"));
    }
}
