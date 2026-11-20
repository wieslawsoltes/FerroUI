//! `Controls/InfoBadge.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/InfoBadge/InfoBadge.xaml (the implicit InfoBadge style)
//!   controls/dev/InfoBadge/InfoBadge_themeresources.xaml (DefaultInfoBadgeStyle and the styles of
//!   the severities; its resources are Resources/InfoBadge_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The template of WinUI's InfoBadge, which FerroUI ports as a control (ferroui-controls,
//! docs/porting/winui-controls.md); the Fluent theme has no document for it.
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The states of DisplayKindStates are the pseudo-classes the control sets (InfoBadge.cpp calls
//!   GoToState): Dot is the base, Icon is :icon, FontIcon is :font-icon, Value is :value.
//! - The root Grid draws a background with a corner radius and has a padding: a Border of the same
//!   name around a Grid. Its corner radius and the icon are bound to the TemplateSettings of the
//!   badge, the text of the number to its Value.
//! - IsTabStop is IsTabStop. The styles of the severities keep their keys; an application gives a
//!   badge one as its Theme.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::InfoBadgeTemplateSettings;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::FontIconSource;
use ferroui_controls::Grid;
use ferroui_controls::InfoBadge;
use ferroui_controls::Symbol;
use ferroui_controls::SymbolIconSource;
use ferroui_controls::TextBlock;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultInfoBadgeStyle", default_info_badge_style);
    resources.control_theme(type_key::<InfoBadge>(), info_badge_theme);
    resources.control_theme("AttentionDotInfoBadgeStyle", attention_dot_info_badge_style);
    resources.control_theme("AttentionValueInfoBadgeStyle", attention_value_info_badge_style);
    resources.control_theme("AttentionIconInfoBadgeStyle", attention_icon_info_badge_style);
    resources.control_theme("InformationalDotInfoBadgeStyle", informational_dot_info_badge_style);
    resources.control_theme("InformationalValueInfoBadgeStyle", informational_value_info_badge_style);
    resources.control_theme("InformationalIconInfoBadgeStyle", informational_icon_info_badge_style);
    resources.control_theme("SuccessDotInfoBadgeStyle", success_dot_info_badge_style);
    resources.control_theme("SuccessValueInfoBadgeStyle", success_value_info_badge_style);
    resources.control_theme("SuccessIconInfoBadgeStyle", success_icon_info_badge_style);
    resources.control_theme("CautionDotInfoBadgeStyle", caution_dot_info_badge_style);
    resources.control_theme("CautionValueInfoBadgeStyle", caution_value_info_badge_style);
    resources.control_theme("CautionIconInfoBadgeStyle", caution_icon_info_badge_style);
    resources.control_theme("CriticalDotInfoBadgeStyle", critical_dot_info_badge_style);
    resources.control_theme("CriticalValueInfoBadgeStyle", critical_value_info_badge_style);
    resources.control_theme("CriticalIconInfoBadgeStyle", critical_icon_info_badge_style);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `InfoBadge` of `default_info_badge_style`.
fn default_info_badge_style_template(t: &Template<InfoBadge>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .bind(
            Border::corner_radius_property(),
            path()
                .templated_parent()
                .prop(InfoBadge::template_settings_property())
                .prop(InfoBadgeTemplateSettings::info_badge_corner_radius_property())
                .binding(),
        )
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .child(
            Decorator::child_property(),
            t.make(Grid::new()).children([
                t.make(TextBlock::new())
                    .name("ValueTextBlock")?
                    .bind(
                        TextBlock::text_property(),
                        path().templated_parent().prop(InfoBadge::value_property()).binding(),
                    )
                    .dynamic(TextBlock::font_size_property(), "InfoBadgeValueFontSize")
                    .set(Visual::is_visible_property(), false)
                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                    .control(),
                t.make(Viewbox::new())
                    .name("IconPresenter")?
                    .set(Visual::is_visible_property(), false)
                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                    .child(
                        Viewbox::child_property(),
                        t.make(ContentPresenter::new()).bind(
                            ContentPresenter::content_property(),
                            path()
                                .templated_parent()
                                .prop(InfoBadge::template_settings_property())
                                .prop(InfoBadgeTemplateSettings::icon_element_property())
                                .binding(),
                        ),
                    )
                    .control(),
            ]),
        )
        .control())
}

/// `DefaultInfoBadgeStyle`: a control theme of `InfoBadge`.
fn default_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.dynamic(Layoutable::min_height_property(), "InfoBadgeMinHeight");
    theme.dynamic(Layoutable::min_width_property(), "InfoBadgeMinWidth");
    theme.dynamic(Layoutable::max_height_property(), "InfoBadgeMaxHeight");
    theme.dynamic(TemplatedControl::background_property(), "InfoBadgeBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "InfoBadgeForeground");
    theme.dynamic(TemplatedControl::padding_property(), "InfoBadgePadding");
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.template(default_info_badge_style_template);
    // DisplayKindStates: Icon
    theme.style(
        nesting()
            .class(":icon")
            .template()
            .of::<Viewbox>()
            .name("IconPresenter"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            style.dynamic(Layoutable::margin_property(), "IconInfoBadgeIconMargin");
            Ok(())
        },
    )?;
    // DisplayKindStates: FontIcon
    theme.style(
        nesting()
            .class(":font-icon")
            .template()
            .of::<Viewbox>()
            .name("IconPresenter"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            style.dynamic(Layoutable::margin_property(), "IconInfoBadgeFontIconMargin");
            Ok(())
        },
    )?;
    // DisplayKindStates: Value
    theme.style(
        nesting()
            .class(":value")
            .template()
            .of::<TextBlock>()
            .name("ValueTextBlock"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            style.dynamic(Layoutable::margin_property(), "ValueInfoBadgeTextMargin");
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `InfoBadge`.
fn info_badge_theme(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("DefaultInfoBadgeStyle")?;
    Ok(())
}

/// `AttentionDotInfoBadgeStyle`: a control theme of `InfoBadge`.
fn attention_dot_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("DefaultInfoBadgeStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "SystemFillColorAttentionBrush");
    Ok(())
}

/// `AttentionValueInfoBadgeStyle`: a control theme of `InfoBadge`.
fn attention_value_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("AttentionDotInfoBadgeStyle")?;
    Ok(())
}

/// `AttentionIconInfoBadgeStyle`: a control theme of `InfoBadge`.
fn attention_icon_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("AttentionDotInfoBadgeStyle")?;
    theme.set(TemplatedControl::padding_property(), Thickness::new(0.0, 4.0, 0.0, 2.0));
    theme.set_value(
        InfoBadge::icon_source_property(),
        value(
            theme
                .scope()
                .make(FontIconSource::new())
                .set(FontIconSource::glyph_property(), String::from(""))
                .end(),
        ),
    );
    Ok(())
}

/// `InformationalDotInfoBadgeStyle`: a control theme of `InfoBadge`.
fn informational_dot_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("DefaultInfoBadgeStyle")?;
    theme.dynamic(
        TemplatedControl::background_property(),
        "SystemFillColorSolidNeutralBrush",
    );
    Ok(())
}

/// `InformationalValueInfoBadgeStyle`: a control theme of `InfoBadge`.
fn informational_value_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("InformationalDotInfoBadgeStyle")?;
    Ok(())
}

/// `InformationalIconInfoBadgeStyle`: a control theme of `InfoBadge`.
fn informational_icon_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("InformationalDotInfoBadgeStyle")?;
    theme.set(TemplatedControl::padding_property(), Thickness::new(0.0, 4.0, 0.0, 2.0));
    theme.set_value(
        InfoBadge::icon_source_property(),
        value(
            theme
                .scope()
                .make(FontIconSource::new())
                .set(FontIconSource::glyph_property(), String::from(""))
                .end(),
        ),
    );
    Ok(())
}

/// `SuccessDotInfoBadgeStyle`: a control theme of `InfoBadge`.
fn success_dot_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("DefaultInfoBadgeStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "SystemFillColorSuccessBrush");
    Ok(())
}

/// `SuccessValueInfoBadgeStyle`: a control theme of `InfoBadge`.
fn success_value_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("SuccessDotInfoBadgeStyle")?;
    Ok(())
}

/// `SuccessIconInfoBadgeStyle`: a control theme of `InfoBadge`.
fn success_icon_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("SuccessDotInfoBadgeStyle")?;
    theme.set_value(
        InfoBadge::icon_source_property(),
        value(
            theme
                .scope()
                .make(SymbolIconSource::new())
                .set(SymbolIconSource::symbol_property(), Symbol::Accept)
                .end(),
        ),
    );
    Ok(())
}

/// `CautionDotInfoBadgeStyle`: a control theme of `InfoBadge`.
fn caution_dot_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("DefaultInfoBadgeStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "SystemFillColorCautionBrush");
    Ok(())
}

/// `CautionValueInfoBadgeStyle`: a control theme of `InfoBadge`.
fn caution_value_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("CautionDotInfoBadgeStyle")?;
    Ok(())
}

/// `CautionIconInfoBadgeStyle`: a control theme of `InfoBadge`.
fn caution_icon_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("CautionDotInfoBadgeStyle")?;
    theme.set_value(
        InfoBadge::icon_source_property(),
        value(
            theme
                .scope()
                .make(SymbolIconSource::new())
                .set(SymbolIconSource::symbol_property(), Symbol::Important)
                .end(),
        ),
    );
    Ok(())
}

/// `CriticalDotInfoBadgeStyle`: a control theme of `InfoBadge`.
fn critical_dot_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("DefaultInfoBadgeStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "SystemFillColorCriticalBrush");
    Ok(())
}

/// `CriticalValueInfoBadgeStyle`: a control theme of `InfoBadge`.
fn critical_value_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("CriticalDotInfoBadgeStyle")?;
    Ok(())
}

/// `CriticalIconInfoBadgeStyle`: a control theme of `InfoBadge`.
fn critical_icon_info_badge_style(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBadge>();
    theme.based_on("CriticalDotInfoBadgeStyle")?;
    theme.set_value(
        InfoBadge::icon_source_property(),
        value(
            theme
                .scope()
                .make(SymbolIconSource::new())
                .set(SymbolIconSource::symbol_property(), Symbol::Cancel)
                .end(),
        ),
    );
    Ok(())
}
