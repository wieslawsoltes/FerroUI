# Dock helper compiler reference

This test-only probe checks the C# compiler's lowering of the repeated `Owner`
property pattern used in `DockHelpers.ResolveFloatingWindowHostMode(IDockable)`.
The root, unrelated-owner and null-owner cases each read `Owner` once, despite
its two appearances in the pattern. The subsequent separate factory branch in
the original helper reads `Owner` again when the pattern fails.

Run with an installed .NET 10 SDK, from this directory to use its SDK selector:

```sh
dotnet run --verbosity quiet
```

The program asserts the result and getter count and prints three lines. It copies
only the original pattern shape into a counting fixture; it does not compile the
original DockHelpers class or establish full C#/native UI parity. Rust production
model fixtures separately verify the helper's complete model-resolution precedence
and getter counts. Normal Rust tests do not require this probe or .NET.
