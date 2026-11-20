//! `Controls/RadioButton.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/RadioButton_themeresources.xaml (the styles; its resources are
//!   Resources/RadioButton_themeresources.xaml)
//!   controls/dev/RadioButtons/RadioButton.xaml (the same implicit style, merged before it)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The Grid RootGrid has a background, a border and a corner radius: it is a Border of the same
//!   name around the Grid.
//! - The visual states animate the size of CheckGlyph and PressedCheckGlyph to a value
//!   (SplineDoubleKeyFrame with ControlFastOutSlowInKeySpline "0,0,0,1" over
//!   ControlNormalAnimationDuration, 250 ms, or ControlFastAnimationDuration, 167 ms). Here the
//!   state sets the value and a transition of the element animates it, with the same easing; the
//!   return to the base value animates too, where WinUI ends the animation at once.
//! - The groups CommonStates and CheckStates both set the stroke of CheckGlyph and the opacity of
//!   PressedCheckGlyph; the check state, the later group, is the later style and wins.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::TextWrapping;
use ferroui_base::CornerRadius;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::RadioButton;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultRadioButtonStyle", default_radio_button_style);
    resources.control_theme(type_key::<RadioButton>(), radio_button_theme);
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

/// The focus adorner of `default_radio_button_style`.
fn default_radio_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::new(-7.0, -3.0, -7.0, -3.0))
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

/// The template of `RadioButton` of `default_radio_button_style`.
fn default_radio_button_style_template(t: &Template<RadioButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
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
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("20,*"))?);
                    Ok(())
                })?
                .children([
                    t.make(Grid::new())
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                        .set(Layoutable::height_property(), 32.0)
                        // A separate element is added since the two orthogonal state groups that cannot touch the same property
                        // A separate element is added since the two orthogonal state groups that cannot touch the same property
                        .children([
                            t.make(Ellipse::new())
                                .name("OuterEllipse")?
                                .set(Layoutable::width_property(), 20.0)
                                .set(Layoutable::height_property(), 20.0)
                                .set(Layoutable::use_layout_rounding_property(), false)
                                .dynamic(Shape::stroke_property(), "RadioButtonOuterEllipseStroke")
                                .dynamic(Shape::fill_property(), "RadioButtonOuterEllipseFill")
                                .dynamic(Shape::stroke_thickness_property(), "RadioButtonBorderThemeThickness")
                                .control(),
                            t.make(Ellipse::new())
                                .name("CheckOuterEllipse")?
                                .set(Layoutable::width_property(), 20.0)
                                .set(Layoutable::height_property(), 20.0)
                                .set(Layoutable::use_layout_rounding_property(), false)
                                .dynamic(Shape::stroke_property(), "RadioButtonOuterEllipseCheckedStroke")
                                .dynamic(Shape::fill_property(), "RadioButtonOuterEllipseCheckedFill")
                                .set(Visual::opacity_property(), 0.0)
                                .dynamic(Shape::stroke_thickness_property(), "RadioButtonBorderThemeThickness")
                                .control(),
                            t.make(Ellipse::new())
                                .name("CheckGlyph")?
                                .dynamic(Layoutable::width_property(), "RadioButtonCheckGlyphSize")
                                .dynamic(Layoutable::height_property(), "RadioButtonCheckGlyphSize")
                                .set(Layoutable::use_layout_rounding_property(), false)
                                .set(Visual::opacity_property(), 0.0)
                                .dynamic(Shape::fill_property(), "RadioButtonCheckGlyphFill")
                                .dynamic(Shape::stroke_property(), "RadioButtonCheckGlyphStroke")
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Layoutable::width_property().property()),
                                                )
                                                .set(
                                                    TransitionBase::duration_property(),
                                                    SignedDuration::from_milliseconds(250.0),
                                                )
                                                .set(
                                                    TransitionBase::easing_property(),
                                                    literal(Easing::parse("0,0,0,1"))?,
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Layoutable::height_property().property()),
                                                )
                                                .set(
                                                    TransitionBase::duration_property(),
                                                    SignedDuration::from_milliseconds(250.0),
                                                )
                                                .set(
                                                    TransitionBase::easing_property(),
                                                    literal(Easing::parse("0,0,0,1"))?,
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object
                                    }),
                                )
                                .control(),
                            t.make(Border::new())
                                .name("PressedCheckGlyph")?
                                .set(Layoutable::width_property(), 4.0)
                                .set(Layoutable::height_property(), 4.0)
                                .set(Border::corner_radius_property(), CornerRadius::uniform(6.0))
                                .set(Layoutable::use_layout_rounding_property(), false)
                                .set(Visual::opacity_property(), 0.0)
                                .dynamic(Border::background_property(), "RadioButtonCheckGlyphFill")
                                .set(Border::background_sizing_property(), BackgroundSizing::OuterBorderEdge)
                                .dynamic(Border::border_brush_property(), "RadioButtonCheckGlyphStroke")
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Layoutable::width_property().property()),
                                                )
                                                .set(
                                                    TransitionBase::duration_property(),
                                                    SignedDuration::from_milliseconds(167.0),
                                                )
                                                .set(
                                                    TransitionBase::easing_property(),
                                                    literal(Easing::parse("0,0,0,1"))?,
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Layoutable::height_property().property()),
                                                )
                                                .set(
                                                    TransitionBase::duration_property(),
                                                    SignedDuration::from_milliseconds(167.0),
                                                )
                                                .set(
                                                    TransitionBase::easing_property(),
                                                    literal(Easing::parse("0,0,0,1"))?,
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object
                                    }),
                                )
                                .control(),
                        ])
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(Grid::column_property(), 1)
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .template_bind(
                            ContentPresenter::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                        .template_bind(
                            Layoutable::horizontal_alignment_property(),
                            ContentControl::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            Layoutable::vertical_alignment_property(),
                            ContentControl::vertical_content_alignment_property(),
                        )
                        .set(ContentPresenter::text_wrapping_property(), TextWrapping::Wrap)
                        .set(ContentPresenter::recognizes_access_key_property(), true)
                        .control(),
                ]),
        )
        .control())
}

/// `DefaultRadioButtonStyle`: a control theme of `RadioButton`.
fn default_radio_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RadioButton>();
    theme.dynamic(TemplatedControl::background_property(), "RadioButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "RadioButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "RadioButtonBorderBrush");
    theme.set(TemplatedControl::padding_property(), Thickness::new(8.0, 6.0, 0.0, 0.0));
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Top,
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(Layoutable::min_width_property(), 120.0);
    theme.static_resource(TemplatedControl::corner_radius_property(), "ControlCornerRadius")?;
    // UseSystemFocusVisuals with FocusVisualMargin="-7,-3,-7,-3".
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_radio_button_style_focus_adorner)),
    );
    theme.template(default_radio_button_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "RadioButtonForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "RadioButtonBackgroundPointerOver");
            style.dynamic(Border::border_brush_property(), "RadioButtonBorderBrushPointerOver");
            Ok(())
        })?;
        style.style(nesting().template().of::<Ellipse>().name("OuterEllipse"), |style| {
            style.dynamic(Shape::stroke_property(), "RadioButtonOuterEllipseStrokePointerOver");
            style.dynamic(Shape::fill_property(), "RadioButtonOuterEllipseFillPointerOver");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Ellipse>().name("CheckOuterEllipse"),
            |style| {
                style.dynamic(
                    Shape::stroke_property(),
                    "RadioButtonOuterEllipseCheckedStrokePointerOver",
                );
                style.dynamic(Shape::fill_property(), "RadioButtonOuterEllipseCheckedFillPointerOver");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Ellipse>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "RadioButtonCheckGlyphFillPointerOver");
            style.dynamic(Shape::stroke_property(), "RadioButtonCheckGlyphStrokePointerOver");
            style.set(Layoutable::width_property(), 14.0);
            style.set(Layoutable::height_property(), 14.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "RadioButtonForegroundPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "RadioButtonBackgroundPressed");
            style.dynamic(Border::border_brush_property(), "RadioButtonBorderBrushPressed");
            Ok(())
        })?;
        style.style(nesting().template().of::<Ellipse>().name("OuterEllipse"), |style| {
            style.dynamic(Shape::stroke_property(), "RadioButtonOuterEllipseStrokePressed");
            style.dynamic(Shape::fill_property(), "RadioButtonOuterEllipseFillPressed");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Ellipse>().name("CheckOuterEllipse"),
            |style| {
                style.dynamic(Shape::stroke_property(), "RadioButtonOuterEllipseCheckedStrokePressed");
                style.dynamic(Shape::fill_property(), "RadioButtonOuterEllipseCheckedFillPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Ellipse>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "RadioButtonCheckGlyphFillPressed");
            style.dynamic(Shape::stroke_property(), "RadioButtonCheckGlyphStrokePressed");
            style.set(Layoutable::width_property(), 10.0);
            style.set(Layoutable::height_property(), 10.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("PressedCheckGlyph"), |style| {
            style.set(Visual::opacity_property(), 1.0);
            style.set(Layoutable::width_property(), 10.0);
            style.set(Layoutable::height_property(), 10.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "RadioButtonForegroundDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "RadioButtonBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "RadioButtonBorderBrushDisabled");
            Ok(())
        })?;
        style.style(nesting().template().of::<Ellipse>().name("OuterEllipse"), |style| {
            style.dynamic(Shape::stroke_property(), "RadioButtonOuterEllipseStrokeDisabled");
            style.dynamic(Shape::fill_property(), "RadioButtonOuterEllipseFillDisabled");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Ellipse>().name("CheckOuterEllipse"),
            |style| {
                style.dynamic(Shape::stroke_property(), "RadioButtonOuterEllipseCheckedStrokeDisabled");
                style.dynamic(Shape::fill_property(), "RadioButtonOuterEllipseCheckedFillDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Ellipse>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "RadioButtonCheckGlyphFillDisabled");
            style.dynamic(Shape::stroke_property(), "RadioButtonCheckGlyphStrokeDisabled");
            style.set(Layoutable::width_property(), 14.0);
            style.set(Layoutable::height_property(), 14.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Checked
    theme.style(nesting().class(":checked"), |style| {
        style.style(nesting().template().of::<Ellipse>().name("CheckGlyph"), |style| {
            style.set(Visual::opacity_property(), 1.0);
            style.dynamic(Shape::stroke_property(), "RadioButtonCheckGlyphStrokeChecked");
            Ok(())
        })?;
        style.style(nesting().template().of::<Ellipse>().name("OuterEllipse"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Ellipse>().name("CheckOuterEllipse"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("PressedCheckGlyph"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            style.dynamic(Border::background_property(), "RadioButtonCheckGlyphFillPressed");
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `RadioButton`.
fn radio_button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<RadioButton>();
    theme.based_on("DefaultRadioButtonStyle")?;
    Ok(())
}
