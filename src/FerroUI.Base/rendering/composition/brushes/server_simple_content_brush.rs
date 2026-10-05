//! The server-side counterpart of the mutable scene brushes (the visual
//! brush and the drawing brush).

use crate::rendering::composition::server::{
    impl_simple_server_render_resource, server_simple_brush, IServerObject, IServerRenderResource,
    IServerRenderResourceObserver, ServerCompositor, SimpleServerRenderResource,
};
use crate::media::{
    AlignmentX, AlignmentY, IBrush, ISceneBrush, ISceneBrushContent, ITileBrush, ITransform, Stretch, TileMode,
};
use crate::rendering::composition::drawing::{
    CompositionRenderDataSceneBrushContent, CompositionRenderDataSceneBrushContentProperties,
};
use crate::rendering::composition::generated::{
    ServerCompositionSimpleBrushHooks, ServerCompositionSimpleBrushProps, ServerCompositionSimpleTileBrushHooks,
    ServerCompositionSimpleTileBrushProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{RelativePoint, RelativeRect};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

server_simple_brush! {
    /// The server-side counterpart of a mutable scene brush: the tile brush
    /// properties and the render data the content was recorded into.
    ServerCompositionSimpleContentBrush {
        props: ServerCompositionSimpleTileBrushProps = ServerCompositionSimpleTileBrushProps::new(),
        content: RefCell<Option<CompositionRenderDataSceneBrushContentProperties>> = RefCell::new(None),
    }
    brush: |b| b.props.base();
    deserialize: |d, reader, committed_at| {
        d.props.deserialize_changes_core(d, reader, committed_at);
        let content =
            CompositionRenderDataSceneBrushContentProperties::deserialize(reader, d.base.compositor().as_ref());
        d.set_content(content);
    };
    find: |f, type_id| f.props.find_props(type_id);
    dispose: |x| x.remove_content_observer();
    as: { as_tile_brush -> ITileBrush, as_scene_brush -> ISceneBrush }
}

impl ServerCompositionSimpleTileBrushHooks for ServerCompositionSimpleContentBrush {}

impl ServerCompositionSimpleContentBrush {
    /// The properties of the brush.
    pub fn props(&self) -> &ServerCompositionSimpleTileBrushProps {
        &self.props
    }

    fn as_observer(&self) -> Option<Rc<dyn IServerRenderResourceObserver>> {
        self.to_rc().map(|this| this as Rc<dyn IServerRenderResourceObserver>)
    }

    /// Replaces the content; the brush observes the render data of its
    /// content, so that a change of what was recorded (or of a resource it
    /// was recorded with) invalidates whatever draws with the brush.
    fn set_content(&self, content: Option<CompositionRenderDataSceneBrushContentProperties>) {
        let old = self.content.borrow().as_ref().map(|content| content.render_data.clone());
        let new = content.as_ref().map(|content| content.render_data.clone());
        let same = match (&old, &new) {
            (Some(old), Some(new)) => Rc::ptr_eq(old, new),
            (None, None) => true,
            _ => false,
        };
        if !same {
            if let Some(observer) = self.as_observer() {
                if let Some(old) = &old {
                    old.remove_observer(&observer);
                }
                if let Some(new) = &new {
                    new.add_observer(&observer);
                }
            }
        }

        *self.content.borrow_mut() = content;
    }

    fn remove_content_observer(&self) {
        let render_data = self.content.borrow().as_ref().map(|content| content.render_data.clone());
        if let (Some(render_data), Some(observer)) = (render_data, self.as_observer()) {
            render_data.remove_observer(&observer);
        }
    }
}

impl ITileBrush for ServerCompositionSimpleContentBrush {
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

impl ISceneBrush for ServerCompositionSimpleContentBrush {
    fn create_content(&self) -> Option<Rc<dyn ISceneBrushContent>> {
        let content = self.content.borrow();
        let content = content.as_ref()?;
        if content.render_data.is_disposed() {
            return None;
        }
        let this: Rc<dyn ITileBrush> = self.to_rc()?;
        Some(Rc::new(CompositionRenderDataSceneBrushContent::new(this, content)))
    }
}
