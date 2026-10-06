//! Scene graph primitives shared by the drawing contexts and the render
//! data.

mod custom_draw_operation;
mod line_bounds_helper;

pub use custom_draw_operation::ICustomDrawOperation;
#[allow(unused_imports)]
pub(crate) use line_bounds_helper::LineBoundsHelper;

#[cfg(test)]
pub(crate) mod scene_graph_test_support;
#[cfg(test)]
mod draw_operation_tests;
#[cfg(test)]
mod render_data_resources_tests;
#[cfg(test)]
mod render_data_stream_bounds_tests;
#[cfg(test)]
mod render_data_stream_effect_tests;
#[cfg(test)]
mod render_data_stream_ellipse_hit_test_tests;
#[cfg(test)]
mod render_data_stream_hit_test_tests;
#[cfg(test)]
mod render_data_stream_line_hit_test_tests;
#[cfg(test)]
mod render_data_stream_serialization_tests;
#[cfg(test)]
mod render_data_stream_tests;
#[cfg(test)]
mod render_data_writer_reader_tests;
