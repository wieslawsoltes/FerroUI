//! `Controls/MenuItem.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/MenuFlyout_themeresources.xaml (the styles DefaultMenuFlyoutItemStyle,
//!   DefaultToggleMenuFlyoutItemStyle, DefaultMenuFlyoutSubItemStyle and the template of
//!   MenuFlyoutSeparator; the resources of the document are Resources/MenuFlyout_themeresources.xaml)
//!   controls/dev/RadioMenuFlyoutItem/RadioMenuFlyoutItem_themeresources.xaml (the styles
//!   DefaultRadioMenuFlyoutItemStyle and RadioMenuFlyoutSubItemStyle)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI has one class of menu item where WinUI has four: MenuFlyoutItem, ToggleMenuFlyoutItem,
//!   RadioMenuFlyoutItem and MenuFlyoutSubItem. A MenuItem is a toggle item when its ToggleType is
//!   CheckBox (:toggle), a radio item when it is Radio (:radio), and a sub item when it has items
//!   (:not(MenuItem:empty)). DefaultMenuFlyoutItemStyle holds one template with the parts of the four
//!   templates (the check glyph of the toggle items, the icon, the text, the keyboard accelerator
//!   text and the chevron of the sub items) and the states of each kind, the states of a toggle or
//!   radio item and of a sub item after those of a plain item. The keyed themes of the other kinds
//!   are based on it: DefaultToggleMenuFlyoutItemStyle and DefaultRadioMenuFlyoutItemStyle set the
//!   ToggleType of their kind, DefaultMenuFlyoutSubItemStyle the setters of a sub item.
//!   DefaultSplitMenuFlyoutItemStyle is waived: FerroUI's MenuItem has no split mode.
//! - Template parts: TextBlock, the text of the item, is the content presenter PART_HeaderPresenter
//!   of the header (FerroUI's header is content, with access keys); the popup of a sub item is
//!   PART_Popup with the items presenter PART_ItemsPresenter. The popup content is the template of
//!   MenuFlyoutPresenter (WinUI opens a MenuFlyoutPresenter for a sub item); it opens at the right
//!   edge of the item, 4 px over it (CascadingMenuHelper::m_subMenuOverlapPixels). The shadow of
//!   the presenter of a sub item (ApplyElevationEffect at the depth 1 of a submenu: a ThemeShadow at
//!   a Translation.Z of 40) is a ThemeShadow (the shared WinUIThemeShadow, Translation.Z as in WinUI) on the border
//!   ThemeShadowWrapper around the border of the popup (a border of its own), with the margin
//!   ThemeShadowSubMenuMargin as the room for it in the popup; deeper submenus have the same
//!   shadow (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24). The background of the popup is
//!   AcrylicBackgroundFillColorDefaultBrush, the acrylic brush of the flyouts, where the presenter
//!   draws its system backdrop MenuFlyoutSystemBackdrop (desktop acrylic; a border has no system
//!   backdrop; docs/porting/winui-materials.md).
//! - CheckPlaceholderStates, which the presenter computes from all of its items (the text of every
//!   item moves by MenuFlyoutItemPlaceholderThemeThickness when one item is a toggle item or has an
//!   icon), are columns of shared size: the check glyph column (12 px glyph and 16 px margin) and
//!   the icon column (16 px icon and 12 px margin) take their width in every item of a presenter
//!   when one item shows them. The keyboard accelerator text column is shared the same way, as
//!   TemplateSettings.KeyboardAcceleratorTextMinWidth makes the accelerator texts one width; the
//!   text shows when the item has an InputGesture (KeyboardAcceleratorTextVisible).
//! - PointerOver is the :selected state of FerroUI's menus (the item under the pointer or reached
//!   with the keyboard); Pressed is :pressed while the pointer is over the item's own layout root,
//!   so that pressing an item of an open submenu does not press its parent.
//! - NarrowPadding (PaddingSizeStates) applies always: WinUI uses it when the menu was opened with
//!   the mouse, the pen or the keyboard and MenuFlyoutItemThemePadding for touch; FerroUI does not
//!   report the input device that opened a menu. The narrow padding is the Padding of the item
//!   (the template binds LayoutRoot's padding to it).
//! - The glyphs E73E (check), E915 (radio bullet), E974 and E76C (sub item chevrons) are the
//!   geometries of Resources/Glyphs.xaml at the ink size of the 12 px glyphs.
//! - MenuFlyoutItemTextTrimming (Clip) is TextTrimming="None" of the header presenter, which
//!   clips; FerroUI's TextTrimming has no Clip value to hold in a resource.
//!   WinUI: MenuFlyoutItemTextTrimming
//! - A MenuItem whose Header is "-" (:separator) is drawn with the template of MenuFlyoutSeparator.
//! - UseSystemFocusVisuals is the default focus adorner; KeyboardAcceleratorPlacementMode,
//!   IsTextScaleFactorEnabled and AccessibilityView have no counterpart.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::data::BindingMode;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextAlignment;
use ferroui_base::media::TextTrimming;
use ferroui_base::styling::Selectors;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::converters::PlatformKeyGestureConverter;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::HeaderedSelectingItemsControl;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinition;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::DefinitionBase;
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::ItemsControl;
use ferroui_controls::MenuItem;
use ferroui_controls::MenuItemToggleType;
use ferroui_controls::Panel;
use ferroui_controls::PlacementMode;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TextBlock;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.deferred("MenuItemKeyGestureConverter", |scope| {
        Ok(value(::std::rc::Rc::new(PlatformKeyGestureConverter::new())))
    });
    resources.control_theme("DefaultMenuFlyoutItemStyle", default_menu_flyout_item_style);
    // WinUI applies the default style of a control beneath an explicit style, so the keyed
    //       themes are based on the default theme (docs/porting/winui-theme.md, section 4.2).
    resources.control_theme(
        "DefaultToggleMenuFlyoutItemStyle",
        default_toggle_menu_flyout_item_style,
    );
    resources.control_theme("DefaultRadioMenuFlyoutItemStyle", default_radio_menu_flyout_item_style);
    resources.control_theme("DefaultMenuFlyoutSubItemStyle", default_menu_flyout_sub_item_style);
    // RadioMenuFlyoutSubItemStyle: a sub item with the radio bullet E915 of its checked state in
    //       the check column (15 px margin) and the chevron E76C.
    resources.control_theme("RadioMenuFlyoutSubItemStyle", radio_menu_flyout_sub_item_style);
    resources.control_theme(type_key::<MenuItem>(), menu_item_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `MenuItem` of `default_menu_flyout_item_style`.
fn default_menu_flyout_item_style_template(t: &Template<MenuItem>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        // The submenu: a MenuFlyoutPresenter (DefaultMenuFlyoutPresenterStyle).
        .children([
            t.make(Border::new())
                .name("LayoutRoot")?
                .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
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
                .static_resource(Layoutable::margin_property(), "MenuFlyoutItemMargin")?
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .child(
                    Decorator::child_property(),
                    t.make(Grid::new())
                        .with(|e| {
                            let items = Grid::column_definitions(e);
                            items.add(
                                t.make(ColumnDefinition::new())
                                    .set(ColumnDefinition::width_property(), GridLength::auto())
                                    .set(
                                        DefinitionBase::shared_size_group_property(),
                                        Some(String::from("MenuFlyoutItemCheckPlaceholder")),
                                    )
                                    .end()
                                    .into(),
                            );
                            items.add(
                                t.make(ColumnDefinition::new())
                                    .set(ColumnDefinition::width_property(), GridLength::auto())
                                    .set(
                                        DefinitionBase::shared_size_group_property(),
                                        Some(String::from("MenuFlyoutItemIconPlaceholder")),
                                    )
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
                                    .set(
                                        DefinitionBase::shared_size_group_property(),
                                        Some(String::from("MenuFlyoutItemKeyboardAcceleratorText")),
                                    )
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
                        // E73E at 12 px in a cell of 12 px, with the margin 0,0,16,0 of the source.
                        // E974 at 12 px in a cell of 12 px, with the margin MenuFlyoutItemChevronMargin
                        //                   (24,0,0,-1) of the source.
                        .children([
                            t.make(Path::new())
                                .name("CheckGlyph")?
                                .set(Grid::column_property(), 0)
                                .static_resource(Path::data_property(), "GlyphGeometryE73E")?
                                .dynamic(Shape::fill_property(), "MenuFlyoutSubItemChevron")
                                .set(Shape::stretch_property(), Stretch::Uniform)
                                .set(Layoutable::width_property(), 10.0)
                                .set(Layoutable::margin_property(), Thickness::new(1.0, 0.0, 17.0, 0.0))
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(Visual::opacity_property(), 0.0)
                                .set(Visual::is_visible_property(), false)
                                .control(),
                            t.make(Viewbox::new())
                                .name("IconRoot")?
                                .set(Grid::column_property(), 1)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(Layoutable::width_property(), 16.0)
                                .set(Layoutable::height_property(), 16.0)
                                .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 12.0, 0.0))
                                .set(Visual::is_visible_property(), false)
                                // A path icon is unscaled there, as WinUI's, so that the Viewbox scales its
                                //                     geometry as WinUI's does: the icon is a logical child of the item, which a
                                //                     style of this presenter does not reach, so a style of the theme does it for
                                //                     the path icons of menu items (MenuItem > PathIcon, Controls/WinUIControls.xaml, PI-3).
                                .child(
                                    Viewbox::child_property(),
                                    t.make(ContentPresenter::new())
                                        .name("IconContent")?
                                        .template_bind(ContentPresenter::content_property(), MenuItem::icon_property())
                                        .template_bind(
                                            ContentPresenter::foreground_property(),
                                            TemplatedControl::foreground_property(),
                                        ),
                                )
                                .control(),
                            t.make(ContentPresenter::new())
                                .name("PART_HeaderPresenter")?
                                .set(Grid::column_property(), 2)
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
                                .set(ContentPresenter::text_trimming_property(), <dyn TextTrimming>::none())
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(ContentPresenter::recognizes_access_key_property(), true)
                                .control(),
                            t.make(TextBlock::new())
                                .name("KeyboardAcceleratorTextBlock")?
                                .set(Grid::column_property(), 3)
                                .static_resource(StyledElement::theme_property(), "CaptionTextBlockStyle")?
                                .template_bind_with(
                                    TextBlock::text_property(),
                                    Some(MenuItem::input_gesture_property().property()),
                                    Some(t.scope().converter("MenuItemKeyGestureConverter")?),
                                    None,
                                    None,
                                )
                                .set(Layoutable::margin_property(), Thickness::new(24.0, 4.0, 0.0, 0.0))
                                .dynamic(
                                    TextBlock::foreground_property(),
                                    "MenuFlyoutItemKeyboardAcceleratorTextForeground",
                                )
                                .set(TextBlock::text_alignment_property(), TextAlignment::Right)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .template_bind_with(
                                    Visual::is_visible_property(),
                                    Some(MenuItem::input_gesture_property().property()),
                                    Some(ObjectConverters::is_not_null()),
                                    None,
                                    None,
                                )
                                .control(),
                            t.make(Path::new())
                                .name("SubItemChevron")?
                                .set(Grid::column_property(), 4)
                                .static_resource(Path::data_property(), "GlyphGeometryE974")?
                                .dynamic(Shape::fill_property(), "MenuFlyoutSubItemChevron")
                                .set(Shape::stretch_property(), Stretch::Uniform)
                                .set(Layoutable::width_property(), 5.5)
                                .set(Layoutable::margin_property(), Thickness::new(27.25, 0.0, 3.25, -1.0))
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(Visual::is_visible_property(), false)
                                .control(),
                        ]),
                )
                .control(),
            t.make(Popup::new())
                .name("PART_Popup")?
                .set(Popup::window_manager_add_shadow_hint_property(), false)
                .set(Popup::placement_property(), PlacementMode::RightEdgeAlignedTop)
                .set(Popup::horizontal_offset_property(), -4.0)
                .set(Popup::is_light_dismiss_enabled_property(), false)
                .template_bind_with(
                    Popup::is_open_property(),
                    Some(MenuItem::is_sub_menu_open_property().property()),
                    None,
                    Some(BindingMode::TwoWay),
                    None,
                )
                // The shadow on a border of its own (G-24).
                .child(
                    Popup::child_property(),
                    t.make(Border::new())
                        .name("ThemeShadowWrapper")?
                        .set(Visual::translation_property(), literal(Vector3D::parse("0,0,40"))?)
                        .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
                        .static_resource(Layoutable::margin_property(), "ThemeShadowSubMenuMargin")?
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

/// The template of `MenuItem` of `default_menu_flyout_item_style`.
fn default_menu_flyout_item_style_template_2(t: &Template<MenuItem>) -> Result<Ref<Control>> {
    Ok(t.make(Rectangle::new())
        .name("SeparatorRectangle")?
        .dynamic(Shape::fill_property(), "MenuFlyoutSeparatorBackground")
        .static_resource(Layoutable::margin_property(), "MenuFlyoutSeparatorThemePadding")?
        .static_resource(Layoutable::height_property(), "MenuFlyoutSeparatorHeight")?
        .control())
}

/// `DefaultMenuFlyoutItemStyle`: a control theme of `MenuItem`.
fn default_menu_flyout_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<MenuItem>();
    theme.dynamic(TemplatedControl::background_property(), "MenuFlyoutItemBackground");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "MenuFlyoutItemBackgroundBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "MenuFlyoutItemBorderThickness",
    );
    theme.dynamic(TemplatedControl::foreground_property(), "MenuFlyoutItemForeground");
    // MenuFlyoutItemThemePadding, replaced by NarrowPadding of PaddingSizeStates
    //         (MenuFlyoutItemThemePaddingNarrow), which applies always: set on the item, as a style of
    //         the template's LayoutRoot would not override its template binding of the padding.
    theme.static_resource(TemplatedControl::padding_property(), "MenuFlyoutItemThemePaddingNarrow")?;
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(default_menu_flyout_item_style_template);
    // MenuFlyoutItem: PointerOver
    theme.style(nesting().class(":selected"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "MenuFlyoutItemBackgroundPointerOver");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "MenuFlyoutItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<ContentPresenter>().name("IconContent"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "MenuFlyoutItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
            |style| {
                style.dynamic(
                    TextBlock::foreground_property(),
                    "MenuFlyoutItemKeyboardAcceleratorTextForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // MenuFlyoutItem: Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("LayoutRoot")
            .class(":pointerover"),
        |style| {
            style.dynamic(Border::background_property(), "MenuFlyoutItemBackgroundPressed");
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutItemForegroundPressed",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting().descendant().of::<ContentPresenter>().name("IconContent"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutItemForegroundPressed",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .descendant()
                    .of::<TextBlock>()
                    .name("KeyboardAcceleratorTextBlock"),
                |style| {
                    style.dynamic(
                        TextBlock::foreground_property(),
                        "MenuFlyoutItemKeyboardAcceleratorTextForegroundPressed",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // MenuFlyoutItem: Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "MenuFlyoutItemBackgroundDisabled");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "MenuFlyoutItemForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<ContentPresenter>().name("IconContent"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "MenuFlyoutItemForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
            |style| {
                style.dynamic(
                    TextBlock::foreground_property(),
                    "MenuFlyoutItemKeyboardAcceleratorTextForegroundDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // IconPlaceholder: the icon column of an item with an icon.
    theme.style(
        nesting().class(":icon").template().of::<Viewbox>().name("IconRoot"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // ToggleMenuFlyoutItem and RadioMenuFlyoutItem: the check glyph column (CheckPlaceholder),
    //         the radio bullet E915 of RadioMenuFlyoutItem, and the keyboard accelerator text.
    theme.style(
        Selectors::or([
            nesting().class(":toggle").template().of::<Path>().name("CheckGlyph"),
            nesting().class(":radio").template().of::<Path>().name("CheckGlyph"),
        ]),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":radio").template().of::<Path>().name("CheckGlyph"),
        |style| {
            style.static_resource(Path::data_property(), "GlyphGeometryE915")?;
            style.set(Layoutable::width_property(), 6.0);
            style.set(Layoutable::margin_property(), Thickness::new(3.0, 0.0, 19.0, 0.0));
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .template()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
            nesting()
                .class(":radio")
                .template()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
        ]),
        |style| {
            style.set(Layoutable::margin_property(), Thickness::new(24.0, 0.0, 0.0, 0.0));
            style.dynamic(
                TextBlock::foreground_property(),
                "ToggleMenuFlyoutItemKeyboardAcceleratorTextForeground",
            );
            Ok(())
        },
    )?;
    // ToggleMenuFlyoutItem, RadioMenuFlyoutItem: Checked of CheckStates
    theme.style(
        nesting().class(":checked").template().of::<Path>().name("CheckGlyph"),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        },
    )?;
    // ToggleMenuFlyoutItem, RadioMenuFlyoutItem: PointerOver
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":selected")
                .template()
                .of::<Border>()
                .name("LayoutRoot"),
            nesting()
                .class(":radio")
                .class(":selected")
                .template()
                .of::<Border>()
                .name("LayoutRoot"),
        ]),
        |style| {
            style.dynamic(Border::background_property(), "MenuFlyoutSubItemBackgroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":selected")
                .template()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
            nesting()
                .class(":radio")
                .class(":selected")
                .template()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
        ]),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "MenuFlyoutItemForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":selected")
                .template()
                .of::<Path>()
                .name("CheckGlyph"),
            nesting()
                .class(":radio")
                .class(":selected")
                .template()
                .of::<Path>()
                .name("CheckGlyph"),
        ]),
        |style| {
            style.dynamic(Shape::fill_property(), "MenuFlyoutItemForegroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":selected")
                .template()
                .of::<ContentPresenter>()
                .name("IconContent"),
            nesting()
                .class(":radio")
                .class(":selected")
                .template()
                .of::<ContentPresenter>()
                .name("IconContent"),
        ]),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "MenuFlyoutItemForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":selected")
                .template()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
            nesting()
                .class(":radio")
                .class(":selected")
                .template()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
        ]),
        |style| {
            style.dynamic(
                TextBlock::foreground_property(),
                "ToggleMenuFlyoutItemKeyboardAcceleratorTextForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    // ToggleMenuFlyoutItem, RadioMenuFlyoutItem: Pressed
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover"),
            nesting()
                .class(":radio")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover"),
        ]),
        |style| {
            style.dynamic(Border::background_property(), "MenuFlyoutSubItemBackgroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
            nesting()
                .class(":radio")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
        ]),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "MenuFlyoutSubItemForegroundPressed",
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<Path>()
                .name("CheckGlyph"),
            nesting()
                .class(":radio")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<Path>()
                .name("CheckGlyph"),
        ]),
        |style| {
            style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemForegroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<ContentPresenter>()
                .name("IconContent"),
            nesting()
                .class(":radio")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<ContentPresenter>()
                .name("IconContent"),
        ]),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "MenuFlyoutSubItemForegroundPressed",
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
            nesting()
                .class(":radio")
                .class(":pressed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
        ]),
        |style| {
            style.dynamic(
                TextBlock::foreground_property(),
                "ToggleMenuFlyoutItemKeyboardAcceleratorTextForegroundPressed",
            );
            Ok(())
        },
    )?;
    // ToggleMenuFlyoutItem, RadioMenuFlyoutItem: Disabled (which does not set the background
    //         of the layout root)
    theme.style(
        nesting()
            .class(":toggle")
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("LayoutRoot"),
        |style| {
            style.template_bind(Border::background_property(), TemplatedControl::background_property());
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":radio")
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("LayoutRoot"),
        |style| {
            style.template_bind(Border::background_property(), TemplatedControl::background_property());
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":disabled")
                .template()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
            nesting()
                .class(":radio")
                .class(":disabled")
                .template()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
        ]),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "MenuFlyoutSubItemForegroundDisabled",
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":disabled")
                .template()
                .of::<Path>()
                .name("CheckGlyph"),
            nesting()
                .class(":radio")
                .class(":disabled")
                .template()
                .of::<Path>()
                .name("CheckGlyph"),
        ]),
        |style| {
            style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemForegroundDisabled");
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":disabled")
                .template()
                .of::<ContentPresenter>()
                .name("IconContent"),
            nesting()
                .class(":radio")
                .class(":disabled")
                .template()
                .of::<ContentPresenter>()
                .name("IconContent"),
        ]),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "MenuFlyoutSubItemForegroundDisabled",
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":toggle")
                .class(":disabled")
                .template()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
            nesting()
                .class(":radio")
                .class(":disabled")
                .template()
                .of::<TextBlock>()
                .name("KeyboardAcceleratorTextBlock"),
        ]),
        |style| {
            style.dynamic(
                TextBlock::foreground_property(),
                "ToggleMenuFlyoutItemKeyboardAcceleratorTextForegroundDisabled",
            );
            Ok(())
        },
    )?;
    // MenuFlyoutSubItem: the setters of DefaultMenuFlyoutSubItemStyle and the chevron.
    theme.style(nesting().not(of::<MenuItem>().class(":empty")), |style| {
        style.dynamic(TemplatedControl::background_property(), "MenuFlyoutSubItemBackground");
        style.dynamic(
            TemplatedControl::border_brush_property(),
            "MenuFlyoutSubItemBackgroundBrush",
        );
        style.dynamic(
            TemplatedControl::border_thickness_property(),
            "MenuFlyoutSubItemBorderThickness",
        );
        style.dynamic(TemplatedControl::foreground_property(), "MenuFlyoutSubItemForeground");
        style.style(nesting().template().of::<Path>().name("SubItemChevron"), |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        })?;
        Ok(())
    })?;
    // MenuFlyoutSubItem: PointerOver
    theme.style(
        nesting().class(":selected").not(of::<MenuItem>().class(":empty")),
        |style| {
            style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                style.dynamic(Border::background_property(), "MenuFlyoutSubItemBackgroundPointerOver");
                Ok(())
            })?;
            style.style(
                nesting()
                    .template()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutSubItemForegroundPointerOver",
                    );
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Path>().name("SubItemChevron"), |style| {
                style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemChevronPointerOver");
                Ok(())
            })?;
            style.style(
                nesting().template().of::<ContentPresenter>().name("IconContent"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutSubItemForegroundPointerOver",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // MenuFlyoutSubItem: Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .not(of::<MenuItem>().class(":empty"))
            .template()
            .of::<Border>()
            .name("LayoutRoot")
            .class(":pointerover"),
        |style| {
            style.dynamic(Border::background_property(), "MenuFlyoutSubItemBackgroundPressed");
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutSubItemForegroundPressed",
                    );
                    Ok(())
                },
            )?;
            style.style(nesting().descendant().of::<Path>().name("SubItemChevron"), |style| {
                style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemChevronPressed");
                Ok(())
            })?;
            style.style(
                nesting().descendant().of::<ContentPresenter>().name("IconContent"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutSubItemForegroundPressed",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // MenuFlyoutSubItem: SubMenuOpened
    theme.style(
        nesting().class(":open").not(of::<MenuItem>().class(":empty")),
        |style| {
            style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                style.dynamic(
                    Border::background_property(),
                    "MenuFlyoutSubItemBackgroundSubMenuOpened",
                );
                Ok(())
            })?;
            style.style(
                nesting()
                    .template()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutSubItemForegroundSubMenuOpened",
                    );
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Path>().name("SubItemChevron"), |style| {
                style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemChevronSubMenuOpened");
                Ok(())
            })?;
            style.style(
                nesting().template().of::<ContentPresenter>().name("IconContent"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutSubItemForegroundSubMenuOpened",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // MenuFlyoutSubItem: Disabled
    theme.style(
        nesting().class(":disabled").not(of::<MenuItem>().class(":empty")),
        |style| {
            style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                style.dynamic(Border::background_property(), "MenuFlyoutSubItemBackgroundDisabled");
                Ok(())
            })?;
            style.style(
                nesting()
                    .template()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutSubItemForegroundDisabled",
                    );
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Path>().name("SubItemChevron"), |style| {
                style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemChevronDisabled");
                Ok(())
            })?;
            style.style(
                nesting().template().of::<ContentPresenter>().name("IconContent"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "MenuFlyoutSubItemForegroundDisabled",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // MenuFlyoutSeparator: a menu item whose header is "-".
    theme.style(nesting().class(":separator"), |style| {
        style.template(default_menu_flyout_item_style_template_2);
        Ok(())
    })?;
    Ok(())
}

/// `DefaultToggleMenuFlyoutItemStyle`: a control theme of `MenuItem`.
fn default_toggle_menu_flyout_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<MenuItem>();
    theme.based_on("DefaultMenuFlyoutItemStyle")?;
    theme.set(MenuItem::toggle_type_property(), MenuItemToggleType::CheckBox);
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    Ok(())
}

/// `DefaultRadioMenuFlyoutItemStyle`: a control theme of `MenuItem`.
fn default_radio_menu_flyout_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<MenuItem>();
    theme.based_on("DefaultMenuFlyoutItemStyle")?;
    theme.set(MenuItem::toggle_type_property(), MenuItemToggleType::Radio);
    Ok(())
}

/// `DefaultMenuFlyoutSubItemStyle`: a control theme of `MenuItem`.
fn default_menu_flyout_sub_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<MenuItem>();
    theme.based_on("DefaultMenuFlyoutItemStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "MenuFlyoutSubItemBackground");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "MenuFlyoutSubItemBackgroundBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "MenuFlyoutSubItemBorderThickness",
    );
    theme.dynamic(TemplatedControl::foreground_property(), "MenuFlyoutSubItemForeground");
    Ok(())
}

/// `RadioMenuFlyoutSubItemStyle`: a control theme of `MenuItem`.
fn radio_menu_flyout_sub_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<MenuItem>();
    theme.based_on("DefaultMenuFlyoutSubItemStyle")?;
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
        style.set(Visual::is_visible_property(), true);
        style.static_resource(Path::data_property(), "GlyphGeometryE915")?;
        style.set(Layoutable::width_property(), 6.0);
        style.set(Layoutable::margin_property(), Thickness::new(3.0, 0.0, 18.0, 0.0));
        Ok(())
    })?;
    theme.style(nesting().template().of::<Path>().name("SubItemChevron"), |style| {
        style.static_resource(Path::data_property(), "GlyphGeometryE76C")?;
        style.set(Layoutable::width_property(), 6.4);
        style.set(Layoutable::margin_property(), Thickness::new(26.8, 0.0, 2.8, -1.0));
        Ok(())
    })?;
    // PointerOver, Pressed, SubMenuOpened and Disabled of the check glyph.
    theme.style(
        nesting().class(":selected").template().of::<Path>().name("CheckGlyph"),
        |style| {
            style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemForegroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("LayoutRoot")
            .class(":pointerover")
            .descendant()
            .of::<Path>()
            .name("CheckGlyph"),
        |style| {
            style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemForegroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":open").template().of::<Path>().name("CheckGlyph"),
        |style| {
            style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemForegroundSubMenuOpened");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Path>().name("CheckGlyph"),
        |style| {
            style.dynamic(Shape::fill_property(), "MenuFlyoutSubItemForegroundDisabled");
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `MenuItem`.
fn menu_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<MenuItem>();
    theme.based_on("DefaultMenuFlyoutItemStyle")?;
    Ok(())
}
