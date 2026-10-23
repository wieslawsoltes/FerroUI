# StaticViewLocatorDemo

The sample of `ferroui-static-view-locator`. It ports `StaticViewLocatorDemo` and `StaticViewLocatorReactiveUIDemo` of StaticViewLocator (<https://github.com/wieslawsoltes/StaticViewLocator>). The views of its view models are found by view locators that the build script generates (`build.rs`); no view is looked up by the name of a type at run time.

## What it shows

- **Conventions.** The view models are one module per type (`view_models/dashboard_view_model.rs`) and so are the views (`views/dashboard_view.rs`). The default rules of the generator map `view_models::dashboard_view_model::DashboardViewModel` to `views::dashboard_view::DashboardView`.
- **A generated data template.** `ViewLocator` (`view_locator.rs`) is `#[static_view_locator(generate_data_template = true, ..)]`, the data template of the window. The tabs show the pages through it. The dashboard and the reports show each metric and report in a content control, which the same locator fills. The window's own view model has no view, and the locator lists it among its missing views.
- **ReactiveUI.** The last tab shows one navigation twice: with `ReactiveViewLocator` as the data template of a content control, and as the view locator of a `ViewModelViewHost`. Its views implement `IControlViewFor` (ReactiveUI's `IViewFor<TViewModel>`), which the generator discovers. "Fallback wrapper" shows a view model wrapped in a `ContextHost` through the `build_fallback_view` hook the locator declares.

## Running it

```sh
cargo run -p static-view-locator-demo
```

The generated locators are in `target/<profile>/build/static-view-locator-demo-*/out/static_view_locator/`.
