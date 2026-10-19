# NOTICE

This crate is a port of the example application of the ReactiveUI integration with the reference
UI framework (<https://github.com/reactiveui/ReactiveUI>, the folder `src/examples` of the
integration's repository; MIT, Copyright (c) 2019-2026 ReactiveUI Association Incorporated).

- The markup documents are converted from the reference documents; the conversion changes names and
  the namespaces of the controls of the integration.
- The Rust sources are ports of the reference C# sources, file by file: the shared library of the
  example (`App`, the models, the services, the view models and the views), the desktop entry point
  (`program.rs`) and the tests (`tests/`).
- The DynamicData page (`ViewModels/dynamic_data_view_model.rs`,
  `ViewModels/feature_item_view_model.rs`, `Models/showcase_feature.rs`, `Views/DynamicDataView.xaml`
  and its code) is an addition of the port with no counterpart in the reference.
- The Android, iOS and browser heads of the reference are not ported: FerroUI builds its own
  Android, iOS and browser applications.
