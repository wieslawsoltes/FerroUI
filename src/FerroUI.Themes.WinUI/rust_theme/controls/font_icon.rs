//! `Controls/FontIcon.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: WinUI's FontIcon has no template in microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0). The control builds its visual in code
//! (CFontIcon::ApplyTemplate, dxaml/xcp/core/core/elements/icon.cpp): one TextBlock, stretched
//! horizontally and centred vertically, its text centred and the glyph of the icon as its text,
//! with the font of the icon, and the accessibility view Raw so that a screen reader does not read
//! the glyph. FerroUI's FontIcon is a templated control (IconElement), so the theme supplies that
//! text block as the part PART_Text, and the control gives it the glyph, the font and the
//! foreground of the icon (docs/porting/winui-controls.md, FontIcon). The font family of a font
//! icon is the theme resource SymbolThemeFontFamily, as WinUI's default font of a font icon; the
//! size, 20, is the default of the control.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::TextAlignment;
use ferroui_controls::automation::AccessibilityView;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Control;
use ferroui_controls::FontIcon;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<FontIcon>(), font_icon_theme);
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

/// The template of `FontIcon` of `font_icon_theme`.
fn font_icon_theme_template(t: &Template<FontIcon>) -> Result<Ref<Control>> {
    Ok(t.make(TextBlock::new())
        .name("PART_Text")?
        .set(
            Layoutable::horizontal_alignment_property(),
            HorizontalAlignment::Stretch,
        )
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
        .set(TextBlock::text_alignment_property(), TextAlignment::Center)
        .set(
            AutomationProperties::accessibility_view_property(),
            AccessibilityView::Raw,
        )
        .control())
}

/// The default control theme of `FontIcon`.
fn font_icon_theme(theme: &StyleBuilder) -> Result {
    theme.target::<FontIcon>();
    theme.dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily");
    // FontIcon.FontSize is a property of its own in WinUI, 20 by default and not inherited;
    // FerroUI's is the inherited font size of the templated control, so the theme sets 20 over
    // the inherited value (FI-3).
    theme.set(TemplatedControl::font_size_property(), 20.0);
    theme.template(font_icon_theme_template);
    Ok(())
}
