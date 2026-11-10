use super::test_support::*;
use super::theme_candidates::builds;
use super::ThemeStep::{Attach, Descend, Whole};
use super::*;
use crate::layout::{Layoutable, LayoutableImpl};
use crate::*;
use std::cell::Cell;
use std::rc::Rc;

fn foo(value: &str) -> Rc<Setter> {
    Setter::new(Class1::foo_property(), value.to_string())
}

fn opacity(value: f64) -> Rc<Setter> {
    Setter::new(Visual::opacity_property(), value)
}

fn z_index(value: i32) -> Rc<Setter> {
    Setter::new(Visual::z_index_property(), value)
}

fn nest() -> Selector {
    Selectors::nesting(None)
}

/// `^ /template/ T`: a part of the template of the control the theme is for.
fn in_template<T: ObjectType + Upcast<StyledElement>>() -> Selector {
    nest().template().of_type::<T>()
}

fn plain(selector: Selector, setter: Rc<Setter>) -> Ref<Style> {
    Style::with_setters(selector, [setter])
}

/// A style as the generator of the themes makes it: the selector is index-enabled.
fn style(selector: Selector, setter: Rc<Setter>) -> Ref<Style> {
    let style = plain(selector, setter);
    style.enable_selector_index();
    style
}

fn describe(steps: &[(Rc<dyn IStyle>, ThemeStep)]) -> String {
    steps
        .iter()
        .map(|(style, step)| {
            let name = style
                .as_object()
                .and_then(|object| object.downcast_ref::<StyleBase>())
                .map_or_else(|| "custom".to_string(), StyleBase::to_display_string);
            format!("{name}: {step:?}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn steps(theme: &ControlTheme, target: &StyledElement) -> Vec<(Rc<dyn IStyle>, ThemeStep)> {
    theme.candidates(target).expect("the theme has a plan for the element").steps()
}

#[track_caller]
fn assert_steps(theme: &ControlTheme, target: &StyledElement, expected: &[(&Ref<Style>, ThemeStep)]) {
    let actual = steps(theme, target);
    assert_eq!(actual.len(), expected.len(), "{}", describe(&actual));
    for ((style, step), (expected_style, expected_step)) in actual.iter().zip(expected) {
        let expected_style: Rc<dyn IStyle> = (*expected_style).clone().into();
        assert!(style_ptr_eq(style, &expected_style), "{}", describe(&actual));
        assert_eq!(step, expected_step, "{}", describe(&actual));
    }
}

/// A theme for `Class1` with styles for the states of the control and for a `Class3` part of its
/// template, nested as the themes nest them:
///
/// ```text
/// ^.a                       foo = a
///     ^.b                   foo = a b
///     ^ /template/ Class3   opacity = 0.5
/// ^ /template/ Class3.x     z-index = 3
/// ^.c                       foo = c        (never index-enabled)
/// ^.d                       z-index = 9
///     ^ /template/ Class3   height = 7
/// ```
struct Scenario {
    theme: Ref<ControlTheme>,
    a: Ref<Style>,
    a_b: Ref<Style>,
    a_part: Ref<Style>,
    part: Ref<Style>,
    plain: Ref<Style>,
    d: Ref<Style>,
    d_part: Ref<Style>,
}

fn scenario(enable: bool) -> Scenario {
    let make = |selector: Selector, setter: Rc<Setter>| if enable { style(selector, setter) } else { plain(selector, setter) };
    let theme = ControlTheme::with_setters(Class1::TYPE, [foo("theme")]);
    let a = make(nest().class("a"), foo("a"));
    let a_b = make(nest().class("b"), foo("a b"));
    let a_part = make(in_template::<Class3>(), opacity(0.5));
    let part = make(in_template::<Class3>().class("x"), z_index(3));
    let plain = plain(nest().class("c"), foo("c"));
    let d = make(nest().class("d"), z_index(9));
    let d_part = make(in_template::<Class3>(), Setter::new(Layoutable::height_property(), 7.0));
    // A nested style is added to its parent last, as the generator of the themes adds it.
    a.children().add(a_b.clone());
    a.children().add(a_part.clone());
    d.children().add(d_part.clone());
    theme.children().add(a.clone());
    theme.children().add(part.clone());
    theme.children().add(plain.clone());
    theme.children().add(d.clone());
    Scenario { theme, a, a_b, a_part, part, plain, d, d_part }
}

#[test]
fn a_plan_lists_the_styles_that_can_match_the_key_in_source_order() {
    let s = scenario(true);
    let control = Class1::new();
    let derived = Class2::new();
    let part = Class3::new();
    let panel = TestPanel::new();

    let for_control = [(&s.a, Attach), (&s.a_b, Attach), (&s.plain, Attach), (&s.d, Attach)];
    assert_steps(&s.theme, &control, &for_control);
    assert_steps(&s.theme, &derived, &for_control);
    // A style that cannot match the part is kept for the styles below it.
    assert_steps(
        &s.theme,
        &part,
        &[(&s.a, Descend), (&s.a_part, Attach), (&s.part, Attach), (&s.plain, Attach), (&s.d, Descend), (&s.d_part, Attach)],
    );
    // A style whose selector is not index-enabled is a candidate for every element.
    assert_steps(&s.theme, &panel, &[(&s.plain, Attach)]);

    // The control and the control derived from it have the same entries, and one plan.
    assert_eq!(s.theme.plan_stats(), (4, 3, 4 + 6 + 1));
}

#[test]
fn a_plan_is_built_once_for_a_key() {
    let s = scenario(true);
    let control = Class1::new();
    let other = Class1::new();
    let part = Class3::new();
    let before = builds();
    steps(&s.theme, &control);
    steps(&s.theme, &other);
    steps(&s.theme, &part);
    steps(&s.theme, &control);
    assert_eq!(builds() - before, 2);
}

/// What a control with the theme of the scenario and a part of its template end with.
fn applied(enable: bool) -> (String, i32, f64, i32, f64, usize, usize) {
    let s = scenario(enable);
    let control = Class1::new();
    for class in ["a", "b", "d"] {
        control.classes().add(class);
    }
    control.set_theme(s.theme.clone());
    let part = Class3::new();
    part.classes().add("x");
    part.set_templated_parent(control.clone().upcast::<FerroObject>());
    set_child(&control, &part);
    let _root = TestRoot::with_child(&control);
    if enable {
        assert_eq!(s.theme.plan_stats(), (2, 2, 4 + 6));
    }
    (
        control.foo(),
        control.z_index(),
        part.opacity(),
        part.z_index(),
        part.height(),
        control.values().frames().len(),
        part.values().frames().len(),
    )
}

#[test]
fn a_plan_applies_what_the_traversal_of_every_style_applies() {
    // Without index-enabled selectors every style is a candidate for every element.
    let reference = applied(false);
    let planned = applied(true);
    assert_eq!(planned, reference);
    assert_eq!((planned.0.as_str(), planned.1), ("a b", 9));
    assert_eq!((planned.2, planned.3, planned.4), (0.5, 3, 7.0));
}

#[test]
fn a_change_of_what_a_plan_is_made_from_makes_a_new_plan() {
    let s = scenario(true);
    let control = Class1::new();
    let part = Class3::new();
    assert_steps(&s.theme, &control, &[(&s.a, Attach), (&s.a_b, Attach), (&s.plain, Attach), (&s.d, Attach)]);

    // A replaced selector is a candidate for every element until it is enabled again.
    s.part.set_selector(Some(in_template::<Class3>().class("y")));
    assert_steps(
        &s.theme,
        &control,
        &[(&s.a, Attach), (&s.a_b, Attach), (&s.part, Attach), (&s.plain, Attach), (&s.d, Attach)],
    );
    s.part.enable_selector_index();
    assert_steps(&s.theme, &control, &[(&s.a, Attach), (&s.a_b, Attach), (&s.plain, Attach), (&s.d, Attach)]);

    // A style added below a nested style.
    let e = style(nest().class("e"), foo("e"));
    s.a.children().add(e.clone());
    assert_steps(
        &s.theme,
        &control,
        &[(&s.a, Attach), (&s.a_b, Attach), (&e, Attach), (&s.plain, Attach), (&s.d, Attach)],
    );
    let for_part =
        [(&s.a, Descend), (&s.a_part, Attach), (&s.part, Attach), (&s.plain, Attach), (&s.d, Descend), (&s.d_part, Attach)];
    assert_steps(&s.theme, &part, &for_part);

    // Replaced.
    let f = style(in_template::<Class3>(), opacity(0.25));
    s.a.children().set(2, f.clone());
    assert_steps(&s.theme, &control, &[(&s.a, Attach), (&s.a_b, Attach), (&s.plain, Attach), (&s.d, Attach)]);
    assert_steps(
        &s.theme,
        &part,
        &[
            (&s.a, Descend),
            (&s.a_part, Attach),
            (&f, Attach),
            (&s.part, Attach),
            (&s.plain, Attach),
            (&s.d, Descend),
            (&s.d_part, Attach),
        ],
    );

    // Removed.
    assert!(s.a.children().remove(f.clone()));
    assert_steps(&s.theme, &part, &for_part);

    // A style that cannot match and has nothing below it any more has no entry.
    s.d.children().clear();
    assert_steps(&s.theme, &part, &[(&s.a, Descend), (&s.a_part, Attach), (&s.part, Attach), (&s.plain, Attach)]);

    // A style that was added to another style since: its nesting selector stands for the
    // selector of that style, which the plan knows nothing of.
    let other = style(nest().class("z"), foo("z"));
    other.children().add(s.a_b.clone());
    assert_steps(&s.theme, &control, &[(&s.a, Attach), (&s.a_b, Whole), (&s.plain, Attach), (&s.d, Attach)]);

    // The nesting selector of the styles stands for the target type of the theme.
    s.theme.set_target_type(Some(Class3::TYPE));
    assert_steps(&s.theme, &control, &[(&s.a, Descend), (&s.a_b, Whole), (&s.plain, Attach)]);
    assert_steps(
        &s.theme,
        &part,
        &[(&s.a, Attach), (&s.a_b, Whole), (&s.a_part, Attach), (&s.part, Attach), (&s.plain, Attach), (&s.d, Attach)],
    );
}

#[test]
fn a_change_during_the_traversal_is_followed_as_the_traversal_of_every_style_follows_it() {
    let theme = ControlTheme::for_type::<Class1>();
    let first = style(nest().class("a"), foo("first"));
    let second = style(in_template::<Class3>(), opacity(0.5));
    let third = style(nest().class("a"), foo("third"));
    theme.children().add(first.clone());
    theme.children().add(second.clone());
    theme.children().add(third.clone());
    let target = Class1::new();
    target.classes().add("a");

    let candidates = theme.candidates(&target).expect("the theme has a plan for the element");
    assert_eq!(candidates.len(), 2);

    // What attaching the first style could do: give it a nested style, and make the style
    // after it, which the list left out, match the element.
    first.children().add(style(nest().class("a"), z_index(4)));
    second.set_selector(Some(nest().class("a")));
    target.apply_theme_candidates_in_pass(&candidates);

    assert_eq!(target.z_index(), 4);
    assert_eq!(target.opacity(), 0.5);
    assert_eq!(target.foo(), "third");
    // Each of the four styles once.
    assert_eq!(target.values().frames().len(), 4);
}

/// A style of another implementation, which runs code when it is applied.
struct Hook {
    run: Box<dyn Fn(&StyledElement) -> bool>,
    done: Cell<bool>,
}

impl Hook {
    /// `run` is called for each element the style is applied to until it returns `true`.
    fn new(run: impl Fn(&StyledElement) -> bool + 'static) -> Rc<dyn IStyle> {
        Rc::new(Hook { run: Box::new(run), done: Cell::new(false) })
    }
}

impl crate::controls::IResourceNode for Hook {
    fn has_resources(&self) -> bool {
        false
    }

    fn try_get_resource(
        &self,
        _: &crate::controls::ResourceKey,
        _: Option<&ThemeVariant>,
    ) -> Option<crate::controls::ResourceValue> {
        None
    }
}

impl IStyle for Hook {
    fn children(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        empty_styles()
    }

    fn children_for_target<'a>(&self, target: &'a StyledElement) -> StyleCandidates<'a> {
        if !self.done.get() && (self.run)(target) {
            self.done.set(true);
        }
        StyleCandidates::all(empty_styles())
    }
}

#[test]
fn a_change_made_by_a_nested_entry_reaches_the_styles_after_it_at_every_level() {
    let theme = ControlTheme::for_type::<Class1>();
    let outer = style(nest().class("a"), foo("outer"));
    let late = style(in_template::<Class3>(), z_index(4));
    let tail = style(in_template::<Class3>(), opacity(0.5));
    let hook = Hook::new({
        let (late, tail) = (late.clone(), tail.clone());
        move |_| {
            late.set_selector(Some(nest().class("a")));
            tail.set_selector(Some(nest().class("a")));
            true
        }
    });
    outer.children().add(hook.clone());
    outer.children().add(late.clone());
    theme.children().add(outer.clone());
    theme.children().add(tail.clone());

    let target = Class1::new();
    target.classes().add("a");
    let listed = steps(&theme, &target);
    assert_eq!(listed.len(), 2, "{}", describe(&listed));
    assert!(style_ptr_eq(&listed[1].0, &hook) && listed[1].1 == Whole);

    target.set_theme(theme.clone());
    let _root = TestRoot::with_child(&target);
    assert_eq!((target.foo(), target.z_index(), target.opacity()), ("outer".to_string(), 4, 0.5));
    assert_eq!(target.values().frames().len(), 3);

    // The next element has a plan with the styles as they are now.
    let next = Class1::new();
    next.classes().add("a");
    let listed = steps(&theme, &next);
    assert_eq!(listed.len(), 4, "{}", describe(&listed));
    next.set_theme(theme.clone());
    let _next_root = TestRoot::with_child(&next);
    assert_eq!((next.foo(), next.z_index(), next.opacity()), ("outer".to_string(), 4, 0.5));
    assert_eq!(next.values().frames().len(), 3);
}

crate::ferro_thread_local! {
    static KEY: Cell<Option<&'static TypeInfo>> = const { Cell::new(None) };
}

/// A control whose style key the test sets: the key of `TestPanel` until then.
#[repr(C)]
struct Keyed {
    base: Class1,
}

ferro_class!(Keyed: Class1);
ferro_impl_classes!(Keyed: FerroObjectImpl, VisualImpl, LayoutableImpl);

impl StyledElementImpl for Keyed {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        KEY.with(Cell::get).unwrap_or(TestPanel::TYPE)
    }
}

impl Keyed {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Class1::construct() })
    }
}

#[test]
fn a_style_key_that_changes_during_the_traversal_ends_the_use_of_the_plan() {
    let theme = ControlTheme::for_type::<Class1>();
    let tail = style(in_template::<Class3>(), opacity(0.5));
    // Applied to the part, the entry gives the part the key the style after it is for.
    theme.children().add(Hook::new(|target| {
        if target.style_key() != TestPanel::TYPE {
            return false;
        }
        KEY.with(|key| key.set(Some(Class3::TYPE)));
        true
    }));
    theme.children().add(tail.clone());

    let control = Class1::new();
    control.set_theme(theme.clone());
    let part = Keyed::new();
    assert_eq!(steps(&theme, &part).len(), 1);
    part.set_templated_parent(control.clone().upcast::<FerroObject>());
    set_child(&control, &part);
    let _root = TestRoot::with_child(&control);

    assert_eq!(part.style_key(), Class3::TYPE);
    assert_eq!(part.opacity(), 0.5);
}

#[test]
fn styles_nested_deeper_than_a_plan_follows_are_applied_by_the_traversal() {
    let theme = ControlTheme::for_type::<Class1>();
    let styles: Vec<Ref<Style>> = (0..20).map(|depth| style(nest().class("a"), foo(&depth.to_string()))).collect();
    for depth in (1..styles.len()).rev() {
        styles[depth - 1].children().add(styles[depth].clone());
    }
    theme.children().add(styles[0].clone());

    let target = Class1::new();
    target.classes().add("a");
    let before = builds();
    assert!(theme.candidates(&target).is_none());
    assert!(theme.candidates(&target).is_none());
    // The theme found out once.
    assert_eq!(builds() - before, 1);

    target.set_theme(theme.clone());
    let _root = TestRoot::with_child(&target);
    assert_eq!(target.foo(), "19");
    assert_eq!(target.values().frames().len(), 20);

    // A change far below the theme still reaches it.
    styles[10].children().clear();
    let listed = steps(&theme, &target);
    assert_eq!(listed.len(), 11, "{}", describe(&listed));
}

#[test]
fn keys_that_no_style_can_match_share_one_empty_plan() {
    let theme = ControlTheme::for_type::<Class1>();
    theme.children().add(style(nest().class("a"), foo("a")));
    theme.children().add(style(in_template::<Class3>(), opacity(0.5)));

    let panel = TestPanel::new();
    let root = TestRoot::new();
    assert_eq!(theme.candidates(&panel).expect("a plan").len(), 0);
    assert_eq!(theme.candidates(&root).expect("a plan").len(), 0);
    assert_eq!(theme.plan_stats(), (2, 1, 0));

    let control = Class1::new();
    assert_eq!(theme.candidates(&control).expect("a plan").len(), 1);
    assert_eq!(theme.plan_stats(), (3, 2, 1));
}

#[test]
fn a_container_query_is_applied_whole_with_the_styles_below_it() {
    let theme = ControlTheme::for_type::<Class1>();
    let query = ContainerQuery::new();
    query.children().add(style(Selectors::is::<Class1>(), foo("in a container")));
    theme.children().add(query.clone());
    theme.children().add(style(nest().class("a"), foo("a")));

    let part = Class3::new();
    let listed = steps(&theme, &part);
    assert_eq!(listed.len(), 1, "{}", describe(&listed));
    let query: Rc<dyn IStyle> = query.into();
    assert!(style_ptr_eq(&listed[0].0, &query) && listed[0].1 == Whole);
}
