//! Render data: the recorded form of what a visual draws, its resources,
//! and the operations on it (replay, bounds, hit testing).

mod composition_render_data;
mod compositor_resource_helpers;
mod i_composition_render_resource;
mod i_render_data_geometry;
mod i_render_data_visitor;
mod immediate_render_data_scene_brush_content;
mod render_data_drawing_context;
mod render_data_resources;
mod render_data_stream;
mod render_data_stream_bounds;
mod render_data_stream_hit_test;
mod render_data_stream_replay;
mod server_composition_render_data;
mod server_composition_simple_pen;
mod server_resource_helper_extensions;

pub use composition_render_data::CompositionRenderData;
pub use compositor_resource_helpers::{CompositorRefCountableResource, CompositorResourceHolder};
pub use i_composition_render_resource::ICompositionRenderResource;
pub use i_render_data_geometry::IRenderDataGeometry;
pub use i_render_data_visitor::IRenderDataVisitor;
pub use immediate_render_data_scene_brush_content::ImmediateRenderDataSceneBrushContent;
pub use render_data_drawing_context::RenderDataDrawingContext;
pub use render_data_resources::{RenderDataResource, RenderDataResources, NULL_HANDLE};
pub use render_data_stream::{RenderDataOp, RenderDataStream};
pub use render_data_stream_bounds::{BoundsScope, BoundsVisitor};
pub use render_data_stream_hit_test::{HitTestScope, HitTestVisitor};
pub use render_data_stream_replay::{ReplayScope, ReplayVisitor};
pub use server_composition_render_data::ServerCompositionRenderData;
pub(crate) use server_resource_helper_extensions::{brush_get_server_resource, transform_get_server};
pub use server_composition_simple_pen::ServerCompositionSimplePen;

#[cfg(test)]
mod render_data_tests;
#[cfg(test)]
mod render_resource_tests;
