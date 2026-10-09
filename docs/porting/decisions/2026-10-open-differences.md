# Open differences prepared for a decision (October 2026)

Each section states what upstream does, what the port does, what an application could observe, what an exact port would cost, and a recommendation. Nothing here changes behaviour: the code is as the sections describe it until the owner decides.

Upstream files and lines are those of the reference checkout as it stood on 2026-10-09 (`16572aef`, ahead of the tracked commit `17350180`); the members quoted are the same at the tracked commit unless a section says otherwise. Written without a build or a test run.

## 1. `IPlatformHandle` and `PlatformHandle` are declared twice, and the two have diverged

**Upstream.** One of each, in the base library: `src/Avalonia.Base/Platform/IPlatformHandle.cs` (lines 8 to 19: `Handle` and `HandleDescriptor`) and `src/Avalonia.Base/Platform/PlatformHandle.cs`. `Avalonia.Controls/Platform/` has neither. The controls library adds a contract that extends the handle, `INativeControlHostDestroyableControlHandle : IPlatformHandle` (`Platform/INativeControlHostImpl.cs`), and `NativeControlHost.DestroyNativeControlCore` reaches it with a cast (`src/Avalonia.Controls/NativeControlHost.cs`, lines 223 to 229: `if (control is INativeControlHostDestroyableControlHandle h) h.Destroy();`).

**The port.** Two traits and two structs:

| | `src/FerroUI.Base/platform/` | `src/FerroUI.Controls/platform/` |
|---|---|---|
| `IPlatformHandle` (`i_platform_handle.rs`) | `handle`, `handle_descriptor`, `as_any`, `equals` (identity by default) | the same four, and `as_native_control_host_destroyable_control_handle(&self) -> Option<&dyn INativeControlHostDestroyableControlHandle>` (`None` by default) |
| `PlatformHandle` (`platform_handle.rs`) | value handle; implements the trait of the base crate | the same text, with the same three tests; implements the trait of the controls crate |

The extra member is the port's form of upstream's cast: Rust cannot cast a trait object to another trait, so the handle answers the question itself. It names a type of the controls crate, which the base crate cannot name, and that is why the second declaration exists. The two traits are unrelated types: a `Rc<dyn ferroui_controls::platform::IPlatformHandle>` is not a `Rc<dyn ferroui_base::platform::IPlatformHandle>`, and the two structs called `PlatformHandle` are different types too.

**Who depends on which.** No file uses both.

- The trait and the struct of the base crate, the handles of GPU objects handed over for an import: `src/FerroUI.Base/platform/i_external_objects_render_interface_context_feature.rs`, `src/FerroUI.Base/rendering/composition/composition_interop.rs`, `composition_external_memory.rs`, `composition_drawing_surface_tests.rs`; `src/FerroUI.OpenGL/i_gl_context_external_objects_feature.rs`, `features/external_objects_open_gl_extension_feature.rs`, `egl/egl_external_objects_feature.rs`, `egl/egl_external_objects_feature_drm.rs`; `src/Skia/FerroUI.Skia/metal/i_metal_external_objects_feature.rs`, `gpu/metal/skia_metal_external_objects_feature.rs`, `gpu/metal/tests.rs`, `gpu/open_gl/gl_skia_external_objects_feature.rs`; `src/FerroUI.Native/metal.rs`, `gpu_handle_wrap_feature.rs`.
- The trait and the struct of the controls crate, the handles of windows, views and native controls: `src/FerroUI.Controls/top_level.rs`, `native_control_host.rs`, `platform/i_top_level_impl.rs`, `platform/i_native_control_host_impl.rs`, `platform/i_platform_native_surface_handle.rs`, `platform/i_screen_impl.rs`, `platform/i_screen_impl_tests.rs`, `platform/platform_manager_tests.rs`, `platform/screen.rs`, `embedding/offscreen/offscreen_top_level_impl.rs`, `automation/peers/interop_automation_peer.rs`, `testing/mock_window_impl.rs`, `storage_misc_tests.rs`; `src/FerroUI.Native/top_level_impl.rs`, `native_control_host_impl.rs`, `screen_impl.rs`; `src/Browser/FerroUI.Browser/browser_top_level_impl.rs`, `browser_native_control_host.rs`, `js_object_control_handle.rs`; `src/Headless/FerroUI.Headless/headless_window_impl.rs`, `headless_platform_stubs.rs`; `samples/ControlCatalog/Pages/native_embed_page.rs`, `samples/ControlCatalog.Browser/embed_sample_browser.rs`.
- The extra member is overridden by `JsObjectControlHandle` (browser) and by the handle of a test (`storage_misc_tests.rs`), and called in one place, `NativeControlHost::destroy_native_control_core`.

**What an application could observe.** A handle of a window or of a native control (`TopLevel::try_get_platform_handle`, the handle of a `NativeControlHost`) cannot be passed where a GPU handle is expected (`ICompositionGpuInterop::import_image`) without building a second `PlatformHandle` of the other crate from its value and descriptor; upstream passes the one object. Nothing in the tree does this today. Two public types with one name, `ferroui_base::platform::PlatformHandle` and `ferroui_controls::platform::PlatformHandle`, that do not convert into each other.

**A fault the second declaration hid** (fixed on this branch, in a commit of its own): `DestroyableNSView` of the macOS backend implemented `INativeControlHostDestroyableControlHandle` and did not override the extra member, so the cast of `destroy_native_control_core` answered `None` and the default child view of a native control host was never destroyed. Upstream's cast cannot be forgotten by an implementation; the port's hook can.

**What one declaration would cost.** The base trait cannot name the controls contract, so the cast has to become a question the base crate can ask without it. The form the code base already has for that is the one of `IOptionalFeatureProvider`: a provided member of the base trait that is asked by type, for example `fn try_cast(self: Rc<Self>, type_id: TypeId) -> Option<Rc<dyn Any>>` returning `None` by default, with a typed helper next to it, and `INativeControlHostDestroyableControlHandle` asked for through it. Then:

- `src/FerroUI.Controls/platform/i_platform_handle.rs` and `platform_handle.rs` are removed and `platform/mod.rs` re-exports the two names from the base crate, so no public path changes and the twenty-three files of the second list keep their imports.
- The three implementations of the destroyable contract (`JsObjectControlHandle`, `DestroyableNSView`, the handle of the test) answer the new member instead of the old one; `destroy_native_control_core` asks it. Four files with a change of substance.
- The `rust_only` entry of `path-overrides.toml` for `src/FerroUI.Controls/platform/*platform_handle.rs` goes; the three tests of the struct stay once, in the base crate.
- Tests: the existing ones of the native control host (`storage_misc_tests.rs`) cover the cast; one new test that a handle which does not answer is not destroyed, and one that a window handle is accepted by an import.
- No effect on performance.

The smaller step, keeping two traits and one struct (the controls crate implementing its trait for the struct of the base crate), removes the duplicated struct and its tests and leaves the two traits; it does not remove the fault above.

**Recommendation.** Merge into the base declaration with a cast by type on the base trait, in a change of its own that is built and tested (it touches the three platforms). Until then the two stay, and every new implementation of `INativeControlHostDestroyableControlHandle` has to override the extra member.
