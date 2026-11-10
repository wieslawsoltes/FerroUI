//! The layout subsystem, one mechanism at a time: the measure and arrange of each panel, the
//! depth of a tree, the cost of an invalidation and of the pass that answers it, the queues of the
//! layout manager, a change of the size of the root, visibility, the content of a scroll viewer,
//! the attached properties that place a child in its panel, and the events a pass raises.
//! FerroUI's own regression benchmarks, run by the local performance gate
//! (`scripts/perf/gate.py`, suite `framework`): each guards a finding of the performance notes
//! (`docs/porting/desktop-performance.md`, sections 8.1 and 9), mostly the values a layoutable
//! keeps a copy of (9.9), the measure holds (9.15), the walks of the tree that stop where nothing
//! changes (9.21) and the events nobody hears (9.28).
//!
//! The leaves are borders of a fixed size, so that a benchmark measures the panels, the layoutables
//! and the layout manager and not the content. The services are those of the tree benchmarks
//! (`trees::fixture`): the Fluent theme in its light variant and a root of 1280 x 820 that renders
//! nothing. The root measures its child with the size of its client area and is arranged at the
//! size its child desires.
//!
//! A pass is run by hand (`execute_layout_pass`), so the first invalidation after a pass asks the
//! render loop for a pass that never has anything to do. Benchmarks whose body is a few
//! microseconds repeat it in one invocation and drop those requests between iterations
//! (`fixture::run_queued_jobs`); the others carry one request per invocation, as
//! `layout::Measure.remeasure` does.

use crate::harness::Registry;

pub mod attached_layout_properties;
pub mod fixture;
pub mod invalidation;
pub mod layout_manager_pass;
pub mod listeners;
pub mod nested_depth;
pub mod panel_measure_arrange;
pub mod scroll_content;
pub mod size_change;
pub mod visibility;

pub fn register(registry: &mut Registry) {
    panel_measure_arrange::register(registry);
    nested_depth::register(registry);
    invalidation::register(registry);
    layout_manager_pass::register(registry);
    size_change::register(registry);
    visibility::register(registry);
    scroll_content::register(registry);
    attached_layout_properties::register(registry);
    listeners::register(registry);
}
