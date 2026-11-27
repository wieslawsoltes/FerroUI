# How much does generated XAML retain in ControlCatalog?

A macOS arm64 diagnostic build omitted the population bodies from 585 generated functions in 228 modules across the desktop application's dependency graph. The experiment kept root constructors, registrations, assets, native code-behind, framework features and the ordinary Cargo dist profile. The static linker could discard deferred factories, resources and helpers used only by the removed operations.

| Measurement | Stock | Population omitted | Decrease |
| --- | ---: | ---: | ---: |
| Final executable, Apple stripped and ad-hoc signed | 65,493,168 B | 54,006,752 B | 11,486,416 B |
| Instructions attributed directly to generated XAML | 7,997,836 B | 51,988 B | 7,945,848 B |
| Instructions attributed to typed binding expressions | 4,514,656 B | 4,514,456 B | 200 B |
| Embedded assets matched byte-for-byte | 3,799,711 B | 3,799,711 B | 0 B |

The 11.49 MB decrease includes code and data reachable only through a XAML operation. A functioning implementation still needs its runtime, the instructions, typed imports, and the framework operations that the UI calls. The diagnostic binary has empty pages and cannot be used to measure UI behavior or startup. It estimates the opportunity from replacing population within this application and its current architecture. Architectural changes to bindings or controls can affect the remaining 54.01 MB independently. The map attributes code by its linked symbols, so inlining can change a category.

The stock file before the normalization used here was 65,876,016 B. Both files in this comparison received the same final strip and ad-hoc signature. See [the two reconciled allocations](catalog-omission.json) and the [stock allocation in the original signing convention](catalog-allocation.json).

## Reproduction

On macOS arm64 use two empty, independent Cargo target directories with the same source, toolchain, features and SDK. Keep the output of the omission wrapper isolated since its generated files intentionally lose their XAML population bodies. Use the same final binary name in both directories.

```sh
export FERROUI_XAML_BACKEND=native
export CARGO_TARGET_DIR="$(mktemp -d)"
cargo rustc --locked --profile dist -p control-catalog-desktop -- -C link-arg=-Wl,-map,"$CARGO_TARGET_DIR/catalog.map"
cp "$CARGO_TARGET_DIR/dist/control-catalog-desktop" "$CARGO_TARGET_DIR/catalog"
strip "$CARGO_TARGET_DIR/catalog"
codesign -f -s - "$CARGO_TARGET_DIR/catalog"
python3 scripts/themes/full-opcode-spike/catalog_allocation.py "$CARGO_TARGET_DIR/catalog" "$CARGO_TARGET_DIR/catalog.map"

export CARGO_TARGET_DIR="$(mktemp -d)"
export ATTRIBUTION_TARGET="$CARGO_TARGET_DIR"
export RUSTC_WRAPPER="$PWD/scripts/themes/full-opcode-spike/omit_xaml_population.py"
cargo rustc --locked --profile dist -p control-catalog-desktop -- -C link-arg=-Wl,-map,"$CARGO_TARGET_DIR/catalog.map"
cp "$CARGO_TARGET_DIR/dist/control-catalog-desktop" "$CARGO_TARGET_DIR/catalog"
strip "$CARGO_TARGET_DIR/catalog"
codesign -f -s - "$CARGO_TARGET_DIR/catalog"
python3 scripts/themes/full-opcode-spike/catalog_allocation.py "$CARGO_TARGET_DIR/catalog" "$CARGO_TARGET_DIR/catalog.map"
```

The linker map is diagnostic. Build paths, compiler and signing-tool versions can affect individual section sizes.
