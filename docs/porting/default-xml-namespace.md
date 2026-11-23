# The default XML namespace: guard and collisions

Ground rule 7 of `PORTING-GUIDE.md`: every public type, attached property and markup
extension of a crate in this repository is usable from `https://github.com/ferroui`, with no
`xmlns:` prefix. This page records how that is checked and which names stand in the way
(#1279, tracking issue #1286).

## The guard

```sh
cargo run -p ferroui-build-scan --example xml_namespaces -- --check
```

The conventions job of CI runs it. It reads the crates in `src` and `components` that have a
`register_types.rs` with the source scanner, so a type has the namespace the build gives it,
and compares them with two lists in `scripts/conventions`:

- `xml-namespaces.txt`, written by hand: the namespaces that stay out with the reason, the
  namespaces still to map, the other XML namespaces and the names known to collide;
- `xml-namespace-prefixes.txt`, generated with `--write-prefixes`: the prefixes for mapped
  built-in namespaces that product markup and docs still have, per file.

The check fails on anything new: a public namespace that is not mapped or listed, a new XML
namespace, a new name in two namespaces, or more prefixes in a file than the list allows.
Something that went away is printed as a note and does not fail, so a change that shrinks
the work does not have to edit the lists.

`--report` prints the current collisions.

## Why a collision matters

When two namespaces mapped to one XML namespace hold the same name, the compiler takes the
type of the first assembly in the order of the type system and says nothing. Which type a
document gets can then depend on what the application links.

## Collisions

Measured with the scanner on `main` at `ea825fdfc`. "Live" means both namespaces are already
in the default XML namespace. The owner approved all four decisions on 2026-11-23.

| # | Names | Where | Live | Decision |
|---|---|---|---|---|
| 1 | `OverlayLayer`, `DragPreviewWindow` | Dock against `FerroUI.Controls.Primitives` and the drag and drop behaviors | `OverlayLayer` | Done: the framework keeps the plain name; Dock's are `DockOverlayLayer` and `DockDragPreviewWindow` |
| 2 | `DensityStyle` | One identical enum in each of the four themes | yes | Done: one shared enum in `ferroui-controls`, re-exported by each theme |
| 3 | `Chart`, `Axis`, `AreaSeries`, `LineSeries`, `DateTimeAxis`, `ChartSeriesCollection` | Charts against LiveCharts | no | Charts keeps the names; LiveCharts gets `LiveChart`, `LiveAxis` and so on, with a per-URI alias so its old XML namespace keeps the old names |
| 4 | `InfoBarPanel`, `TabViewEqualWidthPanel`, `CornerRadiusToThicknessConverter`, `CornerRadiusToThicknessConverterKind`, `SystemAccentColors`, `Application`, `ColumnDefinition` | Details of the themes and of the developer tools | no | Move their namespaces to `*.Internal`, which the rule leaves out |

These need no decision:

- `ChartAutomationPeer`: automation peers are created by their controls and are not markup.
  The automation namespaces stay out, so the Charts peer does not meet the LiveCharts one.
- The six icon types of `FerroRibbon.Compatibility` (`FontIcon`, `IconSource`,
  `FontIconSource`, `PathIconSource`, `SymbolIcon`, `SymbolIconSource`) are being removed
  with the ribbon's own icons, before that component lands.
- `ICommand`: neither of its two namespaces joins the default XML namespace.
- A generic type listed once per instantiation is one type.
