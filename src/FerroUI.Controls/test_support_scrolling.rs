//! Classes shared by the scrolling tests of this crate.

use crate::primitives::{register_logical_scrollable, ILogicalScrollable};
use crate::{Control, ControlImpl};
use ferroui_base::input::{IScrollable, InputElementImpl, NavigationDirection};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Rect, Ref, Size,
    StyledElementImpl, Vector, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A control that scrolls logically, with settable scroll state.
#[repr(C)]
pub struct TestScrollable {
    base: Control,
    extent: Cell<Size>,
    offset: Cell<Vector>,
    viewport: Cell<Size>,
    scroll_invalidated: RefCell<Vec<(u64, Rc<dyn Fn()>)>>,
    next_token: Cell<u64>,
    can_horizontally_scroll: Cell<bool>,
    can_vertically_scroll: Cell<bool>,
    is_logical_scroll_enabled: Cell<bool>,
    available_size: Cell<Size>,
    scroll_size: Cell<Size>,
    page_scroll_size: Cell<Option<Size>>,
}

ferro_class!(TestScrollable: Control);
ferro_impl_classes!(TestScrollable: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for TestScrollable {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        register_logical_scrollable::<TestScrollable>();
    }
}

impl LayoutableImpl for TestScrollable {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.available_size.set(available_size);
        Size::new(150.0, 150.0)
    }
}

impl TestScrollable {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            extent: Cell::new(Size::default()),
            offset: Cell::new(Vector::default()),
            viewport: Cell::new(Size::default()),
            scroll_invalidated: RefCell::new(Vec::new()),
            next_token: Cell::new(0),
            can_horizontally_scroll: Cell::new(false),
            can_vertically_scroll: Cell::new(false),
            is_logical_scroll_enabled: Cell::new(true),
            available_size: Cell::new(Size::default()),
            scroll_size: Cell::new(Size::new(f64::INFINITY, 1.0)),
            page_scroll_size: Cell::new(None),
        })
    }

    pub fn with_state(extent: Size, offset: Vector, viewport: Size) -> Ref<Self> {
        let result = Self::new();
        result.set_extent(extent);
        result.set_offset(offset);
        result.set_viewport(viewport);
        result
    }

    pub fn available_size(&self) -> Size {
        self.available_size.get()
    }

    pub fn has_scroll_invalidated_subscriber(&self) -> bool {
        !self.scroll_invalidated.borrow().is_empty()
    }

    pub fn can_horizontally_scroll(&self) -> bool {
        self.can_horizontally_scroll.get()
    }

    pub fn can_vertically_scroll(&self) -> bool {
        self.can_vertically_scroll.get()
    }

    pub fn set_is_logical_scroll_enabled(&self, value: bool) {
        self.is_logical_scroll_enabled.set(value)
    }

    pub fn extent(&self) -> Size {
        self.extent.get()
    }

    pub fn set_extent(&self, value: Size) {
        self.extent.set(value);
        self.raise_scroll_invalidated();
    }

    pub fn offset(&self) -> Vector {
        self.offset.get()
    }

    pub fn set_offset(&self, value: Vector) {
        self.offset.set(value);
        self.raise_scroll_invalidated();
    }

    pub fn viewport(&self) -> Size {
        self.viewport.get()
    }

    pub fn set_viewport(&self, value: Size) {
        self.viewport.set(value);
        self.raise_scroll_invalidated();
    }

    /// Overrides the size to scroll by.
    pub fn set_scroll_size(&self, value: Size) {
        self.scroll_size.set(value)
    }

    /// Overrides the size to page by.
    pub fn set_page_scroll_size(&self, value: Size) {
        self.page_scroll_size.set(Some(value))
    }

    pub fn raise_scroll_invalidated(&self) {
        let handlers: Vec<Rc<dyn Fn()>> = self
            .scroll_invalidated
            .borrow()
            .iter()
            .map(|(_, handler)| handler.clone())
            .collect();
        for handler in handlers {
            handler();
        }
    }
}

impl IScrollable for TestScrollable {
    fn extent(&self) -> Size {
        TestScrollable::extent(self)
    }

    fn offset(&self) -> Vector {
        TestScrollable::offset(self)
    }

    fn set_offset(&self, value: Vector) {
        TestScrollable::set_offset(self, value)
    }

    fn viewport(&self) -> Size {
        TestScrollable::viewport(self)
    }

    fn can_horizontally_scroll(&self) -> bool {
        self.can_horizontally_scroll.get()
    }

    fn can_vertically_scroll(&self) -> bool {
        self.can_vertically_scroll.get()
    }
}

impl ILogicalScrollable for TestScrollable {
    fn set_can_horizontally_scroll(&self, value: bool) {
        self.can_horizontally_scroll.set(value)
    }

    fn set_can_vertically_scroll(&self, value: bool) {
        self.can_vertically_scroll.set(value)
    }

    fn is_logical_scroll_enabled(&self) -> bool {
        self.is_logical_scroll_enabled.get()
    }

    fn scroll_size(&self) -> Size {
        self.scroll_size.get()
    }

    fn page_scroll_size(&self) -> Size {
        match self.page_scroll_size.get() {
            Some(size) => size,
            None => Size::new(f64::INFINITY, self.viewport.get().height),
        }
    }

    fn scroll_invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.scroll_invalidated.borrow_mut().push((token, handler));

        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.scroll_invalidated
                    .borrow_mut()
                    .retain(|(candidate, _)| *candidate != token);
            }
        })
    }

    fn bring_into_view(&self, _target: &Ref<Control>, _target_rect: Rect) -> bool {
        panic!("not implemented")
    }

    fn get_control_in_direction(
        &self,
        _direction: NavigationDirection,
        _from: Option<&Ref<Control>>,
    ) -> Option<Ref<Control>> {
        panic!("not implemented")
    }

    fn raise_scroll_invalidated(&self) {
        TestScrollable::raise_scroll_invalidated(self)
    }
}
