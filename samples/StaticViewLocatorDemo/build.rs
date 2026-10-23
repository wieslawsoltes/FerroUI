//! Generates the view locators of the sample (`ferroui-static-view-locator-build`): the view models
//! of `crate::view_models` and below, mapped to the views of `crate::views` by name.

fn main() {
    ferroui_static_view_locator_build::StaticViewLocatorBuild::from_env().view_model_namespace_prefixes(["crate::view_models"]).run();
}
