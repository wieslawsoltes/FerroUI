use super::ServerCompositionRenderData;
use crate::media::{IBrush, IImmutableBrush, ISceneBrushContent, ITileBrush, ITransform};
use crate::platform::IDrawingContextImpl;
use crate::rendering::composition::server::{ServerCompositor, ServerObjectId};
use crate::rendering::composition::transport::{BatchStreamReader, BatchStreamWriter};
use crate::{Matrix, Rect, RelativePoint};
use std::any::Any;
use std::rc::Rc;

/// What a scene brush sends to its server-side counterpart: the render data
/// its content was recorded into, the bounds of the content (`None`: the
/// bounds of what was drawn) and how the content is rasterized.
#[derive(Clone)]
pub struct CompositionRenderDataSceneBrushContentProperties {
    pub render_data: Rc<ServerCompositionRenderData>,
    pub rect: Option<Rect>,
    pub use_scalable_rasterization: bool,
}

impl CompositionRenderDataSceneBrushContentProperties {
    pub fn new(render_data: Rc<ServerCompositionRenderData>, rect: Option<Rect>, use_scalable_rasterization: bool) -> Self {
        Self { render_data, rect, use_scalable_rasterization }
    }

    /// Writes the properties (or their absence) on the UI thread, where the
    /// render data is known by the id of its server object.
    pub fn serialize(writer: &mut BatchStreamWriter<'_>, properties: Option<(ServerObjectId, Option<Rect>, bool)>) {
        match properties {
            Some((render_data, rect, use_scalable_rasterization)) => {
                writer.write_server_object(Some(render_data));
                writer.write(rect);
                writer.write(use_scalable_rasterization);
            }
            None => writer.write_server_object(None),
        }
    }

    /// Reads what [`serialize`](Self::serialize) wrote, on the server.
    ///
    /// A scene brush creates the render data before its changes are
    /// written, so the render data exists when they are read. Should it not
    /// (or the compositor be gone), there are no properties: a brush
    /// without content is valid and draws nothing.
    pub fn deserialize(reader: &mut BatchStreamReader<'_>, compositor: Option<&Rc<ServerCompositor>>) -> Option<Self> {
        let id = reader.read_server_object()?;
        let rect = reader.read::<Option<Rect>>();
        let use_scalable_rasterization = reader.read::<bool>();
        let render_data = compositor?.get::<ServerCompositionRenderData>(id)?;
        Some(Self { render_data, rect, use_scalable_rasterization })
    }
}

/// The content of a scene brush on the server: the render data the UI
/// thread recorded for it.
pub struct CompositionRenderDataSceneBrushContent {
    render_data: Rc<ServerCompositionRenderData>,
    rect: Option<Rect>,
    brush: Rc<dyn ITileBrush>,
    use_scalable_rasterization: bool,
}

impl CompositionRenderDataSceneBrushContent {
    pub fn new(brush: Rc<dyn ITileBrush>, properties: &CompositionRenderDataSceneBrushContentProperties) -> Self {
        Self {
            brush,
            rect: properties.rect,
            use_scalable_rasterization: properties.use_scalable_rasterization,
            render_data: properties.render_data.clone(),
        }
    }

    /// The render data that is replayed.
    pub fn render_data(&self) -> &Rc<ServerCompositionRenderData> {
        &self.render_data
    }
}

impl IBrush for CompositionRenderDataSceneBrushContent {
    fn opacity(&self) -> f64 {
        self.brush.opacity()
    }

    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        self.brush.transform()
    }

    fn transform_origin(&self) -> RelativePoint {
        self.brush.transform_origin()
    }

    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.brush.relative_transform()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_immutable_brush(self: Rc<Self>) -> Option<Rc<dyn IImmutableBrush>> {
        Some(self)
    }
}

impl IImmutableBrush for CompositionRenderDataSceneBrushContent {}

impl ISceneBrushContent for CompositionRenderDataSceneBrushContent {
    fn brush(&self) -> Rc<dyn ITileBrush> {
        self.brush.clone()
    }

    fn rect(&self) -> Rect {
        self.rect.unwrap_or_else(|| self.render_data.bounds().map(|bounds| bounds.to_rect()).unwrap_or_default())
    }

    fn render(&self, context: &mut dyn IDrawingContextImpl, transform: Option<Matrix>) {
        match transform {
            Some(transform) => {
                let old_transform = context.transform();
                context.set_transform(transform * old_transform);
                self.render_data.render(context);
                context.set_transform(old_transform);
            }
            None => self.render_data.render(context),
        }
    }

    fn use_scalable_rasterization(&self) -> bool {
        self.use_scalable_rasterization
    }

    fn dispose(&self) {
        // No-op on server
    }
}
