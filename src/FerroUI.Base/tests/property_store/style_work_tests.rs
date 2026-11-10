//! Deterministic probes for empty styling batches and inherited theme work.
//! The ignored probe times only property-store work, not window startup.

use super::super::*;
use crate::data::BindingPriority;
use crate::diagnostics::perf_counters::{self, PerfCounter as C};
use crate::property_store::{ImmediateValueFrame, ValueFrame};
use crate::styling::ThemeVariant;
use crate::styling::test_support::{Class1 as StyledProbe, TestRoot};
use crate::styling::{Selectors, Setter, Style};
use crate::*;
use std::hint::black_box;
use std::rc::Rc;
use std::time::Instant;

test_class!(Probe: FerroObject);

impl Probe {
    ferro_property!(pub fn number_property() -> StyledProperty<i32> {
        FerroProperty::register::<Probe, _>("Number", 0)
    });

    ferro_property!(pub fn inherited_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Probe, _>("Inherited", StyledPropertyOptions::new(0).inherits(true))
    });
}

// Drop the root before its children so teardown does not measure repeated
// removals from the root's inheritance-children vector on main.
struct Wall {
    root: Ref<Probe>,
    children: Vec<Ref<Probe>>,
}

impl Wall {
    fn new(count: usize) -> Self {
        let root = Probe::new();
        let children = (0..count).map(|_| {
            let child = Probe::new();
            child.set_inheritance_parent(&root);
            child
        }).collect();
        Self { root, children }
    }
}

fn assert_count(counted: &perf_counters::PerfCountersSnapshot, counter: C, expected: u64) {
    if perf_counters::ENABLED {
        assert_eq!(counted.get(counter), expected, "{}", counter.name());
    }
}

#[test]
fn equal_theme_write_and_clean_batch_do_not_notify_descendants() {
    let wall = Wall::new(16);
    let property = ThemeVariant::actual_theme_variant_property();
    wall.root.set_value_with_priority(property, Some(ThemeVariant::light()), BindingPriority::Template);
    let before = perf_counters::snapshot();

    wall.root.set_value_with_priority(property, Some(ThemeVariant::light()), BindingPriority::Template);
    wall.root.values().begin_styling();
    wall.root.values().end_styling(&wall.root);

    let counted = perf_counters::snapshot().since(&before);
    assert_count(&counted, C::PropertyChangesEffective, 0);
    assert_count(&counted, C::InheritedEffectiveValueWalkVisits, 0);
    assert_count(&counted, C::InheritanceAncestorWalkVisits, 0);
    for child in &wall.children {
        assert_eq!(child.get_value(property), Some(ThemeVariant::light()));
    }
}

#[test]
fn first_inherited_holder_with_default_value_updates_ancestors_without_notifications() {
    let wall = Wall::new(16);
    let before = perf_counters::snapshot();

    wall.root.set_value(Probe::inherited_property(), 0);

    let counted = perf_counters::snapshot().since(&before);
    assert_count(&counted, C::InheritanceAncestorWalkVisits, 17);
    assert_count(&counted, C::InheritedEffectiveValueWalkVisits, 0);
    assert_count(&counted, C::PropertyChangesEffective, 0);
    for child in &wall.children {
        assert!(child.values().inheritance_ancestor().unwrap().ptr_eq(&wall.root));
    }
}

#[test]
fn actual_theme_change_stops_at_local_override() {
    let wall = Wall::new(2);
    let grandchild = Probe::new();
    grandchild.set_inheritance_parent(&wall.children[0]);
    let property = ThemeVariant::actual_theme_variant_property();
    wall.root.set_value(property, Some(ThemeVariant::light()));
    wall.children[0].set_value(property, Some(ThemeVariant::light()));
    let before = perf_counters::snapshot();

    wall.root.set_value(property, Some(ThemeVariant::dark()));

    let counted = perf_counters::snapshot().since(&before);
    assert_count(&counted, C::InheritedEffectiveValueWalkVisits, 2);
    assert_count(&counted, C::PropertyChangesEffective, 2); // root and unblocked child
    assert_eq!(grandchild.get_value(property), Some(ThemeVariant::light()));
    assert_eq!(wall.children[1].get_value(property), Some(ThemeVariant::dark()));
}

#[test]
fn nested_styling_flushes_new_frame_only_at_outer_end() {
    let target = Probe::new();
    let frame = ImmediateValueFrame::new(BindingPriority::Style);
    frame.add_value(Probe::number_property(), 42);
    let frame: Rc<dyn ValueFrame> = frame;

    target.values().begin_styling();
    target.values().begin_styling();
    target.values().add_frame(&target, frame);
    assert_eq!(target.get_value(Probe::number_property()), 0);
    target.values().end_styling(&target);
    assert_eq!(target.get_value(Probe::number_property()), 0);
    target.values().end_styling(&target);
    assert_eq!(target.get_value(Probe::number_property()), 42);
}

#[test]
fn frame_removal_during_styling_restores_default() {
    let target = Probe::new();
    let frame = ImmediateValueFrame::new(BindingPriority::Style);
    frame.add_value(Probe::number_property(), 42);
    let frame: Rc<dyn ValueFrame> = frame;
    target.values().add_frame(&target, frame.clone());

    target.values().begin_styling();
    target.values().remove_frame(&target, &frame);
    assert_eq!(target.get_value(Probe::number_property()), 42);
    target.values().end_styling(&target);
    assert_eq!(target.get_value(Probe::number_property()), 0);
}

#[test]
fn binding_update_during_styling_is_flushed() {
    let target = Probe::new();
    let source = Subject::behavior(1);
    let _binding = target.bind(Probe::number_property(), source.observable(), BindingPriority::Style);
    assert_eq!(target.get_value(Probe::number_property()), 1);

    target.values().begin_styling();
    source.on_next(2);
    assert_eq!(target.get_value(Probe::number_property()), 1);
    target.values().end_styling(&target);
    assert_eq!(target.get_value(Probe::number_property()), 2);
}

#[test]
fn clean_batch_does_not_replay_existing_style_values() {
    let target = Probe::new();
    target.set_value_with_priority(Probe::number_property(), 42, BindingPriority::Style);
    let generation = target.values().value_generation();
    let before = perf_counters::snapshot();

    target.values().begin_styling();
    target.values().end_styling(&target);

    let counted = perf_counters::snapshot().since(&before);
    assert_count(&counted, C::StylingPassesCompleted, 1);
    assert_count(&counted, C::FullValueReevaluations, 0);
    assert_count(&counted, C::ReevaluatedEffectiveValues, 0);
    assert_eq!(target.values().value_generation(), generation);
    assert_eq!(target.get_value(Probe::number_property()), 42);
}

#[test]
fn style_activation_during_styling_is_flushed() {
    let target = StyledProbe::new();
    let style = Style::with_setters(
        Selectors::of_type::<StyledProbe>().class("active"),
        [Setter::new(StyledProbe::double_property(), 42.0)],
    );
    crate::styling::testing::try_attach(&style, &target, None);

    for (active, expected) in [(true, 42.0), (false, 0.0)] {
        target.values().begin_styling();
        target.classes().set("active", active);
        target.values().end_styling(&target);
        assert_eq!(target.get_value(StyledProbe::double_property()), expected);
    }
}

#[test]
#[ignore = "manual style-work measurement; run release, one test thread, with --nocapture"]
fn style_work_probe() {
    let count: usize = std::env::var("FERROUI_STYLE_PROBE_ELEMENTS")
        .ok().map(|v| v.parse().unwrap()).unwrap_or(10_000);
    let wall = Wall::new(count);
    for child in &wall.children {
        child.set_value(Probe::number_property(), 42);
    }
    let theme = ThemeVariant::actual_theme_variant_property();
    wall.root.set_value_with_priority(theme, Some(ThemeVariant::light()), BindingPriority::Template);
    for case in ["clean_batches", "equal_theme", "changed_theme", "root_clean_batch"] {
        let before = perf_counters::snapshot();
        let start = Instant::now();
        match case {
            "clean_batches" => {
                for child in &wall.children {
                    child.values().begin_styling();
                    child.values().end_styling(child);
                }
            }
            "equal_theme" => {
                wall.root.set_value_with_priority(theme, Some(ThemeVariant::light()), BindingPriority::Template);
            }
            "changed_theme" => {
                wall.root.set_value_with_priority(theme, Some(ThemeVariant::dark()), BindingPriority::Template);
            }
            "root_clean_batch" => {
                wall.root.values().begin_styling();
                wall.root.values().end_styling(&wall.root);
            }
            _ => unreachable!(),
        }
        let elapsed = start.elapsed();
        let counted = perf_counters::snapshot().since(&before);
        println!("{case}: elements={count}, elapsed_us={:.3}, full={}, values={}, entries={}, ancestor_visits={}, inherited_visits={}, effective_changes={}",
            elapsed.as_secs_f64() * 1e6,
            counted.get(C::FullValueReevaluations), counted.get(C::ReevaluatedEffectiveValues),
            counted.get(C::ReevaluatedFrameEntries), counted.get(C::InheritanceAncestorWalkVisits),
            counted.get(C::InheritedEffectiveValueWalkVisits), counted.get(C::PropertyChangesEffective));
        black_box(&wall);
    }

    // Exercise the real styling path as well as the isolated store operations.
    // An inheritance parent is also the base styling parent; logical/visual
    // attachment and layout are deliberately outside this probe's scope.
    for matched in [false, true] {
        let root = TestRoot::new();
        let selector = if matched {
            Selectors::of_type::<StyledProbe>()
        } else {
            Selectors::of_type::<TestRoot>()
        };
        root.styles().add(&Style::with_setters(selector, [Setter::new(StyledProbe::double_property(), 5.0)]));
        let elements: Vec<_> = (0..count).map(|_| {
            let element = StyledProbe::new();
            element.set_value(StyledProbe::foo_property(), "local".to_owned());
            element.set_inheritance_parent(&root);
            element
        }).collect();
        let before = perf_counters::snapshot();
        let start = Instant::now();
        for element in &elements {
            assert!(element.try_apply_styling().unwrap());
        }
        let elapsed = start.elapsed();
        let counted = perf_counters::snapshot().since(&before);
        println!("apply_styling_{}: elements={count}, elapsed_us={:.3}, full={}, values={}, entries={}, evaluated={}, matched={}, effective_changes={}",
            if matched { "match" } else { "no_match" }, elapsed.as_secs_f64() * 1e6,
            counted.get(C::FullValueReevaluations), counted.get(C::ReevaluatedEffectiveValues),
            counted.get(C::ReevaluatedFrameEntries), counted.get(C::StylesEvaluated),
            counted.get(C::StylesMatched), counted.get(C::PropertyChangesEffective));
        assert_eq!(elements.last().unwrap().get_value(StyledProbe::double_property()), if matched { 5.0 } else { 0.0 });
        // Release the parent first to keep teardown linear on main too.
        drop(root);
    }
}
