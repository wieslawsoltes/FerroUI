//! ferroui-benchmarks
//!
//! The benchmarks of the framework (the counterpart of the benchmark
//! project of the upstream project): layout, styling, bindings, text,
//! rendering and collections. One file per upstream file, under the same
//! folders; each benchmark class is a struct whose constructor is the
//! constructor and the global setup of the class and whose methods are its
//! benchmarks, and each file registers its benchmarks with the harness
//! ([`harness`]) under the snake case of the upstream names.
//!
//! The crate measures with the clock of the standard library; see
//! `README.md` for how to run the benchmarks and [`harness`] for how they
//! are measured. Every benchmark also runs once in `cargo test` (one test
//! per benchmark class, in the file of the class).

pub mod harness;

pub mod control_hierarchy_creator;
pub mod test_binding_observable;
pub mod test_root;
pub mod test_styles;
pub mod test_types;

pub mod animations;
pub mod base;
pub mod compositor;
pub mod data;
pub mod layout;
pub mod markup;
pub mod navigation;
pub mod rendering;
pub mod styling;
pub mod text;
pub mod themes;
pub mod traversal;
pub mod utilities;
pub mod visuals;

mod program;

pub use program::{profile_text_layout, run};

use harness::Registry;

/// Registers every benchmark of the crate, folder by folder.
pub fn register(registry: &mut Registry) {
    animations::register(registry);
    base::register(registry);
    compositor::register(registry);
    data::register(registry);
    layout::register(registry);
    markup::register(registry);
    navigation::register(registry);
    rendering::register(registry);
    styling::register(registry);
    text::register(registry);
    themes::register(registry);
    traversal::register(registry);
    utilities::register(registry);
    visuals::register(registry);
}
