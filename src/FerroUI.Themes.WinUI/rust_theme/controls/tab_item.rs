//! `Controls/TabItem.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TabView/TabView.xaml (the implicit style of TabViewItem; the resources of
//!   controls/dev/TabView/TabView_themeresources.xaml are Resources/TabView_themeresources.xaml)
//!   controls/dev/NavigationView/NavigationView_themeresources.xaml
//!   (MUX_NavigationViewItemPresenterStyleWhenOnLeftPane, the look of the tabs at the left and at
//!   the right; the resources are Resources/NavigationView_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! FerroUI's TabItem is WinUI's TabViewItem, a tab of TabView (Controls/TabControl.xaml). Its tabs
//! cannot be closed or dragged: the close button (CloseButton with TabViewCloseButtonStyle, the
//! state CloseButtonCollapsed applies), the drag visual TabDragVisualContainer and the states of
//! dragging and reordering (DragStates, ReorderHintStates, DragDropVisualStates) are left out.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The Grids LayoutRoot (padding) and TabContainer (background, border, padding, corner radius)
//!   are Borders of the same names around the Grids.
//! - The header: ContentPresenter (whose content TabViewItem sets to its Header in code) is the
//!   PART_ContentPresenter of the header (Content bound to Header, ContentTemplate to
//!   HeaderTemplate). The icon: IconControl shows the IconElement TabViewItem makes from its
//!   IconSource; FerroUI's TabItem has an Icon and an IconTemplate, shown by a content presenter
//!   of the same name in the Viewbox IconBox, which is hidden without an icon (IconStates); a path
//!   icon there is unscaled as WinUI's (PathIconUnscaledTemplate, DEVIATIONS.md PI-3).
//! - SelectedBackgroundPath is a Path in a Canvas whose geometry TabViewItem computes in code from
//!   its size (TabViewTemplateSettings.TabGeometry: a rectangle with the top corners of
//!   OverlayCornerRadius and two 4 px feet curving out at the bottom). It is a Grid of the same
//!   name with the same margins: the left foot, a Border with the top corners of the corner radius
//!   of the tab and the right foot, filled with TabViewItemHeaderBackgroundSelected. The paths
//!   LeftRadiusRenderArc and RightRadiusRenderArc keep their data.
//! - The states TabView sets from code are written as selectors where FerroUI has the condition:
//!   the selected tab is drawn above its neighbours (Canvas.ZIndex 20 of the code, ZIndex) and
//!   hides its separator (SetTabSeparatorOpacity of TabView); the separator of the tab left of the
//!   selected one, the margins of the bottom line beside the selected tab (LeftOfSelectedTab,
//!   RightOfSelectedTab) and the compact width mode are left out. ForegroundSet (an application's
//!   Foreground on the tab replaces the brushes of the states) is left out: FerroUI's TabItem
//!   passes its foreground to an icon element in code, the states set the header presenter.
//! - TabViewItem's width is computed by TabView (TabWidthMode Equal, between TabViewItemMinWidth
//!   and TabViewItemMaxWidth) and set as its Width in code: the items panel of the strip of the
//!   TabControl (TabViewEqualWidthPanel, Controls/TabControl.xaml) gives the tab that width, as it
//!   gives every tab of the strip. A tab outside such a strip is as wide as its header, as a
//!   TabViewItem without a width.
//! - The states PointerOverSelected and PressedSelected set what Selected sets: ^:selected,
//!   written after the pointer states, applies to them. LayoutRoot.Background="Transparent" of
//!   the selected states is left out (the tab container's background makes the tab hit-testable).
//! - The tabs at the bottom are the tabs of the top turned upside down by the strip
//!   (Controls/TabControl.xaml); the header and the icon are turned back. The tabs at the left and
//!   at the right are the items of the left pane of NavigationView: the template of
//!   MUX_NavigationViewItemPresenterStyleWhenOnLeftPane with the values of the implicit style of
//!   NavigationViewItem (font, corner radius), its pointer states, the selection indicator and
//!   the disabled opacity; the expand chevron, the info badge and the compact pane states are left
//!   out.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::FontWeight;
use ferroui_base::media::Geometry;
use ferroui_base::styling::Selectors;
use ferroui_base::RelativePoint;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::HeaderedContentControl;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Dock;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_controls::PathIcon;
use ferroui_controls::TabItem;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<TabItem>(), tab_item_theme);
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

/// The template of `TabItem` of `tab_item_theme`.
fn tab_item_theme_template(t: &Template<TabItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .set(Layoutable::use_layout_rounding_property(), false)
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*,Auto"))?);
                    Ok(())
                })?
                // The tab shape of TabViewTemplateSettings.TabGeometry.
                // CloseIconStates: CloseButtonCollapsed (TabViewItemHeaderPaddingWithoutCloseButton).
                .children([
                    t.make(Border::new())
                        .name("BottomBorderLine")?
                        .set(Grid::column_span_property(), 3)
                        .dynamic(Border::background_property(), "TabViewBorderBrush")
                        .set(Layoutable::height_property(), 1.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                        .control(),
                    t.make(Path::new())
                        .name("LeftRadiusRenderArc")?
                        .set(Visual::is_visible_property(), false)
                        .dynamic(Shape::fill_property(), "TabViewBorderBrush")
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                        .set(Layoutable::margin_property(), Thickness::new(-4.0, 0.0, 0.0, 0.0))
                        .set(Layoutable::height_property(), 4.0)
                        .set(Layoutable::width_property(), 4.0)
                        .set(
                            Path::data_property(),
                            Some(literal(Geometry::parse(
                                "M4 0C4 1.19469 3.47624 2.26706 2.64582 3H0C1.65685 3 3 1.65685 3 0H4Z",
                            ))?),
                        )
                        .control(),
                    t.make(Path::new())
                        .name("RightRadiusRenderArc")?
                        .set(Grid::column_property(), 2)
                        .set(Visual::is_visible_property(), false)
                        .dynamic(Shape::fill_property(), "TabViewBorderBrush")
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                        .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, -4.0, 0.0))
                        .set(Layoutable::height_property(), 4.0)
                        .set(Layoutable::width_property(), 4.0)
                        .set(
                            Path::data_property(),
                            Some(literal(Geometry::parse(
                                "M0 0C0 1.19469 0.523755 2.26706 1.35418 3H4C2.34315 3 1 1.65685 1 0H0Z",
                            ))?),
                        )
                        .control(),
                    t.make(Grid::new())
                        .name("SelectedBackgroundPath")?
                        .set(Grid::column_span_property(), 3)
                        .set(Layoutable::margin_property(), Thickness::symmetric(-4.0, 0.0))
                        .set(Visual::is_visible_property(), false)
                        .with(|e| {
                            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("4,*,4"))?);
                            Ok(())
                        })?
                        .children([
                            t.make(Path::new())
                                .set(Layoutable::width_property(), 4.0)
                                .set(Layoutable::height_property(), 4.0)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                                .dynamic(Shape::fill_property(), "TabViewItemHeaderBackgroundSelected")
                                .set(
                                    Path::data_property(),
                                    Some(literal(Geometry::parse("M0,4 A4,4 0 0 0 4,0 L4,4 Z"))?),
                                )
                                .control(),
                            t.make(Border::new())
                                .set(Grid::column_property(), 1)
                                .dynamic(Border::background_property(), "TabViewItemHeaderBackgroundSelected")
                                .template_bind_with(
                                    Border::corner_radius_property(),
                                    Some(TemplatedControl::corner_radius_property().property()),
                                    Some(t.scope().converter("TopCornerRadiusFilterConverter")?),
                                    None,
                                    None,
                                )
                                .control(),
                            t.make(Path::new())
                                .set(Grid::column_property(), 2)
                                .set(Layoutable::width_property(), 4.0)
                                .set(Layoutable::height_property(), 4.0)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                                .dynamic(Shape::fill_property(), "TabViewItemHeaderBackgroundSelected")
                                .set(
                                    Path::data_property(),
                                    Some(literal(Geometry::parse("M0,0 A4,4 0 0 0 4,4 L0,4 Z"))?),
                                )
                                .control(),
                        ])
                        .control(),
                    t.make(Border::new())
                        .name("TabSeparator")?
                        .set(Grid::column_property(), 1)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                        .set(Layoutable::width_property(), 1.0)
                        .dynamic(Border::background_property(), "TabViewItemSeparator")
                        .dynamic(Layoutable::margin_property(), "TabViewItemSeparatorMargin")
                        .control(),
                    t.make(Border::new())
                        .name("TabContainer")?
                        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                        .set(Grid::column_property(), 1)
                        .template_bind(Border::background_property(), TemplatedControl::background_property())
                        .template_bind(
                            Border::border_brush_property(),
                            TemplatedControl::border_brush_property(),
                        )
                        .template_bind(
                            Border::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .static_resource(
                            Decorator::padding_property(),
                            "TabViewItemHeaderPaddingWithoutCloseButton",
                        )?
                        .template_bind_with(
                            Border::corner_radius_property(),
                            Some(TemplatedControl::corner_radius_property().property()),
                            Some(t.scope().converter("TopCornerRadiusFilterConverter")?),
                            None,
                            None,
                        )
                        .child(
                            Decorator::child_property(),
                            t.make(Grid::new())
                                .name("TabContentGrid")?
                                .with(|e| {
                                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*"))?);
                                    Ok(())
                                })?
                                .set(
                                    Visual::render_transform_origin_property(),
                                    literal(RelativePoint::parse("50%,50%"))?,
                                )
                                .children([
                                    t.make(Viewbox::new())
                                        .name("IconBox")?
                                        .dynamic(Layoutable::max_width_property(), "TabViewItemHeaderIconSize")
                                        .dynamic(Layoutable::max_height_property(), "TabViewItemHeaderIconSize")
                                        .dynamic(Layoutable::margin_property(), "TabViewItemHeaderIconMargin")
                                        .template_bind_with(
                                            Visual::is_visible_property(),
                                            Some(TabItem::icon_property().property()),
                                            Some(ObjectConverters::is_not_null()),
                                            None,
                                            None,
                                        )
                                        .child(
                                            Viewbox::child_property(),
                                            t.make(ContentPresenter::new())
                                                .name("IconControl")?
                                                .template_bind(
                                                    ContentPresenter::content_property(),
                                                    TabItem::icon_property(),
                                                )
                                                .template_bind(
                                                    ContentPresenter::content_template_property(),
                                                    TabItem::icon_template_property(),
                                                )
                                                .dynamic(
                                                    ContentPresenter::foreground_property(),
                                                    "TabViewItemIconForeground",
                                                )
                                                // A path icon as WinUI's, unscaled, so that the Viewbox scales its geometry as WinUI's does (PI-3).
                                                .style(of::<PathIcon>(), |style| {
                                                    style.set(Layoutable::width_property(), f64::NAN);
                                                    style.set(Layoutable::height_property(), f64::NAN);
                                                    style.static_resource(
                                                        TemplatedControl::template_property(),
                                                        "PathIconUnscaledTemplate",
                                                    )?;
                                                    Ok(())
                                                })?,
                                        )
                                        .control(),
                                    t.make(ContentPresenter::new())
                                        .name("PART_ContentPresenter")?
                                        .set(Grid::column_property(), 1)
                                        .template_bind(
                                            Layoutable::horizontal_alignment_property(),
                                            ContentControl::horizontal_content_alignment_property(),
                                        )
                                        .template_bind(
                                            Layoutable::vertical_alignment_property(),
                                            ContentControl::vertical_content_alignment_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::content_property(),
                                            HeaderedContentControl::header_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::content_template_property(),
                                            HeaderedContentControl::header_template_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::font_weight_property(),
                                            TemplatedControl::font_weight_property(),
                                        )
                                        .dynamic(ContentPresenter::font_size_property(), "TabViewItemHeaderFontSize")
                                        .dynamic(ContentPresenter::foreground_property(), "TabViewItemHeaderForeground")
                                        .set(ContentPresenter::recognizes_access_key_property(), true)
                                        .control(),
                                ]),
                        )
                        .control(),
                ]),
        )
        .control())
}

/// The template of `TabItem` of `tab_item_theme`.
fn tab_item_theme_template_2(t: &Template<TabItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .dynamic(Layoutable::min_height_property(), "NavigationViewItemOnLeftMinHeight")
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .dynamic(Layoutable::margin_property(), "NavigationViewItemButtonMargin")
        .child(
            Decorator::child_property(),
            t.make(Panel::new()).name("PresenterContentRootGrid")?.children([
                t.make(Panel::new())
                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                    .children([t
                        .make(Rectangle::new())
                        .name("SelectionIndicator")?
                        .dynamic(Layoutable::width_property(), "NavigationViewSelectionIndicatorWidth")
                        .dynamic(Layoutable::height_property(), "NavigationViewSelectionIndicatorHeight")
                        .dynamic(Shape::fill_property(), "NavigationViewSelectionIndicatorForeground")
                        .set(Visual::opacity_property(), 0.0)
                        .static_resource(Rectangle::radius_x_property(), "NavigationViewSelectionIndicatorRadius")?
                        .static_resource(Rectangle::radius_y_property(), "NavigationViewSelectionIndicatorRadius")?
                        .control()])
                    .control(),
                t.make(Grid::new())
                    .name("ContentGrid")?
                    .dynamic(Layoutable::min_height_property(), "NavigationViewItemOnLeftMinHeight")
                    .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 14.0, 0.0))
                    .set(
                        Layoutable::horizontal_alignment_property(),
                        HorizontalAlignment::Stretch,
                    )
                    .with(|e| {
                        Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*"))?);
                        Ok(())
                    })?
                    // IconColumn: the icon box in the compact pane length less 8 (40), or 8 wide
                    // without an icon (IconCollapsed).
                    .children([
                        t.make(Border::new())
                            .name("IconColumn")?
                            .set(Layoutable::width_property(), 40.0)
                            .template_bind_with(
                                Visual::is_visible_property(),
                                Some(TabItem::icon_property().property()),
                                Some(ObjectConverters::is_not_null()),
                                None,
                                None,
                            )
                            .child(
                                Decorator::child_property(),
                                t.make(Viewbox::new())
                                    .name("IconBox")?
                                    .dynamic(Layoutable::height_property(), "NavigationViewItemOnLeftIconBoxHeight")
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                    .dynamic(Layoutable::margin_property(), "NavigationViewItemOnLeftIconBoxMargin")
                                    .child(
                                        Viewbox::child_property(),
                                        t.make(ContentPresenter::new())
                                            .name("Icon")?
                                            .template_bind(
                                                ContentPresenter::content_property(),
                                                TabItem::icon_property(),
                                            )
                                            .template_bind(
                                                ContentPresenter::content_template_property(),
                                                TabItem::icon_template_property(),
                                            )
                                            .template_bind(
                                                ContentPresenter::foreground_property(),
                                                TemplatedControl::foreground_property(),
                                            )
                                            .dynamic(
                                                ContentPresenter::background_property(),
                                                "NavigationViewItemIconBackground",
                                            )
                                            // A path icon as WinUI's, unscaled, so that the Viewbox scales its geometry as WinUI's does (PI-3).
                                            .style(of::<PathIcon>(), |style| {
                                                style.set(Layoutable::width_property(), f64::NAN);
                                                style.set(Layoutable::height_property(), f64::NAN);
                                                style.static_resource(
                                                    TemplatedControl::template_property(),
                                                    "PathIconUnscaledTemplate",
                                                )?;
                                                Ok(())
                                            })?,
                                    ),
                            )
                            .control(),
                        t.make(Border::new())
                            .name("IconColumnCollapsed")?
                            .set(Layoutable::width_property(), 8.0)
                            .template_bind_with(
                                Visual::is_visible_property(),
                                Some(TabItem::icon_property().property()),
                                Some(ObjectConverters::is_null()),
                                None,
                                None,
                            )
                            .control(),
                        t.make(ContentPresenter::new())
                            .name("PART_ContentPresenter")?
                            .set(Grid::column_property(), 1)
                            .dynamic(
                                Layoutable::margin_property(),
                                "NavigationViewItemContentPresenterMargin",
                            )
                            .template_bind(
                                ContentPresenter::content_property(),
                                HeaderedContentControl::header_property(),
                            )
                            .template_bind(
                                ContentPresenter::content_template_property(),
                                HeaderedContentControl::header_template_property(),
                            )
                            .template_bind(
                                ContentPresenter::foreground_property(),
                                TemplatedControl::foreground_property(),
                            )
                            .template_bind(
                                Layoutable::horizontal_alignment_property(),
                                ContentControl::horizontal_content_alignment_property(),
                            )
                            .template_bind(
                                Layoutable::vertical_alignment_property(),
                                ContentControl::vertical_content_alignment_property(),
                            )
                            .template_bind(
                                ContentPresenter::padding_property(),
                                TemplatedControl::padding_property(),
                            )
                            .set(ContentPresenter::recognizes_access_key_property(), true)
                            .control(),
                    ])
                    .control(),
            ]),
        )
        .control())
}

/// The template of `TabItem` of `tab_item_theme`.
fn tab_item_theme_template_3(t: &Template<TabItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .dynamic(Layoutable::min_height_property(), "NavigationViewItemOnLeftMinHeight")
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .dynamic(Layoutable::margin_property(), "NavigationViewItemButtonMargin")
        .child(
            Decorator::child_property(),
            t.make(Panel::new()).name("PresenterContentRootGrid")?.children([
                t.make(Panel::new())
                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                    .children([t
                        .make(Rectangle::new())
                        .name("SelectionIndicator")?
                        .dynamic(Layoutable::width_property(), "NavigationViewSelectionIndicatorWidth")
                        .dynamic(Layoutable::height_property(), "NavigationViewSelectionIndicatorHeight")
                        .dynamic(Shape::fill_property(), "NavigationViewSelectionIndicatorForeground")
                        .set(Visual::opacity_property(), 0.0)
                        .static_resource(Rectangle::radius_x_property(), "NavigationViewSelectionIndicatorRadius")?
                        .static_resource(Rectangle::radius_y_property(), "NavigationViewSelectionIndicatorRadius")?
                        .control()])
                    .control(),
                t.make(Grid::new())
                    .name("ContentGrid")?
                    .dynamic(Layoutable::min_height_property(), "NavigationViewItemOnLeftMinHeight")
                    .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 14.0, 0.0))
                    .set(
                        Layoutable::horizontal_alignment_property(),
                        HorizontalAlignment::Stretch,
                    )
                    .with(|e| {
                        Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*"))?);
                        Ok(())
                    })?
                    // IconColumn: the icon box in the compact pane length less 8 (40), or 8 wide
                    // without an icon (IconCollapsed).
                    .children([
                        t.make(Border::new())
                            .name("IconColumn")?
                            .set(Layoutable::width_property(), 40.0)
                            .template_bind_with(
                                Visual::is_visible_property(),
                                Some(TabItem::icon_property().property()),
                                Some(ObjectConverters::is_not_null()),
                                None,
                                None,
                            )
                            .child(
                                Decorator::child_property(),
                                t.make(Viewbox::new())
                                    .name("IconBox")?
                                    .dynamic(Layoutable::height_property(), "NavigationViewItemOnLeftIconBoxHeight")
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                    .dynamic(Layoutable::margin_property(), "NavigationViewItemOnLeftIconBoxMargin")
                                    .child(
                                        Viewbox::child_property(),
                                        t.make(ContentPresenter::new())
                                            .name("Icon")?
                                            .template_bind(
                                                ContentPresenter::content_property(),
                                                TabItem::icon_property(),
                                            )
                                            .template_bind(
                                                ContentPresenter::content_template_property(),
                                                TabItem::icon_template_property(),
                                            )
                                            .template_bind(
                                                ContentPresenter::foreground_property(),
                                                TemplatedControl::foreground_property(),
                                            )
                                            .dynamic(
                                                ContentPresenter::background_property(),
                                                "NavigationViewItemIconBackground",
                                            )
                                            // A path icon as WinUI's, unscaled, so that the Viewbox scales its geometry as WinUI's does (PI-3).
                                            .style(of::<PathIcon>(), |style| {
                                                style.set(Layoutable::width_property(), f64::NAN);
                                                style.set(Layoutable::height_property(), f64::NAN);
                                                style.static_resource(
                                                    TemplatedControl::template_property(),
                                                    "PathIconUnscaledTemplate",
                                                )?;
                                                Ok(())
                                            })?,
                                    ),
                            )
                            .control(),
                        t.make(Border::new())
                            .name("IconColumnCollapsed")?
                            .set(Layoutable::width_property(), 8.0)
                            .template_bind_with(
                                Visual::is_visible_property(),
                                Some(TabItem::icon_property().property()),
                                Some(ObjectConverters::is_null()),
                                None,
                                None,
                            )
                            .control(),
                        t.make(ContentPresenter::new())
                            .name("PART_ContentPresenter")?
                            .set(Grid::column_property(), 1)
                            .dynamic(
                                Layoutable::margin_property(),
                                "NavigationViewItemContentPresenterMargin",
                            )
                            .template_bind(
                                ContentPresenter::content_property(),
                                HeaderedContentControl::header_property(),
                            )
                            .template_bind(
                                ContentPresenter::content_template_property(),
                                HeaderedContentControl::header_template_property(),
                            )
                            .template_bind(
                                ContentPresenter::foreground_property(),
                                TemplatedControl::foreground_property(),
                            )
                            .template_bind(
                                Layoutable::horizontal_alignment_property(),
                                ContentControl::horizontal_content_alignment_property(),
                            )
                            .template_bind(
                                Layoutable::vertical_alignment_property(),
                                ContentControl::vertical_content_alignment_property(),
                            )
                            .template_bind(
                                ContentPresenter::padding_property(),
                                TemplatedControl::padding_property(),
                            )
                            .set(ContentPresenter::recognizes_access_key_property(), true)
                            .control(),
                    ])
                    .control(),
            ]),
        )
        .control())
}

/// The default control theme of `TabItem`.
fn tab_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TabItem>();
    theme.dynamic(TemplatedControl::background_property(), "TabViewItemHeaderBackground");
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    // The default of a WinUI control (FerroUI's default is Top).
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(Layoutable::min_height_property(), "TabViewItemMinHeight");
    theme.dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TabViewItemBorderThickness",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "TabViewItemBorderBrush");
    theme.template(tab_item_theme_template);
    // CommonStates: PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("TabContainer"), |style| {
            style.dynamic(Border::background_property(), "TabViewItemHeaderBackgroundPointerOver");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("TabContainer")
                .descendant()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TabViewItemHeaderForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<ContentPresenter>().name("IconControl"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TabViewItemIconForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("TabSeparator"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("TabContainer"), |style| {
            style.dynamic(Border::background_property(), "TabViewItemHeaderBackgroundPressed");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("TabContainer")
                .descendant()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TabViewItemHeaderForegroundPressed",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<ContentPresenter>().name("IconControl"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TabViewItemIconForegroundPressed",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("TabSeparator"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Selected (also PointerOverSelected and PressedSelected, which set the same values)
    theme.style(nesting().class(":selected"), |style| {
        // Canvas.ZIndex 20 of TabViewItem::OnIsSelectedPropertyChanged.
        style.set(Visual::z_index_property(), 20);
        style.style(nesting().template().of::<Border>().name("BottomBorderLine"), |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        })?;
        style.style(
            Selectors::or([
                nesting().template().of::<Path>().name("RightRadiusRenderArc"),
                nesting().template().of::<Path>().name("LeftRadiusRenderArc"),
                nesting().template().of::<Grid>().name("SelectedBackgroundPath"),
            ]),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("TabContainer"), |style| {
            style.dynamic(Border::background_property(), "TabViewItemHeaderBackground");
            style.dynamic(Layoutable::margin_property(), "TabViewSelectedItemHeaderMargin");
            style.dynamic(Border::border_brush_property(), "TabViewSelectedItemBorderBrush");
            style.dynamic(
                Border::border_thickness_property(),
                "TabViewSelectedItemBorderThickness",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("TabContainer")
                .descendant()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TabViewItemHeaderForegroundSelected",
                );
                style.set(ContentPresenter::font_weight_property(), FontWeight::SemiBold);
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<ContentPresenter>().name("IconControl"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TabViewItemIconForegroundSelected",
                );
                Ok(())
            },
        )?;
        // TabView::SetTabSeparatorOpacity hides the separator of the selected tab.
        style.style(nesting().template().of::<Border>().name("TabSeparator"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // DisabledStates: Disabled (IconControl.Foreground is set twice in the source; the second
    // value, TabViewButtonForegroundDisabled, is the one in effect)
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("TabContainer"), |style| {
            style.dynamic(Border::background_property(), "TabViewItemHeaderBackgroundDisabled");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ContentPresenter>().name("IconControl"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TabViewButtonForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("TabContainer")
                .descendant()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TabViewItemHeaderForegroundDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // The tabs at the bottom: the strip turns them upside down; the header and the icon are
    // turned back.
    theme.style(
        nesting()
            .property_is(TabItem::tab_strip_placement_property(), Some(Dock::Bottom))
            .template()
            .of::<Grid>()
            .name("TabContentGrid"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scaleY(-1)"))?),
            );
            Ok(())
        },
    )?;
    // The tabs at the left and at the right: the items of the left pane of NavigationView
    // (MUX_NavigationViewItemPresenterStyleWhenOnLeftPane, NavigationViewItem).
    theme.style(
        nesting().property_is(TabItem::tab_strip_placement_property(), Some(Dock::Left)),
        |style| {
            style.dynamic(TemplatedControl::foreground_property(), "NavigationViewItemForeground");
            style.dynamic(TemplatedControl::background_property(), "NavigationViewItemBackground");
            style.dynamic(
                TemplatedControl::border_brush_property(),
                "NavigationViewItemBorderBrush",
            );
            style.static_resource(
                TemplatedControl::border_thickness_property(),
                "NavigationViewItemBorderThickness",
            )?;
            style.dynamic(
                TemplatedControl::font_family_property(),
                "ContentControlThemeFontFamily",
            );
            style.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
            style.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
            style.set(
                ContentControl::horizontal_content_alignment_property(),
                HorizontalAlignment::Stretch,
            );
            style.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
            style.template(tab_item_theme_template_2);
            // PointerStates: PointerOver
            style.style(nesting().class(":pointerover"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(Border::background_property(), "NavigationViewItemBackgroundPointerOver");
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundPointerOver",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // Pressed
            style.style(nesting().class(":pressed"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(Border::background_property(), "NavigationViewItemBackgroundPressed");
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundPressed",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // Selected, with the selection indicator NavigationView shows on its selected item
            style.style(nesting().class(":selected"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(Border::background_property(), "NavigationViewItemBackgroundSelected");
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundSelected",
                        );
                        Ok(())
                    },
                )?;
                style.style(
                    nesting().template().of::<Rectangle>().name("SelectionIndicator"),
                    |style| {
                        style.set(Visual::opacity_property(), 1.0);
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // PointerOverSelected
            style.style(nesting().class(":selected").class(":pointerover"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(
                        Border::background_property(),
                        "NavigationViewItemBackgroundSelectedPointerOver",
                    );
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundSelectedPointerOver",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // PressedSelected
            style.style(nesting().class(":selected").class(":pressed"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(
                        Border::background_property(),
                        "NavigationViewItemBackgroundSelectedPressed",
                    );
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundSelectedPressed",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // DisabledStates: Disabled
            style.style(
                nesting()
                    .class(":disabled")
                    .template()
                    .of::<Border>()
                    .name("LayoutRoot"),
                |style| {
                    style.dynamic(Visual::opacity_property(), "ListViewItemDisabledThemeOpacity");
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    theme.style(
        nesting().property_is(TabItem::tab_strip_placement_property(), Some(Dock::Right)),
        |style| {
            style.dynamic(TemplatedControl::foreground_property(), "NavigationViewItemForeground");
            style.dynamic(TemplatedControl::background_property(), "NavigationViewItemBackground");
            style.dynamic(
                TemplatedControl::border_brush_property(),
                "NavigationViewItemBorderBrush",
            );
            style.static_resource(
                TemplatedControl::border_thickness_property(),
                "NavigationViewItemBorderThickness",
            )?;
            style.dynamic(
                TemplatedControl::font_family_property(),
                "ContentControlThemeFontFamily",
            );
            style.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
            style.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
            style.set(
                ContentControl::horizontal_content_alignment_property(),
                HorizontalAlignment::Stretch,
            );
            style.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
            style.template(tab_item_theme_template_3);
            // PointerStates: PointerOver
            style.style(nesting().class(":pointerover"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(Border::background_property(), "NavigationViewItemBackgroundPointerOver");
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundPointerOver",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // Pressed
            style.style(nesting().class(":pressed"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(Border::background_property(), "NavigationViewItemBackgroundPressed");
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundPressed",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // Selected, with the selection indicator NavigationView shows on its selected item
            style.style(nesting().class(":selected"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(Border::background_property(), "NavigationViewItemBackgroundSelected");
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundSelected",
                        );
                        Ok(())
                    },
                )?;
                style.style(
                    nesting().template().of::<Rectangle>().name("SelectionIndicator"),
                    |style| {
                        style.set(Visual::opacity_property(), 1.0);
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // PointerOverSelected
            style.style(nesting().class(":selected").class(":pointerover"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(
                        Border::background_property(),
                        "NavigationViewItemBackgroundSelectedPointerOver",
                    );
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundSelectedPointerOver",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // PressedSelected
            style.style(nesting().class(":selected").class(":pressed"), |style| {
                style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                    style.dynamic(
                        Border::background_property(),
                        "NavigationViewItemBackgroundSelectedPressed",
                    );
                    Ok(())
                })?;
                style.style(
                    Selectors::or([
                        nesting().template().of::<ContentPresenter>().name("Icon"),
                        nesting()
                            .template()
                            .of::<ContentPresenter>()
                            .name("PART_ContentPresenter"),
                    ]),
                    |style| {
                        style.dynamic(
                            ContentPresenter::foreground_property(),
                            "NavigationViewItemForegroundSelectedPressed",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // DisabledStates: Disabled
            style.style(
                nesting()
                    .class(":disabled")
                    .template()
                    .of::<Border>()
                    .name("LayoutRoot"),
                |style| {
                    style.dynamic(Visual::opacity_property(), "ListViewItemDisabledThemeOpacity");
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    Ok(())
}
