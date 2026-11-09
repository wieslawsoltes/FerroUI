//! The side check: the lines and the height each library lays the strings of a corpus out in at
//! the width of the cold scenario, so that a run shows that the libraries lay out the same text
//! (the same font, size and widths give roughly the same lines). [`print_once`] prints the table
//! of the card corpus to stderr before the measurements.

use crate::corpus::{Corpus, LAYOUT_WIDTH};
use crate::cosmic_target::CosmicTextTarget;
use crate::epaint_target::EpaintTarget;
use crate::ferroui_target::FerroUiTarget;
use crate::parley_target::ParleyTarget;
use crate::target::{TextTarget, Totals};
use crate::{LibraryKind, Scale};
use std::fmt::Write as _;
use std::sync::Once;

/// The stack of the thread a library lays out on, as the harness gives a benchmark.
const STACK_SIZE: usize = 64 * 1024 * 1024;

/// What a library laid out: the totals of each string, in the order of the corpus.
#[derive(Clone, Debug)]
pub struct LibraryLines {
    pub library: LibraryKind,
    pub per_string: Vec<Totals>,
}

impl LibraryLines {
    pub fn total(&self) -> Totals {
        let mut total = Totals::default();
        for totals in &self.per_string {
            total += *totals;
        }
        total
    }
}

/// Runs `action` on a thread of its own: the services of FerroUI belong to a thread, so each
/// library starts from the state a new thread has, as in a benchmark.
pub fn on_own_thread<R: Send + 'static>(action: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::Builder::new()
        .name("text-comparison".into())
        .stack_size(STACK_SIZE)
        .spawn(action)
        .expect("the thread starts")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

fn lines_of<T: TextTarget>(corpus: Corpus, scale: Scale) -> LibraryLines {
    on_own_thread(move || {
        let mut target = T::new(corpus.texts(scale));
        let per_string = (0..target.len()).map(|index| target.cold_one(index, LAYOUT_WIDTH)).collect();
        LibraryLines { library: T::KIND, per_string }
    })
}

/// The lines every library lays the strings of `corpus` out in at 240 px, cold.
pub fn lines_of_every_library(corpus: Corpus, scale: Scale) -> Vec<LibraryLines> {
    vec![
        lines_of::<FerroUiTarget>(corpus, scale),
        lines_of::<ParleyTarget>(corpus, scale),
        lines_of::<CosmicTextTarget>(corpus, scale),
        lines_of::<EpaintTarget>(corpus, scale),
    ]
}

/// The table: a row per library, with its lines, its height, the ratio of its lines to
/// FerroUI's and the strings it lays out in as many lines as FerroUI. The first entry is
/// FerroUI.
pub fn format_table(corpus: Corpus, libraries: &[LibraryLines]) -> String {
    let mut out = String::new();
    let strings = libraries.first().map_or(0, |library| library.per_string.len());
    let _ = writeln!(
        out,
        "Lines at {LAYOUT_WIDTH} px, corpus {} ({strings} strings), Roboto Regular 14 px:",
        corpus.name()
    );
    let _ = writeln!(
        out,
        "| {:<12} | {:>8} | {:>12} | {:>16} | {:>22} |",
        "Library", "Lines", "Height (px)", "Lines / FerroUI", "Same lines as FerroUI"
    );
    let _ = writeln!(out, "|{:-<14}|{:->9}:|{:->13}:|{:->17}:|{:->23}:|", "", "", "", "", "");
    let reference = libraries.first();
    for library in libraries {
        let total = library.total();
        let (ratio, same) = match reference {
            Some(reference) => {
                let reference_lines = reference.total().lines.max(1) as f64;
                let same = library
                    .per_string
                    .iter()
                    .zip(&reference.per_string)
                    .filter(|(this, other)| this.lines == other.lines)
                    .count();
                (total.lines as f64 / reference_lines, same)
            }
            None => (1.0, strings),
        };
        let _ = writeln!(
            out,
            "| {:<12} | {:>8} | {:>12.1} | {:>16.3} | {:>22} |",
            library.library.heading(),
            total.lines,
            total.height,
            ratio,
            format!("{same} of {strings}")
        );
    }
    out
}

static PRINTED: Once = Once::new();

/// Prints the table of the card corpus to stderr, once a process.
pub fn print_once() {
    PRINTED.call_once(|| {
        let libraries = lines_of_every_library(Corpus::Card, Scale::Full);
        eprintln!("{}", format_table(Corpus::Card, &libraries));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The libraries lay the cards out in roughly as many lines as FerroUI: the same font, size
    /// and width. A large difference means that a library does not see the same font or size.
    #[test]
    fn the_libraries_lay_the_cards_out_in_roughly_the_same_lines() {
        let libraries = lines_of_every_library(Corpus::Card, Scale::Full);
        eprintln!("{}", format_table(Corpus::Card, &libraries));
        let reference = libraries[0].total().lines as f64;
        for library in &libraries {
            let lines = library.total().lines as f64;
            assert!(
                (lines - reference).abs() <= reference * 0.15,
                "{} lays the cards out in {lines} lines, FerroUI in {reference}",
                library.library.heading()
            );
        }
    }
}
