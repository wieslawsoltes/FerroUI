// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::{LayoutContext, NonVirtualizingLayoutContext, VirtualizingLayoutContext};
use crate::ItemCollectionTransitionProvider;
use ferroui_controls::ItemsChangedEventArgs;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl,
    Ref, Size,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IndexBasedLayoutOrientation {
    #[default]
    None = 0,
    TopToBottom = 1,
    LeftToRight = 2,
}

/// Host-independent layout object. A single instance can keep independent
/// per-host state through each context's `LayoutState`.
#[repr(C)]
pub struct Layout {
    base: FerroObject,
    measure_invalidated: HandlerList<dyn Fn()>,
    arrange_invalidated: HandlerList<dyn Fn()>,
    index_based_layout_orientation: Cell<IndexBasedLayoutOrientation>,
    forced_index_orientation: Cell<Option<IndexBasedLayoutOrientation>>,
    log_item_index: Cell<i32>,
    layout_anchor_info: Cell<(i32, f64)>,
    layout_id: RefCell<String>,
}
ferro_class! {
    Layout: FerroObject, virtuals LayoutImpl: FerroObjectImpl {
        fn create_default_item_transition_provider(this) -> Option<Ref<ItemCollectionTransitionProvider>>;
    }
}
ferro_class_info!(Layout {});
ferro_impl_classes!(Layout: FerroObjectImpl);
impl LayoutImpl for Layout {
    fn create_default_item_transition_provider(
        _this: &Self,
    ) -> Option<Ref<ItemCollectionTransitionProvider>> {
        None
    }
}
impl Layout {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            measure_invalidated: HandlerList::new(),
            arrange_invalidated: HandlerList::new(),
            index_based_layout_orientation: Cell::new(IndexBasedLayoutOrientation::None),
            forced_index_orientation: Cell::new(None),
            log_item_index: Cell::new(-1),
            layout_anchor_info: Cell::new((-1, -1.0)),
            layout_id: RefCell::new(String::new()),
        }
    }
    pub fn initialize_for_context(&self, context: &Ref<LayoutContext>) {
        if let Some(layout) = self.to_ref().cast::<VirtualizingLayout>() {
            layout.initialize_for_context_core(&context.as_virtualizing_context());
        } else if let Some(layout) = self.to_ref().cast::<NonVirtualizingLayout>() {
            layout.initialize_for_context_core(&context.as_non_virtualizing_context());
        } else {
            panic!("Layout must derive from VirtualizingLayout or NonVirtualizingLayout");
        }
    }
    pub fn uninitialize_for_context(&self, context: &Ref<LayoutContext>) {
        if let Some(layout) = self.to_ref().cast::<VirtualizingLayout>() {
            layout.uninitialize_for_context_core(&context.as_virtualizing_context());
        } else if let Some(layout) = self.to_ref().cast::<NonVirtualizingLayout>() {
            layout.uninitialize_for_context_core(&context.as_non_virtualizing_context());
        } else {
            panic!("Layout must derive from VirtualizingLayout or NonVirtualizingLayout");
        }
    }
    pub fn measure(&self, context: &Ref<LayoutContext>, available_size: Size) -> Size {
        if let Some(layout) = self.to_ref().cast::<VirtualizingLayout>() {
            layout.measure_override(&context.as_virtualizing_context(), available_size)
        } else if let Some(layout) = self.to_ref().cast::<NonVirtualizingLayout>() {
            layout.measure_override(&context.as_non_virtualizing_context(), available_size)
        } else {
            panic!("Layout must derive from VirtualizingLayout or NonVirtualizingLayout");
        }
    }
    pub fn arrange(&self, context: &Ref<LayoutContext>, final_size: Size) -> Size {
        if let Some(layout) = self.to_ref().cast::<VirtualizingLayout>() {
            layout.arrange_override(&context.as_virtualizing_context(), final_size)
        } else if let Some(layout) = self.to_ref().cast::<NonVirtualizingLayout>() {
            layout.arrange_override(&context.as_non_virtualizing_context(), final_size)
        } else {
            panic!("Layout must derive from VirtualizingLayout or NonVirtualizingLayout");
        }
    }
    pub fn on_items_changed(&self, context: &Ref<LayoutContext>, args: &ItemsChangedEventArgs<'_>) {
        if let Some(layout) = self.to_ref().cast::<VirtualizingLayout>() {
            layout.on_items_changed_core(&context.as_virtualizing_context(), args);
        } else {
            self.invalidate_measure();
        }
    }
    pub fn measure_invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.measure_invalidated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.measure_invalidated.remove(token);
            }
        })
    }
    pub fn arrange_invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.arrange_invalidated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.arrange_invalidated.remove(token);
            }
        })
    }
    pub fn invalidate_measure(&self) {
        for (_, handler) in self.measure_invalidated.snapshot().iter() {
            handler();
        }
    }
    pub fn invalidate_arrange(&self) {
        for (_, handler) in self.arrange_invalidated.snapshot().iter() {
            handler();
        }
    }
    pub fn index_based_layout_orientation(&self) -> IndexBasedLayoutOrientation {
        self.forced_index_orientation
            .get()
            .unwrap_or_else(|| self.index_based_layout_orientation.get())
    }
    pub fn set_index_based_layout_orientation(&self, value: IndexBasedLayoutOrientation) {
        self.index_based_layout_orientation.set(value);
    }
    pub fn layout_id(&self) -> String {
        self.layout_id.borrow().clone()
    }
    pub fn set_layout_id(&self, value: String) {
        *self.layout_id.borrow_mut() = value;
    }
    pub(crate) fn forced_index_orientation(&self) -> Option<IndexBasedLayoutOrientation> {
        self.forced_index_orientation.get()
    }
    pub(crate) fn set_forced_index_orientation(&self, value: Option<IndexBasedLayoutOrientation>) {
        self.forced_index_orientation.set(value);
    }
    pub(crate) fn log_item_index(&self) -> i32 {
        self.log_item_index.get()
    }
    pub(crate) fn set_log_item_index(&self, index: i32) {
        self.log_item_index.set(index);
    }
    pub(crate) fn layout_anchor_info(&self) -> (i32, f64) {
        self.layout_anchor_info.get()
    }
    pub(crate) fn set_layout_anchor_info(&self, index: i32, offset: f64) {
        let old = self.layout_anchor_info.replace((index, offset));
        if old.0 != index {
            super::LayoutsTestHooks::notify_anchor_index_changed(&self.to_ref().upcast());
        }
        if old.1 != offset {
            super::LayoutsTestHooks::notify_anchor_offset_changed(&self.to_ref().upcast());
        }
    }
}

#[repr(C)]
pub struct VirtualizingLayout {
    base: Layout,
}
ferro_class! {
    VirtualizingLayout: Layout, virtuals VirtualizingLayoutImpl: LayoutImpl {
        fn initialize_for_context_core(this, context: &Ref<VirtualizingLayoutContext>);
        fn uninitialize_for_context_core(this, context: &Ref<VirtualizingLayoutContext>);
        fn measure_override(this, context: &Ref<VirtualizingLayoutContext>, available_size: Size) -> Size;
        fn arrange_override(this, context: &Ref<VirtualizingLayoutContext>, final_size: Size) -> Size;
        fn on_items_changed_core(this, context: &Ref<VirtualizingLayoutContext>, args: &ItemsChangedEventArgs<'_>);
    }
}
ferro_class_info!(VirtualizingLayout {
    new: VirtualizingLayout::new
});
ferro_impl_classes!(VirtualizingLayout: FerroObjectImpl, LayoutImpl);
impl VirtualizingLayoutImpl for VirtualizingLayout {
    fn initialize_for_context_core(_this: &Self, _context: &Ref<VirtualizingLayoutContext>) {}
    fn uninitialize_for_context_core(_this: &Self, _context: &Ref<VirtualizingLayoutContext>) {}
    fn measure_override(
        _this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        _available_size: Size,
    ) -> Size {
        panic!("VirtualizingLayout is abstract: measure_override must be overridden");
    }
    fn arrange_override(
        _this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        final_size
    }
    fn on_items_changed_core(
        this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        _args: &ItemsChangedEventArgs<'_>,
    ) {
        this.invalidate_measure();
    }
}
impl VirtualizingLayout {
    pub fn construct() -> Self {
        Self {
            base: Layout::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

#[repr(C)]
pub struct NonVirtualizingLayout {
    base: Layout,
}
ferro_class! {
    NonVirtualizingLayout: Layout, virtuals NonVirtualizingLayoutImpl: LayoutImpl {
        fn initialize_for_context_core(this, context: &Ref<NonVirtualizingLayoutContext>);
        fn uninitialize_for_context_core(this, context: &Ref<NonVirtualizingLayoutContext>);
        fn measure_override(this, context: &Ref<NonVirtualizingLayoutContext>, available_size: Size) -> Size;
        fn arrange_override(this, context: &Ref<NonVirtualizingLayoutContext>, final_size: Size) -> Size;
    }
}
ferro_class_info!(NonVirtualizingLayout {
    new: NonVirtualizingLayout::new
});
ferro_impl_classes!(NonVirtualizingLayout: FerroObjectImpl, LayoutImpl);
impl NonVirtualizingLayoutImpl for NonVirtualizingLayout {
    fn initialize_for_context_core(_this: &Self, _context: &Ref<NonVirtualizingLayoutContext>) {}
    fn uninitialize_for_context_core(_this: &Self, _context: &Ref<NonVirtualizingLayoutContext>) {}
    fn measure_override(
        _this: &Self,
        _context: &Ref<NonVirtualizingLayoutContext>,
        _available_size: Size,
    ) -> Size {
        panic!("NonVirtualizingLayout is abstract: measure_override must be overridden");
    }
    fn arrange_override(
        _this: &Self,
        _context: &Ref<NonVirtualizingLayoutContext>,
        _final_size: Size,
    ) -> Size {
        panic!("NonVirtualizingLayout is abstract: arrange_override must be overridden");
    }
}
impl NonVirtualizingLayout {
    pub fn construct() -> Self {
        Self {
            base: Layout::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
