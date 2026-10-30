# Default template reference probe

This console-only probe checks two framework behaviors used by the original
`DockDataTemplateHelper`. It references the repository's original grid enum and
the original framework version 12.1.3. It does not launch a docking sample or
participate in the Rust runtime/build.

Run with .NET 10 installed. To use an installed SDK independently of the root
repository's exact SDK pin, run from outside this repository and pass the absolute
path to `TemplateReference.csproj` to `dotnet run --project`.

Verified output:

```text
Columns: Avalonia.Data.BindingNotification: {Error: System.InvalidCastException: Could not convert 'Columns' to 'GridResizeDirection'.}
Rows: Avalonia.Data.BindingNotification: {Error: System.InvalidCastException: Could not convert 'Rows' to 'GridResizeDirection'.}
Null matches: True
```

The original helper does not install a cross-enum converter. The Rust port also
passes the original model enum to the shared default converter, retaining the
conversion failure/default native direction. It does not invent a name or numeric
mapping. The generic reference-type templates also accept null. Native tests cover
both behaviors, all fourteen template types/order, fresh controls, model identity,
proportional splitter live updates and source/context subscription replacement.
