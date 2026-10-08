use super::drawing::CompositionRenderData;
use super::generated::{
    CompositionContainerVisualProps, CompositionExperimentalAcrylicVisualHooks,
    CompositionExperimentalAcrylicVisualProps,
};
use super::server::{IServerVisualContent, ServerCompositionDrawListVisual, ServerCompositionExperimentalAcrylicVisual};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::visual::CompositionVisualKind;
use super::{CompositionVisual, Compositor};
use crate::media::{Geometry, IntersectionResult};
use crate::{Point, Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// What a class derived from the draw list visual outside of this module
/// (the custom composition visual of a control) adds to its UI-thread side:
/// the serialization of its own changes.
pub trait ICompositionDrawListVisualExtension: 'static {
    /// `SerializeChangesCore` of the derived class: called after the draw
    /// list visual has written its own changes. What is written here is read
    /// by the server content of the visual, after the changes of the draw
    /// list visual.
    fn serialize_changes_core(&self, writer: &mut BatchStreamWriter<'_>);
}

/// What a draw list visual adds to a visual: the visual of the tree it
/// renders, and what that visual drew.
pub(crate) struct DrawListData {
    props: CompositionContainerVisualProps,
    visual: WeakRef<Visual>,
    draw_list_changed: Cell<bool>,
    draw_list: RefCell<Option<Rc<CompositionRenderData>>>,
    /// The properties of the acrylic visual, when the visual is one.
    pub(super) acrylic: Option<CompositionExperimentalAcrylicVisualProps>,
    /// The part of a derived class defined outside of this module.
    extension: Option<Rc<dyn ICompositionDrawListVisualExtension>>,
}

impl DrawListData {
    pub(super) fn props(&self) -> &CompositionContainerVisualProps {
        &self.props
    }

    pub(super) fn initialize_defaults(&self, host: &CompositionVisual) {
        self.props.initialize_defaults(host);
        if let Some(acrylic) = &self.acrylic {
            acrylic.initialize_own_defaults(host);
        }
    }

    fn serialize_draw_list(&self, host: &CompositionVisual, writer: &mut BatchStreamWriter<'_>) {
        writer.write(u8::from(self.draw_list_changed.get()));
        if self.draw_list_changed.get() {
            writer.write_server_object(self.draw_list.borrow().as_ref().map(|draw_list| draw_list.server()));
            self.draw_list_changed.set(false);
        }
        self.props.serialize_changes_core(host, writer);
    }

    pub(super) fn serialize_changes_core(&self, host: &CompositionVisual, writer: &mut BatchStreamWriter<'_>) {
        match &self.acrylic {
            // The acrylic properties follow those of the draw list visual,
            // which the generated code writes through the base hook.
            Some(acrylic) => acrylic.serialize_changes_core(host, writer),
            None => {
                self.serialize_draw_list(host, writer);
                if let Some(extension) = &self.extension {
                    extension.serialize_changes_core(writer);
                }
            }
        }
    }

    pub(super) fn hit_test(&self, pt: Point) -> bool {
        let custom = self.visual.upgrade().and_then(|visual| visual.custom_hit_test(pt));
        let draw_list = self.draw_list.borrow().clone();
        if draw_list.is_none() && custom.is_none() {
            return false;
        }
        if let Some(custom) = custom {
            return custom;
        }
        draw_list.is_some_and(|draw_list| draw_list.hit_test(pt))
    }

    pub(super) fn hit_test_geometry(&self, geometry: &Ref<Geometry>) -> IntersectionResult {
        let custom = self.visual.upgrade().and_then(|visual| visual.custom_hit_test_geometry(geometry));
        let draw_list = self.draw_list.borrow().clone();
        if draw_list.is_none() && custom.is_none() {
            return IntersectionResult::Empty;
        }
        if let Some(custom) = custom {
            return custom;
        }
        draw_list.map_or(IntersectionResult::Empty, |draw_list| draw_list.hit_test_geometry(geometry))
    }
}

impl CompositionExperimentalAcrylicVisualHooks for CompositionVisual {
    fn base_serialize_changes_core(&self, writer: &mut BatchStreamWriter<'_>) {
        if let CompositionVisualKind::DrawList(data) = &self.kind {
            data.serialize_draw_list(self, writer);
        }
    }
}

/// The composition visual of a visual of the visual tree: draws what the
/// visual rendered.
#[derive(Clone)]
pub struct CompositionDrawListVisual(Rc<CompositionVisual>);

impl Deref for CompositionDrawListVisual {
    type Target = Rc<CompositionVisual>;

    fn deref(&self) -> &Rc<CompositionVisual> {
        &self.0
    }
}

impl CompositionDrawListVisual {
    /// Creates the composition visual of `visual`.
    pub fn new(compositor: &Rc<Compositor>, visual: &Visual) -> CompositionDrawListVisual {
        Self::create(compositor, visual, false)
    }

    /// Creates the composition visual of `visual` for a class derived from
    /// the draw list visual: `server_content` creates the server-side
    /// counterpart (which wraps a `ServerCompositionDrawListVisual` and
    /// forwards to it), `extension` serializes what the derived class adds.
    pub fn with_extension(
        compositor: &Rc<Compositor>,
        visual: &Visual,
        server_content: impl FnOnce() -> Box<dyn IServerVisualContent> + Send + 'static,
        extension: Rc<dyn ICompositionDrawListVisualExtension>,
    ) -> CompositionDrawListVisual {
        Self::create_core(compositor, visual, false, Some(extension), server_content)
    }

    pub(super) fn create(compositor: &Rc<Compositor>, visual: &Visual, acrylic: bool) -> CompositionDrawListVisual {
        Self::create_core(compositor, visual, acrylic, None, move || {
            if acrylic {
                Box::new(ServerCompositionExperimentalAcrylicVisual::new())
            } else {
                Box::new(ServerCompositionDrawListVisual::new())
            }
        })
    }

    fn create_core(
        compositor: &Rc<Compositor>,
        visual: &Visual,
        acrylic: bool,
        extension: Option<Rc<dyn ICompositionDrawListVisualExtension>>,
        server_content: impl FnOnce() -> Box<dyn IServerVisualContent> + Send + 'static,
    ) -> CompositionDrawListVisual {
        let data = DrawListData {
            props: CompositionContainerVisualProps::new(),
            visual: visual.to_ref().downgrade(),
            draw_list_changed: Cell::new(false),
            draw_list: RefCell::new(None),
            acrylic: acrylic.then(CompositionExperimentalAcrylicVisualProps::new),
            extension,
        };
        let result = CompositionVisual::create(compositor, CompositionVisualKind::DrawList(data), server_content);
        // A visual with a custom hit test decides for itself whether it is
        // hit, whatever its bounds are.
        let has_custom_hit_test = visual.custom_hit_test(Point::default()).is_some();
        result.custom_hit_test_count_in_sub_tree.set(i32::from(has_custom_hit_test));
        CompositionDrawListVisual(result)
    }

    /// The handle of `visual` as a draw list visual, if it is one.
    pub fn from_visual(visual: &Rc<CompositionVisual>) -> Option<CompositionDrawListVisual> {
        matches!(visual.kind, CompositionVisualKind::DrawList(_)).then(|| CompositionDrawListVisual(visual.clone()))
    }

    fn data(&self) -> &DrawListData {
        match &self.0.kind {
            CompositionVisualKind::DrawList(data) => data,
            _ => unreachable!("a draw list visual handle wraps a draw list visual"),
        }
    }

    /// The visual of the visual tree this composition visual renders, if
    /// it is alive.
    pub fn visual(&self) -> Option<Ref<Visual>> {
        self.data().visual.upgrade()
    }

    pub fn draw_list(&self) -> Option<Rc<CompositionRenderData>> {
        self.data().draw_list.borrow().clone()
    }

    pub fn set_draw_list(&self, value: Option<Rc<CompositionRenderData>>) {
        let data = self.data();
        // Nothing to do
        if value.is_none() && data.draw_list.borrow().is_none() {
            return;
        }

        let old = data.draw_list.replace(value);
        if let Some(old) = old {
            old.dispose();
        }
        data.draw_list_changed.set(true);
        self.0.register_for_serialization();
    }
}
