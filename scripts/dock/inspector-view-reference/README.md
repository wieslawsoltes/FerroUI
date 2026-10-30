# Original inspector reference

Run in this directory:

```sh
dotnet run -- ../../../tests/Dock/data/inspector-view-grids.tsv
```

The harness compiles all 18 unchanged original XAML/code-behind pairs and the complete
RootDockDebug, extensions and selection classes against actual original projects and
Avalonia Headless 12.1.3. It captures each original grid's definitions/fields/states,
24 actual enum edits and all 26 factory event messages (excluding variable timestamps).
It asserts original editing/nested-owner updates, complete inspector structure, filter
notifications/equality, factory replacement, detach-retained logging and actual filtered
ListBox clearing. F11/default/custom gestures, lazy layout resolution, custom factories,
user-close/reopen, canceled close, repeated disposal and main-window closing are checked.
Rust Headless tests compare committed outputs without requiring .NET for ordinary tests.

The unchanged original Fluent theme also measures a 12px grip inside a 600px header.
The Rust theme deliberately corrects this to span available header width, following
the user's request; Fluent and Simple retain the same linked control template.

The existing SDK is selected by this directory's global.json. If the local NuGet cache
path lacks a trailing slash, pass `-p:NuGetPackageRoot=/path/to/package-cache/` with the
trailing slash. No original source or build file is changed. The separate
`python3 scripts/dock/check_inspector_views.py` guard verifies the leaf views with only
namespace/assembly migration and the main view with declared compiled-scope and
passive behavior/command migrations.

These checks do not establish full platform, culture, unknown numeric enum, collection
enumeration or CLR lifetime parity. Those source scopes remain explicitly partial.
