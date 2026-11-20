//! `Controls/TabbedPage.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/NavigationView/NavigationView.xaml (the top pane of the implicit style of
//!   NavigationView: TopNavArea, TopNavGrid and the content grid in the TopNavigationMinimal
//!   state; the implicit style of NavigationViewItem for the font and the corner radius)
//!   controls/dev/NavigationView/NavigationView_themeresources.xaml
//!   (MUX_NavigationViewItemPresenterStyleWhenOnTopPane, the items of the top pane; the resources
//!   are Resources/NavigationView_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! WinUI has no page of tabbed pages: an application navigates between pages with a NavigationView
//! whose pane is at the top (PaneDisplayMode="Top"). FerroUI's TabbedPage is drawn as that
//! NavigationView: the TabControl PART_TabControl it requires has the keyed theme
//! TabbedPageTabControlTheme (the top pane over the content grid) and its tabs the keyed theme
//! TabbedPageTabItemTheme (the items of the top pane). The page background is
//! ApplicationPageBackgroundThemeBrush (as Controls/ContentPage.xaml).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The ItemsRepeater TopNavMenuItemsHost with a horizontal StackLayout is the ItemsPresenter of
//!   the TabControl with a horizontal StackPanel. The back button placeholder, the pane header,
//!   title, custom content and footer, the overflow button (NavigationViewOverflowButtonStyleWhenPaneOnTop
//!   and its flyout), the auto suggest box, the footer items and the settings item of the top pane
//!   are left out: FerroUI's TabbedPage has none of them.
//! - The selection indicator of a top pane item (the Rectangle SelectionIndicator, 16 x 3, whose
//!   opacity NavigationView animates to the selected item) is the content presenter of the
//!   IndicatorTemplate of FerroUI's TabItem, whose default (set by the theme) is that rectangle;
//!   an IndicatorTemplate of the TabbedPage replaces it. It is shown on the selected item.
//! - The icon states (IconOnLeft, IconOnly, ContentOnly) are the visibility of the icon: the margins
//!   of the header (TopNavigationViewItemContentPresenterMargin with an icon,
//!   TopNavigationViewItemContentOnlyContentPresenterMargin without) are the 12 px left of the
//!   icon box and the 8 px after it, which show with the icon; the margin of the selection
//!   indicator is the one of ContentOnly. IconOnly (an item without content) is left out.
//! - The Rectangle PointerRectangle (collapsed in every state of the top pane style) is left out.
//! - The placements of FerroUI that the top pane has not: at the bottom the pane is under the
//!   content (the border of the content grid on its bottom edge, the indicator at the top of the
//!   item); at the left and at the right the pane is the left pane of NavigationView, with the
//!   items of Controls/TabItem.xaml for those placements and the content grid of the left pane
//!   (NavigationViewContentGridBorderThickness, NavigationViewContentGridCornerRadius, mirrored
//!   at the right).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextWrapping;
use ferroui_base::styling::Selectors;
use ferroui_base::CornerRadius;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::HeaderedContentControl;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::PathIcon;
use ferroui_controls::StackPanel;
use ferroui_controls::TabControl;
use ferroui_controls::TabItem;
use ferroui_controls::TabbedPage;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::data_template;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The NavigationView with its pane at the top.
    resources.control_theme("TabbedPageTabControlTheme", tabbed_page_tab_control_theme);
    // The items of the top pane (MUX_NavigationViewItemPresenterStyleWhenOnTopPane); at the left
    //       and at the right the tabs of the base theme draw the items of the left pane.
    resources.control_theme("TabbedPageTabItemTheme", tabbed_page_tab_item_theme);
    resources.control_theme(type_key::<TabbedPage>(), tabbed_page_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The items panel of `tabbed_page_tab_control_theme`.
fn tabbed_page_tab_control_theme_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(StackPanel::new())
        .set(StackPanel::orientation_property(), Orientation::Horizontal)
        .end()
        .upcast())
}

/// The template of `TabControl` of `tabbed_page_tab_control_theme`.
fn tabbed_page_tab_control_theme_template(t: &Template<TabControl>) -> Result<Ref<Control>> {
    Ok(t.make(DockPanel::new())
        .children([
            t.make(Border::new())
                .name("TopNavArea")?
                .template_bind(DockPanel::dock_property(), TabControl::tab_strip_placement_property())
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .child(
                    Decorator::child_property(),
                    t.make(Border::new())
                        .name("TopNavGrid")?
                        .dynamic(Layoutable::min_height_property(), "NavigationViewTopPaneHeight")
                        .dynamic(Layoutable::margin_property(), "TopNavigationViewTopNavGridMargin")
                        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
                        .child(
                            Decorator::child_property(),
                            t.make(ItemsPresenter::new())
                                .name("PART_ItemsPresenter")?
                                .template_bind(
                                    ItemsPresenter::items_panel_property(),
                                    ItemsControl::items_panel_property(),
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch),
                        ),
                )
                .control(),
            t.make(Border::new())
                .name("ContentGrid")?
                .dynamic(Border::border_brush_property(), "NavigationViewContentGridBorderBrush")
                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                .dynamic(
                    Border::border_thickness_property(),
                    "TopNavigationViewContentGridBorderThickness",
                )
                .dynamic(Border::background_property(), "NavigationViewContentBackground")
                .dynamic(
                    Border::corner_radius_property(),
                    "TopNavigationViewContentGridCornerRadius",
                )
                .dynamic(Layoutable::margin_property(), "TopNavigationViewContentMargin")
                .set(Visual::clip_to_bounds_property(), true)
                .child(
                    Decorator::child_property(),
                    t.make(Panel::new()).children([
                        t.make(ContentPresenter::new())
                            .name("PART_SelectedContentHost2")?
                            .set(
                                ContentPresenter::horizontal_content_alignment_property(),
                                HorizontalAlignment::Stretch,
                            )
                            .set(
                                ContentPresenter::vertical_content_alignment_property(),
                                VerticalAlignment::Stretch,
                            )
                            .set(Visual::is_visible_property(), false)
                            .control(),
                        t.make(ContentPresenter::new())
                            .name("PART_SelectedContentHost")?
                            .set(
                                ContentPresenter::horizontal_content_alignment_property(),
                                HorizontalAlignment::Stretch,
                            )
                            .set(
                                ContentPresenter::vertical_content_alignment_property(),
                                VerticalAlignment::Stretch,
                            )
                            .control(),
                    ]),
                )
                .control(),
        ])
        .control())
}

/// `TabbedPageTabControlTheme`: a control theme of `TabControl`.
fn tabbed_page_tab_control_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TabControl>();
    theme.based_on(type_key::<TabControl>())?;
    theme.dynamic(
        TemplatedControl::background_property(),
        "NavigationViewTopPaneBackground",
    );
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    // The items of the top pane are as wide as their content (the horizontal StackLayout of
    //         TopNavMenuItemsHost), not as wide as each other as the tabs of the base theme; at the left
    //         and at the right the base theme stacks them vertically.
    theme.set(
        ItemsControl::items_panel_property(),
        items_panel(theme.scope(), tabbed_page_tab_control_theme_items_panel),
    );
    theme.template(tabbed_page_tab_control_theme_template);
    // The pane under the content.
    theme.style(
        nesting()
            .property_is(TabControl::tab_strip_placement_property(), Dock::Bottom)
            .template()
            .of::<Border>()
            .name("ContentGrid"),
        |style| {
            style.set(Border::border_thickness_property(), Thickness::new(0.0, 0.0, 0.0, 1.0));
            Ok(())
        },
    )?;
    // The left pane of NavigationView (PaneNotOverlaying: NavigationViewExpandedPaneBackground).
    theme.style(
        nesting().property_is(TabControl::tab_strip_placement_property(), Dock::Left),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "NavigationViewExpandedPaneBackground",
            );
            style.style(nesting().template().of::<Border>().name("TopNavGrid"), |style| {
                style.dynamic(Layoutable::margin_property(), "NavigationViewPaneContentGridMargin");
                Ok(())
            })?;
            style.style(nesting().template().of::<Border>().name("ContentGrid"), |style| {
                style.dynamic(
                    Border::border_thickness_property(),
                    "NavigationViewContentGridBorderThickness",
                );
                style.dynamic(
                    Border::corner_radius_property(),
                    "NavigationViewContentGridCornerRadius",
                );
                style.dynamic(Layoutable::margin_property(), "NavigationViewContentMargin");
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // The same pane at the right, its content grid mirrored.
    theme.style(
        nesting().property_is(TabControl::tab_strip_placement_property(), Dock::Right),
        |style| {
            style.dynamic(
                TemplatedControl::background_property(),
                "NavigationViewExpandedPaneBackground",
            );
            style.style(nesting().template().of::<Border>().name("TopNavGrid"), |style| {
                style.dynamic(Layoutable::margin_property(), "NavigationViewPaneContentGridMargin");
                Ok(())
            })?;
            style.style(nesting().template().of::<Border>().name("ContentGrid"), |style| {
                style.set(Border::border_thickness_property(), Thickness::new(0.0, 1.0, 1.0, 0.0));
                style.set(Border::corner_radius_property(), CornerRadius::new(0.0, 8.0, 0.0, 0.0));
                style.dynamic(Layoutable::margin_property(), "NavigationViewContentMargin");
                Ok(())
            })?;
            Ok(())
        },
    )?;
    Ok(())
}

/// A data template of `tabbed_page_tab_item_theme`.
fn tabbed_page_tab_item_theme_data_template(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Rectangle::new())
        .set(Layoutable::height_property(), 3.0)
        .set(Layoutable::width_property(), 16.0)
        .dynamic(Shape::fill_property(), "NavigationViewSelectionIndicatorForeground")
        .set(Rectangle::radius_x_property(), 2.0)
        .set(Rectangle::radius_y_property(), 2.0)
        .control())
}

/// The template of `TabItem` of `tabbed_page_tab_item_theme`.
fn tabbed_page_tab_item_theme_template(t: &Template<TabItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .static_resource(Layoutable::margin_property(), "NavigationViewItemButtonMargin")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Panel::new()).children([
                t.make(Grid::new())
                    .name("ContentGrid")?
                    .with(|e| {
                        Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*"))?);
                        Ok(())
                    })?
                    .children([
                        t.make(StackPanel::new())
                            .set(StackPanel::orientation_property(), Orientation::Horizontal)
                            .set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 0.0, 0.0))
                            .children([
                                t.make(Viewbox::new())
                                    .name("IconBox")?
                                    .set(Layoutable::height_property(), 16.0)
                                    .set(Layoutable::width_property(), 16.0)
                                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
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
                                t.make(Border::new())
                                    .set(Layoutable::width_property(), 8.0)
                                    .template_bind_with(
                                        Visual::is_visible_property(),
                                        Some(TabItem::icon_property().property()),
                                        Some(ObjectConverters::is_not_null()),
                                        None,
                                        None,
                                    )
                                    .control(),
                            ])
                            .control(),
                        t.make(ContentPresenter::new())
                            .name("PART_ContentPresenter")?
                            .set(Grid::column_property(), 1)
                            .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 12.0, 0.0))
                            .set(ContentPresenter::text_wrapping_property(), TextWrapping::NoWrap)
                            .template_bind(
                                ContentPresenter::foreground_property(),
                                TemplatedControl::foreground_property(),
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
                                Layoutable::horizontal_alignment_property(),
                                ContentControl::horizontal_content_alignment_property(),
                            )
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                            .set(ContentPresenter::recognizes_access_key_property(), true)
                            .control(),
                    ])
                    .control(),
                t.make(ContentPresenter::new())
                    .name("SelectionIndicator")?
                    .set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 12.0, 4.0))
                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                    .set(Visual::opacity_property(), 0.0)
                    .template_bind(
                        ContentPresenter::content_property(),
                        StyledElement::data_context_property(),
                    )
                    .template_bind(
                        ContentPresenter::content_template_property(),
                        TabItem::indicator_template_property(),
                    )
                    .control(),
            ]),
        )
        .control())
}

/// `TabbedPageTabItemTheme`: a control theme of `TabItem`.
fn tabbed_page_tab_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TabItem>();
    theme.based_on(type_key::<TabItem>())?;
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "TopNavigationViewItemForeground",
    );
    theme.dynamic(TemplatedControl::background_property(), "NavigationViewItemBackground");
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        TabItem::indicator_template_property(),
        Some(data_template(theme.scope(), tabbed_page_tab_item_theme_data_template)),
    );
    theme.template(tabbed_page_tab_item_theme_template);
    // PointerStates: PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(
                Border::background_property(),
                "TopNavigationViewItemBackgroundPointerOver",
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
                    "TopNavigationViewItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "TopNavigationViewItemBackgroundPressed");
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
                    "TopNavigationViewItemForegroundPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Selected
    theme.style(nesting().class(":selected"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "TopNavigationViewItemBackgroundSelected");
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
                    "TopNavigationViewItemForegroundSelected",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<ContentPresenter>().name("SelectionIndicator"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // PointerOverSelected
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(
                Border::background_property(),
                "TopNavigationViewItemBackgroundSelectedPointerOver",
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
                    "TopNavigationViewItemForegroundSelectedPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // PressedSelected
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(
                Border::background_property(),
                "TopNavigationViewItemBackgroundSelectedPressed",
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
                    "TopNavigationViewItemForegroundSelectedPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // DisabledStates: Disabled
    theme.style(
        Selectors::or([
            nesting()
                .class(":disabled")
                .template()
                .of::<ContentPresenter>()
                .name("Icon"),
            nesting()
                .class(":disabled")
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
        ]),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TopNavigationViewItemForegroundDisabled",
            );
            Ok(())
        },
    )?;
    // The pane under the content: the indicator at the top of the item.
    theme.style(
        nesting()
            .property_is(TabItem::tab_strip_placement_property(), Some(Dock::Bottom))
            .template()
            .of::<ContentPresenter>()
            .name("SelectionIndicator"),
        |style| {
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
            style.set(Layoutable::margin_property(), Thickness::new(12.0, 4.0, 12.0, 0.0));
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `TabbedPage` of `tabbed_page_theme`.
fn tabbed_page_theme_template(t: &Template<TabbedPage>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .children([t
            .make(TabControl::new())
            .name("PART_TabControl")?
            .static_resource(StyledElement::theme_property(), "TabbedPageTabControlTheme")?
            .static_resource(ItemsControl::item_container_theme_property(), "TabbedPageTabItemTheme")?
            .set(
                Layoutable::horizontal_alignment_property(),
                HorizontalAlignment::Stretch,
            )
            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
            .control()])
        .control())
}

/// The default control theme of `TabbedPage`.
fn tabbed_page_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TabbedPage>();
    theme.dynamic(
        TemplatedControl::background_property(),
        "ApplicationPageBackgroundThemeBrush",
    );
    theme.template(tabbed_page_theme_template);
    Ok(())
}
