//! `Controls/NumberBox.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/NumberBox/NumberBox.xaml (the implicit style of NumberBox; NumberBoxSpinButtonStyle,
//!   NumberBoxPopupSpinButtonStyle and NumberBoxTextBoxStyle are ported in NumericUpDown.xaml, and
//!   the resources of controls/dev/NumberBox/NumberBox_themeresources.xaml are
//!   Resources/NumberBox_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's NumberBox is WinUI's NumberBox (a WinUI control in ferroui-controls,
//!   docs/porting/winui-controls.md). The parts keep WinUI's names, which the control looks up:
//!   HeaderContentPresenter, InputBox, UpSpinButton, DownSpinButton, UpDownPopup,
//!   PopupUpSpinButton and PopupDownSpinButton.
//! - The keyed themes of the spin buttons and of the text box are the ones NumericUpDown.xaml ports
//!   (one definition of each key in the theme): NumberBoxTextBoxStyle for InputBox,
//!   NumericUpDownSpinButtonStyle (NumberBoxSpinButtonStyle with the RepeatButton brushes that the
//!   Grid.Resources of the template set to the TextControlButton brushes) for UpSpinButton and
//!   DownSpinButton, and NumberBoxPopupContentRootButtonStyle (NumberBoxPopupSpinButtonStyle with the
//!   same brushes, which the Grid.Resources of PopupContentRoot set) for the popup buttons.
//! - The spin buttons show the glyphs E70E and E70D of the symbol font: Paths of GlyphGeometryE70E
//!   and GlyphGeometryE70D filled with the Foreground of the button, 12 px wide in the inline
//!   buttons (the font size of the control, 14 px) and 14 px wide in the popup buttons (16 px).
//! - SpinButtonStates are the pseudo-classes the control sets: SpinButtonsVisible is
//!   :spinbuttonsvisible, SpinButtonsPopup is :spinbuttonspopup, SpinButtonsCollapsed is neither.
//!   NumberBox also sets the state on the text box, whose NumberBoxTextBoxStyle reserves the column
//!   of the buttons (SpinButtonsVisible) or shows the popup indicator (SpinButtonsPopup): the Tag of
//!   InputBox, which NumberBoxTextBoxStyle selects, as NumericUpDown.xaml does. The Style setter of
//!   SpinButtonsPopup (InputBox.Style = NumberBoxTextBoxStyle) is the theme the text box has.
//! - UpSpinButtonEnabledStates and DownSpinButtonEnabledStates are :upspinbuttondisabled and
//!   :downspinbuttondisabled (the enabled states set nothing).
//! - HeaderContentPresenter (Visibility Collapsed, x:DeferLoadStrategy Lazy) is shown by the
//!   control, as UpdateHeaderPresenterState does.
//! - The Button InputEater, which takes the pointer input between the inline buttons so that it
//!   does not reach the text box, is a Border of the same name with a transparent background: a
//!   pointer press on it reaches the number box, not the text box.
//! - UpDownPopup: the popup is placed at the top-right corner of the text box (RightEdgeAlignedTop,
//!   the layout slot of the popup in WinUI) with the offsets NumberBoxPopupHorizonalOffset and
//!   NumberBoxPopupVerticalOffset. The shadow of PopupContentRoot (a ThemeShadow at the
//!   Translation.Z NumberBoxPopupShadowDepth, 16, set by NumberBox::OnApplyTemplate) is the
//!   same: the shared WinUIThemeShadow at Translation.Z 16, with ThemeShadowToolTipMargin as its
//!   room (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24). PopupContentRoot, a Grid with a
//!   border and padding, is a Border of the same name around the Grid. ShouldConstrainToRootBounds
//!   has no counterpart.
//! - Left out: SelectionFlyout, SelectionHighlightColor, InputScope, TextReadingOrder and
//!   PreventKeyboardDisplayOnProgrammaticFocus (FerroUI's NumberBox has none of them), the
//!   FontFamily setters of the symbol font (the glyphs are paths), IsTabStop of the spin buttons
//!   (Focusable False in their theme) and AutomationProperties.AccessibilityView.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::media::FontWeight;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextWrapping;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::NumberBox;
use ferroui_controls::PlacementMode;
use ferroui_controls::RepeatButton;
use ferroui_controls::RowDefinitions;
use ferroui_controls::TextBox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The popup spin buttons with the RepeatButton brushes that the Grid.Resources of
    //       PopupContentRoot set to the TextControlButton brushes (RepeatButtonBorderBrushDisabled to
    //       TextControlButtonBorderBrush; the other disabled brushes are not overridden).
    resources.control_theme(
        "NumberBoxPopupContentRootButtonStyle",
        number_box_popup_content_root_button_style,
    );
    // The implicit style of NumberBox (it has no key in WinUI).
    resources.control_theme(type_key::<NumberBox>(), number_box_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// `NumberBoxPopupContentRootButtonStyle`: a control theme of `RepeatButton`.
fn number_box_popup_content_root_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.based_on("NumberBoxPopupSpinButtonStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "TextControlButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "TextControlButtonForeground");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "TextControlButtonBorderBrush",
    );
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

/// The template of `NumberBox` of `number_box_theme`.
fn number_box_theme_template(t: &Template<NumberBox>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .template_bind(Layoutable::height_property(), Layoutable::height_property())
        .with(|e| {
            Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,*,Auto"))?);
            Ok(())
        })?
        .with(|e| {
            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,Auto,Auto"))?);
            Ok(())
        })?
        .children([
            t.make(ContentPresenter::new())
                .name("HeaderContentPresenter")?
                .set(Grid::column_span_property(), 3)
                .template_bind(ContentPresenter::content_property(), NumberBox::header_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    NumberBox::header_template_property(),
                )
                .set(ContentPresenter::font_weight_property(), FontWeight::Normal)
                .dynamic(ContentPresenter::foreground_property(), "TextControlHeaderForeground")
                .dynamic(Layoutable::margin_property(), "TextBoxTopHeaderMargin")
                .template_bind(
                    ContentPresenter::font_size_property(),
                    TemplatedControl::font_size_property(),
                )
                .template_bind(
                    ContentPresenter::font_family_property(),
                    TemplatedControl::font_family_property(),
                )
                .set(ContentPresenter::text_wrapping_property(), TextWrapping::Wrap)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .set(Visual::is_visible_property(), false)
                .control(),
            t.make(TextBox::new())
                .name("InputBox")?
                .set(Grid::row_property(), 1)
                .set(Grid::column_span_property(), 3)
                .static_resource(StyledElement::theme_property(), "NumberBoxTextBoxStyle")?
                .template_bind(
                    TextBox::placeholder_text_property(),
                    NumberBox::placeholder_text_property(),
                )
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
                .template_bind(TextBox::text_alignment_property(), NumberBox::text_alignment_property())
                .template_bind(
                    TemplatedControl::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .set(TextBox::accepts_return_property(), false)
                .set(TextBox::text_wrapping_property(), TextWrapping::NoWrap)
                .control(),
            t.make(Popup::new())
                .name("UpDownPopup")?
                .set(Grid::row_property(), 1)
                .set(Grid::column_property(), 1)
                .bind(
                    Popup::placement_target_property(),
                    path().named(t.names(), "InputBox").binding(),
                )
                .set(Popup::placement_property(), PlacementMode::RightEdgeAlignedTop)
                .dynamic(Popup::vertical_offset_property(), "NumberBoxPopupVerticalOffset")
                .dynamic(Popup::horizontal_offset_property(), "NumberBoxPopupHorizonalOffset")
                .set(Popup::window_manager_add_shadow_hint_property(), false)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .child(
                    Popup::child_property(),
                    t.make(Border::new())
                        .static_resource(Layoutable::margin_property(), "ThemeShadowToolTipMargin")?
                        .child(
                            Decorator::child_property(),
                            t.make(Border::new())
                                .name("PopupContentRoot")?
                                .set(Decorator::padding_property(), Thickness::uniform(6.0))
                                .dynamic(Border::background_property(), "NumberBoxPopupBackground")
                                .dynamic(Border::border_brush_property(), "NumberBoxPopupBorderBrush")
                                .dynamic(Border::border_thickness_property(), "NumberBoxPopupBorderThickness")
                                .dynamic(Border::corner_radius_property(), "OverlayCornerRadius")
                                .set(Visual::translation_property(), literal(Vector3D::parse("0,0,16"))?)
                                .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
                                .child(
                                    Decorator::child_property(),
                                    t.make(Grid::new())
                                        .with(|e| {
                                            Grid::set_row_definitions(e, literal(RowDefinitions::parse("*,*"))?);
                                            Ok(())
                                        })?
                                        .children([
                                            t.make(RepeatButton::new())
                                                .name("PopupUpSpinButton")?
                                                .static_resource(
                                                    StyledElement::theme_property(),
                                                    "NumberBoxPopupContentRootButtonStyle",
                                                )?
                                                .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 0.0, 4.0))
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
                                                        .set(Layoutable::width_property(), 14.0)
                                                        .set(
                                                            Layoutable::horizontal_alignment_property(),
                                                            HorizontalAlignment::Center,
                                                        )
                                                        .set(
                                                            Layoutable::vertical_alignment_property(),
                                                            VerticalAlignment::Center,
                                                        ),
                                                )?
                                                .control(),
                                            t.make(RepeatButton::new())
                                                .name("PopupDownSpinButton")?
                                                .set(Grid::row_property(), 1)
                                                .static_resource(
                                                    StyledElement::theme_property(),
                                                    "NumberBoxPopupContentRootButtonStyle",
                                                )?
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
                                                        .set(Layoutable::width_property(), 14.0)
                                                        .set(
                                                            Layoutable::horizontal_alignment_property(),
                                                            HorizontalAlignment::Center,
                                                        )
                                                        .set(
                                                            Layoutable::vertical_alignment_property(),
                                                            VerticalAlignment::Center,
                                                        ),
                                                )?
                                                .control(),
                                        ]),
                                ),
                        ),
                )
                .control(),
            t.make(Border::new())
                .name("InputEater")?
                .set(Grid::row_property(), 1)
                .set(Grid::column_property(), 1)
                .set(Grid::column_span_property(), 2)
                .set(
                    Border::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                )
                .set(
                    Layoutable::horizontal_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                .set(Layoutable::margin_property(), Thickness::new(4.0, 0.0, 0.0, 0.0))
                .set(Visual::is_visible_property(), false)
                .control(),
            t.make(RepeatButton::new())
                .name("UpSpinButton")?
                .set(Grid::row_property(), 1)
                .set(Grid::column_property(), 1)
                .set(Visual::is_visible_property(), false)
                .template_bind(
                    TemplatedControl::font_size_property(),
                    TemplatedControl::font_size_property(),
                )
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
                .name("DownSpinButton")?
                .set(Grid::row_property(), 1)
                .set(Grid::column_property(), 2)
                .set(Visual::is_visible_property(), false)
                .template_bind(
                    TemplatedControl::font_size_property(),
                    TemplatedControl::font_size_property(),
                )
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
            t.make(ContentPresenter::new())
                .name("DescriptionPresenter")?
                .set(Grid::row_property(), 2)
                .set(Grid::column_span_property(), 3)
                .template_bind(ContentPresenter::content_property(), NumberBox::description_property())
                .dynamic(
                    ContentPresenter::foreground_property(),
                    "SystemControlDescriptionTextForegroundBrush",
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `NumberBox`.
fn number_box_theme(theme: &StyleBuilder) -> Result {
    theme.target::<NumberBox>();
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
    theme.template(number_box_theme_template);
    // CommonStates
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("HeaderContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TextControlHeaderForegroundDisabled",
            );
            Ok(())
        },
    )?;
    // SpinButtonStates (SpinButtonsCollapsed sets the base values).
    theme.style(nesting().class(":spinbuttonsvisible"), |style| {
        style.style(
            nesting().template().of::<RepeatButton>().name("DownSpinButton"),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<RepeatButton>().name("UpSpinButton"),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("InputEater"), |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBox>().name("InputBox"), |style| {
            style.dynamic(Layoutable::min_width_property(), "NumberBoxMinWidth");
            style.set(
                Control::tag_property(),
                Some(value("SpinButtonsVisible".to_string()).expect("a value")),
            );
            Ok(())
        })?;
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":spinbuttonspopup")
            .template()
            .of::<TextBox>()
            .name("InputBox"),
        |style| {
            style.set(
                Control::tag_property(),
                Some(value("SpinButtonsPopup".to_string()).expect("a value")),
            );
            Ok(())
        },
    )?;
    // UpSpinButtonEnabledStates and DownSpinButtonEnabledStates.
    theme.style(nesting().class(":upspinbuttondisabled"), |style| {
        style.style(
            nesting().template().of::<RepeatButton>().name("UpSpinButton"),
            |style| {
                style.set(InputElement::is_enabled_property(), false);
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<RepeatButton>().name("PopupUpSpinButton"),
            |style| {
                style.set(InputElement::is_enabled_property(), false);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":downspinbuttondisabled"), |style| {
        style.style(
            nesting().template().of::<RepeatButton>().name("DownSpinButton"),
            |style| {
                style.set(InputElement::is_enabled_property(), false);
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<RepeatButton>().name("PopupDownSpinButton"),
            |style| {
                style.set(InputElement::is_enabled_property(), false);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}
