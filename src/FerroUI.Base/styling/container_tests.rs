//! Tests for container queries, theme variants and style queries.

use super::test_support::*;
use super::*;
use crate::layout::Layoutable;
use crate::*;

fn width_setter(value: f64) -> std::rc::Rc<Setter> {
    Setter::new(Layoutable::width_property(), value)
}

fn height_setter(value: f64) -> std::rc::Rc<Setter> {
    Setter::new(Layoutable::height_property(), value)
}

fn container_query(query: StyleQuery, name: Option<&str>, setter: std::rc::Rc<Setter>) -> Ref<ContainerQuery> {
    let container_query = ContainerQuery::with_query(query, name.map(str::to_string));
    container_query.children().add(Style::with_setters(Selectors::is::<Class1>(), [setter]));
    container_query
}

/// Builds `root > container > child` with the styles of `queries` on the
/// root.
fn tree(queries: &[Ref<ContainerQuery>]) -> (Ref<TestRoot>, Ref<Class3>, Ref<Class1>) {
    let root = TestRoot::new();
    for query in queries {
        root.styles().add(query);
    }
    let container = Class3::new();
    let child = Class1::new();
    set_child(&container, &child);
    (root, container, child)
}

#[test]
#[should_panic(expected = "Container cannot be added as a nested style.")]
fn container_cannot_be_added_to_style_children() {
    let target = ContainerQuery::new();
    let style = Style::new();
    style.children().add(target);
}

#[test]
fn container_can_be_added_to_control_theme_children() {
    let target = ContainerQuery::new();
    let theme = ControlTheme::for_type::<Class1>();
    theme.children().add(&target);
    assert_eq!(target.parent().unwrap(), theme);
}

#[test]
fn container_width_queries_matches() {
    let (root, container, child) = tree(&[
        container_query(
            StyleQueries::width(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
            None,
            width_setter(200.0),
        ),
        container_query(
            StyleQueries::width(None, StyleQueryComparisonOperator::GreaterThan, 500.0),
            None,
            width_setter(500.0),
        ),
    ]);
    Container::set_sizing(&container, ContainerSizing::Width);
    set_child(&root, &container);

    root.measure(Size::new(400.0, 400.0));
    assert_eq!(child.width(), 200.0);

    root.invalidate_measure();
    container.invalidate_measure();
    root.measure(Size::new(600.0, 600.0));
    assert_eq!(child.width(), 500.0);
}

#[test]
fn container_height_queries_matches() {
    let (root, container, child) = tree(&[
        container_query(
            StyleQueries::height(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
            None,
            height_setter(200.0),
        ),
        container_query(
            StyleQueries::height(None, StyleQueryComparisonOperator::GreaterThan, 500.0),
            None,
            height_setter(500.0),
        ),
    ]);
    Container::set_sizing(&container, ContainerSizing::Height);
    set_child(&root, &container);

    root.measure(Size::new(400.0, 400.0));
    assert_eq!(child.height(), 200.0);

    root.invalidate_measure();
    container.invalidate_measure();
    root.measure(Size::new(600.0, 600.0));
    assert_eq!(child.height(), 500.0);
}

#[test]
fn container_width_queries_matches_name() {
    let (root, container, child) = tree(&[
        container_query(
            StyleQueries::width(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
            None,
            width_setter(200.0),
        ),
        container_query(
            StyleQueries::width(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
            Some("TEST"),
            width_setter(300.0),
        ),
    ]);
    Container::set_sizing(&container, ContainerSizing::Width);
    Container::set_name(&container, Some("TEST".to_string()));
    set_child(&root, &container);

    root.measure(Size::new(400.0, 400.0));
    assert_eq!(child.width(), 300.0);
}

#[test]
fn container_height_queries_matches_name() {
    let (root, container, child) = tree(&[
        container_query(
            StyleQueries::height(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
            None,
            height_setter(200.0),
        ),
        container_query(
            StyleQueries::height(None, StyleQueryComparisonOperator::LessThanOrEquals, 450.0),
            Some("TEST"),
            height_setter(300.0),
        ),
    ]);
    Container::set_sizing(&container, ContainerSizing::Height);
    Container::set_name(&container, Some("TEST".to_string()));
    set_child(&root, &container);

    root.measure(Size::new(400.0, 400.0));
    assert_eq!(child.height(), 300.0);
}

#[test]
fn container_queries_do_not_match_without_container() {
    let (root, container, child) = tree(&[container_query(
        StyleQueries::width(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
        None,
        width_setter(200.0),
    )]);
    set_child(&root, &container);

    root.measure(Size::new(400.0, 400.0));
    assert!(child.width().is_nan());
}

#[test]
fn and_or_queries_combine_size_queries() {
    let and = StyleQueries::and([
        StyleQueries::width(None, StyleQueryComparisonOperator::GreaterThanOrEquals, 300.0),
        StyleQueries::height(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
    ]);
    let or = StyleQueries::or([
        StyleQueries::width(None, StyleQueryComparisonOperator::GreaterThan, 1000.0),
        StyleQueries::height(None, StyleQueryComparisonOperator::GreaterThan, 1000.0),
    ]);
    assert_eq!(and.to_string(), "min-width:300 and max-height:500");

    let (root, container, child) =
        tree(&[container_query(and, None, width_setter(200.0)), container_query(or, None, height_setter(50.0))]);
    Container::set_sizing(&container, ContainerSizing::WidthAndHeight);
    set_child(&root, &container);

    root.measure(Size::new(400.0, 400.0));
    assert_eq!(child.width(), 200.0);
    assert!(child.height().is_nan());

    root.invalidate_measure();
    container.invalidate_measure();
    root.measure(Size::new(400.0, 1200.0));
    assert!(child.width().is_nan());
    assert_eq!(child.height(), 50.0);
}

#[test]
fn width_query_without_subscription_requires_width_and_height_container() {
    let container = Class3::new();
    let child = Class1::new();
    set_child(&container, &child);
    Container::set_sizing(&container, ContainerSizing::WidthAndHeight);
    container.measure(Size::new(100.0, 100.0));

    let query = StyleQueries::width(None, StyleQueryComparisonOperator::LessThan, 200.0);
    let element: Ref<StyledElement> = child.clone().upcast();
    assert_eq!(query.match_(&element, None, false, None).result(), SelectorMatchResult::AlwaysThisInstance);

    let query = StyleQueries::width(None, StyleQueryComparisonOperator::GreaterThan, 200.0);
    assert_eq!(query.match_(&element, None, false, None).result(), SelectorMatchResult::NeverThisInstance);
}

#[test]
fn sizing_creates_and_removes_the_query_provider() {
    let container = Class3::new();
    assert!(Container::get_query_provider(&container).is_none());
    Container::set_sizing(&container, ContainerSizing::Width);
    assert!(Container::get_query_provider(&container).is_some());
    assert_eq!(Container::get_sizing(&container), ContainerSizing::Width);
    Container::set_sizing(&container, ContainerSizing::Normal);
    assert!(Container::get_query_provider(&container).is_none());
}

#[test]
fn width_container_takes_the_available_width() {
    let container = Class3::new();
    Container::set_sizing(&container, ContainerSizing::Width);
    container.measure(Size::new(120.0, 80.0));
    assert_eq!(container.desired_size(), Size::new(120.0, 0.0));
}

// --- theme variant -----------------------------------------------------------

#[test]
fn theme_variants_are_compared_by_key() {
    assert_eq!(ThemeVariant::light(), ThemeVariant::light());
    assert_ne!(ThemeVariant::light(), ThemeVariant::dark());
    assert_eq!(ThemeVariant::new("Custom", None), ThemeVariant::new("Custom", None));
    assert_eq!(ThemeVariant::new("Light", None), ThemeVariant::light());
    assert_eq!(ThemeVariant::dark().to_string(), "Dark");
    assert_eq!("Dark".parse::<ThemeVariant>(), Ok(ThemeVariant::dark()));
    assert!("Custom".parse::<ThemeVariant>().is_err());
}

#[test]
#[should_panic(expected = "Inheriting default theme variant is not supported.")]
fn theme_variant_cannot_inherit_default() {
    ThemeVariant::new("Custom", Some(ThemeVariant::default()));
}

#[test]
fn requested_theme_variant_sets_actual_theme_variant_and_is_inherited() {
    let root = TestRoot::new();
    let child = Class1::new();
    set_child(&root, &child);
    assert_eq!(child.actual_theme_variant(), None);

    let raised = std::rc::Rc::new(std::cell::Cell::new(0));
    let r = raised.clone();
    child.actual_theme_variant_changed(move || r.set(r.get() + 1));

    root.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));
    assert_eq!(root.actual_theme_variant(), Some(ThemeVariant::dark()));
    assert_eq!(child.actual_theme_variant(), Some(ThemeVariant::dark()));
    assert_eq!(raised.get(), 1);

    child.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::light()));
    assert_eq!(child.actual_theme_variant(), Some(ThemeVariant::light()));

    child.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::default()));
    assert_eq!(child.actual_theme_variant(), Some(ThemeVariant::dark()));
}
