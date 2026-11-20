//! `Controls/ListView.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   dxaml/xcp/dxaml/themes/generic.xaml (the implicit style of ListView)
//!   controls/dev/CommonStyles/ListViewItem_themeresources.xaml (the implicit style of
//!   ListViewItem, based on DefaultListViewItemStyle of Controls/ListBoxItem.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The templates of WinUI's ListView and ListViewItem, which FerroUI ports as controls
//! (ferroui-controls, docs/porting/winui-calculator-gaps.md, LV-1); the Fluent theme has no document
//! for them. Translation notes (docs/porting/winui-theme.md, section 4):
//! - The template keeps WinUI's structure, Border > ScrollViewer (the part ScrollViewer) >
//!   ItemsPresenter. FerroUI's ItemsPresenter has no Header and Footer: the header and the footer
//!   are content presenters above and below it, in a vertical StackPanel, with the padding of the
//!   list around the three, as WinUI's presenter lays them out.
//! - The scroll modes and rails of the scroll viewer (HorizontalScrollMode, VerticalScrollMode,
//!   IsHorizontalRailEnabled, IsVerticalRailEnabled) are not FerroUI properties: the scroll bar
//!   visibilities say the same (Disabled horizontally). ZoomMode stays Disabled, the default.
//! - ItemContainerTransitions (AddDeleteThemeTransition, ContentThemeTransition,
//!   ReorderThemeTransition, EntranceThemeTransition) are theme transitions, which the theme does
//!   not port (G-22).
//! - The items are ListViewItems drawn with DefaultListViewItemStyle (decision D6: the chrome of
//!   ListViewItemPresenter drawn with elements, LBI-6 to LBI-9).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
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
use ferroui_controls::ItemsControl;
use ferroui_controls::ItemsStackPanel;
use ferroui_controls::ListView;
use ferroui_controls::ListViewBase;
use ferroui_controls::ListViewItem;
use ferroui_controls::Panel;
use ferroui_controls::ScrollViewer;
use ferroui_controls::StackPanel;
use ferroui_controls::VirtualizingStackPanel;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultListViewStyle", default_list_view_style);
    resources.control_theme(type_key::<ListView>(), list_view_theme);
    resources.control_theme(type_key::<ListViewItem>(), list_view_item_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The items panel of `default_list_view_style`.
fn default_list_view_style_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(ItemsStackPanel::new())
        .set(VirtualizingStackPanel::orientation_property(), Orientation::Vertical)
        .end()
        .upcast())
}

/// The template of `ListView` of `default_list_view_style`.
fn default_list_view_style_template(t: &Template<ListView>) -> Result<Ref<Control>> {
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

/// `DefaultListViewStyle`: a control theme of `ListView`.
fn default_list_view_style(theme: &StyleBuilder) -> Result {
    theme.target::<ListView>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.set(ListViewBase::is_swipe_enabled_property(), true);
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
    theme.set(
        ItemsControl::items_panel_property(),
        items_panel(theme.scope(), default_list_view_style_items_panel),
    );
    theme.template(default_list_view_style_template);
    Ok(())
}

/// The default control theme of `ListView`.
fn list_view_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ListView>();
    theme.based_on("DefaultListViewStyle")?;
    Ok(())
}

/// The default control theme of `ListViewItem`.
fn list_view_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ListViewItem>();
    theme.based_on("DefaultListViewItemStyle")?;
    Ok(())
}
