//! `Controls/ScrollBar.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/ScrollBar_themeresources.xaml (the styles; its resources are
//!   Resources/ScrollBar_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's ScrollBar arranges its thumb and page buttons with a Track and names its parts:
//!   VerticalSmallDecrease/HorizontalSmallDecrease is PART_LineUpButton, the SmallIncrease buttons
//!   PART_LineDownButton, the LargeDecrease buttons PART_PageUpButton (the decrease button of the
//!   Track) and the LargeIncrease buttons PART_PageDownButton (its increase button). The two roots
//!   of the source, which the control shows by its orientation, are the templates of the
//!   :vertical and :horizontal pseudo-classes, so that each template has its parts once.
//! - The control templates of the resources of the template (RepeatButtonTemplate,
//!   HorizontalIncrementTemplate, ..., VerticalThumbTemplate) are the keyed control themes
//!   ScrollBarRepeatButtonTemplate, ScrollBarHorizontalIncrementTemplate, ...,
//!   ScrollBarVerticalThumbTemplate. Their arrows are glyphs of the symbol font (EDD9, EDDA,
//!   EDDB, EDDC), drawn as GlyphGeometry paths; the pressed arrow is scaled to
//!   ScrollBarButtonArrowScalePressed (0.875).
//! - ConsciousStates: Expanded is FerroUI's IsExpanded, which the control sets when the pointer
//!   has rested over the bar for its ShowDelay and clears when it has been away for its HideDelay:
//!   the setters ShowDelay 0.4 s and HideDelay 0.5 s are the begin times of the source
//!   (ScrollBarExpandBeginTime, ScrollBarContractBeginTime). Collapsed is the thin thumb in
//!   ScrollBarPanningThumbBackground. The opacities animate over ScrollBarOpacityChangeDuration
//!   (83 ms) and the thumb grows over ScrollBarExpandDuration (167 ms, spline 0,0,0,1) as
//!   transitions.
//! - ScrollingIndicatorStates: MouseIndicator is the state of the bar itself (the thumb shown, hit
//!   testing on); NoIndicator, the bars hidden while the pointer is not over their scroll viewer,
//!   is the opacity of the bars in the template of the scroll viewer (Controls/ScrollViewer.xaml),
//!   which sets the indicator mode of its bars in WinUI too. A scroll bar outside a scroll viewer
//!   stays in MouseIndicator (WinUI's IndicatorMode, None by default, has no counterpart).
//!   TouchIndicator and the panning roots (HorizontalPanningRoot, VerticalPanningRoot) have no
//!   counterpart: the collapsed bar of MouseIndicator is the thin indicator for any pointer.
//! - The track rectangles (radius RadiusX/RadiusY of twice the corner radius) are borders with
//!   the corner radius scaled by ScrollBarTrackCornerRadiusConverter.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransformOperationsTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::Stretch;
use ferroui_base::styling::Selectors;
use ferroui_base::RelativePoint;
use ferroui_base::StyledElement;
use ferroui_base::Visual;
use ferroui_controls::converters::CornerRadiusFilterConverter;
use ferroui_controls::converters::Corners;
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::primitives::ScrollBar;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::Thumb;
use ferroui_controls::primitives::Track;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_controls::RepeatButton;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // RadiusX/RadiusY of the track rectangles: the corner radius with
    //       TopLeftCornerRadiusDoubleValueConverter2x and BottomRightCornerRadiusDoubleValueConverter2x.
    resources.deferred("ScrollBarTrackCornerRadiusConverter", |scope| {
        Ok(value({
            let object = ::std::rc::Rc::new(CornerRadiusFilterConverter::new());
            object.set_filter(Corners::TOP_LEFT | Corners::TOP_RIGHT | Corners::BOTTOM_RIGHT | Corners::BOTTOM_LEFT);
            object.set_scale(2.0);
            object
        }))
    });
    // RepeatButtonTemplate: the page buttons.
    resources.control_theme("ScrollBarRepeatButtonTemplate", scroll_bar_repeat_button_template);
    // HorizontalIncrementTemplate
    resources.control_theme(
        "ScrollBarHorizontalIncrementTemplate",
        scroll_bar_horizontal_increment_template,
    );
    // HorizontalDecrementTemplate
    resources.control_theme(
        "ScrollBarHorizontalDecrementTemplate",
        scroll_bar_horizontal_decrement_template,
    );
    // VerticalIncrementTemplate
    resources.control_theme(
        "ScrollBarVerticalIncrementTemplate",
        scroll_bar_vertical_increment_template,
    );
    // VerticalDecrementTemplate
    resources.control_theme(
        "ScrollBarVerticalDecrementTemplate",
        scroll_bar_vertical_decrement_template,
    );
    // VerticalThumbTemplate
    resources.control_theme("ScrollBarVerticalThumbTemplate", scroll_bar_vertical_thumb_template);
    // HorizontalThumbTemplate
    resources.control_theme("ScrollBarHorizontalThumbTemplate", scroll_bar_horizontal_thumb_template);
    resources.control_theme("DefaultScrollBarStyle", default_scroll_bar_style);
    resources.control_theme(type_key::<ScrollBar>(), scroll_bar_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `RepeatButton` of `scroll_bar_repeat_button_template`.
fn scroll_bar_repeat_button_template_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("Root")?
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .control())
}

/// `ScrollBarRepeatButtonTemplate`: a control theme of `RepeatButton`.
fn scroll_bar_repeat_button_template(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.template(scroll_bar_repeat_button_template_template);
    Ok(())
}

/// The template of `RepeatButton` of `scroll_bar_horizontal_increment_template`.
fn scroll_bar_horizontal_increment_template_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .dynamic(Border::background_property(), "ScrollBarButtonBackground")
        .dynamic(Border::border_brush_property(), "ScrollBarButtonBorderBrush")
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .dynamic(Decorator::padding_property(), "ScrollBarHorizontalIncreaseMargin")
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("Arrow")?
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
                )
                .static_resource(Path::data_property(), "GlyphGeometryEDDA")?
                .dynamic(Shape::fill_property(), "ScrollBarButtonArrowForeground")
                .set(Shape::stretch_property(), Stretch::Uniform)
                .static_resource(Layoutable::height_property(), "ScrollBarButtonArrowIconFontSize")?
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
        )
        .control())
}

/// `ScrollBarHorizontalIncrementTemplate`: a control theme of `RepeatButton`.
fn scroll_bar_horizontal_increment_template(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.template(scroll_bar_horizontal_increment_template_template);
    theme.style(
        nesting().class(":pointerover").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundPressed");
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scale(0.875)"))?),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundDisabled");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `RepeatButton` of `scroll_bar_horizontal_decrement_template`.
fn scroll_bar_horizontal_decrement_template_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .dynamic(Border::background_property(), "ScrollBarButtonBackground")
        .dynamic(Border::border_brush_property(), "ScrollBarButtonBorderBrush")
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .dynamic(Decorator::padding_property(), "ScrollBarHorizontalDecreaseMargin")
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("Arrow")?
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
                )
                .static_resource(Path::data_property(), "GlyphGeometryEDD9")?
                .dynamic(Shape::fill_property(), "ScrollBarButtonArrowForeground")
                .set(Shape::stretch_property(), Stretch::Uniform)
                .static_resource(Layoutable::height_property(), "ScrollBarButtonArrowIconFontSize")?
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
        )
        .control())
}

/// `ScrollBarHorizontalDecrementTemplate`: a control theme of `RepeatButton`.
fn scroll_bar_horizontal_decrement_template(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.template(scroll_bar_horizontal_decrement_template_template);
    theme.style(
        nesting().class(":pointerover").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundPressed");
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scale(0.875)"))?),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundDisabled");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `RepeatButton` of `scroll_bar_vertical_increment_template`.
fn scroll_bar_vertical_increment_template_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .dynamic(Border::background_property(), "ScrollBarButtonBackground")
        .dynamic(Border::border_brush_property(), "ScrollBarButtonBorderBrush")
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .dynamic(Decorator::padding_property(), "ScrollBarVerticalIncreaseMargin")
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("Arrow")?
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
                )
                .static_resource(Path::data_property(), "GlyphGeometryEDDC")?
                .dynamic(Shape::fill_property(), "ScrollBarButtonArrowForeground")
                .set(Shape::stretch_property(), Stretch::Uniform)
                .static_resource(Layoutable::width_property(), "ScrollBarButtonArrowIconFontSize")?
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
        )
        .control())
}

/// `ScrollBarVerticalIncrementTemplate`: a control theme of `RepeatButton`.
fn scroll_bar_vertical_increment_template(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.template(scroll_bar_vertical_increment_template_template);
    theme.style(
        nesting().class(":pointerover").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundPressed");
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scale(0.875)"))?),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundDisabled");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `RepeatButton` of `scroll_bar_vertical_decrement_template`.
fn scroll_bar_vertical_decrement_template_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .dynamic(Border::background_property(), "ScrollBarButtonBackground")
        .dynamic(Border::border_brush_property(), "ScrollBarButtonBorderBrush")
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .dynamic(Decorator::padding_property(), "ScrollBarVerticalDecreaseMargin")
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("Arrow")?
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
                )
                .static_resource(Path::data_property(), "GlyphGeometryEDDB")?
                .dynamic(Shape::fill_property(), "ScrollBarButtonArrowForeground")
                .set(Shape::stretch_property(), Stretch::Uniform)
                .static_resource(Layoutable::width_property(), "ScrollBarButtonArrowIconFontSize")?
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
        )
        .control())
}

/// `ScrollBarVerticalDecrementTemplate`: a control theme of `RepeatButton`.
fn scroll_bar_vertical_decrement_template(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.template(scroll_bar_vertical_decrement_template_template);
    theme.style(
        nesting().class(":pointerover").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundPressed");
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scale(0.875)"))?),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Path>().name("Arrow"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarButtonArrowForegroundDisabled");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Thumb` of `scroll_bar_vertical_thumb_template`.
fn scroll_bar_vertical_thumb_template_template(t: &Template<Thumb>) -> Result<Ref<Control>> {
    Ok(t.make(Rectangle::new())
        .name("ThumbVisual")?
        .template_bind(Shape::fill_property(), TemplatedControl::background_property())
        .template_bind(Shape::stroke_property(), TemplatedControl::border_brush_property())
        .dynamic(Shape::stroke_thickness_property(), "ScrollBarThumbStrokeThickness")
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
        .set(
            Animatable::transitions_property(),
            Some({
                let object = Transitions::new();
                object.add(
                    t.make(DoubleTransition::new())
                        .set(
                            TransitionBase::property_property(),
                            Some(Visual::opacity_property().property()),
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
        .control())
}

/// `ScrollBarVerticalThumbTemplate`: a control theme of `Thumb`.
fn scroll_bar_vertical_thumb_template(theme: &StyleBuilder) -> Result {
    theme.target::<Thumb>();
    theme.template(scroll_bar_vertical_thumb_template_template);
    // Disabled: the fill after ScrollBarColorChangeDuration, the opacity over ScrollBarOpacityChangeDuration.
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<Rectangle>()
            .name("ThumbVisual"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarThumbFillDisabled");
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Thumb` of `scroll_bar_horizontal_thumb_template`.
fn scroll_bar_horizontal_thumb_template_template(t: &Template<Thumb>) -> Result<Ref<Control>> {
    Ok(t.make(Rectangle::new())
        .name("ThumbVisual")?
        .template_bind(Shape::fill_property(), TemplatedControl::background_property())
        .template_bind(Shape::stroke_property(), TemplatedControl::border_brush_property())
        .dynamic(Shape::stroke_thickness_property(), "ScrollBarThumbStrokeThickness")
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
        .set(
            Animatable::transitions_property(),
            Some({
                let object = Transitions::new();
                object.add(
                    t.make(DoubleTransition::new())
                        .set(
                            TransitionBase::property_property(),
                            Some(Visual::opacity_property().property()),
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
        .control())
}

/// `ScrollBarHorizontalThumbTemplate`: a control theme of `Thumb`.
fn scroll_bar_horizontal_thumb_template(theme: &StyleBuilder) -> Result {
    theme.target::<Thumb>();
    theme.template(scroll_bar_horizontal_thumb_template_template);
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<Rectangle>()
            .name("ThumbVisual"),
        |style| {
            style.dynamic(Shape::fill_property(), "ScrollBarThumbFillDisabled");
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `ScrollBar` of `default_scroll_bar_style`.
fn default_scroll_bar_style_template(t: &Template<ScrollBar>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .name("VerticalRoot")?
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                .child(
                    Decorator::child_property(),
                    t.make(Grid::new())
                        .with(|e| {
                            Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,*,Auto"))?);
                            Ok(())
                        })?
                        .children([
                            t.make(Border::new())
                                .name("VerticalTrackRect")?
                                .set(Grid::row_span_property(), 3)
                                .set(Visual::opacity_property(), 0.0)
                                .template_bind_with(
                                    Border::corner_radius_property(),
                                    Some(TemplatedControl::corner_radius_property().property()),
                                    Some(t.scope().converter("ScrollBarTrackCornerRadiusConverter")?),
                                    None,
                                    None,
                                )
                                .dynamic(
                                    Border::border_thickness_property(),
                                    "ScrollBarTrackBorderThemeThickness",
                                )
                                .dynamic(Border::background_property(), "ScrollBarTrackFill")
                                .dynamic(Border::border_brush_property(), "ScrollBarTrackStroke")
                                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Visual::opacity_property().property()),
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
                                .control(),
                            t.make(RepeatButton::new())
                                .name("PART_LineUpButton")?
                                .set(Grid::row_property(), 0)
                                .set(Visual::opacity_property(), 0.0)
                                .static_resource(Layoutable::height_property(), "ScrollBarSize")?
                                .static_resource(Layoutable::min_width_property(), "ScrollBarSize")?
                                .set(InputElement::focusable_property(), false)
                                .set(RepeatButton::interval_property(), 50)
                                .dynamic(TemplatedControl::padding_property(), "ScrollBarVerticalDecreaseMargin")
                                .static_resource(StyledElement::theme_property(), "ScrollBarVerticalDecrementTemplate")?
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Visual::opacity_property().property()),
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
                                .control(),
                            t.make(Track::new())
                                .set(Grid::row_property(), 1)
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
                                    Track::defer_thumb_drag_property(),
                                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                                )
                                .template_bind(Track::viewport_size_property(), ScrollBar::viewport_size_property())
                                .template_bind(Track::orientation_property(), ScrollBar::orientation_property())
                                .set(Track::is_direction_reversed_property(), true)
                                .child(
                                    Track::decrease_button_property(),
                                    t.make(RepeatButton::new())
                                        .name("PART_PageUpButton")?
                                        .set(InputElement::focusable_property(), false)
                                        .set(RepeatButton::interval_property(), 50)
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "ScrollBarRepeatButtonTemplate",
                                        )?,
                                )
                                .child(
                                    Track::increase_button_property(),
                                    t.make(RepeatButton::new())
                                        .name("PART_PageDownButton")?
                                        .set(InputElement::focusable_property(), false)
                                        .set(RepeatButton::interval_property(), 50)
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "ScrollBarRepeatButtonTemplate",
                                        )?,
                                )
                                .child(
                                    Track::thumb_property(),
                                    t.make(Thumb::new())
                                        .name("VerticalThumb")?
                                        .dynamic(
                                            TemplatedControl::background_property(),
                                            "ScrollBarPanningThumbBackground",
                                        )
                                        .dynamic(TemplatedControl::border_brush_property(), "ScrollBarThumbBorderBrush")
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "ScrollBarVerticalThumbTemplate",
                                        )?
                                        .dynamic(Layoutable::width_property(), "ScrollBarVerticalThumbMinWidth")
                                        .static_resource(
                                            Layoutable::min_height_property(),
                                            "ScrollBarVerticalThumbMinHeight",
                                        )?
                                        .set(
                                            Visual::render_transform_origin_property(),
                                            literal(RelativePoint::parse("100%,50%"))?,
                                        )
                                        .set(
                                            Visual::render_transform_property(),
                                            Some(literal(TransformOperations::parse("translateX(2px)"))?),
                                        )
                                        .template_bind(
                                            TemplatedControl::corner_radius_property(),
                                            TemplatedControl::corner_radius_property(),
                                        )
                                        // translateX(2px) is the TranslateTransform X="{StaticResource ScrollBarThumbOffset}" (2).
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
                                                    t.make(TransformOperationsTransition::new())
                                                        .set(
                                                            TransitionBase::property_property(),
                                                            Some(Visual::render_transform_property().property()),
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
                                        ),
                                )
                                .control(),
                            t.make(RepeatButton::new())
                                .name("PART_LineDownButton")?
                                .set(Grid::row_property(), 2)
                                .set(Visual::opacity_property(), 0.0)
                                .static_resource(Layoutable::height_property(), "ScrollBarSize")?
                                .static_resource(Layoutable::min_width_property(), "ScrollBarSize")?
                                .set(InputElement::focusable_property(), false)
                                .set(RepeatButton::interval_property(), 50)
                                .dynamic(TemplatedControl::padding_property(), "ScrollBarVerticalIncreaseMargin")
                                .static_resource(StyledElement::theme_property(), "ScrollBarVerticalIncrementTemplate")?
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Visual::opacity_property().property()),
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
                                .control(),
                        ]),
                ),
        )
        .control())
}

/// The template of `ScrollBar` of `default_scroll_bar_style`.
fn default_scroll_bar_style_template_2(t: &Template<ScrollBar>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .name("HorizontalRoot")?
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .child(
                    Decorator::child_property(),
                    t.make(Grid::new())
                        .with(|e| {
                            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*,Auto"))?);
                            Ok(())
                        })?
                        .children([
                            t.make(Border::new())
                                .name("HorizontalTrackRect")?
                                .set(Grid::column_span_property(), 3)
                                .set(Visual::opacity_property(), 0.0)
                                .template_bind_with(
                                    Border::corner_radius_property(),
                                    Some(TemplatedControl::corner_radius_property().property()),
                                    Some(t.scope().converter("ScrollBarTrackCornerRadiusConverter")?),
                                    None,
                                    None,
                                )
                                .dynamic(
                                    Border::border_thickness_property(),
                                    "ScrollBarTrackBorderThemeThickness",
                                )
                                .dynamic(Border::background_property(), "ScrollBarTrackFill")
                                .dynamic(Border::border_brush_property(), "ScrollBarTrackStroke")
                                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Visual::opacity_property().property()),
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
                                .control(),
                            t.make(RepeatButton::new())
                                .name("PART_LineUpButton")?
                                .set(Grid::column_property(), 0)
                                .set(Visual::opacity_property(), 0.0)
                                .static_resource(Layoutable::min_height_property(), "ScrollBarSize")?
                                .static_resource(Layoutable::width_property(), "ScrollBarSize")?
                                .set(InputElement::focusable_property(), false)
                                .set(RepeatButton::interval_property(), 50)
                                .dynamic(
                                    TemplatedControl::padding_property(),
                                    "ScrollBarHorizontalDecreaseMargin",
                                )
                                .static_resource(
                                    StyledElement::theme_property(),
                                    "ScrollBarHorizontalDecrementTemplate",
                                )?
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Visual::opacity_property().property()),
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
                                .control(),
                            t.make(Track::new())
                                .set(Grid::column_property(), 1)
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
                                    Track::defer_thumb_drag_property(),
                                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                                )
                                .template_bind(Track::viewport_size_property(), ScrollBar::viewport_size_property())
                                .template_bind(Track::orientation_property(), ScrollBar::orientation_property())
                                .child(
                                    Track::decrease_button_property(),
                                    t.make(RepeatButton::new())
                                        .name("PART_PageUpButton")?
                                        .set(InputElement::focusable_property(), false)
                                        .set(RepeatButton::interval_property(), 50)
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "ScrollBarRepeatButtonTemplate",
                                        )?,
                                )
                                .child(
                                    Track::increase_button_property(),
                                    t.make(RepeatButton::new())
                                        .name("PART_PageDownButton")?
                                        .set(InputElement::focusable_property(), false)
                                        .set(RepeatButton::interval_property(), 50)
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "ScrollBarRepeatButtonTemplate",
                                        )?,
                                )
                                .child(
                                    Track::thumb_property(),
                                    t.make(Thumb::new())
                                        .name("HorizontalThumb")?
                                        .dynamic(
                                            TemplatedControl::background_property(),
                                            "ScrollBarPanningThumbBackground",
                                        )
                                        .dynamic(TemplatedControl::border_brush_property(), "ScrollBarThumbBorderBrush")
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "ScrollBarHorizontalThumbTemplate",
                                        )?
                                        .dynamic(Layoutable::height_property(), "ScrollBarHorizontalThumbMinHeight")
                                        .static_resource(
                                            Layoutable::min_width_property(),
                                            "ScrollBarHorizontalThumbMinWidth",
                                        )?
                                        .set(
                                            Visual::render_transform_origin_property(),
                                            literal(RelativePoint::parse("50%,100%"))?,
                                        )
                                        .set(
                                            Visual::render_transform_property(),
                                            Some(literal(TransformOperations::parse("translateY(2px)"))?),
                                        )
                                        .template_bind(
                                            TemplatedControl::corner_radius_property(),
                                            TemplatedControl::corner_radius_property(),
                                        )
                                        // translateY(2px) is the TranslateTransform Y="{StaticResource ScrollBarThumbOffset}" (2).
                                        .set(
                                            Animatable::transitions_property(),
                                            Some({
                                                let object = Transitions::new();
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
                                                object.add(
                                                    t.make(TransformOperationsTransition::new())
                                                        .set(
                                                            TransitionBase::property_property(),
                                                            Some(Visual::render_transform_property().property()),
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
                                        ),
                                )
                                .control(),
                            t.make(RepeatButton::new())
                                .name("PART_LineDownButton")?
                                .set(Grid::column_property(), 2)
                                .set(Visual::opacity_property(), 0.0)
                                .static_resource(Layoutable::min_height_property(), "ScrollBarSize")?
                                .static_resource(Layoutable::width_property(), "ScrollBarSize")?
                                .set(InputElement::focusable_property(), false)
                                .set(RepeatButton::interval_property(), 50)
                                .dynamic(
                                    TemplatedControl::padding_property(),
                                    "ScrollBarHorizontalIncreaseMargin",
                                )
                                .static_resource(
                                    StyledElement::theme_property(),
                                    "ScrollBarHorizontalIncrementTemplate",
                                )?
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Visual::opacity_property().property()),
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
                                .control(),
                        ]),
                ),
        )
        .control())
}

/// `DefaultScrollBarStyle`: a control theme of `ScrollBar`.
fn default_scroll_bar_style(theme: &StyleBuilder) -> Result {
    theme.target::<ScrollBar>();
    theme.static_resource(Layoutable::min_width_property(), "ScrollBarSize")?;
    theme.static_resource(Layoutable::min_height_property(), "ScrollBarSize")?;
    theme.dynamic(TemplatedControl::background_property(), "ScrollBarBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "ScrollBarForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "ScrollBarBorderBrush");
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(TemplatedControl::corner_radius_property(), "ScrollBarCornerRadius");
    // ScrollBarExpandBeginTime (00:00:00.40) and ScrollBarContractBeginTime (00:00:00.50).
    theme.set(ScrollBar::show_delay_property(), std::time::Duration::from_millis(400));
    theme.set(ScrollBar::hide_delay_property(), std::time::Duration::from_millis(500));
    theme.style(nesting().class(":vertical"), |style| {
        style.template(default_scroll_bar_style_template);
        Ok(())
    })?;
    theme.style(nesting().class(":horizontal"), |style| {
        style.template(default_scroll_bar_style_template_2);
        Ok(())
    })?;
    // Expanded (ConsciousStates)
    theme.style(
        nesting().property_is(ScrollBar::is_expanded_property(), true),
        |style| {
            style.style(nesting().template().of::<Border>().name("Root"), |style| {
                style.dynamic(Border::background_property(), "ScrollBarBackgroundPointerOver");
                style.dynamic(Border::border_brush_property(), "ScrollBarBorderBrushPointerOver");
                Ok(())
            })?;
            style.style(
                Selectors::or([
                    nesting().template().of::<Border>().name("VerticalTrackRect"),
                    nesting().template().of::<Border>().name("HorizontalTrackRect"),
                ]),
                |style| {
                    style.dynamic(Border::border_brush_property(), "ScrollBarTrackStrokePointerOver");
                    style.dynamic(Border::background_property(), "ScrollBarTrackFillPointerOver");
                    style.set(Visual::opacity_property(), 1.0);
                    Ok(())
                },
            )?;
            style.style(
                Selectors::or([
                    nesting().template().of::<RepeatButton>().name("PART_LineUpButton"),
                    nesting().template().of::<RepeatButton>().name("PART_LineDownButton"),
                ]),
                |style| {
                    style.set(Visual::opacity_property(), 1.0);
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Thumb>().name("VerticalThumb"), |style| {
                style.dynamic(TemplatedControl::background_property(), "ScrollBarThumbBackground");
                style.static_resource(Layoutable::width_property(), "ScrollBarSize")?;
                style.set(
                    Visual::render_transform_property(),
                    Some(literal(TransformOperations::parse("none"))?),
                );
                Ok(())
            })?;
            style.style(nesting().template().of::<Thumb>().name("HorizontalThumb"), |style| {
                style.dynamic(TemplatedControl::background_property(), "ScrollBarThumbBackground");
                style.static_resource(Layoutable::height_property(), "ScrollBarSize")?;
                style.set(
                    Visual::render_transform_property(),
                    Some(literal(TransformOperations::parse("none"))?),
                );
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // Disabled (CommonStates); the panning thumbs of the source are not part of the template.
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("Root"), |style| {
            style.dynamic(Border::background_property(), "ScrollBarBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "ScrollBarBorderBrushDisabled");
            style.set(Visual::opacity_property(), 0.5);
            Ok(())
        })?;
        style.style(
            Selectors::or([
                nesting().template().of::<Border>().name("VerticalTrackRect"),
                nesting().template().of::<Border>().name("HorizontalTrackRect"),
            ]),
            |style| {
                style.dynamic(Border::border_brush_property(), "ScrollBarTrackStrokeDisabled");
                style.dynamic(Border::background_property(), "ScrollBarTrackFillDisabled");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `ScrollBar`.
fn scroll_bar_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ScrollBar>();
    theme.based_on("DefaultScrollBarStyle")?;
    Ok(())
}
