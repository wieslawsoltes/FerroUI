# Windows ControlCatalog generated factory inventory

The integrated bytecode build emits 173 of 203 class populate bodies as a single `ObjectDocumentProgram` (85.2%), up from 161 (79.3%) before the staged-setter change. In both bytecode arms, two further classes, `Pages/SettingsPage.xaml` and `Pages/TabbedPage/TabbedPageCustomTabBarPage.xaml`, use construction regions and typed resource helpers throughout. Classes that still contain additional native statements fell from 40 to 28. All four standalone dictionary build bodies retain native deferred-resource registration.

| Deferred factory body | Native functions | Before functions | Integrated functions | Native sites | Before sites | Integrated sites |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Construction regions and generated glue only | 0 | 174 | 176 | 0 | 169 | 171 |
| Shared helper only | 54 | 54 | 54 | 45 | 45 | 45 |
| Construction regions with native residuals | 0 | 41 | 39 | 0 | 33 | 31 |
| Native residuals without construction regions | 236 | 21 | 21 | 223 | 21 | 21 |
| Total | 290 | 290 | 290 | 268 | 268 | 268 |

At 171 of 268 distinct sites (63.8%), construction programs and their calling glue account for the entire deferred body. Another 45 sites delegate to `TypedStyleProgram.build`, bringing the total with no additional native body statement to 216 (80.6%). The native arm delegates those same 45 sites to `build_control_theme`. Counting each generated function includes two emissions of `CustomThemes.xaml` with 22 repeated deferred sites, one in `compiled_app.rs` and one in `compiled_custom_themes.rs`. Both copies classify alike. The four dictionary bodies represent three distinct documents.

## Newly complete class bodies and changed factories

Each of these class bodies previously retained a native `new_setter` or `new_setter_with_parent` call. The table names the property of its first such call. Each integrated populate body runs one whole-document program.

| Class XAML under `Pages/` | First previously retained setter property |
| --- | --- |
| `AcrylicPage.xaml` | `Layoutable.Height` |
| `CursorPage.xaml` | `InputElement.Cursor` |
| `FlexPage.xaml` | `Flex.AlignSelf` |
| `FocusPage.xaml` | `XYFocus.UpNavigationStrategy` |
| `ListBoxPage.xaml` | `Control.ContextFlyout` |
| `MenuPage.xaml` | `HeaderedSelectingItemsControl.Header` |
| `PipsPager/PipsPagerCustomTemplatesPage.xaml` | `TemplatedControl.Template` |
| `SliderPage.xaml` | `ToolTip.Tip` |
| `TabStripPage.xaml` | `InputElement.IsEnabled` |
| `TabbedPage/TabbedPageFabPage.xaml` | `ItemsControl.ItemsPanel` |
| `TabbedPage/TabbedPageFluidNavPage.xaml` | `TemplatedControl.Template` |
| `WrapPanelPage.xaml` | `TextBlock.FontWeight` |

Two deferred factory bodies changed. Both previously split construction over seven regions and retained a native setter for `TemplatedControl.Template`. Each now runs one construction region with generated glue. Comparing all 290 generated bodies with each instruction-image offset replaced by its referenced bytes found the same two differences.

| XAML site | Generated function |
| --- | --- |
| `ClassicShell/Fluent.xaml(815,14)` | `build_classicshell_fluent_xaml_deferred_98` |
| `ClassicShell/Simple.xaml(793,14)` | `build_classicshell_simple_xaml_deferred_96` |

## First retained calls

In both bytecode arms, in sorted module order and generated definition order, the first class residual is `App.xaml`'s `ResourceInclude` of `CustomThemes.xaml`. The generated `populate_app_xaml` calls `build_customthemes_xaml` after casting the service provider at (11,26), at line 59 of `compiled_app.rs`. The first deferred factory residual is `App.xaml(38,8)`, the `StyleInclude` keyed `ColorPickerFluent`. At line 324 before the change and line 294 after it, `populate_app_xaml_deferred_3` casts the provider at (38,47) and calls the color-picker crate's `build_themes_fluent_fluent_xaml` directly.

## Inputs and reproduction

All three inputs use the Windows `control-catalog` crate's own generated modules. The native dist, before-change bytecode dist and integrated bytecode debug outputs each contain 206 module groups and the same 497 module-qualified class, build, and deferred function identities. The TSV records the first residual statement and generated line for each body, following generated split parts in call order. The scan recognizes the emitter's construction calls, typed table calls, and glue for context, namescope, imports, register extraction, and simple returns. Unrecognized statements count as residuals. Imported Rust callbacks, helper implementations, dependency crates, linked bytes, and runtime execution frequency are outside this source-level measurement.

Run the scanner with Python 3.9 or later against the three `out/xaml` directories. Each directory's generated `mod.rs` identifies the groups to scan.

```sh
python scripts/themes/full-opcode-spike/factory_inventory.py --input native=PATH_TO_NATIVE_OUT_XAML --input before=PATH_TO_BEFORE_BYTECODE_OUT_XAML --input bytecode=PATH_TO_INTEGRATED_BYTECODE_OUT_XAML --output windows-catalog-factories.tsv.gz
```

| Input | Cargo output group | SHA-256 over sorted module filenames and source hashes |
| --- | --- | --- |
| Native, dist | `control-catalog-e6fd4e7b102dc02b` | `07544ed3b71834f98c5b54478c7e353d3762d4aee838e192fa9c57bc4afe20a9` |
| Before-change bytecode, dist | `control-catalog-e6fd4e7b102dc02b` | `c8f90d1551e9f50f7067d7fe64f163d05aa488807108d9d7e5362bd118bd52e6` |
| Integrated bytecode, debug | `control-catalog-a646050f14dcc4bd` | `39298cdd5ab628abc3910ca39e272a665a65f74451659c670a869f924aebd3e1` |

[Per-function inventory for all three arms](windows-catalog-factories.tsv.gz)
