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

/// A styled element that is not a visual, with a styled number property.
#[repr(C)]
struct NonVisualStyledElement {
    base: StyledElement,
}

ferro_class!(NonVisualStyledElement: StyledElement);
ferro_impl_classes!(NonVisualStyledElement: FerroObjectImpl, StyledElementImpl);

impl NonVisualStyledElement {
    ferro_property!(pub fn value_property() -> StyledProperty<i32> {
        FerroProperty::register::<NonVisualStyledElement, _>("Value", 0)
    });

    fn new() -> Ref<Self> {
        Self::value_property();
        instantiate(Self { base: StyledElement::construct() })
    }

    fn value(&self) -> i32 {
        self.get_value(Self::value_property())
    }
}

#[derive(Clone, Copy, Debug)]
enum ContainerQueryType {
    Width,
    Height,
    And,
    Or,
}

fn non_visual_query(query: StyleQuery, name: Option<&str>) -> Ref<ContainerQuery> {
    let container_query = ContainerQuery::with_query(query, name.map(str::to_string));
    container_query.children().add(Style::with_setters(
        Selectors::is::<NonVisualStyledElement>(),
        [Setter::new(NonVisualStyledElement::value_property(), 42)],
    ));
    container_query
}

#[test]
fn container_does_not_query_itself() {
    let root = TestRoot::new();
    let container_query = ContainerQuery::with_query(
        StyleQueries::width(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
        None,
    );
    container_query
        .children()
        .add(Style::with_setters(Selectors::is::<Class3>(), [Setter::new(Visual::opacity_property(), 0.5)]));
    root.styles().add(&container_query);
    let container = Class3::new();
    Container::set_sizing(&container, ContainerSizing::Width);
    set_child(&root, &container);

    root.measure(Size::new(400.0, 400.0));

    assert_eq!(container.opacity(), 1.0);
}

#[test]
fn container_query_tracks_non_visual_target_after_logical_reparenting() {
    // Upstream styles a text run, a non-visual element, through its theme and
    // reparents it between two text blocks; here a non-visual element with a
    // styled property stands in for the run and its data context.
    let root = TestRoot::new();
    let container_query = non_visual_query(
        StyleQueries::width(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
        None,
    );
    let theme = ControlTheme::for_type::<NonVisualStyledElement>();
    theme.children().add(&container_query);
    let target = NonVisualStyledElement::new();
    target.set_theme(&theme);
    target.apply_styling();
    let small_container = Class3::new();
    small_container.set_width(400.0);
    let large_container = Class3::new();
    large_container.set_width(600.0);
    Container::set_sizing(&small_container, ContainerSizing::Width);
    Container::set_sizing(&large_container, ContainerSizing::Width);
    let panel = TestPanel::new();
    panel.add_child(&small_container);
    panel.add_child(&large_container);
    set_child(&root, &panel);

    root.measure(Size::new(1000.0, 400.0));

    target.set_parent(&small_container);
    assert_eq!(target.value(), 42);

    target.set_parent(None);
    target.set_parent(&large_container);
    assert_eq!(target.value(), 0);

    large_container.set_width(400.0);
    root.invalidate_measure();
    panel.invalidate_measure();
    large_container.invalidate_measure();
    root.measure(Size::new(1000.0, 400.0));
    assert_eq!(target.value(), 42);
}

#[test]
fn container_query_matches_non_visual_styled_element() {
    for query_type in [ContainerQueryType::Width, ContainerQueryType::Height, ContainerQueryType::And, ContainerQueryType::Or]
    {
        let root = TestRoot::new();
        let client_size =
            if matches!(query_type, ContainerQueryType::Or) { Size::new(400.0, 600.0) } else { Size::new(400.0, 400.0) };
        let width_query = || StyleQueries::width(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0);
        let height_query = || StyleQueries::height(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0);
        let query = match query_type {
            ContainerQueryType::Width => width_query(),
            ContainerQueryType::Height => height_query(),
            ContainerQueryType::And => StyleQueries::and([width_query(), height_query()]),
            ContainerQueryType::Or => StyleQueries::or([width_query(), height_query()]),
        };
        root.styles().add(&non_visual_query(query, None));
        let container = Class3::new();
        Container::set_sizing(
            &container,
            match query_type {
                ContainerQueryType::Width => ContainerSizing::Width,
                ContainerQueryType::Height => ContainerSizing::Height,
                _ => ContainerSizing::WidthAndHeight,
            },
        );
        set_child(&root, &container);
        let target = NonVisualStyledElement::new();
        target.set_parent(&container);

        root.measure(client_size);
        assert_eq!(target.value(), 42, "{query_type:?}");

        let client_size = match query_type {
            ContainerQueryType::Width | ContainerQueryType::And => Size::new(600.0, 400.0),
            ContainerQueryType::Height => Size::new(400.0, 600.0),
            ContainerQueryType::Or => Size::new(600.0, 600.0),
        };
        root.invalidate_measure();
        root.measure(client_size);
        assert_eq!(target.value(), 0, "{query_type:?}");
    }
}

#[test]
fn container_query_uses_named_visual_ancestor_for_non_visual_target() {
    let root = TestRoot::new();
    root.styles().add(&non_visual_query(
        StyleQueries::width(None, StyleQueryComparisonOperator::LessThanOrEquals, 500.0),
        Some("target"),
    ));
    let inner_container = Class3::new();
    inner_container.set_width(400.0);
    Container::set_name(&inner_container, Some("other".to_string()));
    Container::set_sizing(&inner_container, ContainerSizing::Width);
    let named_container = Class3::new();
    named_container.set_width(600.0);
    set_child(&named_container, &inner_container);
    Container::set_name(&named_container, Some("target".to_string()));
    Container::set_sizing(&named_container, ContainerSizing::Width);
    set_child(&root, &named_container);
    let target = NonVisualStyledElement::new();
    target.set_parent(&inner_container);

    root.measure(Size::new(1000.0, 400.0));
    assert_eq!(target.value(), 0);

    named_container.set_width(400.0);
    root.invalidate_measure();
    named_container.invalidate_measure();
    root.measure(Size::new(1000.0, 400.0));
    assert_eq!(target.value(), 42);
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
    child.actual_theme_variant_changed(move || r.set(r.get() + 1)).detach();

    root.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));
    assert_eq!(root.actual_theme_variant(), Some(ThemeVariant::dark()));
    assert_eq!(child.actual_theme_variant(), Some(ThemeVariant::dark()));
    assert_eq!(raised.get(), 1);

    child.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::light()));
    assert_eq!(child.actual_theme_variant(), Some(ThemeVariant::light()));

    child.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::default()));
    assert_eq!(child.actual_theme_variant(), Some(ThemeVariant::dark()));
}
