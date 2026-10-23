# Native Gallery rendering checkpoints

These are actual frames from `winui-gallery-capture`, using the existing FerroUI
headless window platform, **Skia drawing, HarfBuzz text shaping and the WinUI theme**.
The capture host rendered at **1280 × 900**, scale 1, on Linux x86_64 without a
display server. The theme selector in each frame identifies the requested theme.
The upstream Gallery is pinned to `14061eeac21c38ae99f302edf3d9136d4f652c4c`.

The checked-in frames below retain their original checkpoints. Subsequent CI on
the FerroUI 0.4.0 integration at `83623379` produced **126 light and eight dark
frames**, all decoded and verified as nonblank 1280 × 900 PNGs. Its artifact, source
revision and hashes are recorded separately in the
[integration validation record](../data/integration-validation.json). These earlier
PNGs and their manifest have not been relabeled or replaced by that CI run.

## Native reuse checkpoint: 183 verified outputs

[Run 39947560966](https://github.com/wieslawsoltes/FerroUI/actions/runs/39947560966)
at published `fe3d523e` produced **183 verified PNGs**: 126 light catalog pages,
13 representative dark pages, 16 named NavigationView views and 28 named views for
the 14 ColorPicker, ConnectedAnimation, Image and FlipView compositions. The
[immutable manifest](../data/captures/native-reuse-39947560966.json) records every
filename, byte count, hash and 1280 × 900 dimensions, plus the downloaded archive's
matching GitHub digest. The [CI ledger](../data/native-reuse-ci-validation.json)
retains the actual checkout/tree and logs, including the passing real-Skia image test.

The Image captures show the original photos, the 100-pixel decode result, four
stretch choices, fixed-size nine-grid drawings, the explicit SVG diagnostic and
static GIF previews with disabled playback controls. FlipView shows original
photos or bound image/caption content, selection indicators and navigation controls.
ConnectedAnimation shows all initial collection/card/configuration/tile compositions;
ColorPicker shows its explicit native spectrum/component/editor composition.
These observations concern initial viewports; interaction and source equivalence
remain separately bounded in the page ledger.

Visual review also found three corrections: collection row content alignment and
description ellipses, dark ColorView tab foregrounds, and the Gallery's outer scroll
position after navigating from another page. This checkpoint records the images
before those corrections. Follow-up source changes and regression tests do not
retroactively change its results. The earlier retained PNGs below remain unchanged,
and no page, API or full visual-parity status is promoted.

## NavigationView checkpoint: all eight source compositions

The native implementation is published at `9ce7530c`, its Gallery compositions at
`31eba1b6`, and named-control capture support at
[`8365ed0b`](https://github.com/wieslawsoltes/FerroUI/commit/8365ed0b33252c5169f48d56f0a9f3d511e53908).
The new [navigation capture manifest](navigation-capture-manifest.json) records
**151 actual outputs**: all **126 default light pages**, **nine default dark pages**,
and the **eight NavigationView controls in both themes**. It retains every command,
log hash, source/binary hash, PNG hash and decoded-pixel hash. Every image decoded
as opaque, nonblank 1280 × 900 content. The eighteen reviewed images below were
copied byte for byte from those runs; the earlier captures and manifest remain
unchanged.

Each of the four scopes has distinct PNG and decoded-pixel hashes internally.
There are 150 distinct images across the 151 outputs: the default light
NavigationView frame equals the named `nvSample4` frame byte for byte. The manifest
records that cross-scope duplicate explicitly. All eighteen retained images are distinct.

| Source composition | Light | Dark | Observed initial state and review boundary |
| --- | --- | --- | --- |
| Default/adaptive left, `nvSample5` | [Light](navigation/NavigationView--nvSample5-light.png) | [Dark](navigation/NavigationView--nvSample5-dark.png) | Compact rail, first-item selection, bottom Settings and SamplePage1 content. Hidden captions and missing icon glyphs limit visual identification of compact items. |
| Top navigation, `nvSample6` | [Light](navigation/NavigationView--nvSample6-light.png) | [Dark](navigation/NavigationView--nvSample6-dark.png) | Four readable labels, selected underline, right-aligned Settings, header and destination tiles. Prose extends below the inner content viewport; the frame does not establish scrolling. |
| Orientation switching, `nvSample2` | [Light](navigation/NavigationView--nvSample2-light.png) | [Dark](navigation/NavigationView--nvSample2-dark.png) | Wide-host Top state, four labels, first-item selection and Settings. The image does not establish the narrow-host transition. |
| Focus-following tabs, `nvSample7` | [Light](navigation/NavigationView--nvSample7-light.png) | [Dark](navigation/NavigationView--nvSample7-dark.png) | Item1 selected, Settings, no back toolbar, destination tiles and readable body text. Focus-following and journal behavior have separate native tests. |
| Data-bound categories, `nvSample4` | [Light](navigation/NavigationView--nvSample4-light.png) | [Dark](navigation/NavigationView--nvSample4-dark.png) | Compact first-item selection, grouping gaps, bottom Settings and destination. Category identity/template dispatch are established by tests; compact captions are hidden. |
| Footer items, `nvSample9` | [Light](navigation/NavigationView--nvSample9-light.png) | [Dark](navigation/NavigationView--nvSample9-dark.png) | Browse selected; Track an Order and Order History; Account, Your Cart and Help in the footer; Settings absent as configured. Left/Top controls are readable. |
| Hierarchy, `nvSample8` | [Light](navigation/NavigationView--nvSample8-light.png) | [Dark](navigation/NavigationView--nvSample8-dark.png) | Home selected, Account and Document options collapsed with chevrons, bottom Settings and pane-mode options. Expanded children and popup behavior are not shown. |
| API options, `nvSample` | [Light](navigation/NavigationView--nvSample-light.png) | [Dark](navigation/NavigationView--nvSample-dark.png) | Pane title, search, Actions heading, menu labels, Header and Settings are visible. No initial selection and an empty Frame match the source page. |
| InfoBadge native hosting | [Light](navigation/InfoBadge-light.png) | [Dark](navigation/InfoBadge-dark.png) | The native NavigationView composition hosts the first source badge example. This is a viewport of the page; tests separately cover all four native source compositions. |

The sixteen NavigationView captures use native `BringIntoView` through ancestor
scroll viewers. The executable checks and logs the actual control's visible
intersection, bounds and scroll offsets after layout. Every requested NavigationView
rectangle in this run lies fully within the captured viewport. Adjacent examples
can appear above or below it because the Gallery remains a normally scrolled page.
The API example's Settings row is visible near the bottom of both complete frames.

Narrow destination gray tiles in the footer/hierarchy samples follow the preserved
source `Auto,*,*` columns and the blue tile's 250-pixel minimum width beside the
320-pixel open pane and options column. The destination's own ScrollViewer retains
text below its viewport. These are observations of the source-constrained initial
layout; a static image does not prove inner scrolling or interaction.

Some menu, Settings, back, toggle and search symbols remain replacement rectangles
because this host lacks the Segoe icon fonts. No font is redistributed. These
captures do not establish WinUI template fidelity, upstream Windows pixels,
animation, resizing, keyboard traversal or Windows accessibility. All page and
control statuses remain partial.

### Reproduce named captures

```sh
cargo build --locked --offline -p winui-gallery --features capture --bin winui-gallery-capture
target/debug/winui-gallery-capture --all --query=Button --output-dir=gallery-captures/light
target/debug/winui-gallery-capture \
  --pages=NumberBox,RadioButton,AutoSuggestBox,InfoBar,InfoBadge,ProgressRing,RatingControl,CompactSizing,NavigationView \
  --dark --query=Button --output-dir=gallery-captures/dark
target/debug/winui-gallery-capture --pages=NavigationView --control=nvSample8 \
  --dark --output-dir=gallery-captures/dark
```

Repeat the last command for all eight names above with and without `--dark`.
Named captures require exactly one explicit page and cannot be combined with
`--all`. The workflow now runs all sixteen named captures in addition to its
default-page sweeps. These recorded local invocations used the isolated Rust 1.99.0
toolchain and real renderer; their exact host paths and hashes are in the manifest
and [checkpoint record](../data/navigation-validation.json).

## Retained checkpoint: native feedback controls and CompactSizing

The Gallery sources for this checkpoint are published at
[`3c7ceb81`](https://github.com/wieslawsoltes/FerroUI/commit/3c7ceb81dcc4948f374c05c4757c6cb72ed5de5d).
Native controls are recorded at `9501f091`, opt-in XAML migration at `f50450d0`, and
the compiled WinUI theme at `a9ede2a5`. The
[capture manifest](capture-manifest.json) records the exact source hashes, binary and
log hashes, every frame ID/hash from the **126-page light sweep** and **nine-page dark
sweep**, and the eight reviewed PNGs retained below.

| Page | Retained light frame | Retained dark frame | Visible content and evidence boundary |
| --- | --- | --- | --- |
| InfoBadge | [Light](InfoBadge-light.png) | [Dark](InfoBadge-dark.png) | Icon/value/dot compositions, severity selector, Refresh composition and dynamic-value input. Some icon glyphs are unavailable on this host. |
| ProgressRing | [Light](ProgressRing-light.png) | [Dark](ProgressRing-dark.png) | Indeterminate and determinate native controls with activation, background and value options. A static frame does not verify motion. |
| RatingControl | [Light](RatingControl-light.png) | [Dark](RatingControl-dark.png) | Caption and placeholder examples with clear/read-only options. The rating stars appear as missing-glyph boxes. |
| CompactSizing | [Light](CompactSizing-light.png) | [Dark](CompactSizing-dark.png) | Standard/Compact selector and the initial **Standard** form. These frames do not exercise switching to compact sizing or retaining entered values. |

All eight frames were visually reviewed for construction, layout and rendering.
**All four pages remain partial.** The Linux host does not provide the required
Segoe Fluent Icons / Segoe MDL2 Assets glyphs: rating stars and some icons display
missing-font boxes. The API/glyph values are retained; complete cross-platform font
presentation remains pending. The font is not redistributed. See the existing
[WinUI theme notice](../../../../src/FerroUI.Themes.WinUI/NOTICE.md).

### Exact commands

The following environment and commands produced this checkpoint, from the repository
root. Paths identify the recorded host's toolchain/cache and output directories;
another environment can use its own paths. `--offline` requires cached dependencies.

```sh
export RUSTUP_HOME=/tmp/ferroui-rustup
export CARGO_HOME=/tmp/ferroui-cargo
export PATH="/tmp/ferroui-cargo/bin:$PATH"
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export SKIA_BINARIES_URL=file:///tmp/ferroui-skia.tar.gz

cargo run --locked --offline -p winui-gallery --features capture --bin winui-gallery-capture -- \
  --pages=InfoBadge,ProgressRing,RatingControl,CompactSizing \
  --output-dir=/tmp/ferroui-gallery-current-light

target/debug/winui-gallery-capture \
  --all --query=Button --output-dir=/tmp/ferroui-gallery-current-all

target/debug/winui-gallery-capture \
  --pages=NumberBox,RadioButton,AutoSuggestBox,InfoBar,InfoBadge,ProgressRing,RatingControl,CompactSizing,WebView2 \
  --dark --query=Button --output-dir=/tmp/ferroui-gallery-current-dark
```

The local Skia archive is the public dependency artifact documented in
[the sample validation ledger](../../../../samples/WinUIGallery/VALIDATION.md), with
SHA-256 `9ebe4c448cc9f789bad241cbd11739cdf759e27db5719c23a7635e096d5807b3`.
The captured executable's SHA-256 is
`f59fb8575e5342777aa0089f7b4402620c44cf6937b4bb52d834a274a758f4e6`.

| Run | Measured result |
| --- | --- |
| Selected light run used for retained images | Four decoded, nonblank 1280 × 900 PNGs; four distinct hashes. |
| Full light sweep | All 126 catalog pages completed; 126 decoded, nonblank 1280 × 900 PNGs; 126 distinct hashes. |
| Selected dark sweep | All nine requested pages completed; nine decoded, nonblank 1280 × 900 PNGs; nine distinct hashes. |

Validation decoded every output and checked that at least one RGB channel had pixel
variation. Logged IDs matched the output files. The capture host rejects native
adaptation construction errors; `--query=Button` also rejects failure to decode the
selected original AutoSuggestBox detail PNG through the real renderer.

The retained light images come from the separate four-page run, **not** the full light
sweep. Their digests are recorded separately: three differ from the same-page sweep
images. These captures are observations of those runs, not deterministic image hashes
that a rebuild must reproduce.

| Capture/build log | SHA-256 |
| --- | --- |
| `/tmp/ferroui-capture-build.log` | `a3bc806a0cf7f1a0f8e7943fe01b0b95251b16513feca60273b80793ad38c8c1` |
| `/tmp/ferroui-capture-all.log` | `acb9ec40532bdecd10b017f05660ffb15cc1660a2872e710964cd98f2628e760` |
| `/tmp/ferroui-capture-dark.log` | `b81d2a803c792c0377596c800df67f3f5772df864344877b47657d94602aea01` |

The other sweep images, executable and logs are temporary artifacts; their hashes and
frame IDs are preserved in the manifest. The checked-in theme reference pointer is
not a production input: the manifest separately hashes the actual generated theme
files referenced by the capture executable's dependency file.

### Current retained-image hashes

The PNGs were copied byte for byte from the original capture output. No resizing,
retouching or image conversion was applied.

| File | SHA-256 |
| --- | --- |
| `InfoBadge-light.png` | `9ae63ab11fe04eccf3e5ee281288319db7ae83f7d3b2f89f878e28d1fd29b441` |
| `InfoBadge-dark.png` | `d515f3d3f207d5d77fad94498a92f60c97f681f8e48f92f759db9b4060e77e90` |
| `ProgressRing-light.png` | `14445302cfba279365dd53ab286c857b102f017c4bdd46b93270260ddd2b8812` |
| `ProgressRing-dark.png` | `5f5d3ecd3c6ed67af9ec9c197cc77fd2f53186ab16216cfe031cd89b9ad71338` |
| `RatingControl-light.png` | `646758eb13723257e08beb2aba77e798094eb2759b295f42b7d28caee5047e3a` |
| `RatingControl-dark.png` | `ffd42ba03be96b9a72a404d57b8317c03d6ae6f1a3f0a6e5b02207d16ef1d934` |
| `CompactSizing-light.png` | `5e19ba3f771e2d77152414b30a79ea8af65893b8150ca3fbe21eb5bdd7ccc5ec` |
| `CompactSizing-dark.png` | `025405e151d17339c6f2e92b009d72a17082bb8c5617af2218e1bf92de9d52b0` |

## Historical checkpoint: `d008845`

The following five images are **unchanged historical captures from `d008845`**. They
are retained as evidence of that earlier checkpoint and do not show the current
feedback-control or CompactSizing implementation.

| Frame | Content exercised at the historical checkpoint | Page state |
| --- | --- | --- |
| [NumberBox, light](NumberBox-light.png) | Native WinUI NumberBox inputs, inline spin buttons, expression input and value output. | Partial |
| [AutoSuggestBox, light](AutoSuggestBox-light.png) | Both native source examples; the second submitted `Button` and decoded the original embedded control PNG through the native asset loader. | Partial |
| [RadioButton, dark](RadioButton-dark.png) | Native option group, color groups and the original selected background/border composition. | Partial |
| [InfoBar, dark](InfoBar-dark.png) | All three native source compositions, severity/message/action controls and close/icon/open options. | Partial |
| [WebView2, light](WebView2-pending-light.png) | Navigation and source inspection for a page whose native platform control remained pending. No WebView was instantiated. | Pending |

At that checkpoint all 126 light-theme pages and five selected dark-theme pages
completed rendering. Every light-sweep output decoded as a nonblank 1280 × 900 PNG,
with 126 distinct hashes. The AutoSuggestBox Find symbol and InfoBar status/close
glyphs showed missing font boxes on the same kind of Linux host.

### Historical commands

These commands belong to `d008845`; `CARGO_TARGET_DIR=/tmp/ferroui-gallery-target`
selected that checkpoint's build directory. The constrained build used one Cargo job,
disabled incremental compilation/development debug information, and the public Skia
archive listed above.

```sh
cargo run -p winui-gallery --features capture --bin winui-gallery-capture --offline -- \
  --pages=NumberBox,AutoSuggestBox,RadioButton,WebView2 --query=Button \
  --output-dir=/tmp/ferroui-gallery-captures

/tmp/ferroui-gallery-target/debug/winui-gallery-capture \
  --all --query=Button --output-dir=/tmp/ferroui-gallery-captures-all

/tmp/ferroui-gallery-target/debug/winui-gallery-capture \
  --pages=NumberBox,AutoSuggestBox,RadioButton,InfoBar,WebView2 --query=Button --dark \
  --output-dir=/tmp/ferroui-gallery-captures-dark
```

### Historical retained-image hashes

| File | SHA-256 |
| --- | --- |
| `NumberBox-light.png` | `c83f75bf391c8615f1a01ca273d0974f29b81b32d665ed7b29968ae81545b0c0` |
| `AutoSuggestBox-light.png` | `6d9a74c9dffabd54e5558672c84d5914f6a38021ae721608ceb1e0da4ed38056` |
| `RadioButton-dark.png` | `80ce8e13ab197536311262ac2c71727bdc15e19976177c7fdf8b41aea9e8e639` |
| `InfoBar-dark.png` | `5c3685c0852008681233967db1e9c2cc0ec8403637877dd8a45f374046232da0` |
| `WebView2-pending-light.png` | `3ed745bad107abe566bb6879c076cb6491df49a2e65bca4a158dd07586df4ec9` |

## Acceptance boundary

These checkpoints provide **construction/rendering evidence only**. They do not
establish complete page acceptance, upstream Windows pixel parity, platform UIA or
screen-reader interoperability. Rendering a pending page's source browser does not
instantiate or implement its missing platform control. No page/API status is promoted
by these screenshots; source-example behavior tests and the explicit parity ledgers
remain separate evidence.
