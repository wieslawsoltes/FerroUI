//! Scaling pairs of the reactive runtime of `ferroui-view`: effects that leave a signal which
//! many effects read, at 1,000 and at 4,000 effects (experiment #818; `scripts/perf/gate.py`,
//! "scaling pairs"). A signal every element of a screen follows (a frame counter, a theme, a
//! selection) has as many subscribers as the screen has elements; what one of them costs when it
//! leaves must not grow with the others, or a screen that goes costs its size squared.
//!
//! Each benchmark is one whole operation, so the instructions of a pair compare as they are:
//!
//! - `rows_leave`: every effect has an owner of its own (a row of a list), and the rows are
//!   disposed one at a time, the oldest first;
//! - `owner_disposed`: the effects have one owner (a screen), which is disposed;
//! - `stop_reading`: every effect reads the signal behind a condition, and the condition goes;
//! - `broadcast`: the signal is written, and every effect runs.

use crate::harness::Registry;
use ferroui_view::prelude::*;

/// The sizes of a pair: the second is `SCALING_FACTOR` of the gate times the first.
const SIZES: [usize; 2] = [1_000, 4_000];

pub struct Leaving {
    effects: usize,
    root: Owner,
    shared: Signal<u64>,
    reads: Signal<bool>,
    rows: Vec<Owner>,
    screen: Option<Owner>,
}

impl Leaving {
    fn new(effects: usize) -> Self {
        let root = Owner::new_root();
        let (shared, reads) = root.with(|| (signal(0_u64), signal(true)));
        Self { effects, root, shared, reads, rows: Vec::new(), screen: None }
    }

    /// What an iteration before left is disposed, and the condition is as it was.
    fn reset(&mut self) {
        for row in self.rows.drain(..) {
            row.dispose();
        }
        if let Some(screen) = self.screen.take() {
            screen.dispose();
        }
        self.reads.set(true);
    }

    /// Every effect in an owner of its own.
    fn prepare_rows(&mut self) {
        self.reset();
        let shared = self.shared;
        self.rows = self.root.with(|| {
            (0..self.effects)
                .map(|_| {
                    let row = Owner::new();
                    row.with(|| effect(move || drop(shared.get())));
                    row
                })
                .collect()
        });
    }

    /// The effects in one owner; with `conditional` they read the signal while the condition
    /// holds.
    fn prepare_screen(&mut self, conditional: bool) {
        self.reset();
        let (shared, reads) = (self.shared, self.reads);
        let screen = self.root.with(Owner::new);
        screen.with(|| {
            for _ in 0..self.effects {
                match conditional {
                    true => effect(move || {
                        if reads.get() {
                            drop(shared.get());
                        }
                    }),
                    false => effect(move || drop(shared.get())),
                }
            }
        });
        self.screen = Some(screen);
    }

    pub fn rows_leave(&mut self) {
        for row in self.rows.drain(..) {
            row.dispose();
        }
    }

    pub fn owner_disposed(&mut self) {
        if let Some(screen) = self.screen.take() {
            screen.dispose();
        }
    }

    pub fn stop_reading(&mut self) {
        self.reads.set(false);
    }

    pub fn broadcast(&mut self) {
        self.shared.update(|value| *value += 1);
    }
}

impl Drop for Leaving {
    fn drop(&mut self) {
        self.root.dispose();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("declarative", "Leaving").min_iteration_time(200);
    for effects in SIZES {
        let parameters = format!("Effects={effects}");
        let new = move || Leaving::new(effects);
        class.benchmark_with_iteration_setup(
            "rows_leave",
            parameters.clone(),
            new,
            Leaving::prepare_rows,
            Leaving::rows_leave,
        );
        class.benchmark_with_iteration_setup(
            "owner_disposed",
            parameters.clone(),
            new,
            |b: &mut Leaving| b.prepare_screen(false),
            Leaving::owner_disposed,
        );
        class.benchmark_with_iteration_setup(
            "stop_reading",
            parameters.clone(),
            new,
            |b: &mut Leaving| b.prepare_screen(true),
            Leaving::stop_reading,
        );
        class.benchmark_with_iteration_setup(
            "broadcast",
            parameters,
            new,
            |b: &mut Leaving| {
                if b.screen.is_none() {
                    b.prepare_screen(false);
                }
            },
            Leaving::broadcast,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn leaving() {
        crate::harness::smoke_class(super::register, "Leaving");
    }
}
