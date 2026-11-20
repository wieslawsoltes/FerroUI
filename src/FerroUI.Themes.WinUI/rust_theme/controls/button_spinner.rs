//! `Controls/ButtonSpinner.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a document: WinUI 3 has no ButtonSpinner. Built from microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml), commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/NumberBox/NumberBox.xaml (the spin buttons UpSpinButton and DownSpinButton of
//!   the NumberBox template and NumberBoxSpinButtonStyle, ported in NumericUpDown.xaml)
//!   controls/dev/CommonStyles/TextBox_themeresources.xaml (the box: the resources and the visual
//!   states of BorderElement of DefaultTextBoxStyle)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The spinner looks as a NumberBox with inline spin buttons: a box with the background, the
//! border and the states of a text box (PointerOver, Focused as :focus-within, Disabled), its
//! content, and the spin buttons PART_IncreaseButton and PART_DecreaseButton of the NumberBox
//! template (the keyed theme NumericUpDownSpinButtonStyle of NumericUpDown.xaml, the margins 4 and
//! 0,4,4,4, the glyphs E70E and E70D at 14 px as Paths). NumericUpDown uses the keyed theme
//! NumericUpDownButtonSpinnerStyle of NumericUpDown.xaml instead, in which the text box draws the
//! box. The FerroUI contract: ButtonSpinnerLocation="Left" (:left) docks the buttons at the left,
//! ShowButtonSpinner hides them, AllowSpin disables them (the control sets IsEnabled), and :error
//! (data validation) gives the border the critical brush; the template is wrapped in
//! DataValidationErrors, which shows the errors under the box, as the Fluent theme wraps it. As the text box of WinUI, the spinner
//! shows no focus rectangle (its focused state is the border of the box).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Stretch;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ButtonSpinner;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::DataValidationErrors;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_controls::Grid;
use ferroui_controls::RepeatButton;
use ferroui_controls::StackPanel;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<ButtonSpinner>(), button_spinner_theme);
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

/// The template of `ButtonSpinner` of `button_spinner_theme`.
fn button_spinner_theme_template(t: &Template<ButtonSpinner>) -> Result<Ref<Control>> {
    // The errors of FerroUI's data validation under the box (DataValidationErrors.xaml).
    Ok(t.make(DataValidationErrors::new())
        .content(
            ContentControl::content_property(),
            t.make(Grid::new()).children([
                t.make(Border::new())
                    .name("BorderElement")?
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
                    .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                    .template_bind(Layoutable::min_width_property(), Layoutable::min_width_property())
                    .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
                    .control(),
                t.make(DockPanel::new())
                    .template_bind(
                        Layoutable::margin_property(),
                        TemplatedControl::border_thickness_property(),
                    )
                    .children([
                        t.make(StackPanel::new())
                            .name("SpinButtons")?
                            .set(DockPanel::dock_property(), Dock::Right)
                            .set(StackPanel::orientation_property(), Orientation::Horizontal)
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
                                            .set(
                                                Layoutable::horizontal_alignment_property(),
                                                HorizontalAlignment::Center,
                                            )
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
                                            .set(
                                                Layoutable::horizontal_alignment_property(),
                                                HorizontalAlignment::Center,
                                            )
                                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
                                    )?
                                    .control(),
                            ])
                            .control(),
                        t.make(ContentPresenter::new())
                            .name("PART_ContentPresenter")?
                            .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                            .template_bind(
                                ContentPresenter::content_template_property(),
                                ContentControl::content_template_property(),
                            )
                            .template_bind(
                                ContentPresenter::foreground_property(),
                                TemplatedControl::foreground_property(),
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
                            .control(),
                    ])
                    .control(),
            ]),
        )?
        .control())
}

/// The default control theme of `ButtonSpinner`.
fn button_spinner_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ButtonSpinner>();
    theme.dynamic(TemplatedControl::foreground_property(), "TextControlForeground");
    theme.dynamic(TemplatedControl::background_property(), "TextControlBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "TextControlBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TextControlBorderThemeThickness",
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(Layoutable::min_height_property(), "TextControlThemeMinHeight");
    theme.dynamic(Layoutable::min_width_property(), "TextControlThemeMinWidth");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(InputElement::focusable_property(), true);
    theme.set_null(Control::focus_adorner_property());
    theme.template(button_spinner_theme_template);
    // The buttons at the left.
    theme.style(
        nesting()
            .class(":left")
            .template()
            .of::<StackPanel>()
            .name("SpinButtons"),
        |style| {
            style.set(DockPanel::dock_property(), Dock::Left);
            Ok(())
        },
    )?;
    // The states of BorderElement of DefaultTextBoxStyle: PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("BorderElement"),
        |style| {
            style.dynamic(Border::border_brush_property(), "TextControlBorderBrushPointerOver");
            style.dynamic(Border::background_property(), "TextControlBackgroundPointerOver");
            Ok(())
        },
    )?;
    // Focused
    theme.style(
        nesting()
            .class(":focus-within")
            .template()
            .of::<Border>()
            .name("BorderElement"),
        |style| {
            style.dynamic(Border::background_property(), "TextControlBackgroundFocused");
            style.dynamic(Border::border_brush_property(), "TextControlBorderBrushFocused");
            style.dynamic(
                Border::border_thickness_property(),
                "TextControlBorderThemeThicknessFocused",
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("BorderElement"), |style| {
            style.dynamic(Border::background_property(), "TextControlBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "TextControlBorderBrushDisabled");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "TextControlForegroundDisabled");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Data validation (FerroUI).
    theme.style(
        nesting()
            .class(":error")
            .template()
            .of::<Border>()
            .name("BorderElement"),
        |style| {
            style.dynamic(Border::border_brush_property(), "SystemFillColorCriticalBrush");
            Ok(())
        },
    )?;
    Ok(())
}
