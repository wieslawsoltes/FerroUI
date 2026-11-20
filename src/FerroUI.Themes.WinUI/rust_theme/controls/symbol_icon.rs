//! `Controls/SymbolIcon.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: WinUI's SymbolIcon has no template in microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0). The control builds its visual in code
//! (CSymbolIcon::ApplyTemplate, dxaml/xcp/core/core/elements/icon.cpp): one TextBlock, stretched
//! horizontally and centred vertically, its text centred, 20 pixels, normal style and weight, in
//! the font Segoe Fluent Icons, with the accessibility view Raw. FerroUI's SymbolIcon is a
//! templated control (IconElement), so the theme supplies that text block as the part PART_Text,
//! and the control gives it the glyph of its symbol and its foreground. The font is the theme
//! resource SymbolThemeFontFamily (Segoe Fluent Icons, Segoe MDL2 Assets) where WinUI names Segoe
//! Fluent Icons in code, so that an application can supply another symbol font
//! (docs/porting/winui-controls.md, section 5.2).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::FontStyle;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextAlignment;
use ferroui_controls::automation::AccessibilityView;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::Control;
use ferroui_controls::SymbolIcon;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<SymbolIcon>(), symbol_icon_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`: the document included
/// (`ResourceInclude`) rather than merged.
#[allow(dead_code)]
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `SymbolIcon` of `symbol_icon_theme`.
fn symbol_icon_theme_template(t: &Template<SymbolIcon>) -> Result<Ref<Control>> {
    Ok(t.make(TextBlock::new())
        .name("PART_Text")?
        .set(
            Layoutable::horizontal_alignment_property(),
            HorizontalAlignment::Stretch,
        )
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
        .set(TextBlock::text_alignment_property(), TextAlignment::Center)
        .dynamic(TextBlock::font_family_property(), "SymbolThemeFontFamily")
        .set(TextBlock::font_size_property(), 20.0)
        .set(TextBlock::font_style_property(), FontStyle::Normal)
        .set(TextBlock::font_weight_property(), FontWeight::Normal)
        .set(
            AutomationProperties::accessibility_view_property(),
            AccessibilityView::Raw,
        )
        .control())
}

/// The default control theme of `SymbolIcon`.
fn symbol_icon_theme(theme: &StyleBuilder) -> Result {
    theme.target::<SymbolIcon>();
    theme.template(symbol_icon_theme_template);
    Ok(())
}
