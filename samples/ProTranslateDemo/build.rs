//! Compiles the markup documents of the sample and generates its per-document tests
//! (`sample-build`; docs/porting/xaml.md, 9.5.22).

fn main() {
    ferroui_protranslate_build::Builder::new()
        .notify_properties(true)
        .catalog("Strings.en-US.json")
        .catalog("Strings.pl-PL.json")
        .catalog("Strings.ar-SA.json")
        .generate()
        .expect("valid translation catalogs");
    sample_build::SampleBuild::new("ProTranslateDemo").run();
}
