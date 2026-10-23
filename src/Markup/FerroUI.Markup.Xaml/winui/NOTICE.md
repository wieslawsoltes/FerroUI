# WinUI property-path binding compatibility

The observable binding contract is derived from Microsoft WinUI at revision
`a71c42bd3e352060e5f5ecf3b5e1a98f8a745809`:
https://github.com/microsoft/microsoft-ui-xaml

| Implementation | Reference source |
|---|---|
| `binding.rs` | `src/XamlCompiler/BuildTasks/Microsoft/Xaml/XamlCompiler/BindAssignment.cs`, especially `NeedsLostFocusForTwoWay` and `ValidateUpdateSourceTrigger`; `DefaultBindMode.cs` |
| `bindings.rs` | `src/XamlCompiler/BuildTasks/Microsoft/Xaml/XamlCompiler/CodeGenerators/CSharpPagePass2.tt`: generated `Initialize`, `Update`, `StopTracking` and template instance lifecycle |
| `path_node.rs` | `src/XamlCompiler/Microsoft.UI.Xaml.Markup.Compiler.Parsing/BindPathListener.cs`: root vs typed-template name precedence |

The implementation uses the existing FerroUI property, metadata, notification and binding
engine. It is an explicit property-path adapter, not the WinUI C#/C++ generated binding
compiler. Application fields must expose registered Rust property or model metadata. The
supported grammar consists of dotted names, an empty OneTime/OneWay path, explicit binding
mode and source-update trigger. Functions, event binding, casts, indexers, converters,
fallback/null-value parameters, x:DefaultBindMode, x:Phase and deferred lifecycle remain
diagnosed requirements. New support must retain those diagnostics until implemented.

The host explicitly initializes a document or template root after construction. It must
assign a typed template's DataContext before initializing it. Item replacement then refreshes
that instance's paths; it never changes another template instance. Loaded is not substituted
for WinUI Loading. Unload stopping is opt-in. StopTracking resets the explicit controller to
uninitialized even for groups containing only OneTime paths, whereas WinUI omits that reset
when its generated group requires no tracking. A subsequent Initialize therefore refreshes
all paths under this explicit host lifecycle. Update is not transactional if a user callback
or getter fails; unrelated Rust panics propagate.

OneTime paths do not subscribe to property notifications. Source roots, named elements and
intermediate object values are held weakly to avoid Rust ownership cycles; the declaring
root/model owns its fields. This matches the lifetime policy of the existing native binding
engine rather than relying on a managed garbage collector. Public lifecycle tests check
subscriber counts, root/target release, template isolation and failure recovery.

## Microsoft license

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
