# AppWithoutLifetime: gaps

What the sample uses that the framework lacks or does differently. Status: 3 documents, converted without an edit, all compiled by the build; `cargo test -p app-without-lifetime --lib` passes.

| Gap | What the sample does | The upstream statement | The framework | Status |
|---|---|---|---|---|
| A001 | A debug build attaches the developer tools to the application. | `Program.cs`: `#if DEBUG .WithDeveloperTools() #endif` in the method that configures the application builder (`build_ferro_app` here) | The application builder has no developer tools extension (the diagnostics support package of the upstream project is not ported). The statement is left out of `program.rs`; nothing else depends on it. | open: nothing to reproduce |
| A002 | The entry point is marked as running in a single-threaded apartment. | `Program.cs`: `[STAThread]` on `Main` | An attribute of one platform's component model with no counterpart: the thread that calls `main` is the UI thread of the dispatcher. Left out. | not a gap of the framework |
| A003 | The project names a Windows application manifest and references the Inter font library without using it. | `AppWithoutLifetime.csproj`: `<ApplicationManifest>app.manifest</ApplicationManifest>`, the project references to the colour picker and the Inter font project | The manifest is a file of one platform and is not ported. The crate depends on neither library because no statement of the sample names them (a project reference alone is not a statement). | not a gap of the framework: recorded as a difference of the project file |

What the port adds, which the upstream sample does not have:

- The smoke option of the entry point (`FERROUI_SMOKE_EXIT_MS`). The other samples take it from
  `sample_support::smoke_run`, which closes the main window of the desktop lifetime; this sample
  has no lifetime, so `program.rs` closes the window it created after the interval, which ends the
  main loop as closing the window by hand does.

Read and found the same as upstream, confirmed by `tests/shell.rs` and one smoke run of the desktop host:

- `AppBuilder::start(main, args)` sets the application up and calls `main` with the application
  and the arguments, without a lifetime (`Application::application_lifetime()` is `None`).
- `Application::run_window(window)` is `app.Run(Window)`: it shows the window when it is not
  visible and runs the main loop of the dispatcher until the window is closed.
- `Window::show_with_owner(owner)` is `Show(Window owner)`: the second window is in
  `owned_windows()` of the main window and is closed with it.
