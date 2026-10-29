//! Compiles the markup documents of the gallery, embeds its assets and generates its per-document
//! tests (`sample-build`; docs/porting/xaml.md, 9.5.22).

fn main() {
    sample_build::SampleBuild::new("BehaviorsGallery").run();
}
