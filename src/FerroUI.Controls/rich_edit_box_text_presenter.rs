//! The persistent document view hosted by a rich editor's `ContentElement`.

use crate::automation::{AccessibilityView, AutomationProperties};
use crate::presenters::ScrollContentPresenter;
use crate::primitives::{register_logical_scrollable, ILogicalScrollable};
use crate::rich_text_block::RichTextSelectionChangedEventArgs;
use crate::{
    Control, ControlImpl, RichEditBox, RichTextBlock, RichTextBlockImpl, RichTextBlockImplExt,
};
use ferroui_base::input::{
    IScrollable, InputElement, InputElementImpl, KeyModifiers, NavigationDirection,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{IBrush, TextWrapping};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl, Vector, VisualImpl,
    WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Renders and scrolls the owner's single rich document using FerroUI text layout.
///
/// The presenter survives template replacement. It formats the entire document at
/// the viewport width, and translates every text geometry operation by the same
/// pixel scroll offset. It has no separate editing buffer or undo history.
#[repr(C)]
pub struct RichEditBoxTextPresenter {
    base: RichTextBlock,
    owner: RefCell<Option<WeakRef<RichEditBox>>>,
    layout_width: Cell<f64>,
    has_measured_layout: Cell<bool>,
    natural_size: Cell<Size>,
    extent: Cell<Size>,
    viewport: Cell<Size>,
    offset: Cell<Vector>,
    can_horizontally_scroll: Cell<bool>,
    can_vertically_scroll: Cell<bool>,
    scroll_invalidated: Rc<HandlerList<dyn Fn()>>,
}

ferro_class!(RichEditBoxTextPresenter: RichTextBlock);
ferroui_base::ferro_class_info!(RichEditBoxTextPresenter {
    new: RichEditBoxTextPresenter::new,
});
ferro_impl_classes!(RichEditBoxTextPresenter: StyledElementImpl, InteractiveImpl, VisualImpl);

impl FerroObjectImpl for RichEditBoxTextPresenter {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_paragraph_margin_collapsing(false);
        AutomationProperties::set_accessibility_view(this, AccessibilityView::Raw);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if let Some(owner) = this.owner() {
            owner.on_text_view_property_changed(change);
        }
    }
}

impl ControlImpl for RichEditBoxTextPresenter {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::RichEditBoxTextPresenterAutomationPeer::new(this).upcast()
    }
}

impl InputElementImpl for RichEditBoxTextPresenter {
    fn as_scrollable(this: &Self) -> Option<&dyn IScrollable> {
        Some(this)
    }
}

impl RichTextBlockImpl for RichEditBoxTextPresenter {
    fn default_incremental_tab(this: &Self, font_size: f64) -> f64 {
        this.owner().map_or_else(
            || Self::parent_default_incremental_tab(this, font_size),
            |owner| {
                owner
                    .editor()
                    .default_paragraph_format()
                    .default_tab_stop
                    .unwrap_or(48.0)
            },
        )
    }

    fn text_layout_constraint(this: &Self, _available_size: Size) -> Size {
        // A viewport is not a page boundary. RichEditBox owns all paragraphs,
        // including those below the currently visible portion of the document.
        Size::new(this.layout_width.get(), f64::INFINITY)
    }

    fn text_origin(this: &Self) -> Point {
        let offset = this.offset.get();
        Point::new(-offset.x, -offset.y)
    }

    fn request_selection_change(this: &Self, start: usize, end: usize) -> bool {
        this.owner()
            .is_some_and(|owner| owner.request_presenter_selection_change(start, end))
    }

    fn on_selection_changing(this: &Self, args: &RichTextSelectionChangedEventArgs) -> bool {
        this.owner()
            .is_none_or(|owner| owner.presenter_selection_changing(args))
    }

    fn rebase_selection_on_document_change(this: &Self) -> bool {
        this.owner()
            .is_none_or(|owner| owner.rebase_presenter_selection_on_document_change())
    }

    fn raise_selection_changed_event(this: &Self) {
        if let Some(owner) = this.owner() {
            owner.raise_presenter_selection_changed();
        } else {
            Self::parent_raise_selection_changed_event(this);
        }
    }

    fn text_focus_target(this: &Self) -> Ref<InputElement> {
        this.owner()
            .map_or_else(|| this.to_ref().upcast(), Ref::upcast)
    }

    fn allow_hyperlink_navigation(this: &Self, modifiers: KeyModifiers) -> bool {
        this.owner().is_none_or(|owner| {
            owner.is_read_only() || modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META)
        })
    }

    fn caret_visible(this: &Self) -> bool {
        this.owner().is_some_and(|owner| owner.caret_visible())
    }

    fn selection_is_visible(this: &Self) -> bool {
        this.is_text_selection_enabled()
            && this
                .owner()
                .is_none_or(|owner| owner.selection_is_visible())
    }

    fn selection_render_brush(this: &Self) -> Option<Rc<dyn IBrush>> {
        this.owner().map_or_else(
            || this.selection_highlight_color(),
            |owner| owner.active_selection_brush(),
        )
    }
}

impl LayoutableImpl for RichEditBoxTextPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.layout_width
            .set(if this.text_wrapping() == TextWrapping::NoWrap {
                f64::INFINITY
            } else {
                available_size.width
            });
        let natural = Self::parent_measure_override(this, available_size);
        this.natural_size.set(natural);
        this.has_measured_layout.set(true);
        this.notify_geometry_changed();
        natural.constrain(available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.layout_width
            .set(if this.text_wrapping() == TextWrapping::NoWrap {
                this.natural_size.get().width.max(final_size.width)
            } else {
                final_size.width
            });
        let result = Self::parent_arrange_override(this, final_size);
        let natural = this.page_layout().size;
        let extent = Size::new(
            if this.text_wrapping() == TextWrapping::NoWrap {
                natural.width.max(final_size.width)
            } else {
                final_size.width
            },
            natural.height.max(final_size.height),
        );
        let old_extent = this.extent.replace(extent);
        let old_viewport = this.viewport.replace(final_size);
        let offset = this.normalize_offset(this.offset.get());
        let old_offset = this.offset.replace(offset);
        if old_offset != offset {
            // Embedded controls use the same origin as glyphs. Arrange them at
            // the clamped offset before exposing the new geometry to observers.
            Self::parent_arrange_override(this, final_size);
            this.invalidate_visual();
        }
        if old_extent != extent || old_viewport != final_size || old_offset != offset {
            this.raise_scroll_invalidated();
        }
        this.notify_geometry_changed();
        result
    }
}

impl IScrollable for RichEditBoxTextPresenter {
    fn extent(&self) -> Size {
        self.extent.get()
    }
    fn offset(&self) -> Vector {
        self.offset.get()
    }
    fn set_offset(&self, value: Vector) {
        let value = self.normalize_offset(value);
        if self.offset.replace(value) != value {
            self.invalidate_arrange();
            self.invalidate_visual();
            self.raise_scroll_invalidated();
            self.notify_geometry_changed();
        }
    }
    fn viewport(&self) -> Size {
        self.viewport.get()
    }
    fn can_horizontally_scroll(&self) -> bool {
        self.can_horizontally_scroll.get()
    }
    fn can_vertically_scroll(&self) -> bool {
        self.can_vertically_scroll.get()
    }
}

impl ILogicalScrollable for RichEditBoxTextPresenter {
    fn set_can_horizontally_scroll(&self, value: bool) {
        if self.can_horizontally_scroll.replace(value) != value {
            self.set_offset(self.offset());
            self.raise_scroll_invalidated();
        }
    }
    fn set_can_vertically_scroll(&self, value: bool) {
        if self.can_vertically_scroll.replace(value) != value {
            self.set_offset(self.offset());
            self.raise_scroll_invalidated();
        }
    }
    fn is_logical_scroll_enabled(&self) -> bool {
        true
    }
    fn scroll_size(&self) -> Size {
        // CTextBoxView::Scroll uses ScrollViewerLineDelta in device-independent pixels.
        Size::new(16.0, 16.0)
    }
    fn page_scroll_size(&self) -> Size {
        self.viewport()
    }
    fn scroll_invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.scroll_invalidated.add(handler);
        let handlers = Rc::downgrade(&self.scroll_invalidated);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }
    fn bring_into_view(&self, target: &Ref<Control>, target_rect: Rect) -> bool {
        let Some(transform) = target.transform_to_visual(self) else {
            return false;
        };
        let current = self.offset();
        let rectangle = target_rect.transform_to_aabb(transform).translate(current);
        let viewport = self.viewport();
        let x = ScrollContentPresenter::compute_scroll_offset_with_minimal_scroll(
            current.x,
            current.x + viewport.width,
            rectangle.x,
            rectangle.right(),
        );
        let y = ScrollContentPresenter::compute_scroll_offset_with_minimal_scroll(
            current.y,
            current.y + viewport.height,
            rectangle.y,
            rectangle.bottom(),
        );
        self.set_offset(Vector::new(x, y));
        current != self.offset()
    }
    fn get_control_in_direction(
        &self,
        _direction: NavigationDirection,
        _from: Option<&Ref<Control>>,
    ) -> Option<Ref<Control>> {
        // All embedded controls are realized. Ordinary focus navigation walks
        // their visual tree; no virtualized-container realization is needed.
        None
    }
    fn raise_scroll_invalidated(&self) {
        for (_, handler) in self.scroll_invalidated.snapshot().iter() {
            handler();
        }
    }
}

impl RichEditBoxTextPresenter {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<Self>(false);
        InputElement::is_tab_stop_property().override_default_value::<Self>(false);
        register_logical_scrollable::<Self>();
    }
    pub fn construct() -> Self {
        Self {
            base: RichTextBlock::construct(),
            owner: RefCell::new(None),
            layout_width: Cell::new(f64::INFINITY),
            has_measured_layout: Cell::new(false),
            natural_size: Cell::new(Size::default()),
            extent: Cell::new(Size::default()),
            viewport: Cell::new(Size::default()),
            offset: Cell::new(Vector::default()),
            can_horizontally_scroll: Cell::new(true),
            can_vertically_scroll: Cell::new(true),
            scroll_invalidated: Rc::new(HandlerList::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn set_owner(&self, owner: &Ref<RichEditBox>) {
        *self.owner.borrow_mut() = Some(owner.downgrade());
    }
    pub fn owner(&self) -> Option<Ref<RichEditBox>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub(crate) fn has_measured_layout(&self) -> bool {
        self.has_measured_layout.get()
    }
    pub(crate) fn bring_caret_into_view(&self) {
        // The document view owns its offset even when ContentElement is a
        // decorator or panel. Reveal the caret here before asking outer
        // scrollers to reveal its updated rectangle in the surrounding UI.
        ILogicalScrollable::bring_into_view(self, &self.to_ref().upcast(), self.cursor_rectangle());
        self.bring_into_view_rect(self.cursor_rectangle());
    }
    /// Aligns a text range edge for TOM/UI automation without changing selection.
    /// The rectangle uses presenter coordinates, as all public text geometry does.
    pub(crate) fn bring_text_rect_into_view_aligned(&self, rect: Rect, align_to_top: bool) -> bool {
        let current = self.offset();
        let rect = rect.translate(current);
        let viewport = self.viewport();
        let x = ScrollContentPresenter::compute_scroll_offset_with_minimal_scroll(
            current.x,
            current.x + viewport.width,
            rect.x,
            rect.right(),
        );
        let y = if align_to_top {
            rect.y
        } else {
            rect.bottom() - viewport.height
        };
        self.set_offset(Vector::new(x, y));
        self.offset() != current
    }
    fn normalize_offset(&self, value: Vector) -> Vector {
        fn axis(value: f64, extent: f64, viewport: f64, enabled: bool) -> f64 {
            if enabled && !value.is_nan() {
                value.clamp(0.0, (extent - viewport).max(0.0))
            } else {
                0.0
            }
        }
        let extent = self.extent();
        let viewport = self.viewport();
        Vector::new(
            axis(
                value.x,
                extent.width,
                viewport.width,
                self.can_horizontally_scroll(),
            ),
            axis(
                value.y,
                extent.height,
                viewport.height,
                self.can_vertically_scroll(),
            ),
        )
    }
    fn notify_geometry_changed(&self) {
        if let Some(owner) = self.owner() {
            owner.on_text_view_geometry_changed();
        }
    }
}
