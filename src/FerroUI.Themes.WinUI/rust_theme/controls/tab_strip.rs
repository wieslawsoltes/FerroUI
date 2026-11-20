//! `Controls/TabStrip.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/SelectorBar/SelectorBar.xaml (DefaultSelectorBarStyle and the implicit style of
//!   SelectorBar; the resources of controls/dev/SelectorBar/SelectorBar_themeresources.xaml are
//!   Resources/SelectorBar_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! FerroUI's TabStrip is the strip of selectable items WinUI calls SelectorBar (TabStripItem is
//! SelectorBarItem, Controls/TabStripItem.xaml).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The Grid of the template has a border brush, a background and a corner radius: it is a
//!   Border (without a border thickness, as the Grid of the source has none).
//! - The ItemsView PART_ItemsView with a horizontal StackLayout is the ItemsPresenter
//!   PART_ItemsPresenter of FerroUI's items controls with a horizontal StackPanel as the items
//!   panel; the padding of the items view is the margin of the presenter. Its MaxWidth and
//!   MaxHeight bindings are left out (the control's own limits constrain it).
//! - The RepositionThemeTransition of the children of the Grid has no counterpart.
//! - TabNavigation="Once" is KeyboardNavigation.TabNavigation; IsTabStop is kept.

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
use ferroui_base::media::BackgroundSizing;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::TabStrip;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::StackPanel;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultSelectorBarStyle", default_selector_bar_style);
    resources.control_theme(type_key::<TabStrip>(), tab_strip_theme);
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

/// The items panel of `default_selector_bar_style`.
fn default_selector_bar_style_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(StackPanel::new())
        .set(StackPanel::orientation_property(), Orientation::Horizontal)
        .end()
        .upcast())
}

/// The template of `TabStrip` of `default_selector_bar_style`.
fn default_selector_bar_style_template(t: &Template<TabStrip>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(ItemsPresenter::new())
                .name("PART_ItemsPresenter")?
                .template_bind(
                    ItemsPresenter::items_panel_property(),
                    ItemsControl::items_panel_property(),
                )
                .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property()),
        )
        .control())
}

/// `DefaultSelectorBarStyle`: a control theme of `TabStrip`.
fn default_selector_bar_style(theme: &StyleBuilder) -> Result {
    theme.target::<TabStrip>();
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.dynamic(TemplatedControl::padding_property(), "SelectorBarPadding");
    theme.set(
        ItemsControl::items_panel_property(),
        items_panel(theme.scope(), default_selector_bar_style_items_panel),
    );
    theme.template(default_selector_bar_style_template);
    Ok(())
}

/// The default control theme of `TabStrip`.
fn tab_strip_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TabStrip>();
    theme.based_on("DefaultSelectorBarStyle")?;
    Ok(())
}
