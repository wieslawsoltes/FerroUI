# NOTICE

## WinUI

The `{x:Bind}` markup extension and its per-root lifecycle in
`markup_extensions/x_bind/` (`XBindExtension`, `XBinding`, `XBindings` and the
root source node) implement the observable behaviour of the compiled bindings
of the WinUI XAML compiler (<https://github.com/microsoft/microsoft-ui-xaml>),
MIT License, at commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`. No WinUI
code is copied: the behaviour is derived from these sources (the plan, with
the steps that follow, is `docs/porting/x-bind.md`):

| Implementation | Reference source |
|---|---|
| `x_bind/extension.rs` (the default mode `OneTime`, the modes and update triggers accepted) | `src/XamlCompiler/BuildTasks/Microsoft/Xaml/XamlCompiler/BindAssignment.cs` (`ValidateUpdateSourceTrigger`), `DefaultBindMode.cs`, `UpdateSourceTrigger.cs` |
| `x_bind/binding.rs` (the `LostFocus` default of `TextBox.Text`) | `BindAssignment.cs`, `NeedsLostFocusForTwoWay` |
| `x_bind/bindings.rs` (`Initialize`, `Update`, `StopTracking`; initialization on `Loading`; a template instance updated when its data context changes) | `src/XamlCompiler/BuildTasks/Microsoft/Xaml/XamlCompiler/CodeGenerators/CSharpPagePass2.tt` (the generated `Bindings` class: `Loading`, `Initialize`, `Update`, `StopTracking`, `DataContextChangedHandler`, `SetDataRoot`) |
| `x_bind/root_node.rs` (the root source: the document root, or the typed data context of a template) | `src/XamlCompiler/BuildTasks/Microsoft/Xaml/XamlCompiler/XamlHarvester.cs` (the scope root of a `DataTemplate` with `x:DataType`), `BindUniverse.cs` |

The compiler half (the root type and the `x:DataType` of a template as the
start of the path, named elements, the diagnostics) is in
`ferroui-markup-xaml-loader`, `compiler_extensions/transformers/x_bind.rs`; its
messages follow `src/XamlCompiler/Properties/XamlCompilerResources.resx`
(`Harvester_DataTemplateDoesNotDefineDataType`,
`BindAssignment_XamlXBindUsedInStyleError`, `BindAssignment_TwoWaySourceNotWritable`,
`BindAssignment_UpdateSourceTrigger_ExplicitUnsupported`,
`BindAssignment_UpdateSourceTrigger_UpdateSourceTriggerOnlyWithTwoWay`,
`BindAssignment_FieldNameElementName_Conflict`).

```
MIT License

Copyright (c) Microsoft Corporation. All rights reserved.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
