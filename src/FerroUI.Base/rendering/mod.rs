//! Rendering contracts: the renderer of a visual tree and the presentation
//! source that hosts one.

pub mod composition;
pub mod scene_graph;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod utilities;

mod i_custom_hit_test;
mod i_hit_tester;
mod i_presentation_source;
mod i_renderer;
mod immediate_renderer;
mod scene_invalidated_event_args;
mod z_index_comparer;
mod managed_hit_tester;

pub use i_hit_tester::IHitTester;
pub use i_presentation_source::IPresentationSource;
pub use i_custom_hit_test::ICustomHitTest;
pub use i_renderer::IRenderer;
pub use immediate_renderer::ImmediateRenderer;
pub use scene_invalidated_event_args::SceneInvalidatedEventArgs;
pub use z_index_comparer::ZIndexComparer;
pub use managed_hit_tester::ManagedHitTester;

// ---- render loop / render timers / context manager ----
#[cfg(not(target_family = "wasm"))]
mod auto_reset_event;
mod default_render_timer;
mod i_render_loop;
mod i_render_loop_task;
mod i_render_timer;
mod layout_pass_timing;
mod owned_disposable;
mod platform_render_interface_context_manager;
mod render_loop;
mod renderer_debug_overlays;
mod renderer_diagnostics;
#[cfg(not(target_family = "wasm"))]
mod sleep_loop_render_timer;
mod swapchain_base;
#[cfg(not(target_family = "wasm"))]
mod thread_proxy_render_timer;
mod ui_thread_render_timer;

pub use default_render_timer::{DefaultRenderTimer, DefaultRenderTimerImpl, RenderTimerSubscription};
pub use i_render_loop::IRenderLoop;
pub use i_render_loop_task::IRenderLoopTask;
pub use i_render_timer::{IRenderTimer, RenderTimerTick};
pub use layout_pass_timing::LayoutPassTiming;
pub use owned_disposable::{OwnedDisposable, OwnedDisposableValue};
pub use platform_render_interface_context_manager::PlatformRenderInterfaceContextManager;
pub use render_loop::{DefaultRenderLoop, RenderLoop};
pub use renderer_debug_overlays::RendererDebugOverlays;
pub use renderer_diagnostics::RendererDiagnostics;
#[cfg(not(target_family = "wasm"))]
pub use sleep_loop_render_timer::SleepLoopRenderTimer;
pub use swapchain_base::{ISwapchainImage, SwapchainBase, SwapchainImageFactory, SwapchainImagePresentStatus};
#[cfg(not(target_family = "wasm"))]
pub use thread_proxy_render_timer::ThreadProxyRenderTimer;
pub use ui_thread_render_timer::UiThreadRenderTimer;
