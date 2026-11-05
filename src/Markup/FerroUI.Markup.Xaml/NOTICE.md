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
| `XBindExtension::bind_back`, and the function and `BindBack` helpers of `xaml_il/runtime/compiled.rs` (`path_function`, `path_static_function`, `path_bind_back`, `path_static_bind_back`, `path_key_indexer`; step X2) | `BindAssignment.cs` (`BindBackPath`, `ValidateBindBackAssignment`), `CodeGenerators/BindPathStepCodeGenerators.cs` (the invocation of a function step and its update when an argument changes), the generated code of `Tests/RegressionProjects/Features/CompiledBinding/BindTestbedCS/FunctionTests.xaml` in `src/XamlCompiler/TestMasters/RegressionProjects/fre/Features/CompiledBinding/BindTestbedCS/generated/FunctionTests.g.cs` (`Invoke_*`, `Update_*`, `UpdateTwoWay_*` calling the `BindBack` method, the `PropertyChanged` cases named after a function) |
| `XBindExtension` `Converter`, `ConverterParameter`, `ConverterLanguage`, `FallbackValue`, `TargetNullValue`, their checks in `validate`, and the options of the expression of `x_bind/binding.rs` (step X3) | `BindAssignment.cs` (`Converter`, `ConverterParameter`, `ConverterLanguage`, `GetConverterExpression`, `FallbackValueExpression`, `TargetNullValueExpression`, `ValidateConverter`, `ValidateFallbackValue`), `XamlDomValidator.cs` (`ProcessXBindPropertyNode`, TargetNullValue on a non-nullable type) |
| `x_bind/bindings.rs` (step X5: `IDataTemplateComponent`, its `ProcessBindings` phase by phase with the targets of a later phase hidden until their phase, `Recycle`; the root of a control template reading its templated parent; a `Window` root initialized on `Activated`), the `Phase` and `TargetType` of `XBindExtension` | `CodeGenerators/CSharpPagePass2.tt` (`ProcessBindings`, `Recycle`, `GetNextPhase`, `SuspendRendering`, `ResumeRendering`, `Activated`), `BindUniverse.cs` (`AddPhase`, `GetNextPhase`, the load root of a control template), `BindAssignment.cs` (`ComputedPhase`), `XamlHarvester.cs` (the scope root of a `ControlTemplate`) |
| `x_bind/deferred_element.rs` (`x:Load` and `x:DeferLoadStrategy`, step X5) | `BoundLoadAssignment.cs`, `DeferLoadStrategy.cs`, `CodeGenerators/CSharpPagePass2.tt` (the `FindName` and `UnloadObject` of a bound `x:Load`, and of its `LoadRootElement` in a control template, the fix of bound `x:Load` in control templates, microsoft-ui-xaml #12239) |
| `x_bind/event.rs`, `XBinding::bind_routed_event` and `bind_markup_event`, the event bindings of `x_bind/bindings.rs`, and the helpers `path_event_handler`, `path_static_event_handler`, `x_bind_routed_event`, `x_bind_event` of `xaml_il/runtime/compiled.rs` (step X4) | `src/XamlCompiler/BuildTasks/Microsoft/Xaml/XamlCompiler/BoundEventAssignment.cs`, the generated code of `Tests/RegressionProjects/Features/CompiledBinding/BindTestbedCS/EventTests.xaml` in `src/XamlCompiler/TestMasters/RegressionProjects/chk/Features/CompiledBinding/BindTestbedCS/generated/EventTests.g.cs` (the `obj*ClickHandler` methods that walk the path with null checks and call the method with or without the event's arguments, `Create_obj*Click`, the subscriptions of `Connect`) |

The compiler half (the root type and the `x:DataType` of a template as the
start of the path, named elements, the diagnostics) is in
`ferroui-markup-xaml-loader`, `compiler_extensions/transformers/x_bind.rs`; its
messages follow `src/XamlCompiler/Properties/XamlCompilerResources.resx`
(`Harvester_DataTemplateDoesNotDefineDataType`,
`BindAssignment_XamlXBindUsedInStyleError`, `BindAssignment_TwoWaySourceNotWritable`,
`BindAssignment_UpdateSourceTrigger_ExplicitUnsupported`,
`BindAssignment_UpdateSourceTrigger_UpdateSourceTriggerOnlyWithTwoWay`,
`BindAssignment_FieldNameElementName_Conflict`).

Step X2 adds the path grammar of WinUI's compiled bindings to the compiler:
`compiler_extensions/transformers/x_bind_path.rs` parses the grammar of
`src/XamlCompiler/BuildTasks/BindingPath/BindingPath.g4`, and `x_bind.rs`
resolves it as `src/XamlCompiler/Microsoft.UI.Xaml.Markup.Compiler.Parsing/BindPathListener.cs`
does (static members, indexers, attached properties and the members of a cast
type, casts, functions with the overload of the number of arguments and their
parameters, `FunctionStep.cs` and the `Function*Param.cs` files); the messages
follow `Microsoft.UI.Xaml.Markup.Compiler.Parsing/ErrorMessages.resx`
(`FunctionNotLeaf`, `FunctionAsParameter`, `NoMatchingOverload`,
`MissmatchedParameterCount`, `InvalidParameter`, `ExpectingMethod`,
`ExpectingStaticFunction`, `ExpectingStaticProperty`, `PropertyNotFound`,
`InvalidCast`, `SyntaxError`) and `XamlCompilerResources.resx`
(`BindAssignment_XamlXBindParseError`, `BindAssignment_FunctionReturnTypeInvalid`,
`BindAssignment_BindBack_NotFound`, `BindAssignment_BindBack_NotMethod`,
`BindAssignment_BindBack_InvalidMethod`, `BindAssignment_BindBack_Unexpected`,
`BindPathParser_CantTwoWayCastStep`). The bracket characters of the path of
x:Bind in the markup extension scanner of xamlx follow
`src/XamlCompiler/BuildTasks/Microsoft/Xaml/DirectUI/ProxyDirectUIXamlMember.cs`
(`LookupMarkupExtensionBracketCharacters`). The tests port the behaviours of
`Tests/RegressionProjects/Features/CompiledBinding/BindTestbedCS/FunctionTests.xaml`
and `CastingTests.xaml`.

Step X3 adds the converter and the substitute values: the checks of
`x_bind.rs` follow `BindAssignment.cs` (`ValidateConverter`,
`ValidateFallbackValue`) and `XamlDomValidator.cs` (`ProcessXBindPropertyNode`),
with the messages of `XamlCompilerResources.resx`
(`BindAssignment_OrphanConverterParam`,
`BindAssignment_ConverterWithFunctionBindingNotSupported`,
`BindAssignment_InvalidFallbackValue`,
`XamlCompiler_XBindTargetNullValueOnNonNullableType`). The tests port the
converter, fallback and null-value behaviours of
`Tests/RegressionProjects/Features/CompiledBinding/BindTestbedCS/TwoWayTests.xaml`
and `NullableTests.xaml`.

Step X4 adds event binding: the event path of `x_bind.rs` (`prepare_event`,
`transform_event`) follows `BoundEventAssignment.ValidateEvent` (a method leaf,
no overloads, no parameters or parameters the arguments of the event's `Invoke`
are assignable to), with the messages of `XamlCompilerResources.resx`
(`BoundEventAssignment_NonDelegateProperty`, `BoundEventAssignment_NonLeafMethod`,
`BoundEventAssignment_NoOverloads`, `BoundEventAssignment_InvalidSignature`,
`BoundEventAssignment_SignatureMismatch`, `BindPathParser_CantBindToMethods`).
The tests port the behaviours of
`Tests/RegressionProjects/Features/CompiledBinding/BindTestbedCS/EventTests.xaml`
and `EventTests.xaml.cs`.

Step X5 adds the directives around x:Bind:
`compiler_extensions/transformers/x_bind_directives.rs` and `x_bind.rs`
(`prepare_scope`, `apply_default_mode`) follow `XamlDomValidator.cs` (the checks
of `x:DefaultBindMode`, `x:DeferLoadStrategy` and `x:Load`,
`CanHaveDeferLoadStrategyOrLoad`), `PhaseAssignment.cs`, `DefaultBindMode.cs`,
`XamlHarvester.cs` (the `x:DefaultBindMode` members stripped from the tree, the
scope root of a `ControlTemplate`) and `BindUniverse.cs`, with the messages of
`XamlCompilerResources.resx`
(`XamlValidationError_DefaultBindModeInvalidValue`,
`XamlCompiler_InvalidValueForPhase`, `XamlCompiler_PhaseMustBeUsedWithinADataTemplate`,
`XamlCompiler_PhaseMustHaveAssociatedBind`, `XamlCompiler_LoadNotSupported`,
`XamlCompiler_LoadMissingName`, `XamlCompiler_LoadConflict`,
`XamlValidationError_DeferLoadStrategyInvalidValue`,
`Harvester_ControlTemplateDoesNotDefineTargetType`). The tests port the
behaviours of
`Tests/RegressionProjects/Features/CompiledBinding/BindTestbedCS/PhasingTests.xaml`
and `ListAndTemplateTests.xaml`.

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
