# Lottie

Enable the `lottie` feature of `ferroui` to use the desktop Skia player from Rust through `ferroui::controls_lottie`. Depend directly on `ferroui-controls-lottie` with the `skia` and `markup-functions` features when compiling markup that names the control. Call `register_types()` on that crate when loading XAML at run time.

```xml
<lottie:Lottie xmlns:lottie="https://ferroui.dev/lottie"
               Path="/Assets/Animation.json" />
```

`Path` accepts a document-relative resource, an absolute `ferres://` URI, or a desktop `file://` URL. This is a JSON player for the content supported by the upstream Skottie stream loader. The gallery's Orbit and Signal animations are original, self-contained fixtures.

```sh
cargo run -p control-catalog-desktop --features lottie
```

Select **Media & Graphics → Lottie**. The opt-in build compiles Skia from source so that Skottie's text layout can share the single HarfBuzz library that FerroUI already uses for text shaping. The rust-skia binaries ship their own HarfBuzz, which cannot safely share symbols with FerroUI's version. The source build needs a C++ toolchain, Python, and Ninja. Set `SKIA_NINJA_COMMAND` if Ninja is outside `PATH`. Feature-off builds retain their existing Skia configuration.

Lottie applications built outside the FerroUI workspace need the workspace's `skia-bindings` patch. The patch shares HarfBuzz and excludes an internal libc++ header that produced invalid generated bindings on the tested Xcode toolchain.

## Source mapping

The reference is [Lottie at 94523554](https://github.com/wieslawsoltes/Lottie/tree/94523554a96cbbc0a4282ef4564b32fe84c667e5). The MIT notice is in the control crate.

| Reference | FerroUI | Status |
| --- | --- | --- |
| `src/Lottie/Lottie.cs` | `lottie.rs` | Playback, loading, layout metadata, markup members and seek helpers |
| `src/Lottie/ServiceProviderExtensions.cs` | FerroUI XAML service provider extension | Reused framework API |
| `src/CompositionAnimatedControl/CompositionAnimatedControl.cs` | `composition_animated_control.rs` | Composition lifecycle, controls, and events |
| `src/CompositionAnimatedControl/CompositionAnimatedCustomVisualHandler.cs` | `composition_animated_custom_visual_handler.rs`, `playback.rs`, `feedback.rs` | Compositor commands, timeline and dispatcher feedback |
| `src/CompositionAnimatedControl/NormalizeResult.cs` | `normalize_result.rs` | Normalized time and loop flag |
| `src/CompositionAnimatedControl/VisualPayload.cs` | `visual_payload.rs` | Typed payload and playback settings |

The standalone `AnimationControl` and `ShaderAnimatedControl` packages and their demos are deferred. The gallery demonstrates the player's transport and layout controls using bundled animations. The separate upstream demo's file picking, drag-and-drop and asset browser are outside the gallery sample.

## Behavioral differences

- Custom content implements a render-side trait and is created by a transferable factory. The control can also be instantiated directly for custom content. The source size comes back through the dispatcher, which owns the bindable properties and subscription callbacks. Delayed frame reports may combine their elapsed deltas.
- Finite playback counts skipped loops and holds the end frame. Explicit Stop draws the opening frame. Seeks clamp to the loaded duration, a zero rate freezes the current frame, and malformed numeric arguments are handled without panics.
- Source clear, failure, and unload clear the current position and duration. Reattaching reloads the current path. Failed sources report through `Status`.
- The viewport clips the full scaled destination, including crop-to-fill. The Skia lease draws on the active canvas and uses the render backend's existing retained-frame handling.

## Verification

The two Orbit reference frames at 0.00 s and 1.25 s came from the unchanged upstream player. Compared with the port's Skottie raster output at the intrinsic size, 99.917% and 99.925% of premultiplied RGBA channels were within four levels. The mean differences were 0.012 and 0.013 levels out of 255.

On macOS, the seven control tests cover load errors, exact frame positions, timing and loop boundaries, viewport clipping, transform and opacity, and the Metal lease against raster at two positions. Each channel in the Metal comparison must be within four levels of the raster reference.

See [GPU rendering](gpu-rendering.md) for renderer coverage.
