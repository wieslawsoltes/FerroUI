//! `Controls/Slider.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/Slider_themeresources.xaml (the styles; its resources are
//!   Resources/Slider_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - One template with the roots HorizontalTemplate and VerticalTemplate, which the control shows by
//!   its orientation, is one template per orientation in the nested styles :horizontal and
//!   :vertical, so that each template has its parts once.
//! - FerroUI's slider arranges its thumb and the parts of the track with a Track (PART_Track): the
//!   rectangles HorizontalDecreaseRect and VerticalDecreaseRect, which WinUI's control sizes to the
//!   value, are drawn by the decrease button of the track (PART_DecreaseButton, keyed themes
//!   SliderHorizontalDecreaseButtonStyle and SliderVerticalDecreaseButtonStyle, whose Background is
//!   the fill of the rectangle); the increase button (PART_IncreaseButton,
//!   SliderIncreaseButtonStyle) draws nothing, as the part of the track after the thumb in WinUI.
//!   The track rectangles HorizontalTrackRect and VerticalTrackRect are kept behind the track.
//! - The template-local style SliderThumbStyle (Grid.Resources) is the keyed Thumb theme
//!   SliderThumbStyle. Its inner ellipse is scaled by the states of the thumb (Normal 0.86,
//!   PointerOver 1.167, Pressed 0.71, Disabled 1.167 of SliderInnerThumbWidth): a RenderTransform
//!   with one transition of 250 ms and the spline 0,0,0,1 (ControlNormalAnimationDuration and
//!   ControlFastOutSlowInKeySpline; Normal and Disabled use ControlFastAnimationDuration, 167 ms,
//!   in the source).
//! - FerroUI's Slider has no Header: HeaderContentPresenter and its row are left out. The
//!   TickPlacement Inline (HorizontalInlineTickBar, VerticalInlineTickBar) does not exist in
//!   FerroUI: those tick bars are left out. The visibility of the other tick bars, which the control
//!   sets, is a selector of TickPlacement.
//! - The Border FocusBorder exists to carry the focus rectangle: TemplatedControl.IsTemplateFocusTarget.
//!   UseSystemFocusVisuals with FocusVisualMargin="-7,0,-7,0" is the FocusAdorner. The
//!   FocusEngagementStates (IsFocusEngagementEnabled, game pad focus engagement) and
//!   ManipulationMode have no counterpart. The FocusVisualMargin of the thumbs is left out (the
//!   thumbs are not focusable in FerroUI).
//! - Row and column sizes of resources (SliderPreContentMargin, SliderPostContentMargin, 14) are
//!   written as values. The tick bars take the bounds of the thumb as their ReservedSpace (FerroUI's
//!   TickBar places its ticks inside it), as in the Fluent theme.
//! - The FerroUI contract: :error (data validation, which WinUI 3 does not have) colours the value
//!   fill and the thumb with SystemControlErrorTextForegroundBrush, as the Fluent theme does, and
//!   the templates are wrapped in DataValidationErrors, which shows the errors under the slider,
//!   as the Fluent theme wraps them.
//! - SliderThumbStyle is not based on the default theme of Thumb, as the template-local themes of
//!   the foundation.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransformOperationsTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::styling::Selectors;
use ferroui_base::RelativePoint;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::Thumb;
use ferroui_controls::primitives::Track;
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinition;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::DataValidationErrors;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::Panel;
use ferroui_controls::RepeatButton;
use ferroui_controls::RowDefinition;
use ferroui_controls::RowDefinitions;
use ferroui_controls::Slider;
use ferroui_controls::TickBar;
use ferroui_controls::TickBarPlacement;
use ferroui_controls::TickPlacement;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // SliderThumbStyle of the Grid.Resources of the template.
    resources.control_theme("SliderThumbStyle", slider_thumb_style);
    // The decrease button of the horizontal track: HorizontalDecreaseRect.
    resources.control_theme(
        "SliderHorizontalDecreaseButtonStyle",
        slider_horizontal_decrease_button_style,
    );
    // The decrease button of the vertical track: VerticalDecreaseRect.
    resources.control_theme(
        "SliderVerticalDecreaseButtonStyle",
        slider_vertical_decrease_button_style,
    );
    // The increase button of the track: the part of the track after the thumb, which shows the
    // track rectangle behind it.
    resources.control_theme("SliderIncreaseButtonStyle", slider_increase_button_style);
    resources.control_theme("DefaultSliderStyle", default_slider_style);
    resources.control_theme(type_key::<Slider>(), slider_theme);
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

/// The template of `Thumb` of `slider_thumb_style`.
fn slider_thumb_style_template(t: &Template<Thumb>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-2.0))
        .dynamic(Border::background_property(), "SliderOuterThumbBackground")
        .dynamic(Border::border_brush_property(), "SliderThumbBorderBrush")
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .dynamic(Border::corner_radius_property(), "SliderThumbCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Ellipse::new())
                .name("SliderInnerThumb")?
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
                )
                .set(
                    Visual::render_transform_property(),
                    Some(literal(TransformOperations::parse("scale(0.86)"))?),
                )
                .template_bind(Shape::fill_property(), TemplatedControl::background_property())
                .dynamic(Layoutable::width_property(), "SliderInnerThumbWidth")
                .dynamic(Layoutable::height_property(), "SliderInnerThumbHeight")
                .set(
                    Animatable::transitions_property(),
                    Some({
                        let object = Transitions::new();
                        object.add(
                            t.make(TransformOperationsTransition::new())
                                .set(
                                    TransitionBase::property_property(),
                                    Some(Visual::render_transform_property().property()),
                                )
                                .set(
                                    TransitionBase::duration_property(),
                                    SignedDuration::from_milliseconds(250.0),
                                )
                                .set(TransitionBase::easing_property(), literal(Easing::parse("0,0,0,1"))?)
                                .end()
                                .into(),
                        );
                        object
                    }),
                ),
        )
        .control())
}

/// `SliderThumbStyle`: a control theme of `Thumb`.
fn slider_thumb_style(theme: &StyleBuilder) -> Result {
    theme.target::<Thumb>();
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0));
    theme.dynamic(TemplatedControl::background_property(), "SliderThumbBackground");
    theme.template(slider_thumb_style_template);
    // Normal sets the base value (0.86, from 14 to 12 px).
    // PointerOver (1.167, from 12 to 14 px)
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Ellipse>()
            .name("SliderInnerThumb"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scale(1.167)"))?),
            );
            Ok(())
        },
    )?;
    // Pressed (0.71, from 14 to 10 px)
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<Ellipse>()
            .name("SliderInnerThumb"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scale(0.71)"))?),
            );
            Ok(())
        },
    )?;
    // Disabled (1.167)
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<Ellipse>()
            .name("SliderInnerThumb"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scale(1.167)"))?),
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `RepeatButton` of `slider_horizontal_decrease_button_style`.
fn slider_horizontal_decrease_button_style_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([t
            .make(Rectangle::new())
            .name("HorizontalDecreaseRect")?
            .template_bind(Shape::fill_property(), TemplatedControl::background_property())
            .dynamic(Layoutable::height_property(), "SliderTrackThemeHeight")
            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
            .template_bind_with(
                Rectangle::radius_x_property(),
                Some(TemplatedControl::corner_radius_property().property()),
                Some(t.scope().converter("TopLeftCornerRadiusDoubleValueConverter")?),
                None,
                None,
            )
            .template_bind_with(
                Rectangle::radius_y_property(),
                Some(TemplatedControl::corner_radius_property().property()),
                Some(t.scope().converter("BottomRightCornerRadiusDoubleValueConverter")?),
                None,
                None,
            )
            .control()])
        .control())
}

/// `SliderHorizontalDecreaseButtonStyle`: a control theme of `RepeatButton`.
fn slider_horizontal_decrease_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.template(slider_horizontal_decrease_button_style_template);
    Ok(())
}

/// The template of `RepeatButton` of `slider_vertical_decrease_button_style`.
fn slider_vertical_decrease_button_style_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([t
            .make(Rectangle::new())
            .name("VerticalDecreaseRect")?
            .template_bind(Shape::fill_property(), TemplatedControl::background_property())
            .dynamic(Layoutable::width_property(), "SliderTrackThemeHeight")
            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
            .template_bind_with(
                Rectangle::radius_x_property(),
                Some(TemplatedControl::corner_radius_property().property()),
                Some(t.scope().converter("TopLeftCornerRadiusDoubleValueConverter")?),
                None,
                None,
            )
            .template_bind_with(
                Rectangle::radius_y_property(),
                Some(TemplatedControl::corner_radius_property().property()),
                Some(t.scope().converter("BottomRightCornerRadiusDoubleValueConverter")?),
                None,
                None,
            )
            .control()])
        .control())
}

/// `SliderVerticalDecreaseButtonStyle`: a control theme of `RepeatButton`.
fn slider_vertical_decrease_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.template(slider_vertical_decrease_button_style_template);
    Ok(())
}

/// The template of `RepeatButton` of `slider_increase_button_style`.
fn slider_increase_button_style_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .control())
}

/// `SliderIncreaseButtonStyle`: a control theme of `RepeatButton`.
fn slider_increase_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.template(slider_increase_button_style_template);
    Ok(())
}

/// The focus adorner of `default_slider_style`.
fn default_slider_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::new(-7.0, 0.0, -7.0, 0.0))
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

/// The template of `Slider` of `default_slider_style`.
fn default_slider_style_template(t: &Template<Slider>) -> Result<Ref<Control>> {
    // The errors of FerroUI's data validation under the slider (DataValidationErrors.xaml).
    Ok(t.make(DataValidationErrors::new())
        .content(
            ContentControl::content_property(),
            t.make(Grid::new())
                .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                .children([
                    t.make(Border::new())
                        .name("FocusBorder")?
                        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
                        .set(TemplatedControl::is_template_focus_target_property(), true)
                        .control(),
                    t.make(Grid::new())
                        .name("SliderContainer")?
                        .dynamic(Panel::background_property(), "SliderContainerBackground")
                        .style(of::<TickBar>(), |style| {
                            style.bind(
                                TickBar::reserved_space_property(),
                                path()
                                    .named(t.names(), "PART_Track")
                                    .prop(Track::thumb_property())
                                    .prop(Visual::bounds_property())
                                    .binding(),
                            );
                            Ok(())
                        })?
                        .children([t
                            .make(Grid::new())
                            .name("HorizontalTemplate")?
                            .dynamic(Layoutable::min_height_property(), "SliderHorizontalHeight")
                            .with(|e| {
                                Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,Auto,*"))?);
                                Ok(())
                            })?
                            .with(|e| {
                                let items = Grid::row_definitions(e);
                                items.add(
                                    t.make(RowDefinition::new())
                                        .set(RowDefinition::height_property(), literal(GridLength::parse("14"))?)
                                        .end()
                                        .into(),
                                );
                                items.add(
                                    t.make(RowDefinition::new())
                                        .set(RowDefinition::height_property(), GridLength::auto())
                                        .end()
                                        .into(),
                                );
                                items.add(
                                    t.make(RowDefinition::new())
                                        .set(RowDefinition::height_property(), literal(GridLength::parse("14"))?)
                                        .end()
                                        .into(),
                                );
                                Ok(())
                            })?
                            .children([
                                t.make(Rectangle::new())
                                    .name("HorizontalTrackRect")?
                                    .template_bind(Shape::fill_property(), TemplatedControl::background_property())
                                    .dynamic(Layoutable::height_property(), "SliderTrackThemeHeight")
                                    .set(Grid::row_property(), 1)
                                    .set(Grid::column_span_property(), 3)
                                    .template_bind_with(
                                        Rectangle::radius_x_property(),
                                        Some(TemplatedControl::corner_radius_property().property()),
                                        Some(t.scope().converter("TopLeftCornerRadiusDoubleValueConverter")?),
                                        None,
                                        None,
                                    )
                                    .template_bind_with(
                                        Rectangle::radius_y_property(),
                                        Some(TemplatedControl::corner_radius_property().property()),
                                        Some(t.scope().converter("BottomRightCornerRadiusDoubleValueConverter")?),
                                        None,
                                        None,
                                    )
                                    .control(),
                                t.make(TickBar::new())
                                    .name("TopTickBar")?
                                    .set(Visual::is_visible_property(), false)
                                    .dynamic(TickBar::fill_property(), "SliderTickBarFill")
                                    .dynamic(Layoutable::height_property(), "SliderOutsideTickBarThemeHeight")
                                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                                    .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 0.0, 4.0))
                                    .set(Grid::column_span_property(), 3)
                                    .set(TickBar::placement_property(), TickBarPlacement::Top)
                                    .template_bind(TickBar::ticks_property(), Slider::ticks_property())
                                    .template_bind(
                                        TickBar::tick_frequency_property(),
                                        Slider::tick_frequency_property(),
                                    )
                                    .template_bind(TickBar::orientation_property(), Slider::orientation_property())
                                    .template_bind(TickBar::minimum_property(), RangeBase::minimum_property())
                                    .template_bind(TickBar::maximum_property(), RangeBase::maximum_property())
                                    .template_bind(
                                        TickBar::is_direction_reversed_property(),
                                        Slider::is_direction_reversed_property(),
                                    )
                                    .control(),
                                t.make(TickBar::new())
                                    .name("BottomTickBar")?
                                    .set(Visual::is_visible_property(), false)
                                    .dynamic(TickBar::fill_property(), "SliderTickBarFill")
                                    .dynamic(Layoutable::height_property(), "SliderOutsideTickBarThemeHeight")
                                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                    .set(Layoutable::margin_property(), Thickness::new(0.0, 4.0, 0.0, 0.0))
                                    .set(Grid::row_property(), 2)
                                    .set(Grid::column_span_property(), 3)
                                    .set(TickBar::placement_property(), TickBarPlacement::Bottom)
                                    .template_bind(TickBar::ticks_property(), Slider::ticks_property())
                                    .template_bind(
                                        TickBar::tick_frequency_property(),
                                        Slider::tick_frequency_property(),
                                    )
                                    .template_bind(TickBar::orientation_property(), Slider::orientation_property())
                                    .template_bind(TickBar::minimum_property(), RangeBase::minimum_property())
                                    .template_bind(TickBar::maximum_property(), RangeBase::maximum_property())
                                    .template_bind(
                                        TickBar::is_direction_reversed_property(),
                                        Slider::is_direction_reversed_property(),
                                    )
                                    .control(),
                                t.make(Track::new())
                                    .name("PART_Track")?
                                    .set(Grid::row_property(), 0)
                                    .set(Grid::row_span_property(), 3)
                                    .set(Grid::column_span_property(), 3)
                                    .template_bind(Track::minimum_property(), RangeBase::minimum_property())
                                    .template_bind(Track::maximum_property(), RangeBase::maximum_property())
                                    .template_bind_with(
                                        Track::value_property(),
                                        Some(RangeBase::value_property().property()),
                                        None,
                                        Some(BindingMode::TwoWay),
                                        None,
                                    )
                                    .template_bind(
                                        Track::is_direction_reversed_property(),
                                        Slider::is_direction_reversed_property(),
                                    )
                                    .set(Track::orientation_property(), Orientation::Horizontal)
                                    .child(
                                        Track::decrease_button_property(),
                                        t.make(RepeatButton::new())
                                            .name("PART_DecreaseButton")?
                                            .static_resource(
                                                StyledElement::theme_property(),
                                                "SliderHorizontalDecreaseButtonStyle",
                                            )?
                                            .template_bind(
                                                TemplatedControl::background_property(),
                                                TemplatedControl::foreground_property(),
                                            )
                                            .template_bind(
                                                TemplatedControl::corner_radius_property(),
                                                TemplatedControl::corner_radius_property(),
                                            )
                                            .set(InputElement::focusable_property(), false),
                                    )
                                    .child(
                                        Track::increase_button_property(),
                                        t.make(RepeatButton::new())
                                            .name("PART_IncreaseButton")?
                                            .static_resource(
                                                StyledElement::theme_property(),
                                                "SliderIncreaseButtonStyle",
                                            )?
                                            .set(InputElement::focusable_property(), false),
                                    )
                                    .child(
                                        Track::thumb_property(),
                                        t.make(Thumb::new())
                                            .name("HorizontalThumb")?
                                            .static_resource(StyledElement::theme_property(), "SliderThumbStyle")?
                                            .template_bind(
                                                StyledElement::data_context_property(),
                                                RangeBase::value_property(),
                                            )
                                            .dynamic(Layoutable::height_property(), "SliderHorizontalThumbHeight")
                                            .dynamic(Layoutable::width_property(), "SliderHorizontalThumbWidth")
                                            .static_resource(
                                                TemplatedControl::corner_radius_property(),
                                                "ControlCornerRadius",
                                            )?,
                                    )
                                    .control(),
                            ])
                            .control()])
                        .control(),
                ]),
        )?
        .control())
}

/// The template of `Slider` of `default_slider_style`.
fn default_slider_style_template_2(t: &Template<Slider>) -> Result<Ref<Control>> {
    // The errors of FerroUI's data validation under the slider (DataValidationErrors.xaml).
    Ok(t.make(DataValidationErrors::new())
        .content(
            ContentControl::content_property(),
            t.make(Grid::new())
                .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                .children([
                    t.make(Border::new())
                        .name("FocusBorder")?
                        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
                        .set(TemplatedControl::is_template_focus_target_property(), true)
                        .control(),
                    t.make(Grid::new())
                        .name("SliderContainer")?
                        .dynamic(Panel::background_property(), "SliderContainerBackground")
                        .style(of::<TickBar>(), |style| {
                            style.bind(
                                TickBar::reserved_space_property(),
                                path()
                                    .named(t.names(), "PART_Track")
                                    .prop(Track::thumb_property())
                                    .prop(Visual::bounds_property())
                                    .binding(),
                            );
                            Ok(())
                        })?
                        .children([t
                            .make(Grid::new())
                            .name("VerticalTemplate")?
                            .dynamic(Layoutable::min_width_property(), "SliderVerticalWidth")
                            .with(|e| {
                                Grid::set_row_definitions(e, literal(RowDefinitions::parse("*,Auto,Auto"))?);
                                Ok(())
                            })?
                            .with(|e| {
                                let items = Grid::column_definitions(e);
                                items.add(
                                    t.make(ColumnDefinition::new())
                                        .set(ColumnDefinition::width_property(), literal(GridLength::parse("14"))?)
                                        .end()
                                        .into(),
                                );
                                items.add(
                                    t.make(ColumnDefinition::new())
                                        .set(ColumnDefinition::width_property(), GridLength::auto())
                                        .end()
                                        .into(),
                                );
                                items.add(
                                    t.make(ColumnDefinition::new())
                                        .set(ColumnDefinition::width_property(), literal(GridLength::parse("14"))?)
                                        .end()
                                        .into(),
                                );
                                Ok(())
                            })?
                            .children([
                                t.make(Rectangle::new())
                                    .name("VerticalTrackRect")?
                                    .template_bind(Shape::fill_property(), TemplatedControl::background_property())
                                    .dynamic(Layoutable::width_property(), "SliderTrackThemeHeight")
                                    .set(Grid::column_property(), 1)
                                    .set(Grid::row_span_property(), 3)
                                    .template_bind_with(
                                        Rectangle::radius_x_property(),
                                        Some(TemplatedControl::corner_radius_property().property()),
                                        Some(t.scope().converter("TopLeftCornerRadiusDoubleValueConverter")?),
                                        None,
                                        None,
                                    )
                                    .template_bind_with(
                                        Rectangle::radius_y_property(),
                                        Some(TemplatedControl::corner_radius_property().property()),
                                        Some(t.scope().converter("BottomRightCornerRadiusDoubleValueConverter")?),
                                        None,
                                        None,
                                    )
                                    .control(),
                                t.make(TickBar::new())
                                    .name("LeftTickBar")?
                                    .set(Visual::is_visible_property(), false)
                                    .dynamic(TickBar::fill_property(), "SliderTickBarFill")
                                    .dynamic(Layoutable::width_property(), "SliderOutsideTickBarThemeHeight")
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                                    .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 4.0, 0.0))
                                    .set(Grid::row_span_property(), 3)
                                    .set(TickBar::placement_property(), TickBarPlacement::Left)
                                    .template_bind(TickBar::ticks_property(), Slider::ticks_property())
                                    .template_bind(
                                        TickBar::tick_frequency_property(),
                                        Slider::tick_frequency_property(),
                                    )
                                    .template_bind(TickBar::orientation_property(), Slider::orientation_property())
                                    .template_bind(TickBar::minimum_property(), RangeBase::minimum_property())
                                    .template_bind(TickBar::maximum_property(), RangeBase::maximum_property())
                                    .template_bind(
                                        TickBar::is_direction_reversed_property(),
                                        Slider::is_direction_reversed_property(),
                                    )
                                    .control(),
                                t.make(TickBar::new())
                                    .name("RightTickBar")?
                                    .set(Visual::is_visible_property(), false)
                                    .dynamic(TickBar::fill_property(), "SliderTickBarFill")
                                    .dynamic(Layoutable::width_property(), "SliderOutsideTickBarThemeHeight")
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                    .set(Layoutable::margin_property(), Thickness::new(4.0, 0.0, 0.0, 0.0))
                                    .set(Grid::column_property(), 2)
                                    .set(Grid::row_span_property(), 3)
                                    .set(TickBar::placement_property(), TickBarPlacement::Right)
                                    .template_bind(TickBar::ticks_property(), Slider::ticks_property())
                                    .template_bind(
                                        TickBar::tick_frequency_property(),
                                        Slider::tick_frequency_property(),
                                    )
                                    .template_bind(TickBar::orientation_property(), Slider::orientation_property())
                                    .template_bind(TickBar::minimum_property(), RangeBase::minimum_property())
                                    .template_bind(TickBar::maximum_property(), RangeBase::maximum_property())
                                    .template_bind(
                                        TickBar::is_direction_reversed_property(),
                                        Slider::is_direction_reversed_property(),
                                    )
                                    .control(),
                                t.make(Track::new())
                                    .name("PART_Track")?
                                    .set(Grid::column_property(), 0)
                                    .set(Grid::column_span_property(), 3)
                                    .set(Grid::row_span_property(), 3)
                                    .template_bind(Track::minimum_property(), RangeBase::minimum_property())
                                    .template_bind(Track::maximum_property(), RangeBase::maximum_property())
                                    .template_bind_with(
                                        Track::value_property(),
                                        Some(RangeBase::value_property().property()),
                                        None,
                                        Some(BindingMode::TwoWay),
                                        None,
                                    )
                                    .template_bind(
                                        Track::is_direction_reversed_property(),
                                        Slider::is_direction_reversed_property(),
                                    )
                                    .set(Track::orientation_property(), Orientation::Vertical)
                                    .child(
                                        Track::decrease_button_property(),
                                        t.make(RepeatButton::new())
                                            .name("PART_DecreaseButton")?
                                            .static_resource(
                                                StyledElement::theme_property(),
                                                "SliderVerticalDecreaseButtonStyle",
                                            )?
                                            .template_bind(
                                                TemplatedControl::background_property(),
                                                TemplatedControl::foreground_property(),
                                            )
                                            .template_bind(
                                                TemplatedControl::corner_radius_property(),
                                                TemplatedControl::corner_radius_property(),
                                            )
                                            .set(InputElement::focusable_property(), false),
                                    )
                                    .child(
                                        Track::increase_button_property(),
                                        t.make(RepeatButton::new())
                                            .name("PART_IncreaseButton")?
                                            .static_resource(
                                                StyledElement::theme_property(),
                                                "SliderIncreaseButtonStyle",
                                            )?
                                            .set(InputElement::focusable_property(), false),
                                    )
                                    .child(
                                        Track::thumb_property(),
                                        t.make(Thumb::new())
                                            .name("VerticalThumb")?
                                            .static_resource(StyledElement::theme_property(), "SliderThumbStyle")?
                                            .template_bind(
                                                StyledElement::data_context_property(),
                                                RangeBase::value_property(),
                                            )
                                            .dynamic(Layoutable::width_property(), "SliderVerticalThumbWidth")
                                            .dynamic(Layoutable::height_property(), "SliderVerticalThumbHeight")
                                            .static_resource(
                                                TemplatedControl::corner_radius_property(),
                                                "ControlCornerRadius",
                                            )?,
                                    )
                                    .control(),
                            ])
                            .control()])
                        .control(),
                ]),
        )?
        .control())
}

/// `DefaultSliderStyle`: a control theme of `Slider`.
fn default_slider_style(theme: &StyleBuilder) -> Result {
    theme.target::<Slider>();
    theme.dynamic(TemplatedControl::background_property(), "SliderTrackFill");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "SliderBorderThemeThickness",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "SliderThumbBorderBrush");
    theme.dynamic(TemplatedControl::foreground_property(), "SliderTrackValueFill");
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_slider_style_focus_adorner)),
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "SliderTrackCornerRadius");
    // HorizontalTemplate
    theme.style(nesting().class(":horizontal"), |style| {
        style.template(default_slider_style_template);
        Ok(())
    })?;
    // VerticalTemplate
    theme.style(nesting().class(":vertical"), |style| {
        style.template(default_slider_style_template_2);
        Ok(())
    })?;
    // The tick bars the control shows for its TickPlacement.
    theme.style(
        Selectors::or([
            nesting()
                .property_is(Slider::tick_placement_property(), TickPlacement::TopLeft)
                .template()
                .of::<TickBar>()
                .name("TopTickBar"),
            nesting()
                .property_is(Slider::tick_placement_property(), TickPlacement::Outside)
                .template()
                .of::<TickBar>()
                .name("TopTickBar"),
        ]),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .property_is(Slider::tick_placement_property(), TickPlacement::BottomRight)
                .template()
                .of::<TickBar>()
                .name("BottomTickBar"),
            nesting()
                .property_is(Slider::tick_placement_property(), TickPlacement::Outside)
                .template()
                .of::<TickBar>()
                .name("BottomTickBar"),
        ]),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .property_is(Slider::tick_placement_property(), TickPlacement::TopLeft)
                .template()
                .of::<TickBar>()
                .name("LeftTickBar"),
            nesting()
                .property_is(Slider::tick_placement_property(), TickPlacement::Outside)
                .template()
                .of::<TickBar>()
                .name("LeftTickBar"),
        ]),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .property_is(Slider::tick_placement_property(), TickPlacement::BottomRight)
                .template()
                .of::<TickBar>()
                .name("RightTickBar"),
            nesting()
                .property_is(Slider::tick_placement_property(), TickPlacement::Outside)
                .template()
                .of::<TickBar>()
                .name("RightTickBar"),
        ]),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // Normal sets the base values.
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            Selectors::or([
                nesting().template().of::<Rectangle>().name("HorizontalTrackRect"),
                nesting().template().of::<Rectangle>().name("VerticalTrackRect"),
            ]),
            |style| {
                style.dynamic(Shape::fill_property(), "SliderTrackFillPointerOver");
                Ok(())
            },
        )?;
        style.style(
            Selectors::or([
                nesting().template().of::<Thumb>().name("HorizontalThumb"),
                nesting().template().of::<Thumb>().name("VerticalThumb"),
            ]),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "SliderThumbBackgroundPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Grid>().name("SliderContainer"), |style| {
            style.dynamic(Panel::background_property(), "SliderContainerBackgroundPointerOver");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<RepeatButton>().name("PART_DecreaseButton"),
            |style| {
                style.dynamic(
                    TemplatedControl::background_property(),
                    "SliderTrackValueFillPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            Selectors::or([
                nesting().template().of::<Rectangle>().name("HorizontalTrackRect"),
                nesting().template().of::<Rectangle>().name("VerticalTrackRect"),
            ]),
            |style| {
                style.dynamic(Shape::fill_property(), "SliderTrackFillPressed");
                Ok(())
            },
        )?;
        style.style(
            Selectors::or([
                nesting().template().of::<Thumb>().name("HorizontalThumb"),
                nesting().template().of::<Thumb>().name("VerticalThumb"),
            ]),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SliderThumbBackgroundPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Grid>().name("SliderContainer"), |style| {
            style.dynamic(Panel::background_property(), "SliderContainerBackgroundPressed");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<RepeatButton>().name("PART_DecreaseButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SliderTrackValueFillPressed");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting().template().of::<RepeatButton>().name("PART_DecreaseButton"),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SliderTrackValueFillDisabled");
                Ok(())
            },
        )?;
        style.style(
            Selectors::or([
                nesting().template().of::<Rectangle>().name("HorizontalTrackRect"),
                nesting().template().of::<Rectangle>().name("VerticalTrackRect"),
            ]),
            |style| {
                style.dynamic(Shape::fill_property(), "SliderTrackFillDisabled");
                Ok(())
            },
        )?;
        style.style(
            Selectors::or([
                nesting().template().of::<Thumb>().name("HorizontalThumb"),
                nesting().template().of::<Thumb>().name("VerticalThumb"),
            ]),
            |style| {
                style.dynamic(TemplatedControl::background_property(), "SliderThumbBackgroundDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<TickBar>(), |style| {
            style.dynamic(TickBar::fill_property(), "SliderTickBarFillDisabled");
            Ok(())
        })?;
        style.style(nesting().template().of::<Grid>().name("SliderContainer"), |style| {
            style.dynamic(Panel::background_property(), "SliderContainerBackgroundDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    // Not in WinUI (no input validation): the :error of FerroUI's data validation, as the
    // Fluent theme shows it, with the error brush of the WinUI theme.
    theme.style(nesting().class(":error"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "SystemControlErrorTextForegroundBrush",
        );
        style.style(nesting().template().of::<Thumb>(), |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "SystemControlErrorTextForegroundBrush",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `Slider`.
fn slider_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Slider>();
    theme.based_on("DefaultSliderStyle")?;
    Ok(())
}
