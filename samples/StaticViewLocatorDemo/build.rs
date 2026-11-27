//! Generates the view locators of the sample (`ferroui-static-view-locator-build`): the view models
//! of `crate::view_models` and below, mapped to the views of `crate::views` by name. Then the
//! units of the WinUI theme the sample needs (`ferroui-build`, docs/xaml-themes.md, section 3):
//! the build reads the sources of the crate, finds the controls its views make, and writes the
//! table `main.rs` constructs the theme with.

fn main() {
    ferroui_static_view_locator_build::StaticViewLocatorBuild::from_env().view_model_namespace_prefixes(["crate::view_models"]).run();
    ferroui_build::Build::from_env().type_system(ferroui_build::TypeSystem::Model).theme("WinUITheme").run();
}
