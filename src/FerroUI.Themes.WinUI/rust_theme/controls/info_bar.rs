//! `Controls/InfoBar.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/InfoBar/InfoBar.xaml (the implicit InfoBar style)
//!   controls/dev/InfoBar/InfoBar_themeresources.xaml (its resources are
//!   Resources/InfoBar_themeresources.xaml; InfoBarCloseButtonStyle is in Controls/NotificationCard.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The template of WinUI's InfoBar, which FerroUI ports as a control (ferroui-controls,
//! docs/porting/winui-controls.md); the Fluent theme has no document for it.
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The visual states are the pseudo-classes the control sets (InfoBar.cpp calls GoToState):
//!   SeverityLevels are :informational (the base values), :error, :warning and :success;
//!   IconStates are :standard-icon-visible and :user-icon-visible (NoIconVisible is neither);
//!   CloseButtonCollapsed is :close-button-collapsed; InfoBarVisible is :open, so InfoBarCollapsed
//!   is ^:not(:open); ForegroundSet is :foreground-set; NoBannerContent is :no-banner-content.
//! - ContentArea, the presenter of the Content, is PART_ContentPresenter: InfoBar is a
//!   ContentControl, whose content is hosted by the presenter of that name.
//! - The inner Grid draws a background with a corner radius and has a padding: a Border around the
//!   Grid, which keeps the MinHeight that InfoBarPanel reads from its parent.
//! - The severity icons are the glyphs of WinUI in TextBlocks in SymbolThemeFontFamily, and the
//!   close button the SymbolIcon of InfoBarCloseButtonSymbol, as in WinUI
//!   (docs/porting/winui-controls.md, section 5.2).
//! - The CloseButton has no Button.Resources: the AppBarButton brushes that WinUI gives the button
//!   there are the setters and states of InfoBarCloseButtonStyle (NC-7), its CloseButtonStyle.
//! - The Style of HyperlinkButton in the resources of the presenter of the action button is a style
//!   of its Styles. TextWrapping WrapWholeWords is WrapWithOverflow. AutomationProperties
//!   (LandmarkType, IsDialog, AccessibilityView) are left out.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::TextWrapping;
use ferroui_base::StyledElement;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::InfoBarPanel;
use ferroui_controls::primitives::InfoBarTemplateSettings;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::HyperlinkButton;
use ferroui_controls::InfoBar;
use ferroui_controls::RowDefinitions;
use ferroui_controls::SymbolIcon;
use ferroui_controls::TextBlock;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<InfoBar>(), info_bar_theme);
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

/// The template of `InfoBar` of `info_bar_theme`.
fn info_bar_theme_template(t: &Template<InfoBar>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("ContentRoot")?
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
        .dynamic(
            Border::background_property(),
            "InfoBarInformationalSeverityBackgroundBrush",
        )
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        // Background is used here so that it overrides the severity status color if set.
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(
                    Layoutable::horizontal_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .static_resource(Decorator::padding_property(), "InfoBarContentRootPadding")?
                .child(
                    Decorator::child_property(),
                    t.make(Grid::new())
                        .dynamic(Layoutable::min_height_property(), "InfoBarMinHeight")
                        .with(|e| {
                            e.set_column_definitions(literal(ColumnDefinitions::parse("Auto,*,Auto"))?);
                            Ok(())
                        })?
                        .with(|e| {
                            e.set_row_definitions(literal(RowDefinitions::parse("Auto,Auto"))?);
                            Ok(())
                        })?
                        .children([
                            t.make(Grid::new())
                                .name("StandardIconArea")?
                                .set(Visual::is_visible_property(), false)
                                .children([
                                    t.make(TextBlock::new())
                                        .name("IconBackground")?
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                        .static_resource(Layoutable::margin_property(), "InfoBarIconMargin")?
                                        .static_resource(TextBlock::font_size_property(), "InfoBarIconFontSize")?
                                        .static_resource(TextBlock::text_property(), "InfoBarIconBackgroundGlyph")?
                                        .dynamic(
                                            TextBlock::foreground_property(),
                                            "InfoBarInformationalSeverityIconBackground",
                                        )
                                        .dynamic(TextBlock::font_family_property(), "SymbolThemeFontFamily")
                                        .control(),
                                    t.make(TextBlock::new())
                                        .name("StandardIcon")?
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                        .static_resource(Layoutable::margin_property(), "InfoBarIconMargin")?
                                        .static_resource(TextBlock::font_size_property(), "InfoBarIconFontSize")?
                                        .static_resource(TextBlock::text_property(), "InfoBarInformationalIconGlyph")?
                                        .dynamic(
                                            TextBlock::foreground_property(),
                                            "InfoBarInformationalSeverityIconForeground",
                                        )
                                        .dynamic(TextBlock::font_family_property(), "SymbolThemeFontFamily")
                                        .control(),
                                ])
                                .control(),
                            t.make(Viewbox::new())
                                .name("UserIconBox")?
                                .set(Grid::column_property(), 0)
                                .set(Visual::is_visible_property(), false)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                .dynamic(Layoutable::max_width_property(), "InfoBarIconFontSize")
                                .dynamic(Layoutable::max_height_property(), "InfoBarIconFontSize")
                                .dynamic(Layoutable::margin_property(), "InfoBarIconMargin")
                                .bind(
                                    Viewbox::child_property(),
                                    path()
                                        .templated_parent()
                                        .prop(InfoBar::template_settings_property())
                                        .prop(InfoBarTemplateSettings::icon_element_property())
                                        .binding(),
                                )
                                .control(),
                            t.make(InfoBarPanel::new())
                                .set(Grid::column_property(), 1)
                                .static_resource(Layoutable::margin_property(), "InfoBarPanelMargin")?
                                .static_resource(
                                    InfoBarPanel::horizontal_orientation_padding_property(),
                                    "InfoBarPanelHorizontalOrientationPadding",
                                )?
                                .static_resource(
                                    InfoBarPanel::vertical_orientation_padding_property(),
                                    "InfoBarPanelVerticalOrientationPadding",
                                )?
                                .children([
                                    t.make(TextBlock::new())
                                        .name("Title")?
                                        .template_bind(TextBlock::text_property(), InfoBar::title_property())
                                        .dynamic(TextBlock::foreground_property(), "InfoBarTitleForeground")
                                        .static_resource(
                                            InfoBarPanel::horizontal_orientation_margin_property(),
                                            "InfoBarTitleHorizontalOrientationMargin",
                                        )?
                                        .static_resource(
                                            InfoBarPanel::vertical_orientation_margin_property(),
                                            "InfoBarTitleVerticalOrientationMargin",
                                        )?
                                        .set(TextBlock::text_wrapping_property(), TextWrapping::WrapWithOverflow)
                                        .static_resource(TextBlock::font_weight_property(), "InfoBarTitleFontWeight")?
                                        .static_resource(TextBlock::font_size_property(), "InfoBarTitleFontSize")?
                                        .control(),
                                    t.make(TextBlock::new())
                                        .name("Message")?
                                        .template_bind(TextBlock::text_property(), InfoBar::message_property())
                                        .dynamic(TextBlock::foreground_property(), "InfoBarMessageForeground")
                                        .static_resource(
                                            InfoBarPanel::horizontal_orientation_margin_property(),
                                            "InfoBarMessageHorizontalOrientationMargin",
                                        )?
                                        .static_resource(
                                            InfoBarPanel::vertical_orientation_margin_property(),
                                            "InfoBarMessageVerticalOrientationMargin",
                                        )?
                                        .set(TextBlock::text_wrapping_property(), TextWrapping::WrapWithOverflow)
                                        .static_resource(TextBlock::font_weight_property(), "InfoBarMessageFontWeight")?
                                        .static_resource(TextBlock::font_size_property(), "InfoBarMessageFontSize")?
                                        .control(),
                                    t.make(ContentPresenter::new())
                                        .static_resource(
                                            InfoBarPanel::horizontal_orientation_margin_property(),
                                            "InfoBarActionHorizontalOrientationMargin",
                                        )?
                                        .static_resource(
                                            InfoBarPanel::vertical_orientation_margin_property(),
                                            "InfoBarActionVerticalOrientationMargin",
                                        )?
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                        .template_bind(
                                            ContentPresenter::content_property(),
                                            InfoBar::action_button_property(),
                                        )
                                        .style(of::<HyperlinkButton>(), |style| {
                                            style.static_resource(
                                                Layoutable::margin_property(),
                                                "InfoBarHyperlinkButtonMargin",
                                            )?;
                                            style.dynamic(
                                                TemplatedControl::foreground_property(),
                                                "InfoBarHyperlinkButtonForeground",
                                            );
                                            Ok(())
                                        })?
                                        .control(),
                                ])
                                .control(),
                            t.make(ContentPresenter::new())
                                .name("PART_ContentPresenter")?
                                .set(Grid::column_property(), 1)
                                .set(Grid::row_property(), 1)
                                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    ContentControl::content_template_property(),
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .control(),
                            t.make(Button::new())
                                .name("CloseButton")?
                                .set(Grid::column_property(), 2)
                                .template_bind(Button::command_property(), InfoBar::close_button_command_property())
                                .template_bind(
                                    Button::command_parameter_property(),
                                    InfoBar::close_button_command_parameter_property(),
                                )
                                .template_bind(StyledElement::theme_property(), InfoBar::close_button_style_property())
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Viewbox::new())
                                        .static_resource(Layoutable::width_property(), "InfoBarCloseButtonGlyphSize")?
                                        .static_resource(Layoutable::height_property(), "InfoBarCloseButtonGlyphSize")?
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                        .child(
                                            Viewbox::child_property(),
                                            t.make(SymbolIcon::new()).static_resource(
                                                SymbolIcon::symbol_property(),
                                                "InfoBarCloseButtonSymbol",
                                            )?,
                                        ),
                                )?
                                .control(),
                        ]),
                ),
        )
        .control())
}

/// The default control theme of `InfoBar`.
fn info_bar_theme(theme: &StyleBuilder) -> Result {
    theme.target::<InfoBar>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.static_resource(InfoBar::close_button_style_property(), "InfoBarCloseButtonStyle")?;
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "InfoBarBorderBrush");
    theme.dynamic(TemplatedControl::border_thickness_property(), "InfoBarBorderThickness");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(info_bar_theme_template);
    // SeverityLevels: Error
    theme.style(
        nesting().class(":error").template().of::<Border>().name("ContentRoot"),
        |style| {
            style.dynamic(Border::background_property(), "InfoBarErrorSeverityBackgroundBrush");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":error")
            .template()
            .of::<TextBlock>()
            .name("IconBackground"),
        |style| {
            style.dynamic(TextBlock::foreground_property(), "InfoBarErrorSeverityIconBackground");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":error")
            .template()
            .of::<TextBlock>()
            .name("StandardIcon"),
        |style| {
            style.static_resource(TextBlock::text_property(), "InfoBarErrorIconGlyph")?;
            style.dynamic(TextBlock::foreground_property(), "InfoBarErrorSeverityIconForeground");
            Ok(())
        },
    )?;
    // SeverityLevels: Warning
    theme.style(
        nesting()
            .class(":warning")
            .template()
            .of::<Border>()
            .name("ContentRoot"),
        |style| {
            style.dynamic(Border::background_property(), "InfoBarWarningSeverityBackgroundBrush");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":warning")
            .template()
            .of::<TextBlock>()
            .name("IconBackground"),
        |style| {
            style.dynamic(TextBlock::foreground_property(), "InfoBarWarningSeverityIconBackground");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":warning")
            .template()
            .of::<TextBlock>()
            .name("StandardIcon"),
        |style| {
            style.static_resource(TextBlock::text_property(), "InfoBarWarningIconGlyph")?;
            style.dynamic(TextBlock::foreground_property(), "InfoBarWarningSeverityIconForeground");
            Ok(())
        },
    )?;
    // SeverityLevels: Success
    theme.style(
        nesting()
            .class(":success")
            .template()
            .of::<Border>()
            .name("ContentRoot"),
        |style| {
            style.dynamic(Border::background_property(), "InfoBarSuccessSeverityBackgroundBrush");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":success")
            .template()
            .of::<TextBlock>()
            .name("IconBackground"),
        |style| {
            style.dynamic(TextBlock::foreground_property(), "InfoBarSuccessSeverityIconBackground");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":success")
            .template()
            .of::<TextBlock>()
            .name("StandardIcon"),
        |style| {
            style.static_resource(TextBlock::text_property(), "InfoBarSuccessIconGlyph")?;
            style.dynamic(TextBlock::foreground_property(), "InfoBarSuccessSeverityIconForeground");
            Ok(())
        },
    )?;
    // IconStates: StandardIconVisible, UserIconVisible (NoIconVisible: the base values).
    theme.style(
        nesting()
            .class(":standard-icon-visible")
            .template()
            .of::<Viewbox>()
            .name("UserIconBox"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":standard-icon-visible")
            .template()
            .of::<Grid>()
            .name("StandardIconArea"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":user-icon-visible")
            .template()
            .of::<Viewbox>()
            .name("UserIconBox"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":user-icon-visible")
            .template()
            .of::<Grid>()
            .name("StandardIconArea"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // CloseButtonCollapsed
    theme.style(
        nesting()
            .class(":close-button-collapsed")
            .template()
            .of::<Button>()
            .name("CloseButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // InfoBarVisibility: InfoBarCollapsed
    theme.style(
        nesting()
            .not(class(":open"))
            .template()
            .of::<Border>()
            .name("ContentRoot"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // ForegroundSet: the title and the message take the foreground set on the bar.
    theme.style(
        nesting()
            .class(":foreground-set")
            .template()
            .of::<TextBlock>()
            .name("Title"),
        |style| {
            style.bind(
                TextBlock::foreground_property(),
                path()
                    .ancestor_of::<InfoBar>(0)
                    .prop(TemplatedControl::foreground_property())
                    .binding(),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":foreground-set")
            .template()
            .of::<TextBlock>()
            .name("Message"),
        |style| {
            style.bind(
                TextBlock::foreground_property(),
                path()
                    .ancestor_of::<InfoBar>(0)
                    .prop(TemplatedControl::foreground_property())
                    .binding(),
            );
            Ok(())
        },
    )?;
    // NoBannerContent: the content moves up.
    theme.style(
        nesting()
            .class(":no-banner-content")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.set(Grid::row_property(), 0);
            Ok(())
        },
    )?;
    Ok(())
}
