//! Compiles `Details.xaml` to Rust (`sample-build`; docs/porting/xaml.md, 9.5.22): the
//! component loads it by its URI, and the compiled markup is what the load runs.

fn main() {
    sample_build::SampleBuild::new("ViewInterop").run();
}
