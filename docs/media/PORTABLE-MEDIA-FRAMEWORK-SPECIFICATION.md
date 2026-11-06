# Portable media, camera, QR and 3D texture framework specification

Draft for a new FerroUI implementation. 31 October 2026.

This document specifies observable behavior, composition, ownership and platform
techniques. Section 12 records the selected public API responsibilities and
framework defaults. Method signatures, namespace layout and source organization
follow idiomatic Rust and existing FerroUI conventions, including its WinUI-style
additions. The normative framework decisions in section 12 supersede application-specific
profiles and open choices elsewhere in this document. Historical implementation
evidence remains reference material, not acceptance of the new implementation.

## 1. Product objective and requirements boundary

Provide one portable GPU texture embedding foundation. Camera preview, video
playback and a host for a 3D renderer build on that foundation. QR recognition
is an optional consumer of camera frames and metadata. A camera remains useful
without QR, and embedding remains useful without any camera or media decoder.

**GPU texture sharing/import is the primary presentation path. CPU frame copies
are permitted as a fallback, never the primary design.** Select a valid GPU path
whenever one is available. In GPU mode, the framework must not read frames back
to the CPU, copy them through CPU staging buffers, serialize pixel arrays across
runtime boundaries, or upload a CPU copy again to present them. A required
GPU-to-GPU copy/conversion must be identified and measured. It is not a CPU copy
and must not be described as proof of no copies anywhere in the operating
system, decoder, browser or driver.

The target platforms are Windows, macOS, iOS, Android, Linux and the browser.
Support is declared for an actual combination of platform, renderer, device,
format and operating-system version after validation. A platform name alone is
not a capability claim. The default GPU-preferred policy may select a declared
CPU fallback when no valid GPU route exists, with an inspectable reason, copy
counters and performance limits. Callers can instead require GPU-only operation,
which reports unsupported/error rather than falling back. Do not label a CPU
path as GPU or silently choose it over a valid GPU path. Functional support
through fallback and readiness of a primary GPU implementation are separate claims.

The first product scope includes file/resource/network video playback with
audio; live camera preview, still capture where supported and device selection;
QR detection, geometry and application feedback; arbitrary texture producers;
and presentation of a 3D renderer's output inside normal UI composition. Camera
and video frames must also be usable as textures inside that renderer.

A general 3D scene engine, physics, model/asset language, editor, animation
system for scene assets, media recording/editing/transcoding, DRM playback,
remote desktop, and arbitrary capture of the operating-system desktop are not
required by this scope. They can be subsequent products or capabilities. The
server/backend is not part of this implementation and is not to be rewritten.

Requirements using **must** describe the target implementation. **Current** in
historical evidence describes the inspected reference revision below, rather than
the new implementation's status. **Proposed** and **open** identify
decisions or techniques not established as implemented. The new implementation
does not inherit a passing result merely because an earlier implementation
passed a related test.

### Historical implementation boundary

The inspected framework implementation is frozen at
`3e9f2015702741c7efaca58fc78f140ab89edbcf`, merging public main
`62b4843106ac69d3769b2f8e3dd771837534b960`. Its 299 files under the media source trees
are unchanged from the earlier tested private source milestone. The application
behavior baseline is `3035c2dec06accffe85502e60eef1a92f499170e`. Existing code,
public platform documentation and first-party behavior/reference fixtures are
inputs to this specification; a competitor's decompiled implementation is not.

| Area | Historical inspected implementation/evidence | Consequence for a new implementation |
| --- | --- | --- |
| Apple video/camera presentation | Native decoding/capture, retained native frame storage and Metal presentation exist; an explicit CPU fallback also exists | Preserve the native GPU technique; qualify/report fallback and prove GPU-only behavior |
| Android video/camera presentation | Native decoder and CameraX preview feed external GLES textures; lifecycle and compositor checks exist in bounded emulator cases | Reuse platform techniques through the new embedding contracts; verify the final shared GPU path and actual native resources anew |
| Browser video/webcam | Native browser media feeds the ordinary Skia WebGL compositor without application preview readback in recorded GPU cases | Preserve normal UI composition; retain the distinction between framework counters and hidden browser/driver work |
| Windows/Linux camera | Current capture normalizes/copies pixels to CPU BGRA buffers and uploads a bitmap; desktop QR analysis and torch are rejected | Camera can be a functional CPU fallback after native verification, but primary GPU preview and portable desktop QR still require implementation work |
| Windows/Linux video | No corresponding native playback provider was found in the inspected media implementation | New platform work is required; a camera provider does not establish video playback support |
| QR and photos | Some current paths copy/extract grayscale pixels or use CPU-visible preview frames/canvas snapshots and encoding | Prefer native/borrowed/GPU techniques, avoid unnecessary copies, bound/count any CPU analysis or capture work separately from preview |
| Arbitrary 3D textures | A bounded macOS GL-to-Metal bridge exists, alongside general compositor facilities | It does not establish universal cross-API, cross-device, iOS or renderer-independent embedding |
| WinUI-style additions | Composition surfaces, brushes, effects, animation and visual integration exist with documented backend differences | Use these facilities where suitable; their presence does not establish a complete 3D renderer or every native texture import |

Current runtime evidence is narrower than target support. Physical cameras on
all desktops/phones, the full browser matrix, native Windows/Linux capture,
arbitrary 3D producers, sustained performance and release packaging remain
separate acceptance work. Earlier source-image and retained-paused-image
limitations must remain visible rather than being relabeled as successful
parity.

## 2. Functional layers and composition

| Layer | Responsibility | Must remain independent of |
| --- | --- | --- |
| Portable embedding | Admit compatible external GPU frames, preserve their meaning and lifetime, synchronize producers and consumers, and expose them to normal UI composition and texture consumers | Cameras, codecs, audio, QR recognition and application business rules |
| Camera | Discover and own capture devices, permissions, streams, frames and still-image requests; offer preview through embedding | Video playback controls and QR recognition being installed or enabled |
| QR over camera | Analyze an opted-in stream, decode payloads, associate geometry with frames and publish recognition/feedback | Owning the camera independently or controlling generic embedding |
| Video | Resolve media sources, decode, own audio and transport state, and supply frames through embedding | A camera or QR service |
| 3D canvas | Host a renderer that consumes and produces compatible textures, with a bounded rendering schedule and UI/input integration | A particular scene engine, media decoder or camera |

Camera and video frames can have multiple consumers: a normal 2D preview, a
texture on a 3D surface and, for a camera, optional analysis. Their readers must
share the same source identity and frame metadata without taking write ownership
from the producer. Any required per-consumer GPU conversion/copy is explicit.
Closing one consumer must not destroy storage still used by another.

The 3D renderer produces an output texture that the same embedding foundation
composes with ordinary UI. This is a directional data flow, not a cyclic render
dependency. Sampling an output while simultaneously writing it, or feeding a
composition result into itself, must be rejected or use an explicitly defined
previous-frame resource. No implicit feedback loop is allowed.

UI controls, transport chrome, permission placeholders, QR highlights, captions
when supplied, and accessibility remain normal FerroUI content. A separate native
window or DOM video laid over the UI is not an equivalent default presentation:
it can break clipping, opacity, effects, sibling order and texture reuse.

## 3. GPU-primary presentation and observable CPU fallback

### 3.1 Selection and admission

Embedding, camera preview, video preview and 3D output must have GPU-resident
presentation as their primary route after native acquisition/decoding. In an
admitted GPU route, framework-owned processing must not introduce full-frame CPU
reads, CPU pixel copies/conversions, CPU staging/repacking, per-frame byte-array
transfers, or readback followed by upload. Change orientation, mirroring, crop,
channel order, scaling and supported color interpretation with sampling metadata
and GPU operations. A CPU fallback may perform necessary copies/conversions, but
must not become the default implementation merely because it is easier to build.

An import capability must be established before admitting a route as supported.
Admission checks include actual producer/consumer device compatibility, resource
format/layout, usage permissions, synchronization mechanisms and renderer support.
Allocation success alone is insufficient. The selected path and reason for
rejection must be inspectable. If an admitted route later fails, clear or retain
presentation according to its documented terminal policy and release resources
safely. GPU-preferred operation may then enter a declared bounded CPU fallback;
GPU-only operation must report the unsupported/error outcome. Mode changes and
their causes must be observable; retrying GPU after recovery must be bounded and
must not oscillate between paths every frame.

A graphics API being available is not proof of hardware execution. A known CPU
graphics adapter or software rasterizer is a software fallback even when it
accepts WebGL/GPU-style commands and application readback counters remain zero.
GPU-only operation must reject a known software graphics route. Record actual
renderer/adapter evidence where available; if browser privacy or opaque native
internals prevent reliable identification, disclose unknown hardware status and
do not claim hardware GPU-primary verification. Software decoding feeding a
real GPU is a separate axis from a software graphics backend: report decode
acceleration and presentation acceleration independently.

A native decoder may internally choose software decoding, and a browser may
internally copy an uploaded video frame. The framework cannot certify work it
cannot observe. It must report known hardware/software/unknown decode and import
information where available. A passing framework boundary means **no framework
CPU frame copies in the selected GPU route**, not a universal claim about the whole machine. Stronger
end-to-end claims require platform profiling and a stated measurement boundary.

### 3.2 Operations with different requirements

| Operation | Primary/preferred technique | Permitted fallback and reporting |
| --- | --- | --- |
| Camera/video/3D presentation | Shared native storage, native texture import, same-device sampling or measured GPU copy/conversion | GPU-preferred may use an explicit CPU read/copy/upload path with reason, bounds and counters; GPU-only fails visibly |
| QR recognition | Native recognition with compact results, GPU recognition, or a decoder borrowing a native analysis plane without repacking | Bounded CPU sampling/readback/conversion is allowed when necessary; count it independently and avoid unnecessary full-frame copies |
| Camera photo | Native still capture/encoding or native/GPU encoding from retained storage; transfer owned encoded output | A reported bounded CPU capture/encoding path is allowed; distinguish a preview snapshot from native still capture |
| Diagnostic screenshots/reference comparison | Explicit, development-only capture, outside presentation accounting | Label its pixel transfers and cache/rendering effects; never use it to conceal preview transfers |
| Color/capability calibration | Metadata/declared capability first; any synthetic readback is separate and bounded | Report synthetic probe bytes/rate separately; never call them preview pixels or pretend that no readback occurred |
| Software/bitmap presentation | Available as a distinct fallback under GPU-preferred policy or an explicitly chosen diagnostic mode | Report mode/reason/limits; never satisfy a GPU-only request or conceal an unimplemented primary GPU path |

Reading an already CPU-accessible native luma plane is different from copying a
frame into a newly packed grayscale image. It still consumes CPU/memory bandwidth
and must be measured. Mapping GPU-only storage in order to download it is a
readback even if an implementation calls the result a mapped view. An analysis
consumer must not extend a camera buffer lease indefinitely or stall preview.

Encoded photo bytes, text results and compact geometry are ordinary output data;
moving these values does not justify a full raw-frame CPU copy. Native services
may perform internal encoding/recognition work outside framework visibility.
That boundary must be declared rather than described as proven GPU execution.

Every CPU pixel-transfer path must identify its consumer, reason,
platform/backend, maximum dimensions/bytes/rate, cancellation/lifetime behavior,
selection policy and counters. Preview fallback, QR analysis, photo capture and
diagnostics are distinct purposes. One cannot conceal another's transfers or
justify avoidable per-frame copies. Selection follows the declared host policy;
fallback is already permitted by the GPU-preferred policy.

### 3.3 Required measurements and failure tests

Counters must separately report preview CPU readbacks, preview CPU pixel bytes
copied/staged/uploaded, analysis transfers, photo transfers, diagnostic/probe
transfers, GPU import count, GPU copies/conversions, in-flight frames and retained
native resources. Count attempted transfers as well as successful transfers.
Counter definitions must identify which native/browser boundary is observable.

Each admitted GPU acceptance run must show zero framework preview readbacks and
zero framework CPU preview bytes. Negative tests must make prohibited readback,
CPU conversion and bitmap-upload entry points fail, then run actual camera,
video and 3D presentation. Removing the GPU importer or using an incompatible
resource must make GPU-only operation report unsupported/error. In GPU-preferred
operation it may select the declared CPU fallback, and the test must verify its
reason, counters, limits and correct presentation. Conversely, a valid GPU route
must win over an available CPU fallback. Test the counters with an explicit
diagnostic transfer so a broken counter cannot produce a convincing zero.

### 3.4 CPU fallback behavior

A CPU fallback must state whether native capture/decode already supplies CPU
pixels, whether GPU readback is needed, what conversion/repacking occurs and
whether the result is uploaded or presented by a software renderer. Validate
dimensions, signed stride, plane lengths, multiplication/offset overflow, color
and alpha before reading memory. Bound reusable staging storage and pending
work; a slow consumer must drop/coalesce preview frames rather than accumulate
an unbounded queue. Do not recopy/reupload an unchanged paused frame on every
UI tick.

Fallback must preserve the same source identity, crop/orientation, color/alpha,
layout, transport/capture semantics and ownership guarantees as GPU presentation.
Its pixels and resource retirement require their own acceptance tests. Share
staging work across compatible consumers where safe; do not use fallback as a
reason to decode a video or acquire a camera twice. Protected/unreadable native
resources cannot be made readable by ignoring platform restrictions; where no
legal/safe fallback exists, report unsupported/error under either policy.

## 4. Portable embedding requirements

### 4.1 Frame meaning and transport

Each submitted frame must carry enough information to interpret its storage
without guessing: pixel dimensions, visible image region, storage-plane layout,
channel order/bit depth, color primaries/transfer/range, alpha interpretation,
source origin, rotation/mirroring/crop transform, source identity, frame sequence,
capture/presentation time, resource generation and synchronization ownership.
Device/adapter identity and whether the resource may be sampled, rendered or
shared must be available to admission. Metadata can be normalized without
copying pixel storage.

The initial portable interchange profile must support ordinary 8-bit RGBA/BGRA
color textures, opaque camera/video, and premultiplied alpha for a transparent
3D output or arbitrary producer. Opaque output must not accidentally inherit
undefined alpha. Straight alpha must be declared and converted on the GPU when
needed. YUV/multiplane input, including common semi-planar video formats, is
admitted only when the backend supplies a tested sampler/conversion path.

HDR, wide-gamut and higher bit-depth formats must be described accurately if
encountered. They are not automatically supported by an 8-bit SDR profile. A
future HDR profile requires explicit tone mapping/output behavior and test
assets. Silent interpretation of YUV as RGB, limited range as full range, or an
unknown transfer function as a known one is unacceptable. Unsupported metadata
must yield a defined capability result or an explicitly selected conversion.

In GPU mode the same frame may be sampled in 2D composition or on a 3D surface
without an intermediate CPU image. In fallback mode, share bounded staging work
between consumers where safe instead of duplicating avoidable copies. Native
handle/resource lifetime is distinct from the
lifetime of a UI element, a camera session or a decode callback. Consumers may
not retain a borrowed texture beyond the scope in which its owner guarantees it.
If longer retention is needed, acquire an independent native lease or make an
explicit GPU copy into owned storage.

### 4.2 GPU completion, queues and ownership

Producer submission, producer GPU completion, consumer submission and consumer
GPU completion are distinct milestones. Storage cannot be sampled before the
producer's writes are visible and cannot be reused/destroyed before every
consumer has finished reading. Shared handles alone do not provide this order.
Use native fences, shared events, semaphores or an equivalent tested mechanism.
Cross-queue/API ownership and layout transitions are part of the contract.

No unbounded wait, GPU idle wait, blocking fence poll or busy loop may run on
the UI thread. Completion is asynchronous. A canceled UI request must release
its UI references promptly while safe native retirement can finish independently.
Canceling a future is not evidence that submitted GPU work has stopped.

Queues must be bounded. Define a per-producer in-flight limit, a total retained
storage budget and a retirement budget; include old-size and retiring resources
in those limits. At saturation, drop/coalesce preview work or await a future
render opportunity. Never allocate indefinitely or block the UI to preserve
every camera frame. A producer requesting exact frame processing must use a
separate explicit policy and cannot silently change preview backpressure.

A frame offered but never presented must still release its ownership exactly
once. A failed or indeterminate GPU completion must never cause premature reuse.
If safe release cannot be established, a bounded quarantine or device/context
recreation may be necessary. Its limit and terminal outcome must be inspectable;
it must not grow forever under repeated failures.

### 4.3 Layout, composition and coordinates

The embedding foundation must compose at the arranged device-independent bounds
and current render scale. It must preserve fit, fill, crop, alignment, clipping,
parent transforms, opacity, rounded/masked clips where supported, and sibling
ordering. Output pixel size is derived from arranged size and scale with checked
arithmetic and explicit maximum dimensions. Zero size must not allocate invalid
textures or spin rendering. Resize/scale changes retire old resources after GPU
completion and invalidate stale work by generation.

Sampling transforms must establish one origin and apply rotation, mirroring,
crop and channel/color interpretation exactly once. A producer-specific texture
transform is part of this calculation. The content rectangle can extend beyond
the viewport for a crop-to-fill presentation; clipping must not replace it with
the viewport before overlays are mapped.

A frame shown in normal UI must interact with existing composition effects,
surfaces and animations through the current renderer. If a backend cannot apply
an effect to an imported resource, report that constraint. Do not introduce a
second media-specific compositor to approximate the same UI contract.
If an effect requires CPU processing of the frame, that path is an observable
fallback under GPU-preferred policy and is unavailable to GPU-only operation.
Do not omit effect-related frame transfers from the preview counters or silently
drop an effect in order to claim GPU success.

### 4.4 Scheduling, detachment and recovery

Render on invalidation, new source frame, resize, animation or explicit continuous
3D activity. A paused/static source must not cause an unnecessary full-rate loop.
Avoid performing multiple producer renders for one presentation opportunity
unless a consumer explicitly needs them. Use FerroUI's scheduling and render
lifecycle rather than an independent application-wide timing system.

Source replacement and detachment invalidate queued publications immediately.
Late callbacks may complete native cleanup but must not install an old frame
into a new consumer. Device/context loss invalidates all imports, views, command
resources and synchronization for that generation. Recovery must rebuild through
the existing renderer/device lifecycle, preserve requested product state where
possible and explicitly report lost state. Holding the last valid image during
recovery requires genuinely owned storage, not a stale native texture name.

## 5. Video behavior

### 5.1 Sources, preparation and state

Support application resources, local files and supported network URLs without
copying application-specific tutorial assets into the framework. Resource data
and temporary native URLs/handles belong to the playback lifetime. Cleanup must
release/revoke them after use. Network authentication, cookies, proxy/trust and
redirect behavior must use the host's actual platform/framework integration;
the media layer must not invent a second backend or silently log credentials.

Publish native preparation, known duration, natural size, seekability, buffering,
requested transport state, actual decoder activity, completion and errors as
distinct observable facts. Unknown/live duration must not be fabricated as a
finite duration. A request to play before readiness records intent; it does not
prove that a decoder is currently playing. A failure must not leave a successful
playing indication while no viable session exists.

The initial acceptance media profile includes redistributable MP4/H.264
fixtures and an independently audible fixture. Existing tutorial assets may be
used externally as reference inputs without copying them into the framework. Additional
codecs, containers, streaming protocols, live inputs and subtitle formats are
capabilities, not promises inferred from a file extension. Unsupported media
must produce a useful error and allow replacement with a valid source.

### 5.2 Transport and audio

Provide play, pause, stop, seek, looping, volume and mute behavior. Pause keeps
position and a valid owned displayed frame while preserving the ability to
resume. Stop ends playback and resets to the beginning for ordinary seekable
media; the displayed first-frame/placeholder policy must be explicit and tested.
A non-seekable stream must report that limitation rather than claim a successful
rewind. Clearing the source releases its media/native resources and presentation.

A completed seek reports the achieved native position and a corresponding
presentation opportunity, not only acceptance of a command. Seek precision and
keyframe constraints are reported; tests must use a stated tolerance. Repeated
or overlapping seeks must have a declared latest-request policy and prevent a
superseded completion from overwriting newer state or pausing unrelated playback.
Pause/seek/resume, seek to zero, seek near end and seek during buffering require
separate validation.

Looping restarts according to native stream capabilities without a false terminal
failure. Non-looping playback publishes completion once per playback cycle and
allows an explicit restart. Position notifications must be bounded and must not
create an unnecessary CPU frame-copy path. Audio uses native device/session/focus services;
mute and volume are applied before an initial automatic play attempt. A muted
video is not proof that audio output works. Device-route changes, interruptions,
focus loss and background policy need native tests.

Automatic playback is subject to platform policy. A denied browser/user-gesture
play request must surface its actual outcome and permit a later user Play to
recover. Do not simulate success or request unrelated microphone permission.
The media presentation clock is not the UI animation clock; select frames using
the native playback timeline while keeping presentation synchronized.

### 5.3 Replacement, ordering and source fidelity

A new source cancels publication from the old source immediately. Old retirement
may continue asynchronously, but it must not retain an unreachable page, install
late sessions, clear a newer presentation or duplicate disposal. During an
asynchronous replacement, define whether the previous valid image or inactive
placeholder is visible; preserve the application's established behavior through
its migration profile.

Observable event order, synchronous callbacks during preparation and cancellation
are product contracts. The existing Android behavior publishes duration/readiness
callbacks before applying current play intent or the immutable automatic-play
policy. A callback can issue another command; disposal/replacement during that
callback must prevent stale later publication. Under that legacy policy,
automatic play can still win over an earlier pause at preparation. Preserve
this where the existing app depends on it, or document a deliberate behavior
change with a source reference and migration test.

The current application profile defaults to automatic play and looping enabled,
volume at full scale, unmuted audio and aspect-fit video. Those are source-app
requirements, not a prescription for names or defaults of a new generic API.
Generic framework defaults remain an explicit decision. Four known late-detach
reference differences intentionally keep cancellation/disposal safety instead
of publishing canceled work; a rewrite must not reintroduce those unsafe results
merely to mirror source internals.

## 6. Camera behavior

### 6.1 Discovery, consent and stream lifetime

Discover available devices and disclose actual formats, facing information and
optional capabilities. Unknown facing remains unknown; a desktop webcam, label
or device count is not proof of front/rear orientation. Requested facing,
resolution and frame rate must distinguish preferences from exact requirements.
Report the selected device and actual negotiated settings.

Constructing reusable UI or querying known capabilities must not itself open a
device or request permission. The host's explicit active camera workflow owns
that action. No camera workflow requests a microphone unless an independently
requested feature requires it. No-device, permission-not-yet-decided, denied,
restricted, busy, disconnected and native failure outcomes must be distinguishable.
Do not repeatedly prompt after denial or silently alter host privacy settings.

Pending permission/startup owns all provisional resources until success commits.
Canceling or abandoning startup prevents any late result from appearing. A late
permission grant must have all its tracks/outputs/resources closed if its owner
is gone. Multiple consumers sharing a startup must not cancel each other's
valid ownership; the last abandoned owner must retire an uncommitted stream.

Start, stop, restart, device switch, facing switch, detach/reattach and host
deactivation need explicit lifetime semantics. Stop must end owned capture work
and remove callbacks; subsequent start obtains a valid new native generation.
An optional preview pause may retain a stream if the platform supports it, but
it must be distinguishable from stopping the camera. Camera indicators, hardware
availability and resource release must be checked on real native hosts.

### 6.2 Preview, images and controls

Preview frames use the shared embedding foundation, prioritizing an admitted
GPU route without CPU pixel copies and reporting any selected CPU fallback.
The producer's sensor rotation, display orientation, preview mirror, crop
and texture transform are distinct metadata. A front-camera preview may be
mirrored without changing the orientation/handedness of encoded photo bytes.
Preview geometry must remain correct through rotation, letterboxing, crop to
fill, layout changes and device switching.

Still capture must report dimensions, content type, capture time, orientation
and whether it is a native still or a preview-derived snapshot. Prefer a native
still output/encoder; a CPU preview snapshot is a declared fallback with bounded
transfers, not a falsely advertised high-resolution native still. Encoded output
has independent owned lifetime after the camera
stops. JPEG is the compatibility baseline; PNG or other formats are advertised
only where provided. Unsupported formats or impossible dimensions must fail
before allocating unbounded storage.

Capture completion/publication ordering must preserve the first-party behavior
where the app depends on it, including a request completing while preview or UI
state changes. The result may outlive the initiating presentation; that must not
resurrect detached UI. Explicit cancellation and a native capture already in
progress need separate outcomes, with eventual native cleanup in both cases.

Focus, exposure, zoom, torch and high-resolution still capture are optional
capabilities. Never claim success by storing a requested setting that was not
applied natively. Failure of a required startup setting must retire that startup;
failure of an optional live adjustment must state whether preview continues.
Per-device support is distinct from a platform family having an API for it.

The existing application profile starts an active attached camera, prefers the
rear lens, uses aspect fit and normally stops after the first accepted QR result.
Preserve its visible permission/status/capture behavior through migration;
generic framework activation/defaults are a separate decision.

## 7. QR recognition over camera frames

### 7.1 Recognition and resource use

QR is opt-in and must not require a second camera session or a separate preview
pipeline. It consumes frames/analysis planes from the selected camera generation
and returns compact payload/geometry metadata. Turning QR off must release its
analysis resources and leave preview usable. Camera/video/3D embedding must not
depend on the recognition engine being installed.

Prefer native camera/vision metadata recognition or a suitable GPU recognizer.
Where a portable CPU decoder is selected, it should read a borrowed native luma
plane directly, respecting row/pixel stride and orientation without repacking a
full frame. Detection typically identifies finder/alignment patterns, estimates
perspective, samples the symbol grid, performs error correction and interprets
payload segments. The required behavior is successful, bounded recognition;
this specification does not prescribe reimplementing a decoder algorithm or
adopting its internal organization. A public decoder can recognize rotated and
oblique symbols and multiple symbols; engine capability must be tested rather
than inferred from that general ability. [Public decoder background](https://github.com/dlbeer/quirc).

Analysis must be bounded independently of presentation: at most the declared
number of in-flight analyses, a declared input region/resolution, a maximum
frequency and latest-frame/drop behavior while busy. Release a native analysis
frame after all readers finish, including errors/cancellation. Do not hold the
camera queue while waiting for UI feedback or a network validation response.
CPU decoding and necessary grayscale sampling/conversion/readback are permitted
as bounded, measured analysis work. Prefer native planes and avoid repacking
or downloading a full frame when a smaller analysis region is sufficient.
None of that work may be hidden inside the claimed GPU preview counters.

### 7.2 Payload, detection policy and feedback

Preserve payload bytes and decoding status where the engine exposes them.
Text interpretation must explicitly handle the selected encodings/ECI behavior,
including UTF-8, the declared legacy single-byte interpretation and Kanji when
supported. Unsupported or malformed encodings must not create a fabricated
successful text value. Engine-specific limits, structured-append support and
multi-symbol ordering are capability statements. The framework does not parse
application URLs, trust invitations, authenticate users or execute decoded data.

Recognition, candidate visibility, stable/deduplicated publication and application
validation are separate policies. The application must be able to accept/reject
a value, display feedback and continue or stop scanning. Invalid invitation data
is not a fatal camera error. First-result stopping must occur once and must not
allow already queued analyses to emit additional accepted results afterward.

The existing first-party observation profile emits a new distinct decoded value
immediately, suppresses duplicate detections until loss or a different value,
resets on loss, throttles visible geometry to 66 ms and coalesces changes within
0.001 normalized units. Error feedback can suppress the current value's repeated
candidate until cleared or replaced. Empty/native-null payload handling and
whitespace normalization have source-specific order and require fixtures. The
current profile publishes the first successfully decoded symbol in an analyzed
frame. A configurable multi-symbol/stability policy is a future generic choice,
not an undocumented replacement for those app semantics.

### 7.3 Geometry from camera to 2D and 3D

Associate each candidate with source generation, source dimensions, analysis
frame/timestamp, visible crop and orientation/mirror metadata. Preserve true
corners/quadrilateral when available; a bounding rectangle is a derived view.
Normalize from the recognizer's coordinate system to the camera image, apply
the declared crop/orientation/mirror transform once, then map to the actual
drawn content rectangle and host coordinates. Clip against the visible image
and normal UI clip. A changing layout must not reuse a stale content transform.

All transform inputs must be finite and dimensions positive. Clearly define
whether native geometry is already rotated/cropped. Reapplying a native rotation
is an error. For the source application, preserve the recorded rotate → crop
adaptation when supplied → mirror → clamp → content mapping and its 24-DIP
minimum highlight size as an application presentation policy. A minimum visual
highlight must not mutate the underlying true detection corners. An older
source mapper ignored crop metadata; the tested crop-aware behavior is an
explicit adaptation, not universal original-source parity.

For camera textures on a 3D surface, texture coordinates use the same camera
sampling transform. A planar overlay can then be projected through the same
surface/camera transform to a screen quadrilateral. Perspective division, clipping
and points behind the viewing plane must be handled. An axis-aligned 2D box is
not an equivalent projective overlay. Non-planar meshes require the client
renderer to supply an appropriate surface mapping or to report that overlay
projection is unsupported.

An asynchronous recognition result must not be drawn on a replacement device
or incompatible transform generation. State the maximum result age and whether
overlay presentation follows the analyzed frame or the latest preview. Exact
frame-aligned composition and approximate live tracking are different policies;
tests must say which they validate.

## 8. 3D canvas and texture embedding

The initial 3D product is a composited host for a renderer, not a new scene engine.
The client owns its scene, geometry, materials, viewpoint and scene hit testing.
The framework owns integration with arranged size/scale, scheduling, compatible
GPU resources, completion/lifetimes and normal UI composition.

The renderer must be able to render into an admitted texture/resource using the
current compatible device or a tested interop route. Depth/stencil and
multisampling can be private render resources; any resolve into the final color
texture occurs on the GPU. The final image's color space, origin and alpha must
be declared. A transparent output must compose correctly over UI content with
premultiplied alpha and parent opacity. GPU mode uses no CPU raster snapshot;
any software/render-readback fallback is selected and counted under the same
policy as other embedding consumers.

Camera/video frames are sampled as texture inputs using their existing leases,
color conversion and sampling transforms. The 3D host must not open a second
camera or decode video twice. The primary route must not copy a frame to a CPU
image simply to texture a surface; a required CPU compatibility path is reported
as fallback, with bounded shared work. The renderer may require a GPU conversion/copy for format
compatibility; report that work and its lifetime. It must not sample a resource
after its producer/consumer lease expires.

Support both invalidation-driven and explicitly continuous rendering. Stop or
throttle continuous work when hidden, detached, zero-sized or suspended according
to host policy. Resize recreates resources safely and does not dispose resources
still being sampled. Device loss re-enters the standard embedding recovery
contract rather than leaving a stale scene texture in the UI.

Normal UI overlays retain their normal hit testing and input precedence. The
host routes unhandled pointer/touch/pen/wheel/keyboard input through FerroUI and
converts coordinates to the rendered viewport. The client performs any 3D ray
construction and scene intersection. Pointer capture, gesture competition with
scrolling, focus, cancellation and detachment must use the normal framework
lifecycle. Do not imply that a rendered triangle automatically becomes an
accessible UI element; meaningful scene actions need explicit accessible
descriptions and alternative controls supplied by the application.

## 9. Platform techniques

The techniques below are selected directions constrained by actual framework
capabilities. They are not a promise that every named platform facility is
already wired into every FerroUI renderer.

### 9.1 Windows

**Embedding and 3D.** Prefer the compositor's compatible Direct3D/ANGLE device
and native shared color textures. A compatible same-device resource can be
sampled directly; separate device/API producers need an explicitly supported
shared handle plus synchronization and any required GPU conversion. Device
identity, format, usage and ownership must be checked. Keyed mutexes or supported
fences establish producer/consumer order; opening a shared resource alone does
not. Vulkan/other renderer routes need their own verified import capability,
not an assumption that an arbitrary handle is portable. [Microsoft texture
sharing](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/ne-d3d11-d3d11_resource_misc_flag).

**Video.** The proposed native route is Media Foundation with hardware decoding
into GPU-backed surfaces associated with the chosen graphics device, followed by
GPU sampling/conversion into the embedding path. Windows supports coordination
between a decoder and an external Direct3D 11 device through a graphics device
manager. Codec/OS edition/device availability still needs capability checks.
This route is proposed new work: the inspected media implementation has no
Windows video provider. [Microsoft native decoding integration](https://learn.microsoft.com/en-us/windows/desktop/medfound/supporting-direct3d-11-video-decoding-in-media-foundation).

**Camera and QR.** Obtain native capture frames as GPU surfaces where the device
and capture stack support them. A Windows capture route can expose device frames
for native processing and respond to device changes; whether its output is a
GPU surface or software image is a negotiated capability. Choose a route that
feeds the admitted embedding device and offers borrowed native analysis or
native/GPU recognition for QR. The current CPU RGB/BGRA provider can remain a
declared functional fallback, but does not establish primary GPU preview.
[Microsoft frame processing](https://learn.microsoft.com/en-us/windows/apps/develop/camera/process-media-frames-with-mediaframereader).

Windows privacy can deny camera access for a device, packaged app or unpackaged
apps. Treat access denial as a permission result, with a user-directed settings
action where appropriate. Preserve native callback/apartment ownership and wait
for outstanding capture callbacks/flush work before final native release.
Physical permission/capture/recovery and GPU preview on native Windows remain
required evidence. [Windows camera privacy](https://learn.microsoft.com/en-us/windows/apps/develop/camera/camera-privacy-setting).

### 9.2 macOS

The default macOS media profile is Metal/Skia Graphite. An additional opt-in
experimental OpenGL/CGL profile uses Skia Ganesh, direct retained IOSurface rectangle views
and owning-context GL fences. Both routes use the same AVFoundation providers,
source acquisition and frame budgets. CGL requires the existing
`ferro_skia_apple_ganesh` build configuration and its own native acceptance;
the repeated-reset native-memory gate is currently open on the Apple Silicon
verification host. It does not replace the default Metal profile. Existing bridge observations
below remain historical evidence, separate from current media verification.

**Video and camera.** Use AVFoundation for playback/capture and native timing,
retain the native pixel-buffer/storage owner, and make compatible Metal texture
views of that storage through CoreVideo. Playback frame selection follows the
native item timeline. Camera stream processing can produce a Metal preview
without an intermediate application CPU bitmap; Apple's capture sample shows
GPU filtering of native capture images. Compose the resulting texture through
the existing renderer, not a separate video layer. [Apple playback-frame
output](https://developer.apple.com/documentation/avfoundation/avplayeritemvideooutput),
[Apple GPU capture processing](https://developer.apple.com/documentation/avfoundation/avcamfilter-applying-filters-to-a-capture-stream).

**Embedding and 3D.** Metal producers should share the compositor's admitted
device/resources and completion mechanism. Existing standalone GL producers can
use IOSurface-backed storage imported into GL and Metal on a compatible same-GPU
route. The current bounded bridge uses a producer fence, asynchronous completion
and GPU snapshots; it does not prove no GPU copies. Reuse of the native storage
must wait for the Metal consumer to finish. Unknown/mismatched devices or
unsupported storage formats must fail capability admission. Existing evidence
is bounded to opaque BGRA and a minimum-OS constraint; arbitrary alpha, HDR,
cross-device and all renderer routes need additional work.

**Consent and QR.** Respect native camera authorization and host privacy/signing
metadata. Enumerate actual built-in/external/Continuity devices where available;
do not prompt pointlessly when no camera exists. Native barcode metadata or
vision processing is preferred for QR, with compact geometry and a retained
capture lifetime. Any necessary CPU analysis/photo fallback is bounded and
accounted independently from preview.
The current host's no-device behavior and injected native frames are not proof
of physical camera/permission/Continuity capture.

### 9.3 iOS

Use the same AVFoundation/CoreVideo/Metal principles for video and camera, with
device-specific sensor orientation, display rotation, front-camera mirroring,
native still outputs and native barcode/vision metadata. Native capture offers
direct stream samples; texture views and synchronization must preserve their
owners until GPU use finishes. Permission and still capture use the actual
application lifecycle and declared host privacy metadata. [Apple stream capture](https://developer.apple.com/documentation/avfoundation/audio-and-video-capture),
[Apple CoreVideo-to-Metal integration](https://developer.apple.com/documentation/corevideo/cvmetaltexturecache).

A GL producer does not gain a portable iOS-to-Metal bridge from the macOS proof.
Prefer a compatible Metal producer for the new 3D host. Any legacy GL/native
buffer interop requires a separate device/OS capability decision and actual
tests. Use host suspension/interruption rules to retire capture and preserve
the declared playback intent; do not promise camera background use without the
appropriate native capability and verification.

The current source app declares iOS 14 while media/native dependencies have a
separate higher floor in parts of the integration. A new implementation must
resolve and document that supported minimum, not silently raise the app minimum
or infer old-OS support from a current simulator. Simulator playback/injected
frames do not prove device camera, torch, audio routing or suspension behavior.

### 9.4 Android

**Video/camera and embedding.** The current native technique directs decoded
video or camera preview into a native surface backed by an external GLES
texture. A frame-available notification announces queued storage; the texture
consumer acquires the next image on the correct current context and uses its
native sampling transform. External textures have distinct sampling rules and
cannot be treated as ordinary 2D texture names. Sample/convert on the GPU into
the compositor-compatible resource when required. Retired contexts/surfaces must
reject stale callbacks. [Android external texture pipeline](https://source.android.com/docs/core/graphics/arch-st).

For an admitted Vulkan or other cross-API route, Android native hardware buffers
can share storage with EGL/OpenGL and Vulkan. This is a capability-dependent
alternative, not an implemented universal bridge. Native format/usage,
multiplane/external-format sampling, synchronization and device support must be
checked; some imported video formats are sample-only and need GPU conversion.
[Android native hardware buffers](https://developer.android.com/ndk/reference/group/a-hardware-buffer),
[Vulkan Android external-memory constraints](https://docs.vulkan.org/refpages/latest/refpages/source/VK_ANDROID_external_memory_android_hardware_buffer.html).

**Camera/QR/photo.** Prefer CameraX/native capture lifecycles with separate
preview, still and opt-in analysis uses. Analysis backpressure should keep the
latest work without blocking preview; native analysis planes must be released
only after recognition has finished. The platform documents that a blocking
analysis pipeline can stall other camera uses. GPU preview must remain intact
when QR is disabled or slow. [Android analysis and backpressure](https://developer.android.com/media/camera/camerax/analyze).

Native barcode recognition is an optional engine. Its model packaging,
availability and any download requirements must be disclosed, and capture
buffers must remain alive until asynchronous recognition completes. This does
not require a CPU preview copy. A portable borrowed-plane recognizer is another
possible engine; the exact no-copy behavior needs measurement. [Android native
recognition integration](https://developers.google.com/ml-kit/vision/barcode-scanning/android).

Native activity/surface recreation, permission denial, background/foreground,
orientation, device switch and process shutdown must retire their actual
surfaces, queues and callbacks. Verify the supported minimum API, ABI/native
imports, linker/package alignment and an actual 16 KiB kernel with active video
and camera. Passing an app/API fixture on that kernel does not validate active
media or camera. Emulated sensors and SwiftShader do not prove physical GPU,
torch, focus or camera performance.

### 9.5 Linux

**Camera.** V4L2 capture can hand buffers between a driver and application through
mapped queues; DMA-BUF provides separate cross-device storage sharing. A mapped
capture queue avoiding a driver copy is not by itself a GPU preview. The target
GPU route needs export/import of compatible native buffers into the selected
graphics backend, with per-plane layout and synchronization preserved. Current
CPU conversion/upload can provide a declared functional fallback but leaves the
primary GPU implementation incomplete. [Linux capture queue
semantics](https://docs.kernel.org/userspace-api/media/v4l/mmap.html),
[Linux DMA buffer sharing](https://docs.kernel.org/userspace-api/media/v4l/dmabuf.html).

**Embedding/3D.** An EGL route can import DMA-BUF planes; Vulkan routes need their
own external-memory/synchronization support. Format, plane offsets/pitches and
DRM modifiers must match the exporter. Do not assume a non-linear/tiled buffer
is a packed CPU image. If no safe shared route exists, GPU-only fails visibly;
GPU-preferred can use a declared bounded CPU fallback. Neither path may silently
misinterpret or map a device-only resource unsafely.
[Khronos Linux image import](https://registry.khronos.org/EGL/extensions/EXT/EGL_EXT_image_dma_buf_import.txt).

**Video.** A native media pipeline able to negotiate GPU output is required.
GStreamer is a proposed option, with decoded DMA-BUF/native GPU output feeding
embedding and native audio/timing. Its format negotiation must include modifiers
and actual import compatibility. Selecting the dependency/plugin/codec delivery
policy remains open; there is no existing Linux video provider to label complete.
[Native pipeline buffer negotiation](https://gstreamer.freedesktop.org/documentation/additional/design/dmabuf.html).

**Desktop permissions/QR.** A sandboxed/portal-aware camera path is distinct
from direct device-file capture. The desktop camera portal grants access and
returns an authorized PipeWire connection. Its buffer negotiation then needs an
admitted GPU route. Direct V4L2 permission/device selection remains an explicit
host profile, not a portal substitute. QR needs native/GPU recognition or borrowed
analysis planes; it is absent in the current desktop provider. Device removal,
damaged-buffer requeue, worker stop and mapped-buffer retirement need real Linux
tests. [Linux desktop camera access](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Camera.html).

### 9.6 Browser

**Camera and video sources.** Native browser media provides decoding, audio and
camera streams. Webcam access requires an allowed secure context and user
permission; use video-only capture for camera/QR. Constraints are negotiated,
and device labels/facing data are not universally exposed before consent.
Device changes, track termination and late permission results require explicit
ownership cleanup. Do not pretend that cancellation necessarily dismisses an
in-progress browser permission request. [Browser media capture standard](https://www.w3.org/TR/mediacapture-streams/).

**Current WebGL technique.** The inspected GPU path samples/uploads native video
content within the same WebGL context used by the ordinary Skia compositor,
then exposes the native texture through normal composition. There is no per-frame
framework canvas readback, JavaScript pixel array or WASM preview pixel transfer.
Transforms, clipping, opacity and UI siblings still apply. Browser-internal
upload/conversion choices remain outside that application counter boundary.
Origin/CORS restrictions and permitted texture use must be respected; a security
failure must not trigger a canvas-readback workaround.

**WebGPU direction.** When the actual FerroUI compositor can admit the same
WebGPU device and resource, native external-video sampling is a proposed route
for video, camera and 3D texture use. The standard permits an implementation
without a source copy but does not guarantee it. Such imports are immutable
frame views with source-dependent expiry; use them only within their valid
lifetime and reacquire appropriately. A browser-image-to-texture copy is a
different operation and must be accounted for. Merely creating a separate
WebGPU canvas does not integrate media into the current compositor.
[WebGPU external video semantics](https://www.w3.org/TR/webgpu/#external-texture).

The inspected browser media path is WebGL, and the repository's Graphite/WebGPU
direction is not evidence of a completed media importer. WebGPU should not be
promised across all browsers/devices or selected just to conceal missing texture
interop with the active renderer. WebGL1/2, WebGPU, software, page-thread and
render-worker paths have separate capability/testing profiles. A selected strict
GPU profile must report unsupported where the bridge does not exist.

WebCodecs is an optional future producer for explicit frame/decode control; it
does not require any particular codec and is not a complete source/demux/audio
player. Its output ownership must be closed promptly to avoid retaining decoder
resources. Using it does not by itself eliminate copies or supply worker/compositor
interop. [Browser codec and frame ownership standard](https://www.w3.org/TR/webcodecs/).

**QR/photo and recovery.** Prefer native/GPU recognition where actually available
or an admitted borrowed analysis path. Current canvas/grayscale readbacks and
canvas photo snapshots are permitted bounded analysis/capture work when necessary;
prefer lower-copy native/GPU alternatives and measure them separately from GPU
preview. Context loss must release native
textures, stop/reacquire camera tracks as policy requires and reject stale
pending publications. Test actual Chromium, Safari and Firefox separately;
headless WebKit is not actual Safari, a virtual webcam is not physical consent,
and desktop support does not prove mobile browser support.

## 10. Integration with FerroUI and WinUI-style additions

Use existing property/binding, resource, styling, templating, visual-tree,
composition, animation, scheduling, focus, input and accessibility facilities.
Reusable media/camera/3D presentation must be styleable and usable from Rust and
declarative markup through focused metadata for the actual supported properties,
events and commands. Metadata must not grow into a general runtime type system.

Separate visual presentation, transport/capture state and optional control chrome
at the behavioral level, so a host can use its established themed controls and
WinUI-style layout/composition facilities. That separation does not prescribe
an inheritance tree or duplicate the existing composition engine. Common native
surface/brush integration should be reused for textures, masks, effects and
animations where the renderer actually supports it.

Keyboard focus, pointer capture, routed input, clipboard, fonts and scheduling
come from the existing framework/platform services. Transport controls need
normal tab traversal, accessible names, enabled states and value/state exposure.
Camera denial/errors must be understandable without relying on color or a
moving image. QR status announcements should be bounded and must not announce
the same value every frame. A noninteractive preview should not steal keyboard
focus or intercept gestures intended for scrolling/transport controls.

The new implementation must use idiomatic Rust ownership, borrowing, reference
counting where appropriate, weak UI ownership, asynchronous work and explicit
subscription/cancellation lifetimes. Resolve cycles directly. No CLR/BCL
emulation, general reflection/delegate/task compatibility system or parallel
string/collection runtime is required. Keep platform FFI narrow and make native
thread/queue/ownership requirements explicit.

Source event names, overloads, record-generated methods, runtime hashes and
exception class shapes are not implementation requirements. Observable ordering,
algorithms, assets, serialization boundaries, lifetime and cancellation behavior
are requirements where demonstrated by real consumers. Document deliberate API
or behavior changes and keep any application migration adapter narrow. The
layered foundation must not acquire application-specific business rules.

## 11. Acceptance and evidence

### 11.1 Required behavioral checks

| Gate | Measurable acceptance |
| --- | --- |
| Layer independence | Arbitrary non-media GPU producer presents with camera/video/QR disabled or absent; camera works with QR disabled; QR attaches to the existing camera without opening another device |
| GPU admission and fallback policy | Compatible native resources are admitted; wrong device/generation/format/usage/synchronization is rejected before unsafe access; a valid GPU path wins; missing import makes GPU-only fail and GPU-preferred use only its reported permitted fallback |
| Hardware/software identity | Record actual renderer/adapter evidence; known software graphics is fallback and rejected by GPU-only; unknown browser/native hardware status is labeled unknown; software decode is reported independently |
| No CPU frame copies in GPU mode | Actual video, camera and 3D GPU runs show zero framework preview CPU readbacks/copy/staging/upload bytes; prohibited CPU paths fail under negative instrumentation; an independent diagnostic transfer proves counters can increment |
| CPU fallback | Force an unavailable GPU route, verify selected fallback/reason, actual copied bytes/rate, bounded queues/memory, correct pixels, cancellation and recovery; verify GPU-only never enters that fallback |
| Multiple consumers | One camera/video source is shown in 2D and on a 3D surface without duplicate native acquisition or CPU pixels in GPU mode; fallback avoids duplicated staging; readers retire independently and storage survives until final GPU completion |
| Sampling/layout | Deterministic color/orientation/alpha grids verify fit, fill, crop, mirroring, cardinal rotations, render scales, clips, parent opacity, effects where admitted and sibling overlays |
| Pixel validation | Algebraic coordinate fixtures use a declared numeric tolerance; rendered deterministic interiors/edges use independently generated expected pixels and predeclared per-backend tolerances, never tolerances widened after failure |
| GPU lifetime | Hold real producer/consumer completion separately, saturate the bounded queue, resize/detach/cancel, then release completion; no early reuse/destruction and no unbounded growth; UI input continues while completion is pending |
| Fault/recovery | Real context/device loss where available, stale callbacks, native submission failure, abandoned unpresented frame and indeterminate completion produce declared retirement/recovery outcomes |
| Video transport | Unchanged application clips plus audible fixture cover prepare, automatic play policy/refusal, actual play/pause, paused/playing seek, stop/frame zero, loop/end, mute/volume, replacement, clear and failure→valid-source recovery |
| Video ordering | Reference fixtures cover inline/deferred preparation, synchronous commands during readiness, overlapping seeks, canceled/replaced creation, pending stop/disposal and late completions |
| Camera devices/consent | No device, first prompt, denial/restriction, busy, exact/preferred selection, hotplug/unplug, capability changes and permission revocation are checked on the actual platform |
| Camera lifetime/photo | Start/stop/restart, pending-start abandonment, facing/device switch, pending photo, detach/reactivation and fatal error release native capture; encoded image independently decodes with correct dimensions/orientation/content type |
| QR behavior | Known raw/text payloads, supported encodings, malformed/unsupported payloads, multiple symbols, duplicate/loss/stability, feedback suppression and first-result stop match the selected declared policy |
| QR geometry | Independent fixture corners remain aligned through crop, rotation, mirror, fit/fill/letterbox, layout/scale changes and an optional planar projective transform; stale generation/age is rejected |
| QR resource separation | Slow/error/canceled recognition does not stall GPU preview; analysis leases/rate/dimensions and any necessary CPU sampling/readback are bounded, measured and reported independently |
| 3D host | Independent renderer shows opaque and transparent output, consumes a camera/video texture, resizes, obeys invalidation/continuous policy, maps input coordinates and retires without stale texture sampling |
| UI/AX | Native keyboard/pointer/gesture/focus tests exercise actual controls; accessibility checks cover names/roles/states/value actions and status announcements; headless peer tests alone do not establish native accessibility |
| Independent integration | An independent sample performs real native playback, camera preview/capture and QR-result handling, with navigation/back/deactivation and final resource release; application migration is separate |
| Performance | Fixed resolution/frame-rate/device/renderer runs record CPU/GPU time, frame latency, drops, copy counters, allocations, retained bytes, startup time, thermal behavior and recovery; comparisons state the baseline and hardware |
| Packaging/platform | Actual selected OS minimums, native imports, permissions/entitlements, ABI/page-size/alignment, offline native-recognizer availability and signed host packaging are verified per release profile |

Reference .NET tools may generate immutable first-party algorithm/interaction
fixtures in development tooling only. Keep original source hashes, exact harness
commands, repeated outputs and narrowly documented intentional differences.
They must not be shipped as a runtime compatibility layer. GPU pixels need an
independent native/software reference where practical; a screenshot of the same
implementation is not by itself an independent correctness oracle.

For each gate preserve source revision, command, native engine/renderer/device,
actual platform/OS, artifact hashes, terminal outcome, raw failures and cleanup
checks. Distinguish an observed pass from a build, a unit simulation, a recording
provider or an injected native frame. Validate negative controls before relying
on a green result. Retain a strict failing source-image comparison as failed,
even when a narrower lifecycle comparison passes.

### 11.2 Platform profile evidence

| Profile | New implementation acceptance needed; current evidence is only a starting point |
| --- | --- |
| Windows | Native GPU video and camera, device/privacy/hotplug, shared resource synchronization, 2D/3D texture reuse, bounded QR analysis, separate fallback checks and actual native input/accessibility |
| macOS | Metal/Skia Graphite media, physical camera/permission/photo/QR, actual audio, same-device constraints, pending GPU read retirement and oldest declared OS; optional CGL/Ganesh media has independent native acceptance and requires its explicit build configuration |
| iOS | Device camera/photo/QR/torch where present, playback/audio, rotation/suspension/re-entry, declared minimum OS version and negotiated capture formats, native GPU resources and actual device packaging; simulator results labeled separately |
| Android | Minimum supported API and current API, representative physical GPUs, CameraX/decoder/QR lifetimes, renderer compatibility, surface/activity recreation and active media/camera on an actual 16 KiB kernel |
| Linux | Native video pipeline and codecs, direct and chosen portal camera profiles, DMA-BUF/modifier/synchronization combinations, QR, device removal and actual compositor/window/backend behavior |
| Browser | Actual Chromium/Safari/Firefox on declared desktop/mobile hosts, permission/CORS/autoplay, actual camera/virtual-device distinction, selected WebGL/WebGPU bridge, native context loss and frame ownership |

Performance thresholds must be selected for named reference hardware and workload
before performance acceptance. A nominal 1080p/30 camera and 1080p/60 synthetic
producer are proposed baseline workloads, with lower negotiated capabilities
reported honestly. No unsupported device must be forced to meet a fabricated
frame rate. Functional safety/copy counters remain hard gates even when native
throughput is lower than the proposed performance target.

## 12. Framework delivery contract

This specification concerns the framework, its controls, providers and independent
integration samples. Application migration and changes to any server/backend are
outside its scope. Earlier application defaults, observation policies and event
ordering in this document are historical reference profiles, not generic framework
requirements. Cancellation/disposal safety remains mandatory.

### Public responsibilities and defaults

Use the selective MediaSource, MediaPlayer, MediaPlaybackSession,
MediaPlayerElement and MediaTransportControls responsibilities with idiomatic Rust
ownership, asynchronous work and disposable subscriptions. Reuse existing
composition surfaces and brushes. Camera sessions, optional QR recognition and a
renderer-neutral texture host share the embedding foundation. No general runtime
compatibility layer is required.

Use the `FerroUI.Media.*` crate family. `FerroUI.Media` is the shared base;
`FerroUI.Media.Video` provides playback and depends on Media, while
`FerroUI.Media.Camera` provides capture separately and also depends on Media.
Neither video nor camera requires the other. `FerroUI.Media.Qr`, visual controls
and platform providers are separate optional dependencies under the same family.
The shared base must not pull in native playback, capture or recognition code.

Generic defaults are automatic playback off, looping off, transport controls off,
aspect fit, volume 1, unmuted, GPU-preferred presentation, explicit camera start
and QR opt-in. Controls retire sessions they create when detached. A view of a
caller-owned session releases only its own subscriptions and frame leases; the
caller controls the shared session's lifetime. Source replacement clears previous
presentation immediately. Pause retains an owned frame. Stop requests frame zero
for seekable media, with a placeholder until that frame is available. Commands
during preparation update current intent; latest seek wins.

### Texture and platform profiles

Prefer direct native shared textures. An admitted GPU preview performs no
framework CPU pixel readback, packing, staging or upload. A required GPU copy or
conversion has an inspectable reason and counters. Existing snapshot importers
must not introduce avoidable copies. Resource retention follows actual producer
and consumer GPU completion, independently from render-job submission.

The initial compositor profiles are default Skia native backends (including
Metal/Graphite), the Vello backend's GPU modes on Metal (macOS) and browser WebGL.
Vello imports a shared frame as a texture of its device, converts it once on the
device into the canvas's premultiplied RGBA (BGRA storage, a bottom-up first row,
full-range Rec. 709 video; counted as a GPU conversion) and gives the frame's lease
back once its queue has done that pass. Vello on OpenGL, Vulkan and Direct3D 12,
browser Graphite/WebGPU and render-worker media bridges are deferred and must
report unavailable import capabilities.
Windows uses Media Foundation and compatible D3D11 surfaces/device management,
with native audio/timing. Apple uses AVFoundation/CoreVideo/Metal. Android uses
native playback, CameraX and external GLES textures. Linux uses system GStreamer
and its installed plugins, DMA-BUF negotiation, portal-authorized PipeWire and an
explicit direct V4L2 host profile. Browser media remains inside ordinary WebGL
composition, respecting permission, autoplay and origin restrictions.

### QR and renderer host

QR defaults to continuous multi-symbol results. Deduplication, feedback and
stop-after-acceptance are explicit options. Default analysis bounds are one job,
latest-frame dropping, 10 analyses per second and 1280 by 720 input. Results expire
after 250 ms for approximate live overlays; exact analyzed-frame presentation is
deferred. Prefer available native recognition and an optional Rust-only rxing
fallback. Android recognition must be bundled for offline availability. No QR
runtime model download or C++ QR dependency is required. Any necessary analysis
copy is counted independently of preview.

The renderer host provides invalidation/continuous scheduling, viewport/input
mapping and opaque/premultiplied transparent output. Native examples demonstrate
shared camera/video inputs. Clients own scenes, render libraries, hit testing and
non-planar mapping. Planar projective QR overlay is supported. A built-in scene
engine and maintained wgpu adapter are outside this delivery.

### Acceptance boundary

An independent integration sample must exercise real playback, camera capture,
QR handling, navigation and final cleanup; a catalog alone is insufficient.
Earlier source-application assets and behavioral fixtures may remain reference
inputs without requiring application migration or importing private assets.

Implementation status, missing native verification, performance/copy evidence
and accessibility/packaging acceptance remain separate. Each platform/renderer
profile is release-ready only after its own native gates pass. Builds, injected
frames, virtual cameras and simulators do not establish physical-device support.
Preserve framework OS floors with availability checks and report narrower media
capabilities explicitly. Freeze named reference hardware, workloads and performance
thresholds before performance acceptance.

Playlists, DRM, recording/editing, HDR, subtitle rendering, playback-rate features,
desktop capture and a general scene engine are deferred. GPU-preferred fallback
never closes a missing primary GPU implementation gate.
