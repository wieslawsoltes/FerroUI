//! `Controls/RadioButtons.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/RadioButtons/RadioButtons.xaml (DefaultRadioButtonsStyle and the implicit style;
//!   the resources of controls/dev/RadioButtons/RadioButtons_themeresources.xaml are
//!   Resources/RadioButtons_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's RadioButtons is WinUI's RadioButtons (a WinUI control in ferroui-controls,
//!   docs/porting/winui-controls.md), an items control whose containers are RadioButtons, themed by
//!   RadioButton.xaml.
//! - The ItemsRepeater InnerRepeater with the layout ColumnMajorUniformToLargestGridLayout is the
//!   ItemsPresenter PART_ItemsPresenter of the items control, and the layout is its items panel,
//!   the RadioButtonsPanel of FerroUI.Controls.Primitives (a port of the layout), with the spacing
//!   RadioButtonsColumnSpacing and RadioButtonsRowSpacing; the panel reads MaxColumns from the
//!   group (the binding of MaxColumns to the templated parent in the source).
//! - HeaderContentPresenter is PART_HeaderPresenter, the part of the headered selecting items
//!   control the group derives from.
//! - CommonStates: Disabled is :disabled; Normal sets nothing.
//! - TabNavigation Once is KeyboardNavigation.TabNavigation; IsTabStop False is kept.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::Layoutable;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::HeaderedSelectingItemsControl;
use ferroui_controls::primitives::RadioButtonsPanel;
use ferroui_controls::Control;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::RadioButtons;
use ferroui_controls::StackPanel;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultRadioButtonsStyle", default_radio_buttons_style);
    resources.control_theme(type_key::<RadioButtons>(), radio_buttons_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The items panel of `default_radio_buttons_style`.
fn default_radio_buttons_style_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(RadioButtonsPanel::new())
        .dynamic(
            RadioButtonsPanel::column_spacing_property(),
            "RadioButtonsColumnSpacing",
        )
        .dynamic(RadioButtonsPanel::row_spacing_property(), "RadioButtonsRowSpacing")
        .end()
        .upcast())
}

/// The template of `RadioButtons` of `default_radio_buttons_style`.
fn default_radio_buttons_style_template(t: &Template<RadioButtons>) -> Result<Ref<Control>> {
    Ok(t.make(StackPanel::new())
        .children([
            t.make(ContentPresenter::new())
                .name("PART_HeaderPresenter")?
                .template_bind(
                    ContentPresenter::content_property(),
                    HeaderedSelectingItemsControl::header_property(),
                )
                .template_bind(
                    ContentPresenter::content_template_property(),
                    HeaderedSelectingItemsControl::header_template_property(),
                )
                .dynamic(ContentPresenter::foreground_property(), "RadioButtonsHeaderForeground")
                .dynamic(Layoutable::margin_property(), "RadioButtonsTopHeaderMargin")
                .control(),
            t.make(ItemsPresenter::new())
                .name("PART_ItemsPresenter")?
                .template_bind(
                    ItemsPresenter::items_panel_property(),
                    ItemsControl::items_panel_property(),
                )
                .control(),
        ])
        .control())
}

/// `DefaultRadioButtonsStyle`: a control theme of `RadioButtons`.
fn default_radio_buttons_style(theme: &StyleBuilder) -> Result {
    theme.target::<RadioButtons>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.set(
        ItemsControl::items_panel_property(),
        items_panel(theme.scope(), default_radio_buttons_style_items_panel),
    );
    theme.template(default_radio_buttons_style_template);
    // CommonStates
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_HeaderPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "RadioButtonsHeaderForegroundDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `RadioButtons`.
fn radio_buttons_theme(theme: &StyleBuilder) -> Result {
    theme.target::<RadioButtons>();
    theme.based_on("DefaultRadioButtonsStyle")?;
    Ok(())
}
