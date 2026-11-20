//! `Controls/SplitView.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/SplitView/SplitView_themeresources.xaml (DefaultSplitViewStyle and the implicit
//!   style of SplitView; the resources are Resources/SplitView_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The Grid PaneRoot (border, corner radius, pane background and its BackgroundTransition) is
//!   the Panel PART_PaneRoot that FerroUI's SplitView requires (it clips the pane and its bounds
//!   are the area of the light dismiss), holding a Border PaneRoot of the width of the open pane
//!   with the background, border and corner radius; the BrushTransition is a transition of the
//!   background of the border (150 ms, the default duration of a BrushTransition). WinUI's
//!   BrushTransition interpolates only solid colour brushes and changes any other brush at once: it
//!   is a SolidColorBrushTransition (FerroUI's BrushTransition switches a brush it cannot
//!   interpolate half-way, which left the acrylic background of an overlaying NavigationView pane
//!   out for the first 75 ms of the opening while the content of the pane was already shown). The
//!   transition is set by styles, so that an overlay pane keeps its background while it closes
//!   (DEVIATIONS.md, SPV-7).
//! - The states of DisplayModeStates (Closed, ClosedCompactLeft, OpenOverlayLeft, OpenInlineLeft,
//!   OpenCompactOverlayLeft and the Right ones) are the pseudo-classes FerroUI's SplitView sets:
//!   the display mode (:inline, :compactinline, :overlay, :compactoverlay), the state of the pane
//!   (:open, :closed) and the placement (:left, :right, :top, :bottom). The width of the pane
//!   column and the closed width of the pane are the TemplateSettings of FerroUI's control
//!   (PaneColumnGridLength, ClosedPaneWidth).
//! - The VisualTransitions of DisplayModeStates slide the pane in and out with a translation and
//!   a moving clip (opening an overlay pane in 350 ms with the key spline 0.1,0.9 0.2,1.0 and
//!   closing it in 120 ms; opening an inline pane in SplitViewPaneAnimationOpenDuration, 200 ms,
//!   and closing it in SplitViewPaneAnimationCloseDuration, 100 ms, with the key spline
//!   0.0,0.35 0.15,1.0): the port animates the width of PART_PaneRoot, which clips the pane, with
//!   transitions of those durations and easings; the pane keeps the width of the open pane, so
//!   its content is revealed, not laid out again. The fade of the light dismiss layer is not
//!   reproduced.
//! - The pane placement Top and Bottom of FerroUI has no counterpart in WinUI: those templates are
//!   the left one turned by a quarter, with the height of the pane animated.
//! - The base template is the one of the left placement; the nested styles of the other
//!   placements replace it.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::SolidColorBrushTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::ReflectionBinding;
use ferroui_base::data::RelativeSource;
use ferroui_base::data::RelativeSourceMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::styling::Selectors;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::SplitViewTemplateSettings;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinition;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::Panel;
use ferroui_controls::RowDefinition;
use ferroui_controls::SplitView;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultSplitViewStyle", default_split_view_style);
    resources.control_theme(type_key::<SplitView>(), split_view_theme);
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

/// The template of `SplitView` of `default_split_view_style`.
fn default_split_view_style_template(t: &Template<SplitView>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .with(|e| {
            let items = Grid::column_definitions(e);
            items.add(
                t.make(ColumnDefinition::new())
                    .bind(
                        ColumnDefinition::width_property(),
                        path()
                            .templated_parent()
                            .prop(SplitView::template_settings_property())
                            .prop(SplitViewTemplateSettings::pane_column_grid_length_property())
                            .binding(),
                    )
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
                    .end()
                    .into(),
            );
            Ok(())
        })?
        // Pane Content Area
        // Content Area
        .children([
            t.make(Panel::new())
                .name("PART_PaneRoot")?
                .set(Visual::clip_to_bounds_property(), true)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .set(Visual::z_index_property(), 1)
                .children([t
                    .make(Border::new())
                    .name("PaneRoot")?
                    .template_bind(Layoutable::width_property(), SplitView::open_pane_length_property())
                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                    .template_bind(Border::background_property(), SplitView::pane_background_property())
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
                        t.make(Panel::new()).children([
                            t.make(ContentPresenter::new())
                                .name("PART_PanePresenter")?
                                .template_bind(ContentPresenter::content_property(), SplitView::pane_property())
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    SplitView::pane_template_property(),
                                )
                                .control(),
                            t.make(Rectangle::new())
                                .name("HCPaneBorder")?
                                .set(Visual::is_visible_property(), false)
                                .dynamic(Shape::fill_property(), "SystemControlForegroundTransparentBrush")
                                .set(Layoutable::width_property(), 1.0)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                                .control(),
                        ]),
                    )
                    .control()])
                .control(),
            t.make(Panel::new())
                .name("ContentRoot")?
                .set(Grid::column_property(), 1)
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
                    t.make(Rectangle::new())
                        .name("LightDismissLayer")?
                        .set(Shape::fill_property(), Some(literal(Brush::parse("Transparent"))?))
                        .set(Visual::is_visible_property(), false)
                        .control(),
                ])
                .control(),
        ])
        .control())
}

/// The template of `SplitView` of `default_split_view_style`.
fn default_split_view_style_template_2(t: &Template<SplitView>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .with(|e| {
            let items = Grid::column_definitions(e);
            items.add(
                t.make(ColumnDefinition::new())
                    .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .bind(
                        ColumnDefinition::width_property(),
                        path()
                            .templated_parent()
                            .prop(SplitView::template_settings_property())
                            .prop(SplitViewTemplateSettings::pane_column_grid_length_property())
                            .binding(),
                    )
                    .end()
                    .into(),
            );
            Ok(())
        })?
        .children([
            t.make(Panel::new())
                .name("PART_PaneRoot")?
                .set(Grid::column_property(), 1)
                .set(Visual::clip_to_bounds_property(), true)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                .set(Visual::z_index_property(), 1)
                .children([t
                    .make(Border::new())
                    .name("PaneRoot")?
                    .template_bind(Layoutable::width_property(), SplitView::open_pane_length_property())
                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                    .template_bind(Border::background_property(), SplitView::pane_background_property())
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
                        t.make(Panel::new()).children([
                            t.make(ContentPresenter::new())
                                .name("PART_PanePresenter")?
                                .template_bind(ContentPresenter::content_property(), SplitView::pane_property())
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    SplitView::pane_template_property(),
                                )
                                .control(),
                            t.make(Rectangle::new())
                                .name("HCPaneBorder")?
                                .set(Visual::is_visible_property(), false)
                                .dynamic(Shape::fill_property(), "SystemControlForegroundTransparentBrush")
                                .set(Layoutable::width_property(), 1.0)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                .control(),
                        ]),
                    )
                    .control()])
                .control(),
            t.make(Panel::new())
                .name("ContentRoot")?
                .set(Grid::column_property(), 0)
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
                    t.make(Rectangle::new())
                        .name("LightDismissLayer")?
                        .set(Shape::fill_property(), Some(literal(Brush::parse("Transparent"))?))
                        .set(Visual::is_visible_property(), false)
                        .control(),
                ])
                .control(),
        ])
        .control())
}

/// The template of `SplitView` of `default_split_view_style`.
fn default_split_view_style_template_3(t: &Template<SplitView>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .with(|e| {
            let items = Grid::row_definitions(e);
            items.add(
                t.make(RowDefinition::new())
                    .bind(
                        RowDefinition::height_property(),
                        path()
                            .templated_parent()
                            .prop(SplitView::template_settings_property())
                            .prop(SplitViewTemplateSettings::pane_row_grid_length_property())
                            .binding(),
                    )
                    .end()
                    .into(),
            );
            items.add(
                t.make(RowDefinition::new())
                    .set(RowDefinition::height_property(), literal(GridLength::parse("*"))?)
                    .end()
                    .into(),
            );
            Ok(())
        })?
        .children([
            t.make(Panel::new())
                .name("PART_PaneRoot")?
                .set(Visual::clip_to_bounds_property(), true)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .set(Visual::z_index_property(), 1)
                .children([t
                    .make(Border::new())
                    .name("PaneRoot")?
                    .template_bind(Layoutable::height_property(), SplitView::open_pane_length_property())
                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                    .template_bind(Border::background_property(), SplitView::pane_background_property())
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
                        t.make(Panel::new()).children([
                            t.make(ContentPresenter::new())
                                .name("PART_PanePresenter")?
                                .template_bind(ContentPresenter::content_property(), SplitView::pane_property())
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    SplitView::pane_template_property(),
                                )
                                .control(),
                            t.make(Rectangle::new())
                                .name("HCPaneBorder")?
                                .set(Visual::is_visible_property(), false)
                                .dynamic(Shape::fill_property(), "SystemControlForegroundTransparentBrush")
                                .set(Layoutable::height_property(), 1.0)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                                .control(),
                        ]),
                    )
                    .control()])
                .control(),
            t.make(Panel::new())
                .name("ContentRoot")?
                .set(Grid::row_property(), 1)
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
                    t.make(Rectangle::new())
                        .name("LightDismissLayer")?
                        .set(Shape::fill_property(), Some(literal(Brush::parse("Transparent"))?))
                        .set(Visual::is_visible_property(), false)
                        .control(),
                ])
                .control(),
        ])
        .control())
}

/// The template of `SplitView` of `default_split_view_style`.
fn default_split_view_style_template_4(t: &Template<SplitView>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .with(|e| {
            let items = Grid::row_definitions(e);
            items.add(
                t.make(RowDefinition::new())
                    .set(RowDefinition::height_property(), literal(GridLength::parse("*"))?)
                    .end()
                    .into(),
            );
            items.add(
                t.make(RowDefinition::new())
                    .bind(
                        RowDefinition::height_property(),
                        path()
                            .templated_parent()
                            .prop(SplitView::template_settings_property())
                            .prop(SplitViewTemplateSettings::pane_row_grid_length_property())
                            .binding(),
                    )
                    .end()
                    .into(),
            );
            Ok(())
        })?
        .children([
            t.make(Panel::new())
                .name("PART_PaneRoot")?
                .set(Grid::row_property(), 1)
                .set(Visual::clip_to_bounds_property(), true)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                .set(Visual::z_index_property(), 1)
                .children([t
                    .make(Border::new())
                    .name("PaneRoot")?
                    .template_bind(Layoutable::height_property(), SplitView::open_pane_length_property())
                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                    .template_bind(Border::background_property(), SplitView::pane_background_property())
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
                        t.make(Panel::new()).children([
                            t.make(ContentPresenter::new())
                                .name("PART_PanePresenter")?
                                .template_bind(ContentPresenter::content_property(), SplitView::pane_property())
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    SplitView::pane_template_property(),
                                )
                                .control(),
                            t.make(Rectangle::new())
                                .name("HCPaneBorder")?
                                .set(Visual::is_visible_property(), false)
                                .dynamic(Shape::fill_property(), "SystemControlForegroundTransparentBrush")
                                .set(Layoutable::height_property(), 1.0)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                .control(),
                        ]),
                    )
                    .control()])
                .control(),
            t.make(Panel::new())
                .name("ContentRoot")?
                .set(Grid::row_property(), 0)
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
                    t.make(Rectangle::new())
                        .name("LightDismissLayer")?
                        .set(Shape::fill_property(), Some(literal(Brush::parse("Transparent"))?))
                        .set(Visual::is_visible_property(), false)
                        .control(),
                ])
                .control(),
        ])
        .control())
}

/// `DefaultSplitViewStyle`: a control theme of `SplitView`.
fn default_split_view_style(theme: &StyleBuilder) -> Result {
    theme.target::<SplitView>();
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "SystemControlForegroundTransparentBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "SplitViewLeftBorderThemeThickness",
    );
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Stretch,
    );
    theme.dynamic(SplitView::open_pane_length_property(), "SplitViewOpenPaneThemeLength");
    theme.dynamic(
        SplitView::compact_pane_length_property(),
        "SplitViewCompactPaneThemeLength",
    );
    theme.dynamic(
        SplitView::pane_background_property(),
        "SystemControlPageBackgroundChromeLowBrush",
    );
    theme.dynamic(
        TemplatedControl::corner_radius_property(),
        "SplitViewPaneRootCornerRadius",
    );
    // The pane at the left (the only placement of WinUI).
    theme.template(default_split_view_style_template);
    // The pane over the content (Overlay, CompactOverlay) or beside it (Inline, CompactInline):
    // the columns of ClosedCompactLeft, OpenInlineLeft and OpenCompactOverlayLeft.
    theme.style(nesting().class(":left"), |style| {
        style.style(nesting().class(":overlay"), |style| {
            style.style(nesting().template().of::<Panel>().name("PART_PaneRoot"), |style| {
                style.set(Grid::column_property(), 0);
                style.set(Grid::column_span_property(), 2);
                Ok(())
            })?;
            style.style(nesting().template().of::<Panel>().name("ContentRoot"), |style| {
                style.set(Grid::column_property(), 0);
                style.set(Grid::column_span_property(), 2);
                Ok(())
            })?;
            Ok(())
        })?;
        style.style(nesting().class(":compactoverlay"), |style| {
            style.style(nesting().template().of::<Panel>().name("PART_PaneRoot"), |style| {
                style.set(Grid::column_property(), 0);
                style.set(Grid::column_span_property(), 2);
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    })?;
    // The pane at the right: the states ClosedCompactRight, OpenOverlayRight, OpenInlineRight
    // and OpenCompactOverlayRight.
    theme.style(nesting().class(":right"), |style| {
        style.template(default_split_view_style_template_2);
        style.style(nesting().class(":overlay"), |style| {
            style.style(nesting().template().of::<Panel>().name("PART_PaneRoot"), |style| {
                style.set(Grid::column_property(), 0);
                style.set(Grid::column_span_property(), 2);
                Ok(())
            })?;
            style.style(nesting().template().of::<Panel>().name("ContentRoot"), |style| {
                style.set(Grid::column_property(), 0);
                style.set(Grid::column_span_property(), 2);
                Ok(())
            })?;
            Ok(())
        })?;
        style.style(nesting().class(":compactoverlay"), |style| {
            style.style(nesting().template().of::<Panel>().name("PART_PaneRoot"), |style| {
                style.set(Grid::column_property(), 0);
                style.set(Grid::column_span_property(), 2);
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    })?;
    // The pane at the top (FerroUI only).
    theme.style(nesting().class(":top"), |style| {
        style.template(default_split_view_style_template_3);
        style.style(nesting().class(":overlay"), |style| {
            style.style(nesting().template().of::<Panel>().name("PART_PaneRoot"), |style| {
                style.set(Grid::row_property(), 0);
                style.set(Grid::row_span_property(), 2);
                Ok(())
            })?;
            style.style(nesting().template().of::<Panel>().name("ContentRoot"), |style| {
                style.set(Grid::row_property(), 0);
                style.set(Grid::row_span_property(), 2);
                Ok(())
            })?;
            Ok(())
        })?;
        style.style(nesting().class(":compactoverlay"), |style| {
            style.style(nesting().template().of::<Panel>().name("PART_PaneRoot"), |style| {
                style.set(Grid::row_property(), 0);
                style.set(Grid::row_span_property(), 2);
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    })?;
    // The pane at the bottom (FerroUI only).
    theme.style(nesting().class(":bottom"), |style| {
        style.template(default_split_view_style_template_4);
        style.style(nesting().class(":overlay"), |style| {
            style.style(nesting().template().of::<Panel>().name("PART_PaneRoot"), |style| {
                style.set(Grid::row_property(), 0);
                style.set(Grid::row_span_property(), 2);
                Ok(())
            })?;
            style.style(nesting().template().of::<Panel>().name("ContentRoot"), |style| {
                style.set(Grid::row_property(), 0);
                style.set(Grid::row_span_property(), 2);
                Ok(())
            })?;
            Ok(())
        })?;
        style.style(nesting().class(":compactoverlay"), |style| {
            style.style(nesting().template().of::<Panel>().name("PART_PaneRoot"), |style| {
                style.set(Grid::row_property(), 0);
                style.set(Grid::row_span_property(), 2);
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    })?;
    // The size of the pane: the open length, or the closed one (0, or CompactPaneLength in the
    // compact modes). These setters style a part of the template from outside a ControlTemplate,
    // where a compiled binding cannot know the templated parent's type, and TemplateBinding has
    // no nested path (TemplateSettings.ClosedPaneWidth): they stay reflection bindings (#603).
    theme.style(
        Selectors::or([
            nesting()
                .class(":left")
                .class(":open")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
            nesting()
                .class(":right")
                .class(":open")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
        ]),
        |style| {
            style.bind(
                Layoutable::width_property(),
                ReflectionBinding::new("OpenPaneLength")
                    .with_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent))),
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":left")
                .class(":closed")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
            nesting()
                .class(":right")
                .class(":closed")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
        ]),
        |style| {
            style.bind(
                Layoutable::width_property(),
                ReflectionBinding::new("TemplateSettings.ClosedPaneWidth")
                    .with_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent))),
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":top")
                .class(":open")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
            nesting()
                .class(":bottom")
                .class(":open")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
        ]),
        |style| {
            style.bind(
                Layoutable::height_property(),
                ReflectionBinding::new("OpenPaneLength")
                    .with_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent))),
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":top")
                .class(":closed")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
            nesting()
                .class(":bottom")
                .class(":closed")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
        ]),
        |style| {
            style.bind(
                Layoutable::height_property(),
                ReflectionBinding::new("TemplateSettings.ClosedPaneHeight")
                    .with_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent))),
            );
            Ok(())
        },
    )?;
    // The VisualTransitions of DisplayModeStates. Inline panes: Closed to OpenInline in
    // SplitViewPaneAnimationOpenDuration and back in SplitViewPaneAnimationCloseDuration, key
    // spline 0.0,0.35 0.15,1.0.
    theme.style(
        Selectors::or([
            nesting()
                .class(":inline")
                .class(":open")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
            nesting()
                .class(":compactinline")
                .class(":open")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
        ]),
        |style| {
            style.set_value(
                Animatable::transitions_property(),
                value({
                    let object = Transitions::new();
                    object.add(
                        style
                            .scope()
                            .make(DoubleTransition::new())
                            .set(
                                TransitionBase::property_property(),
                                Some(Layoutable::width_property().property()),
                            )
                            .set(
                                TransitionBase::duration_property(),
                                SignedDuration::from_milliseconds(200.0),
                            )
                            .set(
                                TransitionBase::easing_property(),
                                literal(Easing::parse("0.0,0.35,0.15,1.0"))?,
                            )
                            .end()
                            .into(),
                    );
                    object.add(
                        style
                            .scope()
                            .make(DoubleTransition::new())
                            .set(
                                TransitionBase::property_property(),
                                Some(Layoutable::height_property().property()),
                            )
                            .set(
                                TransitionBase::duration_property(),
                                SignedDuration::from_milliseconds(200.0),
                            )
                            .set(
                                TransitionBase::easing_property(),
                                literal(Easing::parse("0.0,0.35,0.15,1.0"))?,
                            )
                            .end()
                            .into(),
                    );
                    object
                }),
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":inline")
                .class(":closed")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
            nesting()
                .class(":compactinline")
                .class(":closed")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
        ]),
        |style| {
            style.set_value(
                Animatable::transitions_property(),
                value({
                    let object = Transitions::new();
                    object.add(
                        style
                            .scope()
                            .make(DoubleTransition::new())
                            .set(
                                TransitionBase::property_property(),
                                Some(Layoutable::width_property().property()),
                            )
                            .set(
                                TransitionBase::duration_property(),
                                SignedDuration::from_milliseconds(100.0),
                            )
                            .set(
                                TransitionBase::easing_property(),
                                literal(Easing::parse("0.0,0.35,0.15,1.0"))?,
                            )
                            .end()
                            .into(),
                    );
                    object.add(
                        style
                            .scope()
                            .make(DoubleTransition::new())
                            .set(
                                TransitionBase::property_property(),
                                Some(Layoutable::height_property().property()),
                            )
                            .set(
                                TransitionBase::duration_property(),
                                SignedDuration::from_milliseconds(100.0),
                            )
                            .set(
                                TransitionBase::easing_property(),
                                literal(Easing::parse("0.0,0.35,0.15,1.0"))?,
                            )
                            .end()
                            .into(),
                    );
                    object
                }),
            );
            Ok(())
        },
    )?;
    // Overlay panes: opened in 350 ms and closed in 120 ms, key spline 0.1,0.9 0.2,1.0.
    theme.style(
        Selectors::or([
            nesting()
                .class(":overlay")
                .class(":open")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
            nesting()
                .class(":compactoverlay")
                .class(":open")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
        ]),
        |style| {
            style.set_value(
                Animatable::transitions_property(),
                value({
                    let object = Transitions::new();
                    object.add(
                        style
                            .scope()
                            .make(DoubleTransition::new())
                            .set(
                                TransitionBase::property_property(),
                                Some(Layoutable::width_property().property()),
                            )
                            .set(
                                TransitionBase::duration_property(),
                                SignedDuration::from_milliseconds(350.0),
                            )
                            .set(
                                TransitionBase::easing_property(),
                                literal(Easing::parse("0.1,0.9,0.2,1.0"))?,
                            )
                            .end()
                            .into(),
                    );
                    object.add(
                        style
                            .scope()
                            .make(DoubleTransition::new())
                            .set(
                                TransitionBase::property_property(),
                                Some(Layoutable::height_property().property()),
                            )
                            .set(
                                TransitionBase::duration_property(),
                                SignedDuration::from_milliseconds(350.0),
                            )
                            .set(
                                TransitionBase::easing_property(),
                                literal(Easing::parse("0.1,0.9,0.2,1.0"))?,
                            )
                            .end()
                            .into(),
                    );
                    object
                }),
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":overlay")
                .class(":closed")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
            nesting()
                .class(":compactoverlay")
                .class(":closed")
                .template()
                .of::<Panel>()
                .name("PART_PaneRoot"),
        ]),
        |style| {
            style.set_value(
                Animatable::transitions_property(),
                value({
                    let object = Transitions::new();
                    object.add(
                        style
                            .scope()
                            .make(DoubleTransition::new())
                            .set(
                                TransitionBase::property_property(),
                                Some(Layoutable::width_property().property()),
                            )
                            .set(
                                TransitionBase::duration_property(),
                                SignedDuration::from_milliseconds(120.0),
                            )
                            .set(
                                TransitionBase::easing_property(),
                                literal(Easing::parse("0.1,0.9,0.2,1.0"))?,
                            )
                            .end()
                            .into(),
                    );
                    object.add(
                        style
                            .scope()
                            .make(DoubleTransition::new())
                            .set(
                                TransitionBase::property_property(),
                                Some(Layoutable::height_property().property()),
                            )
                            .set(
                                TransitionBase::duration_property(),
                                SignedDuration::from_milliseconds(120.0),
                            )
                            .set(
                                TransitionBase::easing_property(),
                                literal(Easing::parse("0.1,0.9,0.2,1.0"))?,
                            )
                            .end()
                            .into(),
                    );
                    object
                }),
            );
            Ok(())
        },
    )?;
    // The BackgroundTransition of the Grid PaneRoot (a BrushTransition of 150 ms, the default
    // duration): solid colour brushes are interpolated, any other change of the background (to or
    // from the acrylic of an overlaying NavigationView pane) is immediate, as in WinUI.
    theme.style(nesting().template().of::<Border>().name("PaneRoot"), |style| {
        style.set_value(
            Animatable::transitions_property(),
            value({
                let object = Transitions::new();
                object.add(
                    style
                        .scope()
                        .make(SolidColorBrushTransition::new())
                        .set(
                            TransitionBase::property_property(),
                            Some(Border::background_property().property()),
                        )
                        .set(
                            TransitionBase::duration_property(),
                            SignedDuration::from_milliseconds(150.0),
                        )
                        .end()
                        .into(),
                );
                object
            }),
        );
        Ok(())
    })?;
    // Not in WinUI (DEVIATIONS.md, SPV-7): an overlay pane keeps its background while it closes
    // (120 ms, the closing transition), so the content of the pane is not drawn over the content
    // without a background. WinUI changes the background at once (NavigationView's
    // PaneNotOverlaying) and slides the pane out over the content.
    theme.style(
        Selectors::or([
            nesting()
                .class(":overlay")
                .class(":closed")
                .template()
                .of::<Border>()
                .name("PaneRoot"),
            nesting()
                .class(":compactoverlay")
                .class(":closed")
                .template()
                .of::<Border>()
                .name("PaneRoot"),
        ]),
        |style| {
            style.set_value(
                Animatable::transitions_property(),
                value({
                    let object = Transitions::new();
                    object.add(
                        style
                            .scope()
                            .make(SolidColorBrushTransition::new())
                            .set(
                                TransitionBase::property_property(),
                                Some(Border::background_property().property()),
                            )
                            .set(
                                TransitionBase::delay_property(),
                                SignedDuration::from_milliseconds(120.0),
                            )
                            .set(
                                TransitionBase::duration_property(),
                                SignedDuration::from_milliseconds(0.0),
                            )
                            .end()
                            .into(),
                    );
                    object
                }),
            );
            Ok(())
        },
    )?;
    // The open states show the high contrast border of the pane.
    theme.style(
        nesting()
            .class(":open")
            .template()
            .of::<Rectangle>()
            .name("HCPaneBorder"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // OpenOverlay and OpenCompactOverlay show the light dismiss layer; OverlayVisible
    // (LightDismissOverlayMode) fills it.
    theme.style(
        Selectors::or([
            nesting()
                .class(":overlay")
                .class(":open")
                .template()
                .of::<Rectangle>()
                .name("LightDismissLayer"),
            nesting()
                .class(":compactoverlay")
                .class(":open")
                .template()
                .of::<Rectangle>()
                .name("LightDismissLayer"),
        ]),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":lightDismiss")
            .template()
            .of::<Rectangle>()
            .name("LightDismissLayer"),
        |style| {
            style.dynamic(Shape::fill_property(), "SplitViewLightDismissOverlayBackground");
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `SplitView`.
fn split_view_theme(theme: &StyleBuilder) -> Result {
    theme.target::<SplitView>();
    theme.based_on("DefaultSplitViewStyle")?;
    Ok(())
}
