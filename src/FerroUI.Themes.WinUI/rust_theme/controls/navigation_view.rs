//! `Controls/NavigationView.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/NavigationView/NavigationView.xaml (the implicit styles of NavigationView,
//!   NavigationViewItem, NavigationViewItemHeader and NavigationViewItemSeparator)
//!   controls/dev/NavigationView/NavigationView_themeresources.xaml
//!   (MUX_NavigationViewItemPresenterStyleWhenOnLeftPane and
//!   MUX_NavigationViewItemPresenterStyleWhenOnTopPane, folded into the template of
//!   NavigationViewItem, and NavigationViewPaneSearchButtonStyle; the resources are
//!   Resources/NavigationView_themeresources.xaml, PaneToggleButtonStyle and the two text styles are
//!   in Controls/DrawerPage.xaml, the back button styles in Controls/NavigationPage.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The templates of WinUI's NavigationView and its items, which FerroUI ports as controls
//! (ferroui-controls, docs/porting/winui-controls.md, section 10.4); the Fluent theme has no
//! document for them. DrawerPage and TabbedPage keep their own templates, drawn after the left and
//! the top pane of this one (Controls/DrawerPage.xaml, Controls/TabbedPage.xaml).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4; DEVIATIONS.md, NavigationView):
//! - The visual states are the pseudo-classes the controls set (NavigationView.cpp and
//!   NavigationViewItem.cpp call GoToState): DisplayModeGroup :compact, :expanded, :minimal,
//!   :top-navigation-minimal, :minimal-with-back-button; :toggle-pane-button-visible;
//!   :header-collapsed; :auto-suggest-box-collapsed; :closed-compact; :list-size-compact;
//!   :pane-not-overlaying (PaneOverlaying is the base); :back-button-collapsed; :pane-collapsed.
//!   NavigationView sets the height of the row of the back button of PaneContentGrid in code
//!   (c_backButtonRowDefinition): the spacer BackButtonRowSpacer of that row gets it in
//!   :back-button-row-visible. The values the template binds to are the TemplateSettings of the
//!   control (NavigationViewTemplateSettings, the visibilities as booleans).
//! - The ItemsRepeaters (MenuItemsHost, FooterMenuItemsHost, TopNavMenuItemsHost,
//!   TopFooterMenuItemsHost, NavigationViewItemMenuItemsHost) are StackPanels whose children the
//!   controls realize; the ItemsRepeaterScrollHosts are left out.
//! - The overflow of the top pane (TopNavOverflowButton, its flyout and TopNavMenuItemsOverflowHost,
//!   OverflowLabelGroup) is not ported; the items of the top pane stay in TopNavMenuItemsHost.
//! - ShadowCaster and the transitions of PaneOverlayGroup (the shadow of the overlaying pane) are
//!   left out (ThemeShadow, follow-up F3), as is the shadow of the flyout of the children of an
//!   item. TitleBarVisibilityGroup (the title bar of an extended window) and PaneSeparatorStates (the
//!   separator of overflowing menu items) are left out: the controls never go to their states.
//! - Grids with a BorderBrush and no BorderThickness (TopNavGrid, PaneContentGrid) draw nothing:
//!   the brush is left out. The ContentGrid of the content (border, background, corner radius) is a
//!   Border around a Grid (G-17).
//! - ColumnDefinitions bound to resources (BackButtonPlaceholderOnTopNav, the MinWidth of the
//!   custom content column of the top pane) are a placeholder Panel of that width in an Auto column
//!   and the MinWidth of PaneCustomContentOnTopPane; the named columns and rows WinUI sizes in code
//!   (PaneHeaderCloseButtonColumn, PaneHeaderToggleButtonColumn, PaneHeaderContentBorderRow) are the
//!   left margin NavigationView gives PaneHeaderContentBorder and the MinHeight of
//!   PaneContentGridToggleButtonRow.
//! - The ContentPresenter of the content is PART_ContentPresenter (FerroUI's NavigationView is a
//!   ContentControl). TopNavContentOverlayAreaGrid (a Border whose Child is ContentOverlay) is a
//!   ContentPresenter of the overlay.
//! - The Button.Resources of PaneAutoSuggestButton (theme dictionaries mapping the Button brushes to
//!   NavigationViewItem brushes) are the states of NavigationViewPaneSearchButtonStyle.
//! - NavigationViewItemPresenter has no FerroUI control: the template of NavigationViewItem holds
//!   its template for the left pane in the element NavigationViewItemPresenter, and the states
//!   OnTopNavigationPrimary (:on-top-navigation-primary) restyle it as the presenter of the top
//!   pane. The presenter uses :pointerover on LayoutRoot and these pseudo-classes of the item: :pressed,
//!   :selected, :disabled; :icon-collapsed (IconCollapsed), :icon-only and :content-only
//!   (NavigationViewIconPositionStates); :info-badge-collapsed; :chevron-visible-open,
//!   :chevron-visible-closed; :closed-compact-and-top-level-item. The opacity of SelectionIndicator
//!   is set by the control (ShowSelectionIndicator). The width of IconColumn is the
//!   SmallerPaneToggleButtonWidth of the navigation view (the SmallerIconWidth of the presenter's
//!   TemplateSettings has the same value). The right margin of ContentGrid is on the Border
//!   ContentGridHost around it: the control indents ContentGrid with its left margin.
//!   WinUI: MUX_NavigationViewItemPresenterStyleWhenOnLeftPane, MUX_NavigationViewItemPresenterStyleWhenOnTopPane
//! - The settings item of the top pane (MUX_NavigationViewSettingsItemStyleWhenOnTopPane, a 40 by 40
//!   item with the icon and the badge) is an item of the top pane without content: the IconOnly
//!   state of the presenter of the top pane (36 by 36).
//!   WinUI: MUX_NavigationViewSettingsItemStyleWhenOnTopPane
//! - The AnimatedIcon ExpandCollapseChevronIcon (AnimatedChevronUpDownSmall) is the TextBlock of its
//!   fallback glyph NavigationViewItemExpandedGlyph in SymbolThemeFontFamily, turned by 180 degrees
//!   while the children are shown. The AnimatedIcon.State setters are left out.
//! - The Flyout ChildrenFlyout (the FlyoutBase.AttachedFlyout of NVIRootGrid) is a Popup with the
//!   border of its FlyoutPresenterStyle; the control opens it and moves NavigationViewItemMenuItemsHost
//!   into FlyoutContentGrid, as WinUI moves its repeater.
//! - The text of NavigationViewItemHeader (a TextBlock bound to Content with
//!   NavigationViewItemHeaderTextStyle) is a ContentPresenter HeaderText with the font of that style.
//! - The transitions of the header (HeaderTextVisible and HeaderTextCollapsed) are left out.
//! - UseSystemFocusVisuals is the default focus adorner; Control.IsTemplateFocusTarget,
//!   x:Load, x:DeferLoadStrategy, ContentTransitions, XYFocusKeyboardNavigation,
//!   AutomationProperties.LandmarkType, ElementSoundMode and the scroll modes are left out.

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
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::FontFamily;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextAlignment;
use ferroui_base::media::TextWrapping;
use ferroui_base::styling::Selectors;
use ferroui_base::RelativePoint;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::automation::AccessibilityView;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::NavigationViewTemplateSettings;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::NavigationView;
use ferroui_controls::NavigationViewItem;
use ferroui_controls::NavigationViewItemHeader;
use ferroui_controls::NavigationViewItemSeparator;
use ferroui_controls::Panel;
use ferroui_controls::PathIcon;
use ferroui_controls::PlacementMode;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_controls::SelectionIndicator;
use ferroui_controls::SplitView;
use ferroui_controls::SplitViewDisplayMode;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_controls::ToolTip;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The search button of the closed compact pane (PaneAutoSuggestButton), with the
    // NavigationViewItem brushes of the Button.Resources of the template.
    resources.control_theme(
        "NavigationViewPaneSearchButtonStyle",
        navigation_view_pane_search_button_style,
    );
    resources.control_theme(type_key::<NavigationView>(), navigation_view_theme);
    resources.control_theme(type_key::<NavigationViewItem>(), navigation_view_item_theme);
    resources.control_theme(
        type_key::<NavigationViewItemHeader>(),
        navigation_view_item_header_theme,
    );
    resources.control_theme(
        type_key::<NavigationViewItemSeparator>(),
        navigation_view_item_separator_theme,
    );
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

/// The focus adorner of `navigation_view_pane_search_button_style`.
fn navigation_view_pane_search_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::symmetric(-4.0, 0.0))
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

/// `NavigationViewPaneSearchButtonStyle`: a control theme of `Button`.
fn navigation_view_pane_search_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "NavigationViewToggleBorderThickness",
    );
    theme.dynamic(TemplatedControl::background_property(), "NavigationViewItemBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "NavigationViewItemForeground");
    theme.set(
        ContentControl::content_property(),
        Some(value("".to_string()).expect("a value")),
    );
    theme.set(Layoutable::min_height_property(), 36.0);
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    // FocusVisualMargin -4,0 (section 4.6).
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            navigation_view_pane_search_button_style_focus_adorner,
        )),
    );
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "NavigationViewItemBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "NavigationViewItemBorderBrushPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "NavigationViewItemForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "NavigationViewItemBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "NavigationViewItemBorderBrushPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "NavigationViewItemForegroundPressed",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "NavigationViewItemBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "NavigationViewItemBorderBrushDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "NavigationViewItemForegroundDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `NavigationView` of `navigation_view_theme`.
fn navigation_view_theme_template(t: &Template<NavigationView>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new()).name("RootGrid")?
// Button grid

// Content layouts
.children([t.make(Grid::new()).name("PaneToggleButtonGrid")?.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top).set(Visual::z_index_property(), 100).with(|e| { Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,Auto"))?); Ok(()) })?.children([t.make(Panel::new()).name("TogglePaneTopPadding")?.bind(Layoutable::height_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::top_padding_property()).binding()).control(), t.make(Grid::new()).name("ButtonHolderGrid")?.set(Grid::row_property(), 1).with(|e| { Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*"))?); Ok(()) })?.dynamic(Layoutable::margin_property(), "NavigationViewButtonHolderGridMargin").children([t.make(Button::new()).name("NavigationViewBackButton")?.static_resource(StyledElement::theme_property(), "NavigationBackButtonNormalStyle")?.bind(Layoutable::width_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::smaller_pane_toggle_button_width_property()).binding()).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top).bind(Visual::is_visible_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::back_button_visibility_property()).binding()).template_bind(InputElement::is_enabled_property(), NavigationView::is_back_enabled_property()).set(ToolTip::tip_property(), Some(value("Back".to_string()).expect("a value"))).control(), t.make(Button::new()).name("NavigationViewCloseButton")?.static_resource(StyledElement::theme_property(), "NavigationBackButtonNormalStyle")?.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top).set(ToolTip::tip_property(), Some(value("Close Navigation".to_string()).expect("a value"))).control(), t.make(Button::new()).name("TogglePaneButton")?.template_bind(StyledElement::theme_property(), NavigationView::pane_toggle_button_style_property()).bind(Visual::is_visible_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::pane_toggle_button_visibility_property()).binding()).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top).bind(Layoutable::min_width_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::smaller_pane_toggle_button_width_property()).binding()).content(ContentControl::content_property(), t.make(TextBlock::new()).name("PaneTitleTextBlock")?.set(Layoutable::margin_property(), Thickness::new(0.0, -2.0, 0.0, 0.0)).template_bind(TextBlock::text_property(), NavigationView::pane_title_property()).set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center).static_resource(StyledElement::theme_property(), "NavigationViewItemHeaderTextStyle")?)?.control(), t.make(Grid::new()).name("PaneTitleHolder")?.set(Layoutable::height_property(), 40.0).set(Visual::is_visible_property(), false).children([t.make(ContentControl::new()).name("PaneTitlePresenter")?.dynamic(Layoutable::margin_property(), "NavigationViewPaneTitlePresenterMargin").set(InputElement::is_tab_stop_property(), false).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch).control()]).control()]).control()]).control(), t.make(Grid::new()).with(|e| { Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,*"))?); Ok(()) })?
// DisplayMode top

// DisplayMode (compact/minimal/normal) left
.children([t.make(StackPanel::new()).name("TopNavArea")?.dynamic(Panel::background_property(), "NavigationViewTopPaneBackground").set(Grid::row_property(), 0).set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Stretch).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top).set(Visual::z_index_property(), 1).children([t.make(Panel::new()).name("TopNavTopPadding")?.bind(Layoutable::height_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::top_padding_property()).binding()).bind(Visual::is_visible_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::top_pane_visibility_property()).binding()).control(), t.make(Grid::new()).name("TopNavGrid")?.dynamic(Layoutable::height_property(), "NavigationViewTopPaneHeight").dynamic(Layoutable::margin_property(), "TopNavigationViewTopNavGridMargin").bind(Visual::is_visible_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::top_pane_visibility_property()).binding()).with(|e| { Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,Auto,Auto,Auto,Auto,*,Auto,Auto,Auto"))?); Ok(()) })?
// Top nav ItemsRepeater

// Top footer menu ItemsRepeater
.children([t.make(Panel::new()).name("BackButtonPlaceholderOnTopNav")?.dynamic(Layoutable::width_property(), "NavigationBackButtonWidth").control(), t.make(Panel::new()).name("TopNavLeftPadding")?.set(Grid::column_property(), 1).set(Layoutable::width_property(), 0.0).control(), t.make(ContentControl::new()).name("PaneHeaderOnTopPane")?.set(InputElement::is_tab_stop_property(), false).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).set(Grid::column_property(), 2).control(), t.make(ContentControl::new()).name("PaneTitleOnTopPane")?.set(InputElement::is_tab_stop_property(), false).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).dynamic(Layoutable::margin_property(), "NavigationViewItemInnerHeaderMargin").set(Grid::column_property(), 2).control(), t.make(ScrollViewer::new()).set(Grid::column_property(), 3).set(ScrollViewer::horizontal_scroll_bar_visibility_property(), ScrollBarVisibility::Disabled).set(ScrollViewer::vertical_scroll_bar_visibility_property(), ScrollBarVisibility::Disabled).content(ContentControl::content_property(), t.make(StackPanel::new()).name("TopNavMenuItemsHost")?.set(StackPanel::orientation_property(), Orientation::Horizontal))?.control(), t.make(ContentControl::new()).name("PaneCustomContentOnTopPane")?.set(InputElement::is_tab_stop_property(), false).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).dynamic(Layoutable::min_width_property(), "TopNavigationViewPaneCustomContentMinWidth").set(Grid::column_property(), 5).control(), t.make(Grid::new()).name("TopPaneAutoSuggestArea")?.dynamic(Layoutable::height_property(), "NavigationViewTopPaneHeight").set(Grid::column_property(), 6).children([t.make(ContentControl::new()).name("TopPaneAutoSuggestBoxPresenter")?.dynamic(Layoutable::margin_property(), "TopNavigationViewAutoSuggestBoxMargin").set(Layoutable::min_width_property(), 216.0).set(InputElement::is_tab_stop_property(), false).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Center).control()]).control(), t.make(ContentControl::new()).name("PaneFooterOnTopPane")?.set(InputElement::is_tab_stop_property(), false).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).set(Grid::column_property(), 7).control(), t.make(StackPanel::new()).name("TopFooterMenuItemsHost")?.set(Grid::column_property(), 8).set(StackPanel::orientation_property(), Orientation::Horizontal).control()]).control(), t.make(ContentPresenter::new()).name("TopNavContentOverlayAreaGrid")?.template_bind(ContentPresenter::content_property(), NavigationView::content_overlay_property()).control()]).control(), t.make(SplitView::new()).name("RootSplitView")?.template_bind(TemplatedControl::background_property(), TemplatedControl::background_property()).dynamic(TemplatedControl::border_brush_property(), "NavigationViewItemSeparatorForeground").dynamic(TemplatedControl::border_thickness_property(), "NavigationViewBorderThickness").template_bind(SplitView::compact_pane_length_property(), NavigationView::compact_pane_length_property()).set(SplitView::display_mode_property(), SplitViewDisplayMode::Inline).set(InputElement::is_tab_stop_property(), false).bind(SplitView::open_pane_length_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::open_pane_length_property()).binding()).dynamic(SplitView::pane_background_property(), "NavigationViewDefaultPaneBackground").set(Grid::row_property(), 1).template_bind_with(TemplatedControl::corner_radius_property(), Some(TemplatedControl::corner_radius_property().property()), Some(t.scope().converter("RightCornerRadiusFilterConverter")?), None, None).content(SplitView::pane_property(), t.make(Grid::new()).name("PaneContentGrid")?.dynamic(Layoutable::margin_property(), "NavigationViewPaneContentGridMargin").bind(Visual::is_visible_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::left_pane_visibility_property()).binding()).with(|e| { Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,Auto,Auto,Auto,Auto,0,*"))?); Ok(()) })?
// above button margin + back button space

// "Non header" content
.children([t.make(Panel::new()).name("ContentPaneTopPadding")?.bind(Layoutable::height_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::top_padding_property()).binding()).control(), t.make(Panel::new()).name("BackButtonRowSpacer")?.set(Grid::row_property(), 1).set(Layoutable::height_property(), 0.0).control(), t.make(Panel::new()).name("PaneContentGridToggleButtonRow")?.set(Grid::row_property(), 2).children([t.make(ContentControl::new()).name("PaneHeaderContentBorder")?.set(InputElement::is_tab_stop_property(), false).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).control()]).control(), t.make(Grid::new()).name("AutoSuggestArea")?.set(Grid::row_property(), 3).dynamic(Layoutable::min_height_property(), "NavigationViewAutoSuggestAreaHeight").set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 0.0, 8.0)).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center).children([t.make(ContentControl::new()).name("PaneAutoSuggestBoxPresenter")?.dynamic(Layoutable::margin_property(), "NavigationViewAutoSuggestBoxMargin").set(InputElement::is_tab_stop_property(), false).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Center).control(), t.make(Button::new()).name("PaneAutoSuggestButton")?.set(Visual::is_visible_property(), false).static_resource(StyledElement::theme_property(), "NavigationViewPaneSearchButtonStyle")?.bind(Layoutable::width_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::smaller_pane_toggle_button_width_property()).binding()).control()]).control(), t.make(ContentControl::new()).name("PaneCustomContentBorder")?.set(InputElement::is_tab_stop_property(), false).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).set(Grid::row_property(), 4).control(), t.make(Grid::new()).name("ItemsContainerGrid")?.set(Grid::row_property(), 6).with(|e| { Grid::set_row_definitions(e, literal(RowDefinitions::parse("*,Auto,Auto,Auto"))?); Ok(()) })?
// MenuItems

// PaneFooter

// FooterItems
.children([t.make(ScrollViewer::new()).name("MenuItemsScrollViewer")?.set(KeyboardNavigation::tab_navigation_property(), KeyboardNavigationMode::Local).set(ScrollViewer::horizontal_scroll_bar_visibility_property(), ScrollBarVisibility::Disabled).set(ScrollViewer::vertical_scroll_bar_visibility_property(), ScrollBarVisibility::Auto).content(ContentControl::content_property(), t.make(StackPanel::new()).name("MenuItemsHost")?)?.control(), t.make(NavigationViewItemSeparator::new()).name("VisualItemsSeparator")?.set(Grid::row_property(), 1).set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 0.0, 2.0)).set(Visual::is_visible_property(), false).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center).set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Stretch).control(), t.make(ContentControl::new()).name("FooterContentBorder")?.set(InputElement::is_tab_stop_property(), false).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 0.0, 4.0)).set(Grid::row_property(), 2).control(), t.make(ScrollViewer::new()).name("FooterItemsScrollViewer")?.set(Grid::row_property(), 3).set(ScrollViewer::horizontal_scroll_bar_visibility_property(), ScrollBarVisibility::Disabled).set(ScrollViewer::vertical_scroll_bar_visibility_property(), ScrollBarVisibility::Auto).content(ContentControl::content_property(), t.make(StackPanel::new()).name("FooterMenuItemsHost")?)?.control()]).control()]))?.content(ContentControl::content_property(), t.make(Border::new()).name("ContentGrid")?.dynamic(Border::border_brush_property(), "NavigationViewContentGridBorderBrush").dynamic(Border::border_thickness_property(), "NavigationViewContentGridBorderThickness").set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge).dynamic(Border::background_property(), "NavigationViewContentBackground").dynamic(Layoutable::margin_property(), "NavigationViewContentMargin").dynamic(Border::corner_radius_property(), "NavigationViewContentGridCornerRadius").child(Decorator::child_property(), t.make(Grid::new()).with(|e| { Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,Auto,*"))?); Ok(()) })?.with(|e| { Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*"))?); Ok(()) })?.children([t.make(Panel::new()).name("ContentTopPadding")?.set(Grid::column_span_property(), 2).bind(Layoutable::height_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::top_padding_property()).binding()).bind(Visual::is_visible_property(), path().templated_parent().prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::left_pane_visibility_property()).binding()).control(), t.make(Panel::new()).name("ContentLeftPadding")?.set(Grid::row_property(), 1).control(), t.make(ContentControl::new()).name("HeaderContent")?.set(Grid::row_property(), 1).set(Grid::column_property(), 1).static_resource(Layoutable::min_height_property(), "PaneToggleButtonHeight")?.set(InputElement::is_tab_stop_property(), false).template_bind(ContentControl::content_property(), NavigationView::header_property()).template_bind(ContentControl::content_template_property(), NavigationView::header_template_property()).set(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch).set(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch).static_resource(StyledElement::theme_property(), "NavigationViewTitleHeaderContentControlTextStyle")?.control(), t.make(ContentPresenter::new()).name("PART_ContentPresenter")?.set(Grid::row_property(), 2).set(Grid::column_span_property(), 2).template_bind(ContentPresenter::content_property(), ContentControl::content_property()).template_bind(ContentPresenter::content_template_property(), ContentControl::content_template_property()).dynamic(Layoutable::margin_property(), "NavigationViewContentPresenterMargin").control()])))?.control()]).control()]).control())
}

/// The default control theme of `NavigationView`.
fn navigation_view_theme(theme: &StyleBuilder) -> Result {
    theme.target::<NavigationView>();
    theme.static_resource(
        NavigationView::pane_toggle_button_style_property(),
        "PaneToggleButtonStyle",
    )?;
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(
        NavigationView::compact_pane_length_property(),
        "NavigationViewCompactPaneLength",
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius");
    theme.template(navigation_view_theme_template);
    // DisplayModeGroup: Compact and Expanded have no setters. Minimal, MinimalWithBackButton.
    // Minimal: the toggle button (or the pane title) beside the back or close button, not over it
    // them side by side).
    // Centred in its column; a style, so that MinimalWithBackButton can align it left.
    theme.style(nesting().template().of::<Button>().name("TogglePaneButton"), |style| {
        style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
        Ok(())
    })?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":minimal")
                .template()
                .of::<Button>()
                .name("TogglePaneButton"),
            nesting()
                .class(":minimal-with-back-button")
                .template()
                .of::<Button>()
                .name("TogglePaneButton"),
            nesting()
                .class(":minimal")
                .template()
                .of::<Grid>()
                .name("PaneTitleHolder"),
            nesting()
                .class(":minimal-with-back-button")
                .template()
                .of::<Grid>()
                .name("PaneTitleHolder"),
        ]),
        |style| {
            style.set(Grid::column_property(), 1);
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting().class(":minimal"),
            nesting().class(":minimal-with-back-button"),
        ]),
        |style| {
            style.style(
                nesting().template().of::<ContentControl>().name("HeaderContent"),
                |style| {
                    style.dynamic(Layoutable::margin_property(), "NavigationViewMinimalHeaderMargin");
                    Ok(())
                },
            )?;
            style.style(
                nesting().template().of::<Button>().name("NavigationViewBackButton"),
                |style| {
                    style.static_resource(StyledElement::theme_property(), "NavigationBackButtonSmallStyle")?;
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Border>().name("ContentGrid"), |style| {
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
            })?;
            Ok(())
        },
    )?;
    // TopNavigationMinimal
    theme.style(
        nesting()
            .class(":top-navigation-minimal")
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
            style.dynamic(Layoutable::margin_property(), "TopNavigationViewContentMargin");
            Ok(())
        },
    )?;
    // TogglePaneGroup: TogglePaneButtonVisible
    theme.style(nesting().class(":toggle-pane-button-visible"), |style| {
        style.style(
            nesting()
                .template()
                .of::<Panel>()
                .name("PaneContentGridToggleButtonRow"),
            |style| {
                style.static_resource(
                    Layoutable::min_height_property(),
                    "NavigationViewPaneHeaderRowMinHeight",
                )?;
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<ContentControl>().name("PaneTitlePresenter"),
            |style| {
                style.dynamic(Layoutable::margin_property(), "NavigationViewItemInnerHeaderMargin");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // HeaderGroup: HeaderCollapsed
    theme.style(
        nesting()
            .class(":header-collapsed")
            .template()
            .of::<ContentControl>()
            .name("HeaderContent"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // AutoSuggestGroup: AutoSuggestBoxCollapsed
    theme.style(
        nesting()
            .class(":auto-suggest-box-collapsed")
            .template()
            .of::<Grid>()
            .name("AutoSuggestArea"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":auto-suggest-box-collapsed")
            .template()
            .of::<Grid>()
            .name("TopPaneAutoSuggestArea"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // PaneStateGroup: ClosedCompact
    theme.style(
        nesting()
            .class(":closed-compact")
            .template()
            .of::<ContentControl>()
            .name("PaneAutoSuggestBoxPresenter"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":closed-compact")
            .template()
            .of::<Button>()
            .name("PaneAutoSuggestButton"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // PaneStateListSizeGroup: ListSizeCompact
    theme.style(nesting().class(":list-size-compact"), |style| {
        style.style(nesting().template().of::<Grid>().name("PaneContentGrid"), |style| {
            style.bind(
                Layoutable::width_property(),
                path()
                    .templated_parent()
                    .prop(NavigationView::compact_pane_length_property())
                    .binding(),
            );
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
            Ok(())
        })?;
        style.style(
            nesting().template().of::<TextBlock>().name("PaneTitleTextBlock"),
            |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentControl>()
                .name("PaneHeaderContentBorder"),
            |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentControl>()
                .name("PaneCustomContentBorder"),
            |style| {
                style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<ContentControl>().name("FooterContentBorder"),
            |style| {
                style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // PaneOverlayGroup: PaneNotOverlaying (PaneOverlaying is the base; ShadowCaster is left out).
    theme.style(
        nesting()
            .class(":pane-not-overlaying")
            .template()
            .of::<SplitView>()
            .name("RootSplitView"),
        |style| {
            style.set(
                TemplatedControl::border_brush_property(),
                Some(literal(Brush::parse("Transparent"))?),
            );
            style.dynamic(
                SplitView::pane_background_property(),
                "NavigationViewExpandedPaneBackground",
            );
            Ok(())
        },
    )?;
    // BackButtonGroup: BackButtonCollapsed
    theme.style(
        nesting()
            .class(":back-button-collapsed")
            .template()
            .of::<Panel>()
            .name("BackButtonPlaceholderOnTopNav"),
        |style| {
            style.set(Layoutable::width_property(), 0.0);
            Ok(())
        },
    )?;
    // PaneVisibilityGroup: PaneCollapsed (the control sets the display mode of the split view).
    theme.style(nesting().class(":pane-collapsed"), |style| {
        style.style(nesting().template().of::<SplitView>().name("RootSplitView"), |style| {
            style.set(SplitView::compact_pane_length_property(), 0.0);
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Grid>().name("PaneToggleButtonGrid"),
            |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // The row of the back button of PaneContentGrid, which NavigationView sizes in code
    // (c_backButtonRowDefinition, c_backButtonHeight).
    theme.style(
        nesting()
            .class(":back-button-row-visible")
            .template()
            .of::<Panel>()
            .name("BackButtonRowSpacer"),
        |style| {
            style.set(Layoutable::height_property(), 40.0);
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `NavigationViewItem` of `navigation_view_item_theme`.
fn navigation_view_item_theme_template(t: &Template<NavigationViewItem>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new()).name("NVIRootGrid")?.with(|e| { Grid::set_row_definitions(e, literal(RowDefinitions::parse("*,Auto"))?); Ok(()) })?
// The NavigationViewItemPresenter with MUX_NavigationViewItemPresenterStyleWhenOnLeftPane.

// The FlyoutBase.AttachedFlyout ChildrenFlyout of NVIRootGrid.

// The children: the last child of NVIRootGrid, as the repeater WinUI moves into the flyout.
.children([t.make(Panel::new()).name("NavigationViewItemPresenter")?.children([t.make(Border::new()).name("LayoutRoot")?.dynamic(Layoutable::min_height_property(), "NavigationViewItemOnLeftMinHeight").template_bind(Border::background_property(), TemplatedControl::background_property()).template_bind(Border::corner_radius_property(), TemplatedControl::corner_radius_property()).dynamic(Layoutable::margin_property(), "NavigationViewItemButtonMargin").child(Decorator::child_property(), t.make(Panel::new()).name("PresenterContentRootGrid")?
// Wrap SelectionIndicator in a grid so that its offset is 0,0 - this enables the offset animation.
.children([t.make(Panel::new()).name("SelectionIndicatorGrid")?.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center).children([t.make(SelectionIndicator::new()).name("SelectionIndicator")?.dynamic(Layoutable::width_property(), "NavigationViewSelectionIndicatorWidth").dynamic(Layoutable::height_property(), "NavigationViewSelectionIndicatorHeight").dynamic(Shape::fill_property(), "NavigationViewSelectionIndicatorForeground").set(Visual::opacity_property(), 0.0).static_resource(Rectangle::radius_x_property(), "NavigationViewSelectionIndicatorRadius")?.static_resource(Rectangle::radius_y_property(), "NavigationViewSelectionIndicatorRadius")?.control()]).control(), t.make(Border::new()).name("ContentGridHost")?.set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 14.0, 0.0)).child(Decorator::child_property(), t.make(Grid::new()).name("ContentGrid")?.dynamic(Layoutable::min_height_property(), "NavigationViewItemOnLeftMinHeight").set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Stretch).with(|e| { Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*,Auto,Auto"))?); Ok(()) })?.children([t.make(Border::new()).name("IconColumn")?.bind(Layoutable::width_property(), path().ancestor_of::<NavigationView>(0).prop(NavigationView::template_settings_property()).prop(NavigationViewTemplateSettings::smaller_pane_toggle_button_width_property()).binding()).child(Decorator::child_property(), t.make(Viewbox::new()).name("IconBox")?.dynamic(Layoutable::height_property(), "NavigationViewItemOnLeftIconBoxHeight").set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center).dynamic(Layoutable::margin_property(), "NavigationViewItemOnLeftIconBoxMargin").child(Viewbox::child_property(), t.make(ContentPresenter::new()).name("Icon")?.template_bind(ContentPresenter::content_property(), NavigationViewItem::icon_property()).template_bind(ContentPresenter::foreground_property(), TemplatedControl::foreground_property()).dynamic(ContentPresenter::background_property(), "NavigationViewItemIconBackground")
// A path icon as WinUI's, unscaled, so that the Viewbox scales its geometry as WinUI's does (PI-3).
.style(of::<PathIcon>(), |style| {
style.set(Layoutable::width_property(), f64::NAN);
style.set(Layoutable::height_property(), f64::NAN);
style.static_resource(TemplatedControl::template_property(), "PathIconUnscaledTemplate")?;
Ok(())
})?)).control(), t.make(ContentPresenter::new()).name("PART_ContentPresenter")?.set(Grid::column_property(), 1).dynamic(Layoutable::margin_property(), "NavigationViewItemContentPresenterMargin").template_bind(ContentPresenter::content_template_property(), ContentControl::content_template_property()).template_bind(ContentPresenter::content_property(), ContentControl::content_property()).template_bind(ContentPresenter::foreground_property(), TemplatedControl::foreground_property()).template_bind(Layoutable::horizontal_alignment_property(), ContentControl::horizontal_content_alignment_property()).template_bind(Layoutable::vertical_alignment_property(), ContentControl::vertical_content_alignment_property()).template_bind(ContentPresenter::padding_property(), TemplatedControl::padding_property()).control(), t.make(Border::new()).name("ExpandCollapseChevron")?.set(Grid::column_property(), 3).set(Visual::is_visible_property(), false).set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right).set(Layoutable::width_property(), 40.0).dynamic(Layoutable::margin_property(), "NavigationViewItemExpandChevronMargin").set(Border::background_property(), Some(literal(Brush::parse("Transparent"))?)).child(Decorator::child_property(), t.make(TextBlock::new()).name("ExpandCollapseChevronIcon")?.set(Layoutable::width_property(), 12.0).set(Layoutable::height_property(), 12.0).set(Visual::render_transform_origin_property(), literal(RelativePoint::parse("50%,50%"))?).set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center).set(TextBlock::text_alignment_property(), TextAlignment::Center).set(Visual::is_visible_property(), false).static_resource(TextBlock::text_property(), "NavigationViewItemExpandedGlyph")?.dynamic(TextBlock::font_family_property(), "SymbolThemeFontFamily").dynamic(TextBlock::font_size_property(), "NavigationViewItemExpandedGlyphFontSize").dynamic(TextBlock::foreground_property(), "NavigationViewItemForeground").set(AutomationProperties::accessibility_view_property(), AccessibilityView::Raw)).control(), t.make(ContentPresenter::new()).name("InfoBadgePresenter")?.set(Grid::column_property(), 2).set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center).template_bind(ContentPresenter::content_property(), NavigationViewItem::info_badge_property()).control()])).control()])).control()]).control(), t.make(Popup::new()).name("ChildrenFlyout")?.bind(Popup::placement_target_property(), path().named(t.names(), "NavigationViewItemPresenter").binding()).set(Popup::placement_property(), PlacementMode::RightEdgeAlignedTop).set(Popup::is_light_dismiss_enabled_property(), true).set(Popup::window_manager_add_shadow_hint_property(), false).child(Popup::child_property(), t.make(Border::new()).name("ChildrenFlyoutPresenter")?.dynamic(Border::background_property(), "FlyoutPresenterBackground").dynamic(Border::border_brush_property(), "FlyoutBorderThemeBrush").dynamic(Border::border_thickness_property(), "FlyoutBorderThemeThickness").dynamic(Decorator::padding_property(), "NavigationViewItemChildrenMenuFlyoutPadding").set(Layoutable::margin_property(), Thickness::new(0.0, -4.0, 0.0, 0.0)).dynamic(Layoutable::min_width_property(), "FlyoutThemeMinWidth").dynamic(Layoutable::max_width_property(), "FlyoutThemeMaxWidth").dynamic(Layoutable::min_height_property(), "FlyoutThemeMinHeight").dynamic(Layoutable::max_height_property(), "FlyoutThemeMaxHeight").dynamic(Border::corner_radius_property(), "OverlayCornerRadius").set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge).child(Decorator::child_property(), t.make(ScrollViewer::new()).set(ScrollViewer::horizontal_scroll_bar_visibility_property(), ScrollBarVisibility::Auto).set(ScrollViewer::vertical_scroll_bar_visibility_property(), ScrollBarVisibility::Auto).content(ContentControl::content_property(), t.make(Grid::new()).name("FlyoutRootGrid")?.children([t.make(Grid::new()).name("FlyoutContentGrid")?.control()]))?)).control(), t.make(StackPanel::new()).name("NavigationViewItemMenuItemsHost")?.set(Grid::row_property(), 1).set(Visual::is_visible_property(), false).control()]).control())
}

/// The default control theme of `NavigationViewItem`.
fn navigation_view_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<NavigationViewItem>();
    // The content alignment of a WinUI control (Control.VerticalContentAlignment, Center), where a
    // FerroUI content control is at the top: the text of an item is centred on its icon (NVI-12).
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
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
    theme.dynamic(Layoutable::margin_property(), "NavigationViewItemMargin");
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(navigation_view_item_theme_template);
    // PointerStates (MUX_NavigationViewItemPresenterStyleWhenOnLeftPane): PointerOver
    theme.style(
        nesting()
            .template()
            .of::<Border>()
            .name("LayoutRoot")
            .class(":pointerover"),
        |style| {
            style.dynamic(Border::background_property(), "NavigationViewItemBackgroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<ContentPresenter>()
                .name("Icon"),
            nesting()
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
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
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
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
    // Selected
    theme.style(nesting().class(":selected"), |style| {
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
        Ok(())
    })?;
    // PointerOverSelected
    theme.style(
        nesting()
            .class(":selected")
            .template()
            .of::<Border>()
            .name("LayoutRoot")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "NavigationViewItemBackgroundSelectedPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":selected")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<ContentPresenter>()
                .name("Icon"),
            nesting()
                .class(":selected")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
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
    // IconStates: IconCollapsed
    theme.style(nesting().class(":icon-collapsed"), |style| {
        style.style(nesting().template().of::<Viewbox>().name("IconBox"), |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("IconColumn"), |style| {
            style.set(Layoutable::width_property(), 8.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // InfoBadgeStates: InfoBadgeCollapsed
    theme.style(
        nesting()
            .class(":info-badge-collapsed")
            .template()
            .of::<ContentPresenter>()
            .name("InfoBadgePresenter"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // ChevronStates: ChevronVisibleOpen, ChevronVisibleClosed
    theme.style(
        Selectors::or([
            nesting().class(":chevron-visible-open"),
            nesting().class(":chevron-visible-closed"),
        ]),
        |style| {
            style.style(
                nesting().template().of::<Border>().name("ExpandCollapseChevron"),
                |style| {
                    style.set(Visual::is_visible_property(), true);
                    Ok(())
                },
            )?;
            style.style(
                nesting().template().of::<TextBlock>().name("ExpandCollapseChevronIcon"),
                |style| {
                    style.set(Visual::is_visible_property(), true);
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // The AnimatedChevronUpDownSmall of the open chevron points up.
    theme.style(
        nesting()
            .class(":chevron-visible-open")
            .template()
            .of::<TextBlock>()
            .name("ExpandCollapseChevronIcon"),
        |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("rotate(180deg)"))?),
            );
            Ok(())
        },
    )?;
    // PointerChevronStates: the foreground of a visible chevron under the pointer and pressed.
    theme.style(
        Selectors::or([
            nesting()
                .class(":chevron-visible-open")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<TextBlock>()
                .name("ExpandCollapseChevronIcon"),
            nesting()
                .class(":chevron-visible-closed")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover")
                .descendant()
                .of::<TextBlock>()
                .name("ExpandCollapseChevronIcon"),
        ]),
        |style| {
            style.dynamic(
                TextBlock::foreground_property(),
                "NavigationViewItemForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        Selectors::or([
            nesting()
                .class(":pressed")
                .class(":chevron-visible-open")
                .template()
                .of::<TextBlock>()
                .name("ExpandCollapseChevronIcon"),
            nesting()
                .class(":pressed")
                .class(":chevron-visible-closed")
                .template()
                .of::<TextBlock>()
                .name("ExpandCollapseChevronIcon"),
        ]),
        |style| {
            style.dynamic(TextBlock::foreground_property(), "NavigationViewItemForegroundPressed");
            Ok(())
        },
    )?;
    // PaneAndTopLevelItemStates: ClosedCompactAndTopLevelItem
    theme.style(nesting().class(":closed-compact-and-top-level-item"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    Layoutable::margin_property(),
                    "NavigationViewCompactItemContentPresenterMargin",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("ContentGridHost"), |style| {
            style.set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 0.0, 0.0));
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ContentPresenter>().name("InfoBadgePresenter"),
            |style| {
                style.set(Grid::column_property(), 0);
                style.set(Grid::column_span_property(), 4);
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
                style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right);
                style.set(Layoutable::margin_property(), Thickness::new(0.0, 2.0, 2.0, 0.0));
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // ItemOnNavigationViewListPositionStates: OnTopNavigationPrimary, the presenter of the top pane
    // (MUX_NavigationViewItemPresenterStyleWhenOnTopPane).
    theme.style(nesting().class(":on-top-navigation-primary"), |style| {
        style.style(
            nesting().template().of::<Panel>().name("NavigationViewItemPresenter"),
            |style| {
                style.dynamic(Layoutable::margin_property(), "TopNavigationViewItemMargin");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.set(Layoutable::min_height_property(), 0.0);
            style.static_resource(Layoutable::margin_property(), "NavigationViewItemButtonMargin")?;
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Panel>().name("SelectionIndicatorGrid"),
            |style| {
                style.set(Layoutable::margin_property(), Thickness::new(16.0, 0.0, 16.0, 4.0));
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom);
                style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Rectangle>().name("SelectionIndicator"),
            |style| {
                style.set(Layoutable::width_property(), 16.0);
                style.set(Layoutable::height_property(), 3.0);
                style.set(Rectangle::radius_x_property(), 2.0);
                style.set(Rectangle::radius_y_property(), 2.0);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("ContentGridHost"), |style| {
            style.set(Layoutable::margin_property(), Thickness::uniform(0.0));
            Ok(())
        })?;
        style.style(nesting().template().of::<Grid>().name("ContentGrid"), |style| {
            style.set(Layoutable::min_height_property(), 0.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("IconColumn"), |style| {
            style.set(Layoutable::width_property(), f64::NAN);
            Ok(())
        })?;
        style.style(nesting().template().of::<Viewbox>().name("IconBox"), |style| {
            style.set(Layoutable::height_property(), 16.0);
            style.set(Layoutable::width_property(), 16.0);
            style.set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 0.0, 0.0));
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    Layoutable::margin_property(),
                    "TopNavigationViewItemContentPresenterMargin",
                );
                style.set(ContentPresenter::text_wrapping_property(), TextWrapping::NoWrap);
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Border>().name("ExpandCollapseChevron"),
            |style| {
                style.set(Grid::column_property(), 2);
                style.dynamic(
                    Layoutable::margin_property(),
                    "TopNavigationViewItemExpandChevronMargin",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<ContentPresenter>().name("InfoBadgePresenter"),
            |style| {
                style.set(Grid::column_property(), 3);
                style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
                style.set(Layoutable::margin_property(), Thickness::new(-16.0, 0.0, 2.0, 13.0));
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Popup>().name("ChildrenFlyout"), |style| {
            style.set(Popup::placement_property(), PlacementMode::BottomEdgeAlignedLeft);
            Ok(())
        })?;
        // PointerStates of the top pane.
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover"),
            |style| {
                style.dynamic(
                    Border::background_property(),
                    "TopNavigationViewItemBackgroundPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(
            Selectors::or([
                nesting()
                    .template()
                    .of::<Border>()
                    .name("LayoutRoot")
                    .class(":pointerover")
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("Icon"),
                nesting()
                    .template()
                    .of::<Border>()
                    .name("LayoutRoot")
                    .class(":pointerover")
                    .descendant()
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
        style.style(nesting().class(":pressed"), |style| {
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
        style.style(nesting().class(":selected"), |style| {
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
            Ok(())
        })?;
        style.style(
            nesting()
                .class(":selected")
                .template()
                .of::<Border>()
                .name("LayoutRoot")
                .class(":pointerover"),
            |style| {
                style.dynamic(
                    Border::background_property(),
                    "TopNavigationViewItemBackgroundSelectedPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(
            Selectors::or([
                nesting()
                    .class(":selected")
                    .template()
                    .of::<Border>()
                    .name("LayoutRoot")
                    .class(":pointerover")
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("Icon"),
                nesting()
                    .class(":selected")
                    .template()
                    .of::<Border>()
                    .name("LayoutRoot")
                    .class(":pointerover")
                    .descendant()
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
        style.style(nesting().class(":selected").class(":pressed"), |style| {
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
        // DisabledStates of the top pane: the foregrounds, not the opacity.
        style.style(nesting().class(":disabled"), |style| {
            style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                style.set(Visual::opacity_property(), 1.0);
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
                        "TopNavigationViewItemForegroundDisabled",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        })?;
        // NavigationViewIconPositionStates: IconOnly
        style.style(nesting().class(":icon-only"), |style| {
            style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
                style.set(Layoutable::width_property(), 36.0);
                style.set(Layoutable::height_property(), 36.0);
                style.set(Layoutable::margin_property(), Thickness::uniform(2.0));
                Ok(())
            })?;
            style.style(nesting().template().of::<Viewbox>().name("IconBox"), |style| {
                style.set(Layoutable::margin_property(), Thickness::symmetric(10.0, 0.0));
                Ok(())
            })?;
            style.style(
                nesting()
                    .template()
                    .of::<ContentPresenter>()
                    .name("PART_ContentPresenter"),
                |style| {
                    style.set(Visual::is_visible_property(), false);
                    Ok(())
                },
            )?;
            style.style(
                nesting().template().of::<Panel>().name("SelectionIndicatorGrid"),
                |style| {
                    style.set(Layoutable::margin_property(), Thickness::uniform(0.0));
                    Ok(())
                },
            )?;
            style.style(
                nesting().template().of::<Border>().name("ExpandCollapseChevron"),
                |style| {
                    style.dynamic(
                        Layoutable::margin_property(),
                        "TopNavigationViewItemIconOnlyExpandChevronMargin",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        })?;
        // ContentOnly (the icon column has no width of its own in the top pane).
        style.style(nesting().class(":content-only"), |style| {
            style.style(nesting().template().of::<Viewbox>().name("IconBox"), |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            })?;
            style.style(nesting().template().of::<Border>().name("IconColumn"), |style| {
                style.set(Layoutable::width_property(), f64::NAN);
                Ok(())
            })?;
            style.style(
                nesting()
                    .template()
                    .of::<ContentPresenter>()
                    .name("PART_ContentPresenter"),
                |style| {
                    style.dynamic(
                        Layoutable::margin_property(),
                        "TopNavigationViewItemContentOnlyContentPresenterMargin",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting().template().of::<Panel>().name("SelectionIndicatorGrid"),
                |style| {
                    style.set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 12.0, 4.0));
                    Ok(())
                },
            )?;
            style.style(
                nesting().template().of::<Border>().name("ExpandCollapseChevron"),
                |style| {
                    style.dynamic(
                        Layoutable::margin_property(),
                        "TopNavigationViewItemContentOnlyExpandChevronMargin",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `NavigationViewItemHeader` of `navigation_view_item_header_theme`.
fn navigation_view_item_header_theme_template(t: &Template<NavigationViewItemHeader>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("NavigationViewItemHeaderRootGrid")?
        .children([t
            .make(Grid::new())
            .name("InnerHeaderGrid")?
            .set(Layoutable::height_property(), 40.0)
            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
            .dynamic(Layoutable::margin_property(), "NavigationViewItemInnerHeaderMargin")
            // The TextBlock HeaderText with NavigationViewItemHeaderTextStyle.
            .children([t
                .make(ContentPresenter::new())
                .name("HeaderText")?
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(Layoutable::margin_property(), Thickness::new(0.0, -1.0, 0.0, -1.0))
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .set(
                    ContentPresenter::font_family_property(),
                    literal(FontFamily::parse("Segoe UI Variable Text, Segoe UI, $Default"))?,
                )
                .set(ContentPresenter::font_size_property(), 14.0)
                .set(ContentPresenter::font_weight_property(), FontWeight::SemiBold)
                .set(ContentPresenter::text_wrapping_property(), TextWrapping::NoWrap)
                .control()])
            .control()])
        .control())
}

/// The default control theme of `NavigationViewItemHeader`.
fn navigation_view_item_header_theme(theme: &StyleBuilder) -> Result {
    theme.target::<NavigationViewItemHeader>();
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "NavigationViewItemHeaderForeground",
    );
    theme.set(InputElement::is_enabled_property(), false);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(Layoutable::min_height_property(), 0.0);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        AutomationProperties::accessibility_view_property(),
        AccessibilityView::Raw,
    );
    theme.template(navigation_view_item_header_theme_template);
    // PaneStates: HeaderTextCollapsed (the transitions are left out).
    theme.style(nesting().class(":header-text-collapsed"), |style| {
        style.style(
            nesting().template().of::<ContentPresenter>().name("HeaderText"),
            |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Grid>().name("InnerHeaderGrid"), |style| {
            style.set(Layoutable::height_property(), 0.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // DisplayModeStates: TopMode
    theme.style(
        nesting()
            .class(":top-mode")
            .template()
            .of::<Grid>()
            .name("InnerHeaderGrid"),
        |style| {
            style.dynamic(Layoutable::margin_property(), "TopNavigationViewItemInnerHeaderMargin");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `NavigationViewItemSeparator` of `navigation_view_item_separator_theme`.
fn navigation_view_item_separator_theme_template(t: &Template<NavigationViewItemSeparator>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("NavigationViewItemSeparatorRootGrid")?
        .children([t
            .make(Rectangle::new())
            .name("SeparatorLine")?
            .dynamic(Layoutable::height_property(), "NavigationViewItemSeparatorHeight")
            .dynamic(Shape::fill_property(), "NavigationViewItemSeparatorForeground")
            .dynamic(Layoutable::margin_property(), "NavigationViewItemSeparatorMargin")
            .control()])
        .control())
}

/// The default control theme of `NavigationViewItemSeparator`.
fn navigation_view_item_separator_theme(theme: &StyleBuilder) -> Result {
    theme.target::<NavigationViewItemSeparator>();
    theme.set(InputElement::is_enabled_property(), false);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(Layoutable::min_height_property(), 0.0);
    theme.set(
        AutomationProperties::accessibility_view_property(),
        AccessibilityView::Raw,
    );
    theme.template(navigation_view_item_separator_theme_template);
    // NavigationSeparatorLineStates: HorizontalLineCompact, VerticalLine
    theme.style(
        nesting()
            .class(":horizontal-line-compact")
            .template()
            .of::<Rectangle>()
            .name("SeparatorLine"),
        |style| {
            style.dynamic(
                Layoutable::margin_property(),
                "NavigationViewCompactItemSeparatorMargin",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":vertical-line")
            .template()
            .of::<Rectangle>()
            .name("SeparatorLine"),
        |style| {
            style.set(Layoutable::height_property(), 24.0);
            style.dynamic(Layoutable::width_property(), "TopNavigationViewItemSeparatorWidth");
            style.dynamic(Layoutable::margin_property(), "TopNavigationViewItemSeparatorMargin");
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
            style.dynamic(Shape::fill_property(), "TopNavigationViewItemSeparatorForeground");
            Ok(())
        },
    )?;
    Ok(())
}
