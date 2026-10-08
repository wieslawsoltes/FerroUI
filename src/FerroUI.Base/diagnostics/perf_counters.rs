//! Counters of the work the framework does per operation: how many property
//! changes are raised and how many of them anybody listens to, how many
//! styles are evaluated and how many match, how many resources are looked
//! up, how many bindings and text layouts are created. The performance
//! designs (`docs/porting/performance/designs`) each start by reading them.
//!
//! An addition of the port (DEVIATIONS.md, Diagnostics): upstream has no
//! counterpart. The counters exist only with the feature `perf-counters`.
//! Without it [`perf_count!`](crate::perf_count) and
//! [`perf_count_virtual!`](crate::perf_count_virtual) expand to no code, the
//! functions of this module have empty bodies and
//! [`PerfCountersSnapshot`] has no size; no type of the framework gains a
//! field either way.
//!
//! The counters are per thread (the thread of the dispatcher is the one that
//! is measured), and counting changes no behaviour. Read them around the
//! measured work:
//!
//! ```ignore
//! let before = perf_counters::snapshot();
//! // ... the work ...
//! let counted = perf_counters::snapshot().since(&before);
//! println!("{}", counted.report_per(rows, "row"));
//! ```

use std::fmt;

/// Whether the counters are compiled in (the feature `perf-counters`).
pub const ENABLED: bool = cfg!(feature = "perf-counters");

macro_rules! perf_counters {
    ($($(#[$meta:meta])* $name:ident => $label:literal,)*) => {
        /// What is counted. Each counter names, in its documentation, the
        /// function that counts it.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(usize)]
        pub enum PerfCounter {
            $($(#[$meta])* $name,)*
        }

        impl PerfCounter {
            /// Every counter, in the order of the report.
            pub const ALL: &'static [PerfCounter] = &[$(PerfCounter::$name,)*];

            /// The name of the counter in the report.
            pub const fn name(self) -> &'static str {
                match self {
                    $(PerfCounter::$name => $label,)*
                }
            }
        }
    };
}

perf_counters! {
    // --- property changes (design 02) ---
    /// `FerroObject::raise_property_changed`: every notification built and
    /// passed to the `on_property_changed` chain of the class.
    PropertyChangesRaised => "property changes raised",
    /// `FerroObject::raise_property_changed`: the ones of an effective
    /// value, which go on to the handlers of the property and the listeners
    /// of the object.
    PropertyChangesEffective => "property changes of an effective value",
    /// `PropertyChangedObservable::notify`: the property has at least one
    /// handler (the class handlers and the subscriptions to the property).
    PropertyChangesWithPropertyHandlers => "property changes with handlers of the property",
    /// `FerroObject::raise_property_changed`: the object has at least one
    /// listener of its property changes.
    PropertyChangesWithObjectListeners => "property changes with listeners of the object",

    // --- inheritance (design 03) ---
    /// `ValueStore::set_inheritance_parent`: the inheritance ancestor of an
    /// object changed.
    InheritanceAncestorChanges => "inheritance ancestor changes",
    /// `ValueStore::set_inheritance_parent`: the inherited properties of the
    /// old and the new ancestor chain that were paired.
    InheritedValuesCompared => "inherited values compared on an ancestor change",
    /// `ValueStore::set_inheritance_parent`: the ones whose old and new
    /// value differ, each of which starts a walk of the subtree.
    InheritedValuesDiffering => "inherited values differing on an ancestor change",
    /// `ValueStore::inherited_value_changed`: the objects such a walk
    /// reaches (the object itself and its inheritance descendants, once per
    /// property).
    InheritedValueWalkVisits => "objects visited by inherited value walks",

    // --- tree attachment, styles and resources (design 04) ---
    /// `StyledElement::on_attached_to_logical_tree_core`: an element that
    /// was not in a logical tree enters one.
    LogicalTreeAttachments => "elements attached to a logical tree",
    /// `StyledElement::on_detached_from_logical_tree_core`: an element
    /// leaves its logical tree.
    LogicalTreeDetachments => "elements detached from a logical tree",
    /// `StyledElement::get_effective_theme`: the implicit theme was not
    /// known and was looked up as a resource.
    ImplicitThemeLookups => "implicit theme lookups",
    /// `StyledElement::apply_styles`: a style host between the element and
    /// the root was asked for its styles.
    StyleHostsWalked => "style hosts walked",
    /// `Style::try_attach_checked` and `ContainerQuery::try_attach_checked`:
    /// a style with setters (a query with children) was matched against an
    /// element.
    StylesEvaluated => "styles evaluated",
    /// The same functions: the selector (the query) matched and the style
    /// was instanced on the element.
    StylesMatched => "styles matched",
    /// `ControlTheme::try_attach_checked`: a control theme was tested
    /// against the style key of an element.
    ControlThemesEvaluated => "control themes evaluated",
    /// `ControlTheme::try_attach_checked`: it applies and was instanced.
    ControlThemesMatched => "control themes matched",
    /// `StyleBase::try_attach_instance`: a style instance was added to the
    /// value store of an element.
    StyleInstancesAttached => "style instances attached",
    /// `StyleBase::try_attach_instance`: the instance was built (its setters
    /// instanced) instead of the shared instance of the style being taken.
    StyleInstancesCreated => "style instances created",
    /// `IResourceHost::try_find_resource`: a lookup up the logical tree.
    ResourceLookups => "resource lookups",
    /// `IResourceHost::try_find_resource`: the hosts such a lookup asked
    /// (`try_get_resource` of the host and of each styling parent).
    ResourceHostsProbed => "resource hosts probed",

    // --- bindings (design 05) ---
    /// `FerroObject::bind_binding_with_anchor`: a binding was instanced on
    /// a property of an object.
    BindingsInstanced => "bindings instanced",
    /// `UntypedBindingExpressionBase::new` and `TypedBindingExpression::new`:
    /// a binding expression was created (by a binding, a setter, a template
    /// binding or a dynamic resource).
    BindingExpressionsCreated => "binding expressions created",
    /// `TemplateBindingExpression::new`.
    TemplateBindingExpressionsCreated => "template binding expressions created",
    /// `DynamicResourceExpression::new` (the XAML runtime library).
    DynamicResourceExpressionsCreated => "dynamic resource expressions created",
    /// `UntypedBindingExpressionBase::publish_value` of a running
    /// expression and `TypedBindingExpression::publish_value`.
    BindingValuesPublished => "binding values published",

    // --- text (design 06) ---
    /// `TextLayout::from_text_source`.
    TextLayoutsCreated => "text layouts created",
    /// `TextFormatterImpl::format_line_with_cache`: a line was asked for.
    TextLinesFormatted => "text lines formatted",
    /// `TextFormatterImpl::format_line_with_cache`: the shaped runs of the
    /// line came from the text run cache.
    TextRunCacheHits => "text run cache hits",
    /// `TextFormatterImpl::format_line_with_cache`: a cache was given, did
    /// not hold the line, and the shaped runs were added to it.
    TextRunCacheMisses => "text run cache misses",
    /// `TextShaper::shape_text`: a run was shaped by the platform shaper.
    TextRunsShaped => "text runs shaped",

    // --- containers (the controls crate) ---
    /// `VirtualizingStackPanel::recycle_element`: a container left the
    /// viewport, was cleared and went to the recycle pool.
    ContainersRecycled => "containers recycled",
    /// `VirtualizingStackPanel::get_recycled_element`: a container of the
    /// pool was prepared for another item.
    ContainersReused => "containers reused",
    /// `VirtualizingStackPanel::create_element`: a container was created.
    ContainersCreated => "containers created",
    /// `ContentPresenter::update_child_with`: the child of a content
    /// presenter was replaced by another control or removed.
    ContentPresenterChildrenReplaced => "content presenter children replaced",
}

/// The number of counters.
#[cfg(feature = "perf-counters")]
const COUNT: usize = PerfCounter::ALL.len();

/// Counts one occurrence of `$counter` (a variant of
/// [`PerfCounter`](crate::diagnostics::perf_counters::PerfCounter)) on this
/// thread.
#[cfg(feature = "perf-counters")]
#[macro_export]
macro_rules! perf_count {
    ($counter:ident) => {
        $crate::diagnostics::perf_counters::count($crate::diagnostics::perf_counters::PerfCounter::$counter)
    };
}

/// Counts one occurrence of `$counter` (a variant of
/// [`PerfCounter`](crate::diagnostics::perf_counters::PerfCounter)) on this
/// thread. Without the feature `perf-counters` it only names the counter,
/// so that a misspelt one does not compile: there is no code.
#[cfg(not(feature = "perf-counters"))]
#[macro_export]
macro_rules! perf_count {
    ($counter:ident) => {{
        let _ = $crate::diagnostics::perf_counters::PerfCounter::$counter;
    }};
}

/// Counts one virtual call of the member `$member` (a string literal,
/// `"Class::member"`) on this thread.
#[cfg(feature = "perf-counters")]
#[macro_export]
macro_rules! perf_count_virtual {
    ($member:expr) => {
        $crate::diagnostics::perf_counters::count_virtual_call($member)
    };
}

/// Counts one virtual call of the member `$member` (a string literal,
/// `"Class::member"`) on this thread. Without the feature `perf-counters`
/// it expands to the unit value: there is no code.
#[cfg(not(feature = "perf-counters"))]
#[macro_export]
macro_rules! perf_count_virtual {
    ($member:expr) => {
        ()
    };
}

#[cfg(feature = "perf-counters")]
mod counting {
    use super::{PerfCounter, COUNT};
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    thread_local! {
        static COUNTS: [Cell<u64>; COUNT] = const { [const { Cell::new(0) }; COUNT] };
        static VIRTUAL_CALLS: RefCell<HashMap<&'static str, u64>> = RefCell::new(HashMap::new());
    }

    #[inline]
    pub(super) fn count(counter: PerfCounter) {
        // A thread that is ending has no counters left: nothing is counted.
        let _ = COUNTS.try_with(|counts| {
            let cell = &counts[counter as usize];
            cell.set(cell.get() + 1);
        });
    }

    pub(super) fn count_virtual_call(member: &'static str) {
        let _ = VIRTUAL_CALLS.try_with(|calls| {
            // The table is never borrowed across a call of the framework;
            // a call made while it is read would not be counted.
            if let Ok(mut calls) = calls.try_borrow_mut() {
                *calls.entry(member).or_insert(0) += 1;
            }
        });
    }

    pub(super) fn counts() -> [u64; COUNT] {
        COUNTS.with(|counts| std::array::from_fn(|index| counts[index].get()))
    }

    pub(super) fn virtual_calls() -> Vec<(&'static str, u64)> {
        let mut calls: Vec<(&'static str, u64)> =
            VIRTUAL_CALLS.with(|calls| calls.borrow().iter().map(|(member, count)| (*member, *count)).collect());
        super::sort_virtual_calls(&mut calls);
        calls
    }

    pub(super) fn reset() {
        COUNTS.with(|counts| {
            for cell in counts.iter() {
                cell.set(0);
            }
        });
        VIRTUAL_CALLS.with(|calls| calls.borrow_mut().clear());
    }
}

/// The most called member first; members with the same count by name.
#[cfg(feature = "perf-counters")]
fn sort_virtual_calls(calls: &mut [(&'static str, u64)]) {
    calls.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
}

/// Counts one occurrence of `counter` on this thread. Call it through
/// [`perf_count!`](crate::perf_count).
#[cfg(feature = "perf-counters")]
#[inline]
pub fn count(counter: PerfCounter) {
    counting::count(counter)
}

/// Counts one occurrence of `counter` on this thread: nothing, since the
/// feature `perf-counters` is off.
#[cfg(not(feature = "perf-counters"))]
#[inline(always)]
pub fn count(_counter: PerfCounter) {}

/// Counts one virtual call of `member` (`"Class::member"`) on this thread.
/// Call it through [`perf_count_virtual!`](crate::perf_count_virtual).
#[cfg(feature = "perf-counters")]
#[inline]
pub fn count_virtual_call(member: &'static str) {
    counting::count_virtual_call(member)
}

/// Counts one virtual call of `member` on this thread: nothing, since the
/// feature `perf-counters` is off.
#[cfg(not(feature = "perf-counters"))]
#[inline(always)]
pub fn count_virtual_call(_member: &'static str) {}

/// The counters of this thread at one moment, or the difference of two such
/// moments ([`since`](Self::since)). Without the feature `perf-counters` it
/// holds nothing and every count reads zero.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PerfCountersSnapshot {
    #[cfg(feature = "perf-counters")]
    counts: Option<[u64; COUNT]>,
    #[cfg(feature = "perf-counters")]
    virtual_calls: Vec<(&'static str, u64)>,
}

// Without the feature the snapshot is nothing at all.
#[cfg(not(feature = "perf-counters"))]
const _: () = assert!(std::mem::size_of::<PerfCountersSnapshot>() == 0);

/// The counters of this thread now.
pub fn snapshot() -> PerfCountersSnapshot {
    #[cfg(feature = "perf-counters")]
    {
        PerfCountersSnapshot { counts: Some(counting::counts()), virtual_calls: counting::virtual_calls() }
    }
    #[cfg(not(feature = "perf-counters"))]
    {
        PerfCountersSnapshot {}
    }
}

/// Sets every counter of this thread to zero.
pub fn reset() {
    #[cfg(feature = "perf-counters")]
    counting::reset();
}

impl PerfCountersSnapshot {
    /// The count of `counter`.
    pub fn get(&self, counter: PerfCounter) -> u64 {
        #[cfg(feature = "perf-counters")]
        {
            self.counts.map_or(0, |counts| counts[counter as usize])
        }
        #[cfg(not(feature = "perf-counters"))]
        {
            let _ = counter;
            0
        }
    }

    /// The virtual calls by member (`"Class::member"`, the class that
    /// declares the member), the most called first.
    pub fn virtual_calls(&self) -> &[(&'static str, u64)] {
        #[cfg(feature = "perf-counters")]
        {
            &self.virtual_calls
        }
        #[cfg(not(feature = "perf-counters"))]
        {
            &[]
        }
    }

    /// The number of virtual calls, of every member.
    pub fn virtual_calls_total(&self) -> u64 {
        self.virtual_calls().iter().map(|(_, count)| *count).sum()
    }

    /// What was counted between `earlier` and this snapshot.
    pub fn since(&self, earlier: &PerfCountersSnapshot) -> PerfCountersSnapshot {
        #[cfg(feature = "perf-counters")]
        {
            let counts: [u64; COUNT] = std::array::from_fn(|index| {
                let counter = PerfCounter::ALL[index];
                self.get(counter).saturating_sub(earlier.get(counter))
            });
            let mut virtual_calls: Vec<(&'static str, u64)> = self
                .virtual_calls
                .iter()
                .map(|(member, count)| {
                    let before =
                        earlier.virtual_calls.iter().find(|(other, _)| other == member).map_or(0, |(_, count)| *count);
                    (*member, count.saturating_sub(before))
                })
                .filter(|(_, count)| *count > 0)
                .collect();
            sort_virtual_calls(&mut virtual_calls);
            PerfCountersSnapshot { counts: Some(counts), virtual_calls }
        }
        #[cfg(not(feature = "perf-counters"))]
        {
            let _ = earlier;
            PerfCountersSnapshot {}
        }
    }

    /// These counts and the ones of `other` together: the sum of what was
    /// counted over several stretches of work.
    pub fn plus(&self, other: &PerfCountersSnapshot) -> PerfCountersSnapshot {
        #[cfg(feature = "perf-counters")]
        {
            let counts: [u64; COUNT] = std::array::from_fn(|index| {
                let counter = PerfCounter::ALL[index];
                self.get(counter) + other.get(counter)
            });
            let mut virtual_calls = self.virtual_calls.clone();
            for (member, count) in &other.virtual_calls {
                match virtual_calls.iter_mut().find(|(existing, _)| *existing == *member) {
                    Some((_, total)) => *total += count,
                    None => virtual_calls.push((*member, *count)),
                }
            }
            sort_virtual_calls(&mut virtual_calls);
            PerfCountersSnapshot { counts: Some(counts), virtual_calls }
        }
        #[cfg(not(feature = "perf-counters"))]
        {
            let _ = other;
            PerfCountersSnapshot {}
        }
    }

    /// The counters, one a line; then the virtual calls by member.
    pub fn report(&self) -> String {
        self.format(None)
    }

    /// The counters with, next to each, the count divided by `divisor` (the
    /// rows recycled, the frames rendered): `unit` names what it counts.
    pub fn report_per(&self, divisor: u64, unit: &str) -> String {
        self.format(Some((divisor, unit)))
    }

    fn format(&self, per: Option<(u64, &str)>) -> String {
        if !ENABLED {
            return String::from("perf counters: not compiled in (the feature `perf-counters` is off)\n");
        }

        let line = |name: &str, count: u64| match per {
            Some((divisor, unit)) if divisor > 0 => {
                format!("  {name}: {count} ({:.2} per {unit})\n", count as f64 / divisor as f64)
            }
            _ => format!("  {name}: {count}\n"),
        };
        let mut text = String::new();
        for counter in PerfCounter::ALL {
            text.push_str(&line(counter.name(), self.get(*counter)));
        }
        text.push_str(&line("virtual calls", self.virtual_calls_total()));
        for (member, count) in self.virtual_calls() {
            text.push_str(&line(&format!("  {member}"), *count));
        }
        text
    }
}

impl fmt::Display for PerfCountersSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.report())
    }
}

// Without the feature the counting macros are constant expressions of the
// unit type: there is nothing to run.
#[cfg(not(feature = "perf-counters"))]
const _: () = {
    crate::perf_count!(PropertyChangesRaised);
    crate::perf_count_virtual!("FerroObject::on_property_changed")
};
