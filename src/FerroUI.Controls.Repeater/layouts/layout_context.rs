// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use ferroui_controls::Control;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, Point, Rect, Ref, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

bitflags::bitflags! {
    /// Controls creation and automatic recycling during a layout pass.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct ElementRealizationOptions: u32 {
        const NONE = 0;
        const FORCE_CREATE = 1;
        const SUPPRESS_AUTO_RECYCLE = 2;
    }
}

/// The Rust host bridge for the overridable WinUI layout-context contract.
///
/// Implementations must release their interior borrows before creating,
/// measuring, recycling or invoking other application callbacks.
pub trait LayoutContextHost {
    fn item_count(&self) -> i32;
    fn get_item_at(&self, index: i32) -> Option<BoxedValue>;
    fn get_or_create_element_at(
        &self,
        index: i32,
        options: ElementRealizationOptions,
    ) -> Ref<Control>;
    fn recycle_element(&self, element: &Ref<Control>);
    fn layout_state(&self) -> Option<BoxedValue>;
    fn set_layout_state(&self, value: Option<BoxedValue>);
    fn realization_rect(&self) -> Rect;
    fn visible_rect(&self) -> Rect {
        self.realization_rect()
    }
    fn recommended_anchor_index(&self) -> i32 {
        -1
    }
    fn layout_origin(&self) -> Point;
    fn set_layout_origin(&self, value: Point);
    fn children(&self) -> Vec<Ref<Control>> {
        (0..self.item_count())
            .map(|index| self.get_or_create_element_at(index, ElementRealizationOptions::NONE))
            .collect()
    }
    fn is_virtualizing(&self) -> bool {
        true
    }
}

/// Base for contexts supplied by controls to a shareable layout object.
#[repr(C)]
pub struct LayoutContext {
    base: FerroObject,
    host: RefCell<Option<Rc<dyn LayoutContextHost>>>,
    virtual_adapter: RefCell<Option<Ref<VirtualizingLayoutContext>>>,
    nonvirtual_adapter: RefCell<Option<Ref<NonVirtualizingLayoutContext>>>,
}

ferro_class! {
    LayoutContext: FerroObject, virtuals LayoutContextImpl: FerroObjectImpl {
        fn layout_state_core(this) -> Option<BoxedValue>;
        fn set_layout_state_core(this, value: Option<BoxedValue>);
    }
}
ferro_class_info!(LayoutContext {});
ferro_impl_classes!(LayoutContext: FerroObjectImpl);

impl LayoutContextImpl for LayoutContext {
    fn layout_state_core(this: &Self) -> Option<BoxedValue> {
        this.host().layout_state()
    }
    fn set_layout_state_core(this: &Self, value: Option<BoxedValue>) {
        this.host().set_layout_state(value);
    }
}

impl LayoutContext {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            host: RefCell::new(None),
            virtual_adapter: RefCell::new(None),
            nonvirtual_adapter: RefCell::new(None),
        }
    }
    fn host(&self) -> Rc<dyn LayoutContextHost> {
        self.host
            .borrow()
            .clone()
            .expect("LayoutContext core member requires a host or a deriving override")
    }
    pub fn layout_state(&self) -> Option<BoxedValue> {
        self.layout_state_core()
    }
    pub fn set_layout_state(&self, value: Option<BoxedValue>) {
        self.set_layout_state_core(value);
    }
    pub fn as_virtualizing_context(&self) -> Ref<VirtualizingLayoutContext> {
        if let Some(context) = self.to_ref().cast::<VirtualizingLayoutContext>() {
            return context;
        }
        if let Some(context) = self.virtual_adapter.borrow().clone() {
            return context;
        }
        let context = self
            .to_ref()
            .cast::<NonVirtualizingLayoutContext>()
            .expect("LayoutContext must be virtualizing or nonvirtualizing");
        let adapter = VirtualizingLayoutContext::with_host(Rc::new(LayoutContextAdapter {
            context: context.downgrade(),
        }));
        *self.virtual_adapter.borrow_mut() = Some(adapter.clone());
        adapter
    }
    pub fn as_non_virtualizing_context(&self) -> Ref<NonVirtualizingLayoutContext> {
        if let Some(context) = self.to_ref().cast::<NonVirtualizingLayoutContext>() {
            return context;
        }
        if let Some(context) = self.nonvirtual_adapter.borrow().clone() {
            return context;
        }
        let context = self
            .to_ref()
            .cast::<VirtualizingLayoutContext>()
            .expect("LayoutContext must be virtualizing or nonvirtualizing");
        let adapter =
            NonVirtualizingLayoutContext::with_host(Rc::new(VirtualLayoutContextAdapter {
                context: context.downgrade(),
            }));
        *self.nonvirtual_adapter.borrow_mut() = Some(adapter.clone());
        adapter
    }
}

#[repr(C)]
pub struct VirtualizingLayoutContext {
    base: LayoutContext,
}

ferro_class! {
    VirtualizingLayoutContext: LayoutContext, virtuals VirtualizingLayoutContextImpl: LayoutContextImpl {
        fn item_count_core(this) -> i32;
        fn get_item_at_core(this, index: i32) -> Option<BoxedValue>;
        fn get_or_create_element_at_core(this, index: i32, options: ElementRealizationOptions) -> Ref<Control>;
        fn recycle_element_core(this, element: &Ref<Control>);
        fn visible_rect_core(this) -> Rect;
        fn realization_rect_core(this) -> Rect;
        fn recommended_anchor_index_core(this) -> i32;
        fn layout_origin_core(this) -> Point;
        fn set_layout_origin_core(this, value: Point);
    }
}
ferro_class_info!(VirtualizingLayoutContext {
    new: VirtualizingLayoutContext::new
});
ferro_impl_classes!(VirtualizingLayoutContext: FerroObjectImpl, LayoutContextImpl);

impl VirtualizingLayoutContextImpl for VirtualizingLayoutContext {
    fn item_count_core(this: &Self) -> i32 {
        this.host().item_count()
    }
    fn get_item_at_core(this: &Self, index: i32) -> Option<BoxedValue> {
        this.host().get_item_at(index)
    }
    fn get_or_create_element_at_core(
        this: &Self,
        index: i32,
        options: ElementRealizationOptions,
    ) -> Ref<Control> {
        this.host().get_or_create_element_at(index, options)
    }
    fn recycle_element_core(this: &Self, element: &Ref<Control>) {
        this.host().recycle_element(element);
    }
    fn visible_rect_core(this: &Self) -> Rect {
        this.host().visible_rect()
    }
    fn realization_rect_core(this: &Self) -> Rect {
        this.host().realization_rect()
    }
    fn recommended_anchor_index_core(this: &Self) -> i32 {
        this.host().recommended_anchor_index()
    }
    fn layout_origin_core(this: &Self) -> Point {
        this.host().layout_origin()
    }
    fn set_layout_origin_core(this: &Self, value: Point) {
        this.host().set_layout_origin(value);
    }
}

impl VirtualizingLayoutContext {
    pub fn construct() -> Self {
        Self {
            base: LayoutContext::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn with_host(host: Rc<dyn LayoutContextHost>) -> Ref<Self> {
        let this = Self::new();
        *this.host.borrow_mut() = Some(host);
        this
    }
    pub fn item_count(&self) -> i32 {
        self.item_count_core()
    }
    pub fn get_item_at(&self, index: i32) -> Option<BoxedValue> {
        self.get_item_at_core(index)
    }
    pub fn get_or_create_element_at(&self, index: i32) -> Ref<Control> {
        self.get_or_create_element_at_core(index, ElementRealizationOptions::NONE)
    }
    pub fn get_or_create_element_at_with_options(
        &self,
        index: i32,
        options: ElementRealizationOptions,
    ) -> Ref<Control> {
        self.get_or_create_element_at_core(index, options)
    }
    pub fn recycle_element(&self, element: &Ref<Control>) {
        self.recycle_element_core(element);
    }
    pub fn visible_rect(&self) -> Rect {
        self.visible_rect_core()
    }
    pub fn realization_rect(&self) -> Rect {
        self.realization_rect_core()
    }
    pub fn recommended_anchor_index(&self) -> i32 {
        self.recommended_anchor_index_core()
    }
    pub fn layout_origin(&self) -> Point {
        self.layout_origin_core()
    }
    pub fn set_layout_origin(&self, value: Point) {
        self.set_layout_origin_core(value);
    }
    pub fn is_virtualizing(&self) -> bool {
        // Infinite realization is the WinUI adapter's discriminator; a custom
        // context does not need to implement an extra virtual member.
        let rect = self.realization_rect();
        rect.width.is_finite() && rect.height.is_finite()
    }
}

#[repr(C)]
pub struct NonVirtualizingLayoutContext {
    base: LayoutContext,
}
ferro_class! {
    NonVirtualizingLayoutContext: LayoutContext, virtuals NonVirtualizingLayoutContextImpl: LayoutContextImpl {
        fn children_core(this) -> Vec<Ref<Control>>;
    }
}
ferro_class_info!(NonVirtualizingLayoutContext {
    new: NonVirtualizingLayoutContext::new
});
ferro_impl_classes!(NonVirtualizingLayoutContext: FerroObjectImpl, LayoutContextImpl);
impl NonVirtualizingLayoutContextImpl for NonVirtualizingLayoutContext {
    fn children_core(this: &Self) -> Vec<Ref<Control>> {
        this.host().children()
    }
}
impl NonVirtualizingLayoutContext {
    pub fn construct() -> Self {
        Self {
            base: LayoutContext::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn with_host(host: Rc<dyn LayoutContextHost>) -> Ref<Self> {
        let this = Self::new();
        *this.host.borrow_mut() = Some(host);
        this
    }
    pub fn children(&self) -> Vec<Ref<Control>> {
        self.children_core()
    }
}

/// Adapts fixed children to a virtual layout without granting recycling.
struct LayoutContextAdapter {
    context: WeakRef<NonVirtualizingLayoutContext>,
}
impl LayoutContextAdapter {
    fn context(&self) -> Ref<NonVirtualizingLayoutContext> {
        self.context
            .upgrade()
            .expect("The layout context must outlive its adapter")
    }
}
impl LayoutContextHost for LayoutContextAdapter {
    fn item_count(&self) -> i32 {
        self.context().children().len() as i32
    }
    fn get_item_at(&self, index: i32) -> Option<BoxedValue> {
        self.context()
            .children()
            .get(index as usize)
            .map(|child| Control::boxed(child.clone()))
    }
    fn get_or_create_element_at(
        &self,
        index: i32,
        _options: ElementRealizationOptions,
    ) -> Ref<Control> {
        self.context().children()[index as usize].clone()
    }
    fn recycle_element(&self, _element: &Ref<Control>) { /* Fixed children remain owned by the nonvirtualizing host. */
    }
    fn layout_state(&self) -> Option<BoxedValue> {
        self.context().layout_state()
    }
    fn set_layout_state(&self, value: Option<BoxedValue>) {
        self.context().set_layout_state(value);
    }
    fn realization_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY)
    }
    fn layout_origin(&self) -> Point {
        Point::default()
    }
    fn set_layout_origin(&self, value: Point) {
        assert_eq!(
            value,
            Point::default(),
            "A nonvirtualizing context does not support a nonzero layout origin"
        );
    }
    fn is_virtualizing(&self) -> bool {
        false
    }
}

/// Adapts a virtual host to a layout which enumerates every child.
struct VirtualLayoutContextAdapter {
    context: WeakRef<VirtualizingLayoutContext>,
}
impl VirtualLayoutContextAdapter {
    fn context(&self) -> Ref<VirtualizingLayoutContext> {
        self.context
            .upgrade()
            .expect("The layout context must outlive its adapter")
    }
}
impl LayoutContextHost for VirtualLayoutContextAdapter {
    fn item_count(&self) -> i32 {
        self.context().item_count()
    }
    fn get_item_at(&self, index: i32) -> Option<BoxedValue> {
        self.context().get_item_at(index)
    }
    fn get_or_create_element_at(
        &self,
        index: i32,
        options: ElementRealizationOptions,
    ) -> Ref<Control> {
        self.context()
            .get_or_create_element_at_with_options(index, options)
    }
    fn recycle_element(&self, element: &Ref<Control>) {
        self.context().recycle_element(element);
    }
    fn layout_state(&self) -> Option<BoxedValue> {
        self.context().layout_state()
    }
    fn set_layout_state(&self, value: Option<BoxedValue>) {
        self.context().set_layout_state(value);
    }
    fn realization_rect(&self) -> Rect {
        self.context().realization_rect()
    }
    fn layout_origin(&self) -> Point {
        self.context().layout_origin()
    }
    fn set_layout_origin(&self, value: Point) {
        self.context().set_layout_origin(value);
    }
}
