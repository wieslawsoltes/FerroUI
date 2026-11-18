//! WinUI's `PivotItem` and `PivotItemEventArgs` (`dxaml/phone/lib/PivotItem_Partial.cpp`,
//! `PivotItemEventArgs.cpp` of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md).

use super::Pivot;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{ContentControl, ContentControlImpl, ControlImpl};
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_type, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};

/// The arguments of the item events of a [`Pivot`] (WinUI's `PivotItemEventArgs`).
#[derive(Clone, Debug, PartialEq)]
pub struct PivotItemEventArgs {
    item: Ref<PivotItem>,
}

impl PivotItemEventArgs {
    pub(crate) fn new(item: Ref<PivotItem>) -> Self {
        Self { item }
    }

    /// The pivot item that is loading, loaded, unloading or unloaded.
    pub fn item(&self) -> Ref<PivotItem> {
        self.item.clone()
    }
}

ferro_markup_type!(class PivotItemEventArgs {
    namespace: "FerroUI.Controls",
    handles: [PivotItemEventArgs],
    properties: [
        Item: Ref<PivotItem> { get: |args: &PivotItemEventArgs| args.item() },
    ],
});

/// An item of a [`Pivot`] (WinUI's `PivotItem`): a content control with the
/// [`header`](Self::header) the pivot shows for it.
///
/// The pivot shows one item at a time (and its neighbours while it is panned): the others are
/// transparent, not hit testable, and the root of their template is collapsed so that tabbing
/// skips them (WinUI's `SetContentVisibility`).
#[repr(C)]
pub struct PivotItem {
    base: ContentControl,
    parent_pivot: RefCell<Option<WeakRef<Pivot>>>,
    /// A content visibility set before the template was applied.
    pending_content_visibility: Cell<Option<bool>>,
    is_keyboard_focus_pending: Cell<bool>,
    last_measure_available_size: Cell<Size>,
}

ferro_class!(PivotItem: ContentControl);
ferroui_base::ferro_class_info!(PivotItem {
    new: PivotItem::new,
    markup: { namespace: "FerroUI.Controls", content: Content },
});
ferro_impl_classes!(
    PivotItem: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ContentControlImpl,
    FerroObjectImpl
);

ferroui_base::ferro_properties! { impl PivotItem {
    ferro_property!(
        /// Defines the `Header` property: the content of the header of the item.
        pub fn header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<PivotItem, _>("Header", None)
        }
    );
} }

impl ControlImpl for PivotItem {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::PivotItemAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for PivotItem {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        if let Some(visible) = this.pending_content_visibility.take() {
            this.set_content_visibility(visible);
        }
    }
}

impl LayoutableImpl for PivotItem {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.last_measure_available_size.set(available_size);
        let result = Self::parent_measure_override(this, available_size);
        if this.is_keyboard_focus_pending.take() {
            this.focus();
        }
        result
    }
}

impl PivotItem {
    fn static_constructor() {
        // WinUI's default style: the items are not tab stops.
        InputElement::focusable_property().override_default_value::<PivotItem>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            parent_pivot: RefCell::new(None),
            pending_content_visibility: Cell::new(None),
            is_keyboard_focus_pending: Cell::new(false),
            last_measure_available_size: Cell::new(Size::default()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// An item with `header` and `content`.
    pub fn with_header(header: impl Into<BoxedValue>, content: Option<BoxedValue>) -> Ref<Self> {
        let item = Self::new();
        item.set_header(Some(header.into()));
        item.set_content(content);
        item
    }

    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value)
    }

    /// The pivot that shows the item.
    pub fn parent_pivot(&self) -> Option<Ref<Pivot>> {
        self.parent_pivot.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_parent_pivot(&self, pivot: Option<&Pivot>) {
        *self.parent_pivot.borrow_mut() = pivot.map(|pivot| pivot.to_ref().downgrade());
    }

    /// The root of the template (WinUI's `GetFirstChild`).
    fn first_child(&self) -> Option<Ref<Visual>> {
        self.visual_children().snapshot().first().cloned()
    }

    /// Whether the root of the template is visible (WinUI's `GetContentVisibility`); `false`
    /// without a template.
    pub(crate) fn content_visibility(&self) -> bool {
        match self.first_child() {
            Some(child) => child.is_visible(),
            None => self.pending_content_visibility.get().unwrap_or(false),
        }
    }

    /// Shows or collapses the root of the template (WinUI's `SetContentVisibility`), once it is
    /// applied.
    pub(crate) fn set_content_visibility(&self, visible: bool) {
        match self.first_child() {
            Some(child) => child.set_is_visible(visible),
            None => self.pending_content_visibility.set(Some(visible)),
        }
    }

    /// Realizes the item template of the content before the loaded events are raised (WinUI's
    /// `RealizeContent`): the template root is measured once at the last size.
    pub(crate) fn realize_content(&self) {
        if let Some(child) = self.first_child().and_then(|child| child.cast::<crate::Control>()) {
            child.measure(self.last_measure_available_size.get());
        }
    }

    /// Focuses the item now, or after its next measure (WinUI's `SetKeyboardFocus`).
    pub(crate) fn set_keyboard_focus(&self, postpone_until_next_measure: bool) {
        self.is_keyboard_focus_pending.set(postpone_until_next_measure);
        if postpone_until_next_measure {
            self.invalidate_measure();
        } else {
            self.focus();
        }
    }

    /// Whether `element` is in the content of the item (WinUI's `HasElement`).
    pub(crate) fn has_element(&self, element: &Visual) -> bool {
        let Some(child) = self.first_child() else { return false };
        let mut current = Some(element.to_ref());
        while let Some(visual) = current {
            if std::ptr::eq(&*visual, &*child) {
                return true;
            }
            current = visual.visual_parent();
        }
        false
    }
}
