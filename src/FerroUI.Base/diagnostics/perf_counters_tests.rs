//! Tests of the performance counters: each counting site counts what its
//! counter names. The module is compiled only with the feature
//! `perf-counters`; the counters are per thread, and each test runs on a
//! thread of its own, so a test reads only what it did itself.
//!
//! Not ports: the counters are an addition of the port.

use super::perf_counters::{self, PerfCounter, PerfCountersSnapshot};
use crate::media::text_formatting::testing::TextTestScope;
use crate::media::text_formatting::{TextLayout, TextLayoutOptions, TextRunCache};
use crate::media::Typeface;
use crate::reactive::IDisposable;
use crate::styling::test_support::{boxed, Class1, Class3};
use crate::styling::testing::try_attach;
use crate::styling::{Selectors, Setter, Style};
use crate::{BoxedValue, StyledElement};
use std::rc::Rc;

/// What `work` counted.
fn counted(work: impl FnOnce()) -> PerfCountersSnapshot {
    let before = perf_counters::snapshot();
    work();
    perf_counters::snapshot().since(&before)
}

#[test]
fn the_counters_are_compiled_in() {
    assert!(perf_counters::ENABLED);
    assert!(std::mem::size_of::<PerfCountersSnapshot>() > 0);
}

#[test]
fn a_property_change_is_counted_with_and_without_a_listener_of_the_object() {
    let target = Class1::new();

    let without = counted(|| target.set_foo("first"));
    assert_eq!(1, without.get(PerfCounter::PropertyChangesRaised));
    assert_eq!(1, without.get(PerfCounter::PropertyChangesEffective));
    assert_eq!(0, without.get(PerfCounter::PropertyChangesWithObjectListeners));

    let subscription = target.property_changed(|_| {});
    let with = counted(|| target.set_foo("second"));
    assert_eq!(1, with.get(PerfCounter::PropertyChangesRaised));
    assert_eq!(1, with.get(PerfCounter::PropertyChangesEffective));
    assert_eq!(1, with.get(PerfCounter::PropertyChangesWithObjectListeners));

    // Setting the value it already has raises nothing.
    let unchanged = counted(|| target.set_foo("second"));
    assert_eq!(0, unchanged.get(PerfCounter::PropertyChangesRaised));
    subscription.dispose();
}

#[test]
fn a_property_change_is_counted_with_a_handler_of_the_property() {
    let target = Class1::new();
    let handlers_before = counted(|| target.set_value(Class1::double_property(), 1.0));
    let before = handlers_before.get(PerfCounter::PropertyChangesWithPropertyHandlers);

    let subscription = Class1::double_property().changed().subscribe(|_| {});
    let with = counted(|| target.set_value(Class1::double_property(), 2.0));
    subscription.dispose();

    // The property had no handler before the subscription and has one with it.
    assert_eq!(0, before);
    assert_eq!(1, with.get(PerfCounter::PropertyChangesWithPropertyHandlers));
}

#[test]
fn the_virtual_calls_of_a_property_change_are_counted_by_member() {
    let target = Class1::new();
    let change = counted(|| target.set_foo("value"));

    let calls = change.virtual_calls();
    let of = |member: &str| calls.iter().find(|(name, _)| *name == member).map_or(0, |(_, count)| *count);
    assert_eq!(1, of("FerroObject::on_property_changed_core"));
    assert!(change.virtual_calls_total() >= 1);
    // The most called member comes first.
    assert!(calls.windows(2).all(|pair| pair[0].1 >= pair[1].1));

    // Two stretches of work add up, counter by counter and member by member.
    let both = change.plus(&counted(|| target.set_foo("other")));
    assert_eq!(2, both.get(PerfCounter::PropertyChangesRaised));
    let calls = both.virtual_calls();
    let of = |member: &str| calls.iter().find(|(name, _)| *name == member).map_or(0, |(_, count)| *count);
    assert_eq!(2, of("FerroObject::on_property_changed_core"));
}

#[test]
fn an_inheritance_ancestor_change_is_counted_with_the_values_it_compares() {
    let parent = StyledElement::new();
    let child = StyledElement::new();
    parent.set_value(StyledElement::data_context_property(), Some(Rc::new("context".to_string()) as BoxedValue));

    let attached = counted(|| child.set_inheritance_parent(&parent));
    assert_eq!(1, attached.get(PerfCounter::InheritanceAncestorChanges));
    assert!(attached.get(PerfCounter::InheritedValuesCompared) >= 1);
    assert!(attached.get(PerfCounter::InheritedValuesDiffering) >= 1);
    assert!(attached.get(PerfCounter::InheritedValueWalkVisits) >= 1);
    assert!(attached.get(PerfCounter::InheritedValuesDiffering) <= attached.get(PerfCounter::InheritedValuesCompared));
}

#[test]
fn a_style_is_counted_when_it_is_evaluated_and_when_it_matches() {
    let setter = || Setter::new(Class1::foo_property(), "Foo".to_string());
    let target = Class1::new();

    let matching = Style::with_setters(Selectors::of_type::<Class1>(), [setter()]);
    let matched = counted(|| {
        try_attach(&matching, &target, None);
    });
    assert_eq!(1, matched.get(PerfCounter::StylesEvaluated));
    assert_eq!(1, matched.get(PerfCounter::StylesMatched));
    assert_eq!(1, matched.get(PerfCounter::StyleInstancesAttached));
    assert_eq!(1, matched.get(PerfCounter::StyleInstancesCreated));
    assert_eq!("Foo", target.foo());

    let other = Style::with_setters(Selectors::of_type::<Class3>(), [setter()]);
    let unmatched = counted(|| {
        try_attach(&other, &target, None);
    });
    assert_eq!(1, unmatched.get(PerfCounter::StylesEvaluated));
    assert_eq!(0, unmatched.get(PerfCounter::StylesMatched));
    assert_eq!(0, unmatched.get(PerfCounter::StyleInstancesAttached));
}

#[test]
fn a_resource_lookup_is_counted_with_the_hosts_it_asks() {
    let target = Class1::new();
    target.resources().add("key", Some(boxed("value".to_string())));

    let found = counted(|| {
        assert!(target.try_find_resource(&"key".into(), None).is_some());
    });
    assert_eq!(1, found.get(PerfCounter::ResourceLookups));
    // The element itself holds the resource: no other host is asked.
    assert_eq!(1, found.get(PerfCounter::ResourceHostsProbed));

    let missing = counted(|| {
        assert!(target.try_find_resource(&"other".into(), None).is_none());
    });
    assert_eq!(1, missing.get(PerfCounter::ResourceLookups));
    assert!(missing.get(PerfCounter::ResourceHostsProbed) >= 1);
}

#[test]
fn a_text_layout_is_counted_and_the_second_one_of_a_cache_is_a_hit() {
    let _scope = TextTestScope::new();
    let cache = Rc::new(TextRunCache::new());
    let options = || TextLayoutOptions { font_size: 12.0, text_run_cache: Some(cache.clone()), ..Default::default() };

    let first = counted(|| {
        let _ = TextLayout::new("0123456789", Typeface::default_typeface(), options());
    });
    assert_eq!(1, first.get(PerfCounter::TextLayoutsCreated));
    assert_eq!(1, first.get(PerfCounter::TextRunCacheMisses));
    assert_eq!(0, first.get(PerfCounter::TextRunCacheHits));
    assert!(first.get(PerfCounter::TextLinesFormatted) >= 1);
    assert!(first.get(PerfCounter::TextRunsShaped) >= 1);

    let second = counted(|| {
        let _ = TextLayout::new("0123456789", Typeface::default_typeface(), options());
    });
    assert_eq!(1, second.get(PerfCounter::TextLayoutsCreated));
    assert_eq!(0, second.get(PerfCounter::TextRunCacheMisses));
    assert_eq!(1, second.get(PerfCounter::TextRunCacheHits));
    // The line of the second layout is built from the cached runs.
    assert_eq!(0, second.get(PerfCounter::TextRunsShaped));
    cache.dispose();
}

#[test]
fn the_counters_reset_and_the_report_names_each_of_them() {
    let target = Class1::new();
    target.set_foo("value");
    assert!(perf_counters::snapshot().get(PerfCounter::PropertyChangesRaised) >= 1);

    let report = perf_counters::snapshot().report_per(2, "row");
    for counter in PerfCounter::ALL {
        assert!(report.contains(counter.name()), "{} is not in the report", counter.name());
    }
    assert!(report.contains("per row"));
    assert!(report.contains("FerroObject::on_property_changed_core"));

    perf_counters::reset();
    let after = perf_counters::snapshot();
    for counter in PerfCounter::ALL {
        assert_eq!(0, after.get(*counter), "{}", counter.name());
    }
    assert!(after.virtual_calls().is_empty());
}
