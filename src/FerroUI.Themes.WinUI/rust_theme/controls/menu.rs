//! `Controls/Menu.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/MenuBar/MenuBar.xaml (the style DefaultMenuBarStyle)
//!   controls/dev/MenuBar/MenuBarItem.xaml (the implicit style of MenuBarItem)
//!   controls/dev/MenuBar/MenuBar_themeresources.xaml (the resources, Resources/MenuBar_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's Menu is WinUI's MenuBar; its items are MenuItem, which WinUI's MenuBarItem is for the
//!   top level. The implicit style of MenuBarItem is the keyed theme DefaultMenuBarItemStyle, the
//!   ItemContainerTheme of the menu; the items of the open menus have the default theme of MenuItem
//!   (Controls/MenuItem.xaml).
//! - MenuBar: the ItemsControl ContentRoot of the source is the items presenter PART_ItemsPresenter
//!   (FerroUI's menu hosts its items itself), left aligned in the layout root; the horizontal
//!   StackPanel of the source is the default items panel of FerroUI's Menu. Grid LayoutRoot is a
//!   Border that also draws the border, corner radius and padding of the menu, which FerroUI's
//!   Menu honours in its other themes (WinUI's MenuBar template has none of them).
//! - MenuBarItem: the Button ContentButton of the source, which shows the Title and opens the
//!   flyout, is the content presenter of the Header, PART_HeaderPresenter (the part a headered
//!   items control hosts its header in, with FerroUI's access keys): the menu item itself handles
//!   the pointer, so a button inside it would take its clicks. The
//!   content presenter has the setters and the foreground states of that button
//!   (DefaultButtonStyle with its background resources made transparent by the template:
//!   ButtonForegroundPointerOver, ButtonForegroundPressed, ButtonForegroundDisabled). Grid
//!   ContentRoot is a Border (it has a corner radius).
//! - The flyout of a MenuBarItem (a MenuFlyout) is the popup PART_Popup with the template of
//!   MenuFlyoutPresenter, below the item and aligned to its left edge (MenuBarItem::ShowMenuFlyout
//!   shows it at the bottom left corner of the button). The shadow of the menu flyout (a ThemeShadow
//!   at a Translation.Z of 32) is a ThemeShadow (the shared WinUIThemeShadow, Translation.Z as in WinUI) on the border
//!   ThemeShadowWrapper around the border of the popup (a border of its own), with the margin
//!   ThemeShadowFlyoutMargin as the room for it in the popup
//!   (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24). The background of the popup is
//!   AcrylicBackgroundFillColorDefaultBrush, the acrylic brush of the flyouts, where the presenter
//!   draws its system backdrop MenuFlyoutSystemBackdrop (desktop acrylic; a border has no system
//!   backdrop; docs/porting/winui-materials.md).
//! - States: PointerOver is the :selected state of FerroUI's menus (the item under the pointer or
//!   reached with the keyboard), Selected (the flyout is open) is :open, Pressed is :pressed. As
//!   MenuBarItem::UpdateVisualStates, pressed comes before pointer over and pointer over before
//!   open, so an open item under the pointer shows PointerOver (:open:pointerover).
//! - FocusVisualMargin="-3" is the FocusAdorner with that margin; Title, ExitDisplayModeOnAccessKeyInvoked
//!   and the AutomationProperties of the source have no counterpart.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::FontWeight;
use ferroui_base::styling::Selectors;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::HeaderedSelectingItemsControl;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::Menu;
use ferroui_controls::MenuItem;
use ferroui_controls::Panel;
use ferroui_controls::PlacementMode;
use ferroui_controls::ScrollViewer;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The implicit style of MenuBarItem (MenuBarItem.xaml).
    resources.control_theme("DefaultMenuBarItemStyle", default_menu_bar_item_style);
    resources.control_theme("DefaultMenuBarStyle", default_menu_bar_style);
    resources.control_theme(type_key::<Menu>(), menu_theme);
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

/// The focus adorner of `default_menu_bar_item_style`.
fn default_menu_bar_item_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-3.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "SystemControlFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(
                    Border::border_brush_property(),
                    "SystemControlFocusVisualSecondaryBrush",
                )
                .dynamic(Border::corner_radius_property(), "ControlCornerRadius"),
        )
        .control())
}

/// The template of `MenuItem` of `default_menu_bar_item_style`.
fn default_menu_bar_item_style_template(t: &Template<MenuItem>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        // The flyout of the item: a MenuFlyoutPresenter (DefaultMenuFlyoutPresenterStyle).
        .children([
            t.make(Border::new())
                .name("ContentRoot")?
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .child(
                    Decorator::child_property(),
                    t.make(Panel::new())
                        // ContentButton: a Button of DefaultButtonStyle with a transparent background, no
                        // border and the padding MenuBarItemButtonPadding.
                        .children([
                            t.make(Border::new())
                                .name("Background")?
                                .template_bind(Border::background_property(), TemplatedControl::background_property())
                                .template_bind(
                                    Border::corner_radius_property(),
                                    TemplatedControl::corner_radius_property(),
                                )
                                .template_bind(
                                    Border::border_thickness_property(),
                                    TemplatedControl::border_thickness_property(),
                                )
                                .template_bind(
                                    Border::border_brush_property(),
                                    TemplatedControl::border_brush_property(),
                                )
                                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                                .control(),
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
                                .template_bind(
                                    ContentPresenter::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                )
                                .set(
                                    ContentPresenter::background_property(),
                                    Some(literal(Brush::parse("Transparent"))?),
                                )
                                .static_resource(ContentPresenter::padding_property(), "MenuBarItemButtonPadding")?
                                .dynamic(ContentPresenter::corner_radius_property(), "ControlCornerRadius")
                                .dynamic(
                                    ContentPresenter::font_family_property(),
                                    "ContentControlThemeFontFamily",
                                )
                                .dynamic(ContentPresenter::font_size_property(), "ControlContentThemeFontSize")
                                .set(ContentPresenter::font_weight_property(), FontWeight::Normal)
                                .set(
                                    ContentPresenter::horizontal_content_alignment_property(),
                                    HorizontalAlignment::Center,
                                )
                                .set(
                                    ContentPresenter::vertical_content_alignment_property(),
                                    VerticalAlignment::Center,
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                                .set(ContentPresenter::recognizes_access_key_property(), true)
                                .control(),
                        ]),
                )
                .control(),
            t.make(Popup::new())
                .name("PART_Popup")?
                .set(Popup::window_manager_add_shadow_hint_property(), false)
                .set(Popup::placement_property(), PlacementMode::BottomEdgeAlignedLeft)
                .set(Popup::is_light_dismiss_enabled_property(), true)
                .template_bind_with(
                    Popup::is_open_property(),
                    Some(MenuItem::is_sub_menu_open_property().property()),
                    None,
                    Some(BindingMode::TwoWay),
                    None,
                )
                .bind(
                    Popup::overlay_input_pass_through_element_property(),
                    path().ancestor_of::<Menu>(0).binding(),
                )
                // The shadow on a border of its own (G-24).
                .child(
                    Popup::child_property(),
                    t.make(Border::new())
                        .name("ThemeShadowWrapper")?
                        .set(Visual::translation_property(), literal(Vector3D::parse("0,0,32"))?)
                        .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
                        .static_resource(Layoutable::margin_property(), "ThemeShadowFlyoutMargin")?
                        .dynamic(Border::corner_radius_property(), "OverlayCornerRadius")
                        .child(
                            Decorator::child_property(),
                            t.make(Border::new())
                                .dynamic(Border::background_property(), "AcrylicBackgroundFillColorDefaultBrush")
                                .dynamic(Border::border_brush_property(), "MenuFlyoutPresenterBorderBrush")
                                .dynamic(
                                    Border::border_thickness_property(),
                                    "MenuFlyoutPresenterBorderThemeThickness",
                                )
                                .dynamic(Border::corner_radius_property(), "OverlayCornerRadius")
                                .dynamic(Layoutable::min_width_property(), "FlyoutThemeMinWidth")
                                .static_resource(Layoutable::min_height_property(), "MenuFlyoutThemeMinHeight")?
                                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                                .child(
                                    Decorator::child_property(),
                                    t.make(ScrollViewer::new())
                                        .name("MenuFlyoutPresenterScrollViewer")?
                                        .static_resource(
                                            StyledElement::theme_property(),
                                            "MenuFlyoutPresenterScrollViewer",
                                        )?
                                        .static_resource(
                                            Layoutable::margin_property(),
                                            "MenuFlyoutPresenterThemePadding",
                                        )?
                                        .content(
                                            ContentControl::content_property(),
                                            t.make(ItemsPresenter::new())
                                                .name("PART_ItemsPresenter")?
                                                .template_bind(
                                                    ItemsPresenter::items_panel_property(),
                                                    ItemsControl::items_panel_property(),
                                                )
                                                .set(
                                                    KeyboardNavigation::tab_navigation_property(),
                                                    KeyboardNavigationMode::Continue,
                                                )
                                                .set(Grid::is_shared_size_scope_property(), true),
                                        )?,
                                ),
                        ),
                )
                .control(),
        ])
        .control())
}

/// `DefaultMenuBarItemStyle`: a control theme of `MenuItem`.
fn default_menu_bar_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<MenuItem>();
    theme.dynamic(TemplatedControl::foreground_property(), "MenuBarItemForeground");
    theme.dynamic(TemplatedControl::background_property(), "MenuBarItemBackground");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "MenuBarItemBorderThickness",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "MenuBarItemBorderBrush");
    theme.dynamic(Layoutable::margin_property(), "MenuBarItemMargin");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(InputElement::is_tab_stop_property(), true);
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_menu_bar_item_style_focus_adorner)),
    );
    theme.template(default_menu_bar_item_style_template);
    // Selected (the flyout is open)
    theme.style(
        nesting().class(":open").template().of::<Border>().name("Background"),
        |style| {
            style.dynamic(Border::background_property(), "MenuBarItemBackgroundSelected");
            style.dynamic(Border::border_brush_property(), "MenuBarItemBorderBrushSelected");
            Ok(())
        },
    )?;
    // PointerOver
    theme.style(
        Selectors::or([
            nesting()
                .class(":selected")
                .not(of::<MenuItem>().class(":open"))
                .template()
                .of::<Border>()
                .name("Background"),
            nesting()
                .class(":open")
                .template()
                .of::<Border>()
                .name("ContentRoot")
                .class(":pointerover")
                .descendant()
                .of::<Border>()
                .name("Background"),
        ]),
        |style| {
            style.dynamic(Border::background_property(), "MenuBarItemBackgroundPointerOver");
            style.dynamic(Border::border_brush_property(), "MenuBarItemBorderBrushPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":selected")
                .not(of::<MenuItem>().class(":open"))
                .template()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
            nesting()
                .class(":open")
                .template()
                .of::<Border>()
                .name("ContentRoot")
                .class(":pointerover")
                .descendant()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
        ]),
        |style| {
            style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundPointerOver");
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("ContentRoot")
            .class(":pointerover"),
        |style| {
            style.style(nesting().descendant().of::<Border>().name("Background"), |style| {
                style.dynamic(Border::background_property(), "MenuBarItemBackgroundPressed");
                style.dynamic(Border::border_brush_property(), "MenuBarItemBorderBrushPressed");
                Ok(())
            })?;
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundPressed");
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // Disabled of ContentButton
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_HeaderPresenter"),
        |style| {
            style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundDisabled");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Menu` of `default_menu_bar_style`.
fn default_menu_bar_style_template(t: &Template<Menu>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
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
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .set(
            Layoutable::horizontal_alignment_property(),
            HorizontalAlignment::Stretch,
        )
        .child(
            Decorator::child_property(),
            t.make(ItemsPresenter::new())
                .name("PART_ItemsPresenter")?
                .template_bind(
                    ItemsPresenter::items_panel_property(),
                    ItemsControl::items_panel_property(),
                )
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .set(
                    KeyboardNavigation::tab_navigation_property(),
                    KeyboardNavigationMode::Continue,
                ),
        )
        .control())
}

/// `DefaultMenuBarStyle`: a control theme of `Menu`.
fn default_menu_bar_style(theme: &StyleBuilder) -> Result {
    theme.target::<Menu>();
    theme.dynamic(TemplatedControl::background_property(), "MenuBarBackground");
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.static_resource(Layoutable::min_height_property(), "MenuBarHeight")?;
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.static_resource(ItemsControl::item_container_theme_property(), "DefaultMenuBarItemStyle")?;
    theme.template(default_menu_bar_style_template);
    Ok(())
}

/// The default control theme of `Menu`.
fn menu_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Menu>();
    theme.based_on("DefaultMenuBarStyle")?;
    Ok(())
}
