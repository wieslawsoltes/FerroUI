//! `Controls/Expander.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/Expander/Expander.xaml (DefaultExpanderStyle and the implicit style)
//!   controls/dev/Expander/Expander_themeresources.xaml (ExpanderHeaderDownStyle,
//!   ExpanderHeaderUpStyle; its resources are Resources/Expander_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The header: the ToggleButton ExpanderHeader with the keyed themes ExpanderHeaderDownStyle and
//!   ExpanderHeaderUpStyle. Their Grid ToggleButtonGrid (background, border, padding) is a Border of
//!   the same name around the Grid; ContentPresenter is PART_ContentPresenter. These themes are not
//!   based on DefaultToggleButtonStyle, whose visual state styles would set the background and the
//!   border of the content presenter of their templates; the setters of the default style they do
//!   not override (the font, the vertical alignment, the focus visual) are written out.
//! - The chevron is an AnimatedIcon (AnimatedChevronUpDownSmallVisualSource, 12 x 12) with the
//!   fallback glyph ExpanderChevronDownGlyph (E70D, ExpanderChevronUpGlyph E70E in the up style)
//!   of the symbol font at 12 px: a Path of GlyphGeometryE70D (E70E), 10 px wide. The animated icon
//!   shows the chevron turned over in the checked states (NormalOn, PointerOverOn, PressedOn of the
//!   down style): the path is rotated by 180 degrees in :checked; the animation of the turn is not
//!   ported. The Normal state of the header sets the foreground of the content presenter: it is
//!   its base value. The Checked states set the values of the unchecked states (and the icon
//!   state); the Indeterminate states set nothing.
//! - The expander: the states of ExpandDirectionStates (Down, Up) are the pseudo-classes :down and
//!   :up; FerroUI's expander also expands to the left and the right (:left, :right), which WinUI
//!   does not: the header is then docked at the side with the keyed themes
//!   ExpanderHeaderLeftStyle and ExpanderHeaderRightStyle (the down style with the chevron turned to
//!   the side), and the borders and corner radii of the side are filtered as for up and down. The
//!   rows Row0/Row1 of the template, which the Up state swaps, are a DockPanel whose header is
//!   docked by the direction.
//! - ExpandStates (ExpandDown, ExpandUp, CollapseDown, CollapseUp) are :expanded with the direction.
//!   The content slides in from behind the header (TranslateY from the content height to 0 over
//!   333 ms with the spline 0,0,0,1) and out (167 ms, spline 1,1,0,1), clipped by
//!   ExpanderContentClip: FerroUI's expander has no content height in its template settings and
//!   animates its content only through the ContentTransition an application sets, so the content
//!   is shown and hidden at once (IsVisible of ExpanderContent follows IsExpanded); the clip border
//!   is kept (ClipToBounds) for a content transition.
//! - ContentTemplateSelector and HeaderTemplateSelector have no counterpart (FerroUI's data
//!   templates select themselves).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::BindingMode;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::FontWeight;
use ferroui_base::media::Stretch;
use ferroui_base::RelativePoint;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::HeaderedContentControl;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_controls::Expander;
use ferroui_controls::Grid;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("ExpanderHeaderDownStyle", expander_header_down_style);
    resources.control_theme("ExpanderHeaderUpStyle", expander_header_up_style);
    // No WinUI counterpart: the header of an expander that expands to the left (the chevron of
    //       the down style turned to point left, and right when expanded).
    resources.control_theme("ExpanderHeaderLeftStyle", expander_header_left_style);
    // No WinUI counterpart: the header of an expander that expands to the right.
    resources.control_theme("ExpanderHeaderRightStyle", expander_header_right_style);
    resources.control_theme("DefaultExpanderStyle", default_expander_style);
    resources.control_theme(type_key::<Expander>(), expander_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The focus adorner of `expander_header_down_style`.
fn expander_header_down_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `ToggleButton` of `expander_header_down_style`.
fn expander_header_down_style_template(t: &Template<ToggleButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("ToggleButtonGrid")?
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(
            Border::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
        .template_bind(Layoutable::min_width_property(), Layoutable::min_width_property())
        .template_bind(Layoutable::max_width_property(), Layoutable::max_width_property())
        .template_bind(
            Layoutable::horizontal_alignment_property(),
            Layoutable::horizontal_alignment_property(),
        )
        .dynamic(Border::border_brush_property(), "ExpanderHeaderBorderBrush")
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
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
                        .dynamic(ContentPresenter::foreground_property(), "ExpanderHeaderForeground")
                        .set(ContentPresenter::recognizes_access_key_property(), true)
                        .control(),
                    t.make(Border::new())
                        .name("ExpandCollapseChevronBorder")?
                        .set(Grid::column_property(), 1)
                        .dynamic(Layoutable::width_property(), "ExpanderChevronButtonSize")
                        .dynamic(Layoutable::height_property(), "ExpanderChevronButtonSize")
                        .dynamic(Layoutable::margin_property(), "ExpanderChevronMargin")
                        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
                        .dynamic(Border::border_brush_property(), "ExpanderChevronBorderBrush")
                        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                        .dynamic(Border::border_thickness_property(), "ExpanderChevronBorderThickness")
                        .dynamic(Border::background_property(), "ExpanderChevronBackground")
                        // The AnimatedIcon (ExpanderChevronGlyphSize) with the fallback glyph E70D at 12 px.
                        .child(
                            Decorator::child_property(),
                            t.make(Grid::new())
                                .static_resource(Layoutable::width_property(), "ExpanderChevronGlyphSize")?
                                .static_resource(Layoutable::height_property(), "ExpanderChevronGlyphSize")?
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .children([t
                                    .make(Path::new())
                                    .name("ExpandCollapseChevron")?
                                    .static_resource(Path::data_property(), "GlyphGeometryE70D")?
                                    .dynamic(Shape::fill_property(), "ExpanderChevronForeground")
                                    .set(Shape::stretch_property(), Stretch::Uniform)
                                    .set(Layoutable::width_property(), 10.0)
                                    .set(
                                        Visual::render_transform_origin_property(),
                                        literal(RelativePoint::parse("50%,50%"))?,
                                    )
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                    .control()]),
                        )
                        .control(),
                ]),
        )
        .control())
}

/// `ExpanderHeaderDownStyle`: a control theme of `ToggleButton`.
fn expander_header_down_style(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    //         (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    // The setters of DefaultToggleButtonStyle this style does not override.
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), expander_header_down_style_focus_adorner)),
    );
    // The setters of the style.
    theme.static_resource(TemplatedControl::padding_property(), "ExpanderHeaderPadding")?;
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.template(expander_header_down_style_template);
    // PointerOver (and CheckedPointerOver)
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ExpanderHeaderForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("ToggleButtonGrid"), |style| {
            style.dynamic(Border::border_brush_property(), "ExpanderHeaderBorderPointerOverBrush");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Path>().name("ExpandCollapseChevron"),
            |style| {
                style.dynamic(Shape::fill_property(), "ExpanderChevronPointerOverForeground");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Border>().name("ExpandCollapseChevronBorder"),
            |style| {
                style.dynamic(Border::border_brush_property(), "ExpanderChevronBorderPointerOverBrush");
                style.dynamic(Border::background_property(), "ExpanderChevronPointerOverBackground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed (and CheckedPressed)
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ExpanderHeaderForegroundPressed",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("ToggleButtonGrid"), |style| {
            style.dynamic(Border::border_brush_property(), "ExpanderHeaderBorderPressedBrush");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Path>().name("ExpandCollapseChevron"),
            |style| {
                style.dynamic(Shape::fill_property(), "ExpanderChevronPressedForeground");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Border>().name("ExpandCollapseChevronBorder"),
            |style| {
                style.dynamic(Border::border_brush_property(), "ExpanderChevronBorderPressedBrush");
                style.dynamic(Border::background_property(), "ExpanderChevronPressedBackground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled (and CheckedDisabled)
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ExpanderHeaderDisabledForeground",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("ToggleButtonGrid"), |style| {
            style.dynamic(Border::border_brush_property(), "ExpanderHeaderDisabledBorderBrush");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Path>().name("ExpandCollapseChevron"),
            |style| {
                style.dynamic(Shape::fill_property(), "ExpanderHeaderDisabledForeground");
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Border>().name("ExpandCollapseChevronBorder"),
            |style| {
                style.dynamic(Border::border_brush_property(), "ExpanderHeaderDisabledBorderBrush");
                style.dynamic(Border::background_property(), "ExpanderChevronBackground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Checked: the AnimatedIcon.State NormalOn, PointerOverOn and PressedOn (the chevron turned over).
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Path>()
            .name("ExpandCollapseChevron"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("rotate(180deg)"))?),
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `ToggleButton` of `expander_header_up_style`.
fn expander_header_up_style_template(t: &Template<ToggleButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("ToggleButtonGrid")?
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(
            Border::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
        .template_bind(Layoutable::min_width_property(), Layoutable::min_width_property())
        .template_bind(Layoutable::max_width_property(), Layoutable::max_width_property())
        .template_bind(
            Layoutable::horizontal_alignment_property(),
            Layoutable::horizontal_alignment_property(),
        )
        .dynamic(Border::border_brush_property(), "ExpanderHeaderBorderBrush")
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
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
                        .dynamic(ContentPresenter::foreground_property(), "ExpanderHeaderForeground")
                        .set(ContentPresenter::recognizes_access_key_property(), true)
                        .control(),
                    t.make(Border::new())
                        .name("ExpandCollapseChevronBorder")?
                        .set(Grid::column_property(), 1)
                        .dynamic(Layoutable::width_property(), "ExpanderChevronButtonSize")
                        .dynamic(Layoutable::height_property(), "ExpanderChevronButtonSize")
                        .dynamic(Layoutable::margin_property(), "ExpanderChevronMargin")
                        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
                        .dynamic(Border::border_brush_property(), "ExpanderChevronBorderBrush")
                        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                        .dynamic(Border::border_thickness_property(), "ExpanderChevronBorderThickness")
                        .dynamic(Border::background_property(), "ExpanderChevronBackground")
                        // The AnimatedIcon (ExpanderChevronGlyphSize) with the fallback glyph E70E at 12 px.
                        .child(
                            Decorator::child_property(),
                            t.make(Grid::new())
                                .static_resource(Layoutable::width_property(), "ExpanderChevronGlyphSize")?
                                .static_resource(Layoutable::height_property(), "ExpanderChevronGlyphSize")?
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .children([t
                                    .make(Path::new())
                                    .name("ExpandCollapseChevron")?
                                    .static_resource(Path::data_property(), "GlyphGeometryE70E")?
                                    .dynamic(Shape::fill_property(), "ExpanderChevronForeground")
                                    .set(Shape::stretch_property(), Stretch::Uniform)
                                    .set(Layoutable::width_property(), 10.0)
                                    .set(
                                        Visual::render_transform_origin_property(),
                                        literal(RelativePoint::parse("50%,50%"))?,
                                    )
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                    .control()]),
                        )
                        .control(),
                ]),
        )
        .control())
}

/// `ExpanderHeaderUpStyle`: a control theme of `ToggleButton`.
fn expander_header_up_style(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
    theme.based_on("ExpanderHeaderDownStyle")?;
    theme.static_resource(TemplatedControl::padding_property(), "ExpanderHeaderPadding")?;
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    // The template of the down style with the glyph ExpanderChevronUpGlyph (E70E); its visual
    //         states set the same values as the ones of the down style, which this theme is based on.
    theme.template(expander_header_up_style_template);
    Ok(())
}

/// `ExpanderHeaderLeftStyle`: a control theme of `ToggleButton`.
fn expander_header_left_style(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
    theme.based_on("ExpanderHeaderDownStyle")?;
    theme.style(
        nesting().template().of::<Path>().name("ExpandCollapseChevron"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("rotate(90deg)"))?),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Path>()
            .name("ExpandCollapseChevron"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("rotate(-90deg)"))?),
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// `ExpanderHeaderRightStyle`: a control theme of `ToggleButton`.
fn expander_header_right_style(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
    theme.based_on("ExpanderHeaderDownStyle")?;
    theme.style(
        nesting().template().of::<Path>().name("ExpandCollapseChevron"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("rotate(-90deg)"))?),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<Path>()
            .name("ExpandCollapseChevron"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("rotate(90deg)"))?),
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Expander` of `default_expander_style`.
fn default_expander_style_template(t: &Template<Expander>) -> Result<Ref<Control>> {
    Ok(t.make(DockPanel::new())
        .template_bind(Layoutable::min_width_property(), Layoutable::min_width_property())
        .template_bind(Layoutable::max_width_property(), Layoutable::max_width_property())
        // The clip is a composition clip applied in code in WinUI.
        .children([
            t.make(ToggleButton::new())
                .name("ExpanderHeader")?
                .set(DockPanel::dock_property(), Dock::Top)
                .dynamic(TemplatedControl::background_property(), "ExpanderHeaderBackground")
                .template_bind(
                    TemplatedControl::background_sizing_property(),
                    TemplatedControl::background_sizing_property(),
                )
                .dynamic(TemplatedControl::border_brush_property(), "ExpanderHeaderBorderBrush")
                .dynamic(
                    TemplatedControl::border_thickness_property(),
                    "ExpanderHeaderBorderThickness",
                )
                .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
                .template_bind(
                    TemplatedControl::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(InputElement::is_enabled_property(), InputElement::is_enabled_property())
                .static_resource(TemplatedControl::padding_property(), "ExpanderHeaderPadding")?
                .static_resource(StyledElement::theme_property(), "ExpanderHeaderDownStyle")?
                .set(
                    Layoutable::horizontal_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .static_resource(
                    ContentControl::horizontal_content_alignment_property(),
                    "ExpanderHeaderHorizontalContentAlignment",
                )?
                .static_resource(
                    ContentControl::vertical_content_alignment_property(),
                    "ExpanderHeaderVerticalContentAlignment",
                )?
                .template_bind(
                    ContentControl::content_property(),
                    HeaderedContentControl::header_property(),
                )
                .template_bind(
                    ContentControl::content_template_property(),
                    HeaderedContentControl::header_template_property(),
                )
                .template_bind_with(
                    ToggleButton::is_checked_property(),
                    Some(Expander::is_expanded_property().property()),
                    None,
                    Some(BindingMode::TwoWay),
                    None,
                )
                .control(),
            t.make(Border::new())
                .name("ExpanderContentClip")?
                .set(Visual::clip_to_bounds_property(), true)
                .child(
                    Decorator::child_property(),
                    t.make(Border::new())
                        .name("ExpanderContent")?
                        .template_bind(Visual::is_visible_property(), Expander::is_expanded_property())
                        .template_bind(Border::background_property(), TemplatedControl::background_property())
                        .template_bind(
                            Border::background_sizing_property(),
                            TemplatedControl::background_sizing_property(),
                        )
                        .template_bind_with(
                            Border::corner_radius_property(),
                            Some(TemplatedControl::corner_radius_property().property()),
                            Some(t.scope().converter("BottomCornerRadiusFilterConverter")?),
                            None,
                            None,
                        )
                        .template_bind(
                            Border::border_brush_property(),
                            TemplatedControl::border_brush_property(),
                        )
                        .static_resource(
                            Border::border_thickness_property(),
                            "ExpanderContentDownBorderThickness",
                        )?
                        .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
                        .child(
                            Decorator::child_property(),
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
                                ),
                        ),
                )
                .control(),
        ])
        .control())
}

/// `DefaultExpanderStyle`: a control theme of `Expander`.
fn default_expander_style(theme: &StyleBuilder) -> Result {
    theme.target::<Expander>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(TemplatedControl::background_property(), "ExpanderContentBackground");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.dynamic(Layoutable::min_width_property(), "FlyoutThemeMinWidth");
    theme.static_resource(Layoutable::min_height_property(), "ExpanderMinHeight")?;
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ExpanderContentDownBorderThickness",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "ExpanderContentBorderBrush");
    theme.static_resource(TemplatedControl::padding_property(), "ExpanderContentPadding")?;
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    // Not in the style: the defaults of a WinUI control (Control.HorizontalContentAlignment and
    //         VerticalContentAlignment are Center), which the content presenter of ExpanderContent
    //         binds; FerroUI's defaults differ.
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(default_expander_style_template);
    // ExpandDirectionStates: Down sets the base values.
    // Up
    theme.style(nesting().class(":up"), |style| {
        style.style(
            nesting().template().of::<ToggleButton>().name("ExpanderHeader"),
            |style| {
                style.static_resource(StyledElement::theme_property(), "ExpanderHeaderUpStyle")?;
                style.set(DockPanel::dock_property(), Dock::Bottom);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("ExpanderContent"), |style| {
            style.static_resource(Border::border_thickness_property(), "ExpanderContentUpBorderThickness")?;
            style.bind(
                Border::corner_radius_property(),
                TemplateBinding::new(TemplatedControl::corner_radius_property().property())
                    .with_converter_value(Some(style.scope().converter("TopCornerRadiusFilterConverter")?)),
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // No WinUI counterpart: expanding to the left.
    theme.style(nesting().class(":left"), |style| {
        style.style(
            nesting().template().of::<ToggleButton>().name("ExpanderHeader"),
            |style| {
                style.static_resource(StyledElement::theme_property(), "ExpanderHeaderLeftStyle")?;
                style.set(DockPanel::dock_property(), Dock::Right);
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("ExpanderContent"), |style| {
            style.set(Border::border_thickness_property(), Thickness::new(1.0, 1.0, 0.0, 1.0));
            style.bind(
                Border::corner_radius_property(),
                TemplateBinding::new(TemplatedControl::corner_radius_property().property())
                    .with_converter_value(Some(style.scope().converter("LeftCornerRadiusFilterConverter")?)),
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // No WinUI counterpart: expanding to the right.
    theme.style(nesting().class(":right"), |style| {
        style.style(
            nesting().template().of::<ToggleButton>().name("ExpanderHeader"),
            |style| {
                style.static_resource(StyledElement::theme_property(), "ExpanderHeaderRightStyle")?;
                style.set(DockPanel::dock_property(), Dock::Left);
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("ExpanderContent"), |style| {
            style.set(Border::border_thickness_property(), Thickness::new(0.0, 1.0, 1.0, 1.0));
            style.bind(
                Border::corner_radius_property(),
                TemplateBinding::new(TemplatedControl::corner_radius_property().property())
                    .with_converter_value(Some(style.scope().converter("RightCornerRadiusFilterConverter")?)),
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // ExpandStates: ExpandDown and ExpandUp (CollapseDown and CollapseUp keep the corner radius
    //         of the header).
    theme.style(
        nesting()
            .class(":expanded")
            .class(":down")
            .template()
            .of::<ToggleButton>()
            .name("ExpanderHeader"),
        |style| {
            style.bind(
                TemplatedControl::corner_radius_property(),
                TemplateBinding::new(TemplatedControl::corner_radius_property().property())
                    .with_converter_value(Some(style.scope().converter("TopCornerRadiusFilterConverter")?)),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":expanded")
            .class(":up")
            .template()
            .of::<ToggleButton>()
            .name("ExpanderHeader"),
        |style| {
            style.bind(
                TemplatedControl::corner_radius_property(),
                TemplateBinding::new(TemplatedControl::corner_radius_property().property())
                    .with_converter_value(Some(style.scope().converter("BottomCornerRadiusFilterConverter")?)),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":expanded")
            .class(":left")
            .template()
            .of::<ToggleButton>()
            .name("ExpanderHeader"),
        |style| {
            style.bind(
                TemplatedControl::corner_radius_property(),
                TemplateBinding::new(TemplatedControl::corner_radius_property().property())
                    .with_converter_value(Some(style.scope().converter("RightCornerRadiusFilterConverter")?)),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":expanded")
            .class(":right")
            .template()
            .of::<ToggleButton>()
            .name("ExpanderHeader"),
        |style| {
            style.bind(
                TemplatedControl::corner_radius_property(),
                TemplateBinding::new(TemplatedControl::corner_radius_property().property())
                    .with_converter_value(Some(style.scope().converter("LeftCornerRadiusFilterConverter")?)),
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `Expander`.
fn expander_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Expander>();
    theme.based_on("DefaultExpanderStyle")?;
    Ok(())
}
