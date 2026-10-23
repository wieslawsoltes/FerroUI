//! Compiles the markup documents of the demo, embeds its assets and generates its per-document
//! tests (`sample-build`; docs/porting/xaml.md, 9.5.22).

fn main() {
    // The application of the demo (its theme and its window) is the feature `app`; the
    // ControlCatalog uses the views alone.
    sample_build::SampleBuild::new("TreeDataGridDemo")
        .only_with_feature("app", "App.xaml")
        .only_with_feature("app", "MainWindow.xaml")
        .run();
}
