//! `Controls/DrawerPage.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/NavigationView/NavigationView_themeresources.xaml (PaneToggleButtonStyle,
//!   NavigationViewItemHeaderTextStyle and NavigationViewTitleHeaderContentControlTextStyle; the
//!   resources are Resources/NavigationView_themeresources.xaml)
//!   controls/dev/NavigationView/NavigationView.xaml (the left pane of the implicit style of
//!   NavigationView: RootSplitView, PaneContentGrid, ContentGrid, HeaderContent and the states
//!   of the display modes)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! WinUI has no page with a drawer: an application shows its navigation in the pane of a
//! NavigationView. FerroUI's DrawerPage is drawn as that NavigationView with its pane at the left:
//! the SplitView PART_SplitView is RootSplitView (Controls/SplitView.xaml), the drawer with its
//! header and footer is the pane (PaneContentGrid: PaneHeader, the menu items and PaneFooter), and
//! the content is the content grid of NavigationView with its header row (the top bar of
//! FerroUI's DrawerPage, with the pane toggle button and the header). The template keeps the parts
//! and the behaviour of FerroUI's DrawerPage (src/FerroUI.Themes.Fluent/Controls/DrawerPage.xaml):
//! the bars at the top or the bottom, the compact pane toggle, the backdrop and the conditions that
//! hide the top bar.
//!
//! The NavigationView styles (docs/porting/winui-theme.md, section 7, B5): DrawerPage keeps its
//! look, drawn after the left pane of NavigationView. PaneToggleButtonStyle (the pane toggle
//! buttons of DrawerPage and of NavigationView) and the two text styles are in this document,
//! NavigationBackButtonNormalStyle and NavigationBackButtonSmallStyle in
//! Controls/NavigationPage.xaml; the items of the left and the top pane are also written into the
//! themes of TabItem and TabbedPage (Controls/TabItem.xaml, Controls/TabbedPage.xaml). WinUI's
//! NavigationView itself, its items, item headers and separators and the item presenter styles are
//! the templates of FerroUI's NavigationView (Controls/NavigationView.xaml).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - PaneToggleButtonStyle: the Grid LayoutRoot is a Border of the same name around a Grid. The
//!   AnimatedIcon Icon (AnimatedGlobalNavigationButtonVisualSource with the fallback glyph E700 at
//!   the font size of the button, 16) is a path of GlyphGeometryE700, 16 x 16; the
//!   AnimatedIcon.State setters are left out. The content presenter is PART_ContentPresenter; the
//!   states of DefaultButtonStyle, which the keyed theme is based on (G-15), would draw their
//!   backgrounds and borders on it, so the states set them transparent.
//! - DrawerPagePaneToggleButton (a theme of the toggle buttons of DrawerPage, based on
//!   PaneToggleButtonStyle) shows DrawerIcon of FerroUI's DrawerPage in the box of the glyph in
//!   place of the glyph, and has no title column.
//! - RootSplitView: its pane background is NavigationViewDefaultPaneBackground while the pane
//!   overlays the content (PaneOverlaying) and NavigationViewExpandedPaneBackground otherwise
//!   (PaneNotOverlaying, which also makes its border transparent): the pane overlays the content
//!   while it is open in the Overlay and CompactOverlay modes. DrawerBackground of FerroUI's
//!   DrawerPage replaces it (in code). The corner radius is the right corners of the
//!   CornerRadius of the page (OverlayCornerRadius, as the style of NavigationView sets).
//! - The shadow of the overlaying pane (ShadowCaster and its transitions) is left out (ThemeShadow).
//! - ContentGrid (border, background, corner radius) is a Border; in the Overlay mode (WinUI's
//!   Minimal display mode) it has the Minimal values (NavigationViewMinimalContentGridBorderThickness,
//!   NavigationViewMinimalContentGridCornerRadius, NavigationViewMinimalContentMargin). With the
//!   drawer at the right the border and the corner are mirrored, at the top or the bottom the
//!   content grid has the border of the top pane (TopNavigationViewContentGridBorderThickness),
//!   mirrored at the bottom.
//! - The header of the top bar is the HeaderContent of NavigationView with
//!   NavigationViewTitleHeaderContentControlTextStyle; it stands beside the pane toggle button,
//!   8 px from it, where NavigationView places it under the button (NavigationViewHeaderMargin,
//!   NavigationViewMinimalHeaderMargin). The top bar has the padding of the button holder
//!   (NavigationViewButtonHolderGridMargin).
//! - The back button, the close button, the pane title, the auto suggest box, the pane custom
//!   content, the menu and footer items, the item separator and the top pane of NavigationView are
//!   left out: FerroUI's DrawerPage has none of them (its drawer is any content).
//! - The menu items: a drawer is any content, and a control theme styles only its own template, so
//!   the theme offers two keyed themes for the usual drawer, a ListBox of destinations:
//!   DrawerPageNavigationViewMenu (MenuItemsHost in MenuItemsScrollViewer: a list without a
//!   background or a border) and its item container theme DrawerPageNavigationViewItem (the item of
//!   the left pane, MUX_NavigationViewItemPresenterStyleWhenOnLeftPane, as the tabs at the left of
//!   Controls/TabItem.xaml: the subtle fills of the pointer states and the selection indicator).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::FontFamily;
use ferroui_base::media::FontWeight;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextWrapping;
use ferroui_base::styling::Selectors;
use ferroui_base::CornerRadius;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_controls::DrawerBehavior;
use ferroui_controls::DrawerPage;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::ListBox;
use ferroui_controls::ListBoxItem;
use ferroui_controls::Page;
use ferroui_controls::Panel;
use ferroui_controls::ScrollViewer;
use ferroui_controls::SplitView;
use ferroui_controls::SplitViewDisplayMode;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_controls::ToolTip;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(
        "NavigationViewItemHeaderTextStyle",
        navigation_view_item_header_text_style,
    );
    resources.control_theme(
        "NavigationViewTitleHeaderContentControlTextStyle",
        navigation_view_title_header_content_control_text_style,
    );
    resources.control_theme("PaneToggleButtonStyle", pane_toggle_button_style);
    // The pane toggle buttons of DrawerPage: PaneToggleButtonStyle with the DrawerIcon of the page
    //       (the content of the button) in place of the glyph.
    resources.control_theme("DrawerPagePaneToggleButton", drawer_page_pane_toggle_button);
    // An item of the left pane of NavigationView for a ListBoxItem: the template of
    //       MUX_NavigationViewItemPresenterStyleWhenOnLeftPane with the values of the implicit style of
    //       NavigationViewItem (font, corner radius), as the tabs at the left of Controls/TabItem.xaml:
    //       its pointer states, the selection indicator of the selected item and the disabled opacity.
    //       A ListBoxItem has no icon: the icon column has the 8 of IconCollapsed. The expand chevron,
    //       the info badge and the compact pane states are left out.
    resources.control_theme("DrawerPageNavigationViewItem", drawer_page_navigation_view_item);
    // The menu items of the pane (MenuItemsHost of NavigationView.xaml) for a ListBox in the drawer:
    //       a list without a background or a border of its own, whose items are the items of the left
    //       pane (DrawerPageNavigationViewItem). DrawerPage cannot style the content of its drawer (a
    //       control theme reaches only its template), so an application gives its drawer's list this
    //       theme: <ListBox Theme="{StaticResource DrawerPageNavigationViewMenu}">.
    resources.control_theme("DrawerPageNavigationViewMenu", drawer_page_navigation_view_menu);
    resources.control_theme(type_key::<DrawerPage>(), drawer_page_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// `NavigationViewItemHeaderTextStyle`: a control theme of `TextBlock`.
fn navigation_view_item_header_text_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBlock>();
    theme.based_on("BaseTextBlockStyle")?;
    theme.set(TextBlock::font_size_property(), 14.0);
    theme.set(TextBlock::font_weight_property(), FontWeight::SemiBold);
    theme.set(TextBlock::text_wrapping_property(), TextWrapping::NoWrap);
    Ok(())
}

/// `NavigationViewTitleHeaderContentControlTextStyle`: a control theme of `ContentControl`.
fn navigation_view_title_header_content_control_text_style(theme: &StyleBuilder) -> Result {
    theme.target::<ContentControl>();
    theme.set(TemplatedControl::font_weight_property(), FontWeight::SemiBold);
    theme.set(TemplatedControl::font_size_property(), 28.0);
    theme.set(
        TemplatedControl::font_family_property(),
        literal(FontFamily::parse("Segoe UI Variable Text, Segoe UI, $Default"))?,
    );
    theme.dynamic(Layoutable::margin_property(), "NavigationViewHeaderMargin");
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    Ok(())
}

/// The template of `Button` of `pane_toggle_button_style`.
fn pane_toggle_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .template_bind(Layoutable::height_property(), Layoutable::min_height_property())
        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .set(
            Layoutable::horizontal_alignment_property(),
            HorizontalAlignment::Stretch,
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*"))?);
                    Ok(())
                })?
                .children([
                    t.make(Border::new())
                        .template_bind(Layoutable::width_property(), Layoutable::min_width_property())
                        .child(
                            Decorator::child_property(),
                            t.make(Path::new())
                                .name("Icon")?
                                .static_resource(Path::data_property(), "GlyphGeometryE700")?
                                .set(Layoutable::width_property(), 16.0)
                                .set(Layoutable::height_property(), 16.0)
                                .set(Shape::stretch_property(), Stretch::Uniform)
                                .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .template_bind(
                                    Layoutable::vertical_alignment_property(),
                                    ContentControl::vertical_content_alignment_property(),
                                ),
                        )
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(Grid::column_property(), 1)
                        .set(
                            ContentPresenter::vertical_content_alignment_property(),
                            VerticalAlignment::Center,
                        )
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .template_bind(
                            ContentPresenter::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .template_bind(
                            ContentPresenter::font_size_property(),
                            TemplatedControl::font_size_property(),
                        )
                        .set(ContentPresenter::padding_property(), Thickness::new(4.0, 0.0, 0.0, 0.0))
                        .set(ContentPresenter::recognizes_access_key_property(), true)
                        .control(),
                ]),
        )
        .control())
}

/// `PaneToggleButtonStyle`: a control theme of `Button`.
fn pane_toggle_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.set(TemplatedControl::font_size_property(), 16.0);
    theme.static_resource(TemplatedControl::font_family_property(), "SymbolThemeFontFamily")?;
    theme.static_resource(Layoutable::min_height_property(), "PaneToggleButtonHeight")?;
    theme.static_resource(Layoutable::min_width_property(), "PaneToggleButtonWidth")?;
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::background_property(), "NavigationViewItemBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "NavigationViewItemForeground");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "NavigationViewToggleBorderThickness",
    );
    theme.dynamic(TemplatedControl::padding_property(), "NavigationViewItemButtonMargin");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(pane_toggle_button_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(
                Border::background_property(),
                "NavigationViewButtonBackgroundPointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "NavigationViewButtonForegroundPointerOver",
                );
                style.set(
                    ContentPresenter::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                style.set(
                    ContentPresenter::border_brush_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("Icon"), |style| {
            style.dynamic(Shape::fill_property(), "NavigationViewButtonForegroundPointerOver");
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "NavigationViewButtonBackgroundPressed");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "NavigationViewButtonForegroundPressed",
                );
                style.set(
                    ContentPresenter::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                style.set(
                    ContentPresenter::border_brush_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("Icon"), |style| {
            style.dynamic(Shape::fill_property(), "NavigationViewButtonForegroundPressed");
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled (the icon keeps its foreground, as in the source)
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "NavigationViewButtonBackgroundDisabled");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "NavigationViewButtonForegroundDisabled",
                );
                style.set(
                    ContentPresenter::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                style.set(
                    ContentPresenter::border_brush_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `ToggleButton` of `drawer_page_pane_toggle_button`.
fn drawer_page_pane_toggle_button_template(t: &Template<ToggleButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .template_bind(Layoutable::height_property(), Layoutable::min_height_property())
        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .set(
            Layoutable::horizontal_alignment_property(),
            HorizontalAlignment::Stretch,
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .template_bind(Layoutable::width_property(), Layoutable::min_width_property())
                .child(
                    Decorator::child_property(),
                    t.make(Panel::new()).children([
                        t.make(Path::new())
                            .name("Icon")?
                            .static_resource(Path::data_property(), "GlyphGeometryE700")?
                            .set(Layoutable::width_property(), 16.0)
                            .set(Layoutable::height_property(), 16.0)
                            .set(Shape::stretch_property(), Stretch::Uniform)
                            .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                            .template_bind(
                                Layoutable::vertical_alignment_property(),
                                ContentControl::vertical_content_alignment_property(),
                            )
                            .template_bind_with(
                                Visual::is_visible_property(),
                                Some(ContentControl::content_property().property()),
                                Some(ObjectConverters::is_null()),
                                None,
                                None,
                            )
                            .control(),
                        t.make(ContentPresenter::new())
                            .name("PART_ContentPresenter")?
                            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                            .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                            .template_bind(
                                ContentPresenter::content_template_property(),
                                ContentControl::content_template_property(),
                            )
                            .template_bind(
                                ContentPresenter::foreground_property(),
                                TemplatedControl::foreground_property(),
                            )
                            .template_bind_with(
                                Visual::is_visible_property(),
                                Some(ContentControl::content_property().property()),
                                Some(ObjectConverters::is_not_null()),
                                None,
                                None,
                            )
                            .control(),
                    ]),
                ),
        )
        .control())
}

/// `DrawerPagePaneToggleButton`: a control theme of `ToggleButton`.
fn drawer_page_pane_toggle_button(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
    theme.based_on("PaneToggleButtonStyle")?;
    theme.template(drawer_page_pane_toggle_button_template);
    Ok(())
}

/// The template of `ListBoxItem` of `drawer_page_navigation_view_item`.
fn drawer_page_navigation_view_item_template(t: &Template<ListBoxItem>) -> Result<Ref<Control>> {
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
                        Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("8,*"))?);
                        Ok(())
                    })?
                    .children([t
                        .make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(Grid::column_property(), 1)
                        .dynamic(
                            Layoutable::margin_property(),
                            "NavigationViewItemContentPresenterMargin",
                        )
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
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
                        .set(ContentPresenter::text_wrapping_property(), TextWrapping::NoWrap)
                        .control()])
                    .control(),
            ]),
        )
        .control())
}

/// `DrawerPageNavigationViewItem`: a control theme of `ListBoxItem`.
fn drawer_page_navigation_view_item(theme: &StyleBuilder) -> Result {
    theme.target::<ListBoxItem>();
    theme.dynamic(TemplatedControl::foreground_property(), "NavigationViewItemForeground");
    theme.dynamic(TemplatedControl::background_property(), "NavigationViewItemBackground");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "NavigationViewItemBorderBrush",
    );
    theme.static_resource(
        TemplatedControl::border_thickness_property(),
        "NavigationViewItemBorderThickness",
    )?;
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Local,
    );
    theme.template(drawer_page_navigation_view_item_template);
    // PointerStates: PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "NavigationViewItemBackgroundPointerOver");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
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
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "NavigationViewItemBackgroundPressed");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
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
    theme.style(nesting().class(":selected"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "NavigationViewItemBackgroundSelected");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
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
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(
                Border::background_property(),
                "NavigationViewItemBackgroundSelectedPointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
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
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(
                Border::background_property(),
                "NavigationViewItemBackgroundSelectedPressed",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
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
    theme.style(
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
}

/// The items panel of `drawer_page_navigation_view_menu`.
fn drawer_page_navigation_view_menu_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(StackPanel::new()).end().upcast())
}

/// The template of `ListBox` of `drawer_page_navigation_view_menu`.
fn drawer_page_navigation_view_menu_template(t: &Template<ListBox>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
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
        // MenuItemsScrollViewer
        .child(
            Decorator::child_property(),
            t.make(ScrollViewer::new())
                .name("PART_ScrollViewer")?
                .template_bind(
                    TemplatedControl::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .template_bind(
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                )
                .content(
                    ContentControl::content_property(),
                    t.make(ItemsPresenter::new())
                        .name("PART_ItemsPresenter")?
                        .template_bind(
                            ItemsPresenter::items_panel_property(),
                            ItemsControl::items_panel_property(),
                        ),
                )?,
        )
        .control())
}

/// `DrawerPageNavigationViewMenu`: a control theme of `ListBox`.
fn drawer_page_navigation_view_menu(theme: &StyleBuilder) -> Result {
    theme.target::<ListBox>();
    theme.dynamic(TemplatedControl::foreground_property(), "NavigationViewItemForeground");
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Disabled,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.static_resource(
        ItemsControl::item_container_theme_property(),
        "DrawerPageNavigationViewItem",
    )?;
    theme.set(
        ItemsControl::items_panel_property(),
        items_panel(theme.scope(), drawer_page_navigation_view_menu_items_panel),
    );
    theme.template(drawer_page_navigation_view_menu_template);
    Ok(())
}

/// The template of `DrawerPage` of `drawer_page_theme`.
fn drawer_page_theme_template(t: &Template<DrawerPage>) -> Result<Ref<Control>> {
    Ok(t.make(SplitView::new())
        .name("PART_SplitView")?
        .template_bind(
            TemplatedControl::background_property(),
            TemplatedControl::background_property(),
        )
        .dynamic(
            TemplatedControl::border_thickness_property(),
            "NavigationViewBorderThickness",
        )
        .template_bind_with(
            TemplatedControl::corner_radius_property(),
            Some(TemplatedControl::corner_radius_property().property()),
            Some(t.scope().converter("RightCornerRadiusFilterConverter")?),
            None,
            None,
        )
        .template_bind(SplitView::display_mode_property(), DrawerPage::display_mode_property())
        .template_bind(
            SplitView::open_pane_length_property(),
            DrawerPage::drawer_length_property(),
        )
        .template_bind(
            SplitView::compact_pane_length_property(),
            DrawerPage::compact_drawer_length_property(),
        )
        .bind(
            SplitView::is_pane_open_property(),
            path()
                .templated_parent()
                .prop(DrawerPage::is_open_property())
                .binding()
                .with_mode(BindingMode::TwoWay),
        )
        .content(
            SplitView::pane_property(),
            t.make(DockPanel::new())
                .name("PaneContentGrid")?
                .dynamic(Layoutable::margin_property(), "NavigationViewPaneContentGridMargin")
                // The pane toggle button in the compact modes.
                // PaneHeaderContentBorder
                // FooterContentBorder
                // The menu items area
                .children([
                    t.make(ToggleButton::new())
                        .name("PART_CompactPaneToggle")?
                        .set(DockPanel::dock_property(), Dock::Top)
                        .static_resource(StyledElement::theme_property(), "DrawerPagePaneToggleButton")?
                        .dynamic(Layoutable::margin_property(), "NavigationViewButtonHolderGridMargin")
                        .bind(
                            ToggleButton::is_checked_property(),
                            path()
                                .named(t.names(), "PART_SplitView")
                                .prop(SplitView::is_pane_open_property())
                                .binding()
                                .with_mode(BindingMode::TwoWay),
                        )
                        .template_bind(ContentControl::content_property(), DrawerPage::drawer_icon_property())
                        .template_bind(
                            ContentControl::content_template_property(),
                            DrawerPage::drawer_icon_template_property(),
                        )
                        .set(Visual::is_visible_property(), false)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                        .set(
                            AutomationProperties::name_property(),
                            Some(String::from("Toggle navigation drawer")),
                        )
                        .set(
                            ToolTip::tip_property(),
                            Some(value("Toggle navigation drawer".to_string()).expect("a value")),
                        )
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("PART_DrawerHeader")?
                        .set(DockPanel::dock_property(), Dock::Top)
                        .set(
                            ContentPresenter::horizontal_content_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(
                            ContentPresenter::vertical_content_alignment_property(),
                            VerticalAlignment::Stretch,
                        )
                        .template_bind(
                            ContentPresenter::content_property(),
                            DrawerPage::drawer_header_property(),
                        )
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            DrawerPage::drawer_header_template_property(),
                        )
                        .template_bind(
                            ContentPresenter::background_property(),
                            DrawerPage::drawer_header_background_property(),
                        )
                        .template_bind_with(
                            Visual::is_visible_property(),
                            Some(DrawerPage::drawer_header_property().property()),
                            Some(ObjectConverters::is_not_null()),
                            None,
                            None,
                        )
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("PART_DrawerFooter")?
                        .set(DockPanel::dock_property(), Dock::Bottom)
                        .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 0.0, 4.0))
                        .set(
                            ContentPresenter::horizontal_content_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(
                            ContentPresenter::vertical_content_alignment_property(),
                            VerticalAlignment::Stretch,
                        )
                        .template_bind(
                            ContentPresenter::content_property(),
                            DrawerPage::drawer_footer_property(),
                        )
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            DrawerPage::drawer_footer_template_property(),
                        )
                        .template_bind(
                            ContentPresenter::background_property(),
                            DrawerPage::drawer_footer_background_property(),
                        )
                        .template_bind_with(
                            Visual::is_visible_property(),
                            Some(DrawerPage::drawer_footer_property().property()),
                            Some(ObjectConverters::is_not_null()),
                            None,
                            None,
                        )
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("PART_DrawerPresenter")?
                        .set(
                            ContentPresenter::horizontal_content_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(
                            ContentPresenter::vertical_content_alignment_property(),
                            VerticalAlignment::Stretch,
                        )
                        .template_bind(ContentPresenter::content_property(), DrawerPage::drawer_property())
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            DrawerPage::drawer_template_property(),
                        )
                        .control(),
                ]),
        )?
        .content(
            ContentControl::content_property(),
            t.make(Grid::new())
                .set(
                    Layoutable::horizontal_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                // ContentGrid
                .children([
                    t.make(Border::new())
                        .name("ContentGrid")?
                        .dynamic(Border::border_brush_property(), "NavigationViewContentGridBorderBrush")
                        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                        .dynamic(
                            Border::border_thickness_property(),
                            "NavigationViewContentGridBorderThickness",
                        )
                        .dynamic(Border::background_property(), "NavigationViewContentBackground")
                        .dynamic(Layoutable::margin_property(), "NavigationViewContentMargin")
                        .dynamic(
                            Border::corner_radius_property(),
                            "NavigationViewContentGridCornerRadius",
                        )
                        .set(Visual::clip_to_bounds_property(), true)
                        .child(
                            Decorator::child_property(),
                            t.make(DockPanel::new())
                                .name("PART_ContentDock")?
                                // The header row: the pane toggle button and HeaderContent.
                                // The same row at the bottom (the drawer at the bottom).
                                .children([
                                    t.make(Border::new())
                                        .name("PART_TopBar")?
                                        .set(DockPanel::dock_property(), Dock::Top)
                                        .dynamic(Decorator::padding_property(), "NavigationViewButtonHolderGridMargin")
                                        .child(
                                            Decorator::child_property(),
                                            t.make(DockPanel::new())
                                                .set(
                                                    Layoutable::horizontal_alignment_property(),
                                                    HorizontalAlignment::Stretch,
                                                )
                                                .children([
                                                    t.make(ToggleButton::new())
                                                        .name("PART_PaneButton")?
                                                        .static_resource(
                                                            StyledElement::theme_property(),
                                                            "DrawerPagePaneToggleButton",
                                                        )?
                                                        .bind(
                                                            ToggleButton::is_checked_property(),
                                                            path()
                                                                .named(t.names(), "PART_SplitView")
                                                                .prop(SplitView::is_pane_open_property())
                                                                .binding()
                                                                .with_mode(BindingMode::TwoWay),
                                                        )
                                                        .template_bind(
                                                            ContentControl::content_property(),
                                                            DrawerPage::drawer_icon_property(),
                                                        )
                                                        .template_bind(
                                                            ContentControl::content_template_property(),
                                                            DrawerPage::drawer_icon_template_property(),
                                                        )
                                                        .set(DockPanel::dock_property(), Dock::Left)
                                                        .set(
                                                            AutomationProperties::name_property(),
                                                            Some(String::from("Toggle navigation drawer")),
                                                        )
                                                        .set(
                                                            ToolTip::tip_property(),
                                                            Some(
                                                                value("Toggle navigation drawer".to_string())
                                                                    .expect("a value"),
                                                            ),
                                                        )
                                                        .control(),
                                                    t.make(ContentControl::new())
                                                        .name("PART_TitlePresenter")?
                                                        .static_resource(
                                                            StyledElement::theme_property(),
                                                            "NavigationViewTitleHeaderContentControlTextStyle",
                                                        )?
                                                        .template_bind(
                                                            ContentControl::content_property(),
                                                            Page::header_property(),
                                                        )
                                                        .template_bind(
                                                            ContentControl::content_template_property(),
                                                            Page::header_template_property(),
                                                        )
                                                        .set(
                                                            Layoutable::vertical_alignment_property(),
                                                            VerticalAlignment::Center,
                                                        )
                                                        .set(
                                                            ContentControl::vertical_content_alignment_property(),
                                                            VerticalAlignment::Center,
                                                        )
                                                        .set(
                                                            Layoutable::margin_property(),
                                                            Thickness::new(8.0, 0.0, 0.0, 0.0),
                                                        )
                                                        .template_bind_with(
                                                            Visual::is_visible_property(),
                                                            Some(Page::header_property().property()),
                                                            Some(ObjectConverters::is_not_null()),
                                                            None,
                                                            None,
                                                        )
                                                        .control(),
                                                ]),
                                        )
                                        .control(),
                                    t.make(Border::new())
                                        .name("PART_BottomBar")?
                                        .set(DockPanel::dock_property(), Dock::Bottom)
                                        .dynamic(Decorator::padding_property(), "NavigationViewButtonHolderGridMargin")
                                        .set(Visual::is_visible_property(), false)
                                        .child(
                                            Decorator::child_property(),
                                            t.make(DockPanel::new())
                                                .set(
                                                    Layoutable::horizontal_alignment_property(),
                                                    HorizontalAlignment::Stretch,
                                                )
                                                .children([
                                                    t.make(ToggleButton::new())
                                                        .name("PART_BottomPaneButton")?
                                                        .static_resource(
                                                            StyledElement::theme_property(),
                                                            "DrawerPagePaneToggleButton",
                                                        )?
                                                        .bind(
                                                            ToggleButton::is_checked_property(),
                                                            path()
                                                                .named(t.names(), "PART_SplitView")
                                                                .prop(SplitView::is_pane_open_property())
                                                                .binding()
                                                                .with_mode(BindingMode::TwoWay),
                                                        )
                                                        .template_bind(
                                                            ContentControl::content_property(),
                                                            DrawerPage::drawer_icon_property(),
                                                        )
                                                        .template_bind(
                                                            ContentControl::content_template_property(),
                                                            DrawerPage::drawer_icon_template_property(),
                                                        )
                                                        .set(DockPanel::dock_property(), Dock::Left)
                                                        .set(
                                                            AutomationProperties::name_property(),
                                                            Some(String::from("Toggle navigation drawer")),
                                                        )
                                                        .set(
                                                            ToolTip::tip_property(),
                                                            Some(
                                                                value("Toggle navigation drawer".to_string())
                                                                    .expect("a value"),
                                                            ),
                                                        )
                                                        .control(),
                                                    t.make(ContentControl::new())
                                                        .name("PART_BottomTitlePresenter")?
                                                        .static_resource(
                                                            StyledElement::theme_property(),
                                                            "NavigationViewTitleHeaderContentControlTextStyle",
                                                        )?
                                                        .template_bind(
                                                            ContentControl::content_property(),
                                                            Page::header_property(),
                                                        )
                                                        .template_bind(
                                                            ContentControl::content_template_property(),
                                                            Page::header_template_property(),
                                                        )
                                                        .set(
                                                            Layoutable::vertical_alignment_property(),
                                                            VerticalAlignment::Center,
                                                        )
                                                        .set(
                                                            ContentControl::vertical_content_alignment_property(),
                                                            VerticalAlignment::Center,
                                                        )
                                                        .set(
                                                            Layoutable::margin_property(),
                                                            Thickness::new(8.0, 0.0, 0.0, 0.0),
                                                        )
                                                        .template_bind_with(
                                                            Visual::is_visible_property(),
                                                            Some(Page::header_property().property()),
                                                            Some(ObjectConverters::is_not_null()),
                                                            None,
                                                            None,
                                                        )
                                                        .control(),
                                                ]),
                                        )
                                        .control(),
                                    t.make(ContentPresenter::new())
                                        .name("PART_ContentPresenter")?
                                        .dynamic(Layoutable::margin_property(), "NavigationViewContentPresenterMargin")
                                        .template_bind(
                                            ContentPresenter::horizontal_content_alignment_property(),
                                            DrawerPage::horizontal_content_alignment_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::vertical_content_alignment_property(),
                                            DrawerPage::vertical_content_alignment_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::background_property(),
                                            TemplatedControl::background_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::border_brush_property(),
                                            TemplatedControl::border_brush_property(),
                                        )
                                        .set(
                                            ContentPresenter::background_sizing_property(),
                                            BackgroundSizing::InnerBorderEdge,
                                        )
                                        .template_bind(
                                            ContentPresenter::border_thickness_property(),
                                            TemplatedControl::border_thickness_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::content_property(),
                                            DrawerPage::content_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::content_template_property(),
                                            DrawerPage::content_template_property(),
                                        )
                                        .control(),
                                ]),
                        )
                        .control(),
                    t.make(Border::new())
                        .name("PART_Backdrop")?
                        .template_bind(Border::background_property(), DrawerPage::backdrop_brush_property())
                        .set(Visual::is_visible_property(), false)
                        .set(InputElement::is_hit_test_visible_property(), false)
                        .control(),
                ]),
        )?
        .control())
}

/// The default control theme of `DrawerPage`.
fn drawer_page_theme(theme: &StyleBuilder) -> Result {
    theme.target::<DrawerPage>();
    theme.dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius");
    theme.template(drawer_page_theme_template);
    // RootSplitView: PaneOverlaying (the default) and PaneNotOverlaying.
    theme.style(nesting().template().of::<SplitView>().name("PART_SplitView"), |style| {
        style.dynamic(
            SplitView::pane_background_property(),
            "NavigationViewDefaultPaneBackground",
        );
        style.dynamic(
            TemplatedControl::border_brush_property(),
            "NavigationViewItemSeparatorForeground",
        );
        Ok(())
    })?;
    theme.style(
        Selectors::or([
            nesting()
                .property_is(DrawerPage::is_open_property(), false)
                .template()
                .of::<SplitView>()
                .name("PART_SplitView"),
            nesting()
                .property_is(DrawerPage::display_mode_property(), SplitViewDisplayMode::Inline)
                .template()
                .of::<SplitView>()
                .name("PART_SplitView"),
            nesting()
                .property_is(DrawerPage::display_mode_property(), SplitViewDisplayMode::CompactInline)
                .template()
                .of::<SplitView>()
                .name("PART_SplitView"),
        ]),
        |style| {
            style.dynamic(
                SplitView::pane_background_property(),
                "NavigationViewExpandedPaneBackground",
            );
            style.set(
                TemplatedControl::border_brush_property(),
                Some(literal(Brush::parse("Transparent"))?),
            );
            Ok(())
        },
    )?;
    // ContentGrid in the Minimal display mode (Overlay) and the other placements.
    theme.style(
        nesting()
            .property_is(DrawerPage::display_mode_property(), SplitViewDisplayMode::Overlay)
            .template()
            .of::<Border>()
            .name("ContentGrid"),
        |style| {
            style.dynamic(
                Border::border_thickness_property(),
                "NavigationViewMinimalContentGridBorderThickness",
            );
            style.dynamic(
                Border::corner_radius_property(),
                "NavigationViewMinimalContentGridCornerRadius",
            );
            style.dynamic(Layoutable::margin_property(), "NavigationViewMinimalContentMargin");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":placement-right")
            .template()
            .of::<Border>()
            .name("ContentGrid"),
        |style| {
            style.set(Border::border_thickness_property(), Thickness::new(0.0, 1.0, 1.0, 0.0));
            style.set(Border::corner_radius_property(), CornerRadius::new(0.0, 8.0, 0.0, 0.0));
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":placement-top")
            .template()
            .of::<Border>()
            .name("ContentGrid"),
        |style| {
            style.dynamic(
                Border::border_thickness_property(),
                "TopNavigationViewContentGridBorderThickness",
            );
            style.dynamic(
                Border::corner_radius_property(),
                "TopNavigationViewContentGridCornerRadius",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":placement-bottom")
            .template()
            .of::<Border>()
            .name("ContentGrid"),
        |style| {
            style.set(Border::border_thickness_property(), Thickness::new(0.0, 0.0, 0.0, 1.0));
            style.dynamic(
                Border::corner_radius_property(),
                "TopNavigationViewContentGridCornerRadius",
            );
            Ok(())
        },
    )?;
    // The pane toggle button at the right with the drawer at the right.
    theme.style(
        nesting()
            .class(":placement-right")
            .template()
            .of::<ToggleButton>()
            .name("PART_PaneButton"),
        |style| {
            style.set(DockPanel::dock_property(), Dock::Right);
            Ok(())
        },
    )?;
    // The drawer at the bottom: the bottom bar replaces the top bar.
    theme.style(
        nesting()
            .class(":placement-bottom")
            .template()
            .of::<Border>()
            .name("PART_TopBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":placement-bottom")
            .template()
            .of::<Border>()
            .name("PART_BottomBar"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // The drawer at the bottom in the compact modes: the compact strip has the toggle.
    theme.style(
        nesting()
            .property_is(
                DrawerPage::display_mode_property(),
                SplitViewDisplayMode::CompactOverlay,
            )
            .class(":placement-bottom")
            .template()
            .of::<Border>()
            .name("PART_BottomBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(DrawerPage::display_mode_property(), SplitViewDisplayMode::CompactInline)
            .class(":placement-bottom")
            .template()
            .of::<Border>()
            .name("PART_BottomBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // A horizontal pane (top or bottom): the compact toggle docks left so that the drawer fills
    //         the rest.
    theme.style(
        nesting()
            .class(":placement-bottom")
            .template()
            .of::<ToggleButton>()
            .name("PART_CompactPaneToggle"),
        |style| {
            style.set(DockPanel::dock_property(), Dock::Left);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":placement-top")
            .template()
            .of::<ToggleButton>()
            .name("PART_CompactPaneToggle"),
        |style| {
            style.set(DockPanel::dock_property(), Dock::Left);
            Ok(())
        },
    )?;
    // The conditions of FerroUI's DrawerPage that hide the top bar.
    theme.style(
        nesting()
            .property_is(DrawerPage::drawer_behavior_property(), DrawerBehavior::Locked)
            .template()
            .of::<Border>()
            .name("PART_TopBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(DrawerPage::drawer_behavior_property(), DrawerBehavior::Disabled)
            .template()
            .of::<Border>()
            .name("PART_TopBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":detail-is-navpage")
            .template()
            .of::<Border>()
            .name("PART_TopBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(
                DrawerPage::display_mode_property(),
                SplitViewDisplayMode::CompactOverlay,
            )
            .template()
            .of::<Border>()
            .name("PART_TopBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(
                DrawerPage::display_mode_property(),
                SplitViewDisplayMode::CompactOverlay,
            )
            .template()
            .of::<ToggleButton>()
            .name("PART_CompactPaneToggle"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(DrawerPage::display_mode_property(), SplitViewDisplayMode::CompactInline)
            .template()
            .of::<Border>()
            .name("PART_TopBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(DrawerPage::display_mode_property(), SplitViewDisplayMode::CompactInline)
            .template()
            .of::<ToggleButton>()
            .name("PART_CompactPaneToggle"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(Page::is_in_navigation_page_property(), true)
            .template()
            .of::<Border>()
            .name("PART_TopBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // A NavigationPage in the content has its own drawer toggle.
    theme.style(
        nesting()
            .class(":detail-is-navpage")
            .template()
            .of::<ToggleButton>()
            .name("PART_CompactPaneToggle"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    Ok(())
}
