# x:Bind compiler fixtures

Original fixtures of steps X1 to X4 of #525 (docs/porting/x-bind.md). They run through the run-time
loader and through the Rust emitter, whose output, `x_bind.generated.rs`, is checked in, compiled
with the tests and compared with the current compiler; the same assertions run on both
(`../x_bind_extension_tests.rs`).

- `x_bind_root.xaml`: the root source is the document root. `OneTime`, `OneWay` and `TwoWay`
  paths of a property of the root, the `LostFocus` default of `TextBox.Text` and an explicit
  `PropertyChanged` trigger, a path that starts at a named element (`editor.Text`) and an
  attached property of the root (`(Grid.Column)`); the lifecycle (`Initialize`, `Update`,
  `StopTracking`) and the initialization when the root is attached to a window.
- `x_bind_template.xaml`: a `DataTemplate` with `x:DataType`. Two instances, each updated when
  its data context changes to an item, a recycled instance, a named element of the template, and
  stopping one instance without the other.
- `x_bind_functions.xaml` (X2, after WinUI's `FunctionTests`): the root is an `XBindPage`
  (`support/x_bind.rs`). Functions on the root, on a path and static, with path, named-element
  and literal arguments (`x:True`, `x:Null`, numbers, quoted text with a comma), `OneTime` and
  `OneWay`, a function called again on a `PropertyChanged` with its name, `BindBack` on the
  root, on a path and static, a static member, an integer and a string indexer, an interface
  member, and a function on the item of a typed template.
- `x_bind_casts.xaml` (X2, after WinUI's `CastingTests`): casts of the root source, of a leaf and
  inside a path (`((local:XBindModel)Tag).Name`, `TwoWay` included), a function on a cast, an
  interface member after a path (`Tag.(local:IXBindNamed.DisplayName)`) and an attached property
  after a path (`rowHolder.(Grid.Row)`).
- `x_bind_converters.xaml` (X3, after WinUI's `TwoWayTests` and `NullableTests`): a converter
  (`XBindTagConverter`, a `{StaticResource}`) `OneWay`, with `ConverterParameter` and
  `ConverterLanguage`, and `TwoWay` through `ConvertBack`; `FallbackValue` for a path with a null
  in its middle (`OneTime`, `OneWay`, `TwoWay`, `{x:Null}`), and `TargetNullValue` (`OneWay`,
  `TwoWay`).
- `x_bind_events.xaml` (X4, after WinUI's `EventTests`): `Click` bound to methods of the root
  with the event's signature, without parameters and with base parameters, to a method on a path
  (`Model.Poke`, and `Current.Poke`, null at first, then another owner), to a static method
  (`local:XBindHelpers.Clicked`), an event that is not a routed event (`Pinged` of the root),
  and `Click` on the item of a typed template; the handlers are subscribed by `Initialize` and
  released by `StopTracking`.

Regenerate the compiled fixture after reviewing a compiler change:

```sh
cargo test -p ferroui-markup-xaml-tests --lib markup_extensions::x_bind_extension_tests::regenerate_x_bind_fixture -- --ignored --exact
```

Do not edit or rustfmt `x_bind.generated.rs`.
