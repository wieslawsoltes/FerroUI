// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{IMdiLayoutManager, MdiLayoutEntry, MdiResizeDirection};
use ferrodock_model::{
    collections::ListHandle,
    controls::IMdiDocument,
    core::{DockRect, MdiLayoutDefaults as Defaults, MdiWindowState},
};
use ferroui_base::{Rect, Size, Vector};
use std::rc::Rc;
ferroui_base::ferro_thread_local! {static INSTANCE:Rc<ClassicMdiLayoutManager>=ClassicMdiLayoutManager::new();}
/// Original classic MDI layout algorithms for native document windows.
pub struct ClassicMdiLayoutManager;
impl PartialEq for ClassicMdiLayoutManager {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ClassicMdiLayoutManager {
    /// Creates a separate classic manager, as the source public default constructor does.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the native UI thread's shared original default manager.
    pub fn instance() -> Rc<Self> {
        INSTANCE.with(Clone::clone)
    }
}
impl IMdiLayoutManager for ClassicMdiLayoutManager {
    fn arrange(&self, entries: &ListHandle<MdiLayoutEntry>, final_size: Size) {
        if entries.count() == 0 {
            return;
        }
        // Original OrderBy buffers its input before evaluating each sort key once.
        let mut minimized = Vec::new();
        for entry in entries.iter() {
            if entry.document().mdi_state() == MdiWindowState::Minimized {
                minimized.push(entry);
            }
        }
        let minimized = if minimized.len() > 1 {
            let mut keyed: Vec<_> = minimized
                .into_iter()
                .map(|entry| (entry.document().mdi_z_index(), entry))
                .collect();
            keyed.sort_by_key(|(key, _)| *key);
            keyed
                .into_iter()
                .map(|(_, entry)| entry)
                .collect::<Vec<_>>()
        } else {
            // The source runtime's singleton OrderBy.ToList does not evaluate keys.
            minimized
        };
        // This is a second source enumeration, after minimized sorting/key getters.
        let mut normal = Vec::new();
        for entry in entries.iter() {
            if entry.document().mdi_state() != MdiWindowState::Minimized {
                normal.push(entry);
            }
        }
        let reserved = if !minimized.is_empty() {
            Defaults::MINIMIZED_HEIGHT + Defaults::MINIMIZED_SPACING
        } else {
            0.0
        };
        let available_height = max(0.0, final_size.height - reserved);
        let available = Size::new(final_size.width, available_height);
        arrange_minimized_windows(&minimized, final_size, available_height);
        for (index, entry) in normal.iter().enumerate() {
            let mut bounds = to_native_rect(entry.document().mdi_bounds());
            if !is_valid_bounds(bounds) {
                bounds = create_default_bounds(available, index);
                entry.document().set_mdi_bounds(to_dock_rect(bounds));
            }
            bounds = clamp_bounds(bounds, available);
            if entry.document().mdi_state() == MdiWindowState::Maximized {
                bounds = Rect::new(0.0, 0.0, available.width, available.height);
            }
            entry.control().arrange(bounds);
        }
    }
    fn get_drag_bounds(
        &self,
        _document: Rc<dyn IMdiDocument>,
        start: Rect,
        delta: Vector,
        final_size: Size,
        entries: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        clamp_bounds(
            Rect::new(
                start.x + delta.x,
                start.y + delta.y,
                start.width,
                start.height,
            ),
            get_available_size(entries, final_size),
        )
    }
    fn get_resize_bounds(
        &self,
        document: Rc<dyn IMdiDocument>,
        start: Rect,
        delta: Vector,
        direction: MdiResizeDirection,
        final_size: Size,
        entries: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        let (mut x, mut y, mut width, mut height) = (start.x, start.y, start.width, start.height);
        if direction.contains(MdiResizeDirection::LEFT) {
            x += delta.x;
            width -= delta.x;
        }
        if direction.contains(MdiResizeDirection::RIGHT) {
            width += delta.x;
        }
        if direction.contains(MdiResizeDirection::TOP) {
            y += delta.y;
            height -= delta.y;
        }
        if direction.contains(MdiResizeDirection::BOTTOM) {
            height += delta.y;
        }
        let bounds =
            apply_size_constraints(Rect::new(x, y, width, height), document.as_ref(), direction);
        clamp_bounds(bounds, get_available_size(entries, final_size))
    }
    fn update_z_order(
        &self,
        documents: &ListHandle<Rc<dyn IMdiDocument>>,
        active: Option<Rc<dyn IMdiDocument>>,
    ) {
        if documents.count() == 0 {
            return;
        }
        let mut ordered = documents.to_vec();
        if let Some(active) = active {
            if let Some(index) = ordered
                .iter()
                .position(|document| Rc::ptr_eq(document, &active))
            {
                ordered.remove(index);
                ordered.push(active);
            }
        }
        for (index, document) in ordered.iter().enumerate() {
            document.set_mdi_z_index(index as i32);
        }
    }
}
fn to_native_rect(bounds: DockRect) -> Rect {
    Rect::new(bounds.x, bounds.y, bounds.width, bounds.height)
}
fn to_dock_rect(bounds: Rect) -> DockRect {
    DockRect::new(bounds.x, bounds.y, bounds.width, bounds.height)
}
fn arrange_minimized_windows(entries: &[MdiLayoutEntry], final_size: Size, available_height: f64) {
    if entries.is_empty() {
        return;
    }
    let spacing = Defaults::MINIMIZED_SPACING;
    let total = spacing * (entries.len() - 1) as f64;
    let width = max(0.0, (final_size.width - total) / entries.len() as f64);
    let target = min(width, Defaults::MINIMIZED_WIDTH);
    if target.is_nan() || target <= 0.0 {
        return;
    }
    let mut x = 0.0;
    let y = max(0.0, available_height);
    for entry in entries {
        entry
            .control()
            .arrange(Rect::new(x, y, target, Defaults::MINIMIZED_HEIGHT));
        x += target + spacing;
    }
}
fn get_available_size(entries: &ListHandle<MdiLayoutEntry>, final_size: Size) -> Size {
    if entries.count() == 0 {
        return final_size;
    }
    let mut minimized_count = 0;
    for entry in entries.iter() {
        if entry.document().mdi_state() == MdiWindowState::Minimized {
            minimized_count += 1;
        }
    }
    let reserved = if minimized_count > 0 {
        Defaults::MINIMIZED_HEIGHT + Defaults::MINIMIZED_SPACING
    } else {
        0.0
    };
    Size::new(final_size.width, max(0.0, final_size.height - reserved))
}
fn create_default_bounds(available: Size, index: usize) -> Rect {
    let width = min(
        max(
            Defaults::MINIMUM_WIDTH,
            available.width * Defaults::DEFAULT_WIDTH_RATIO,
        ),
        available.width,
    );
    let height = min(
        max(
            Defaults::MINIMUM_HEIGHT,
            available.height * Defaults::DEFAULT_HEIGHT_RATIO,
        ),
        available.height,
    );
    let offset = Defaults::CASCADE_OFFSET * index as f64;
    Rect::new(
        min(offset, max(0.0, available.width - width)),
        min(offset, max(0.0, available.height - height)),
        width,
        height,
    )
}
fn clamp_bounds(bounds: Rect, available: Size) -> Rect {
    let width = min(bounds.width, available.width);
    let height = min(bounds.height, available.height);
    if width < 0.0 || height < 0.0 {
        return Rect::default();
    }
    let (mut x, mut y) = (bounds.x, bounds.y);
    if x < 0.0 {
        x = 0.0;
    }
    if y < 0.0 {
        y = 0.0;
    }
    if x + width > available.width {
        x = max(0.0, available.width - width);
    }
    if y + height > available.height {
        y = max(0.0, available.height - height);
    }
    Rect::new(x, y, width, height)
}
fn apply_size_constraints(
    bounds: Rect,
    dockable: &dyn IMdiDocument,
    direction: MdiResizeDirection,
) -> Rect {
    // The source conditional reads each non-NaN model limit again on its selected branch.
    let min_width = if dockable.min_width().is_nan() {
        Defaults::MINIMUM_WIDTH
    } else {
        dockable.min_width()
    };
    let min_height = if dockable.min_height().is_nan() {
        Defaults::MINIMUM_HEIGHT
    } else {
        dockable.min_height()
    };
    let max_width = if dockable.max_width().is_nan() {
        f64::INFINITY
    } else {
        dockable.max_width()
    };
    let max_height = if dockable.max_height().is_nan() {
        f64::INFINITY
    } else {
        dockable.max_height()
    };
    let (mut x, mut y, mut width, mut height) = (bounds.x, bounds.y, bounds.width, bounds.height);
    if width < min_width {
        if direction.contains(MdiResizeDirection::LEFT) {
            x = bounds.right() - min_width;
        }
        width = min_width;
    } else if width > max_width {
        if direction.contains(MdiResizeDirection::LEFT) {
            x = bounds.right() - max_width;
        }
        width = max_width;
    }
    if height < min_height {
        if direction.contains(MdiResizeDirection::TOP) {
            y = bounds.bottom() - min_height;
        }
        height = min_height;
    } else if height > max_height {
        if direction.contains(MdiResizeDirection::TOP) {
            y = bounds.bottom() - max_height;
        }
        height = max_height;
    }
    Rect::new(x, y, width, height)
}
fn is_valid_bounds(bounds: Rect) -> bool {
    !bounds.width.is_nan() && !bounds.height.is_nan() && bounds.width > 0.0 && bounds.height > 0.0
}
fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a > b || (a == b && a.is_sign_positive()) {
        a
    } else {
        b
    }
}
ferroui_base::ferro_markup_type!(class ClassicMdiLayoutManager {
    this: Rc<ClassicMdiLayoutManager>,
    handles: [Rc<ClassicMdiLayoutManager>, Option<Rc<ClassicMdiLayoutManager>>],
    interfaces: [Rc<dyn IMdiLayoutManager>],
    constructors: [() => ClassicMdiLayoutManager::new],
    fields: [Instance: Rc<ClassicMdiLayoutManager> => ClassicMdiLayoutManager::instance],
    methods: [
        fn Arrange(ListHandle<MdiLayoutEntry>, Size) => (|this: &Rc<ClassicMdiLayoutManager>, entries, size| this.arrange(&entries, size)),
        fn GetDragBounds(Rc<dyn IMdiDocument>, Rect, Vector, Size, ListHandle<MdiLayoutEntry>) -> Rect => (|this: &Rc<ClassicMdiLayoutManager>, document, bounds, delta, size, entries| this.get_drag_bounds(document, bounds, delta, size, &entries)),
        fn GetResizeBounds(Rc<dyn IMdiDocument>, Rect, Vector, MdiResizeDirection, Size, ListHandle<MdiLayoutEntry>) -> Rect => (|this: &Rc<ClassicMdiLayoutManager>, document, bounds, delta, direction, size, entries| this.get_resize_bounds(document, bounds, delta, direction, size, &entries)),
        fn UpdateZOrder(ListHandle<Rc<dyn IMdiDocument>>, Option<Rc<dyn IMdiDocument>>) => (|this: &Rc<ClassicMdiLayoutManager>, documents, active| this.update_z_order(&documents, active)),
    ],
});
