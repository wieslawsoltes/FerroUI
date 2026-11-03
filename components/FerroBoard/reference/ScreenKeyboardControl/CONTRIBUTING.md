# Contributing to ScreenKeyboard

Thanks for your interest in improving ScreenKeyboard! Bug reports, layouts, translations, themes, documentation and
code are all welcome.

## Getting started

1. Install the .NET 10 SDK (and the Uno Platform prerequisites for the targets you want to run — `uno-check` helps).
2. Fork and clone the repository.
3. Build and test:
   ```bash
   dotnet test tests/ScreenKeyboard.Core.Tests
   dotnet run --project samples/ScreenKeyboard.Sample -f net10.0-desktop -p:SkTargetFrameworks=net10.0-desktop
   ```

See [docs/building.md](docs/building.md) for details.

## Guidelines

- Keep `ScreenKeyboard.Core` free of UI dependencies; put platform code in `ScreenKeyboard.Uno`.
- Add unit tests for engine, layout, NLP and gesture changes (`tests/ScreenKeyboard.Core.Tests`). Touch behavior can
  be tested deterministically with `KeyboardTouchProcessor` and a fake `IKeyboardScheduler`.
- Follow the existing code style (`.editorconfig`), nullable annotations and XML documentation for public APIs.
- UI elements inside the keyboard must never take focus (use `TouchButton`, set `IsTabStop = false`).
- Keep public API changes backwards compatible where possible and describe them in `CHANGELOG.md`.

## Layouts and languages

Layout data follows FlorisBoard's format. Improvements to layouts that also apply to FlorisBoard are best
contributed upstream to [FlorisBoard](https://github.com/florisboard/florisboard) as well. Custom layouts can be
proposed as JSON documents (see [docs/layouts.md](docs/layouts.md)).

## Pull requests

1. Create a branch, keep changes focused and add tests/docs.
2. Make sure `dotnet test` passes.
3. Open a pull request describing the motivation and the change; include screenshots for visual changes.

By contributing you agree that your contributions are licensed under the Apache License 2.0.
