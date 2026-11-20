//! `Controls/GridView.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   dxaml/xcp/dxaml/themes/generic.xaml (the implicit style of GridView)
//!   controls/dev/CommonStyles/GridViewItem_themeresources.xaml (DefaultGridViewItemStyle and the
//!   implicit style; the resources are Resources/GridViewItem_themeresources.xaml)
//!   dxaml/xcp/core/core/elements/ListViewBaseItemChrome.cpp (the chrome that
//!   ListViewItemPresenter draws for a GridViewItem)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The templates of WinUI's GridView and GridViewItem, which FerroUI ports as controls
//! (ferroui-controls, docs/porting/winui-calculator-gaps.md, LV-2); the Fluent theme has no document
//! for them. Translation notes (docs/porting/winui-theme.md, section 4):
//! - GridView: as ListView.xaml (the header and the footer as content presenters around the items
//!   presenter; the scroll modes, rails and the theme transitions left out); the items panel is an
//!   ItemsWrapGrid in rows, which does not virtualize (decision D5).
//! - DefaultGridViewItemStyle: the template is a ListViewItemPresenter, chrome WinUI draws in native
//!   code. The port draws it with elements (decision D6): a Border with the corner radius
//!   GridViewItemCornerRadius in the background of the state, and the selection border
//!   (GridViewItemSelectedBorderThickness of GridViewItemSelectedBorderBrush and its pointer and
//!   pressed brushes) over the content while the item is selected; the disabled opacity is
//!   ListViewItemDisabledThemeOpacity. The check mark of the selected item and the check box of the
//!   multi-select mode (SelectionCheckMarkVisualEnabled, CheckMode, the Check and CheckBox brushes)
//!   are left out, as in DefaultListViewItemStyle (LBI-6); the drag and placeholder brushes and the
//!   reorder hint belong to drag and drop, which is not ported.
//! - The states of the presenter are pseudo-classes: :pointerover, :pressed, :selected, :disabled.
//! - UseSystemFocusVisuals with its margin and brushes is the focus adorner, as in
//!   DefaultListViewItemStyle.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::automation::AccessibilityView;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::GridView;
use ferroui_controls::GridViewItem;
use ferroui_controls::ItemsControl;
use ferroui_controls::ItemsWrapGrid;
use ferroui_controls::ListViewBase;
use ferroui_controls::Panel;
use ferroui_controls::ScrollViewer;
use ferroui_controls::StackPanel;
use ferroui_controls::WrapPanel;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultGridViewStyle", default_grid_view_style);
    resources.control_theme(type_key::<GridView>(), grid_view_theme);
    resources.control_theme("DefaultGridViewItemStyle", default_grid_view_item_style);
    resources.control_theme(type_key::<GridViewItem>(), grid_view_item_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The items panel of `default_grid_view_style`.
fn default_grid_view_style_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(ItemsWrapGrid::new())
        .set(WrapPanel::orientation_property(), Orientation::Horizontal)
        .end()
        .upcast())
}

/// The template of `GridView` of `default_grid_view_style`.
fn default_grid_view_style_template(t: &Template<GridView>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(Border::background_property(), TemplatedControl::background_property())
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
                .name("ScrollViewer")?
                .template_bind(
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::bring_into_view_on_focus_change_property(),
                    ScrollViewer::bring_into_view_on_focus_change_property(),
                )
                .template_bind(
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                )
                .set(
                    AutomationProperties::accessibility_view_property(),
                    AccessibilityView::Raw,
                )
                .content(
                    ContentControl::content_property(),
                    t.make(StackPanel::new())
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                        .children([
                            t.make(ContentPresenter::new())
                                .template_bind(ContentPresenter::content_property(), ListViewBase::header_property())
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    ListViewBase::header_template_property(),
                                )
                                .control(),
                            t.make(ItemsPresenter::new())
                                .name("PART_ItemsPresenter")?
                                .template_bind(
                                    ItemsPresenter::items_panel_property(),
                                    ItemsControl::items_panel_property(),
                                )
                                .control(),
                            t.make(ContentPresenter::new())
                                .template_bind(ContentPresenter::content_property(), ListViewBase::footer_property())
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    ListViewBase::footer_template_property(),
                                )
                                .control(),
                        ]),
                )?,
        )
        .control())
}

/// `DefaultGridViewStyle`: a control theme of `GridView`.
fn default_grid_view_style(theme: &StyleBuilder) -> Result {
    theme.target::<GridView>();
    theme.set(
        TemplatedControl::padding_property(),
        Thickness::new(0.0, 0.0, 0.0, 10.0),
    );
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Disabled,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(ScrollViewer::is_deferred_scrolling_enabled_property(), false);
    theme.set(ScrollViewer::bring_into_view_on_focus_change_property(), true);
    theme.set(ListViewBase::is_swipe_enabled_property(), true);
    theme.set(
        ItemsControl::items_panel_property(),
        items_panel(theme.scope(), default_grid_view_style_items_panel),
    );
    theme.template(default_grid_view_style_template);
    Ok(())
}

/// The default control theme of `GridView`.
fn grid_view_theme(theme: &StyleBuilder) -> Result {
    theme.target::<GridView>();
    theme.based_on("DefaultGridViewStyle")?;
    Ok(())
}

/// The focus adorner of `default_grid_view_item_style`.
fn default_grid_view_item_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-3.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "GridViewItemFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "GridViewItemCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(Border::border_brush_property(), "GridViewItemFocusVisualSecondaryBrush")
                .dynamic(Border::corner_radius_property(), "GridViewItemCornerRadius"),
        )
        .control())
}

/// The template of `GridViewItem` of `default_grid_view_item_style`.
fn default_grid_view_item_style_template(t: &Template<GridViewItem>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .name("Root")?
        .children([
            t.make(Border::new())
                .name("Backplate")?
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .dynamic(Border::corner_radius_property(), "GridViewItemCornerRadius")
                .set(InputElement::is_hit_test_visible_property(), false)
                .set(TemplatedControl::is_template_focus_target_property(), true)
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
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
                    ContentPresenter::horizontal_content_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    ContentPresenter::vertical_content_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                )
                .control(),
            t.make(Border::new())
                .name("SelectionBorder")?
                .dynamic(
                    Border::border_thickness_property(),
                    "GridViewItemSelectedBorderThickness",
                )
                .dynamic(Border::border_brush_property(), "GridViewItemSelectedBorderBrush")
                .dynamic(Border::corner_radius_property(), "GridViewItemCornerRadius")
                .set(InputElement::is_hit_test_visible_property(), false)
                .set(Visual::is_visible_property(), false)
                .control(),
        ])
        .control())
}

/// `DefaultGridViewItemStyle`: a control theme of `GridViewItem`.
fn default_grid_view_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<GridViewItem>();
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::background_property(), "GridViewItemBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "GridViewItemForeground");
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Local,
    );
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 4.0, 4.0));
    theme.dynamic(Layoutable::min_width_property(), "GridViewItemMinWidth");
    theme.dynamic(Layoutable::min_height_property(), "GridViewItemMinHeight");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_grid_view_item_style_focus_adorner)),
    );
    theme.template(default_grid_view_item_style_template);
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "GridViewItemBackgroundPointerOver",
        );
        style.dynamic(
            TemplatedControl::foreground_property(),
            "GridViewItemForegroundPointerOver",
        );
        Ok(())
    })?;
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(TemplatedControl::background_property(), "GridViewItemBackgroundPressed");
        Ok(())
    })?;
    theme.style(nesting().class(":selected"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "GridViewItemBackgroundSelected",
        );
        style.dynamic(
            TemplatedControl::foreground_property(),
            "GridViewItemForegroundSelected",
        );
        style.style(nesting().template().of::<Border>().name("SelectionBorder"), |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        })?;
        Ok(())
    })?;
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "GridViewItemBackgroundSelectedPointerOver",
        );
        style.style(nesting().template().of::<Border>().name("SelectionBorder"), |style| {
            style.dynamic(
                Border::border_brush_property(),
                "GridViewItemSelectedPointerOverBorderBrush",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "GridViewItemBackgroundSelectedPressed",
        );
        style.style(nesting().template().of::<Border>().name("SelectionBorder"), |style| {
            style.dynamic(
                Border::border_brush_property(),
                "GridViewItemSelectedPressedBorderBrush",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(Visual::opacity_property(), "ListViewItemDisabledThemeOpacity");
        Ok(())
    })?;
    theme.style(nesting().class(":selected").class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "GridViewItemBackgroundSelectedDisabled",
        );
        style.style(nesting().template().of::<Border>().name("SelectionBorder"), |style| {
            style.dynamic(
                Border::border_brush_property(),
                "GridViewItemSelectedDisabledBorderBrush",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `GridViewItem`.
fn grid_view_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<GridViewItem>();
    theme.based_on("DefaultGridViewItemStyle")?;
    Ok(())
}
