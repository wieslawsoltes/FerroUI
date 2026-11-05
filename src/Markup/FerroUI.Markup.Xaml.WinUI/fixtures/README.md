# Native control and WinUI binding compiler fixtures

These small, original fixtures exercise WinUI's x:Bind on the x:Bind core of the
XAML compiler (docs/porting/x-bind.md, X6) through the runtime interpreter and
through actual Rust emitted by the XAML compiler. They are not copied Gallery
examples; the rest of the x:Bind grammar, its directives and its diagnostics are
tested by the core (`tests/FerroUI.Markup.Xaml.UnitTests/markup_extensions/x_bind/`)
and, for WinUI documents through the interpreter, by `x_bind_tests.rs`.

`winui_root.xaml` tests a document root with a field assigned after construction,
OneTime/OneWay/TwoWay modes, the TextBox.Text LostFocus default, Update,
StopTracking and Initialize after stopping. `winui_template.xaml` places a typed
DataTemplate in ContentControl.ContentTemplate, its normal compiled use. Two
built instances test item isolation, the read of each item when the data context
changes to it, explicit PropertyChanged source updates, DataContext recycling,
detachment of the previous item and independent stopping.

`winui_navigation.xaml` exercises the additive NavigationView family through the
same two backends: original property-element owners, nested and footer collections,
header/separator types, embedded InfoBadge, Frame content and literal Symbol icons.
The opt-in migration expands `Icon="Mail"` and `QueryIcon="Find"` into separate
SymbolIcon objects. Selection, nonselecting footer invocation, ancestor state,
removal and instance ownership are checked on the constructed native controls.

`native_controls.xaml` uses the ordinary FerroUI presentation namespace directly,
without a migration policy. It constructs the promoted controls, icon sources and
template settings, while retaining the established `Page`/`ContentPage` and
`NumericUpDown` identities alongside `FramePage` and `NumberBox`. The interpreter
and the emitted Rust must create the same native types and property values. The
test host supplies the standard mock rendering and font services so path geometry
and text controls exercise their normal construction paths.

The WinUI-source fixtures first migrate the original namespace with the default
x:Bind policy, `WinUIXBindPolicy::Compile`, which keeps x:Bind as written. The
generated file is the output of
`rust_emitter::generate_file`; rustc compiles that file as part of the integration
test. Its build functions execute directly, with the same root service provider
that the runtime loader supplies. They do not call the interpreter. A freshness
test compares the checked-in file with current compiler output.

Run this focused gate:

```sh
cargo test -p ferroui-markup-xaml-winui --lib compiled_tests
```

Regenerate after reviewing relevant compiler or fixture changes:

```sh
cargo test -p ferroui-markup-xaml-winui --lib \
  compiled_tests::regenerate_winui_binding_fixture -- --ignored --exact
```

Do not hand-edit or rustfmt `winui_bindings.generated.rs`. The regeneration test
is intentionally ignored in ordinary runs because validation must not rewrite
its own expected output.

This fixture does not add support for a standalone compiled DataTemplate as a
document root. The existing emitter requires an object-model class at the root;
the typed template is tested as a real deferred property value. Generated C#/C++
fields and the deviations of the x:Bind core (docs/porting/x-bind.md, sections
3.9 to 3.12) remain the documented compatibility limits.
