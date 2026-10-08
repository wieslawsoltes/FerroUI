//! The server-side counterpart of the mutable image brush.

use crate::rendering::composition::server::{
    impl_simple_server_render_resource, server_simple_brush, IServerObject, IServerRenderResource, ServerCompositor,
    SimpleServerRenderResource,
};
use crate::media::{AlignmentX, AlignmentY, IBrush, IImageBrush, IImageBrushSource, ITileBrush, ITransform, Stretch, TileMode};
use crate::platform::IBitmapImpl;
use crate::rendering::composition::generated::{
    ServerCompositionSimpleBrushHooks, ServerCompositionSimpleBrushProps, ServerCompositionSimpleTileBrushHooks,
    ServerCompositionSimpleTileBrushProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::utilities::RefCounted;
use crate::{RelativePoint, RelativeRect};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The image of a server-side image brush: the reference to the platform
/// bitmap the UI thread cloned for the server.
///
/// The server-side brush is its own image source upstream. Here the source
/// is an object of its own, replaced with every change of the brush, so
/// that it can be handed out as a shared handle and can lend its reference.
struct ServerImageBrushSource {
    bitmap: Option<RefCounted<crate::platform::SharedBitmapImpl>>,
}

impl IImageBrushSource for ServerImageBrushSource {
    fn bitmap(&self) -> Option<&RefCounted<crate::platform::SharedBitmapImpl>> {
        self.bitmap.as_ref().filter(|bitmap| bitmap.is_alive())
    }
}

server_simple_brush! {
    /// The server-side counterpart of a mutable image brush.
    ServerCompositionSimpleImageBrush {
        props: ServerCompositionSimpleTileBrushProps = ServerCompositionSimpleTileBrushProps::new(),
        source: RefCell<Rc<ServerImageBrushSource>> = RefCell::new(Rc::new(ServerImageBrushSource { bitmap: None })),
    }
    brush: |b| b.props.base();
    deserialize: |d, reader, committed_at| {
        d.props.deserialize_changes_core(d, reader, committed_at);
        d.release_bitmap();
        let bitmap = reader.read_value::<RefCounted<crate::platform::SharedBitmapImpl>>();
        *d.source.borrow_mut() = Rc::new(ServerImageBrushSource { bitmap });
    };
    find: |f, type_id| f.props.find_props(type_id);
    dispose: |x| x.release_bitmap();
    as: { as_tile_brush -> ITileBrush, as_image_brush -> IImageBrush }
}

impl ServerCompositionSimpleTileBrushHooks for ServerCompositionSimpleImageBrush {}

impl ServerCompositionSimpleImageBrush {
    /// The properties of the brush.
    pub fn props(&self) -> &ServerCompositionSimpleTileBrushProps {
        &self.props
    }

    /// The platform bitmap the brush draws, while it has one.
    pub fn bitmap(&self) -> Option<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        self.source.borrow().get_bitmap()
    }

    /// Releases the reference to the bitmap. A source handed out earlier
    /// has no bitmap from then on.
    fn release_bitmap(&self) {
        let source = std::mem::replace(&mut *self.source.borrow_mut(), Rc::new(ServerImageBrushSource { bitmap: None }));
        if let Some(bitmap) = &source.bitmap {
            bitmap.dispose();
        }
    }
}

impl ITileBrush for ServerCompositionSimpleImageBrush {
    fn alignment_x(&self) -> AlignmentX {
        self.props.alignment_x()
    }

    fn alignment_y(&self) -> AlignmentY {
        self.props.alignment_y()
    }

    fn destination_rect(&self) -> RelativeRect {
        self.props.destination_rect()
    }

    fn source_rect(&self) -> RelativeRect {
        self.props.source_rect()
    }

    fn stretch(&self) -> Stretch {
        self.props.stretch()
    }

    fn tile_mode(&self) -> TileMode {
        self.props.tile_mode()
    }
}

impl IImageBrush for ServerCompositionSimpleImageBrush {
    fn source(&self) -> Option<Rc<dyn IImageBrushSource>> {
        Some(self.source.borrow().clone() as Rc<dyn IImageBrushSource>)
    }
}
