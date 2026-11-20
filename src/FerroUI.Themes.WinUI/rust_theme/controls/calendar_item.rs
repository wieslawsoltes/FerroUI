//! `Controls/CalendarItem.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/CalendarView_themeresources.xaml (the template of
//!   DefaultCalendarViewStyle and its template-local styles WeekDayNameStyle,
//!   HeaderNavigationButtonStyle and NavigationButtonStyle; its resources are
//!   Resources/CalendarView_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's CalendarItem is the body of a Calendar: the header button, the previous and next
//!   buttons and the views. The template is the one of DefaultCalendarViewStyle: the root Border,
//!   the header grid, the separator line, and the views.
//! - Template parts: HeaderButton, PreviousButton and NextButton are PART_HeaderButton,
//!   PART_PreviousButton and PART_NextButton. WinUI's views are scroll viewers of CalendarPanels
//!   that the control fills (MonthViewScrollViewer, YearViewScrollViewer, DecadeViewScrollViewer);
//!   FerroUI's CalendarItem fills two grids: PART_MonthView (the day names in its first row, six
//!   weeks of day buttons below) and PART_YearView (the months, or the years of a decade, in three
//!   rows of four). The year view has four rows as WinUI's has: FerroUI fills three of them, so the
//!   items are as large as WinUI's. PART_ElementWeekNumberLabels (the week numbers FerroUI's
//!   calendar shows when IsWeekNumberVisible is set) has no WinUI counterpart; it uses the size of
//!   the day names (CaptionTextBlockStyle) and the secondary text colour.
//! - The template-local styles are keyed control themes: WeekDayNameStyle is
//!   CalendarItemWeekDayNameStyle (of TextBlock, the DayTitleTemplate of the item uses it),
//!   HeaderNavigationButtonStyle is CalendarItemHeaderNavigationButtonStyle and
//!   NavigationButtonStyle is CalendarItemNavigationButtonStyle. Their ContentPresenter "Text" is
//!   PART_ContentPresenter.
//! - The glyphs of the navigation buttons (EDDB CaretUpSolid8 for PreviousButton, EDDC
//!   CaretDownSolid8 for NextButton, at CalendarViewNavigationButtonFontSize) are the geometries
//!   GlyphGeometryEDDB and GlyphGeometryEDDC, 8 px wide in a box of 8 px, the line of the glyph.
//! - The Disabled state sets the foreground of the seven day names: the names are the text blocks
//!   the item creates from its DayTitleTemplate in PART_MonthView, which inherit the text foreground
//!   of the grid; the state is FerroUI's :calendardisabled.
//! - HeaderButtonStates (the header fades in over 167 ms when the view changes) and the
//!   transitions of DisplayModeStates (the scale and opacity animations between the month, year and
//!   decade views) are not ported: FerroUI's item switches the views by their visibility, without a
//!   state to animate from. The Grid.Clip of the views (TemplateSettings.ClipRect) is ClipToBounds.
//! - The HeaderBackground of FerroUI's item (null by default) is the background of the header grid.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Stretch;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::documents::TextElement;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::CalendarItem;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinition;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::DefinitionBase;
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::Panel;
use ferroui_controls::RowDefinition;
use ferroui_controls::RowDefinitions;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // WeekDayNameStyle of the template of DefaultCalendarViewStyle.
    resources.control_theme("CalendarItemWeekDayNameStyle", calendar_item_week_day_name_style);
    // HeaderNavigationButtonStyle of the template of DefaultCalendarViewStyle.
    resources.control_theme(
        "CalendarItemHeaderNavigationButtonStyle",
        calendar_item_header_navigation_button_style,
    );
    // NavigationButtonStyle of the template of DefaultCalendarViewStyle.
    resources.control_theme(
        "CalendarItemNavigationButtonStyle",
        calendar_item_navigation_button_style,
    );
    resources.control_theme(type_key::<CalendarItem>(), calendar_item_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// `CalendarItemWeekDayNameStyle`: a control theme of `TextBlock`.
fn calendar_item_week_day_name_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("CaptionTextBlockStyle")?;
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(TextBlock::font_weight_property(), "CalendarViewWeekDayFontWeight");
    // CalendarViewWeekDayMargin (1); the bottom margin adds the 2 px between the day names and
    //         the days of the source (the top margin of MonthViewScrollViewer). With the 16 px line of
    //         the 12 px caption the row of the day names is the 2 + 42 of WinUI's, and the views are
    //         the 300 px of WinUI's.
    theme.set(Layoutable::margin_property(), Thickness::new(1.0, 1.0, 1.0, 3.0));
    // CalendarViewWeekDayPadding (12) above and below only: in WinUI the columns of the day
    //         names are the 42 px of the day items (40 and a margin of 1), and a name with its padding
    //         of 12 at both sides (44 px for "We") does not widen them; in FerroUI's grid it would widen
    //         every column, and the calendar by 5 px. The names are centred, so the horizontal padding
    //         does not move them.
    theme.set(TextBlock::padding_property(), Thickness::symmetric(0.0, 12.0));
    Ok(())
}

/// The focus adorner of `calendar_item_header_navigation_button_style`.
fn calendar_item_header_navigation_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-2.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "SystemControlFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(
                    Border::border_brush_property(),
                    "SystemControlFocusVisualSecondaryBrush",
                )
                .dynamic(Border::corner_radius_property(), "ControlCornerRadius"),
        )
        .control())
}

/// The template of `Button` of `calendar_item_header_navigation_button_style`.
fn calendar_item_header_navigation_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(
            ContentPresenter::foreground_property(),
            TemplatedControl::foreground_property(),
        )
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(
            ContentPresenter::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .dynamic(
            ContentPresenter::border_brush_property(),
            "CalendarViewNavigationButtonBorderBrush",
        )
        .template_bind(
            ContentPresenter::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(
            ContentPresenter::padding_property(),
            TemplatedControl::padding_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .control())
}

/// `CalendarItemHeaderNavigationButtonStyle`: a control theme of `Button`.
fn calendar_item_header_navigation_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    //         (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(
        TemplatedControl::font_size_property(),
        "CalendarViewHeaderNavigationButtonFontSize",
    );
    theme.dynamic(
        TemplatedControl::font_weight_property(),
        "CalendarViewHeaderNavigationFontWeight",
    );
    theme.dynamic(
        TemplatedControl::background_property(),
        "CalendarViewNavigationButtonBackground",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ButtonBorderThemeThickness",
    );
    theme.dynamic(
        Layoutable::margin_property(),
        "CalendarViewHeaderNavigationButtonMargin",
    );
    theme.dynamic(
        TemplatedControl::padding_property(),
        "CalendarViewHeaderNavigationButtonPadding",
    );
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    // UseSystemFocusVisuals with FocusVisualMargin="{StaticResource CalendarViewNavigationButtonFocusVisualMargin}" (-2).
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            calendar_item_header_navigation_button_style_focus_adorner,
        )),
    );
    theme.template(calendar_item_header_navigation_button_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "CalendarViewNavigationButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "CalendarViewNavigationButtonBorderBrushPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "CalendarViewHeaderNavigationButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "CalendarViewNavigationButtonBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "CalendarViewHeaderNavigationButtonForegroundPressed",
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "CalendarViewHeaderNavigationButtonForegroundDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The focus adorner of `calendar_item_navigation_button_style`.
fn calendar_item_navigation_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-2.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "SystemControlFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(
                    Border::border_brush_property(),
                    "SystemControlFocusVisualSecondaryBrush",
                )
                .dynamic(Border::corner_radius_property(), "ControlCornerRadius"),
        )
        .control())
}

/// The template of `Button` of `calendar_item_navigation_button_style`.
fn calendar_item_navigation_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(
            ContentPresenter::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .dynamic(
            ContentPresenter::border_brush_property(),
            "CalendarViewNavigationButtonBorderBrush",
        )
        .template_bind(
            ContentPresenter::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(
            ContentPresenter::padding_property(),
            TemplatedControl::padding_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .control())
}

/// `CalendarItemNavigationButtonStyle`: a control theme of `Button`.
fn calendar_item_navigation_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    //         (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.dynamic(
        TemplatedControl::background_property(),
        "CalendarViewNavigationButtonBackground",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ButtonBorderThemeThickness",
    );
    theme.dynamic(
        TemplatedControl::padding_property(),
        "CalendarViewNavigationButtonPadding",
    );
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(InputElement::is_tab_stop_property(), true);
    // UseSystemFocusVisuals with FocusVisualMargin="{StaticResource CalendarViewNavigationButtonFocusVisualMargin}" (-2).
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            calendar_item_navigation_button_style_focus_adorner,
        )),
    );
    theme.template(calendar_item_navigation_button_style_template);
    // The states also set the Foreground of the button itself, which the glyph paths of its
    //         content take (a path does not inherit the foreground of the content presenter as the
    //         glyph of a FontIcon does), over the foreground the CalendarItem template binds.
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "CalendarViewNavigationButtonForegroundPointerOver",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "CalendarViewNavigationButtonBackgroundPointerOver",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "CalendarViewNavigationButtonBorderBrushPointerOver",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "CalendarViewNavigationButtonForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "CalendarViewNavigationButtonForegroundPressed",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "CalendarViewNavigationButtonBackgroundPressed",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "CalendarViewNavigationButtonForegroundPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "CalendarViewNavigationButtonForegroundDisabled",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "CalendarViewNavigationButtonForegroundDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The focus adorner of `calendar_item_theme`.
fn calendar_item_theme_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(TextBlock::new())
        .bind(TextBlock::text_property(), path().binding())
        .static_resource(StyledElement::theme_property(), "CalendarItemWeekDayNameStyle")?
        .control())
}

/// The template of `CalendarItem` of `calendar_item_theme`.
fn calendar_item_theme_template(t: &Template<CalendarItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootBorder")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
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
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,Auto,*"))?);
                    Ok(())
                })?
                // The views: 300 px high, the height of the month view of the source (2 + 42 + 2 + 6 x 42 + 2),
                //                 so that the year view keeps the size of the calendar.
                .children([
                    t.make(Grid::new())
                        .with(|e| {
                            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,Auto,Auto"))?);
                            Ok(())
                        })?
                        .template_bind(Panel::background_property(), CalendarItem::header_background_property())
                        .children([
                            t.make(Button::new())
                                .name("PART_HeaderButton")?
                                .static_resource(
                                    StyledElement::theme_property(),
                                    "CalendarItemHeaderNavigationButtonStyle",
                                )?
                                .template_bind(
                                    TemplatedControl::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                )
                                .set(
                                    ContentControl::horizontal_content_alignment_property(),
                                    HorizontalAlignment::Left,
                                )
                                .control(),
                            t.make(Button::new())
                                .name("PART_PreviousButton")?
                                .set(Grid::column_property(), 1)
                                .static_resource(StyledElement::theme_property(), "CalendarItemNavigationButtonStyle")?
                                .template_bind(
                                    TemplatedControl::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                )
                                .dynamic(
                                    Layoutable::margin_property(),
                                    "CalendarViewNavigationPreviousButtonMargin",
                                )
                                .set(
                                    ContentControl::horizontal_content_alignment_property(),
                                    HorizontalAlignment::Center,
                                )
                                // EDDB at CalendarViewNavigationButtonFontSize (8).
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Panel::new())
                                        .set(Layoutable::width_property(), 8.0)
                                        .set(Layoutable::height_property(), 8.0)
                                        .children([t
                                            .make(Path::new())
                                            .static_resource(Path::data_property(), "GlyphGeometryEDDB")?
                                            .bind(
                                                Shape::fill_property(),
                                                path()
                                                    .ancestor_of::<Button>(0)
                                                    .prop(TemplatedControl::foreground_property())
                                                    .binding(),
                                            )
                                            .set(Shape::stretch_property(), Stretch::Uniform)
                                            .set(Layoutable::width_property(), 8.0)
                                            .set(
                                                Layoutable::horizontal_alignment_property(),
                                                HorizontalAlignment::Center,
                                            )
                                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                            .control()]),
                                )?
                                .control(),
                            t.make(Button::new())
                                .name("PART_NextButton")?
                                .set(Grid::column_property(), 2)
                                .static_resource(StyledElement::theme_property(), "CalendarItemNavigationButtonStyle")?
                                .template_bind(
                                    TemplatedControl::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                )
                                .dynamic(Layoutable::margin_property(), "CalendarViewNavigationNextButtonMargin")
                                .set(
                                    ContentControl::horizontal_content_alignment_property(),
                                    HorizontalAlignment::Center,
                                )
                                // EDDC at CalendarViewNavigationButtonFontSize (8).
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Panel::new())
                                        .set(Layoutable::width_property(), 8.0)
                                        .set(Layoutable::height_property(), 8.0)
                                        .children([t
                                            .make(Path::new())
                                            .static_resource(Path::data_property(), "GlyphGeometryEDDC")?
                                            .bind(
                                                Shape::fill_property(),
                                                path()
                                                    .ancestor_of::<Button>(0)
                                                    .prop(TemplatedControl::foreground_property())
                                                    .binding(),
                                            )
                                            .set(Shape::stretch_property(), Stretch::Uniform)
                                            .set(Layoutable::width_property(), 8.0)
                                            .set(
                                                Layoutable::horizontal_alignment_property(),
                                                HorizontalAlignment::Center,
                                            )
                                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                            .control()]),
                                )?
                                .control(),
                        ])
                        .control(),
                    t.make(Border::new())
                        .set(Grid::row_property(), 1)
                        .set(Layoutable::height_property(), 1.0)
                        .template_bind(Border::background_property(), TemplatedControl::border_brush_property())
                        .control(),
                    t.make(Grid::new())
                        .name("Views")?
                        .set(Grid::row_property(), 2)
                        .with(|e| {
                            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*"))?);
                            Ok(())
                        })?
                        .set(Layoutable::min_height_property(), 300.0)
                        .set(Visual::clip_to_bounds_property(), true)
                        .set(Grid::is_shared_size_scope_property(), true)
                        .children([
                            t.make(Grid::new())
                                .name("PART_ElementWeekNumberLabels")?
                                .set(Grid::column_property(), 0)
                                .set(Layoutable::margin_property(), Thickness::new(2.0, 2.0, 0.0, 2.0))
                                .set(Visual::is_visible_property(), false)
                                .with(|e| {
                                    let items = Grid::row_definitions(e);
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemDayNameRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    Ok(())
                                })?
                                .control(),
                            t.make(Grid::new())
                                .name("PART_MonthView")?
                                .set(Grid::column_property(), 1)
                                .set(Layoutable::margin_property(), Thickness::uniform(2.0))
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .set(Visual::is_visible_property(), false)
                                .with(|e| {
                                    let items = Grid::row_definitions(e);
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemDayNameRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), GridLength::auto())
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("CalendarItemWeekRow")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    Ok(())
                                })?
                                .with(|e| {
                                    let items = Grid::column_definitions(e);
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
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
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
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
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
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
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
                                            .end()
                                            .into(),
                                    );
                                    Ok(())
                                })?
                                .control(),
                            t.make(Grid::new())
                                .name("PART_YearView")?
                                .set(Grid::column_span_property(), 2)
                                .set(Layoutable::margin_property(), Thickness::uniform(2.0))
                                .set(Visual::is_visible_property(), false)
                                .with(|e| {
                                    let items = Grid::row_definitions(e);
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), literal(GridLength::parse("*"))?)
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), literal(GridLength::parse("*"))?)
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), literal(GridLength::parse("*"))?)
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(RowDefinition::new())
                                            .set(RowDefinition::height_property(), literal(GridLength::parse("*"))?)
                                            .end()
                                            .into(),
                                    );
                                    Ok(())
                                })?
                                .with(|e| {
                                    let items = Grid::column_definitions(e);
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
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
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
                                            .end()
                                            .into(),
                                    );
                                    Ok(())
                                })?
                                .control(),
                        ])
                        .control(),
                ]),
        )
        .control())
}

/// The default control theme of `CalendarItem`.
fn calendar_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CalendarItem>();
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        CalendarItem::day_title_template_property(),
        Some(focus_adorner(theme.scope(), calendar_item_theme_focus_adorner)),
    );
    theme.template(calendar_item_theme_template);
    // The foreground of the day names (WeekDay1..WeekDay7, TemplateBinding CalendarItemForeground).
    theme.style(nesting().template().of::<Grid>().name("PART_MonthView"), |style| {
        style.dynamic(TextElement::foreground_property(), "CalendarViewCalendarItemForeground");
        Ok(())
    })?;
    // Disabled
    theme.style(
        nesting()
            .class(":calendardisabled")
            .template()
            .of::<Grid>()
            .name("PART_MonthView"),
        |style| {
            style.dynamic(
                TextElement::foreground_property(),
                "CalendarViewWeekDayForegroundDisabled",
            );
            Ok(())
        },
    )?;
    // The week numbers (FerroUI only).
    theme.style(
        nesting()
            .class(":hasweeknumbers")
            .template()
            .of::<Grid>()
            .name("PART_ElementWeekNumberLabels")
            .descendant()
            .of::<ContentControl>(),
        |style| {
            style.static_resource(TemplatedControl::font_size_property(), "CaptionTextBlockFontSize")?;
            style.dynamic(TemplatedControl::foreground_property(), "TextFillColorSecondaryBrush");
            style.set(TemplatedControl::padding_property(), Thickness::new(8.0, 0.0, 8.0, 0.0));
            style.set(
                ContentControl::horizontal_content_alignment_property(),
                HorizontalAlignment::Center,
            );
            style.set(
                ContentControl::vertical_content_alignment_property(),
                VerticalAlignment::Center,
            );
            Ok(())
        },
    )?;
    Ok(())
}
