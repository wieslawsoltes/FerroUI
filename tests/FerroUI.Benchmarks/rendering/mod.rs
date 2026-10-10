//! The rendering benchmarks.
//!
//! Not ported: the shape rendering benchmarks of upstream (the class
//! `ShapeRendering`: `Render_Line_NoBrushes`, `Render_Line_WithStroke`,
//! `Render_Line_WithFill`, `Render_Line_WithFillAndStroke`). They render a
//! line into the drawing context stub of the headless platform and register
//! the render interface of the headless platform. Both are private to the
//! headless crate here (`HeadlessDrawingContextStub` and
//! `HeadlessPlatformRenderInterface` in
//! `src/Headless/FerroUI.Headless/headless_platform_render_interface.rs`,
//! which the crate does not export, and the render interface has no public
//! constructor), and the benchmark crate does not depend on the headless
//! crate.

use crate::harness::Registry;

pub fn register(_registry: &mut Registry) {}
