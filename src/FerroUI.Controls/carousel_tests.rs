use crate::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter};
use crate::primitives::{ScrollBarVisibility, SelectingItemsControl};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{string_of, test_scope, TestRoot, TestScope};
use crate::{Canvas, Carousel, Control, DataValidationErrors, ItemsControl, ItemsSource, Panel, ScrollViewer, TextBlock};
use ferroui_base::animation::{IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority, TemplateBinding};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::reactive::Observable;
use ferroui_base::{AnyValue, BoxedValue, Ref, StaticType};
use std::rc::Rc;

fn start() -> TestScope {
    test_scope()
}

/// Hosts the carousel in a root and lays it out. The root is returned
/// because it owns the tree.
fn prepare(target: &Ref<Carousel>) -> Ref<TestRoot> {
    let root = TestRoot::with_child(target.clone());
    root.layout_manager().execute_initial_layout_pass();
    root
}

fn layout(target: &Carousel) {
    if let Some(layout_manager) = target.get_layout_manager() {
        layout_manager.execute_layout_pass();
    }
}

fn carousel_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::new(|_, ns| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        scroll_viewer.set_template(Some(scroll_viewer_template()));
        scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**ns))));
        scroll_viewer.register_in_name_scope(&**ns).upcast()
    }))
}

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));

        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.upcast()
    })
}

fn get_container_text_block(control: Option<Ref<Control>>) -> Ref<TextBlock> {
    let control = control.expect("Expected a container.");
    assert!(std::ptr::eq(control.get_type(), <ContentPresenter as StaticType>::TYPE));
    let content_presenter = control.cast::<ContentPresenter>().unwrap();
    let child = content_presenter.child().expect("Expected a child.");
    assert!(std::ptr::eq(child.get_type(), <TextBlock as StaticType>::TYPE));
    child.cast::<TextBlock>().unwrap()
}

fn new_target() -> Ref<Carousel> {
    let target = Carousel::new();
    target.set_template(carousel_template());
    target
}

fn selected_item(target: &Carousel) -> Option<String> {
    target.selected_item().and_then(|item| string_of(&item))
}

fn strings(values: &[&str]) -> Rc<FerroList<String>> {
    Rc::new(FerroList::from_items(values.iter().map(|x| x.to_string())))
}

fn page_slide(orientation: SlideAxis) -> Option<Rc<dyn IPageTransition>> {
    let transition: Rc<dyn IPageTransition> =
        Rc::new(PageSlide::with_duration(TimeSpan::from_milliseconds(100.0), orientation));
    Some(transition)
}

fn raise_key_down(target: &Carousel, key: Key) {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    target.raise_event(&args);
}

#[test]
fn first_item_should_be_selected_by_default() {
    let _app = start();
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar"])));

    let _root = prepare(&target);

    assert_eq!(0, target.selected_index());
    assert_eq!(Some("Foo".to_string()), selected_item(&target));
}

#[test]
fn logical_child_should_be_selected_item() {
    let _app = start();
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar"])));

    let _root = prepare(&target);

    assert_eq!(1, target.get_realized_containers().len());

    let child = get_container_text_block(target.get_realized_containers().into_iter().next());

    assert_eq!(Some("Foo".to_string()), child.text());
}

#[test]
fn viewport_fraction_defaults_to_one() {
    let _app = start();
    let target = Carousel::new();

    assert_eq!(1.0, target.viewport_fraction());
}

#[test]
fn viewport_fraction_coerces_invalid_values_to_one() {
    let _app = start();
    let target = Carousel::new();

    target.set_viewport_fraction(0.0);
    assert_eq!(1.0, target.viewport_fraction());

    target.set_viewport_fraction(f64::NAN);
    assert_eq!(1.0, target.viewport_fraction());
}

#[test]
fn selected_item_changes_to_first_item_when_items_property_changes() {
    let _app = start();
    let items = strings(&["Foo", "Bar", "FooBar"]);

    let target = new_target();
    target.set_items_source(Some(items.clone().into()));

    let _root = prepare(&target);

    assert_eq!(1, target.get_realized_containers().len());

    let child = get_container_text_block(target.get_realized_containers().into_iter().next());

    assert_eq!(Some("Foo".to_string()), child.text());

    let mut new_items = items.to_vec();
    new_items.remove(0);
    layout(&target);

    target.set_items_source(Some(ItemsSource::from_values(new_items)));
    layout(&target);

    let containers = target.get_realized_containers();
    assert_eq!(1, containers.len());
    let child = get_container_text_block(containers.into_iter().next());

    assert_eq!(Some("Bar".to_string()), child.text());
}

#[test]
fn selected_item_changes_to_first_item_when_item_added() {
    let _app = start();
    let items = strings(&[]);
    let target = new_target();
    target.set_items_source(Some(items.clone().into()));

    let _root = prepare(&target);

    assert_eq!(-1, target.selected_index());
    assert!(target.get_realized_containers().is_empty());

    items.add("Foo".to_string());
    layout(&target);

    assert_eq!(0, target.selected_index());
    assert_eq!(1, target.get_realized_containers().len());
}

#[test]
fn selected_index_changes_to_none_when_items_assigned_null() {
    let _app = start();
    let items = strings(&["Foo", "Bar", "FooBar"]);

    let target = new_target();
    target.set_items_source(Some(items.into()));

    let _root = prepare(&target);

    assert_eq!(1, target.get_realized_containers().len());

    let child = get_container_text_block(target.get_realized_containers().into_iter().next());

    assert_eq!(Some("Foo".to_string()), child.text());

    target.set_items_source(None);
    layout(&target);

    assert!(target.get_realized_containers().is_empty());
    assert_eq!(-1, target.selected_index());
}

#[test]
fn selected_index_is_maintained_carousel_created_with_non_zero_selected_index() {
    let _app = start();
    let items = strings(&["Foo", "Bar", "FooBar"]);

    let target = new_target();
    target.set_items_source(Some(items.into()));
    target.set_selected_index(2);

    let _root = prepare(&target);

    assert_eq!(Some("FooBar".to_string()), selected_item(&target));

    let child = get_container_text_block(target.get_realized_containers().into_iter().last());

    assert_eq!(Some("FooBar".to_string()), child.text());
}

#[test]
fn selected_item_changes_to_next_first_item_when_item_removed_from_beggining_of_list() {
    let _app = start();
    let items = strings(&["Foo", "Bar", "FooBar"]);

    let target = new_target();
    target.set_items_source(Some(items.clone().into()));

    let _root = prepare(&target);

    let child = get_container_text_block(target.get_realized_containers().into_iter().next());

    assert_eq!(Some("Foo".to_string()), child.text());

    items.remove_at(0);
    layout(&target);

    let child = get_container_text_block(target.get_realized_containers().into_iter().next());

    assert!(std::ptr::eq(child.get_type(), <TextBlock as StaticType>::TYPE));
    assert_eq!(Some("Bar".to_string()), child.text());
}

#[test]
fn selected_item_changes_to_first_item_if_selected_item_is_removed_from_middle() {
    let _app = start();
    let items = strings(&["Foo", "Bar", "FooBar"]);

    let target = new_target();
    target.set_items_source(Some(items.clone().into()));

    let _root = prepare(&target);

    target.set_selected_index(1);

    items.remove_at(1);

    assert_eq!(0, target.selected_index());
    assert_eq!(Some("Foo".to_string()), selected_item(&target));
}

#[test]
fn selected_item_validation() {
    let _app = start();
    let target = new_target();

    let _root = prepare(&target);

    let exception = BindingError::message("failed validation");
    let text_observable = Observable::single_value(Rc::new(BindingNotification::with_error(
        exception.clone(),
        BindingErrorType::DataValidationError,
    )) as BoxedValue);
    target.bind_property_untyped(
        SelectingItemsControl::selected_item_property().as_property(),
        text_observable,
        BindingPriority::LocalValue,
    );

    assert!(DataValidationErrors::get_has_errors(&target));
    let errors = DataValidationErrors::get_errors(&target).unwrap();
    assert_eq!(1, errors.len());
    let error = errors[0].clone();
    let error: &dyn AnyValue = &*error;
    assert!(error.downcast_ref::<BindingError>() == Some(&exception));
}

#[test]
fn can_move_forward_back_forward() {
    let _app = start();
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_strs(["foo", "bar"])));

    let _root = prepare(&target);

    target.set_selected_index(1);
    layout(&target);

    assert_eq!(1, target.selected_index());

    target.set_selected_index(0);
    layout(&target);

    assert_eq!(0, target.selected_index());

    target.set_selected_index(1);
    layout(&target);

    assert_eq!(1, target.selected_index());
}

#[test]
fn can_move_forward_back_forward_with_control_items() {
    // Issue #11119
    let _app = start();
    let items = [Canvas::new(), Canvas::new()];
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_items(items.iter().map(|x| Some(Control::boxed(x.clone()))))));

    let _root = prepare(&target);

    target.set_selected_index(1);
    layout(&target);

    assert_eq!(1, target.selected_index());

    target.set_selected_index(0);
    layout(&target);

    assert_eq!(0, target.selected_index());

    target.set_selected_index(1);
    let _subscription = target.property_changed(|e| {
        if e.property() == SelectingItemsControl::selected_index_property().as_property() {}
    });
    layout(&target);

    assert_eq!(1, target.selected_index());
}

mod wrap_selection_tests {
    use super::*;

    fn create(wrap_selection: bool, selected_index: i32) -> Ref<Carousel> {
        let target = new_target();
        target.set_items_source(Some(ItemsSource::from_strs(["foo", "bar", "baz"])));
        target.set_wrap_selection(wrap_selection);
        target.set_selected_index(selected_index);
        target
    }

    #[test]
    fn next_loops_when_wrap_selection_is_true() {
        let _app = start();
        let target = create(true, 2);

        let _root = prepare(&target);

        target.next();
        layout(&target);

        assert_eq!(0, target.selected_index());
    }

    #[test]
    fn previous_loops_when_wrap_selection_is_true() {
        let _app = start();
        let target = create(true, 0);

        let _root = prepare(&target);

        target.previous();
        layout(&target);

        assert_eq!(2, target.selected_index());
    }

    #[test]
    fn next_does_not_loop_when_wrap_selection_is_false() {
        let _app = start();
        let target = create(false, 2);

        let _root = prepare(&target);

        target.next();
        layout(&target);

        assert_eq!(2, target.selected_index());
    }

    #[test]
    fn previous_does_not_loop_when_wrap_selection_is_false() {
        let _app = start();
        let target = create(false, 0);

        let _root = prepare(&target);

        target.previous();
        layout(&target);

        assert_eq!(0, target.selected_index());
    }
}

#[test]
fn right_arrow_navigates_to_next_with_horizontal_page_slide() {
    let _app = start();
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar", "Baz"])));
    target.set_page_transition(page_slide(SlideAxis::Horizontal));

    let _root = prepare(&target);
    assert_eq!(0, target.selected_index());

    raise_key_down(&target, Key::Right);
    assert_eq!(1, target.selected_index());
}

#[test]
fn down_arrow_navigates_to_next_with_vertical_page_slide() {
    let _app = start();
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar", "Baz"])));
    target.set_page_transition(page_slide(SlideAxis::Vertical));

    let _root = prepare(&target);
    assert_eq!(0, target.selected_index());

    raise_key_down(&target, Key::Down);
    assert_eq!(1, target.selected_index());
}

#[test]
fn home_navigates_to_first_item() {
    let _app = start();
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar", "Baz"])));
    target.set_selected_index(2);

    let _root = prepare(&target);
    layout(&target);
    assert_eq!(2, target.selected_index());

    raise_key_down(&target, Key::Home);
    assert_eq!(0, target.selected_index());
}

#[test]
fn end_navigates_to_last_item() {
    let _app = start();
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar", "Baz"])));

    let _root = prepare(&target);
    assert_eq!(0, target.selected_index());

    raise_key_down(&target, Key::End);
    assert_eq!(2, target.selected_index());
}

#[test]
fn wrong_axis_arrow_is_ignored() {
    let _app = start();
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar", "Baz"])));
    target.set_page_transition(page_slide(SlideAxis::Horizontal));

    let _root = prepare(&target);
    assert_eq!(0, target.selected_index());

    raise_key_down(&target, Key::Down);
    assert_eq!(0, target.selected_index());
}

#[test]
fn left_arrow_wraps_with_wrap_selection() {
    let _app = start();
    let target = new_target();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar", "Baz"])));
    target.set_page_transition(page_slide(SlideAxis::Horizontal));
    target.set_wrap_selection(true);

    let _root = prepare(&target);
    assert_eq!(0, target.selected_index());

    raise_key_down(&target, Key::Left);
    assert_eq!(2, target.selected_index());
}
