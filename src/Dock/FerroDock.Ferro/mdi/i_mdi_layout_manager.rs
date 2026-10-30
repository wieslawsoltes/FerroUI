// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{MdiLayoutEntry, MdiResizeDirection};
use ferrodock_model::{collections::ListHandle, controls::IMdiDocument};
use ferroui_base::{Rect, Size, Vector};
use std::rc::Rc;
/// Original manager contract for arranging, dragging and resizing MDI windows.
/// Input lists are read through the existing shared source list handle; implementations
/// preserve their read-only use, live identity and custom enumeration behavior.
pub trait IMdiLayoutManager: 'static {
    /// Native projection of the original runtime type name, without reflection.
    /// Registered native descriptors retain the source-visible name. Unregistered
    /// implementations use their compiler-supplied concrete Rust name.
    #[doc(hidden)]
    fn runtime_type_name(&self) -> &'static str {
        ferroui_base::metadata::MarkupType::find_by_handle(std::any::TypeId::of::<Rc<Self>>())
            .map(|m| m.name)
            .unwrap_or_else(|| {
                std::any::type_name::<Self>()
                    .rsplit("::")
                    .next()
                    .expect("compiler type name")
            })
    }

    /// Arranges the original native control/document entries.
    fn arrange(&self, entries: &ListHandle<MdiLayoutEntry>, final_size: Size);
    /// Calculates the drag bounds using the current available window area.
    fn get_drag_bounds(
        &self,
        document: Rc<dyn IMdiDocument>,
        start_bounds: Rect,
        delta: Vector,
        final_size: Size,
        entries: &ListHandle<MdiLayoutEntry>,
    ) -> Rect;
    /// Calculates resized bounds after applying source edge and model constraints.
    fn get_resize_bounds(
        &self,
        document: Rc<dyn IMdiDocument>,
        start_bounds: Rect,
        delta: Vector,
        direction: MdiResizeDirection,
        final_size: Size,
        entries: &ListHandle<MdiLayoutEntry>,
    ) -> Rect;
    /// Reorders document Z indices, moving only an active member to the end.
    fn update_z_order(
        &self,
        documents: &ListHandle<Rc<dyn IMdiDocument>>,
        active_document: Option<Rc<dyn IMdiDocument>>,
    );
}
impl PartialEq for dyn IMdiLayoutManager {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(interface dyn IMdiLayoutManager as "IMdiLayoutManager" {
    this: Rc<dyn IMdiLayoutManager>,
    handles: [Rc<dyn IMdiLayoutManager>, Option<Rc<dyn IMdiLayoutManager>>],
    methods: [
        fn Arrange(ListHandle<MdiLayoutEntry>, Size) => (|this: &Rc<dyn IMdiLayoutManager>, entries, size| this.arrange(&entries, size)),
        fn GetDragBounds(Rc<dyn IMdiDocument>, Rect, Vector, Size, ListHandle<MdiLayoutEntry>) -> Rect => (|this: &Rc<dyn IMdiLayoutManager>, document, bounds, delta, size, entries| this.get_drag_bounds(document, bounds, delta, size, &entries)),
        fn GetResizeBounds(Rc<dyn IMdiDocument>, Rect, Vector, MdiResizeDirection, Size, ListHandle<MdiLayoutEntry>) -> Rect => (|this: &Rc<dyn IMdiLayoutManager>, document, bounds, delta, direction, size, entries| this.get_resize_bounds(document, bounds, delta, direction, size, &entries)),
        fn UpdateZOrder(ListHandle<Rc<dyn IMdiDocument>>, Option<Rc<dyn IMdiDocument>>) => (|this: &Rc<dyn IMdiLayoutManager>, documents, active| this.update_z_order(&documents, active)),
    ],
});
