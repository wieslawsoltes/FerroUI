//! `Controls/TabControl.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TabView/TabView.xaml (DefaultTabViewStyle and the implicit style of TabView, the
//!   implicit style of TabViewListView, TabScrollViewerStyle, TabViewScrollButtonStyle,
//!   TabViewButtonStyle and TabViewCloseButtonStyle; the resources of
//!   controls/dev/TabView/TabView_themeresources.xaml are Resources/TabView_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! FerroUI's TabControl is WinUI's TabView: a strip of tabs over the content of the selected one
//! (TabItem is TabViewItem, Controls/TabItem.xaml). FerroUI's tabs cannot be closed, added,
//! dragged or reordered, and the TabControl has no strip header or footer.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The template of TabView is a Grid of the tab strip (TabContainerGrid) over the content
//!   (TabContentPresenter): a DockPanel with the strip docked at TabStripPlacement. The strip keeps
//!   the column of the left content with its bottom border line and the column of the tabs; the
//!   column of the add tab button, the button (AddButton with TabViewButtonStyle), the presenters of
//!   TabStripHeader and TabStripFooter and the column of the right content with its bottom border
//!   line are left out (FerroUI's TabControl has none). The column of the tabs takes the rest of the
//!   strip (TabView sizes its Auto column in code, as wide as its tabs and at most the width of the
//!   strip): the tabs share the viewport of its scroll viewer, and the footer of the items presenter
//!   fills the rest of the viewport after the last tab with its bottom border line, the line the
//!   right content column of TabView draws there. The content is the PART_SelectedContentHost
//!   FerroUI's TabControl requires, with the PART_SelectedContentHost2 of its page transitions, in a
//!   Border of the border brush, border thickness and corner radius the source binds on
//!   TabContentPresenter.
//! - TabViewListView (the implicit style of the strip) is not a control of its own: its template
//!   (a Border, the ScrollViewer with TabScrollViewerStyle, the ItemsPresenter with a 4 px wide
//!   header and footer that hold the bottom border lines beside the first and the last tab) is
//!   written into the template, and its ItemsStackPanel is TabViewEqualWidthPanel, a primitive of
//!   the theme (Primitives/tab_view_equal_width_panel.rs), which gives every tab the width TabView
//!   computes in its default TabWidthMode Equal (TabView::UpdateTabWidths: the width of the strip
//!   for its tabs divided by their number, between TabViewItemMinWidth and TabViewItemMaxWidth; tabs
//!   that do not fit at the minimum width scroll) and which TabView sets as the Width of every
//!   TabViewItem in code. The ItemsPresenter is docked left of the footer, which fills the rest of
//!   the viewport. The bottom border lines of the header and footer are named
//!   HeaderBottomBorderLine and FooterBottomBorderLine (one name scope holds them with the lines of
//!   TabView). The setters of
//!   the list view are written on its elements (VerticalAlignment, IsTabStop, the scroll bar
//!   visibilities, BringIntoViewOnFocusChange); its TabNavigation="Once" is the tab navigation
//!   FerroUI's TabControl sets on its panel. ItemContainerTransitions (AddDeleteThemeTransition,
//!   ContentThemeTransition, ReorderThemeTransition, EntranceThemeTransition) have no counterpart.
//! - The visual states that TabView sets from code (SingleBottomBorderLine of TabView,
//!   LeftBottomBorderLineShort, RightBottomBorderLineShort and NoBottomBorderLine of the list
//!   view and of TabScrollViewerStyle) are left out: the lines keep the normal state.
//! - TabScrollViewerStyle is a control theme of ScrollViewer. Its scroll buttons are shown by
//!   ComputedHorizontalScrollBarVisibility in the source and enabled by the code of TabView at the
//!   ends of the scroll range: here both are MenuScrollingVisibilityConverter of the offset, extent
//!   and viewport (shown while the tabs overflow, enabled until the end of the range). The
//!   buttons scroll with the LineLeft and LineRight commands of the scroll viewer (TabView scrolls
//!   50 px per click in code). The FontIcon glyphs EDD9 and EDDA of the buttons are paths of the
//!   glyph geometries (G-9), 8 px high (TabViewItemScrollButonFontSize), filled with the foreground
//!   of the button, which the states of TabViewScrollButtonStyle set with the one of its content
//!   presenter.
//! - The placements of FerroUI that TabView has not: at the bottom the strip is the strip of the
//!   top turned upside down (its tabs turn their header back, Controls/TabItem.xaml); at the left
//!   and at the right the tabs are stacked vertically without the bottom border lines, and the
//!   tabs are drawn as the items of the left pane of NavigationView (Controls/TabItem.xaml), 4 px
//!   under the top of the strip (the border of RootSplitView and NavigationViewPaneContentGridMargin),
//!   beside the content grid of NavigationView (NavigationViewContentBackground,
//!   NavigationViewContentGridBorderBrush, NavigationViewContentGridBorderThickness and
//!   NavigationViewContentGridCornerRadius, mirrored at the right).
//! - KeyboardAcceleratorPlacementMode has no counterpart.

#![allow(unused_imports, clippy::all)]

use crate::primitives::TabViewEqualWidthPanel;
use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::MultiBinding;
use ferroui_base::input::gesture_recognizers::ScrollGestureRecognizer;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Stretch;
use ferroui_base::styling::Selectors;
use ferroui_base::CornerRadius;
use ferroui_base::RelativePoint;
use ferroui_base::Size;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Vector;
use ferroui_base::Visual;
use ferroui_controls::converters::MenuScrollingVisibilityConverter;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinition;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::RepeatButton;
use ferroui_controls::ScrollViewer;
use ferroui_controls::StackPanel;
use ferroui_controls::TabControl;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("TabViewScrollButtonStyle", tab_view_scroll_button_style);
    // The style of the add tab button (FerroUI's TabControl has none).
    resources.control_theme("TabViewButtonStyle", tab_view_button_style);
    // The style of the close button of a tab (FerroUI's tabs cannot be closed).
    resources.control_theme("TabViewCloseButtonStyle", tab_view_close_button_style);
    resources.control_theme("TabScrollViewerStyle", tab_scroll_viewer_style);
    resources.control_theme("DefaultTabViewStyle", default_tab_view_style);
    resources.control_theme(type_key::<TabControl>(), tab_control_theme);
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

/// The template of `RepeatButton` of `tab_view_scroll_button_style`.
fn tab_view_scroll_button_style_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(
            ContentPresenter::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(
            ContentPresenter::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(
            ContentPresenter::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .template_bind(
            ContentPresenter::foreground_property(),
            TemplatedControl::foreground_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(
            ContentPresenter::font_size_property(),
            TemplatedControl::font_size_property(),
        )
        .template_bind(
            ContentPresenter::font_family_property(),
            TemplatedControl::font_family_property(),
        )
        .template_bind(
            ContentPresenter::font_weight_property(),
            TemplatedControl::font_weight_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .control())
}

/// `TabViewScrollButtonStyle`: a control theme of `RepeatButton`.
fn tab_view_scroll_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.based_on("DefaultRepeatButtonStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "TabViewScrollButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "TabViewScrollButtonForeground");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.dynamic(TemplatedControl::font_size_property(), "TabViewItemScrollButonFontSize");
    theme.dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily");
    theme.dynamic(Layoutable::width_property(), "TabViewItemScrollButtonWidth");
    theme.dynamic(Layoutable::height_property(), "TabViewItemScrollButtonHeight");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TabViewButtonBorderThickness",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "TabViewScrollButtonBorderBrush",
    );
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    // FocusVisualMargin="-3": the focus adorner of DefaultRepeatButtonStyle.
    theme.template(tab_view_scroll_button_style_template);
    // The foreground of the states is also the foreground of the button, which the glyph of
    // its content follows.
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "TabViewScrollButtonForegroundPointerOver",
        );
        Ok(())
    })?;
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "TabViewScrollButtonForegroundPressed",
        );
        Ok(())
    })?;
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "TabViewScrollButtonForegroundDisabled",
        );
        Ok(())
    })?;
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TabViewScrollButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TabViewScrollButtonForegroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TabViewScrollButtonBorderBrushPointerOver",
            );
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TabViewScrollButtonBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TabViewScrollButtonForegroundPressed",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TabViewScrollButtonBorderBrushPressed",
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TabViewScrollButtonBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TabViewScrollButtonForegroundDisabled",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TabViewScrollButtonBorderBrushDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Button` of `tab_view_button_style`.
fn tab_view_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(
            ContentPresenter::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .template_bind(
            ContentPresenter::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(
            ContentPresenter::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(
            ContentPresenter::foreground_property(),
            TemplatedControl::foreground_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(
            ContentPresenter::font_size_property(),
            TemplatedControl::font_size_property(),
        )
        .template_bind(
            ContentPresenter::font_family_property(),
            TemplatedControl::font_family_property(),
        )
        .template_bind(
            ContentPresenter::font_weight_property(),
            TemplatedControl::font_weight_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .control())
}

/// `TabViewButtonStyle`: a control theme of `Button`.
fn tab_view_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "TabViewButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "TabViewButtonForeground");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.dynamic(TemplatedControl::font_size_property(), "TabViewItemAddButtonFontSize");
    theme.dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily");
    theme.dynamic(Layoutable::width_property(), "TabViewItemAddButtonWidth");
    theme.dynamic(Layoutable::height_property(), "TabViewItemAddButtonHeight");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TabViewButtonBorderThickness",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "TabViewButtonBorderBrush");
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    // FocusVisualMargin="-3": the focus adorner of DefaultButtonStyle.
    theme.template(tab_view_button_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TabViewButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TabViewButtonForegroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TabViewButtonBorderBrushPointerOver",
            );
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TabViewButtonBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TabViewButtonForegroundPressed",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TabViewButtonBorderBrushPressed",
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TabViewButtonBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TabViewButtonForegroundDisabled",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TabViewButtonBorderBrushDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Button` of `tab_view_close_button_style`.
fn tab_view_close_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(
            ContentPresenter::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(
            ContentPresenter::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(
            ContentPresenter::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(
            ContentPresenter::foreground_property(),
            TemplatedControl::foreground_property(),
        )
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .control())
}

/// `TabViewCloseButtonStyle`: a control theme of `Button`.
fn tab_view_close_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily");
    theme.dynamic(TemplatedControl::font_size_property(), "TabViewItemHeaderCloseFontSize");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.dynamic(Layoutable::width_property(), "TabViewItemHeaderCloseButtonWidth");
    theme.dynamic(Layoutable::height_property(), "TabViewItemHeaderCloseButtonHeight");
    theme.dynamic(
        TemplatedControl::background_property(),
        "TabViewItemHeaderCloseButtonBackground",
    );
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "TabViewItemHeaderCloseButtonForeground",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "TabViewItemHeaderCloseButtonBorderBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TabViewItemHeaderCloseButtonBorderThickness",
    );
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    // FocusVisualMargin="-3": the focus adorner of DefaultButtonStyle.
    theme.template(tab_view_close_button_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TabViewItemHeaderCloseButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TabViewItemHeaderCloseButtonForegroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TabViewItemHeaderCloseButtonBorderBrushPointerOver",
            );
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TabViewItemHeaderCloseButtonBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TabViewItemHeaderCloseButtonForegroundPressed",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TabViewItemHeaderCloseButtonBorderBrushPressed",
            );
            Ok(())
        },
    )?;
    // The source has no Disabled state: the brushes of the control stay.
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.template_bind(
                ContentPresenter::background_property(),
                TemplatedControl::background_property(),
            );
            style.template_bind(
                ContentPresenter::foreground_property(),
                TemplatedControl::foreground_property(),
            );
            style.template_bind(
                ContentPresenter::border_brush_property(),
                TemplatedControl::border_brush_property(),
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `ScrollViewer` of `tab_scroll_viewer_style`.
fn tab_scroll_viewer_style_template(t: &Template<ScrollViewer>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    let items = Grid::column_definitions(e);
                    items.add(
                        t.make(ColumnDefinition::new())
                            .set(ColumnDefinition::width_property(), GridLength::auto())
                            .set(ColumnDefinition::min_width_property(), 2.0)
                            .end()
                            .into(),
                    );
                    items.add(
                        t.make(ColumnDefinition::new())
                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
                            .end()
                            .into(),
                    );
                    items.add(
                        t.make(ColumnDefinition::new())
                            .set(ColumnDefinition::width_property(), GridLength::auto())
                            .end()
                            .into(),
                    );
                    Ok(())
                })?
                .children([
                    t.make(Border::new())
                        .name("LeftBottomBorderLine")?
                        .dynamic(Border::background_property(), "TabViewBorderBrush")
                        .set(Layoutable::height_property(), 1.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                        .control(),
                    t.make(Border::new())
                        .name("RightBottomBorderLine")?
                        .set(Grid::column_property(), 2)
                        .dynamic(Border::background_property(), "TabViewBorderBrush")
                        .set(Layoutable::height_property(), 1.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                        .control(),
                    t.make(Border::new())
                        .name("ScrollDecreaseButtonContainer")?
                        .dynamic(
                            Decorator::padding_property(),
                            "TabViewItemLeftScrollButtonContainerPadding",
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                        .bind(Visual::is_visible_property(), {
                            let binding = MultiBinding::new()
                                .with_converter_value(Some(MenuScrollingVisibilityConverter::instance()))
                                .with_converter_parameter(value("-1".to_string()));
                            let bindings = binding.bindings();
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ScrollViewer::horizontal_scroll_bar_visibility_property())
                                    .binding(),
                            );
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ScrollViewer::offset_property())
                                    .member::<Vector>("X")
                                    .binding(),
                            );
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ScrollViewer::extent_property())
                                    .member::<Size>("Width")
                                    .binding(),
                            );
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ScrollViewer::viewport_property())
                                    .member::<Size>("Width")
                                    .binding(),
                            );
                            binding
                        })
                        .child(
                            Decorator::child_property(),
                            t.make(RepeatButton::new())
                                .name("ScrollDecreaseButton")?
                                .static_resource(StyledElement::theme_property(), "TabViewScrollButtonStyle")?
                                .set(InputElement::is_tab_stop_property(), false)
                                .set(RepeatButton::delay_property(), 50)
                                .set(RepeatButton::interval_property(), 100)
                                .bind(
                                    Button::command_property(),
                                    path()
                                        .templated_parent()
                                        .method::<Ref<ScrollViewer>>("LineLeft", |this| {
                                            let _ = this.line_left();
                                        })
                                        .binding(),
                                )
                                .bind(InputElement::is_enabled_property(), {
                                    let binding = MultiBinding::new()
                                        .with_converter_value(Some(MenuScrollingVisibilityConverter::instance()))
                                        .with_converter_parameter(value("0".to_string()));
                                    let bindings = binding.bindings();
                                    bindings.add(
                                        path()
                                            .templated_parent()
                                            .prop(ScrollViewer::horizontal_scroll_bar_visibility_property())
                                            .binding(),
                                    );
                                    bindings.add(
                                        path()
                                            .templated_parent()
                                            .prop(ScrollViewer::offset_property())
                                            .member::<Vector>("X")
                                            .binding(),
                                    );
                                    bindings.add(
                                        path()
                                            .templated_parent()
                                            .prop(ScrollViewer::extent_property())
                                            .member::<Size>("Width")
                                            .binding(),
                                    );
                                    bindings.add(
                                        path()
                                            .templated_parent()
                                            .prop(ScrollViewer::viewport_property())
                                            .member::<Size>("Width")
                                            .binding(),
                                    );
                                    binding
                                })
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Path::new())
                                        .static_resource(Path::data_property(), "GlyphGeometryEDD9")?
                                        .bind(
                                            Shape::fill_property(),
                                            path()
                                                .ancestor_of::<RepeatButton>(0)
                                                .prop(TemplatedControl::foreground_property())
                                                .binding(),
                                        )
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .static_resource(
                                            Layoutable::height_property(),
                                            "TabViewItemScrollButonFontSize",
                                        )?,
                                )?,
                        )
                        .control(),
                    t.make(ScrollContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(Grid::column_property(), 1)
                        .set(ContentPresenter::padding_property(), Thickness::new(1.0, 0.0, 0.0, 0.0))
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
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
                    t.make(Border::new())
                        .name("ScrollIncreaseButtonContainer")?
                        .set(Grid::column_property(), 2)
                        .dynamic(
                            Decorator::padding_property(),
                            "TabViewItemRightScrollButtonContainerPadding",
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                        .bind(Visual::is_visible_property(), {
                            let binding = MultiBinding::new()
                                .with_converter_value(Some(MenuScrollingVisibilityConverter::instance()))
                                .with_converter_parameter(value("-1".to_string()));
                            let bindings = binding.bindings();
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ScrollViewer::horizontal_scroll_bar_visibility_property())
                                    .binding(),
                            );
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ScrollViewer::offset_property())
                                    .member::<Vector>("X")
                                    .binding(),
                            );
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ScrollViewer::extent_property())
                                    .member::<Size>("Width")
                                    .binding(),
                            );
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ScrollViewer::viewport_property())
                                    .member::<Size>("Width")
                                    .binding(),
                            );
                            binding
                        })
                        .child(
                            Decorator::child_property(),
                            t.make(RepeatButton::new())
                                .name("ScrollIncreaseButton")?
                                .static_resource(StyledElement::theme_property(), "TabViewScrollButtonStyle")?
                                .set(InputElement::is_tab_stop_property(), false)
                                .set(RepeatButton::delay_property(), 50)
                                .set(RepeatButton::interval_property(), 100)
                                .bind(
                                    Button::command_property(),
                                    path()
                                        .templated_parent()
                                        .method::<Ref<ScrollViewer>>("LineRight", |this| {
                                            let _ = this.line_right();
                                        })
                                        .binding(),
                                )
                                .bind(InputElement::is_enabled_property(), {
                                    let binding = MultiBinding::new()
                                        .with_converter_value(Some(MenuScrollingVisibilityConverter::instance()))
                                        .with_converter_parameter(value("100".to_string()));
                                    let bindings = binding.bindings();
                                    bindings.add(
                                        path()
                                            .templated_parent()
                                            .prop(ScrollViewer::horizontal_scroll_bar_visibility_property())
                                            .binding(),
                                    );
                                    bindings.add(
                                        path()
                                            .templated_parent()
                                            .prop(ScrollViewer::offset_property())
                                            .member::<Vector>("X")
                                            .binding(),
                                    );
                                    bindings.add(
                                        path()
                                            .templated_parent()
                                            .prop(ScrollViewer::extent_property())
                                            .member::<Size>("Width")
                                            .binding(),
                                    );
                                    bindings.add(
                                        path()
                                            .templated_parent()
                                            .prop(ScrollViewer::viewport_property())
                                            .member::<Size>("Width")
                                            .binding(),
                                    );
                                    binding
                                })
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Path::new())
                                        .static_resource(Path::data_property(), "GlyphGeometryEDDA")?
                                        .bind(
                                            Shape::fill_property(),
                                            path()
                                                .ancestor_of::<RepeatButton>(0)
                                                .prop(TemplatedControl::foreground_property())
                                                .binding(),
                                        )
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .static_resource(
                                            Layoutable::height_property(),
                                            "TabViewItemScrollButonFontSize",
                                        )?,
                                )?,
                        )
                        .control(),
                ]),
        )
        .control())
}

/// `TabScrollViewerStyle`: a control theme of `ScrollViewer`.
fn tab_scroll_viewer_style(theme: &StyleBuilder) -> Result {
    theme.target::<ScrollViewer>();
    theme.template(tab_scroll_viewer_style_template);
    Ok(())
}

/// The items panel of `default_tab_view_style`.
fn default_tab_view_style_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(TabViewEqualWidthPanel::new())
        .dynamic(TabViewEqualWidthPanel::min_item_width_property(), "TabViewItemMinWidth")
        .dynamic(TabViewEqualWidthPanel::max_item_width_property(), "TabViewItemMaxWidth")
        .end()
        .upcast())
}

/// The template of `TabControl` of `default_tab_view_style`.
fn default_tab_view_style_template(t: &Template<TabControl>) -> Result<Ref<Control>> {
    Ok(t.make(DockPanel::new())
        .children([
            t.make(Border::new())
                .name("TabContainerGrid")?
                .template_bind(DockPanel::dock_property(), TabControl::tab_strip_placement_property())
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
                )
                .child(
                    Decorator::child_property(),
                    t.make(Grid::new())
                        // The columns LeftContentColumn and TabColumn; the tabs take the rest of the strip.
                        .with(|e| {
                            let items = Grid::column_definitions(e);
                            items.add(
                                t.make(ColumnDefinition::new())
                                    .set(ColumnDefinition::width_property(), GridLength::auto())
                                    .set(ColumnDefinition::min_width_property(), 2.0)
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
                        // TabViewListView
                        .children([
                            t.make(Border::new())
                                .name("LeftBottomBorderLine")?
                                .dynamic(Border::background_property(), "TabViewBorderBrush")
                                .set(Layoutable::height_property(), 1.0)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                                .control(),
                            t.make(Border::new())
                                .name("TabListView")?
                                .set(Grid::column_property(), 1)
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                .child(
                                    Decorator::child_property(),
                                    t.make(ScrollViewer::new())
                                        .name("ScrollViewer")?
                                        .static_resource(StyledElement::theme_property(), "TabScrollViewerStyle")?
                                        .set(
                                            ScrollViewer::horizontal_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Auto,
                                        )
                                        .set(
                                            ScrollViewer::vertical_scroll_bar_visibility_property(),
                                            ScrollBarVisibility::Disabled,
                                        )
                                        .set(ScrollViewer::bring_into_view_on_focus_change_property(), true)
                                        .set(InputElement::is_tab_stop_property(), false)
                                        .content(
                                            ContentControl::content_property(),
                                            t.make(DockPanel::new())
                                                .name("TabsItemsPresenter")?
                                                .template_bind(
                                                    Layoutable::margin_property(),
                                                    TemplatedControl::padding_property(),
                                                )
                                                // The footer, 4 px after the last tab, and the bottom border line of the right
                                                // content column of TabView in the rest of the viewport.
                                                .children([
                                                    t.make(Panel::new())
                                                        .name("TabsItemsPresenterHeader")?
                                                        .set(DockPanel::dock_property(), Dock::Left)
                                                        .set(Layoutable::width_property(), 4.0)
                                                        .children([t
                                                            .make(Border::new())
                                                            .name("HeaderBottomBorderLine")?
                                                            .dynamic(
                                                                Border::background_property(),
                                                                "TabViewBorderBrush",
                                                            )
                                                            .set(Layoutable::width_property(), 4.0)
                                                            .set(Layoutable::height_property(), 1.0)
                                                            .set(
                                                                Layoutable::vertical_alignment_property(),
                                                                VerticalAlignment::Bottom,
                                                            )
                                                            .set(
                                                                Layoutable::horizontal_alignment_property(),
                                                                HorizontalAlignment::Left,
                                                            )
                                                            .control()])
                                                        .control(),
                                                    t.make(ItemsPresenter::new())
                                                        .name("PART_ItemsPresenter")?
                                                        .set(DockPanel::dock_property(), Dock::Left)
                                                        .template_bind(
                                                            ItemsPresenter::items_panel_property(),
                                                            ItemsControl::items_panel_property(),
                                                        )
                                                        .control(),
                                                    t.make(Panel::new())
                                                        .name("TabsItemsPresenterFooter")?
                                                        .set(Layoutable::min_width_property(), 4.0)
                                                        .children([t
                                                            .make(Border::new())
                                                            .name("FooterBottomBorderLine")?
                                                            .dynamic(
                                                                Border::background_property(),
                                                                "TabViewBorderBrush",
                                                            )
                                                            .set(Layoutable::height_property(), 1.0)
                                                            .set(
                                                                Layoutable::vertical_alignment_property(),
                                                                VerticalAlignment::Bottom,
                                                            )
                                                            .control()])
                                                        .control(),
                                                ]),
                                        )?,
                                )
                                .control(),
                        ]),
                )
                .control(),
            t.make(Border::new())
                .name("TabContentPresenter")?
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
                    t.make(Panel::new())
                        .set(Visual::clip_to_bounds_property(), true)
                        .children([
                            t.make(ContentPresenter::new())
                                .name("PART_SelectedContentHost2")?
                                .template_bind(
                                    ContentPresenter::horizontal_content_alignment_property(),
                                    TabControl::horizontal_content_alignment_property(),
                                )
                                .template_bind(
                                    ContentPresenter::vertical_content_alignment_property(),
                                    TabControl::vertical_content_alignment_property(),
                                )
                                .set(Visual::is_visible_property(), false)
                                .control(),
                            t.make(ContentPresenter::new())
                                .name("PART_SelectedContentHost")?
                                .template_bind(
                                    ContentPresenter::horizontal_content_alignment_property(),
                                    TabControl::horizontal_content_alignment_property(),
                                )
                                .template_bind(
                                    ContentPresenter::vertical_content_alignment_property(),
                                    TabControl::vertical_content_alignment_property(),
                                )
                                .control(),
                        ]),
                )
                .control(),
        ])
        .control())
}

/// The items panel of `default_tab_view_style`.
fn default_tab_view_style_items_panel_2(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(StackPanel::new())
        .set(StackPanel::orientation_property(), Orientation::Vertical)
        .end()
        .upcast())
}

/// The items panel of `default_tab_view_style`.
fn default_tab_view_style_items_panel_3(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(StackPanel::new())
        .set(StackPanel::orientation_property(), Orientation::Vertical)
        .end()
        .upcast())
}

/// `DefaultTabViewStyle`: a control theme of `TabControl`.
fn default_tab_view_style(theme: &StyleBuilder) -> Result {
    theme.target::<TabControl>();
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.dynamic(TemplatedControl::padding_property(), "TabViewHeaderPadding");
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(TemplatedControl::background_property(), "TabViewBackground");
    // The ItemsStackPanel of TabViewListView, with the width TabView gives every tab in its
    // TabWidthMode Equal (the default).
    theme.set(
        ItemsControl::items_panel_property(),
        items_panel(theme.scope(), default_tab_view_style_items_panel),
    );
    theme.template(default_tab_view_style_template);
    // The strip at the bottom: the strip of the top turned upside down.
    theme.style(
        nesting()
            .property_is(TabControl::tab_strip_placement_property(), Dock::Bottom)
            .template()
            .of::<Border>()
            .name("TabContainerGrid"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scaleY(-1)"))?),
            );
            Ok(())
        },
    )?;
    // The strip at the left or the right: the tabs stacked vertically, scrolled vertically,
    // without the bottom border lines.
    theme.style(
        nesting().property_is(TabControl::tab_strip_placement_property(), Dock::Left),
        |style| {
            // The tabs stacked vertically (a StackPanel as the items panel of the strip, which the
            // style below turns vertical also when a derived theme sets a horizontal one).
            style.set(
                ItemsControl::items_panel_property(),
                items_panel(style.scope(), default_tab_view_style_items_panel_2),
            );
            // The pane of NavigationView: its items 3 px under the top of the pane
            // (NavigationViewPaneContentGridMargin) and the 1 px border of RootSplitView.
            style.set(TemplatedControl::padding_property(), Thickness::new(0.0, 4.0, 0.0, 0.0));
            // The content is the content grid of NavigationView beside its pane
            // (NavigationViewContentBackground, the border and the corner of ContentGrid).
            style.style(
                nesting().template().of::<Border>().name("TabContentPresenter"),
                |style| {
                    style.dynamic(Border::background_property(), "NavigationViewContentBackground");
                    style.dynamic(Border::border_brush_property(), "NavigationViewContentGridBorderBrush");
                    style.dynamic(
                        Border::border_thickness_property(),
                        "NavigationViewContentGridBorderThickness",
                    );
                    style.dynamic(
                        Border::corner_radius_property(),
                        "NavigationViewContentGridCornerRadius",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .template()
                    .of::<ItemsPresenter>()
                    .name("PART_ItemsPresenter")
                    .child()
                    .of::<StackPanel>(),
                |style| {
                    style.set(StackPanel::orientation_property(), Orientation::Vertical);
                    Ok(())
                },
            )?;
            style.style(
                nesting().template().of::<ScrollViewer>().name("ScrollViewer"),
                |style| {
                    style.set(
                        ScrollViewer::horizontal_scroll_bar_visibility_property(),
                        ScrollBarVisibility::Disabled,
                    );
                    style.set(
                        ScrollViewer::vertical_scroll_bar_visibility_property(),
                        ScrollBarVisibility::Auto,
                    );
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Border>().name("TabListView"), |style| {
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
                Ok(())
            })?;
            style.style(
                Selectors::or([
                    nesting().template().of::<Border>().name("LeftBottomBorderLine"),
                    nesting().template().of::<Panel>().name("TabsItemsPresenterHeader"),
                    nesting().template().of::<Panel>().name("TabsItemsPresenterFooter"),
                ]),
                |style| {
                    style.set(Visual::is_visible_property(), false);
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    theme.style(
        nesting().property_is(TabControl::tab_strip_placement_property(), Dock::Right),
        |style| {
            // The tabs stacked vertically, as at the left.
            style.set(
                ItemsControl::items_panel_property(),
                items_panel(style.scope(), default_tab_view_style_items_panel_3),
            );
            // The pane of NavigationView: its items 3 px under the top of the pane
            // (NavigationViewPaneContentGridMargin) and the 1 px border of RootSplitView.
            style.set(TemplatedControl::padding_property(), Thickness::new(0.0, 4.0, 0.0, 0.0));
            // The content is the content grid of NavigationView beside its pane
            // (NavigationViewContentBackground, the border and the corner of ContentGrid, mirrored).
            style.style(
                nesting().template().of::<Border>().name("TabContentPresenter"),
                |style| {
                    style.dynamic(Border::background_property(), "NavigationViewContentBackground");
                    style.dynamic(Border::border_brush_property(), "NavigationViewContentGridBorderBrush");
                    style.set(Border::border_thickness_property(), Thickness::new(0.0, 1.0, 1.0, 0.0));
                    style.set(Border::corner_radius_property(), CornerRadius::new(0.0, 8.0, 0.0, 0.0));
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .template()
                    .of::<ItemsPresenter>()
                    .name("PART_ItemsPresenter")
                    .child()
                    .of::<StackPanel>(),
                |style| {
                    style.set(StackPanel::orientation_property(), Orientation::Vertical);
                    Ok(())
                },
            )?;
            style.style(
                nesting().template().of::<ScrollViewer>().name("ScrollViewer"),
                |style| {
                    style.set(
                        ScrollViewer::horizontal_scroll_bar_visibility_property(),
                        ScrollBarVisibility::Disabled,
                    );
                    style.set(
                        ScrollViewer::vertical_scroll_bar_visibility_property(),
                        ScrollBarVisibility::Auto,
                    );
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Border>().name("TabListView"), |style| {
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
                Ok(())
            })?;
            style.style(
                Selectors::or([
                    nesting().template().of::<Border>().name("LeftBottomBorderLine"),
                    nesting().template().of::<Panel>().name("TabsItemsPresenterHeader"),
                    nesting().template().of::<Panel>().name("TabsItemsPresenterFooter"),
                ]),
                |style| {
                    style.set(Visual::is_visible_property(), false);
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `TabControl`.
fn tab_control_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TabControl>();
    theme.based_on("DefaultTabViewStyle")?;
    Ok(())
}
