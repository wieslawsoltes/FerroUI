//! The reference tests run against the control themes of a theme package;
//! here the templates of those themes that take part in the layout are built
//! in code.

use crate::converters::TreeViewItemIndentConverter;
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{HeaderedItemsControl, ScrollBarVisibility, TemplatedControl, ToggleButton};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{test_scope, TestRoot};
use crate::{
    Border, ColumnDefinition, ColumnDefinitions, Control, Grid, GridLength, GridUnitType, ItemsControl, ItemsSource,
    ScrollViewer, StackPanel, TreeView, TreeViewItem,
};
use ferroui_base::data::{BindingBase, BindingMode, IndexerBinding, MultiBinding, ReflectionBinding, TemplateBinding};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, VerticalAlignment};
use ferroui_base::media::Brushes;
use ferroui_base::styling::{ControlTheme, Setter};
use ferroui_base::{BoxedValue, FerroProperty, Rect, Ref, StyledElement, Thickness, Vector, Visual};
use std::rc::Rc;

#[test]
fn bring_into_view_should_scroll_back_to_item_scrolled_off_to_the_left() {
    let _app = test_scope();

    let (root, tree_view) = create_target(vec![create_header(100.0), create_header(500.0), create_header(900.0)]);

    root.layout_manager().execute_initial_layout_pass();

    let scroll_viewer = get_scroll_viewer(&tree_view);

    scroll_viewer.set_offset(Vector::new(scroll_viewer.extent().width - scroll_viewer.viewport().width, 0.0));
    root.layout_manager().execute_layout_pass();

    let start_offset = scroll_viewer.offset().x;
    assert!(start_offset > 0.0);

    // The first item is narrow and now completely off to the left: bringing
    // it into view must scroll back so that its header becomes visible.
    let item = tree_view_item(tree_view.container_from_index(0).unwrap());
    item.bring_into_view();
    root.layout_manager().execute_layout_pass();

    assert!(scroll_viewer.offset().x < 30.0);
}

#[test]
fn bring_into_view_should_not_scroll_when_item_is_already_visible() {
    let _app = test_scope();

    let (root, tree_view) = create_target(vec![create_header(100.0), create_header(500.0), create_header(900.0)]);

    root.layout_manager().execute_initial_layout_pass();

    let scroll_viewer = get_scroll_viewer(&tree_view);
    assert_eq!(0.0, scroll_viewer.offset().x);

    let item = tree_view_item(tree_view.container_from_index(0).unwrap());
    item.bring_into_view();
    root.layout_manager().execute_layout_pass();

    assert_eq!(0.0, scroll_viewer.offset().x);
}

#[test]
fn bring_into_view_should_reveal_a_nested_item() {
    let _app = test_scope();

    // A narrow item nested three levels deep, under wide ancestors that make
    // the tree scroll.
    let nested_header = create_header(100.0);
    let nested_item = TreeViewItem::new();
    nested_item.set_header(Some(Control::boxed(nested_header.clone())));
    let mut item = nested_item.clone();

    for _ in 0..3 {
        let parent = TreeViewItem::new();
        parent.set_header(Some(Control::boxed(create_header(900.0))));
        parent.set_is_expanded(true);
        parent.set_items_source(Some(ItemsSource::from_items(vec![Some(Control::boxed(item))])));
        item = parent;
    }

    let (root, tree_view) = create_target(vec![item.upcast()]);

    root.layout_manager().execute_initial_layout_pass();

    let scroll_viewer = get_scroll_viewer(&tree_view);

    scroll_viewer.set_offset(Vector::new(scroll_viewer.extent().width - scroll_viewer.viewport().width, 0.0));
    root.layout_manager().execute_layout_pass();
    assert!(scroll_viewer.offset().x > 0.0);

    nested_item.bring_into_view();
    root.layout_manager().execute_layout_pass();

    let header_bounds = Rect::from_size(nested_header.bounds().size())
        .transform_to_aabb(nested_header.transform_to_visual(&scroll_viewer).unwrap());

    assert!(header_bounds.left() >= -0.5 && header_bounds.right() <= scroll_viewer.viewport().width + 0.5);
}

fn create_header(width: f64) -> Ref<Control> {
    let header = Border::new();
    header.set_width(width);
    header.set_height(20.0);
    header.set_horizontal_alignment(HorizontalAlignment::Left);
    header.set_background(Some(Brushes::red()));
    header.upcast()
}

fn create_target(headers: Vec<Ref<Control>>) -> (Ref<TestRoot>, Ref<TreeView>) {
    let tree_view = TreeView::new();
    tree_view.set_width(400.0);
    tree_view.set_height(200.0);
    tree_view.set_items_source(Some(ItemsSource::from_items(
        headers.into_iter().map(|header| Some(Control::boxed(header))).collect::<Vec<_>>(),
    )));

    let root = TestRoot::new();
    root.set_width(400.0);
    root.set_height(200.0);
    root.resources().add(TreeView::TYPE, theme_resource(create_tree_view_control_theme()));
    root.resources().add(TreeViewItem::TYPE, theme_resource(create_tree_view_item_control_theme()));
    root.resources().add(ScrollViewer::TYPE, theme_resource(create_scroll_viewer_control_theme()));
    root.set_child(tree_view.clone());

    (root, tree_view)
}

fn get_scroll_viewer(tree_view: &TreeView) -> Ref<ScrollViewer> {
    fn collect(visual: &Visual, result: &mut Vec<Ref<ScrollViewer>>) {
        for child in visual.visual_children().to_vec() {
            if let Some(scroll_viewer) = child.clone().cast::<ScrollViewer>() {
                result.push(scroll_viewer);
            }
            collect(&child, result);
        }
    }

    let mut result = Vec::new();
    collect(tree_view, &mut result);
    assert_eq!(1, result.len());
    result.remove(0)
}

fn tree_view_item(control: Ref<Control>) -> Ref<TreeViewItem> {
    assert!(control.get_type() == TreeViewItem::TYPE);
    control.cast::<TreeViewItem>().unwrap()
}

fn theme_resource(theme: Ref<ControlTheme>) -> Option<BoxedValue> {
    Some(Rc::new(theme))
}

fn template_binding(target: &StyledElement, property: &'static FerroProperty, source: &'static FerroProperty) {
    target.bind_binding(property, &TemplateBinding::new(source));
}

/// The scroll viewer theme: a transparent background and the template of
/// the scroll viewer tests.
fn create_scroll_viewer_control_theme() -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> =
        FuncControlTemplate::for_type::<ScrollViewer>(crate::scroll_viewer_tests::create_template);
    ControlTheme::with_setters(
        ScrollViewer::TYPE,
        [
            Setter::new(TemplatedControl::background_property(), Some(Brushes::transparent())),
            Setter::new(TemplatedControl::template_property(), Some(template)),
        ],
    )
}

fn create_tree_view_control_theme() -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<TreeView>(|_, scope| {
        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        template_binding(
            &items_presenter,
            Layoutable::margin_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        );
        let items_panel = ItemsControl::items_panel_property().as_property();
        template_binding(&items_presenter, items_panel, items_panel);

        let scroll_viewer = ScrollViewer::new();
        for property in [
            ScrollViewer::allow_auto_hide_property().as_property(),
            ScrollViewer::bring_into_view_on_focus_change_property().as_property(),
            TemplatedControl::background_property().as_property(),
            ScrollViewer::horizontal_scroll_bar_visibility_property().as_property(),
            ScrollViewer::is_scroll_chaining_enabled_property().as_property(),
            ScrollViewer::is_deferred_scrolling_enabled_property().as_property(),
            ScrollViewer::vertical_scroll_bar_visibility_property().as_property(),
        ] {
            template_binding(&scroll_viewer, property, property);
        }
        scroll_viewer.set_content(Some(Control::boxed(items_presenter.register_in_name_scope(&**scope))));

        let border = Border::new();
        template_binding(
            &border,
            Border::border_brush_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
        );
        template_binding(
            &border,
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        template_binding(
            &border,
            Border::corner_radius_property().as_property(),
            TemplatedControl::corner_radius_property().as_property(),
        );
        border.set_child(scroll_viewer);
        border.upcast()
    });

    ControlTheme::with_setters(
        TreeView::TYPE,
        [
            Setter::new(TemplatedControl::background_property(), Some(Brushes::transparent())),
            Setter::new(TemplatedControl::padding_property(), Thickness::uniform(4.0)),
            Setter::new(ScrollViewer::horizontal_scroll_bar_visibility_property(), ScrollBarVisibility::Auto),
            Setter::new(ScrollViewer::vertical_scroll_bar_visibility_property(), ScrollBarVisibility::Auto),
            Setter::new(ScrollViewer::is_scroll_chaining_enabled_property(), true),
            Setter::new(TemplatedControl::template_property(), Some(template)),
        ],
    )
}

fn create_toggle_button_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ToggleButton>(|_, _| {
        let border = Border::new();
        border.set_width(14.0);
        border.set_height(12.0);
        border.set_horizontal_alignment(HorizontalAlignment::Center);
        border.set_vertical_alignment(VerticalAlignment::Center);
        border.set_background(Some(Brushes::transparent()));
        border.upcast()
    })
}

fn create_tree_view_item_control_theme() -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<TreeViewItem>(|item, scope| {
        let chevron = ToggleButton::new();
        chevron.set_name(Some("PART_ExpandCollapseChevron".to_string()));
        chevron.set_focusable(false);
        chevron.set_background(Some(Brushes::transparent()));
        chevron.bind_binding(
            ToggleButton::is_checked_property().as_property(),
            &TemplateBinding::new(TreeViewItem::is_expanded_property().as_property()).with_mode(BindingMode::TwoWay),
        );
        chevron.set_template(Some(create_toggle_button_template()));

        let header_presenter = ContentPresenter::new();
        header_presenter.set_name(Some("PART_HeaderPresenter".to_string()));
        Grid::set_column(&header_presenter, 1);
        header_presenter.set_background(Some(Brushes::transparent()));
        // The theme template binds the padding to the one of the item (2);
        // a style of the theme sets the same value.
        header_presenter.set_padding(Thickness::uniform(2.0));
        template_binding(
            &header_presenter,
            ContentPresenter::horizontal_content_alignment_property().as_property(),
            Layoutable::horizontal_alignment_property().as_property(),
        );
        template_binding(
            &header_presenter,
            ContentPresenter::content_property().as_property(),
            HeaderedItemsControl::header_property().as_property(),
        );
        template_binding(
            &header_presenter,
            ContentPresenter::content_template_property().as_property(),
            HeaderedItemsControl::header_template_property().as_property(),
        );
        header_presenter.set_focusable(false);

        let header = Grid::new();
        header.set_name(Some("PART_Header".to_string()));
        header.set_column_definitions(ColumnDefinitions::from_items([
            ColumnDefinition::with_width(GridLength::new(16.0, GridUnitType::Pixel)),
            ColumnDefinition::with_value(1.0, GridUnitType::Star),
        ]));
        let level: Rc<dyn BindingBase> = Rc::new(IndexerBinding::new(
            item.clone().upcast(),
            TreeViewItem::level_property().as_property(),
            BindingMode::OneWay,
        ));
        // The indent of the theme.
        let indent: Rc<dyn BindingBase> =
            ReflectionBinding::empty().with_source(Some(Rc::new(16.0_f64)));
        header.bind_binding(
            Layoutable::margin_property().as_property(),
            &MultiBinding::new().with_bindings(vec![level, indent]).with_converter_value(Some(TreeViewItemIndentConverter::instance())),
        );
        header.children().add(chevron.register_in_name_scope(&**scope));
        header.children().add(header_presenter.register_in_name_scope(&**scope));

        let selection_border = Border::new();
        selection_border.set_name(Some("SelectionBorder".to_string()));
        template_binding(
            &selection_border,
            Border::background_property().as_property(),
            TemplatedControl::background_property().as_property(),
        );
        template_binding(
            &selection_border,
            Border::border_brush_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
        );
        template_binding(
            &selection_border,
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        template_binding(
            &selection_border,
            Border::corner_radius_property().as_property(),
            TemplatedControl::corner_radius_property().as_property(),
        );
        selection_border.set_focusable(true);
        selection_border.set_value(TemplatedControl::is_template_focus_target_property(), true);
        selection_border.set_child(header.register_in_name_scope(&**scope));

        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        template_binding(
            &items_presenter,
            Visual::is_visible_property().as_property(),
            TreeViewItem::is_expanded_property().as_property(),
        );
        let items_panel = ItemsControl::items_panel_property().as_property();
        template_binding(&items_presenter, items_panel, items_panel);

        let panel = StackPanel::new();
        panel.children().add(selection_border.register_in_name_scope(&**scope));
        panel.children().add(items_presenter.register_in_name_scope(&**scope));
        panel.upcast()
    });

    ControlTheme::with_setters(
        TreeViewItem::TYPE,
        [
            Setter::new(TemplatedControl::padding_property(), Thickness::uniform(2.0)),
            Setter::new(TemplatedControl::background_property(), Some(Brushes::transparent())),
            Setter::new(TemplatedControl::template_property(), Some(template)),
        ],
    )
}
