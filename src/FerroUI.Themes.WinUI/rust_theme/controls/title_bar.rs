//! `Controls/TitleBar.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TitleBar/TitleBar.xaml (DefaultTitleBarStyle and the implicit style)
//!   controls/dev/TitleBar/TitleBar_themeresources.xaml (its resources are
//!   Resources/TitleBar_themeresources.xaml; the button styles are in WindowDrawnDecorations.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The template of WinUI's TitleBar, which FerroUI ports as a control (ferroui-controls,
//! docs/porting/winui-controls.md); the Fluent theme has no document for it.
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The visual states are the pseudo-classes the control sets (TitleBar.cpp calls GoToState): the
//!   Visible state of each group is :back-button-visible, :pane-toggle-button-visible,
//!   :left-header-visible, :icon-visible, :title-text-visible, :subtitle-text-visible,
//!   :content-visible and :right-header-visible; the Deactivated state of every group is the
//!   Visible state with :deactivated (the window is not active); ExpandedHeight is
//!   :expanded-height, Compact is :compact and NegativeInsetSpacing is :negative-inset-spacing.
//! - x:DeferLoadStrategy="Lazy" is left out: the parts are in the template, hidden (IsVisible
//!   False) until their state shows them.
//! - TitleBarBackButtonStyle and TitleBarPaneToggleButtonStyle are the ones of
//!   WindowDrawnDecorations.xaml, which ports them for the title bar the toolkit draws; the style of
//!   the control is DefaultTitleBarControlStyle, as WindowDrawnDecorations.xaml has the key
//!   DefaultTitleBarStyle for that title bar.
//! - The text styles: PART_TitleText and PART_SubtitleText are CaptionTextBlockStyle.
//! - LeftPaddingColumn, LeftHeaderPaddingColumn and RightPaddingColumn are Borders of those names in
//!   Auto columns (a ColumnDefinition has no name in FerroUI); the control sets the widths of the
//!   outer two to the caption-button insets of the window, the NegativeInsetSpacing state the width
//!   of the middle one.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::media::TextTrimming;
use ferroui_base::media::TextWrapping;
use ferroui_base::StyledElement;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::TitleBarTemplateSettings;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinition;
use ferroui_controls::Control;
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::Panel;
use ferroui_controls::TextBlock;
use ferroui_controls::TitleBar;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultTitleBarControlStyle", default_title_bar_control_style);
    resources.control_theme(type_key::<TitleBar>(), title_bar_theme);
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

/// The template of `TitleBar` of `default_title_bar_control_style`.
fn default_title_bar_control_style_template(t: &Template<TitleBar>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("PART_LayoutRoot")?
        .dynamic(Layoutable::height_property(), "TitleBarCompactHeight")
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .with(|e| {
            let items = Grid::column_definitions(e);
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .dynamic(ColumnDefinition::width_property(), "TitleBarMinDragRegionWidth")
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), GridLength::auto())
                    .end()
                    .into(),
            );
            Ok(())
        })?
        // The padding columns of WinUI are Auto columns that hold these borders: a column
        // definition has no name in FerroUI. The control sets the width of the outer two to the
        // caption-button insets of the window.
        .children([
            t.make(Border::new())
                .name("LeftPaddingColumn")?
                .set(Grid::column_property(), 0)
                .dynamic(Layoutable::width_property(), "TitleBarLeftPaddingWidth")
                .control(),
            t.make(Border::new())
                .name("LeftHeaderPaddingColumn")?
                .set(Grid::column_property(), 4)
                .dynamic(Layoutable::width_property(), "TitleBarLeftHeaderPaddingWidth")
                .control(),
            t.make(Border::new())
                .name("RightPaddingColumn")?
                .set(Grid::column_property(), 11)
                .dynamic(Layoutable::width_property(), "TitleBarRightPaddingWidth")
                .control(),
            t.make(Button::new())
                .name("PART_BackButton")?
                .set(Grid::column_property(), 1)
                .set(Visual::is_visible_property(), false)
                .static_resource(StyledElement::theme_property(), "TitleBarBackButtonStyle")?
                .template_bind(
                    InputElement::is_enabled_property(),
                    TitleBar::is_back_button_enabled_property(),
                )
                .control(),
            t.make(Button::new())
                .name("PART_PaneToggleButton")?
                .set(Grid::column_property(), 2)
                .set(Visual::is_visible_property(), false)
                .static_resource(StyledElement::theme_property(), "TitleBarPaneToggleButtonStyle")?
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_LeftHeaderPresenter")?
                .set(Grid::column_property(), 3)
                .set(Visual::is_visible_property(), false)
                .template_bind(ContentPresenter::content_property(), TitleBar::left_header_property())
                .dynamic(
                    Layoutable::horizontal_alignment_property(),
                    "TitleBarLeftHeaderHorizontalAlignment",
                )
                .dynamic(
                    Layoutable::vertical_alignment_property(),
                    "TitleBarLeftHeaderVerticalAlignment",
                )
                .control(),
            t.make(Viewbox::new())
                .name("PART_Icon")?
                .set(Grid::column_property(), 5)
                .set(Visual::is_visible_property(), false)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .dynamic(Layoutable::max_width_property(), "TitleBarIconMaxWidth")
                .dynamic(Layoutable::max_height_property(), "TitleBarIconMaxHeight")
                .dynamic(Layoutable::margin_property(), "TitleBarIconMargin")
                .bind(
                    Viewbox::child_property(),
                    path()
                        .templated_parent()
                        .prop(TitleBar::template_settings_property())
                        .prop(TitleBarTemplateSettings::icon_element_property())
                        .binding(),
                )
                .control(),
            t.make(TextBlock::new())
                .name("PART_TitleText")?
                .set(Grid::column_property(), 6)
                .set(Visual::is_visible_property(), false)
                .template_bind(TextBlock::text_property(), TitleBar::title_property())
                .static_resource(StyledElement::theme_property(), "CaptionTextBlockStyle")?
                .template_bind(
                    TextBlock::foreground_property(),
                    TemplatedControl::foreground_property(),
                )
                .dynamic(Layoutable::margin_property(), "TitleBarTitleMargin")
                .set(
                    TextBlock::text_trimming_property(),
                    <dyn TextTrimming>::character_ellipsis(),
                )
                .set(TextBlock::text_wrapping_property(), TextWrapping::NoWrap)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .dynamic(Layoutable::min_width_property(), "TitleBarTitleMinWidth")
                .control(),
            t.make(TextBlock::new())
                .name("PART_SubtitleText")?
                .set(Grid::column_property(), 7)
                .set(Visual::is_visible_property(), false)
                .template_bind(TextBlock::text_property(), TitleBar::subtitle_property())
                .static_resource(StyledElement::theme_property(), "CaptionTextBlockStyle")?
                .dynamic(TextBlock::foreground_property(), "TitleBarSubtitleForegroundBrush")
                .dynamic(Layoutable::margin_property(), "TitleBarSubtitleMargin")
                .set(
                    TextBlock::text_trimming_property(),
                    <dyn TextTrimming>::character_ellipsis(),
                )
                .set(TextBlock::text_wrapping_property(), TextWrapping::NoWrap)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .dynamic(Layoutable::min_width_property(), "TitleBarSubtitleMinWidth")
                .control(),
            t.make(Grid::new())
                .name("PART_ContentPresenterGrid")?
                .set(Grid::column_property(), 8)
                .set(Visual::is_visible_property(), false)
                .children([t
                    .make(ContentPresenter::new())
                    .name("PART_ContentPresenter")?
                    .template_bind(ContentPresenter::content_property(), TitleBar::content_property())
                    .dynamic(
                        Layoutable::horizontal_alignment_property(),
                        "TitleBarContentHorizontalAlignment",
                    )
                    .dynamic(
                        Layoutable::vertical_alignment_property(),
                        "TitleBarContentVerticalAlignment",
                    )
                    .control()])
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_RightHeaderPresenter")?
                .set(Grid::column_property(), 9)
                .set(Visual::is_visible_property(), false)
                .template_bind(ContentPresenter::content_property(), TitleBar::right_header_property())
                .dynamic(
                    Layoutable::horizontal_alignment_property(),
                    "TitleBarRightHeaderHorizontalAlignment",
                )
                .dynamic(
                    Layoutable::vertical_alignment_property(),
                    "TitleBarRightHeaderVerticalAlignment",
                )
                .control(),
        ])
        .control())
}

/// `DefaultTitleBarControlStyle`: a control theme of `TitleBar`.
fn default_title_bar_control_style(theme: &StyleBuilder) -> Result {
    theme.target::<TitleBar>();
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.dynamic(TemplatedControl::foreground_property(), "TitleBarForegroundBrush");
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.template(default_title_bar_control_style_template);
    // BackButtonGroup
    theme.style(
        nesting()
            .class(":back-button-visible")
            .template()
            .of::<Button>()
            .name("PART_BackButton"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":back-button-visible")
            .class(":deactivated")
            .template()
            .of::<Button>()
            .name("PART_BackButton"),
        |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "TitleBarDeactivatedForegroundBrush",
            );
            Ok(())
        },
    )?;
    // PaneToggleButtonGroup
    theme.style(
        nesting()
            .class(":pane-toggle-button-visible")
            .template()
            .of::<Button>()
            .name("PART_PaneToggleButton"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pane-toggle-button-visible")
            .class(":deactivated")
            .template()
            .of::<Button>()
            .name("PART_PaneToggleButton"),
        |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "TitleBarDeactivatedForegroundBrush",
            );
            Ok(())
        },
    )?;
    // LeftHeaderGroup
    theme.style(
        nesting()
            .class(":left-header-visible")
            .template()
            .of::<ContentPresenter>()
            .name("PART_LeftHeaderPresenter"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":left-header-visible")
            .class(":deactivated")
            .template()
            .of::<ContentPresenter>()
            .name("PART_LeftHeaderPresenter"),
        |style| {
            style.dynamic(Visual::opacity_property(), "TitleBarDeactivatedOpacity");
            Ok(())
        },
    )?;
    // IconGroup
    theme.style(
        nesting()
            .class(":icon-visible")
            .template()
            .of::<Viewbox>()
            .name("PART_Icon"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":icon-visible")
            .class(":deactivated")
            .template()
            .of::<Viewbox>()
            .name("PART_Icon"),
        |style| {
            style.dynamic(Visual::opacity_property(), "TitleBarDeactivatedOpacity");
            Ok(())
        },
    )?;
    // TitleTextGroup
    theme.style(
        nesting()
            .class(":title-text-visible")
            .template()
            .of::<TextBlock>()
            .name("PART_TitleText"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":title-text-visible")
            .class(":deactivated")
            .template()
            .of::<TextBlock>()
            .name("PART_TitleText"),
        |style| {
            style.dynamic(TextBlock::foreground_property(), "TitleBarDeactivatedForegroundBrush");
            Ok(())
        },
    )?;
    // SubtitleTextGroup
    theme.style(
        nesting()
            .class(":subtitle-text-visible")
            .template()
            .of::<TextBlock>()
            .name("PART_SubtitleText"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":subtitle-text-visible")
            .class(":deactivated")
            .template()
            .of::<TextBlock>()
            .name("PART_SubtitleText"),
        |style| {
            style.dynamic(
                TextBlock::foreground_property(),
                "TitleBarSubtitleDeactivatedForegroundBrush",
            );
            Ok(())
        },
    )?;
    // ContentGroup
    theme.style(
        nesting()
            .class(":content-visible")
            .template()
            .of::<Grid>()
            .name("PART_ContentPresenterGrid"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":content-visible")
            .class(":deactivated")
            .template()
            .of::<Grid>()
            .name("PART_ContentPresenterGrid"),
        |style| {
            style.dynamic(Visual::opacity_property(), "TitleBarDeactivatedOpacity");
            Ok(())
        },
    )?;
    // RightHeaderGroup
    theme.style(
        nesting()
            .class(":right-header-visible")
            .template()
            .of::<ContentPresenter>()
            .name("PART_RightHeaderPresenter"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":right-header-visible")
            .class(":deactivated")
            .template()
            .of::<ContentPresenter>()
            .name("PART_RightHeaderPresenter"),
        |style| {
            style.dynamic(Visual::opacity_property(), "TitleBarDeactivatedOpacity");
            Ok(())
        },
    )?;
    // LeftHeaderSpacingGroup
    theme.style(
        nesting()
            .class(":negative-inset-spacing")
            .template()
            .of::<Border>()
            .name("LeftHeaderPaddingColumn"),
        |style| {
            style.dynamic(Layoutable::width_property(), "TitleBarHeaderNegativeInsetPaddingWidth");
            Ok(())
        },
    )?;
    // HeightGroup
    theme.style(
        nesting()
            .class(":expanded-height")
            .template()
            .of::<Grid>()
            .name("PART_LayoutRoot"),
        |style| {
            style.dynamic(Layoutable::height_property(), "TitleBarExpandedHeight");
            Ok(())
        },
    )?;
    // DisplayModeGroup: Compact (after the Visible states, which it overrides)
    theme.style(
        nesting()
            .class(":compact")
            .template()
            .of::<TextBlock>()
            .name("PART_TitleText"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":compact")
            .template()
            .of::<TextBlock>()
            .name("PART_SubtitleText"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":compact")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
            style.dynamic(Layoutable::margin_property(), "TitleBarCompactContentMargin");
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `TitleBar`.
fn title_bar_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TitleBar>();
    theme.based_on("DefaultTitleBarControlStyle")?;
    Ok(())
}
