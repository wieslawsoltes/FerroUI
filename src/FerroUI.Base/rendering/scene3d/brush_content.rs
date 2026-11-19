//! The content of a visual or drawing brush of a 3D material (#608, step D6 of
//! `docs/porting/media3d.md`, section 3.16): what the render thread renders offscreen into the
//! texture the material samples.
//!
//! A `VisualBrush` or a `DrawingBrush` belongs to the UI thread, and so does what it records (a
//! render data stream over `Rc` resources). A [`Scene3DBrushContent`] is that recording made
//! `Send + Sync`: the operations as they were recorded, and each resource in a form that crosses
//! threads (a brush or a pen by its shared values, a geometry by its platform geometry, glyph
//! runs, bitmaps, effects, custom operations and 3D scenes as the shared handles they already
//! are). The render thread replays it into a drawing context of its own (an offscreen target of
//! the backend) with [`Scene3DBrushContent::render`].
//!
//! A content has an identity and a generation: a renderer keeps the texture it rendered for the
//! pair and renders again only for a new generation (the content changed). The 3D model records a
//! new generation only when the brush's content is dirty, not for every snapshot.
//!
//! What a recording cannot carry is left out of it: a nested visual or drawing brush (no shared
//! form) draws without its brush.

use crate::media::{ISceneBrush, SharedBrush, SharedPen, shared_brush_of, shared_pen_of};
use crate::platform::{IDrawingContextImpl, IGeometryImpl, IGlyphRunImpl, SharedBitmapImpl};
use crate::rendering::composition::drawing::{
    ImmediateRenderDataSceneBrushContent, RenderDataResource, RenderDataStream,
};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::rendering::scene3d::Scene3DSnapshot;
use crate::utilities::RefCounted;
use crate::{Matrix, Rect};
use std::rc::Rc;
use std::sync::Arc;

/// A resource of a recording in a form that crosses threads.
enum SharedResource {
    /// A slot whose resource is not carried (a nested scene brush, a client-only pen).
    None,
    Brush(SharedBrush),
    Pen(SharedPen),
    GeometryImpl(Arc<dyn IGeometryImpl>),
    GlyphRun(Arc<Arc<dyn IGlyphRunImpl>>),
    Bitmap(Arc<RefCounted<SharedBitmapImpl>>),
    Custom(Arc<dyn ICustomDrawOperation>),
    Effect(Arc<dyn crate::media::IImmutableEffect>),
    Scene3D(Arc<Scene3DSnapshot>),
}

impl SharedResource {
    /// The shared form of a resource of an immediate recording.
    fn of(resource: &RenderDataResource) -> SharedResource {
        match resource {
            RenderDataResource::Brush(brush) => {
                shared_brush_of(&**brush).map_or(SharedResource::None, SharedResource::Brush)
            }
            RenderDataResource::Pen(pen) | RenderDataResource::ClientPen(pen) => {
                shared_pen_of(&**pen).map_or(SharedResource::None, SharedResource::Pen)
            }
            RenderDataResource::GeometryImpl(geometry) => SharedResource::GeometryImpl(geometry.clone()),
            RenderDataResource::Geometry(geometry) => {
                geometry.geometry_impl().map_or(SharedResource::None, SharedResource::GeometryImpl)
            }
            RenderDataResource::GlyphRun(run) => SharedResource::GlyphRun(run.clone()),
            RenderDataResource::Bitmap(bitmap) => SharedResource::Bitmap(bitmap.clone()),
            RenderDataResource::CustomDrawOperation(operation) => SharedResource::Custom(operation.clone()),
            RenderDataResource::Effect(effect) => SharedResource::Effect(effect.clone()),
            RenderDataResource::Scene3D(scene) => SharedResource::Scene3D(scene.clone()),
            // An immediate recording has no server objects.
            RenderDataResource::ServerBrush { .. }
            | RenderDataResource::ServerPen { .. }
            | RenderDataResource::ServerGeometry { .. }
            | RenderDataResource::NotSent => SharedResource::None,
        }
    }

    /// The resource of a stream replayed on this thread.
    fn to_resource(&self) -> RenderDataResource {
        match self {
            SharedResource::None => RenderDataResource::NotSent,
            SharedResource::Brush(brush) => RenderDataResource::Brush(Rc::new(brush.clone())),
            SharedResource::Pen(pen) => RenderDataResource::Pen(Rc::new(pen.clone())),
            SharedResource::GeometryImpl(geometry) => RenderDataResource::GeometryImpl(geometry.clone()),
            SharedResource::GlyphRun(run) => RenderDataResource::GlyphRun(run.clone()),
            SharedResource::Bitmap(bitmap) => RenderDataResource::Bitmap(bitmap.clone()),
            SharedResource::Custom(operation) => RenderDataResource::CustomDrawOperation(operation.clone()),
            SharedResource::Effect(effect) => RenderDataResource::Effect(effect.clone()),
            SharedResource::Scene3D(scene) => RenderDataResource::Scene3D(scene.clone()),
        }
    }
}

struct ContentValues {
    id: u64,
    generation: u64,
    rect: Rect,
    max_depth: i32,
    opcodes: Vec<u8>,
    resources: Vec<SharedResource>,
}

/// The recorded content of a visual or drawing brush, shared with the render thread: see the
/// documentation of the module.
#[derive(Clone)]
pub struct Scene3DBrushContent(Arc<ContentValues>);

// The content is recorded once and never changed: a panic cannot leave it half written. So a
// brush or a 3D scene that holds one stays unwind safe, as it was before scene brushes.
impl std::panic::UnwindSafe for Scene3DBrushContent {}
impl std::panic::RefUnwindSafe for Scene3DBrushContent {}

impl Scene3DBrushContent {
    /// Records the current content of `brush` (a `VisualBrush`: the rendering of its visual; a
    /// `DrawingBrush`: its drawing) as the content `id` at `generation`. `None` when the brush
    /// draws nothing.
    pub fn record(brush: &dyn ISceneBrush, id: u64, generation: u64) -> Option<Scene3DBrushContent> {
        let content = brush.create_content()?;
        let recorded = content.as_any().downcast_ref::<ImmediateRenderDataSceneBrushContent>().and_then(|immediate| {
            immediate.with_stream(|stream| {
                (stream.max_depth(), stream.opcodes().to_vec(), stream.resources().map(SharedResource::of).collect())
            })
        });
        let rect = content.rect();
        content.dispose();
        let (max_depth, opcodes, resources) = recorded?;
        Some(Scene3DBrushContent(Arc::new(ContentValues { id, generation, rect, max_depth, opcodes, resources })))
    }

    /// The identity of the content: the same for every generation of one brush of one material.
    pub fn id(&self) -> u64 {
        self.0.id
    }

    /// The generation of the content: new when the content was recorded again.
    pub fn generation(&self) -> u64 {
        self.0.generation
    }

    /// The bounds of the content, in its own coordinates: the box the brush's viewbox is relative
    /// to, as a 2D scene brush takes it.
    pub fn rect(&self) -> Rect {
        self.0.rect
    }

    /// Replays the content into `context` under `transform` (from the coordinates of the content
    /// to those of the context): on the thread that renders it.
    pub fn render(&self, context: &mut dyn IDrawingContextImpl, transform: Matrix) {
        let values = &*self.0;
        let stream = RenderDataStream::from_parts(
            values.max_depth,
            values.resources.iter().map(SharedResource::to_resource),
            &values.opcodes,
        );
        let saved = context.transform();
        context.set_transform(transform * saved);
        stream.replay(context);
        context.set_transform(saved);
    }
}

impl std::fmt::Debug for Scene3DBrushContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene3DBrushContent")
            .field("id", &self.0.id)
            .field("generation", &self.0.generation)
            .field("rect", &self.0.rect)
            .finish()
    }
}

/// Compile-time proof that a content can be sent to the render thread.
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<Scene3DBrushContent>();
};

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use crate::media::{Brushes, DrawingBrush, GeometryDrawing, IBrush, RectangleGeometry, Stretch};
    use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl, MockPlatformRenderInterface};

    #[test]
    fn a_drawing_brush_is_recorded_with_shared_resources_and_replayed_anywhere() {
        let (_scope, _) = MockPlatformRenderInterface::install();
        let brush = DrawingBrush::new();
        let drawing = GeometryDrawing::new();
        drawing.set_brush(Some(Brushes::red()));
        drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(1.5, 2.5, 10.0, 20.0)).upcast::<crate::media::Geometry>());
        brush.set_drawing(&drawing);
        brush.set_stretch(Stretch::Fill);
        let handle: Rc<dyn IBrush> = brush.clone().into();
        let scene = handle.as_scene_brush().expect("a scene brush");

        let content = Scene3DBrushContent::record(scene, 7, 3).expect("the drawing draws something");
        assert_eq!((7, 3), (content.id(), content.generation()));
        assert_eq!(Rect::new(1.0, 2.0, 11.0, 21.0), content.rect());

        // Replayed on another thread than the one that recorded it, under a transform.
        let replayed = std::thread::spawn(move || {
            let log = DrawingLog::new();
            let mut context = MockDrawingContextImpl::new(log.clone());
            content.render(&mut context, Matrix::create_scale(2.0, 2.0));
            log.entries()
        })
        .join()
        .expect("the replay");
        assert!(replayed.iter().any(|entry| entry == "DrawGeometry Red none 1.5, 2.5, 10, 20"), "{replayed:?}");

        // The shared form of the brush over the content: a tile brush with the brush's settings.
        let content = Scene3DBrushContent::record(scene, 7, 4).expect("recorded again");
        let shared = SharedBrush::from_scene_content(&*handle, content).expect("a tile brush");
        assert_eq!(Some(4), shared.scene_content().map(Scene3DBrushContent::generation));
        assert_eq!(Stretch::Fill, shared.as_tile_brush().expect("tile settings").stretch());
        assert!(shared.as_image_brush().is_none());
    }

    #[test]
    fn a_drawing_brush_reports_the_changes_of_its_drawing() {
        let (_scope, _) = MockPlatformRenderInterface::install();
        let brush = DrawingBrush::new();
        let drawing = GeometryDrawing::new();
        brush.set_drawing(&drawing);
        let handle: Rc<dyn IBrush> = brush.into();
        let changes = Rc::new(std::cell::Cell::new(0));
        let counted = changes.clone();
        let subscription = handle
            .as_scene_brush()
            .and_then(|scene| scene.subscribe_content_changed(Rc::new(move || counted.set(counted.get() + 1))))
            .expect("a drawing brush follows its drawing");
        drawing.set_brush(Some(Brushes::red()));
        assert!(changes.get() > 0);
        drop(subscription);
    }
}
