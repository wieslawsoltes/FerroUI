use super::composition_draw_item::{CompositionDrawItem, DrawItemList};
use super::drawing::CompositionRenderData;
use super::generated::{
    CompositionContainerVisualProps, CompositionExperimentalAcrylicVisualHooks,
    CompositionExperimentalAcrylicVisualProps,
};
use super::server::{
    IServerVisualContent, ServerCompositionDrawListVisual, ServerCompositionExperimentalAcrylicVisual, DRAW_LIST_ITEMS,
    DRAW_LIST_NEW, DRAW_LIST_REDRAWN,
};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::visual::CompositionVisualKind;
use super::{CompositionVisual, Compositor};
use crate::media::{Geometry, IntersectionResult};
use crate::utilities::OptionCell;
use crate::{Point, Ref, Visual, WeakRef};
use std::cell::{Cell, OnceCell};
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

    /// Whether the control writes what this adds whether or not the control is in a visual tree,
    /// so that the composition visual of a control that left its tree and is back before the next
    /// update pass of the renderer is the same as one created then, and is kept (an addition,
    /// docs/porting/desktop-performance.md, section 9.32). `false` by default: the composition
    /// visual is released when its control leaves the tree, as upstream.
    fn follows_control_out_of_tree(&self) -> bool {
        false
    }

    /// Whether the visual shows the drawings of the visuals below its own that have no
    /// composition visual (the drawing items, docs/porting/desktop-performance.md, section
    /// 9.42): its server content hands out the items of the draw list visual it wraps
    /// (`IServerVisualContent::draw_items`). `false` by default: each of those visuals gets a
    /// composition visual of its own.
    fn holds_drawing_items(&self) -> bool {
        false
    }
}

/// What a draw list visual adds to a visual: the visual of the tree it
/// renders, and what that visual drew.
pub(crate) struct DrawListData {
    props: CompositionContainerVisualProps,
    /// `None` for a draw list visual of no visual of the tree, which only
    /// the benchmarks create (`composition::testing`), as upstream's
    /// benchmark passes `null` for the visual.
    visual: Option<WeakRef<Visual>>,
    draw_list_changed: Cell<bool>,
    /// The drawing is the same but what it shows changed (section 9.14): the server redraws it.
    content_invalidated: Cell<bool>,
    draw_list: OptionCell<Rc<CompositionRenderData>>,
    /// The drawings of the visuals below that have no composition visual (section 9.42): made
    /// when the first one is listed.
    items: OnceCell<Box<DrawItemList>>,
    /// The visual left its visual tree and this composition visual is kept for its return
    /// (section 9.32). Until then it is hit tested as a released one is: it
    /// has no drawing and no children.
    departed: Cell<bool>,
    /// The visual was shown without a composition visual before this one was created
    /// (section 9.41): what is under it was drawn by the composition visual above, and the
    /// readback of what is under it says so until the first frame with this one.
    shown_through: Cell<bool>,
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
        // The drawing: unchanged, a new one, or the same one drawn again (section 9.14). And
        // whether the changes of the items follow (section 9.42).
        let items = self.items.get().filter(|items| items.has_changes());
        let items_bit = if items.is_some() { DRAW_LIST_ITEMS } else { 0 };
        if self.draw_list_changed.get() {
            writer.write(DRAW_LIST_NEW | items_bit);
            writer.write_server_object(self.draw_list.get().map(|draw_list| draw_list.server()));
            self.draw_list_changed.set(false);
        } else if self.content_invalidated.get() {
            writer.write(DRAW_LIST_REDRAWN | items_bit);
        } else {
            writer.write(items_bit);
        }
        self.content_invalidated.set(false);
        if let Some(items) = items {
            items.serialize(host, writer);
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

    /// The drawing the visual is hit tested with: none while its visual is out of the tree.
    fn hit_test_draw_list(&self) -> Option<Rc<CompositionRenderData>> {
        if self.departed.get() {
            return None;
        }
        self.draw_list.get()
    }

    pub(super) fn is_departed(&self) -> bool {
        self.departed.get()
    }

    pub(super) fn is_shown_through(&self) -> bool {
        self.shown_through.get()
    }

    pub(super) fn hit_test(&self, pt: Point) -> bool {
        let custom = self.visual.as_ref().and_then(WeakRef::upgrade).and_then(|visual| visual.custom_hit_test(pt));
        let draw_list = self.hit_test_draw_list();
        if draw_list.is_none() && custom.is_none() {
            return false;
        }
        if let Some(custom) = custom {
            return custom;
        }
        draw_list.is_some_and(|draw_list| draw_list.hit_test(pt))
    }

    pub(super) fn hit_test_geometry(&self, geometry: &Ref<Geometry>) -> IntersectionResult {
        let custom =
            self.visual.as_ref().and_then(WeakRef::upgrade).and_then(|visual| visual.custom_hit_test_geometry(geometry));
        let draw_list = self.hit_test_draw_list();
        if draw_list.is_none() && custom.is_none() {
            return IntersectionResult::Empty;
        }
        if let Some(custom) = custom {
            return custom;
        }
        draw_list.map_or(IntersectionResult::Empty, |draw_list| draw_list.hit_test_geometry(geometry))
    }
}

impl CompositionVisual {
    /// The items the visual shows (section 9.42), if it ever had one.
    pub(crate) fn draw_items(&self) -> Option<&DrawItemList> {
        match &self.kind {
            CompositionVisualKind::DrawList(data) => data.items.get().map(|items| &**items),
            _ => None,
        }
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
        Self::create_core(compositor, Some(visual), false, Some(extension), server_content)
    }

    pub(super) fn create(compositor: &Rc<Compositor>, visual: &Visual, acrylic: bool) -> CompositionDrawListVisual {
        Self::create_core(compositor, Some(visual), acrylic, None, move || {
            if acrylic {
                Box::new(ServerCompositionExperimentalAcrylicVisual::new())
            } else {
                Box::new(ServerCompositionDrawListVisual::new())
            }
        })
    }

    /// `new CompositionDrawListVisual(compositor, new
    /// ServerCompositionDrawListVisual(compositor.Server, null), null)`: a
    /// draw list visual of no visual of the tree, which draws its draw list
    /// and has no custom hit test. For the benchmarks
    /// (`composition::testing`).
    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn without_visual(compositor: &Rc<Compositor>) -> CompositionDrawListVisual {
        Self::create_core(compositor, None, false, None, || Box::new(ServerCompositionDrawListVisual::new()))
    }

    fn create_core(
        compositor: &Rc<Compositor>,
        visual: Option<&Visual>,
        acrylic: bool,
        extension: Option<Rc<dyn ICompositionDrawListVisualExtension>>,
        server_content: impl FnOnce() -> Box<dyn IServerVisualContent> + Send + 'static,
    ) -> CompositionDrawListVisual {
        let data = DrawListData {
            props: CompositionContainerVisualProps::new(),
            visual: visual.map(|visual| visual.to_ref().downgrade()),
            draw_list_changed: Cell::new(false),
            content_invalidated: Cell::new(false),
            draw_list: OptionCell::new(),
            items: OnceCell::new(),
            departed: Cell::new(false),
            shown_through: Cell::new(false),
            acrylic: acrylic.then(CompositionExperimentalAcrylicVisualProps::new),
            extension,
        };
        let result = CompositionVisual::create(compositor, CompositionVisualKind::DrawList(data), server_content);
        // A visual with a custom hit test decides for itself whether it is
        // hit, whatever its bounds are.
        let has_custom_hit_test = visual.is_some_and(|visual| visual.custom_hit_test(Point::default()).is_some());
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
        self.data().visual.as_ref().and_then(WeakRef::upgrade)
    }

    /// Whether the visual can be kept while its visual is out of the tree for a moment (section
    /// 9.32): nothing but the synchronisation of the visual tree writes to a plain draw list
    /// visual, and a derived one says so itself.
    pub(crate) fn can_be_kept_out_of_tree(&self) -> bool {
        let data = self.data();
        data.acrylic.is_none()
            && data.extension.as_ref().is_none_or(|extension| extension.follows_control_out_of_tree())
    }

    /// Whether the visual shows the drawings of the visuals below its own that have no
    /// composition visual (section 9.42): a plain draw list visual does, and a derived one says
    /// so itself.
    pub(crate) fn holds_drawing_items(&self) -> bool {
        let data = self.data();
        data.acrylic.is_none() && data.extension.as_ref().is_none_or(|extension| extension.holds_drawing_items())
    }

    /// Makes `items` the drawings the visual shows for the visuals below its own that have no
    /// composition visual, in paint order, and `positions` the number of them that are painted
    /// before each of its child visuals.
    pub(crate) fn set_draw_items(&self, items: Vec<Rc<CompositionDrawItem>>, mut positions: Vec<u32>) {
        let data = self.data();
        if items.is_empty() {
            if data.items.get().is_none() {
                return;
            }
            positions.clear();
        }
        let list = data.items.get_or_init(Default::default);
        if list.set(&self.0, items, positions) {
            self.0.register_for_serialization();
        }
    }

    /// The visual shows no drawing but its own.
    pub(crate) fn clear_draw_items(&self) {
        self.set_draw_items(Vec::new(), Vec::new());
    }

    /// Marks the visual as kept for a visual that left its tree, or as in use again.
    pub(crate) fn set_departed(&self, departed: bool) {
        self.data().departed.set(departed);
    }

    /// Marks the visual as created for a visual that was shown without a composition visual
    /// until now (section 9.41): hit tests pass through it until it has a readback.
    pub(crate) fn set_shown_through(&self) {
        self.data().shown_through.set(true);
    }

    pub fn draw_list(&self) -> Option<Rc<CompositionRenderData>> {
        self.data().draw_list.get()
    }

    /// Keeps the drawing but has the server draw it again: the visual was invalidated and
    /// recorded the drawing it has, while something it draws through (a mask, an effect, a
    /// brush) may have changed (an addition, desktop-performance.md section 9.14).
    pub fn invalidate_draw_list_content(&self) {
        let data = self.data();
        if !data.draw_list.is_some() {
            return;
        }
        data.content_invalidated.set(true);
        self.0.register_for_serialization();
    }

    pub fn set_draw_list(&self, value: Option<Rc<CompositionRenderData>>) {
        let data = self.data();
        // Nothing to do
        if value.is_none() && !data.draw_list.is_some() {
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
