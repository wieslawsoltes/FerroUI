//! `Controls/CalendarDatePicker.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/CalendarDatePicker_themeresources.xaml (DefaultCalendarDatePicker
//!   and the implicit style of CalendarDatePicker; its resources are
//!   Resources/CalendarDatePicker_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - WinUI's picker shows the date in a TextBlock (DateText) and opens its calendar when it is
//!   clicked anywhere. FerroUI's CalendarDatePicker edits the date in a text box (PART_TextBox) and
//!   opens the calendar with a button (PART_Button): DateText is the text box, with the keyed theme
//!   CalendarDatePickerTextBoxStyle (a transparent text box without a border, the placeholder in its
//!   foreground as the TextBlock of the source shows it, and the floating placeholder of FerroUI's
//!   UseFloatingPlaceholder above the text), and the glyph is the content of the button,
//!   with the keyed theme CalendarDatePickerButtonStyle (a transparent button of the 32 px column
//!   of the glyph).
//! - The padding 12,0,0,2 of DateText is the Padding of the control (a setter), bound to the text
//!   box, so that the Padding of FerroUI's control applies.
//! - The glyph E787 (Calendar) of FontIcon CalendarGlyph at 12 px is the geometry
//!   GlyphGeometryE787, 10.5 px wide (the ink of the glyph at 12 px).
//! - DateText lies in the column of the border, not inside it: the text box has no margin of the
//!   border thickness, so its text starts 12 px from the edge of the control, as WinUI's.
//! - The Flyout of FlyoutBase.AttachedFlyout is the Popup PART_Popup (light dismiss, below the
//!   control) with the Calendar PART_Calendar, whose corner radius is OverlayCornerRadius as in the
//!   source; the FlyoutPresenterStyle of the source (no padding, no border) leaves the calendar as
//!   it is, so the popup has no presenter. CalendarViewStyle is the default theme of Calendar.
//!   The shadow of that style (IsDefaultShadowEnabled: a ThemeShadow at a Translation.Z of 32) is
//!   a ThemeShadow (the shared WinUIThemeShadow, Translation.Z 32) on the Border CalendarShadow around the calendar,
//!   with the margin ThemeShadowFlyoutMargin as the room for it in the popup
//!   (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24).
//! - The Border Background is the focus target (Control.IsTemplateFocusTarget) of the focus visual,
//!   FocusVisualMargin -3: the FocusAdorner of the control, drawn around the border.
//! - HeaderContentPresenter, DescriptionPresenter, their rows and HeaderStates are left out:
//!   FerroUI's CalendarDatePicker has no Header and no Description.
//! - SelectionStates (Selected: the date text and the glyph in CalendarDatePickerTextForegroundSelected
//!   while a date is selected) is not ported: FerroUI's control has no state for a selected date;
//!   the brush is TextFillColorPrimaryBrush, the one of the normal state.
//! - CalendarDatePickerLightDismissOverlayBackground is not used: FerroUI's popups have no light
//!   dismiss overlay.
//! - The FerroUI contract: the template is wrapped in DataValidationErrors, which shows the errors
//!   of FerroUI's data validation under the control, and :error gives the border the critical brush
//!   (SystemFillColorCriticalBrush), as the Fluent theme does and as Controls/TextBox.xaml does.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::BoolConverters;
use ferroui_base::data::converters::StringConverters;
use ferroui_base::data::BindingMode;
use ferroui_base::data::MultiBinding;
use ferroui_base::input::Cursor;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::Stretch;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::Calendar;
use ferroui_controls::CalendarDatePicker;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::DataValidationErrors;
use ferroui_controls::Decorator;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_controls::PlacementMode;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TextBlock;
use ferroui_controls::TextBox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // DateText of the template of DefaultCalendarDatePicker, as the text box FerroUI's picker edits.
    resources.control_theme("CalendarDatePickerTextBoxStyle", calendar_date_picker_text_box_style);
    // The column of CalendarGlyph of the template of DefaultCalendarDatePicker, as the button FerroUI's picker opens its calendar with.
    resources.control_theme("CalendarDatePickerButtonStyle", calendar_date_picker_button_style);
    resources.control_theme("DefaultCalendarDatePicker", default_calendar_date_picker);
    resources.control_theme(type_key::<CalendarDatePicker>(), calendar_date_picker_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `TextBox` of `calendar_date_picker_text_box_style`.
fn calendar_date_picker_text_box_style_template(t: &Template<TextBox>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .children([t
            .make(DockPanel::new())
            .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
            // FerroUI's floating placeholder, above the text while there is text.
            .children([
                t.make(TextBlock::new())
                    .name("PART_FloatingPlaceholder")?
                    .set(DockPanel::dock_property(), Dock::Top)
                    .template_bind(
                        TextBlock::foreground_property(),
                        TextBox::placeholder_foreground_property(),
                    )
                    .template_bind(TextBlock::text_property(), TextBox::placeholder_text_property())
                    .set(Visual::is_visible_property(), false)
                    .control(),
                t.make(ScrollViewer::new())
                    .name("PART_ScrollViewer")?
                    .template_bind(
                        ScrollViewer::horizontal_scroll_bar_visibility_property(),
                        ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    )
                    .template_bind(
                        ScrollViewer::vertical_scroll_bar_visibility_property(),
                        ScrollViewer::vertical_scroll_bar_visibility_property(),
                    )
                    .template_bind(
                        TemplatedControl::foreground_property(),
                        TemplatedControl::foreground_property(),
                    )
                    .set(InputElement::focusable_property(), false)
                    .style(of::<ScrollContentPresenter>().name("PART_ContentPresenter"), |style| {
                        style.set(InputElement::cursor_property(), Some(literal(Cursor::parse("IBeam"))?));
                        Ok(())
                    })?
                    .content(
                        ContentControl::content_property(),
                        t.make(Panel::new()).children([
                            t.make(TextBlock::new())
                                .name("PlaceholderTextContentPresenter")?
                                .template_bind(
                                    TextBlock::foreground_property(),
                                    TextBox::placeholder_foreground_property(),
                                )
                                .template_bind(TextBlock::text_property(), TextBox::placeholder_text_property())
                                .template_bind(TextBlock::text_alignment_property(), TextBox::text_alignment_property())
                                .template_bind(TextBlock::text_wrapping_property(), TextBox::text_wrapping_property())
                                .template_bind(
                                    Layoutable::horizontal_alignment_property(),
                                    TextBox::horizontal_content_alignment_property(),
                                )
                                .template_bind(
                                    Layoutable::vertical_alignment_property(),
                                    TextBox::vertical_content_alignment_property(),
                                )
                                .set(InputElement::is_hit_test_visible_property(), false)
                                .bind(Visual::is_visible_property(), {
                                    let binding = MultiBinding::new().with_converter_value(Some(BoolConverters::and()));
                                    let bindings = binding.bindings();
                                    bindings.add(
                                        path()
                                            .named(t.names(), "PART_TextPresenter")
                                            .prop(TextPresenter::preedit_text_property())
                                            .binding()
                                            .with_converter_value(Some(StringConverters::is_null_or_empty())),
                                    );
                                    bindings.add(
                                        path()
                                            .templated_parent()
                                            .prop(TextBox::text_property())
                                            .binding()
                                            .with_converter_value(Some(StringConverters::is_null_or_empty())),
                                    );
                                    binding
                                })
                                .control(),
                            t.make(TextPresenter::new())
                                .name("PART_TextPresenter")?
                                .template_bind_with(
                                    TextPresenter::text_property(),
                                    Some(TextBox::text_property().property()),
                                    None,
                                    Some(BindingMode::TwoWay),
                                    None,
                                )
                                .template_bind(
                                    TextPresenter::caret_blink_interval_property(),
                                    TextBox::caret_blink_interval_property(),
                                )
                                .template_bind(TextPresenter::caret_index_property(), TextBox::caret_index_property())
                                .template_bind(
                                    TextPresenter::selection_start_property(),
                                    TextBox::selection_start_property(),
                                )
                                .template_bind(
                                    TextPresenter::selection_end_property(),
                                    TextBox::selection_end_property(),
                                )
                                .template_bind(
                                    TextPresenter::text_alignment_property(),
                                    TextBox::text_alignment_property(),
                                )
                                .template_bind(
                                    TextPresenter::text_wrapping_property(),
                                    TextBox::text_wrapping_property(),
                                )
                                .template_bind(TextPresenter::line_height_property(), TextBox::line_height_property())
                                .template_bind(
                                    TextPresenter::letter_spacing_property(),
                                    TemplatedControl::letter_spacing_property(),
                                )
                                .template_bind(
                                    TextPresenter::password_char_property(),
                                    TextBox::password_char_property(),
                                )
                                .template_bind(
                                    TextPresenter::reveal_password_property(),
                                    TextBox::reveal_password_property(),
                                )
                                .template_bind(
                                    TextPresenter::selection_brush_property(),
                                    TextBox::selection_brush_property(),
                                )
                                .template_bind(
                                    TextPresenter::selection_foreground_brush_property(),
                                    TextBox::selection_foreground_brush_property(),
                                )
                                .template_bind(TextPresenter::caret_brush_property(), TextBox::caret_brush_property())
                                .template_bind(
                                    Layoutable::horizontal_alignment_property(),
                                    TextBox::horizontal_content_alignment_property(),
                                )
                                .template_bind(
                                    Layoutable::vertical_alignment_property(),
                                    TextBox::vertical_content_alignment_property(),
                                )
                                .control(),
                        ]),
                    )?
                    .control(),
            ])
            .control()])
        .control())
}

/// `CalendarDatePickerTextBoxStyle`: a control theme of `TextBox`.
fn calendar_date_picker_text_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBox>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    //         (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        TextBox::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(
        TemplatedControl::border_brush_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.dynamic(
        TextBox::selection_brush_property(),
        "TextControlSelectionHighlightColor",
    );
    theme.dynamic(TextBox::caret_brush_property(), "TextControlForegroundFocused");
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Hidden,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Hidden,
    );
    theme.set_null(Control::focus_adorner_property());
    theme.template(calendar_date_picker_text_box_style_template);
    theme.style(
        nesting()
            .property_is(TextBox::use_floating_placeholder_property(), true)
            .not(of::<TextBox>().class(":empty"))
            .template()
            .of::<TextBlock>()
            .name("PART_FloatingPlaceholder"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Button` of `calendar_date_picker_button_style`.
fn calendar_date_picker_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .control())
}

/// `CalendarDatePickerButtonStyle`: a control theme of `Button`.
fn calendar_date_picker_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(InputElement::focusable_property(), false);
    theme.set_null(Control::focus_adorner_property());
    theme.template(calendar_date_picker_button_style_template);
    Ok(())
}

/// The focus adorner of `default_calendar_date_picker`.
fn default_calendar_date_picker_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `CalendarDatePicker` of `default_calendar_date_picker`.
fn default_calendar_date_picker_template(t: &Template<CalendarDatePicker>) -> Result<Ref<Control>> {
    // The errors of FerroUI's data validation under the control (DataValidationErrors.xaml).
    Ok(t.make(DataValidationErrors::new())
        .content(
            ContentControl::content_property(),
            t.make(Grid::new())
                .name("Root")?
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,32"))?);
                    Ok(())
                })?
                .children([
                    t.make(Border::new())
                        .name("Background")?
                        .set(Grid::column_span_property(), 2)
                        .template_bind(
                            Border::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .template_bind(
                            Border::border_brush_property(),
                            TemplatedControl::border_brush_property(),
                        )
                        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                        .template_bind(Border::background_property(), TemplatedControl::background_property())
                        .template_bind(
                            Border::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .set(TemplatedControl::is_template_focus_target_property(), true)
                        .set(Layoutable::min_height_property(), 32.0)
                        .control(),
                    t.make(TextBox::new())
                        .name("PART_TextBox")?
                        .set(Grid::column_property(), 0)
                        .static_resource(StyledElement::theme_property(), "CalendarDatePickerTextBoxStyle")?
                        .template_bind(
                            TemplatedControl::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .template_bind(
                            TemplatedControl::padding_property(),
                            TemplatedControl::padding_property(),
                        )
                        .template_bind(
                            TextBox::placeholder_text_property(),
                            CalendarDatePicker::placeholder_text_property(),
                        )
                        .template_bind(
                            TextBox::placeholder_foreground_property(),
                            CalendarDatePicker::placeholder_foreground_property(),
                        )
                        .template_bind(
                            TextBox::use_floating_placeholder_property(),
                            CalendarDatePicker::use_floating_placeholder_property(),
                        )
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                        .template_bind(
                            TextBox::horizontal_content_alignment_property(),
                            CalendarDatePicker::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            TextBox::vertical_content_alignment_property(),
                            CalendarDatePicker::vertical_content_alignment_property(),
                        )
                        .control(),
                    t.make(Button::new())
                        .name("PART_Button")?
                        .set(Grid::column_property(), 1)
                        .static_resource(StyledElement::theme_property(), "CalendarDatePickerButtonStyle")?
                        // E787 at 12 px: the glyph is drawn at the size of its ink.
                        .content(
                            ContentControl::content_property(),
                            t.make(Path::new())
                                .name("CalendarGlyph")?
                                .static_resource(Path::data_property(), "GlyphGeometryE787")?
                                .dynamic(Shape::fill_property(), "CalendarDatePickerCalendarGlyphForeground")
                                .set(Shape::stretch_property(), Stretch::Uniform)
                                .set(Layoutable::width_property(), 10.5)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
                        )?
                        .control(),
                    t.make(Popup::new())
                        .name("PART_Popup")?
                        .set(Popup::window_manager_add_shadow_hint_property(), false)
                        .template_bind_with(Popup::placement_target_property(), None, None, None, None)
                        .set(Popup::placement_property(), PlacementMode::Bottom)
                        .set(Popup::is_light_dismiss_enabled_property(), true)
                        .child(
                            Popup::child_property(),
                            t.make(Border::new())
                                .name("CalendarShadow")?
                                .set(Visual::translation_property(), literal(Vector3D::parse("0,0,32"))?)
                                .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
                                .dynamic(Border::corner_radius_property(), "OverlayCornerRadius")
                                .static_resource(Layoutable::margin_property(), "ThemeShadowFlyoutMargin")?
                                .child(
                                    Decorator::child_property(),
                                    t.make(Calendar::new())
                                        .name("PART_Calendar")?
                                        .dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius")
                                        .template_bind(
                                            Calendar::first_day_of_week_property(),
                                            CalendarDatePicker::first_day_of_week_property(),
                                        )
                                        .template_bind(
                                            Calendar::is_today_highlighted_property(),
                                            CalendarDatePicker::is_today_highlighted_property(),
                                        )
                                        .bind(
                                            Calendar::selected_date_property(),
                                            path()
                                                .templated_parent()
                                                .prop(CalendarDatePicker::selected_date_property())
                                                .binding()
                                                .with_mode(BindingMode::TwoWay),
                                        )
                                        .template_bind(
                                            Calendar::display_date_property(),
                                            CalendarDatePicker::display_date_property(),
                                        )
                                        .template_bind(
                                            Calendar::display_date_start_property(),
                                            CalendarDatePicker::display_date_start_property(),
                                        )
                                        .template_bind(
                                            Calendar::display_date_end_property(),
                                            CalendarDatePicker::display_date_end_property(),
                                        )
                                        .template_bind(
                                            Calendar::is_week_number_visible_property(),
                                            CalendarDatePicker::is_week_number_visible_property(),
                                        )
                                        .template_bind(
                                            Calendar::week_number_rule_property(),
                                            CalendarDatePicker::week_number_rule_property(),
                                        ),
                                ),
                        )
                        .control(),
                ]),
        )?
        .control())
}

/// `DefaultCalendarDatePicker`: a control theme of `CalendarDatePicker`.
fn default_calendar_date_picker(theme: &StyleBuilder) -> Result {
    theme.target::<CalendarDatePicker>();
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "CalendarDatePickerTextForeground",
    );
    theme.dynamic(TemplatedControl::background_property(), "CalendarDatePickerBackground");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "CalendarDatePickerBorderBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "CalendarDatePickerBorderThemeThickness",
    );
    // The placeholder is the text of DateText, in its foreground.
    theme.dynamic(
        CalendarDatePicker::placeholder_foreground_property(),
        "CalendarDatePickerTextForeground",
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.set(
        CalendarDatePicker::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    theme.set(
        CalendarDatePicker::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    // The Padding of DateText.
    theme.set(
        TemplatedControl::padding_property(),
        Thickness::new(12.0, 0.0, 0.0, 2.0),
    );
    // UseSystemFocusVisuals with FocusVisualMargin="-3".
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_calendar_date_picker_focus_adorner)),
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(default_calendar_date_picker_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("Background"), |style| {
            style.dynamic(
                Border::border_brush_property(),
                "CalendarDatePickerBorderBrushPointerOver",
            );
            style.dynamic(Border::background_property(), "CalendarDatePickerBackgroundPointerOver");
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBox>().name("PART_TextBox"), |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "CalendarDatePickerTextForegroundPointerOver",
            );
            style.dynamic(
                TextBox::placeholder_foreground_property(),
                "CalendarDatePickerTextForegroundPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("CalendarGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "CalendarDatePickerCalendarGlyphForegroundPointerOver",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("Background"), |style| {
            style.dynamic(Border::background_property(), "CalendarDatePickerBackgroundPressed");
            style.dynamic(Border::border_brush_property(), "CalendarDatePickerBorderBrushPressed");
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBox>().name("PART_TextBox"), |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "CalendarDatePickerTextForegroundPressed",
            );
            style.dynamic(
                TextBox::placeholder_foreground_property(),
                "CalendarDatePickerTextForegroundPressed",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("CalendarGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "CalendarDatePickerCalendarGlyphForegroundPressed",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("Background"), |style| {
            style.dynamic(Border::background_property(), "CalendarDatePickerBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "CalendarDatePickerBorderBrushDisabled");
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBox>().name("PART_TextBox"), |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "CalendarDatePickerTextForegroundDisabled",
            );
            style.dynamic(
                TextBox::placeholder_foreground_property(),
                "CalendarDatePickerTextForegroundDisabled",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("CalendarGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "CalendarDatePickerCalendarGlyphForegroundDisabled",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // Not in WinUI: the :error of FerroUI's data validation (as the text box).
    theme.style(
        nesting().class(":error").template().of::<Border>().name("Background"),
        |style| {
            style.dynamic(Border::border_brush_property(), "SystemFillColorCriticalBrush");
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `CalendarDatePicker`.
fn calendar_date_picker_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CalendarDatePicker>();
    theme.based_on("DefaultCalendarDatePicker")?;
    Ok(())
}
