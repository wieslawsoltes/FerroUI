# x:Bind compiler fixtures

Original fixtures of step X1 of #525 (docs/porting/x-bind.md). They run through the run-time
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

Regenerate the compiled fixture after reviewing a compiler change:

```sh
cargo test -p ferroui-markup-xaml-tests --lib markup_extensions::x_bind_extension_tests::regenerate_x_bind_fixture -- --ignored --exact
```

Do not edit or rustfmt `x_bind.generated.rs`.
