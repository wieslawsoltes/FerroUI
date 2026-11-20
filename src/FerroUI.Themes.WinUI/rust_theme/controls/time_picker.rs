//! `Controls/TimePicker.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/TimePicker_themeresources.xaml (the implicit styles of TimePicker,
//!   with its template-local TimePickerFlyoutButtonStyle, and of TimePickerFlyoutPresenter; its
//!   resources are Resources/TimePicker_themeresources.xaml)
//!   controls/dev/CommonStyles/DateTimePickerFlyout_themeresources.xaml (the template of
//!   DefaultLoopingSelectorStyle, written into the hosts of the presenter; the themes of the buttons
//!   and the items are in Controls/DateTimePickerShared.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts of TimePicker: FlyoutButton, FlyoutButtonContentGrid, FirstPickerHost,
//!   SecondPickerHost, ThirdPickerHost, HourTextBlock, MinuteTextBlock, PeriodTextBlock,
//!   FirstColumnDivider and SecondColumnDivider are PART_FlyoutButton,
//!   PART_FlyoutButtonContentGrid, PART_FirstPickerHost, PART_SecondPickerHost, PART_ThirdPickerHost,
//!   PART_HourTextBlock, PART_MinuteTextBlock, PART_PeriodTextBlock, PART_FirstColumnDivider and
//!   PART_SecondColumnDivider. FerroUI's TimePicker can show seconds (UseSeconds): it has a fourth
//!   host and a third divider (PART_FourthPickerHost, PART_ThirdColumnDivider) and the text block
//!   PART_SecondTextBlock; the seconds are the third host, the period the fourth. The control sets the
//!   columns (* for a host, Auto for a divider) and the visibility of the hosts in code.
//! - WinUI's TimePickerFlyout is a flyout the control creates in code; FerroUI's TimePicker has the
//!   Popup PART_Popup with the TimePickerPresenter PART_PickerPresenter in its template.
//! - The texts of the placeholders are the localized strings of WinUI ("hour", "minute",
//!   TEXT_TIMEPICKER_*_PLACEHOLDER of dxaml/xcp/dxaml/dllsrv/winrt/Microsoft.UI.Xaml.Common.rc) in
//!   English, and "second" for the seconds of FerroUI; the period without a time is set by the control
//!   (the designator of the current time, where WinUI shows "AM").
//! - The template-local TimePickerFlyoutButtonStyle is the keyed theme of the same name; its
//!   ContentPresenter is PART_ContentPresenter.
//! - HasTimeStates: HasNoTime is FerroUI's :hasnotime.
//! - The MinWidth and MaxWidth of the flyout button (TimePickerThemeMinWidth, TimePickerThemeMaxWidth) are
//!   setters of the control bound to the button, so that the sizes of FerroUI's control apply.
//! - The focus visual, the header and the properties without a counterpart are as for DatePicker
//!   (Controls/DatePicker.xaml).
//! - The FerroUI contract: the template is wrapped in DataValidationErrors, which shows the errors
//!   of FerroUI's data validation under the control, and :error gives the border the critical brush
//!   (SystemFillColorCriticalBrush), as the Fluent theme does and as Controls/TextBox.xaml does.
//! - Template parts of TimePickerPresenter: the LoopingSelectors the WinUI presenter creates in code
//!   and puts into FirstPickerHost, SecondPickerHost and ThirdPickerHost are the host panels
//!   PART_HourHost, PART_MinuteHost, PART_SecondHost and PART_PeriodHost, each with the template of
//!   DefaultLoopingSelectorStyle (as DatePickerPresenter); PickerHostGrid is PART_PickerContainer;
//!   FirstPickerSpacing and SecondPickerSpacing are PART_FirstSpacer and PART_SecondSpacer, and
//!   PART_ThirdSpacer is the divider of the seconds.
//! - HighlightRect (a Grid with a background and a corner radius) is a Border, under the items; the
//!   three MonochromaticOverlayPresenters (with the background of the highlight, 3 px inside each
//!   column) that draw the selected items again in TimePickerFlyoutPresenterHighlightForegroundColor
//!   are the selected state of the items (Controls/DateTimePickerShared.xaml).
//! - The accept button has the margin DatePickerFlyoutPresenterDismissMargin, as in the source.
//! - The ThemeShadow of the flyout (a Translation.Z of 32, as DatePickerFlyoutPresenter) is the
//!   ThemeShadow (the shared WinUIThemeShadow, Translation.Z 32) on the border Background of the presenter, and the margin
//!   ThemeShadowFlyoutMargin of PART_PickerPresenter is the room for it in the popup
//!   (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::FontWeight;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextAlignment;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::date_time_pickers::DateTimePickerPanel;
use ferroui_controls::date_time_pickers::DateTimePickerPanelType;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::DataValidationErrors;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ListBoxItem;
use ferroui_controls::Panel;
use ferroui_controls::PlacementMode;
use ferroui_controls::RepeatButton;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TextBlock;
use ferroui_controls::TimePicker;
use ferroui_controls::TimePickerPresenter;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // TimePickerFlyoutButtonStyle of the template of the implicit style of TimePicker.
    resources.control_theme("TimePickerFlyoutButtonStyle", time_picker_flyout_button_style);
    resources.control_theme(type_key::<TimePicker>(), time_picker_theme);
    resources.control_theme(type_key::<TimePickerPresenter>(), time_picker_presenter_theme);
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

/// The template of `Button` of `time_picker_flyout_button_style`.
fn time_picker_flyout_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .children([t
            .make(ContentPresenter::new())
            .name("PART_ContentPresenter")?
            .template_bind(
                ContentPresenter::background_property(),
                TemplatedControl::background_property(),
            )
            .template_bind(
                ContentPresenter::foreground_property(),
                TemplatedControl::foreground_property(),
            )
            .template_bind(
                ContentPresenter::border_brush_property(),
                TemplatedControl::border_brush_property(),
            )
            .set(
                ContentPresenter::background_sizing_property(),
                BackgroundSizing::InnerBorderEdge,
            )
            .template_bind(
                ContentPresenter::border_thickness_property(),
                TemplatedControl::border_thickness_property(),
            )
            .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
            .set(
                ContentPresenter::horizontal_content_alignment_property(),
                HorizontalAlignment::Stretch,
            )
            .set(
                ContentPresenter::vertical_content_alignment_property(),
                VerticalAlignment::Stretch,
            )
            .template_bind(
                ContentPresenter::corner_radius_property(),
                TemplatedControl::corner_radius_property(),
            )
            .control()])
        .control())
}

/// `TimePickerFlyoutButtonStyle`: a control theme of `Button`.
fn time_picker_flyout_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.template(time_picker_flyout_button_style_template);
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
                "TimePickerButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TimePickerButtonForegroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TimePickerButtonBorderBrushPointerOver",
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
                "TimePickerButtonBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TimePickerButtonForegroundPressed",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TimePickerButtonBorderBrushPressed",
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
                ContentPresenter::background_property(),
                "TimePickerButtonBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TimePickerButtonForegroundDisabled",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TimePickerButtonBorderBrushDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The focus adorner of `time_picker_theme`.
fn time_picker_theme_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-3.0))
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

/// The template of `TimePicker` of `time_picker_theme`.
fn time_picker_theme_template(t: &Template<TimePicker>) -> Result<Ref<Control>> {
    // The errors of FerroUI's data validation under the control (DataValidationErrors.xaml).
    Ok(t.make(DataValidationErrors::new())
        .content(
            ContentControl::content_property(),
            t.make(Grid::new())
                .name("LayoutRoot")?
                .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                .children([
                    t.make(Button::new())
                        .name("PART_FlyoutButton")?
                        .static_resource(StyledElement::theme_property(), "TimePickerFlyoutButtonStyle")?
                        .template_bind(
                            TemplatedControl::background_property(),
                            TemplatedControl::background_property(),
                        )
                        .template_bind(
                            TemplatedControl::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .template_bind(
                            TemplatedControl::border_brush_property(),
                            TemplatedControl::border_brush_property(),
                        )
                        .template_bind(
                            TemplatedControl::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .template_bind(InputElement::is_enabled_property(), InputElement::is_enabled_property())
                        .template_bind(Layoutable::min_width_property(), Layoutable::min_width_property())
                        .template_bind(Layoutable::max_width_property(), Layoutable::max_width_property())
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(
                            ContentControl::horizontal_content_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                        .template_bind(
                            TemplatedControl::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .set(TemplatedControl::is_template_focus_target_property(), true)
                        // The columns are set by the control.
                        .content(
                            ContentControl::content_property(),
                            t.make(Grid::new()).name("PART_FlyoutButtonContentGrid")?.children([
                                t.make(Border::new())
                                    .name("PART_FirstPickerHost")?
                                    .set(Grid::column_property(), 0)
                                    .child(
                                        Decorator::child_property(),
                                        t.make(TextBlock::new())
                                            .name("PART_HourTextBlock")?
                                            .set(TextBlock::text_property(), Some(String::from("hour")))
                                            .set(TextBlock::text_alignment_property(), TextAlignment::Center)
                                            .dynamic(TextBlock::padding_property(), "TimePickerHostPadding")
                                            .template_bind(
                                                TextBlock::font_family_property(),
                                                TemplatedControl::font_family_property(),
                                            )
                                            .template_bind(
                                                TextBlock::font_weight_property(),
                                                TemplatedControl::font_weight_property(),
                                            )
                                            .template_bind(
                                                TextBlock::font_size_property(),
                                                TemplatedControl::font_size_property(),
                                            ),
                                    )
                                    .control(),
                                t.make(Rectangle::new())
                                    .name("PART_FirstColumnDivider")?
                                    .dynamic(Shape::fill_property(), "TimePickerSpacerFill")
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                    .dynamic(Layoutable::width_property(), "TimePickerSpacerThemeWidth")
                                    .set(Grid::column_property(), 1)
                                    .control(),
                                t.make(Border::new())
                                    .name("PART_SecondPickerHost")?
                                    .set(Grid::column_property(), 2)
                                    .child(
                                        Decorator::child_property(),
                                        t.make(TextBlock::new())
                                            .name("PART_MinuteTextBlock")?
                                            .set(TextBlock::text_property(), Some(String::from("minute")))
                                            .set(TextBlock::text_alignment_property(), TextAlignment::Center)
                                            .dynamic(TextBlock::padding_property(), "TimePickerHostPadding")
                                            .template_bind(
                                                TextBlock::font_family_property(),
                                                TemplatedControl::font_family_property(),
                                            )
                                            .template_bind(
                                                TextBlock::font_weight_property(),
                                                TemplatedControl::font_weight_property(),
                                            )
                                            .template_bind(
                                                TextBlock::font_size_property(),
                                                TemplatedControl::font_size_property(),
                                            ),
                                    )
                                    .control(),
                                t.make(Rectangle::new())
                                    .name("PART_SecondColumnDivider")?
                                    .dynamic(Shape::fill_property(), "TimePickerSpacerFill")
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                    .dynamic(Layoutable::width_property(), "TimePickerSpacerThemeWidth")
                                    .set(Grid::column_property(), 3)
                                    .control(),
                                t.make(Border::new())
                                    .name("PART_ThirdPickerHost")?
                                    .set(Grid::column_property(), 4)
                                    .child(
                                        Decorator::child_property(),
                                        t.make(TextBlock::new())
                                            .name("PART_SecondTextBlock")?
                                            .set(TextBlock::text_property(), Some(String::from("second")))
                                            .set(TextBlock::text_alignment_property(), TextAlignment::Center)
                                            .dynamic(TextBlock::padding_property(), "TimePickerHostPadding")
                                            .template_bind(
                                                TextBlock::font_family_property(),
                                                TemplatedControl::font_family_property(),
                                            )
                                            .template_bind(
                                                TextBlock::font_weight_property(),
                                                TemplatedControl::font_weight_property(),
                                            )
                                            .template_bind(
                                                TextBlock::font_size_property(),
                                                TemplatedControl::font_size_property(),
                                            ),
                                    )
                                    .control(),
                                t.make(Rectangle::new())
                                    .name("PART_ThirdColumnDivider")?
                                    .dynamic(Shape::fill_property(), "TimePickerSpacerFill")
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                    .dynamic(Layoutable::width_property(), "TimePickerSpacerThemeWidth")
                                    .set(Grid::column_property(), 5)
                                    .control(),
                                t.make(Border::new())
                                    .name("PART_FourthPickerHost")?
                                    .set(Grid::column_property(), 6)
                                    .child(
                                        Decorator::child_property(),
                                        t.make(TextBlock::new())
                                            .name("PART_PeriodTextBlock")?
                                            .set(TextBlock::text_alignment_property(), TextAlignment::Center)
                                            .dynamic(TextBlock::padding_property(), "TimePickerHostPadding")
                                            .template_bind(
                                                TextBlock::font_family_property(),
                                                TemplatedControl::font_family_property(),
                                            )
                                            .template_bind(
                                                TextBlock::font_weight_property(),
                                                TemplatedControl::font_weight_property(),
                                            )
                                            .template_bind(
                                                TextBlock::font_size_property(),
                                                TemplatedControl::font_size_property(),
                                            ),
                                    )
                                    .control(),
                            ]),
                        )?
                        .control(),
                    t.make(Popup::new())
                        .name("PART_Popup")?
                        .set(Popup::window_manager_add_shadow_hint_property(), false)
                        .set(Popup::is_light_dismiss_enabled_property(), true)
                        .template_bind_with(Popup::placement_target_property(), None, None, None, None)
                        .set(Popup::placement_property(), PlacementMode::Bottom)
                        .child(
                            Popup::child_property(),
                            t.make(TimePickerPresenter::new())
                                .name("PART_PickerPresenter")?
                                .static_resource(Layoutable::margin_property(), "ThemeShadowFlyoutMargin")?,
                        )
                        .control(),
                ]),
        )?
        .control())
}

/// The default control theme of `TimePicker`.
fn time_picker_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TimePicker>();
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::background_property(), "DatePickerButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "TimePickerButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "DatePickerButtonBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "DatePickerBorderThemeThickness",
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    // MinWidth and MaxWidth of the flyout button in the source, bound to it from the control.
    theme.static_resource(Layoutable::min_width_property(), "TimePickerThemeMinWidth")?;
    theme.static_resource(Layoutable::max_width_property(), "TimePickerThemeMaxWidth")?;
    // UseSystemFocusVisuals with FocusVisualMargin="-3", around the flyout button.
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), time_picker_theme_focus_adorner)),
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(time_picker_theme_template);
    // The vertical alignment of the texts (FerroUI's VerticalContentAlignment of the control).
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("PART_FlyoutButton")
            .descendant()
            .of::<TextBlock>(),
        |style| {
            style.template_bind(
                Layoutable::vertical_alignment_property(),
                TimePicker::vertical_content_alignment_property(),
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("PART_FirstColumnDivider"),
            |style| {
                style.dynamic(Shape::fill_property(), "TimePickerSpacerFillDisabled");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Rectangle>().name("PART_SecondColumnDivider"),
            |style| {
                style.dynamic(Shape::fill_property(), "TimePickerSpacerFillDisabled");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Rectangle>().name("PART_ThirdColumnDivider"),
            |style| {
                style.dynamic(Shape::fill_property(), "TimePickerSpacerFillDisabled");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // HasNoTime
    theme.style(
        nesting()
            .class(":hasnotime")
            .template()
            .of::<Button>()
            .name("PART_FlyoutButton"),
        |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "TimePickerButtonForegroundDefault",
            );
            Ok(())
        },
    )?;
    // Not in WinUI: the :error of FerroUI's data validation (as the text box).
    theme.style(
        nesting()
            .class(":error")
            .template()
            .of::<Button>()
            .name("PART_FlyoutButton"),
        |style| {
            style.dynamic(
                TemplatedControl::border_brush_property(),
                "SystemFillColorCriticalBrush",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `TimePickerPresenter` of `time_picker_presenter_theme`.
fn time_picker_presenter_theme_template(t: &Template<TimePickerPresenter>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Background")?
        .set(Visual::translation_property(), literal(Vector3D::parse("0,0,32"))?)
        .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .dynamic(Decorator::padding_property(), "DateTimeFlyoutBorderPadding")
        .set(Layoutable::max_height_property(), 398.0)
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .name("ContentPanel")?
                .with(|e| {
                    e.set_row_definitions(literal(RowDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
                // The columns are set by the presenter.
                .children([
                    t.make(Grid::new())
                        .name("PART_PickerContainer")?
                        .style(of::<DateTimePickerPanel>().child().of::<ListBoxItem>(), |style| {
                            style.static_resource(
                                StyledElement::theme_property(),
                                "DateTimePickerLoopingSelectorItemStyle",
                            )?;
                            Ok(())
                        })?
                        .children([
                            t.make(Border::new())
                                .name("HighlightRect")?
                                .set(Visual::z_index_property(), -1)
                                .dynamic(Border::background_property(), "TimePickerFlyoutPresenterHighlightFill")
                                .template_bind(
                                    Border::corner_radius_property(),
                                    TemplatedControl::corner_radius_property(),
                                )
                                .set(Grid::column_property(), 0)
                                .set(Grid::column_span_property(), 7)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .dynamic(
                                    Layoutable::height_property(),
                                    "TimePickerFlyoutPresenterHighlightHeight",
                                )
                                .set(Layoutable::margin_property(), Thickness::new(4.0, 2.0, 4.0, 2.0))
                                .set(InputElement::is_hit_test_visible_property(), false)
                                .control(),
                            t.make(Grid::new())
                                .name("PART_HourHost")?
                                .set(Grid::column_property(), 0)
                                .children([
                                    t.make(ScrollViewer::new())
                                        .set(
                                            ScrollViewer::horizontal_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Disabled,
                                        )
                                        .set(
                                            ScrollViewer::vertical_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Hidden,
                                        )
                                        .static_resource(
                                            TemplatedControl::template_property(),
                                            "ScrollViewerScrollBarlessTemplate",
                                        )?
                                        .content(
                                            ContentControl::content_property(),
                                            t.make(DateTimePickerPanel::new())
                                                .name("PART_HourSelector")?
                                                .set(
                                                    DateTimePickerPanel::panel_type_property(),
                                                    DateTimePickerPanelType::Hour,
                                                )
                                                .set(DateTimePickerPanel::should_loop_property(), true)
                                                .dynamic(
                                                    DateTimePickerPanel::item_height_property(),
                                                    "TimePickerFlyoutPresenterItemHeight",
                                                ),
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_HourUpButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorUpButtonStyle",
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_HourDownButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorDownButtonStyle",
                                        )?
                                        .control(),
                                ])
                                .control(),
                            t.make(Rectangle::new())
                                .name("PART_FirstSpacer")?
                                .set(Visual::z_index_property(), 10000)
                                .dynamic(Shape::fill_property(), "TimePickerFlyoutPresenterSpacerFill")
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .dynamic(Layoutable::width_property(), "TimePickerSpacerThemeWidth")
                                .set(Grid::column_property(), 1)
                                .control(),
                            t.make(Grid::new())
                                .name("PART_MinuteHost")?
                                .set(Grid::column_property(), 2)
                                .children([
                                    t.make(ScrollViewer::new())
                                        .set(
                                            ScrollViewer::horizontal_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Disabled,
                                        )
                                        .set(
                                            ScrollViewer::vertical_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Hidden,
                                        )
                                        .static_resource(
                                            TemplatedControl::template_property(),
                                            "ScrollViewerScrollBarlessTemplate",
                                        )?
                                        .content(
                                            ContentControl::content_property(),
                                            t.make(DateTimePickerPanel::new())
                                                .name("PART_MinuteSelector")?
                                                .set(
                                                    DateTimePickerPanel::panel_type_property(),
                                                    DateTimePickerPanelType::Minute,
                                                )
                                                .set(DateTimePickerPanel::should_loop_property(), true)
                                                .dynamic(
                                                    DateTimePickerPanel::item_height_property(),
                                                    "TimePickerFlyoutPresenterItemHeight",
                                                ),
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_MinuteUpButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorUpButtonStyle",
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_MinuteDownButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorDownButtonStyle",
                                        )?
                                        .control(),
                                ])
                                .control(),
                            t.make(Rectangle::new())
                                .name("PART_SecondSpacer")?
                                .set(Visual::z_index_property(), 10000)
                                .dynamic(Shape::fill_property(), "TimePickerFlyoutPresenterSpacerFill")
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .dynamic(Layoutable::width_property(), "TimePickerSpacerThemeWidth")
                                .set(Grid::column_property(), 3)
                                .control(),
                            t.make(Grid::new())
                                .name("PART_SecondHost")?
                                .set(Grid::column_property(), 4)
                                .children([
                                    t.make(ScrollViewer::new())
                                        .set(
                                            ScrollViewer::horizontal_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Disabled,
                                        )
                                        .set(
                                            ScrollViewer::vertical_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Hidden,
                                        )
                                        .static_resource(
                                            TemplatedControl::template_property(),
                                            "ScrollViewerScrollBarlessTemplate",
                                        )?
                                        .content(
                                            ContentControl::content_property(),
                                            t.make(DateTimePickerPanel::new())
                                                .name("PART_SecondSelector")?
                                                .set(
                                                    DateTimePickerPanel::panel_type_property(),
                                                    DateTimePickerPanelType::Second,
                                                )
                                                .set(DateTimePickerPanel::should_loop_property(), true)
                                                .dynamic(
                                                    DateTimePickerPanel::item_height_property(),
                                                    "TimePickerFlyoutPresenterItemHeight",
                                                ),
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_SecondUpButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorUpButtonStyle",
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_SecondDownButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorDownButtonStyle",
                                        )?
                                        .control(),
                                ])
                                .control(),
                            t.make(Rectangle::new())
                                .name("PART_ThirdSpacer")?
                                .set(Visual::z_index_property(), 10000)
                                .dynamic(Shape::fill_property(), "TimePickerFlyoutPresenterSpacerFill")
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .dynamic(Layoutable::width_property(), "TimePickerSpacerThemeWidth")
                                .set(Grid::column_property(), 5)
                                .control(),
                            t.make(Grid::new())
                                .name("PART_PeriodHost")?
                                .set(Grid::column_property(), 6)
                                .children([
                                    t.make(ScrollViewer::new())
                                        .set(
                                            ScrollViewer::horizontal_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Disabled,
                                        )
                                        .set(
                                            ScrollViewer::vertical_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Hidden,
                                        )
                                        .static_resource(
                                            TemplatedControl::template_property(),
                                            "ScrollViewerScrollBarlessTemplate",
                                        )?
                                        .content(
                                            ContentControl::content_property(),
                                            t.make(DateTimePickerPanel::new())
                                                .name("PART_PeriodSelector")?
                                                .set(
                                                    DateTimePickerPanel::panel_type_property(),
                                                    DateTimePickerPanelType::TimePeriod,
                                                )
                                                .set(DateTimePickerPanel::should_loop_property(), false)
                                                .dynamic(
                                                    DateTimePickerPanel::item_height_property(),
                                                    "TimePickerFlyoutPresenterItemHeight",
                                                ),
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_PeriodUpButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorUpButtonStyle",
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_PeriodDownButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorDownButtonStyle",
                                        )?
                                        .control(),
                                ])
                                .control(),
                        ])
                        .control(),
                    t.make(Grid::new())
                        .name("AcceptDismissHostGrid")?
                        .set(Grid::row_property(), 1)
                        .dynamic(
                            Layoutable::height_property(),
                            "TimePickerFlyoutPresenterAcceptDismissHostGridHeight",
                        )
                        .with(|e| {
                            e.set_column_definitions(literal(ColumnDefinitions::parse("*,*"))?);
                            Ok(())
                        })?
                        .children([
                            t.make(Rectangle::new())
                                .dynamic(Layoutable::height_property(), "TimePickerSpacerThemeWidth")
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                .dynamic(Shape::fill_property(), "TimePickerFlyoutPresenterSpacerFill")
                                .set(Grid::column_span_property(), 2)
                                .control(),
                            t.make(Button::new())
                                .name("PART_AcceptButton")?
                                .set(Grid::column_property(), 0)
                                .static_resource(StyledElement::theme_property(), "DateTimePickerFlyoutButtonStyle")?
                                .set(TemplatedControl::font_size_property(), 16.0)
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                                .set(
                                    ContentControl::horizontal_content_alignment_property(),
                                    HorizontalAlignment::Center,
                                )
                                .set(
                                    ContentControl::vertical_content_alignment_property(),
                                    VerticalAlignment::Center,
                                )
                                .static_resource(
                                    Layoutable::margin_property(),
                                    "DatePickerFlyoutPresenterDismissMargin",
                                )?
                                .set(TemplatedControl::padding_property(), Thickness::uniform(4.0))
                                // E8FB at 16 px.
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Path::new())
                                        .static_resource(Path::data_property(), "GlyphGeometryE8FB")?
                                        .bind(
                                            Shape::fill_property(),
                                            path()
                                                .ancestor_of::<Button>(0)
                                                .prop(TemplatedControl::foreground_property())
                                                .binding(),
                                        )
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .set(Layoutable::width_property(), 14.0),
                                )?
                                .control(),
                            t.make(Button::new())
                                .name("PART_DismissButton")?
                                .set(Grid::column_property(), 1)
                                .static_resource(StyledElement::theme_property(), "DateTimePickerFlyoutButtonStyle")?
                                .set(TemplatedControl::font_size_property(), 16.0)
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                                .set(
                                    ContentControl::horizontal_content_alignment_property(),
                                    HorizontalAlignment::Center,
                                )
                                .set(
                                    ContentControl::vertical_content_alignment_property(),
                                    VerticalAlignment::Center,
                                )
                                .static_resource(
                                    Layoutable::margin_property(),
                                    "DatePickerFlyoutPresenterDismissMargin",
                                )?
                                .set(TemplatedControl::padding_property(), Thickness::uniform(4.0))
                                // E711 at 16 px.
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Path::new())
                                        .static_resource(Path::data_property(), "GlyphGeometryE711")?
                                        .bind(
                                            Shape::fill_property(),
                                            path()
                                                .ancestor_of::<Button>(0)
                                                .prop(TemplatedControl::foreground_property())
                                                .binding(),
                                        )
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .set(Layoutable::width_property(), 12.0),
                                )?
                                .control(),
                        ])
                        .control(),
                ]),
        )
        .control())
}

/// The default control theme of `TimePickerPresenter`.
fn time_picker_presenter_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TimePickerPresenter>();
    // The shadow is drawn outside the bounds of the control: a templated control clips to its
    // bounds unless its theme says otherwise (G-24).
    theme.set(Visual::clip_to_bounds_property(), false);
    theme.set(Layoutable::width_property(), 242.0);
    theme.set(Layoutable::min_width_property(), 242.0);
    theme.set(Layoutable::max_height_property(), 398.0);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(
        TemplatedControl::background_property(),
        "TimePickerFlyoutPresenterBackground",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "TimePickerFlyoutPresenterBorderBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "DateTimeFlyoutBorderThickness",
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(time_picker_presenter_theme_template);
    // PointerOver of DefaultLoopingSelectorStyle: the up and down buttons of the host under the pointer.
    theme.style(
        nesting()
            .template()
            .of::<Grid>()
            .name("PART_HourHost")
            .class(":pointerover")
            .child()
            .of::<RepeatButton>(),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .template()
            .of::<Grid>()
            .name("PART_MinuteHost")
            .class(":pointerover")
            .child()
            .of::<RepeatButton>(),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .template()
            .of::<Grid>()
            .name("PART_SecondHost")
            .class(":pointerover")
            .child()
            .of::<RepeatButton>(),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .template()
            .of::<Grid>()
            .name("PART_PeriodHost")
            .class(":pointerover")
            .child()
            .of::<RepeatButton>(),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    Ok(())
}
