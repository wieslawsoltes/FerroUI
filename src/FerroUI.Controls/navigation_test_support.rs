//! What the tests of WinUI's NavigationView and its items share (`navigation_view_tests`,
//! `navigation_view_item_tests`): the parts of the templates of the WinUI theme that the controls
//! look up, the services of the tests, and the lookup of parts.
//!
//! The controls have no template of their own (the WinUI theme has them,
//! `src/FerroUI.Themes.WinUI/Controls/NavigationView.xaml`), so the theme of the tests gives them
//! small templates with the parts and the template bindings of that document. The split view
//! `RootSplitView` gets the split view theme of the tests.

use crate::presenters::ContentPresenter;
use crate::primitives::{Popup, TemplatedControl};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{add_template_theme, create_test_theme, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Border, Button, ContentControl, Control, Grid, NavigationView, NavigationViewItem, NavigationViewItemHeader,
    NavigationViewItemSeparator, Panel, ScrollViewer, SplitView, StackPanel, TextBlock, Window,
};
use ferroui_base::animation::{ClockBase, IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::{ReflectionBinding, RelativeSource, RelativeSourceMode, TemplateBinding};
use ferroui_base::layout::Orientation;
use ferroui_base::media::{Geometry, GeometryHitTestResult};
use ferroui_base::rendering::IHitTester;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver};
use ferroui_base::styling::{styles_as_style, IStyle, Styles};
use ferroui_base::input::InputElement;
use ferroui_base::{FerroProperty, ObjectType, Point, Ref, StyledElement, Visual};
use std::rc::Rc;

/// A global clock whose time stands still: the transitions of the split view need one, and the
/// tests check their synchronous state only.
#[derive(Default)]
struct TestGlobalClock(ClockBase);

impl IObservable<TimeSpan> for TestGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.0.subscribe(observer)
    }
}

impl IClock for TestGlobalClock {
    fn play_state(&self) -> PlayState {
        self.0.play_state()
    }

    fn set_play_state(&self, value: PlayState) {
        self.0.set_play_state(value);
    }
}

impl IGlobalClock for TestGlobalClock {}

/// The theme of the tests with the templates of the navigation controls.
fn navigation_theme() -> Rc<dyn IStyle> {
    let styles = Styles::new();
    styles.add(create_test_theme());
    add_template_theme::<NavigationView>(&styles, navigation_view_template());
    add_template_theme::<NavigationViewItem>(&styles, navigation_view_item_template());
    add_template_theme::<NavigationViewItemHeader>(&styles, navigation_view_item_header_template());
    add_template_theme::<NavigationViewItemSeparator>(&styles, navigation_view_item_separator_template());
    styles_as_style(&styles)
}

fn services() -> TestServices {
    TestServices::styled_window().with_global_clock(Rc::new(TestGlobalClock::default())).with_theme(navigation_theme)
}

/// An application with a styled window and the templates of the navigation controls.
pub(crate) fn start() -> UnitTestApplicationScope {
    crate::register_types();
    UnitTestApplication::start(services())
}

/// [`start`] with real keyboard focus.
pub(crate) fn start_with_focus() -> UnitTestApplicationScope {
    crate::register_types();
    let mut services = services();
    let focus = TestServices::real_focus();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    UnitTestApplication::start(services)
}

/// The element named `name` of the template of `control`, after the template is applied.
pub(crate) fn part<T: ObjectType>(control: &TemplatedControl, name: &str) -> Ref<T> {
    control.apply_template();
    control
        .get_template_descendants()
        .into_iter()
        .find(|element| element.name().as_deref() == Some(name))
        .and_then(|element| element.cast::<T>())
        .unwrap_or_else(|| panic!("the template part {name}"))
}

/// Whether `control` has the pseudo-class `class`.
pub(crate) fn has_class(control: &StyledElement, class: &str) -> bool {
    control.classes().contains(class)
}

fn named<T: ObjectType>(element: Ref<T>, name: &str) -> Ref<T>
where
    T: ferroui_base::Upcast<StyledElement>,
{
    let styled: &StyledElement = (*element).upcast();
    styled.set_name(Some(name.to_string()));
    element
}

fn template_bind(target: &StyledElement, target_property: &'static FerroProperty, source: &'static FerroProperty) {
    target.bind_binding(target_property, &*TemplateBinding::new(source));
}

/// A binding to `path` of the templated parent.
fn templated_parent_binding(path: &str) -> Rc<ReflectionBinding> {
    let binding = ReflectionBinding::new(path);
    binding.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent)));
    binding
}

/// The parts of the template of NavigationViewItem in `Controls/NavigationView.xaml`: `NVIRootGrid`
/// with the presenter `NavigationViewItemPresenter` (`SelectionIndicator`, `ContentGrid` with the
/// icon, the content, the chevron `ExpandCollapseChevron` and the badge `InfoBadgePresenter`), the
/// children `NavigationViewItemMenuItemsHost` and the popup `ChildrenFlyout` with
/// `FlyoutContentGrid`.
pub(crate) fn navigation_view_item_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let root = named(Grid::new(), "NVIRootGrid").register_in_name_scope(&**scope);
        root.set_row_definitions("*,Auto".parse().expect("the rows of NVIRootGrid"));
        let presenter = named(Border::new(), "NavigationViewItemPresenter").register_in_name_scope(&**scope);
        let presenter_root = Panel::new();
        let indicator = named(Border::new(), "SelectionIndicator").register_in_name_scope(&**scope);
        indicator.set_width(3.0);
        indicator.set_height(16.0);
        indicator.set_opacity(0.0);
        presenter_root.children().add(&indicator);
        let content_grid = named(Grid::new(), "ContentGrid").register_in_name_scope(&**scope);
        content_grid.set_column_definitions("Auto,*,Auto,Auto".parse().expect("the columns of ContentGrid"));
        let icon = named(ContentPresenter::new(), "Icon").register_in_name_scope(&**scope);
        template_bind(&icon, ContentPresenter::content_property().as_property(), NavigationViewItem::icon_property().as_property());
        content_grid.children().add(&icon);
        let content = named(ContentPresenter::new(), "PART_ContentPresenter").register_in_name_scope(&**scope);
        template_bind(&content, ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        template_bind(
            &content,
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        );
        Grid::set_column(&content, 1);
        content_grid.children().add(&content);
        let badge = named(ContentPresenter::new(), "InfoBadgePresenter").register_in_name_scope(&**scope);
        template_bind(&badge, ContentPresenter::content_property().as_property(), NavigationViewItem::info_badge_property().as_property());
        Grid::set_column(&badge, 2);
        content_grid.children().add(&badge);
        let chevron = named(Border::new(), "ExpandCollapseChevron").register_in_name_scope(&**scope);
        chevron.set_width(40.0);
        let glyph = named(TextBlock::new(), "ExpandCollapseChevronIcon").register_in_name_scope(&**scope);
        glyph.set_text(Some("v"));
        chevron.set_child(glyph);
        Grid::set_column(&chevron, 3);
        content_grid.children().add(&chevron);
        presenter_root.children().add(&content_grid);
        presenter.set_child(presenter_root);
        root.children().add(&presenter);
        let host = named(StackPanel::new(), "NavigationViewItemMenuItemsHost").register_in_name_scope(&**scope);
        host.set_is_visible(false);
        Grid::set_row(&host, 1);
        let flyout = named(Popup::new(), "ChildrenFlyout").register_in_name_scope(&**scope);
        flyout.set_is_light_dismiss_enabled(true);
        let flyout_root = named(Panel::new(), "FlyoutRootGrid").register_in_name_scope(&**scope);
        let flyout_content = named(Panel::new(), "FlyoutContentGrid").register_in_name_scope(&**scope);
        flyout_root.children().add(&flyout_content);
        flyout.set_child(flyout_root);
        flyout.set_placement_target(presenter.clone());
        root.children().add(&flyout);
        // The host is the last child of the root grid, as the repeater that WinUI moves.
        root.children().add(&host);
        root.upcast::<Control>()
    })
}

/// The parts of the template of NavigationViewItemHeader: `NavigationViewItemHeaderRootGrid` with the
/// text `HeaderText`.
pub(crate) fn navigation_view_item_header_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let root = named(Border::new(), "NavigationViewItemHeaderRootGrid").register_in_name_scope(&**scope);
        let text = named(ContentPresenter::new(), "HeaderText").register_in_name_scope(&**scope);
        template_bind(&text, ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        root.set_child(text);
        root.upcast::<Control>()
    })
}

/// The parts of the template of NavigationViewItemSeparator: `NavigationViewItemSeparatorRootGrid`
/// with the line `SeparatorLine`.
pub(crate) fn navigation_view_item_separator_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let root = named(Border::new(), "NavigationViewItemSeparatorRootGrid").register_in_name_scope(&**scope);
        let line = named(Border::new(), "SeparatorLine").register_in_name_scope(&**scope);
        line.set_height(1.0);
        root.set_child(line);
        root.upcast::<Control>()
    })
}

/// A content control part of NavigationView (`PaneHeaderContentBorder` and the others).
fn content_part(scope: &NameScopeRef, name: &str) -> Ref<ContentControl> {
    named(ContentControl::new(), name).register_in_name_scope(&**scope)
}

/// The parts of the template of NavigationView in `Controls/NavigationView.xaml`: the buttons of
/// `PaneToggleButtonGrid`, the top pane `TopNavArea`, and the split view `RootSplitView` with the
/// pane `PaneContentGrid` and the content grid `ContentGrid`.
pub(crate) fn navigation_view_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let root = named(Grid::new(), "RootGrid").register_in_name_scope(&**scope);
        root.set_row_definitions("Auto,*".parse().expect("the rows of RootGrid"));

        // PaneToggleButtonGrid.
        let buttons = named(StackPanel::new(), "ButtonHolderGrid").register_in_name_scope(&**scope);
        let back = named(Button::new(), "NavigationViewBackButton").register_in_name_scope(&**scope);
        back.bind_binding(
            Visual::is_visible_property().as_property(),
            &*templated_parent_binding("TemplateSettings.BackButtonVisibility"),
        );
        template_bind(&back, InputElement::is_enabled_property().as_property(), NavigationView::is_back_enabled_property().as_property());
        buttons.children().add(&back);
        let close = named(Button::new(), "NavigationViewCloseButton").register_in_name_scope(&**scope);
        buttons.children().add(&close);
        let toggle = named(Button::new(), "TogglePaneButton").register_in_name_scope(&**scope);
        toggle.bind_binding(
            Visual::is_visible_property().as_property(),
            &*templated_parent_binding("TemplateSettings.PaneToggleButtonVisibility"),
        );
        let title = named(TextBlock::new(), "PaneTitleTextBlock").register_in_name_scope(&**scope);
        template_bind(&title, TextBlock::text_property().as_property(), NavigationView::pane_title_property().as_property());
        toggle.set_content(Some(Control::boxed(title)));
        buttons.children().add(&toggle);
        let holder = named(Border::new(), "PaneTitleHolder").register_in_name_scope(&**scope);
        holder.set_is_visible(false);
        holder.set_child(content_part(scope, "PaneTitlePresenter"));
        buttons.children().add(&holder);
        Grid::set_row_span(&buttons, 2);
        buttons.set_z_index(100);

        // TopNavArea.
        let top_area = named(StackPanel::new(), "TopNavArea").register_in_name_scope(&**scope);
        let top_grid = named(StackPanel::new(), "TopNavGrid").register_in_name_scope(&**scope);
        top_grid.set_orientation(Orientation::Horizontal);
        top_grid.bind_binding(
            Visual::is_visible_property().as_property(),
            &*templated_parent_binding("TemplateSettings.TopPaneVisibility"),
        );
        top_grid.children().add(content_part(scope, "PaneHeaderOnTopPane"));
        top_grid.children().add(content_part(scope, "PaneTitleOnTopPane"));
        let top_menu = named(StackPanel::new(), "TopNavMenuItemsHost").register_in_name_scope(&**scope);
        top_menu.set_orientation(Orientation::Horizontal);
        top_grid.children().add(&top_menu);
        top_grid.children().add(content_part(scope, "PaneCustomContentOnTopPane"));
        top_grid.children().add(content_part(scope, "TopPaneAutoSuggestBoxPresenter"));
        top_grid.children().add(content_part(scope, "PaneFooterOnTopPane"));
        let top_footer = named(StackPanel::new(), "TopFooterMenuItemsHost").register_in_name_scope(&**scope);
        top_footer.set_orientation(Orientation::Horizontal);
        top_grid.children().add(&top_footer);
        top_area.children().add(&top_grid);
        let overlay = named(ContentPresenter::new(), "TopNavContentOverlayAreaGrid").register_in_name_scope(&**scope);
        template_bind(&overlay, ContentPresenter::content_property().as_property(), NavigationView::content_overlay_property().as_property());
        top_area.children().add(&overlay);
        root.children().add(&top_area);

        // RootSplitView.
        let split = named(SplitView::new(), "RootSplitView").register_in_name_scope(&**scope);
        template_bind(&split, SplitView::compact_pane_length_property().as_property(), NavigationView::compact_pane_length_property().as_property());
        split.bind_binding(
            SplitView::open_pane_length_property().as_property(),
            &*templated_parent_binding("TemplateSettings.OpenPaneLength"),
        );
        Grid::set_row(&split, 1);
        let pane = named(StackPanel::new(), "PaneContentGrid").register_in_name_scope(&**scope);
        pane.bind_binding(
            Visual::is_visible_property().as_property(),
            &*templated_parent_binding("TemplateSettings.LeftPaneVisibility"),
        );
        pane.children().add(content_part(scope, "PaneHeaderContentBorder"));
        pane.children().add(content_part(scope, "PaneAutoSuggestBoxPresenter"));
        let search = named(Button::new(), "PaneAutoSuggestButton").register_in_name_scope(&**scope);
        search.set_is_visible(false);
        pane.children().add(&search);
        pane.children().add(content_part(scope, "PaneCustomContentBorder"));
        let menu = named(StackPanel::new(), "MenuItemsHost").register_in_name_scope(&**scope);
        let menu_scroll = named(ScrollViewer::new(), "MenuItemsScrollViewer").register_in_name_scope(&**scope);
        menu_scroll.set_content(Some(Control::boxed(menu)));
        pane.children().add(&menu_scroll);
        pane.children().add(content_part(scope, "FooterContentBorder"));
        let footer = named(StackPanel::new(), "FooterMenuItemsHost").register_in_name_scope(&**scope);
        pane.children().add(&footer);
        split.set_pane(Some(Control::boxed(pane)));

        let content_grid = named(StackPanel::new(), "ContentGrid").register_in_name_scope(&**scope);
        let padding = named(Border::new(), "ContentLeftPadding").register_in_name_scope(&**scope);
        content_grid.children().add(&padding);
        let header = content_part(scope, "HeaderContent");
        template_bind(&header, ContentControl::content_property().as_property(), NavigationView::header_property().as_property());
        template_bind(
            &header,
            ContentControl::content_template_property().as_property(),
            NavigationView::header_template_property().as_property(),
        );
        content_grid.children().add(&header);
        let content = named(ContentPresenter::new(), "PART_ContentPresenter").register_in_name_scope(&**scope);
        template_bind(&content, ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        template_bind(
            &content,
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        );
        content_grid.children().add(&content);
        split.set_content(Some(Control::boxed(content_grid)));
        root.children().add(&split);
        root.children().add(&buttons);
        root.upcast::<Control>()
    })
}

/// The hit tester of the pointer tests: the root of the search when it contains the point (the
/// one of `button_tests`), so that a release over an item is over it.
pub(crate) struct BoundsHitTester;

impl IHitTester for BoundsHitTester {
    fn hit_test(&self, p: Point, root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        if root.bounds().contains(p) {
            vec![root.to_ref()]
        } else {
            Vec::new()
        }
    }

    fn hit_test_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        Vec::new()
    }

    fn hit_test_first(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>) -> Option<Ref<Visual>> {
        self.hit_test(p, root, filter).into_iter().next()
    }

    fn hit_test_first_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        None
    }
}

/// A shown window that closes when the scene is dropped.
pub(crate) struct Scene(pub(crate) Ref<Window>);

impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}

/// Shows `control` as the content of a window of `width` by `height`, with the bounds hit tester,
/// and lays it out: the theme of the tests gives the controls their templates once they are in the
/// tree.
pub(crate) fn show(control: &Control, width: f64, height: f64) -> Scene {
    let window = Window::new();
    window.set_hit_tester_override(Some(Rc::new(BoundsHitTester)));
    window.set_width(width);
    window.set_height(height);
    window.set_content(Some(Control::boxed(control.to_ref())));
    window.show();
    window.update_layout();
    Scene(window)
}
