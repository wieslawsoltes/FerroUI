# Combined XAML and typed bindings in ControlCatalog

This experiment crosses the frozen native or bytecode XAML backend with stock or shared typed bindings. The four macOS arm64 executable sizes come from full Skia ControlCatalog builds of the same workspace, using its existing `dist` profile. `source.json` gives the revision, the SHA-256 of the decompressed patch, and the hashes of all 43 modified or added files. The snapshot includes the first whole-class document programs and predates the later change excluding boolean literals from object operands.

| XAML backend | Typed bindings | Complete file, bytes | Typed binding instructions, bytes |
| --- | --- | ---: | ---: |
| Native | Stock | 65,859,488 | 4,514,652 |
| Native | Shared | 63,839,040 | 2,684,264 |
| Bytecode | Stock | 61,562,624 | 4,515,728 |
| Bytecode | Shared | 59,542,176 | 2,686,720 |

The two changes together saved **6,317,312 bytes (9.59%)**. Shared bindings saved **2,020,448 bytes** with either backend. Typed XAML saved **4,296,864 bytes** with either binding implementation. These four linked files show a zero-byte interaction in total file size for this snapshot. The linker assigned individual instruction families slightly differently across arms. The earlier native-only executables came from a different source graph, and the native stock file here is 16 bytes smaller.

The two bytecode arms shared identical hashes for all generated Catalog and WinUI Rust sources and the same full-graph program counts. Across 241 generated modules the emitter wrote 545,636 image bytes and 24 whole-class document programs, including 23 in Catalog. The archived census counts the WinUI trimming wrapper among 223 public population functions, corresponding to 222 class documents. The newer repository census excludes the wrapper directly.

With the WinUI Light theme, all four executables rendered identical 1900×1360 framework images for Buttons, ListBox, Data Validation and TreeDataGrid. All 12 comparisons with native stock had zero differing pixels. This smoke covers initial rendering and page navigation. The [binding experiment](../README.md) separately measures update propagation, startup, navigation and draw time for native XAML. The combined experiment records file size and output equivalence.

[Four allocations, binary and map hashes, and generated program counts](results.json) · [Per-page pixel hashes](smoke-samples.json) · [Frozen source file hashes](source.json)

## Reproduce the four builds

Run these commands from the repository root on macOS arm64 with Xcode and the repository's Skia prerequisites available. Set `experiment` to a new directory on a volume with room for a full build. The optional `SKIA_NINJA_COMMAND` can point at the existing Ninja executable in the local SDK.

```sh
set -eu
experiment=/absolute/path/to/new/combined-comparison
mkdir -p "$experiment"
origin=$PWD
support="$origin/scripts/perf/typed-binding-size"
base=933fd46325772fd1cdec81add9d4912dda2fe931
pr=8678e15b8ae738931237c0e70ef66f61a6ecbfd2
binding_file=src/FerroUI.Base/data/core/typed_binding_expression.rs
git worktree add --detach "$experiment/workspace" "$base"
cd "$experiment/workspace"
gzip -dc "$support/combined-xaml/xaml-source.patch.gz" | git apply -
export CARGO_TARGET_DIR="$experiment/cargo"
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
for arm in typed combined frozen-native-binding frozen-native; do
    case "$arm" in
        typed) export FERROUI_XAML_BACKEND=bytecode ;;
        combined)
            git show "$pr:$binding_file" > "$binding_file"
            git apply "$support/controller.patch"
            ;;
        frozen-native-binding) export FERROUI_XAML_BACKEND=native ;;
        frozen-native) git show "$base:$binding_file" > "$binding_file" ;;
    esac
    cargo rustc --locked --profile dist -p control-catalog-desktop --bin control-catalog-desktop -- \
        -C "link-arg=-Wl,-map,$experiment/$arm.map"
    cp "$CARGO_TARGET_DIR/dist/control-catalog-desktop" "$experiment/$arm"
    python3 "$origin/scripts/themes/full-opcode-spike/catalog_allocation.py" \
        "$experiment/$arm" "$experiment/$arm.map" > "$experiment/$arm-allocation.json"
    python3 "$origin/scripts/themes/full-opcode-spike/application_image.py" \
        "$CARGO_TARGET_DIR/dist/control-catalog-desktop.d" > "$experiment/$arm-images.json"
done
python3 "$support/combined-xaml/smoke.py" "$experiment" "$experiment/captures" \
    frozen-native frozen-native-binding typed combined
```

`typed` uses stock bindings with bytecode XAML, `combined` adds shared typed bindings, `frozen-native-binding` keeps shared bindings with native XAML, and `frozen-native` uses native XAML with stock bindings. The driver applies the same XAML snapshot in each arm. An APFS copy-on-write copy of the existing Cargo cache was used for the measured builds. Absolute file paths can affect Mach-O C strings, and a page boundary can amplify that difference in the file size. Compare arms within the same worktree and cache.
