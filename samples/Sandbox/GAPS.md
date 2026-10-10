# Sandbox: gaps

What the sample uses that the framework lacks or does differently. Status: 2 documents, converted without an edit, both compiled by the build; `cargo test -p sandbox --lib` passes.

| Gap | What the sample does | The upstream statement | The framework | Status |
|---|---|---|---|---|
| S001 | A debug build attaches the developer tools to the application. | `Program.cs`: `#if DEBUG .WithDeveloperTools() #endif` in the method that configures the application builder (`build_ferro_app` here) | The application builder has no developer tools extension (the diagnostics support package of the upstream project is not ported). The statement is left out of `program.rs`; nothing else depends on it. | open: nothing to reproduce |
| S002 | The project references the colour picker, the Simple theme and the Inter font library beside the Fluent theme, without a statement that uses them. | `Sandbox.csproj`: the three project references | Nothing is lacking: the crates exist. The crate does not depend on them because no statement of the sample names them (a project reference alone is not a statement); markup tried in the sandbox that names one of them needs the dependency and its `register_types()`. | not a gap of the framework: a difference of the project file |

Not gaps:

- `MainWindow.xaml.cs` imports namespaces it does not use (presenters, text input, the composition
  of one platform). The port imports what it uses.
- `App.OnFrameworkInitializationCompleted` does not call the implementation of the base class in
  the upstream sample; the port does not either.
