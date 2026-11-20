//! `Controls/DatePicker.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/DatePicker_themeresources.xaml (DefaultDatePickerStyle with its
//!   template-local DatePickerFlyoutButtonStyle, DefaultDatePickerFlyoutPresenterStyle and the
//!   implicit styles of DatePicker and DatePickerFlyoutPresenter; its resources are
//!   Resources/DatePicker_themeresources.xaml)
//!   controls/dev/CommonStyles/DateTimePickerFlyout_themeresources.xaml (the template of
//!   DefaultLoopingSelectorStyle, written into the hosts of the presenter; the themes of the buttons
//!   and the items are in Controls/DateTimePickerShared.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts of DatePicker: FlyoutButton, FlyoutButtonContentGrid, DayTextBlock,
//!   MonthTextBlock, YearTextBlock, FirstPickerSpacing and SecondPickerSpacing are PART_FlyoutButton,
//!   PART_ButtonContentGrid, PART_DayTextBlock, PART_MonthTextBlock, PART_YearTextBlock,
//!   PART_FirstSpacer and PART_SecondSpacer. FerroUI's control sets the columns of the grid and
//!   their order from the date pattern of the culture, as WinUI's does (78*, 132* for the month).
//! - WinUI's DatePickerFlyout is a flyout the control creates in code; FerroUI's DatePicker has the
//!   Popup PART_Popup with the DatePickerPresenter PART_PickerPresenter in its template. The control
//!   places the popup so that the selected values are over the button, as WinUI places its flyout.
//! - The texts of the placeholders are the localized strings of WinUI ("day", "month", "year",
//!   TEXT_DATEPICKER_*_PLACEHOLDER of dxaml/xcp/dxaml/dllsrv/winrt/Microsoft.UI.Xaml.Common.rc) in
//!   English; the control clears the text it sets when it has no date, which restores them.
//! - The template-local DatePickerFlyoutButtonStyle is the keyed theme of the same name; its
//!   ContentPresenter is PART_ContentPresenter.
//! - HasDateStates: HasNoDate is FerroUI's :hasnodate.
//! - The MinWidth and MaxWidth of the flyout button (DatePickerThemeMinWidth, DatePickerThemeMaxWidth) are
//!   setters of the control bound to the button, so that the sizes of FerroUI's control apply.
//! - The focus visual: IsTabStop False on the control and UseSystemFocusVisuals with
//!   FocusVisualMargin -3 on the flyout button. FerroUI's DatePicker takes the focus itself: the
//!   FocusAdorner of the control (margin -3) is drawn around the flyout button
//!   (TemplatedControl.IsTemplateFocusTarget), and IsTabStop is left out.
//! - HeaderContentPresenter and its row are left out: FerroUI's DatePicker has no Header.
//!   Orientation (Horizontal) and ElementSoundMode have no counterpart.
//! - The FerroUI contract: the template is wrapped in DataValidationErrors, which shows the errors
//!   of FerroUI's data validation under the control, and :error gives the border the critical brush
//!   (SystemFillColorCriticalBrush), as the Fluent theme does and as Controls/TextBox.xaml does.
//! - Template parts of DatePickerPresenter: PickerHostGrid is PART_PickerContainer; the
//!   LoopingSelectors the WinUI presenter creates in code are the hosts PART_DayHost, PART_MonthHost
//!   and PART_YearHost, each with the template of DefaultLoopingSelectorStyle (a ScrollViewer with
//!   ScrollViewerScrollBarlessTemplate around the DateTimePickerPanel PART_DaySelector, ... and the
//!   up and down RepeatButtons PART_DayUpButton, PART_DayDownButton, ..., shown while the pointer is
//!   over the host); AcceptButton and DismissButton are PART_AcceptButton and PART_DismissButton;
//!   FirstPickerSpacing and SecondPickerSpacing are PART_FirstSpacer and PART_SecondSpacer. The
//!   presenter sets the columns in code (78*, 138* for the month), the month column is 138* in
//!   FerroUI where WinUI's is 132*.
//! - HighlightRect (a Grid with a background and a corner radius) is a Border, under the items; the
//!   three MonochromaticOverlayPresenters that draw the selected items again in
//!   DatePickerFlyoutPresenterHighlightForegroundColor are the selected state of the items
//!   (Controls/DateTimePickerShared.xaml).
//! - The glyphs of the buttons (E8FB Accept, E711 Cancel at 16 px) are GlyphGeometryE8FB (14 px
//!   wide) and GlyphGeometryE711 (12 px wide).
//! - The ThemeShadow of the flyout (ApplyElevationEffect of DatePickerFlyoutPresenter at a
//!   Translation.Z of 32) is a ThemeShadow (the shared WinUIThemeShadow, Translation.Z as in WinUI) on the border Background of
//!   the presenter, and the margin ThemeShadowFlyoutMargin of PART_PickerPresenter is the room for
//!   it in the popup (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24); WindowManagerAddShadowHint
//!   is off, as in the Fluent theme.

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
use ferroui_controls::DatePicker;
use ferroui_controls::DatePickerPresenter;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ListBoxItem;
use ferroui_controls::Panel;
use ferroui_controls::PlacementMode;
use ferroui_controls::RepeatButton;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // DatePickerFlyoutButtonStyle of the template of DefaultDatePickerStyle.
    resources.control_theme("DatePickerFlyoutButtonStyle", date_picker_flyout_button_style);
    resources.control_theme("DefaultDatePickerStyle", default_date_picker_style);
    resources.control_theme(type_key::<DatePicker>(), date_picker_theme);
    resources.control_theme(
        "DefaultDatePickerFlyoutPresenterStyle",
        default_date_picker_flyout_presenter_style,
    );
    resources.control_theme(type_key::<DatePickerPresenter>(), date_picker_presenter_theme);
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

/// The template of `Button` of `date_picker_flyout_button_style`.
fn date_picker_flyout_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
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

/// `DatePickerFlyoutButtonStyle`: a control theme of `Button`.
fn date_picker_flyout_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.template(date_picker_flyout_button_style_template);
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
                "DatePickerButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "DatePickerButtonForegroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "DatePickerButtonBorderBrushPointerOver",
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
                "DatePickerButtonBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "DatePickerButtonForegroundPressed",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "DatePickerButtonBorderBrushPressed",
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
                "DatePickerButtonBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "DatePickerButtonForegroundDisabled",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "DatePickerButtonBorderBrushDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The focus adorner of `default_date_picker_style`.
fn default_date_picker_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `DatePicker` of `default_date_picker_style`.
fn default_date_picker_style_template(t: &Template<DatePicker>) -> Result<Ref<Control>> {
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
                        .static_resource(StyledElement::theme_property(), "DatePickerFlyoutButtonStyle")?
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
                        .content(
                            ContentControl::content_property(),
                            t.make(Grid::new())
                                .name("PART_ButtonContentGrid")?
                                .with(|e| {
                                    Grid::set_column_definitions(
                                        e,
                                        literal(ColumnDefinitions::parse("78*,Auto,132*,Auto,78*"))?,
                                    );
                                    Ok(())
                                })?
                                .children([
                                    t.make(TextBlock::new())
                                        .name("PART_DayTextBlock")?
                                        .set(TextBlock::text_property(), Some(String::from("day")))
                                        .set(TextBlock::text_alignment_property(), TextAlignment::Center)
                                        .dynamic(TextBlock::padding_property(), "DatePickerHostPadding")
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
                                        )
                                        .control(),
                                    t.make(TextBlock::new())
                                        .name("PART_MonthTextBlock")?
                                        .set(TextBlock::text_property(), Some(String::from("month")))
                                        .set(TextBlock::text_alignment_property(), TextAlignment::Left)
                                        .dynamic(TextBlock::padding_property(), "DatePickerHostMonthPadding")
                                        .set(Layoutable::margin_property(), Thickness::new(1.0, 0.0, 0.0, 0.0))
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
                                        )
                                        .control(),
                                    t.make(TextBlock::new())
                                        .name("PART_YearTextBlock")?
                                        .set(TextBlock::text_property(), Some(String::from("year")))
                                        .set(TextBlock::text_alignment_property(), TextAlignment::Center)
                                        .dynamic(TextBlock::padding_property(), "DatePickerHostPadding")
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
                                        )
                                        .control(),
                                    t.make(Rectangle::new())
                                        .name("PART_FirstSpacer")?
                                        .dynamic(Shape::fill_property(), "DatePickerSpacerFill")
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                        .dynamic(Layoutable::width_property(), "DatePickerSpacerThemeWidth")
                                        .set(Grid::column_property(), 1)
                                        .control(),
                                    t.make(Rectangle::new())
                                        .name("PART_SecondSpacer")?
                                        .dynamic(Shape::fill_property(), "DatePickerSpacerFill")
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                        .dynamic(Layoutable::width_property(), "DatePickerSpacerThemeWidth")
                                        .set(Grid::column_property(), 3)
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
                            t.make(DatePickerPresenter::new())
                                .name("PART_PickerPresenter")?
                                .static_resource(Layoutable::margin_property(), "ThemeShadowFlyoutMargin")?,
                        )
                        .control(),
                ]),
        )?
        .control())
}

/// `DefaultDatePickerStyle`: a control theme of `DatePicker`.
fn default_date_picker_style(theme: &StyleBuilder) -> Result {
    theme.target::<DatePicker>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    // (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        DatePicker::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::background_property(), "DatePickerButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "DatePickerButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "DatePickerButtonBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "DatePickerBorderThemeThickness",
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    // MinWidth and MaxWidth of the flyout button in the source, bound to it from the control.
    theme.static_resource(Layoutable::min_width_property(), "DatePickerThemeMinWidth")?;
    theme.static_resource(Layoutable::max_width_property(), "DatePickerThemeMaxWidth")?;
    // UseSystemFocusVisuals with FocusVisualMargin="-3", around the flyout button.
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_date_picker_style_focus_adorner)),
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(default_date_picker_style_template);
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
                DatePicker::vertical_content_alignment_property(),
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("PART_FirstSpacer"),
            |style| {
                style.dynamic(Shape::fill_property(), "DatePickerSpacerFillDisabled");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Rectangle>().name("PART_SecondSpacer"),
            |style| {
                style.dynamic(Shape::fill_property(), "DatePickerSpacerFillDisabled");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // HasNoDate
    theme.style(
        nesting()
            .class(":hasnodate")
            .template()
            .of::<Button>()
            .name("PART_FlyoutButton"),
        |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "DatePickerButtonForegroundDefault",
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

/// The default control theme of `DatePicker`.
fn date_picker_theme(theme: &StyleBuilder) -> Result {
    theme.target::<DatePicker>();
    theme.based_on("DefaultDatePickerStyle")?;
    Ok(())
}

/// The template of `DatePickerPresenter` of `default_date_picker_flyout_presenter_style`.
fn default_date_picker_flyout_presenter_style_template(t: &Template<DatePickerPresenter>) -> Result<Ref<Control>> {
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
                    Grid::set_row_definitions(e, literal(RowDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
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
                        // The columns are set by the presenter.
                        .children([
                            t.make(Border::new())
                                .name("HighlightRect")?
                                .set(Visual::z_index_property(), -1)
                                .dynamic(Border::background_property(), "DatePickerFlyoutPresenterHighlightFill")
                                .template_bind(
                                    Border::corner_radius_property(),
                                    TemplatedControl::corner_radius_property(),
                                )
                                .set(Grid::column_property(), 0)
                                .set(Grid::column_span_property(), 5)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .dynamic(
                                    Layoutable::height_property(),
                                    "DatePickerFlyoutPresenterHighlightHeight",
                                )
                                .set(Layoutable::margin_property(), Thickness::new(4.0, 2.0, 4.0, 2.0))
                                .set(InputElement::is_hit_test_visible_property(), false)
                                .control(),
                            t.make(Grid::new())
                                .name("PART_MonthHost")?
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
                                                .name("PART_MonthSelector")?
                                                .set(
                                                    DateTimePickerPanel::panel_type_property(),
                                                    DateTimePickerPanelType::Month,
                                                )
                                                .dynamic(
                                                    DateTimePickerPanel::item_height_property(),
                                                    "DatePickerFlyoutPresenterItemHeight",
                                                )
                                                .set(DateTimePickerPanel::should_loop_property(), true),
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_MonthUpButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorUpButtonStyle",
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_MonthDownButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorDownButtonStyle",
                                        )?
                                        .control(),
                                ])
                                .control(),
                            t.make(Grid::new())
                                .name("PART_DayHost")?
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
                                                .name("PART_DaySelector")?
                                                .set(
                                                    DateTimePickerPanel::panel_type_property(),
                                                    DateTimePickerPanelType::Day,
                                                )
                                                .dynamic(
                                                    DateTimePickerPanel::item_height_property(),
                                                    "DatePickerFlyoutPresenterItemHeight",
                                                )
                                                .set(DateTimePickerPanel::should_loop_property(), true),
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_DayUpButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorUpButtonStyle",
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_DayDownButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorDownButtonStyle",
                                        )?
                                        .control(),
                                ])
                                .control(),
                            t.make(Grid::new())
                                .name("PART_YearHost")?
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
                                                .name("PART_YearSelector")?
                                                .set(
                                                    DateTimePickerPanel::panel_type_property(),
                                                    DateTimePickerPanelType::Year,
                                                )
                                                .dynamic(
                                                    DateTimePickerPanel::item_height_property(),
                                                    "DatePickerFlyoutPresenterItemHeight",
                                                )
                                                .set(DateTimePickerPanel::should_loop_property(), false),
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_YearUpButton")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "DateTimePickerLoopingSelectorUpButtonStyle",
                                        )?
                                        .control(),
                                    t.make(RepeatButton::new())
                                        .name("PART_YearDownButton")?
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
                                .dynamic(Shape::fill_property(), "DatePickerFlyoutPresenterSpacerFill")
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .dynamic(Layoutable::width_property(), "DatePickerSpacerThemeWidth")
                                .set(Grid::column_property(), 1)
                                .control(),
                            t.make(Rectangle::new())
                                .name("PART_SecondSpacer")?
                                .set(Visual::z_index_property(), 10000)
                                .dynamic(Shape::fill_property(), "DatePickerFlyoutPresenterSpacerFill")
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .dynamic(Layoutable::width_property(), "DatePickerSpacerThemeWidth")
                                .set(Grid::column_property(), 3)
                                .control(),
                        ])
                        .control(),
                    t.make(Grid::new())
                        .name("AcceptDismissHostGrid")?
                        .set(Grid::row_property(), 1)
                        .dynamic(
                            Layoutable::height_property(),
                            "DatePickerFlyoutPresenterAcceptDismissHostGridHeight",
                        )
                        .with(|e| {
                            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,*"))?);
                            Ok(())
                        })?
                        .children([
                            t.make(Rectangle::new())
                                .dynamic(Layoutable::height_property(), "DatePickerSpacerThemeWidth")
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                .dynamic(Shape::fill_property(), "DatePickerFlyoutPresenterSpacerFill")
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
                                    "DatePickerFlyoutPresenterAcceptMargin",
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

/// `DefaultDatePickerFlyoutPresenterStyle`: a control theme of `DatePickerPresenter`.
fn default_date_picker_flyout_presenter_style(theme: &StyleBuilder) -> Result {
    theme.target::<DatePickerPresenter>();
    // The shadow is drawn outside the bounds of the control: a templated control clips to its
    // bounds unless its theme says otherwise (G-24).
    theme.set(Visual::clip_to_bounds_property(), false);
    theme.set(Layoutable::width_property(), 296.0);
    theme.set(Layoutable::min_width_property(), 296.0);
    theme.set(Layoutable::max_height_property(), 398.0);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(
        TemplatedControl::background_property(),
        "DatePickerFlyoutPresenterBackground",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "DatePickerFlyoutPresenterBorderBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "DateTimeFlyoutBorderThickness",
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(default_date_picker_flyout_presenter_style_template);
    // PointerOver of DefaultLoopingSelectorStyle: the up and down buttons of the host under the pointer.
    theme.style(
        nesting()
            .template()
            .of::<Grid>()
            .name("PART_MonthHost")
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
            .name("PART_DayHost")
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
            .name("PART_YearHost")
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

/// The default control theme of `DatePickerPresenter`.
fn date_picker_presenter_theme(theme: &StyleBuilder) -> Result {
    theme.target::<DatePickerPresenter>();
    theme.based_on("DefaultDatePickerFlyoutPresenterStyle")?;
    Ok(())
}
