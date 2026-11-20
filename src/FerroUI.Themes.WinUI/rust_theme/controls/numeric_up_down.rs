//! `Controls/NumericUpDown.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/NumberBox/NumberBox.xaml (the implicit style of NumberBox,
//!   NumberBoxSpinButtonStyle, NumberBoxPopupSpinButtonStyle, NumberBoxTextBoxStyle; the resources
//!   of controls/dev/NumberBox/NumberBox_themeresources.xaml are
//!   Resources/NumberBox_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's NumericUpDown is WinUI's NumberBox with the spin buttons inline
//!   (SpinButtonPlacementMode="Inline"). Its parts are a ButtonSpinner PART_Spinner, which raises
//!   the spins, holding the TextBox PART_TextBox. The layout of the NumberBox template (the text box
//!   InputBox over the whole width, the buttons UpSpinButton and DownSpinButton over its right end)
//!   is the keyed ButtonSpinner theme NumericUpDownButtonSpinnerStyle: its content spans the
//!   control and the buttons, PART_IncreaseButton (UpSpinButton) and PART_DecreaseButton
//!   (DownSpinButton), are laid over it with the margins of the source.
//! - The Grid.Resources of the template set the RepeatButton brushes to the TextControlButton
//!   brushes for the spin buttons: the keyed theme NumericUpDownSpinButtonStyle, based on
//!   NumberBoxSpinButtonStyle, sets those brushes in its states.
//! - The spin buttons show the glyphs E70E and E70D of the symbol font (their content, at the font
//!   size of the control, 14 px): Paths of GlyphGeometryE70E and GlyphGeometryE70D, 12 px wide,
//!   filled with the Foreground of the button, which the states of NumericUpDownSpinButtonStyle set
//!   as they set the foreground of its content presenter.
//! - SpinButtonStates: SpinButtonsVisible (ShowButtonSpinner) shows the buttons, sets MinWidth of
//!   the text box to NumberBoxMinWidth and reserves the column of the buttons in the text box
//!   (SpinButtonsColumn, 72 px), which NumberBox sets as a visual state of the text box; here the
//!   NumericUpDown theme sets the Tag of the text box to SpinButtonsVisible and
//!   NumberBoxTextBoxStyle selects it (SpinButtonsPopup likewise shows the popup indicator).
//!   SpinButtonsPopup (the buttons in a popup) has no counterpart in FerroUI's NumericUpDown;
//!   NumberBoxPopupSpinButtonStyle is ported for applications. ButtonSpinnerLocation="Left", which
//!   WinUI does not have, puts the buttons at the left and reserves their column there (the Tag
//!   SpinButtonsVisibleLeft).
//! - UpSpinButtonEnabledStates and DownSpinButtonEnabledStates are the IsEnabled the ButtonSpinner
//!   sets on its buttons.
//! - Header, HeaderContentPresenter, Description and DescriptionPresenter: FerroUI's NumericUpDown
//!   has none; they are left out with their rows. SelectionFlyout and SelectionHighlightColor have
//!   no counterpart on NumericUpDown.
//! - NumberBoxTextBoxStyle: its template is the one of DefaultTextBoxStyle (ported in TextBox.xaml)
//!   with the column of the spin buttons and the popup indicator (glyph EC8F at 12 px, the Path of
//!   GlyphGeometryEC8F). Its visual states are those of DefaultTextBoxStyle, which this theme is
//!   based on and whose styles select the same element names; the Focused state of the source also
//!   sets RequestedTheme="Light" on the scroll viewer, which is left out. Its DeleteButtonStyle is
//!   the one of DefaultTextBoxStyle: TextBoxDeleteButtonStyle.
//! - The FerroUI contract: InnerLeftContent and InnerRightContent of NumericUpDown (shown by the
//!   Fluent theme) are passed to the text box, whose NumberBoxTextBoxStyle shows them beside the
//!   text; :error (data validation) gives the border the critical brush, and the template is
//!   wrapped in DataValidationErrors, which shows the errors under the box (as the Fluent theme
//!   wraps it, and as Controls/TextBox.xaml does).
//! - IsTabStop="False" of the spin buttons is also Focusable="False"; the FontFamily setters of the
//!   symbol font are left out (the glyphs are paths).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::BoolConverters;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::data::converters::StringConverters;
use ferroui_base::data::BindingMode;
use ferroui_base::data::MultiBinding;
use ferroui_base::input::Cursor;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextWrapping;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ButtonSpinner;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::DataValidationErrors;
use ferroui_controls::Grid;
use ferroui_controls::Location;
use ferroui_controls::NumericUpDown;
use ferroui_controls::RepeatButton;
use ferroui_controls::ScrollViewer;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_controls::TextBox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("NumberBoxSpinButtonStyle", number_box_spin_button_style);
    // WinUI applies the default style beneath a keyed style without BasedOn
    //       (docs/porting/winui-theme.md, section 4.2).
    resources.control_theme("NumberBoxPopupSpinButtonStyle", number_box_popup_spin_button_style);
    // The spin buttons of the template, with the RepeatButton brushes the Grid.Resources of the
    //       template set to the TextControlButton brushes (RepeatButtonBorderBrushDisabled to
    //       TextControlButtonBorderBrush; the other disabled brushes are not overridden).
    resources.control_theme("NumericUpDownSpinButtonStyle", numeric_up_down_spin_button_style);
    resources.control_theme("NumberBoxTextBoxStyle", number_box_text_box_style);
    // The layout of the NumberBox template: the text box over the whole control, the spin buttons
    //       over its end.
    resources.control_theme("NumericUpDownButtonSpinnerStyle", numeric_up_down_button_spinner_style);
    // The implicit style of NumberBox (it has no key in WinUI).
    resources.control_theme(type_key::<NumericUpDown>(), numeric_up_down_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// `NumberBoxSpinButtonStyle`: a control theme of `RepeatButton`.
fn number_box_spin_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.based_on("DefaultRepeatButtonStyle")?;
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(InputElement::focusable_property(), false);
    theme.set(Layoutable::min_width_property(), 32.0);
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "NumberBoxSpinButtonBorderThickness",
    );
    theme.set(TemplatedControl::font_size_property(), 12.0);
    Ok(())
}

/// `NumberBoxPopupSpinButtonStyle`: a control theme of `RepeatButton`.
fn number_box_popup_spin_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.based_on("DefaultRepeatButtonStyle")?;
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(InputElement::focusable_property(), false);
    theme.set(Layoutable::width_property(), 36.0);
    theme.set(Layoutable::height_property(), 36.0);
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "NumberBoxPopupSpinButtonBorderThickness",
    );
    theme.set(TemplatedControl::font_size_property(), 16.0);
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    Ok(())
}

/// `NumericUpDownSpinButtonStyle`: a control theme of `RepeatButton`.
fn numeric_up_down_spin_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.based_on("NumberBoxSpinButtonStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "TextControlButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "TextControlButtonForeground");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "TextControlButtonBorderBrush",
    );
    // The states also set the Foreground of the button itself, which the glyph paths in the
    //         content of the spin buttons take (a path does not inherit the foreground of the content
    //         presenter as the glyph of a FontIcon does).
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "TextControlButtonForegroundPointerOver",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "TextControlButtonBackgroundPointerOver",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "TextControlButtonBorderBrushPointerOver",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TextControlButtonForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "TextControlButtonForegroundPressed",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "TextControlButtonBackgroundPressed",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "TextControlButtonBorderBrushPressed",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TextControlButtonForegroundPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "RepeatButtonForegroundDisabled",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "TextControlButtonBorderBrush",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `TextBox` of `number_box_text_box_style`.
fn number_box_text_box_style_template(t: &Template<TextBox>) -> Result<Ref<Control>> {
    // Columns: the spin buttons at the left (FerroUI), InnerLeftContent (FerroUI), the text
    //             (ContentElement), InnerRightContent (FerroUI), DeleteButton, SpinButtonsColumn.
    Ok(t.make(Grid::new())
        .with(|e| {
            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,Auto,*,Auto,Auto,Auto"))?);
            Ok(())
        })?
        // SpinButtonsColumn (Width 72 in SpinButtonsVisible)
        // The glyph EC8F at 12 px.
        .children([
            t.make(Border::new())
                .name("BorderElement")?
                .set(Grid::column_property(), 0)
                .set(Grid::column_span_property(), 6)
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
                .template_bind(Layoutable::min_width_property(), Layoutable::min_width_property())
                .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
                .control(),
            t.make(Border::new())
                .name("SpinButtonsSpaceLeft")?
                .set(Grid::column_property(), 0)
                .set(Layoutable::width_property(), 72.0)
                .set(Visual::is_visible_property(), false)
                .control(),
            t.make(ContentPresenter::new())
                .name("InnerLeftContentPresenter")?
                .template_bind_with(
                    Visual::is_visible_property(),
                    Some(TextBox::inner_left_content_property().property()),
                    Some(ObjectConverters::is_not_null()),
                    None,
                    None,
                )
                .set(Grid::column_property(), 1)
                .template_bind(
                    Layoutable::margin_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    ContentPresenter::vertical_content_alignment_property(),
                    TextBox::vertical_content_alignment_property(),
                )
                .template_bind(
                    ContentPresenter::content_property(),
                    TextBox::inner_left_content_property(),
                )
                .control(),
            t.make(ScrollViewer::new())
                .name("PART_ScrollViewer")?
                .set(Grid::column_property(), 2)
                .template_bind(
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                )
                .template_bind(
                    Layoutable::margin_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    TemplatedControl::padding_property(),
                    TemplatedControl::padding_property(),
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
                        ),
                )?
                .control(),
            t.make(TextBlock::new())
                .name("PlaceholderTextContentPresenter")?
                .set(Grid::column_property(), 2)
                .set(Grid::column_span_property(), 3)
                .template_bind(
                    TextBlock::foreground_property(),
                    TextBox::placeholder_foreground_property(),
                )
                .template_bind(
                    Layoutable::margin_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(TextBlock::padding_property(), TemplatedControl::padding_property())
                .template_bind(TextBlock::text_property(), TextBox::placeholder_text_property())
                .template_bind(TextBlock::text_alignment_property(), TextBox::text_alignment_property())
                .template_bind(TextBlock::text_wrapping_property(), TextBox::text_wrapping_property())
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
            t.make(ContentPresenter::new())
                .name("InnerRightContentPresenter")?
                .template_bind_with(
                    Visual::is_visible_property(),
                    Some(TextBox::inner_right_content_property().property()),
                    Some(ObjectConverters::is_not_null()),
                    None,
                    None,
                )
                .set(Grid::column_property(), 3)
                .template_bind(
                    Layoutable::margin_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    ContentPresenter::vertical_content_alignment_property(),
                    TextBox::vertical_content_alignment_property(),
                )
                .template_bind(
                    ContentPresenter::content_property(),
                    TextBox::inner_right_content_property(),
                )
                .control(),
            t.make(Button::new())
                .name("DeleteButton")?
                .set(Grid::column_property(), 4)
                .static_resource(StyledElement::theme_property(), "TextBoxDeleteButtonStyle")?
                .bind(
                    Button::command_property(),
                    path()
                        .ancestor_of::<TextBox>(0)
                        .method::<Ref<TextBox>>("Clear", |this| {
                            this.clear();
                        })
                        .binding(),
                )
                .template_bind(
                    TemplatedControl::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    TemplatedControl::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .dynamic(TemplatedControl::padding_property(), "HelperButtonThemePadding")
                .set(InputElement::focusable_property(), false)
                .set(Visual::is_visible_property(), false)
                .template_bind(
                    TemplatedControl::font_size_property(),
                    TemplatedControl::font_size_property(),
                )
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                .set(Layoutable::min_width_property(), 40.0)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                .control(),
            t.make(Border::new())
                .name("SpinButtonsSpace")?
                .set(Grid::column_property(), 5)
                .set(Layoutable::width_property(), 72.0)
                .set(Visual::is_visible_property(), false)
                .control(),
            t.make(Path::new())
                .name("PopupIndicator")?
                .set(Grid::column_property(), 5)
                .set(Visual::is_visible_property(), false)
                .static_resource(Layoutable::margin_property(), "NumberBoxPopupIndicatorMargin")?
                .static_resource(Path::data_property(), "GlyphGeometryEC8F")?
                .dynamic(Shape::fill_property(), "NumberBoxPopupIndicatorForeground")
                .set(Shape::stretch_property(), Stretch::Uniform)
                .set(Layoutable::width_property(), 10.0)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .control(),
        ])
        .control())
}

/// `NumberBoxTextBoxStyle`: a control theme of `TextBox`.
fn number_box_text_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBox>();
    theme.based_on("DefaultTextBoxStyle")?;
    theme.template(number_box_text_box_style_template);
    // SpinButtonStates, set by the NumericUpDown theme on its text box (SpinButtonsCollapsed
    //         sets the base values).
    theme.style(
        nesting()
            .property_is(
                Control::tag_property(),
                Some(value("SpinButtonsVisible".to_string()).expect("a value")),
            )
            .template()
            .of::<Border>()
            .name("SpinButtonsSpace"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(
                Control::tag_property(),
                Some(value("SpinButtonsVisibleLeft".to_string()).expect("a value")),
            )
            .template()
            .of::<Border>()
            .name("SpinButtonsSpaceLeft"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(
                Control::tag_property(),
                Some(value("SpinButtonsPopup".to_string()).expect("a value")),
            )
            .template()
            .of::<Path>()
            .name("PopupIndicator"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `ButtonSpinner` of `numeric_up_down_button_spinner_style`.
fn numeric_up_down_button_spinner_style_template(t: &Template<ButtonSpinner>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .children([
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
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
                .control(),
            t.make(StackPanel::new())
                .name("SpinButtons")?
                .set(StackPanel::orientation_property(), Orientation::Horizontal)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                .template_bind(
                    Visual::is_visible_property(),
                    ButtonSpinner::show_button_spinner_property(),
                )
                .children([
                    t.make(RepeatButton::new())
                        .name("PART_IncreaseButton")?
                        .static_resource(StyledElement::theme_property(), "NumericUpDownSpinButtonStyle")?
                        .set(Layoutable::margin_property(), Thickness::uniform(4.0))
                        .template_bind(
                            TemplatedControl::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .content(
                            ContentControl::content_property(),
                            t.make(Path::new())
                                .static_resource(Path::data_property(), "GlyphGeometryE70E")?
                                .bind(
                                    Shape::fill_property(),
                                    path()
                                        .ancestor_of::<RepeatButton>(0)
                                        .prop(TemplatedControl::foreground_property())
                                        .binding(),
                                )
                                .set(Shape::stretch_property(), Stretch::Uniform)
                                .set(Layoutable::width_property(), 12.0)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
                        )?
                        .control(),
                    t.make(RepeatButton::new())
                        .name("PART_DecreaseButton")?
                        .static_resource(StyledElement::theme_property(), "NumericUpDownSpinButtonStyle")?
                        .set(Layoutable::margin_property(), Thickness::new(0.0, 4.0, 4.0, 4.0))
                        .template_bind(
                            TemplatedControl::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .content(
                            ContentControl::content_property(),
                            t.make(Path::new())
                                .static_resource(Path::data_property(), "GlyphGeometryE70D")?
                                .bind(
                                    Shape::fill_property(),
                                    path()
                                        .ancestor_of::<RepeatButton>(0)
                                        .prop(TemplatedControl::foreground_property())
                                        .binding(),
                                )
                                .set(Shape::stretch_property(), Stretch::Uniform)
                                .set(Layoutable::width_property(), 12.0)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
                        )?
                        .control(),
                ])
                .control(),
        ])
        .control())
}

/// `NumericUpDownButtonSpinnerStyle`: a control theme of `ButtonSpinner`.
fn numeric_up_down_button_spinner_style(theme: &StyleBuilder) -> Result {
    theme.target::<ButtonSpinner>();
    theme.set(InputElement::focusable_property(), false);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.template(numeric_up_down_button_spinner_style_template);
    // No WinUI counterpart: the buttons at the left.
    theme.style(
        nesting()
            .class(":left")
            .template()
            .of::<StackPanel>()
            .name("SpinButtons"),
        |style| {
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `NumericUpDown` of `numeric_up_down_theme`.
fn numeric_up_down_theme_template(t: &Template<NumericUpDown>) -> Result<Ref<Control>> {
    // The errors of FerroUI's data validation under the box (DataValidationErrors.xaml).
    Ok(t.make(DataValidationErrors::new())
        .content(
            ContentControl::content_property(),
            t.make(ButtonSpinner::new())
                .name("PART_Spinner")?
                .static_resource(StyledElement::theme_property(), "NumericUpDownButtonSpinnerStyle")?
                .template_bind(
                    TemplatedControl::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(
                    ButtonSpinner::allow_spin_property(),
                    NumericUpDown::allow_spin_property(),
                )
                .template_bind(
                    ButtonSpinner::show_button_spinner_property(),
                    NumericUpDown::show_button_spinner_property(),
                )
                .template_bind(
                    ButtonSpinner::button_spinner_location_property(),
                    NumericUpDown::button_spinner_location_property(),
                )
                .set(
                    ContentControl::horizontal_content_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .set(
                    ContentControl::vertical_content_alignment_property(),
                    VerticalAlignment::Stretch,
                )
                .content(
                    ContentControl::content_property(),
                    t.make(TextBox::new())
                        .name("PART_TextBox")?
                        .static_resource(StyledElement::theme_property(), "NumberBoxTextBoxStyle")?
                        .template_bind(
                            TemplatedControl::font_size_property(),
                            TemplatedControl::font_size_property(),
                        )
                        .template_bind(
                            TemplatedControl::font_weight_property(),
                            TemplatedControl::font_weight_property(),
                        )
                        .template_bind(
                            TemplatedControl::font_family_property(),
                            TemplatedControl::font_family_property(),
                        )
                        .template_bind(
                            TemplatedControl::background_property(),
                            TemplatedControl::background_property(),
                        )
                        .template_bind(
                            TemplatedControl::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .template_bind(
                            TemplatedControl::border_brush_property(),
                            TemplatedControl::border_brush_property(),
                        )
                        .template_bind(
                            TemplatedControl::padding_property(),
                            TemplatedControl::padding_property(),
                        )
                        .template_bind(
                            TemplatedControl::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .template_bind(
                            TextBox::text_alignment_property(),
                            NumericUpDown::text_alignment_property(),
                        )
                        .template_bind(
                            TemplatedControl::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .template_bind(
                            TextBox::placeholder_text_property(),
                            NumericUpDown::placeholder_text_property(),
                        )
                        .template_bind(
                            TextBox::placeholder_foreground_property(),
                            NumericUpDown::placeholder_foreground_property(),
                        )
                        .template_bind(TextBox::is_read_only_property(), NumericUpDown::is_read_only_property())
                        .template_bind(TextBox::text_property(), NumericUpDown::text_property())
                        .template_bind(
                            TextBox::horizontal_content_alignment_property(),
                            NumericUpDown::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            TextBox::vertical_content_alignment_property(),
                            NumericUpDown::vertical_content_alignment_property(),
                        )
                        .set(TextBox::accepts_return_property(), false)
                        .set(TextBox::text_wrapping_property(), TextWrapping::NoWrap)
                        .bind(
                            TextBox::inner_left_content_property(),
                            path()
                                .templated_parent()
                                .prop(NumericUpDown::inner_left_content_property())
                                .binding(),
                        )
                        .bind(
                            TextBox::inner_right_content_property(),
                            path()
                                .templated_parent()
                                .prop(NumericUpDown::inner_right_content_property())
                                .binding(),
                        ),
                )?,
        )?
        .control())
}

/// The default control theme of `NumericUpDown`.
fn numeric_up_down_theme(theme: &StyleBuilder) -> Result {
    theme.target::<NumericUpDown>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.dynamic(TemplatedControl::background_property(), "TextControlBackground");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TextControlBorderThemeThickness",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "TextControlBorderBrush");
    theme.dynamic(TemplatedControl::foreground_property(), "TextControlForeground");
    theme.dynamic(TemplatedControl::padding_property(), "TextControlThemePadding");
    // Not in the style: the placeholder brush the text box of WinUI uses when NumberBox sets
    //         none (FerroUI's NumericUpDown passes its own to the text box).
    theme.dynamic(
        NumericUpDown::placeholder_foreground_property(),
        "TextControlPlaceholderForeground",
    );
    theme.template(numeric_up_down_theme_template);
    // SpinButtonsVisible (the buttons are shown by the binding of ShowButtonSpinner).
    theme.style(
        nesting()
            .property_is(NumericUpDown::show_button_spinner_property(), true)
            .template()
            .of::<TextBox>()
            .name("PART_TextBox"),
        |style| {
            style.static_resource(Layoutable::min_width_property(), "NumberBoxMinWidth")?;
            style.set(
                Control::tag_property(),
                Some(value("SpinButtonsVisible".to_string()).expect("a value")),
            );
            Ok(())
        },
    )?;
    // No WinUI counterpart: the buttons at the left.
    theme.style(
        nesting()
            .property_is(NumericUpDown::show_button_spinner_property(), true)
            .property_is(NumericUpDown::button_spinner_location_property(), Location::Left)
            .template()
            .of::<TextBox>()
            .name("PART_TextBox"),
        |style| {
            style.set(
                Control::tag_property(),
                Some(value("SpinButtonsVisibleLeft".to_string()).expect("a value")),
            );
            Ok(())
        },
    )?;
    // Not in WinUI: the :error of FerroUI's data validation.
    theme.style(
        nesting()
            .class(":error")
            .template()
            .of::<TextBox>()
            .name("PART_TextBox"),
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
