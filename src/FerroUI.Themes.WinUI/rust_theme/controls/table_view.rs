//! `Controls/TableView.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TableView/TableView.xaml (the implicit TableView style)
//!   controls/dev/TableView/TableView.cpp (RebuildHeaders: the header band)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! WinUI's TableView belongs to the tabular library of microsoft-ui-xaml, not to the merged theme of
//! XamlControlsResources; its resources (TabularSurface*, TableView*, ResizeGripper*) are converted
//! from its own documents into Resources/TabularSurfaces_themeresources.xaml,
//! Resources/TableView_themeresources.xaml and Resources/ResizeGripper_themeresources.xaml
//! (scripts/winui-theme/convert.py, OUTSIDE_THE_MERGE). The rows, the cells and the column headers
//! are TableViewRow.xaml, TableViewCell.xaml and TableViewColumnHeader.xaml.
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's TableView is a ListBox: the template is the border of the source around the
//!   PART_ScrollViewer and the PART_ItemsPresenter of a ListBox; the rows are the items. The header
//!   band (PART_HeaderRow with PART_HeaderHost, the column headers FerroUI's
//!   TableViewColumnHeadersPresenter builds) is the first row of the template of the scroll viewer
//!   (TableViewScrollViewerStyle), above the rows and the vertical scroll bar as above
//!   PART_BodyScroller in the source; the header presenter follows the horizontal offset of the
//!   scroll viewer itself, so PART_HeaderScroller has no counterpart.
//! - The border of the table clips its content to its corner radius, as the rounded border of WinUI
//!   does: ClipToBounds is set by the theme and honoured by the border (as in the Fluent theme).
//! - The padding of the table is the margin of the items presenter and the padding of the scroll
//!   viewer is the padding of its presenter, as FerroUI's templates honour them.
//! - PART_EmptyStatePresenter, the frozen columns, the sorting, the grouping and the hierarchy of
//!   WinUI's TableView have no counterpart in FerroUI's TableView. IsTabStop="False" (the table is a
//!   focus scope of two tab stops) is left out: FerroUI's ListBox keeps its own keyboard handling.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::gesture_recognizers::ScrollGestureRecognizer;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::TextTrimming;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::presenters::TableViewColumnHeadersPresenter;
use ferroui_controls::primitives::ScrollBar;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TableView;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The scroll viewer of the table: DefaultScrollViewerStyle with the header band of the table
    // above the rows.
    resources.control_theme("TableViewScrollViewerStyle", table_view_scroll_viewer_style);
    resources.control_theme(type_key::<TableView>(), table_view_theme);
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

/// The template of `ScrollViewer` of `table_view_scroll_viewer_style`.
fn table_view_scroll_viewer_style_template(t: &Template<ScrollViewer>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
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
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .template_bind(Panel::background_property(), TemplatedControl::background_property())
                .with(|e| {
                    e.set_column_definitions(literal(ColumnDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
                .with(|e| {
                    e.set_row_definitions(literal(RowDefinitions::parse("Auto,*,Auto"))?);
                    Ok(())
                })?
                // Opacity 0: NoIndicator until the pointer is over the table, as the bars of
                // DefaultScrollViewerStyle (its nested styles show them).
                .children([
                    t.make(Border::new())
                        .name("PART_HeaderRow")?
                        .set(Grid::column_span_property(), 2)
                        .set(Visual::clip_to_bounds_property(), true)
                        .dynamic(Border::background_property(), "TabularSurfaceHeaderBackgroundBrush")
                        .dynamic(Border::border_brush_property(), "TabularSurfaceGridLineBrush")
                        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                        .set(Border::border_thickness_property(), Thickness::new(0.0, 0.0, 0.0, 1.0))
                        .child(
                            Decorator::child_property(),
                            t.make(TableViewColumnHeadersPresenter::new()).name("PART_HeaderHost")?,
                        )
                        .control(),
                    t.make(ScrollContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(Grid::row_property(), 1)
                        .set(Grid::row_span_property(), 2)
                        .set(Grid::column_span_property(), 2)
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .template_bind(
                            ContentPresenter::padding_property(),
                            TemplatedControl::padding_property(),
                        )
                        .template_bind(
                            ScrollContentPresenter::horizontal_snap_points_type_property(),
                            ScrollViewer::horizontal_snap_points_type_property(),
                        )
                        .template_bind(
                            ScrollContentPresenter::vertical_snap_points_type_property(),
                            ScrollViewer::vertical_snap_points_type_property(),
                        )
                        .template_bind(
                            ScrollContentPresenter::horizontal_snap_points_alignment_property(),
                            ScrollViewer::horizontal_snap_points_alignment_property(),
                        )
                        .template_bind(
                            ScrollContentPresenter::vertical_snap_points_alignment_property(),
                            ScrollViewer::vertical_snap_points_alignment_property(),
                        )
                        .template_bind(
                            ScrollViewer::is_scroll_inertia_enabled_property(),
                            ScrollViewer::is_scroll_inertia_enabled_property(),
                        )
                        .with(|e| {
                            let items = InputElement::gesture_recognizers(e);
                            items.add(
                                t.make(ScrollGestureRecognizer::new())
                                    .bind(
                                        ScrollGestureRecognizer::can_horizontally_scroll_property(),
                                        path()
                                            .named(t.names(), "PART_ContentPresenter")
                                            .prop(ScrollContentPresenter::can_horizontally_scroll_property())
                                            .binding(),
                                    )
                                    .bind(
                                        ScrollGestureRecognizer::can_vertically_scroll_property(),
                                        path()
                                            .named(t.names(), "PART_ContentPresenter")
                                            .prop(ScrollContentPresenter::can_vertically_scroll_property())
                                            .binding(),
                                    )
                                    .bind(
                                        ScrollGestureRecognizer::is_scroll_inertia_enabled_property(),
                                        path()
                                            .named(t.names(), "PART_ContentPresenter")
                                            .prop(ScrollViewer::is_scroll_inertia_enabled_property())
                                            .binding(),
                                    )
                                    .bind(
                                        ScrollGestureRecognizer::offset_property(),
                                        path()
                                            .named(t.names(), "PART_ContentPresenter")
                                            .prop(ScrollContentPresenter::offset_property())
                                            .binding(),
                                    )
                                    .bind(
                                        ScrollGestureRecognizer::viewport_property(),
                                        path()
                                            .named(t.names(), "PART_ContentPresenter")
                                            .prop(ScrollContentPresenter::viewport_property())
                                            .binding(),
                                    )
                                    .bind(
                                        ScrollGestureRecognizer::extent_property(),
                                        path()
                                            .named(t.names(), "PART_ContentPresenter")
                                            .prop(ScrollContentPresenter::extent_property())
                                            .binding(),
                                    )
                                    .end(),
                            );
                            Ok(())
                        })?
                        .control(),
                    t.make(ScrollBar::new())
                        .name("PART_VerticalScrollBar")?
                        .set(Grid::row_property(), 1)
                        .set(Grid::column_property(), 1)
                        .dynamic(Layoutable::margin_property(), "ScrollViewerScrollBarMargin")
                        .set(InputElement::is_tab_stop_property(), false)
                        .set(ScrollBar::orientation_property(), Orientation::Vertical)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                        .set(Visual::opacity_property(), 0.0)
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
                    t.make(ScrollBar::new())
                        .name("PART_HorizontalScrollBar")?
                        .set(Grid::row_property(), 2)
                        .dynamic(Layoutable::margin_property(), "ScrollViewerScrollBarMargin")
                        .set(InputElement::is_tab_stop_property(), false)
                        .set(ScrollBar::orientation_property(), Orientation::Horizontal)
                        .set(Visual::opacity_property(), 0.0)
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
                    t.make(Border::new())
                        .name("ScrollBarSeparator")?
                        .set(Grid::row_property(), 2)
                        .set(Grid::column_property(), 1)
                        .set(Visual::opacity_property(), 0.0)
                        .dynamic(
                            Border::background_property(),
                            "ScrollViewerScrollBarSeparatorBackground",
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
                                            SignedDuration::from_milliseconds(100.0),
                                        )
                                        .end()
                                        .into(),
                                );
                                object
                            }),
                        )
                        .control(),
                ]),
        )
        .control())
}

/// `TableViewScrollViewerStyle`: a control theme of `ScrollViewer`.
fn table_view_scroll_viewer_style(theme: &StyleBuilder) -> Result {
    theme.target::<ScrollViewer>();
    theme.based_on("DefaultScrollViewerStyle")?;
    theme.template(table_view_scroll_viewer_style_template);
    Ok(())
}

/// The template of `TableView` of `table_view_theme`.
fn table_view_theme_template(t: &Template<TableView>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .template_bind(Visual::clip_to_bounds_property(), Visual::clip_to_bounds_property())
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
            t.make(ScrollViewer::new())
                .name("PART_ScrollViewer")?
                .static_resource(StyledElement::theme_property(), "TableViewScrollViewerStyle")?
                .template_bind(
                    ScrollViewer::vertical_snap_points_type_property(),
                    ScrollViewer::vertical_snap_points_type_property(),
                )
                .template_bind(
                    ScrollViewer::horizontal_snap_points_type_property(),
                    ScrollViewer::horizontal_snap_points_type_property(),
                )
                .template_bind(
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::is_scroll_inertia_enabled_property(),
                    ScrollViewer::is_scroll_inertia_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::allow_auto_hide_property(),
                    ScrollViewer::allow_auto_hide_property(),
                )
                .template_bind(
                    ScrollViewer::bring_into_view_on_focus_change_property(),
                    ScrollViewer::bring_into_view_on_focus_change_property(),
                )
                .content(
                    ContentControl::content_property(),
                    t.make(ItemsPresenter::new())
                        .name("PART_ItemsPresenter")?
                        .template_bind(
                            ItemsPresenter::items_panel_property(),
                            ItemsControl::items_panel_property(),
                        )
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property()),
                )?,
        )
        .control())
}

/// The default control theme of `TableView`.
fn table_view_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TableView>();
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.dynamic(
        TemplatedControl::background_property(),
        "TabularSurfaceRowBackgroundBrush",
    );
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "TabularSurfaceRowForegroundBrush",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "TabularSurfaceGridLineBrush");
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0));
    // The rounded border of WinUI clips the table to its corner radius.
    theme.set(Visual::clip_to_bounds_property(), true);
    // PART_BodyScroller: both scroll bars Auto.
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(ScrollViewer::is_scroll_chaining_enabled_property(), true);
    theme.set(ScrollViewer::is_scroll_inertia_enabled_property(), true);
    // The text of the cells and the headers is trimmed (TextTrimming="CharacterEllipsis" of the
    // text blocks TableView creates).
    theme.set(
        TextBlock::text_trimming_property(),
        <dyn TextTrimming>::character_ellipsis(),
    );
    theme.template(table_view_theme_template);
    Ok(())
}
