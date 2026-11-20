//! `Controls/DropDownButton.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/DropDownButton/DropDownButton.xaml (the styles; the resources of
//!   controls/dev/DropDownButton/DropDownButton_themeresources.xaml are
//!   Resources/DropDownButton_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The Grid RootGrid draws the background, the border and the padding: it is a Border of the
//!   same name around the Grid of the two columns; Grid.BackgroundTransition is a
//!   SolidColorBrushTransition of its Background (WinUI's BrushTransition, BTN-3).
//! - ContentPresenter is PART_ContentPresenter.
//! - The chevron is an AnimatedIcon (AnimatedChevronDownSmallVisualSource) of 12 x 12 with the
//!   fallback glyph E96E (ChevronDownSmall) of the symbol font at 8 px: a Path of
//!   GlyphGeometryE96E, 8 px wide, centred in a 12 x 12 box. The AnimatedIcon.State setters of
//!   the states are left out (no AnimatedIcon in FerroUI); its Foreground is the Fill of the path.
//! - UseSystemFocusVisuals with FocusVisualMargin="-3" is the FocusAdorner.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::SolidColorBrushTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::FontWeight;
use ferroui_base::media::Stretch;
use ferroui_base::Thickness;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::DropDownButton;
use ferroui_controls::Grid;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultDropDownButtonStyle", default_drop_down_button_style);
    resources.control_theme(type_key::<DropDownButton>(), drop_down_button_theme);
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

/// The focus adorner of `default_drop_down_button_style`.
fn default_drop_down_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `DropDownButton` of `default_drop_down_button_style`.
fn default_drop_down_button_style_template(t: &Template<DropDownButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
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
        .template_bind(
            Border::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .set(
            Animatable::transitions_property(),
            Some({
                let object = Transitions::new();
                object.add(
                    t.make(SolidColorBrushTransition::new())
                        .set(
                            TransitionBase::property_property(),
                            Some(Border::background_property().property()),
                        )
                        .set(
                            TransitionBase::duration_property(),
                            SignedDuration::from_milliseconds(83.0),
                        )
                        .end()
                        .into(),
                );
                object
            }),
        )
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    e.set_column_definitions(literal(ColumnDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
                // The AnimatedIcon ChevronIcon (12 x 12) with its fallback glyph E96E at 8 px.
                .children([
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
                            ContentPresenter::horizontal_content_alignment_property(),
                            ContentControl::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            ContentPresenter::vertical_content_alignment_property(),
                            ContentControl::vertical_content_alignment_property(),
                        )
                        .set(ContentPresenter::recognizes_access_key_property(), true)
                        .control(),
                    t.make(Grid::new())
                        .set(Grid::column_property(), 1)
                        .set(Layoutable::margin_property(), Thickness::new(8.0, 0.0, 0.0, 0.0))
                        .set(Layoutable::width_property(), 12.0)
                        .set(Layoutable::height_property(), 12.0)
                        .children([t
                            .make(Path::new())
                            .name("ChevronIcon")?
                            .static_resource(Path::data_property(), "GlyphGeometryE96E")?
                            .dynamic(Shape::fill_property(), "DropDownButtonForegroundSecondary")
                            .set(Shape::stretch_property(), Stretch::Uniform)
                            .set(Layoutable::width_property(), 8.0)
                            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                            .control()])
                        .control(),
                ]),
        )
        .control())
}

/// `DefaultDropDownButtonStyle`: a control theme of `DropDownButton`.
fn default_drop_down_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<DropDownButton>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    // (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::background_property(), "ButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "ButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "ButtonBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ButtonBorderThemeThickness",
    );
    theme.static_resource(TemplatedControl::padding_property(), "ButtonPadding")?;
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            default_drop_down_button_style_focus_adorner,
        )),
    );
    theme.template(default_drop_down_button_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "ButtonBackgroundPointerOver");
            style.dynamic(Border::border_brush_property(), "ButtonBorderBrushPointerOver");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundPointerOver");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("ChevronIcon"), |style| {
            style.dynamic(Shape::fill_property(), "DropDownButtonForegroundSecondaryPointerOver");
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "ButtonBackgroundPressed");
            style.dynamic(Border::border_brush_property(), "ButtonBorderBrushPressed");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("ChevronIcon"), |style| {
            style.dynamic(Shape::fill_property(), "DropDownButtonForegroundSecondaryPressed");
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "ButtonBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "ButtonBorderBrushDisabled");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("ChevronIcon"), |style| {
            style.dynamic(Shape::fill_property(), "ButtonForegroundDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `DropDownButton`.
fn drop_down_button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<DropDownButton>();
    theme.based_on("DefaultDropDownButtonStyle")?;
    Ok(())
}
