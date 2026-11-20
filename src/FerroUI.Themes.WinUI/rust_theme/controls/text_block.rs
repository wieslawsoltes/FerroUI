//! `Controls/TextBlock.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/TextBlock_themeresources.xaml (the styles; its resources are
//!   Resources/TextBlock_themeresources.xaml)
//!   controls/dev/CommonStyles/TextBlock_themeresources_v2.5.xaml (BodyStrongTextBlockStyle,
//!   TitleLargeTextBlockStyle and DisplayTextBlockStyle, which replace the ones of the document
//!   above in the merged theme; its resources are Resources/TextBlock_themeresources_v2.5.xaml)
//!   dxaml/xcp/dxaml/themes/generic.xaml (HeaderTextBlockStyle and SubheaderTextBlockStyle, the
//!   two styles of the type ramp of the platform theme that the controls library does not replace)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The keyed styles of TextBlock are keyed control themes of TextBlock: a WinUI application writes
//! Style="{StaticResource BodyTextBlockStyle}", a FerroUI one Theme="{StaticResource
//! BodyTextBlockStyle}" (docs/porting/winui-theme.md, section 4.2). The font family
//! XamlAutoFontFamily is the value of ContentControlThemeFontFamily (section 3.5).
//! TextLineBounds is FerroUI's TextBlock.TextLineBounds (an addition after WinUI's).
//! LineStackingStrategy and OpticalMarginAlignment have no counterpart in FerroUI's TextBlock and
//! are left out.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::FontFamily;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextLineBounds;
use ferroui_base::media::TextTrimming;
use ferroui_base::media::TextWrapping;
use ferroui_controls::TextBlock;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("BaseTextBlockStyle", base_text_block_style);
    resources.control_theme("CaptionTextBlockStyle", caption_text_block_style);
    resources.control_theme("BodyTextBlockStyle", body_text_block_style);
    // TextBlock_themeresources_v2.5.xaml
    resources.control_theme("BodyStrongTextBlockStyle", body_strong_text_block_style);
    resources.control_theme("BodyLargeTextBlockStyle", body_large_text_block_style);
    resources.control_theme("BodyLargeStrongTextBlockStyle", body_large_strong_text_block_style);
    resources.control_theme("SubtitleTextBlockStyle", subtitle_text_block_style);
    resources.control_theme("TitleTextBlockStyle", title_text_block_style);
    // TextBlock_themeresources_v2.5.xaml
    resources.control_theme("TitleLargeTextBlockStyle", title_large_text_block_style);
    // TextBlock_themeresources_v2.5.xaml
    resources.control_theme("DisplayTextBlockStyle", display_text_block_style);
    // generic.xaml
    resources.control_theme("HeaderTextBlockStyle", header_text_block_style);
    resources.control_theme("SubheaderTextBlockStyle", subheader_text_block_style);
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

/// `BaseTextBlockStyle`: a control theme of `TextBlock`.
fn base_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.set(
        TextBlock::font_family_property(),
        literal(FontFamily::parse("Segoe UI Variable Text, Segoe UI, $Default"))?,
    );
    theme.static_resource(TextBlock::font_size_property(), "BodyTextBlockFontSize")?;
    theme.set(TextBlock::font_weight_property(), FontWeight::SemiBold);
    theme.set(
        TextBlock::text_trimming_property(),
        <dyn TextTrimming>::character_ellipsis(),
    );
    theme.set(TextBlock::text_wrapping_property(), TextWrapping::Wrap);
    theme.set(TextBlock::text_line_bounds_property(), TextLineBounds::Full);
    Ok(())
}

/// `CaptionTextBlockStyle`: a control theme of `TextBlock`.
fn caption_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.static_resource(TextBlock::font_size_property(), "CaptionTextBlockFontSize")?;
    theme.set(TextBlock::font_weight_property(), FontWeight::Normal);
    Ok(())
}

/// `BodyTextBlockStyle`: a control theme of `TextBlock`.
fn body_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.set(TextBlock::font_weight_property(), FontWeight::Normal);
    Ok(())
}

/// `BodyStrongTextBlockStyle`: a control theme of `TextBlock`.
fn body_strong_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.static_resource(TextBlock::font_size_property(), "BodyStrongTextBlockFontSize")?;
    Ok(())
}

/// `BodyLargeTextBlockStyle`: a control theme of `TextBlock`.
fn body_large_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.set(TextBlock::font_weight_property(), FontWeight::Normal);
    theme.static_resource(TextBlock::font_size_property(), "BodyLargeTextBlockFontSize")?;
    Ok(())
}

/// `BodyLargeStrongTextBlockStyle`: a control theme of `TextBlock`.
fn body_large_strong_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.static_resource(TextBlock::font_size_property(), "BodyLargeTextBlockFontSize")?;
    Ok(())
}

/// `SubtitleTextBlockStyle`: a control theme of `TextBlock`.
fn subtitle_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.static_resource(TextBlock::font_size_property(), "SubtitleTextBlockFontSize")?;
    Ok(())
}

/// `TitleTextBlockStyle`: a control theme of `TextBlock`.
fn title_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.static_resource(TextBlock::font_size_property(), "TitleTextBlockFontSize")?;
    Ok(())
}

/// `TitleLargeTextBlockStyle`: a control theme of `TextBlock`.
fn title_large_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.static_resource(TextBlock::font_size_property(), "TitleLargeTextBlockFontSize")?;
    Ok(())
}

/// `DisplayTextBlockStyle`: a control theme of `TextBlock`.
fn display_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.static_resource(TextBlock::font_size_property(), "DisplayTextBlockFontSize")?;
    Ok(())
}

/// `HeaderTextBlockStyle`: a control theme of `TextBlock`.
fn header_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.set(TextBlock::font_size_property(), 46.0);
    theme.set(TextBlock::font_weight_property(), FontWeight::Light);
    Ok(())
}

/// `SubheaderTextBlockStyle`: a control theme of `TextBlock`.
fn subheader_text_block_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.set(TextBlock::font_size_property(), 34.0);
    theme.set(TextBlock::font_weight_property(), FontWeight::Light);
    Ok(())
}
