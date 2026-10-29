# macOS 26 native reference capture

`capture-native.swift` measures real AppKit controls and captures their appearance on a Mac running **macOS 26**. It refuses other major OS versions so later system changes cannot silently become macOS 26 reference data. Build it using the macOS 26 SDK or later.

```sh
swift scripts/macos-theme/capture-native.swift --output /tmp/ferroui-macos26
```

The output directory contains `native-reference.json` and four view snapshots: light/dark appearance with active/inactive windows. The JSON includes the precise OS version, backing scale, system accessibility preferences, resolved semantic sRGB colors, control frames, intrinsic/fitting sizes, alignment insets, and font metrics. Every control uses the system's requested mini, small, regular, large, or extra-large size; if AppKit does not offer a distinct size for a particular control, the recorded measurements expose that fact.

The fixture covers push/default/disabled buttons, checked/mixed checkboxes, radio buttons, switches, text/search fields, pop-up buttons, segments, sliders, steppers, native titlebar controls, and a native `NSGlassEffectView`.

## Capturing compositor output

`NSView.cacheDisplay` is useful for geometry and basic control rendering, but is not sufficient evidence of Liquid Glass optical fidelity. For WindowServer output, run:

```sh
swift scripts/macos-theme/capture-native.swift --output /tmp/ferroui-macos26 --screen-capture
```

This invokes the system `screencapture` tool for only the fixture window. macOS may require Screen Recording permission for the invoking terminal. The script reports capture failure rather than claiming a missing image was produced. Native captures can contain wallpaper or nearby content reflected by the material; inspect captures before committing them.

## Comparison matrix

Repeat the probe under the required display scaling, accent, Increase Contrast, Reduce Transparency, and Reduce Motion settings. The probe reads those settings and does not change them. Compare captured native metrics with FerroUI rendered output at the same scale and font configuration. Do not compare a 1× fallback-font Linux capture directly with a 2× SF Pro Mac reference and interpret the result as a theme-only difference.

Open-menu behavior, sheet transitions, hover/pressed sequences, backdrop-dependent optics, and animation trajectories require additional native captures. Passing XAML/resource tests or this probe's successful execution does not establish 100% visual or motion equivalence.

Apple reference: [Build an AppKit app with the new design (WWDC25)](https://developer.apple.com/videos/play/wwdc2025/310/). The theme implementation and remaining verification scope are documented in [macOS 26 theme port](../../docs/porting/macos26-theme.md).

## CI evidence

[The macOS theme workflow](../../.github/workflows/macos-theme.yml) runs the source audit, the resource-key check, the complete theme test suite, the rendered state matrix and checks the catalogue with its optional `macos-theme` feature on Linux. A manual workflow run can additionally build the desktop catalogue. A separate job compiles and runs this fixture on GitHub's explicit `macos-26` runner and uploads the native metrics, view captures, toolchain details, and logs. The fixture and workflow both verify the OS major version before accepting images. The [runner label is documented by GitHub](https://github.com/actions/runner-images/issues/13739).

The native reference job does not request Screen Recording permission or claim a pixel comparison. Its uploaded artifact is reference evidence for subsequent visual calibration. A separate macOS 26 job compiles the generated interop code and Objective-C++ backend and tests automatic accessibility preferences, compatibility with older native libraries, and subscription disposal.

The Linux contract job also compiles `native-interop-smoke.cpp` against a freshly generated native header. It checks the literal public UUID bytes, discovery of both settings interfaces, shared COM identity/reference counts, and callback lifetime during unregistration. This catches interop defects independently of AppKit; the macOS job still validates the actual native implementation.
