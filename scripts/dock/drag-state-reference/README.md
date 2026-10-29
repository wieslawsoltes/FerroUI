# Drag-state numeric reference

This test-only .NET 10 probe checks the numeric expressions used by the original
DockControlState: `unchecked((int)Math.Round(value))` for fallback screen pixels and
`Math.Max(0.0, value)` for preview dimensions. It does not compile the entire state class
or establish any input/drag/host behavior.

From this directory:

```sh
dotnet run --verbosity quiet
```

The local SDK selector uses the latest installed .NET 10 feature band. Review the output
before saving it to `../../../tests/Dock/data/drag-state-numeric.tsv`. Normal Rust tests read that
fixture without requiring .NET. Input and result double bits retain NaNs and signed zero.
Cases cover both infinities, positive/negative ties and both int32 boundaries. Older CLR
versions/platforms are not verified by this reference. No UI or external package dependency is needed.
