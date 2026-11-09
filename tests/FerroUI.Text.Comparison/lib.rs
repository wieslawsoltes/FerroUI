//! ferroui-text-comparison
//!
//! Compares the text layout of FerroUI with that of parley, cosmic-text and epaint (the text of
//! egui) on identical inputs: the same font file (Roboto Regular of this repository, and no font
//! of the system), the same size (14 px), the same widths and the same strings
//! ([`corpus`]), measured with the harness of the framework's benchmarks
//! (`ferroui_benchmarks::harness`) in the same process. The scenarios:
//!
//! - `cold`: every string of the corpus laid out at 240 px from its text, with no cache of that
//!   string (the cold path of each library; see `README.md`);
//! - `rewrap_sweep`: the strings shaped once in the setup, then every string laid out at the next
//!   width of a sweep from 180 to 320 px in steps of 7 (cycled), the width changing every frame;
//! - `same_width`: every prepared string laid out at 240 px again, every cache warm (the steady
//!   state when nothing changed);
//! - `rewrap_cached_same_width` (epaint alone): the galley cache hit at the same width without a
//!   frame in between.
//!
//! An operation lays out every string of the corpus once. Each benchmark is registered as
//! `text_comparison::<Library>.<scenario>[Corpus=<corpus>]`, so that the filter of the harness
//! selects a scenario on every library (`rewrap_sweep`), a library (`Parley.`) or a corpus
//! (`Corpus=card`). [`run`] prints, first, the lines and heights each library lays the cards out
//! in at 240 px ([`agreement`]), then one table: a row per scenario and corpus, a column per
//! library, and the ratio of each library to FerroUI.
//!
//! See `README.md` for the command, the corpora, what each library caches and the caveats.

pub mod agreement;
pub mod corpus;
pub mod cosmic_target;
pub mod epaint_target;
pub mod ferroui_target;
pub mod font;
pub mod parley_target;
pub mod target;

mod program;

pub use program::run;

use corpus::{Corpus, LAYOUT_WIDTH};
use cosmic_target::CosmicTextTarget;
use epaint_target::EpaintTarget;
use ferroui_benchmarks::harness::Registry;
use ferroui_target::FerroUiTarget;
use parley_target::ParleyTarget;
use std::hint::black_box;
use target::{Sweep, TextTarget};

/// The category of every benchmark of the crate.
pub const CATEGORY: &str = "text_comparison";

/// A library of the comparison: a column of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryKind {
    FerroUi,
    Parley,
    CosmicText,
    Epaint,
}

impl LibraryKind {
    pub const ALL: [LibraryKind; 4] =
        [LibraryKind::FerroUi, LibraryKind::Parley, LibraryKind::CosmicText, LibraryKind::Epaint];

    /// The class of the library's benchmarks.
    pub fn class(self) -> &'static str {
        match self {
            LibraryKind::FerroUi => "FerroUi",
            LibraryKind::Parley => "Parley",
            LibraryKind::CosmicText => "CosmicText",
            LibraryKind::Epaint => "Epaint",
        }
    }

    /// The heading of the library's column in the table.
    pub fn heading(self) -> &'static str {
        match self {
            LibraryKind::FerroUi => "FerroUI",
            LibraryKind::Parley => "parley",
            LibraryKind::CosmicText => "cosmic-text",
            LibraryKind::Epaint => "epaint",
        }
    }
}

/// How many strings a corpus has: the full counts, or a few for the quick tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Full,
    Quick,
}

/// A scenario every library runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    Cold,
    RewrapSweep,
    SameWidth,
}

impl Scenario {
    pub const ALL: [Scenario; 3] = [Scenario::Cold, Scenario::RewrapSweep, Scenario::SameWidth];

    pub fn name(self) -> &'static str {
        match self {
            Scenario::Cold => "cold",
            Scenario::RewrapSweep => "rewrap_sweep",
            Scenario::SameWidth => "same_width",
        }
    }
}

/// The scenario of epaint alone: the galley cache hit at the same width.
pub const EPAINT_CACHED_SCENARIO: &str = "rewrap_cached_same_width";

/// Registers every benchmark of the crate with the full corpora.
pub fn register(registry: &mut Registry) {
    register_scaled(registry, Scale::Full);
}

/// Registers every benchmark of the crate with a few strings of each corpus: what the quick
/// tests run.
pub fn register_quick(registry: &mut Registry) {
    register_scaled(registry, Scale::Quick);
}

/// Registers the benchmarks corpus by corpus and scenario by scenario, so that the libraries of
/// a scenario run one after another.
pub fn register_scaled(registry: &mut Registry, scale: Scale) {
    for corpus in Corpus::ALL {
        for scenario in Scenario::ALL {
            register_scenario::<FerroUiTarget>(registry, corpus, scenario, scale);
            register_scenario::<ParleyTarget>(registry, corpus, scenario, scale);
            register_scenario::<CosmicTextTarget>(registry, corpus, scenario, scale);
            register_scenario::<EpaintTarget>(registry, corpus, scenario, scale);
        }
        registry.class(CATEGORY, LibraryKind::Epaint.class()).benchmark(
            EPAINT_CACHED_SCENARIO,
            parameters(corpus),
            move || {
                let mut target = EpaintTarget::new(corpus.texts(scale));
                target.prepare();
                target
            },
            |target: &mut EpaintTarget| black_box(target.relayout_cached_without_pass(LAYOUT_WIDTH)),
        );
    }
}

/// The parameters of the benchmarks of `corpus`.
pub fn parameters(corpus: Corpus) -> String {
    format!("Corpus={}", corpus.name())
}

fn register_scenario<T: TextTarget>(registry: &mut Registry, corpus: Corpus, scenario: Scenario, scale: Scale) {
    let mut class = registry.class(CATEGORY, T::KIND.class());
    let name = scenario.name();
    let parameters = parameters(corpus);
    match scenario {
        Scenario::Cold => {
            class.benchmark(
                name,
                parameters,
                move || T::new(corpus.texts(scale)),
                |target: &mut T| black_box(target.cold(LAYOUT_WIDTH)),
            );
        }
        Scenario::RewrapSweep => {
            class.benchmark(
                name,
                parameters,
                move || Sweep::prepared(T::new(corpus.texts(scale))),
                |sweep: &mut Sweep<T>| black_box(sweep.relayout_next()),
            );
        }
        Scenario::SameWidth => {
            class.benchmark(
                name,
                parameters,
                move || {
                    let mut target = T::new(corpus.texts(scale));
                    target.prepare();
                    target
                },
                |target: &mut T| black_box(target.relayout(LAYOUT_WIDTH)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_benchmarks::harness;
    use std::collections::HashSet;

    #[test]
    fn every_benchmark_has_a_name_of_its_own() {
        let mut registry = Registry::new();
        register(&mut registry);
        // Three corpora, three scenarios on four libraries, and the cached scenario of epaint.
        assert_eq!(registry.len(), 3 * (3 * 4 + 1));

        let mut names = HashSet::new();
        for descriptor in registry.descriptors() {
            assert!(names.insert(descriptor.full_name()), "{} is registered twice", descriptor.full_name());
        }
    }

    // One invocation of every benchmark of a library, with a few strings of each corpus, on a
    // thread of its own: the setup and the body of every scenario.

    #[test]
    fn every_scenario_runs_once_on_ferroui() {
        harness::smoke_class(register_quick, LibraryKind::FerroUi.class());
    }

    #[test]
    fn every_scenario_runs_once_on_parley() {
        harness::smoke_class(register_quick, LibraryKind::Parley.class());
    }

    #[test]
    fn every_scenario_runs_once_on_cosmic_text() {
        harness::smoke_class(register_quick, LibraryKind::CosmicText.class());
    }

    #[test]
    fn every_scenario_runs_once_on_epaint() {
        harness::smoke_class(register_quick, LibraryKind::Epaint.class());
    }

    /// The prepared path at the width of the cold scenario lays out what the cold path does, on
    /// every library.
    #[test]
    fn the_prepared_layout_at_the_cold_width_is_the_cold_layout() {
        fn check<T: TextTarget>() {
            let texts = Corpus::Card.texts(Scale::Quick);
            let mut cold = T::new(texts.clone());
            let cold_totals = cold.cold(LAYOUT_WIDTH);
            drop(cold);
            let mut prepared = T::new(texts);
            prepared.prepare();
            let prepared_totals = prepared.relayout(LAYOUT_WIDTH);
            assert_eq!(cold_totals.lines, prepared_totals.lines, "{:?}", T::KIND);
            assert!(cold_totals.lines >= Corpus::Card.count(Scale::Quick), "{:?}: a line per card at least", T::KIND);
            assert!((cold_totals.height - prepared_totals.height).abs() < 0.5, "{:?}", T::KIND);
        }
        // Each library on a thread of its own, as the harness runs a benchmark: the services of
        // FerroUI belong to a thread.
        agreement::on_own_thread(check::<FerroUiTarget>);
        agreement::on_own_thread(check::<ParleyTarget>);
        agreement::on_own_thread(check::<CosmicTextTarget>);
        agreement::on_own_thread(check::<EpaintTarget>);
    }
}
