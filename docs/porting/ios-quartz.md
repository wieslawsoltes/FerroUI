# Native iOS rendering and text

The opt-in iOS 16+ backend now reuses `ferroui-quartz`: Core Text shaping, font lookup,
glyphs and metrics; Core Graphics geometry operations; ImageIO decoding/encoding; and
Metal drawing, effects, retained layers and presentation. UIKit windowing, input, accessibility,
safe areas and CADisplayLink remain in `ferroui-ios`. There is no copied iOS renderer fork.

This is a working prototype validated in the iPhone Air / iOS 27.0 simulator using Xcode 27.2.
It is not physical-device certification. See [recorded evidence](ios-quartz-results.json).

## Selection

| Feature of `ferroui-ios` / `control-catalog-ios` | Renderer and text |
|---|---|
| `skia` (default) | Existing Skia / HarfBuzz |
| `quartz-metal` | Shared native Metal / Core Text; no runtime WGSL compiler |
| `quartz-runtime-shaders` | Native backend plus optional Naga WGSL-to-MSL compiler |

Use `--no-default-features` for native-only builds. Quartz takes precedence if both backend
features are enabled, but enabling Skia too keeps its dependencies in the build. The existing
`use_ios()` and `use_ios_with_delegate()` application APIs select the configured backend.
Native rendering accepts Metal surfaces only; it never falls back to OpenGL or a CPU framebuffer.
The other workspace iOS hosts explicitly retain their original Skia selection.

```sh
IPHONEOS_DEPLOYMENT_TARGET=16.0 cargo build --locked -p ferroui-ios \
  --example ios_view --target aarch64-apple-ios-sim \
  --no-default-features --features example,quartz-metal

IPHONEOS_DEPLOYMENT_TARGET=16.0 cargo build --locked -p control-catalog-ios \
  --target aarch64-apple-ios-sim \
  --no-default-features --features quartz-runtime-shaders
```

Use `aarch64-apple-ios` for a physical-device build; installation requires the application's
own signing/provisioning. Minimum iOS 16 follows the public Core Graphics path-boolean APIs.
The build links the appropriate Xcode Clang availability runtime independently of Skia.

## GPU portability

The macOS implementation originally required RGBA8 read/write texture tier 2. The simulator
reports a lower tier, so the shared backend now selects a separate-destination Metal kernel
when needed. It copies the affected destination rectangle into a pooled private GPU texture,
then reads that texture while writing the target. Clip-mask generation and full clears avoid
that copy. Tier-2 devices retain the original in-place path. Both use the same compositing
math; no CPU pixel transfer is introduced. The compatibility path costs GPU bandwidth and
needs device performance measurements before making mobile performance claims.

The simulator SDK does not provide MPS. Simulator blur uses the existing two-pass Metal
Gaussian kernel; macOS and physical iOS builds retain the system MPS path for supported
axis-aligned blur. The device SDK syntax-checks both native Objective-C files, including MPS.
Physical GPU behavior remains unverified.

For regression testing on a tier-2 Mac, `FERROUI_QUARTZ_FORCE_SEPARATE_DESTINATION=1` forces
the portable destination path. It is a development override; normal selection uses the Metal
capability. It does not force the simulator-only blur branch.

## Review fixes shared with iOS

The relevant fixes from `origin/render/quartz-native-prototype` (`147d1ff1f`,
`21925eacc`) are adapted into the shared Apple implementation. Missing graphics contexts or
Metal features now produce an unavailable context and a not-ready render target. Oversized
or failed GPU allocations return errors; APIs without an error return draw transparently.
Metal initialization/presentation failures return failure rather than asserting. Objective-C
exceptions are contained on the native side of the C ABI; a caught Metal exception invalidates
the context generation so the compositor can recreate its queue and caches. The temporary
image-upload surface is checked before it can reach `CFRelease`.

This port retains the simulator's separate-destination GPU path, native hinted/color glyph
cache, ranged OpenType shaping, and exact writable bitmap storage for BGRA8888, RGBA8888 and
RGB565. Embedded family lookup keeps the actual face and its font bytes, with directional CSS
weight matching. Acrylic's noise asset and attribution now live inside the Quartz crate, and
failure to allocate its GPU image no longer panics. Shader-source embedding accepts CRLF files.

Metal builds remain GPU-only. They do not adopt the review branch's CPU framebuffer fallback
or its tier-2-only device requirement. A mismatched device is rejected without throwing; this
port does not add the review branch's macOS multi-GPU renderer cache. iOS uses one Metal device.
Cargo's package file list includes the local asset; full package verification still requires
publishing the workspace's `ferroui-base 0.5.0-preview.1` dependency.

Regression coverage includes missing Metal, unsupported target dimensions, embedded Regular/Bold
family resolution and original font bytes. An isolated native fixture throws an Objective-C
exception and separately fails a temporary image allocation: both invalidate the generation,
and recovery is verified by rendering red/blue pixels on fresh GPU contexts. These fixtures
require `media-test-fixtures` and are absent from normal iOS builds. The CPU suite passes
**25 tests**. Device and simulator SDKs syntax-check both native files; simulator smoke adds
missing-Metal and oversized-allocation checks alongside its normal presentation assertions.

## Validation

The small iOS example passes **41 checks**, including GPU presentation, 1260×2736 frame
pixels at 3× scaling, visible text, safe areas, UIKit text responder focus, insertion,
selection, marked text, deletion/replacement, clipboard and accessibility. It calls the
system text-input responder as the keyboard does; this does not certify physical touch or
hardware keyboard delivery. One explicit platform-frame capture supplies pixel assertions.
The Quartz counters separately report zero renderer readbacks and zero GPU errors.

ControlCatalog passes **six pages**: Renderer parity, TextBox, TextBlock, Visuals, Effect
brushes and Shader effects. The parity action passes Arabic selection, text hit testing,
replacement and undo; screenshots include Latin, Arabic, Hebrew, CJK and an upright color
emoji. Blur and runtime shaders render on the GPU. Desktop-sized fixture canvases can be
horizontally clipped by the phone viewport; these captures are not an audit of responsive
catalog layout or every catalog interaction.

The macOS graphics suite passes **61 tests on each destination path**, covering blending,
clipping, geometry, bitmap formats, font metadata, OpenType features, glyph orientation,
effects, snapshots and texture interop. The simulator validates actual presentation as well
as the shared numerical references exercised on macOS.

```sh
CARGO_BUILD_JOBS=1 IPHONEOS_DEPLOYMENT_TARGET=16.0 \
  scripts/ios/sim-smoke.sh 'iPhone Air' --backend quartz-metal --keep

CARGO_BUILD_JOBS=1 IPHONEOS_DEPLOYMENT_TARGET=16.0 \
  scripts/ios/sim-catalog.sh 'iPhone Air' --backend quartz-runtime-shaders --keep \
  --pages 'Renderer parity,TextBox,TextBlock,Visuals,Effect brushes,Shader effects' --page-ms 6000

FERROUI_QUARTZ_FORCE_SEPARATE_DESTINATION=1 cargo test --locked -p ferroui-quartz \
  --features metal,runtime-shaders,media-test-fixtures --lib -j 1 -- --test-threads=1
```

The scripts preserve the default Skia behavior, use separate native app identifiers and
artifact directories, and support `--no-build`. The bundler now accepts `--no-default-features`.
The native catalog runner requires presented frames and zero GPU errors, and the Renderer
parity action when that page is selected; screenshots alone are not a pass.

Normal dependency graphs were checked for both native variants: neither includes Skia,
HarfBuzz, wgpu or Vello. The small example excludes Naga; the runtime-shader catalog includes
it deliberately. Debug executable sizes are not a paired release-size benchmark.

## Next validation

Physical-device installation/performance and memory-pressure behavior, background/foreground
and rotation coverage, iOS video/camera acquisition, the newer 3D drawing contract, and complete mobile feature parity remain
future work. The shared Metal frame-import contracts are wired, but this prototype does not
claim real iOS camera/video validation. A comparable minimal-app release benchmark is also
still needed. Existing macOS size reports remain measurements of their recorded commits.
