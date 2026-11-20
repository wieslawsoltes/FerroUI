//! `Controls/ScrollViewer.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/ScrollViewer_themeresources.xaml (the styles; its resources are
//!   Resources/ScrollViewer_themeresources.xaml)
//!   dxaml/xcp/dxaml/themes/generic.xaml (the ControlTemplate ScrollViewerScrollBarlessTemplate)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts: ScrollContentPresenter, VerticalScrollBar and HorizontalScrollBar are FerroUI's
//!   PART_ContentPresenter, PART_VerticalScrollBar and PART_HorizontalScrollBar. FerroUI's scroll
//!   viewer sets the range, value, viewport and visibility of its scroll bars in code, so the
//!   bindings of the source (Maximum, Value, ViewportSize, the Computed*ScrollBarVisibility of the
//!   Grid around each bar) are not in the template; ScrollViewerScrollBarMargin, the padding of that
//!   Grid, is the margin of the bar. The scroll gesture recognizer and the snap point bindings of
//!   the presenter are FerroUI's touch scrolling, which WinUI's presenter does in the platform.
//! - ScrollingIndicatorStates: WinUI's scroll viewer shows the mouse indicators (MouseIndicator,
//!   the IndicatorMode of its bars) while the pointer moves over it, and goes to NoIndicator, which
//!   fades the bars out over ScrollBarOpacityChangeDuration (83 ms), 2 s after the last move
//!   (ScrollViewerSeparatorContractDelay); MouseIndicatorFull is a bar under the pointer. Here
//!   NoIndicator is the opacity 0 of the scroll bars, MouseIndicator their opacity 1 while the
//!   pointer is over the scroll viewer (:pointerover) or a bar is expanded (IsExpanded, which is
//!   also the state of AllowAutoHide off), with a transition of 83 ms both ways. The bars hide
//!   when the pointer leaves the scroll viewer, without the delay of 2 s, and stay shown while it
//!   rests over it (FerroUI tells no pointer moves from rest); TouchIndicator (the panning
//!   indicator of touch) has no counterpart, the collapsed bar of MouseIndicator is drawn for any
//!   pointer. ScrollBarSeparatorExpanded is the IsExpanded state of the scroll viewer, with the
//!   opacity animated over ScrollViewerSeparatorExpandDuration (100 ms) and without the begin
//!   delays.
//! - VerticalScrollBarVisibility="Visible" of the source is not set: FerroUI's scroll viewer does
//!   not disable a bar whose content does not scroll, so a visible bar would show a thumb of the
//!   full length over such content under the pointer; the default Auto of the control shows the
//!   same indicators wherever there is something to scroll. The modes of the source that FerroUI's scroll
//!   viewer does not have (HorizontalScrollMode, VerticalScrollMode, IsHorizontalRailEnabled,
//!   IsVerticalRailEnabled, ZoomMode) are left out.
//! - ScrollViewerScrollBarlessTemplate, a template of the platform theme for the scroll viewers of
//!   templates that scroll without scroll bars (the looping lists of the date and time pickers,
//!   Pivot, CalendarView), is a keyed ControlTemplate of the same key: a WinUI application writes
//!   Template="{StaticResource ScrollViewerScrollBarlessTemplate}" on a ScrollViewer, and so does a
//!   FerroUI one. ScrollContentPresenter is PART_ContentPresenter; FerroUI's scroll viewer runs
//!   without its optional parts PART_HorizontalScrollBar and PART_VerticalScrollBar (it scrolls by
//!   the wheel, the keyboard and its methods). The padding of the scroll viewer is the padding of
//!   the presenter, not its margin, and the snap point bindings and the scroll gesture recognizer of
//!   the presenter are the ones of DefaultScrollViewerStyle (touch scrolling, which WinUI's
//!   presenter does in the platform), as in the default template.

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
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::styling::Selectors;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::ScrollBar;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::control_template;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultScrollViewerStyle", default_scroll_viewer_style);
    resources.control_theme(type_key::<ScrollViewer>(), scroll_viewer_theme);
    resources.deferred("ScrollViewerScrollBarlessTemplate", |scope| {
        Ok(value(control_template(
            scope,
            scroll_viewer_scroll_barless_template_template,
        )))
    });
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `ScrollViewer` of `default_scroll_viewer_style`.
fn default_scroll_viewer_style_template(t: &Template<ScrollViewer>) -> Result<Ref<Control>> {
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
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
                .with(|e| {
                    Grid::set_row_definitions(e, literal(RowDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
                // Opacity 0: NoIndicator; the opacity animates over ScrollBarOpacityChangeDuration.
                .children([
                    t.make(ScrollContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(
                            ContentPresenter::background_property(),
                            Some(literal(Brush::parse("Transparent"))?),
                        )
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
                    t.make(Grid::new())
                        .set(Grid::row_span_property(), 2)
                        .set(Grid::column_span_property(), 2)
                        .control(),
                    t.make(ScrollBar::new())
                        .name("PART_VerticalScrollBar")?
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
                        .set(Grid::row_property(), 1)
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
                        .set(Grid::row_property(), 1)
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

/// `DefaultScrollViewerStyle`: a control theme of `ScrollViewer`.
fn default_scroll_viewer_style(theme: &StyleBuilder) -> Result {
    theme.target::<ScrollViewer>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Top,
    );
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.set(
        TemplatedControl::border_brush_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.template(default_scroll_viewer_style_template);
    // MouseIndicator (ScrollingIndicatorStates): the bars are shown while the pointer is over the
    //         scroll viewer or a bar is expanded; NoIndicator otherwise (the opacity 0 of the template).
    theme.style(
        Selectors::or([
            nesting()
                .class(":pointerover")
                .template()
                .of::<ScrollBar>()
                .name("PART_VerticalScrollBar"),
            nesting()
                .class(":pointerover")
                .template()
                .of::<ScrollBar>()
                .name("PART_HorizontalScrollBar"),
        ]),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .property_is(ScrollViewer::is_expanded_property(), true)
                .template()
                .of::<ScrollBar>()
                .name("PART_VerticalScrollBar"),
            nesting()
                .property_is(ScrollViewer::is_expanded_property(), true)
                .template()
                .of::<ScrollBar>()
                .name("PART_HorizontalScrollBar"),
        ]),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        },
    )?;
    // ScrollBarSeparatorExpanded
    theme.style(
        nesting()
            .property_is(ScrollViewer::is_expanded_property(), true)
            .template()
            .of::<Border>()
            .name("ScrollBarSeparator"),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `ScrollViewer`.
fn scroll_viewer_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ScrollViewer>();
    theme.based_on("DefaultScrollViewerStyle")?;
    Ok(())
}

/// The template of `ScrollViewer` of `ScrollViewerScrollBarlessTemplate`.
fn scroll_viewer_scroll_barless_template_template(t: &Template<ScrollViewer>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .children([t
            .make(ScrollContentPresenter::new())
            .name("PART_ContentPresenter")?
            .set(
                ContentPresenter::background_property(),
                Some(literal(Brush::parse("Transparent"))?),
            )
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
            .control()])
        .control())
}
