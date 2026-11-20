//! `Controls/SplitButton.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/SplitButton/SplitButton.xaml (SplitButtonStyle and the implicit styles of
//!   SplitButton and ToggleSplitButton)
//!   controls/dev/SplitButton/SplitButton_themeresources.xaml (SplitButtonCommandBarStyle; its
//!   resources are Resources/SplitButton_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's ToggleSplitButton takes the style of SplitButton (its style key is SplitButton), as
//!   WinUI's implicit ToggleSplitButton style is SplitButtonStyle: one default control theme
//!   serves both; the :checked pseudo-class is the InternalIsChecked of the source.
//! - Template parts: PrimaryButton is PART_PrimaryButton, SecondaryButton PART_SecondaryButton.
//! - The Grid RootGrid has a corner radius, which clips the backgrounds of the halves in WinUI:
//!   it is a Border of the same name with ClipToBounds around the Grid of the columns. The grids
//!   PrimaryButtonBorder and SecondaryButtonBorder, which draw borders, are Borders.
//! - The implicit Button style of Grid.Resources (the style of the two inner buttons) is the keyed
//!   theme SplitButtonInnerButtonStyle (SplitButtonCommandBarInnerButtonStyle for the command bar
//!   style). These themes are not based on DefaultButtonStyle, whose visual state styles would
//!   select the content presenter of their templates; the setters of the source are written out.
//! - The visual states of CommonStates are chosen by the code of WinUI's SplitButton from the
//!   pointer and press state of each inner button. FerroUI's inner buttons have their own
//!   :pointerover and :pressed, so the states of one half (PrimaryPointerOver, SecondaryPressed,
//!   ...) are styles of the inner button (Button#PART_PrimaryButton:pointerover). A state sets
//!   the background grid and the border of its half: the grids PrimaryBackgroundGrid and
//!   SecondaryBackgroundGrid take the Background of their button and the borders its BorderBrush
//!   (bindings), and the styles set those properties of the button. The values a state of one
//!   half gives the other half are the :pointerover style of the split button, which comes first.
//! - FlyoutOpen is :flyout-open; TouchPressed (the pointer of the last press was touch, or a key
//!   is down) is :pressed, which FerroUI's split button sets while a key is down; the placement
//!   state SecondaryButtonSpan (a key is down) moves the secondary button over both columns in
//!   :pressed. Checked and the Checked* states are :checked and its combinations.
//! - The chevron of the secondary button is an AnimatedIcon (AnimatedChevronDownSmallVisualSource)
//!   of 12 x 12 with the fallback glyph E96E (ChevronDownSmall) at 8 px; the command bar style
//!   shows E972 (ChevronDownMed) at 8 px in a TextBlock. Both are Paths of the glyph geometries
//!   whose Fill is the Foreground of the inner button (an icon inherits the foreground of the content
//!   presenter in WinUI, which is the one of the button except in Disabled, where the disabled styles
//!   of the split button set the foreground of the buttons as the inner states set the one of the
//!   presenter).
//! - UseSystemFocusVisuals with FocusVisualMargin="-1" (SplitButtonStyle) and "-3" (the command bar
//!   style and the inner buttons) is the FocusAdorner. IsTabStop="True" of the split button is
//!   also Focusable="True"; IsTabStop="False" of the inner buttons is also Focusable="False".
//! - Column widths of resources (SplitButtonSecondaryButtonSize, 35) are written as values: a
//!   double resource does not convert to a GridLength.

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
use ferroui_base::CornerRadius;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
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
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::Panel;
use ferroui_controls::SplitButton;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The implicit Button style of the Grid.Resources of the template of SplitButtonStyle.
    resources.control_theme("SplitButtonInnerButtonStyle", split_button_inner_button_style);
    resources.control_theme("SplitButtonStyle", split_button_style);
    resources.control_theme(type_key::<SplitButton>(), split_button_theme);
    // The implicit Button style of the Grid.Resources of the template of
    //       SplitButtonCommandBarStyle.
    resources.control_theme(
        "SplitButtonCommandBarInnerButtonStyle",
        split_button_command_bar_inner_button_style,
    );
    // WinUI applies the default style beneath a keyed style; this theme is not based on
    //       SplitButtonStyle, whose state styles select the same element names, and writes out the
    //       setter of SplitButtonStyle it does not override (BorderThickness).
    resources.control_theme("SplitButtonCommandBarStyle", split_button_command_bar_style);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The focus adorner of `split_button_inner_button_style`.
fn split_button_inner_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `Button` of `split_button_inner_button_style`.
fn split_button_inner_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("RootGrid")?
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([t
            .make(ContentPresenter::new())
            .name("PART_ContentPresenter")?
            .template_bind(
                ContentPresenter::foreground_property(),
                TemplatedControl::foreground_property(),
            )
            .template_bind(
                ContentPresenter::border_brush_property(),
                TemplatedControl::border_brush_property(),
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
            .set(ContentPresenter::recognizes_access_key_property(), true)
            .control()])
        .control())
}

/// `SplitButtonInnerButtonStyle`: a control theme of `Button`.
fn split_button_inner_button_style(theme: &StyleBuilder) -> Result {
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
    theme.dynamic(TemplatedControl::foreground_property(), "SplitButtonForeground");
    theme.set(
        TemplatedControl::border_brush_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "SplitButtonBorderThemeThickness",
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            split_button_inner_button_style_focus_adorner,
        )),
    );
    theme.template(split_button_inner_button_style_template);
    // PointerOver and Pressed set only the state of an animated icon in the content.
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "SplitButtonForegroundDisabled");
                style.set(
                    ContentPresenter::border_brush_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Grid>().name("RootGrid"), |style| {
            style.dynamic(Panel::background_property(), "SplitButtonBackgroundDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The focus adorner of `split_button_style`.
fn split_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-1.0))
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

/// The template of `SplitButton` of `split_button_style`.
fn split_button_style_template(t: &Template<SplitButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .set(
            Border::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .set(Visual::clip_to_bounds_property(), true)
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    let items = Grid::column_definitions(e);
                    items.add(
                        t.make(ColumnDefinition::new())
                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
                            .set(ColumnDefinition::min_width_property(), 35.0)
                            .end()
                            .into(),
                    );
                    items.add(
                        t.make(ColumnDefinition::new())
                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("1"))?)
                            .end()
                            .into(),
                    );
                    items.add(
                        t.make(ColumnDefinition::new())
                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("35"))?)
                            .end()
                            .into(),
                    );
                    Ok(())
                })?
                .children([
                    t.make(Grid::new())
                        .name("PrimaryBackgroundGrid")?
                        .set(Grid::column_span_property(), 2)
                        .bind(
                            Panel::background_property(),
                            path()
                                .named(t.names(), "PART_PrimaryButton")
                                .prop(TemplatedControl::background_property())
                                .binding(),
                        )
                        .control(),
                    t.make(Grid::new())
                        .name("DividerBackgroundGrid")?
                        .set(Grid::column_property(), 1)
                        .set(Layoutable::width_property(), 1.0)
                        .dynamic(Panel::background_property(), "SplitButtonBorderBrushDivider")
                        .control(),
                    t.make(Grid::new())
                        .name("SecondaryBackgroundGrid")?
                        .set(Grid::column_property(), 2)
                        .bind(
                            Panel::background_property(),
                            path()
                                .named(t.names(), "PART_SecondaryButton")
                                .prop(TemplatedControl::background_property())
                                .binding(),
                        )
                        .control(),
                    t.make(Button::new())
                        .name("PART_PrimaryButton")?
                        .set(Grid::column_property(), 0)
                        .static_resource(StyledElement::theme_property(), "SplitButtonInnerButtonStyle")?
                        .template_bind(
                            TemplatedControl::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .template_bind(
                            TemplatedControl::background_property(),
                            TemplatedControl::background_property(),
                        )
                        .set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0))
                        .template_bind(
                            TemplatedControl::border_brush_property(),
                            TemplatedControl::border_brush_property(),
                        )
                        .template_bind(ContentControl::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentControl::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .template_bind(Button::command_property(), SplitButton::command_property())
                        .template_bind(
                            Button::command_parameter_property(),
                            SplitButton::command_parameter_property(),
                        )
                        .template_bind(
                            TemplatedControl::font_family_property(),
                            TemplatedControl::font_family_property(),
                        )
                        .template_bind(
                            TemplatedControl::font_size_property(),
                            TemplatedControl::font_size_property(),
                        )
                        .template_bind(
                            TemplatedControl::font_weight_property(),
                            TemplatedControl::font_weight_property(),
                        )
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                        .template_bind(
                            ContentControl::horizontal_content_alignment_property(),
                            ContentControl::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            ContentControl::vertical_content_alignment_property(),
                            ContentControl::vertical_content_alignment_property(),
                        )
                        .template_bind(
                            TemplatedControl::padding_property(),
                            TemplatedControl::padding_property(),
                        )
                        .set(InputElement::is_tab_stop_property(), false)
                        .set(InputElement::focusable_property(), false)
                        .control(),
                    t.make(Button::new())
                        .name("PART_SecondaryButton")?
                        .set(Grid::column_property(), 2)
                        .static_resource(StyledElement::theme_property(), "SplitButtonInnerButtonStyle")?
                        .dynamic(
                            TemplatedControl::foreground_property(),
                            "SplitButtonForegroundSecondary",
                        )
                        .template_bind(
                            TemplatedControl::background_property(),
                            TemplatedControl::background_property(),
                        )
                        .set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0))
                        .template_bind(
                            TemplatedControl::border_brush_property(),
                            TemplatedControl::border_brush_property(),
                        )
                        .set(
                            ContentControl::horizontal_content_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(
                            ContentControl::vertical_content_alignment_property(),
                            VerticalAlignment::Stretch,
                        )
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                        .set(
                            TemplatedControl::padding_property(),
                            Thickness::new(0.0, 0.0, 12.0, 0.0),
                        )
                        .set(InputElement::is_tab_stop_property(), false)
                        .set(InputElement::focusable_property(), false)
                        // The AnimatedIcon (12 x 12) with its fallback glyph E96E at 8 px.
                        .content(
                            ContentControl::content_property(),
                            t.make(Grid::new())
                                .set(Layoutable::width_property(), 12.0)
                                .set(Layoutable::height_property(), 12.0)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                                .children([t
                                    .make(Path::new())
                                    .static_resource(Path::data_property(), "GlyphGeometryE96E")?
                                    .bind(
                                        Shape::fill_property(),
                                        path()
                                            .ancestor_of::<Button>(0)
                                            .prop(TemplatedControl::foreground_property())
                                            .binding(),
                                    )
                                    .set(Shape::stretch_property(), Stretch::Uniform)
                                    .set(Layoutable::width_property(), 8.0)
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                    .control()]),
                        )?
                        .control(),
                    t.make(Border::new())
                        .name("PrimaryButtonBorder")?
                        .set(Grid::column_property(), 0)
                        .bind(
                            Border::border_brush_property(),
                            path()
                                .named(t.names(), "PART_PrimaryButton")
                                .prop(TemplatedControl::border_brush_property())
                                .binding(),
                        )
                        .set(Border::border_thickness_property(), Thickness::new(1.0, 1.0, 0.0, 1.0))
                        .set(Border::corner_radius_property(), CornerRadius::new(4.0, 0.0, 0.0, 4.0))
                        .control(),
                    t.make(Border::new())
                        .name("SecondaryButtonBorder")?
                        .set(Grid::column_property(), 2)
                        .bind(
                            Border::border_brush_property(),
                            path()
                                .named(t.names(), "PART_SecondaryButton")
                                .prop(TemplatedControl::border_brush_property())
                                .binding(),
                        )
                        .set(Border::border_thickness_property(), Thickness::new(0.0, 1.0, 1.0, 1.0))
                        .set(Border::corner_radius_property(), CornerRadius::new(0.0, 4.0, 4.0, 0.0))
                        .control(),
                ]),
        )
        .control())
}

/// `SplitButtonStyle`: a control theme of `SplitButton`.
fn split_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<SplitButton>();
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
    theme.dynamic(TemplatedControl::background_property(), "SplitButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "SplitButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "SplitButtonBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "SplitButtonBorderThemeThickness",
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), split_button_style_focus_adorner)),
    );
    theme.set(InputElement::is_tab_stop_property(), true);
    theme.set(InputElement::focusable_property(), true);
    theme.dynamic(TemplatedControl::padding_property(), "SplitButtonPadding");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(split_button_style_template);
    // The values PrimaryPointerOver, PrimaryPressed, SecondaryPointerOver and SecondaryPressed
    //         give the half that is not under the pointer.
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SplitButtonBackground");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SplitButtonBackground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // PrimaryPointerOver
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("PART_PrimaryButton")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "SplitButtonBackgroundPointerOver",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "SplitButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    // PrimaryPressed
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("PART_PrimaryButton")
            .class(":pressed"),
        |style| {
            style.dynamic(TemplatedControl::background_property(), "SplitButtonBackgroundPressed");
            style.dynamic(TemplatedControl::foreground_property(), "SplitButtonForegroundPressed");
            style.dynamic(
                TemplatedControl::border_brush_property(),
                "SplitButtonBorderBrushPressed",
            );
            Ok(())
        },
    )?;
    // SecondaryPointerOver (its setter of SecondaryButton.BorderBrush, a brush the inner
    //         template draws with a thickness of 0, has no effect in WinUI and is left out)
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("PART_SecondaryButton")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "SplitButtonBackgroundPointerOver",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "SplitButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    // SecondaryPressed
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("PART_SecondaryButton")
            .class(":pressed"),
        |style| {
            style.dynamic(TemplatedControl::background_property(), "SplitButtonBackgroundPressed");
            style.dynamic(
                TemplatedControl::foreground_property(),
                "SplitButtonForegroundSecondaryPressed",
            );
            style.dynamic(
                TemplatedControl::border_brush_property(),
                "SplitButtonBorderBrushPressed",
            );
            Ok(())
        },
    )?;
    // TouchPressed, with SecondaryButtonSpan of SecondaryButtonPlacementStates
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SplitButtonBackgroundPressed");
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushPressed",
                );
                style.dynamic(TemplatedControl::foreground_property(), "SplitButtonForegroundPressed");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SplitButtonBackgroundPressed");
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "SplitButtonForegroundSecondaryPressed",
                );
                style.set(Grid::column_property(), 0);
                style.set(Grid::column_span_property(), 3);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // FlyoutOpen
    theme.style(nesting().class(":flyout-open"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SplitButtonBackgroundPressed");
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushPressed",
                );
                style.dynamic(TemplatedControl::foreground_property(), "SplitButtonForegroundPressed");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SplitButtonBackgroundPressed");
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "SplitButtonForegroundSecondaryPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Checked
    theme.style(nesting().class(":checked"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SplitButtonBackgroundChecked");
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushChecked",
                );
                style.dynamic(TemplatedControl::foreground_property(), "SplitButtonForegroundChecked");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SplitButtonBackgroundChecked");
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushChecked",
                );
                style.dynamic(TemplatedControl::foreground_property(), "SplitButtonForegroundChecked");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Grid>().name("DividerBackgroundGrid"),
            |style| {
                style.dynamic(Panel::background_property(), "SplitButtonBorderBrushCheckedDivider");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // CheckedPrimaryPointerOver
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Button>()
            .name("PART_PrimaryButton")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "SplitButtonBackgroundCheckedPointerOver",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "SplitButtonForegroundCheckedPointerOver",
            );
            Ok(())
        },
    )?;
    // CheckedPrimaryPressed
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Button>()
            .name("PART_PrimaryButton")
            .class(":pressed"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "SplitButtonBackgroundCheckedPressed",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "SplitButtonForegroundCheckedPressed",
            );
            Ok(())
        },
    )?;
    // CheckedSecondaryPointerOver
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Button>()
            .name("PART_SecondaryButton")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "SplitButtonBackgroundCheckedPointerOver",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "SplitButtonForegroundCheckedPointerOver",
            );
            Ok(())
        },
    )?;
    // CheckedSecondaryPressed
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Button>()
            .name("PART_SecondaryButton")
            .class(":pressed"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "SplitButtonBackgroundCheckedPressed",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "SplitButtonForegroundCheckedPressed",
            );
            Ok(())
        },
    )?;
    // CheckedTouchPressed
    theme.style(nesting().class(":checked").class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "SplitButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "SplitButtonForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "SplitButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "SplitButtonForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // CheckedFlyoutOpen
    theme.style(nesting().class(":checked").class(":flyout-open"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "SplitButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "SplitButtonForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "SplitButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "SplitButtonForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled (the state replaces every other state of the group)
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.set(
                    TemplatedControl::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushDisabled",
                );
                style.dynamic(TemplatedControl::foreground_property(), "SplitButtonForegroundDisabled");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.set(
                    TemplatedControl::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                style.dynamic(
                    TemplatedControl::border_brush_property(),
                    "SplitButtonBorderBrushDisabled",
                );
                style.dynamic(TemplatedControl::foreground_property(), "SplitButtonForegroundDisabled");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Grid>().name("DividerBackgroundGrid"),
            |style| {
                style.dynamic(Panel::background_property(), "SplitButtonBorderBrushDivider");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.set(
                Border::background_property(),
                Some(literal(Brush::parse("Transparent"))?),
            );
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `SplitButton`.
fn split_button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<SplitButton>();
    theme.based_on("SplitButtonStyle")?;
    Ok(())
}

/// The focus adorner of `split_button_command_bar_inner_button_style`.
fn split_button_command_bar_inner_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `Button` of `split_button_command_bar_inner_button_style`.
fn split_button_command_bar_inner_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("RootGrid")?
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([t
            .make(ContentPresenter::new())
            .name("PART_ContentPresenter")?
            .template_bind(
                ContentPresenter::foreground_property(),
                TemplatedControl::foreground_property(),
            )
            .template_bind(
                ContentPresenter::border_brush_property(),
                TemplatedControl::border_brush_property(),
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
            .set(ContentPresenter::recognizes_access_key_property(), true)
            .control()])
        .control())
}

/// `SplitButtonCommandBarInnerButtonStyle`: a control theme of `Button`.
fn split_button_command_bar_inner_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForeground");
    theme.set(
        TemplatedControl::border_brush_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            split_button_command_bar_inner_button_style_focus_adorner,
        )),
    );
    theme.template(split_button_command_bar_inner_button_style_template);
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "AppBarButtonForegroundDisabled",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "AppBarButtonForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Grid>().name("RootGrid"), |style| {
            style.dynamic(Panel::background_property(), "AppBarButtonBackgroundDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The focus adorner of `split_button_command_bar_style`.
fn split_button_command_bar_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `SplitButton` of `split_button_command_bar_style`.
fn split_button_command_bar_style_template(t: &Template<SplitButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .set(
            Border::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .dynamic(Decorator::padding_property(), "AppBarButtonInnerBorderMargin")
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,23"))?);
                    Ok(())
                })?
                // The corner radii of the backgrounds are ControlCornerRadius through the filters
                //                 in WinUI; the CornerRadius of the control, which this theme sets to it, here.
                .children([
                    t.make(Border::new())
                        .name("PrimaryBackgroundGrid")?
                        .bind(
                            Border::background_property(),
                            path()
                                .named(t.names(), "PART_PrimaryButton")
                                .prop(TemplatedControl::background_property())
                                .binding(),
                        )
                        .template_bind_with(
                            Border::corner_radius_property(),
                            Some(TemplatedControl::corner_radius_property().property()),
                            Some(t.scope().converter("LeftCornerRadiusFilterConverter")?),
                            None,
                            None,
                        )
                        .control(),
                    t.make(Border::new())
                        .name("SecondaryBackgroundGrid")?
                        .set(Grid::column_property(), 1)
                        .bind(
                            Border::background_property(),
                            path()
                                .named(t.names(), "PART_SecondaryButton")
                                .prop(TemplatedControl::background_property())
                                .binding(),
                        )
                        .template_bind_with(
                            Border::corner_radius_property(),
                            Some(TemplatedControl::corner_radius_property().property()),
                            Some(t.scope().converter("RightCornerRadiusFilterConverter")?),
                            None,
                            None,
                        )
                        .control(),
                    t.make(Border::new())
                        .name("Border")?
                        .set(Grid::column_span_property(), 2)
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
                        .control(),
                    t.make(Button::new())
                        .name("PART_PrimaryButton")?
                        .set(Grid::column_property(), 0)
                        .static_resource(StyledElement::theme_property(), "SplitButtonCommandBarInnerButtonStyle")?
                        .template_bind(
                            TemplatedControl::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .template_bind(
                            TemplatedControl::background_property(),
                            TemplatedControl::background_property(),
                        )
                        .template_bind(
                            TemplatedControl::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .set(
                            TemplatedControl::border_brush_property(),
                            Some(literal(Brush::parse("Transparent"))?),
                        )
                        .template_bind(ContentControl::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentControl::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .template_bind(Button::command_property(), SplitButton::command_property())
                        .template_bind(
                            Button::command_parameter_property(),
                            SplitButton::command_parameter_property(),
                        )
                        .template_bind(
                            TemplatedControl::font_family_property(),
                            TemplatedControl::font_family_property(),
                        )
                        .template_bind(
                            TemplatedControl::font_size_property(),
                            TemplatedControl::font_size_property(),
                        )
                        .template_bind(
                            TemplatedControl::font_weight_property(),
                            TemplatedControl::font_weight_property(),
                        )
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                        .template_bind(
                            ContentControl::horizontal_content_alignment_property(),
                            ContentControl::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            ContentControl::vertical_content_alignment_property(),
                            ContentControl::vertical_content_alignment_property(),
                        )
                        .template_bind(
                            TemplatedControl::padding_property(),
                            TemplatedControl::padding_property(),
                        )
                        .set(InputElement::is_tab_stop_property(), false)
                        .set(InputElement::focusable_property(), false)
                        .control(),
                    t.make(Button::new())
                        .name("PART_SecondaryButton")?
                        .set(Grid::column_property(), 1)
                        .static_resource(StyledElement::theme_property(), "SplitButtonCommandBarInnerButtonStyle")?
                        .dynamic(TemplatedControl::foreground_property(), "AppBarButtonForeground")
                        .template_bind(
                            TemplatedControl::background_property(),
                            TemplatedControl::background_property(),
                        )
                        .template_bind(
                            TemplatedControl::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .set(
                            TemplatedControl::border_brush_property(),
                            Some(literal(Brush::parse("Transparent"))?),
                        )
                        .set(
                            ContentControl::horizontal_content_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(
                            ContentControl::vertical_content_alignment_property(),
                            VerticalAlignment::Stretch,
                        )
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                        .set(TemplatedControl::padding_property(), Thickness::new(2.0, 0.0, 8.0, 0.0))
                        .set(InputElement::is_tab_stop_property(), false)
                        .set(InputElement::focusable_property(), false)
                        // The TextBlock of the glyph E972 at 8 px with Padding="2,1,2,0".
                        .content(
                            ContentControl::content_property(),
                            t.make(Path::new())
                                .static_resource(Path::data_property(), "GlyphGeometryE972")?
                                .bind(
                                    Shape::fill_property(),
                                    path()
                                        .ancestor_of::<Button>(0)
                                        .prop(TemplatedControl::foreground_property())
                                        .binding(),
                                )
                                .set(Shape::stretch_property(), Stretch::Uniform)
                                .set(Layoutable::width_property(), 8.0)
                                .set(Layoutable::margin_property(), Thickness::new(2.0, 1.0, 2.0, 0.0))
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right),
                        )?
                        .control(),
                ]),
        )
        .control())
}

/// `SplitButtonCommandBarStyle`: a control theme of `SplitButton`.
fn split_button_command_bar_style(theme: &StyleBuilder) -> Result {
    theme.target::<SplitButton>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    //         (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::background_property(), "AppBarButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "AppBarButtonBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "SplitButtonBorderThemeThickness",
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Right,
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.set(TemplatedControl::font_size_property(), 12.0);
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            split_button_command_bar_style_focus_adorner,
        )),
    );
    theme.set(InputElement::is_tab_stop_property(), true);
    theme.set(InputElement::focusable_property(), true);
    theme.set(
        TemplatedControl::padding_property(),
        Thickness::new(12.0, 8.0, 4.0, 8.0),
    );
    theme.dynamic(Layoutable::min_height_property(), "AppBarThemeCompactHeight");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(split_button_command_bar_style_template);
    // The values PrimaryPointerOver, PrimaryPressed, SecondaryPointerOver and SecondaryPressed
    //         give the half that is not under the pointer.
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "SplitButtonInAppBarUnfocusedPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "SplitButtonInAppBarUnfocusedPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // PrimaryPointerOver
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("PART_PrimaryButton")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "AppBarButtonBackgroundPointerOver",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "AppBarButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    // PrimaryPressed
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("PART_PrimaryButton")
            .class(":pressed"),
        |style| {
            style.dynamic(TemplatedControl::background_property(), "AppBarButtonBackgroundPressed");
            style.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForegroundPressed");
            Ok(())
        },
    )?;
    // SecondaryPointerOver
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("PART_SecondaryButton")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "AppBarButtonBackgroundPointerOver",
            );
            style.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForeground");
            Ok(())
        },
    )?;
    // SecondaryPressed
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("PART_SecondaryButton")
            .class(":pressed"),
        |style| {
            style.dynamic(TemplatedControl::background_property(), "AppBarButtonBackgroundPressed");
            style.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForegroundPressed");
            Ok(())
        },
    )?;
    // TouchPressed, with SecondaryButtonSpan of SecondaryButtonPlacementStates
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "AppBarButtonBackgroundPressed");
                style.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForegroundPressed");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "AppBarButtonBackgroundPressed");
                style.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForegroundPressed");
                style.set(Grid::column_property(), 0);
                style.set(Grid::column_span_property(), 3);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("Border"), |style| {
            style.dynamic(Border::border_brush_property(), "AppBarButtonBorderBrushPressed");
            Ok(())
        })?;
        Ok(())
    })?;
    // FlyoutOpen
    theme.style(nesting().class(":flyout-open"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "AppBarButtonBackgroundPressed");
                style.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForegroundPressed");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "AppBarButtonBackgroundPressed");
                style.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForegroundPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("Border"), |style| {
            style.dynamic(Border::border_brush_property(), "AppBarButtonBorderBrushPressed");
            Ok(())
        })?;
        Ok(())
    })?;
    // Checked
    theme.style(nesting().class(":checked"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "AppBarToggleButtonBackgroundChecked",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "AppBarToggleButtonForegroundChecked",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "AppBarToggleButtonBackgroundChecked",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "AppBarToggleButtonForegroundChecked",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("Border"), |style| {
            style.dynamic(Border::border_brush_property(), "AppBarToggleButtonBorderBrushChecked");
            Ok(())
        })?;
        Ok(())
    })?;
    // CheckedPrimaryPointerOver
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Button>()
            .name("PART_PrimaryButton")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "AppBarToggleButtonBackgroundCheckedPointerOver",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "AppBarToggleButtonForegroundCheckedPointerOver",
            );
            Ok(())
        },
    )?;
    // CheckedPrimaryPressed
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Button>()
            .name("PART_PrimaryButton")
            .class(":pressed"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "AppBarToggleButtonBackgroundCheckedPressed",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "AppBarToggleButtonForegroundCheckedPressed",
            );
            Ok(())
        },
    )?;
    // CheckedSecondaryPointerOver
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Button>()
            .name("PART_SecondaryButton")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "AppBarToggleButtonBackgroundCheckedPointerOver",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "AppBarToggleButtonForegroundCheckedPointerOver",
            );
            Ok(())
        },
    )?;
    // CheckedSecondaryPressed
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Button>()
            .name("PART_SecondaryButton")
            .class(":pressed"),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "AppBarToggleButtonBackgroundCheckedPressed",
            );
            style.dynamic(
                TemplatedControl::foreground_property(),
                "AppBarToggleButtonForegroundCheckedPressed",
            );
            Ok(())
        },
    )?;
    // CheckedTouchPressed
    theme.style(nesting().class(":checked").class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "AppBarToggleButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "AppBarToggleButtonForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "AppBarToggleButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "AppBarToggleButtonForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("Border"), |style| {
            style.dynamic(
                Border::border_brush_property(),
                "AppBarToggleButtonBorderBrushCheckedPressed",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // CheckedFlyoutOpen
    theme.style(nesting().class(":checked").class(":flyout-open"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "AppBarToggleButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "AppBarToggleButtonForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "AppBarToggleButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "AppBarToggleButtonForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("Border"), |style| {
            style.dynamic(
                Border::border_brush_property(),
                "AppBarToggleButtonBorderBrushCheckedPressed",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled (the state replaces every other state of the group)
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting().template().of::<Button>().name("PART_PrimaryButton"),
            |style| {
                style.template_bind(
                    TemplatedControl::background_property(),
                    TemplatedControl::background_property(),
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "AppBarButtonForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_SecondaryButton"),
            |style| {
                style.template_bind(
                    TemplatedControl::background_property(),
                    TemplatedControl::background_property(),
                );
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "AppBarButtonForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("Border"), |style| {
            style.dynamic(Border::border_brush_property(), "AppBarButtonBorderBrushDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}
