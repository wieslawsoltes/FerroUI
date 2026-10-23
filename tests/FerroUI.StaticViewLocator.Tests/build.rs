//! Generates the locators of the fixtures (`ferroui-static-view-locator-build`), with the default
//! options of the generator.

fn main() {
    ferroui_static_view_locator_build::StaticViewLocatorBuild::from_env().run();
}
