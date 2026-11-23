//! WinUI's `PivotHeaderItem` (`dxaml/phone/lib/PivotHeaderItem.cpp`, `PivotHeaderItem.h` of
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft
//! Corporation, see NOTICE.md).

use super::Pivot;
use crate::metadata::PseudoClassesAttribute;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{ContentControl, ContentControlImpl, ControlImpl, VisualStateManager};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, TappedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs,
    Ref, StyledElementImpl, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};

const PC_SELECTED: &str = ":selected";
const PC_PRESSED: &str = ":pressed";
const PC_LOCKED: &str = ":locked";

/// A header of a [`Pivot`] (WinUI's `PivotHeaderItem`): the pivot makes one per item, its content
/// bound to the item's header.
///
/// Its selection state is WinUI's: `Disabled`, `Selected`, `SelectedPointerOver`,
/// `SelectedPressed`, `Unselected`, `UnselectedPointerOver`, `UnselectedPressed` and
/// `UnselectedLocked` (the pivot is locked). The states are shown through the pseudo-classes
/// `:selected`, `:pressed` and `:locked` (with `:pointerover` and `:disabled`), as the other WinUI
/// controls of the theme (decision D2), and a template with visual state groups gets the state by
/// name through the visual state manager.
#[repr(C)]
pub struct PivotHeaderItem {
    base: ContentControl,
    pivot: RefCell<Option<WeakRef<Pivot>>>,
    is_selected: Cell<bool>,
    is_pressed: Cell<bool>,
    is_pointer_over: Cell<bool>,
    is_locked: Cell<bool>,
}

ferro_class!(PivotHeaderItem: ContentControl);
ferroui_base::ferro_class_info!(PivotHeaderItem {
    new: PivotHeaderItem::new,
    markup: { namespace: "FerroUI.Controls.Primitives", content: Content },
});
ferro_impl_classes!(
    PivotHeaderItem: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ContentControlImpl
);

impl FerroObjectImpl for PivotHeaderItem {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        // WinUI's `IsEnabledChanged` handler.
        if change.property() == InputElement::is_effectively_enabled_property().as_property() {
            this.update_visual_state(true);
        }
    }
}

impl ControlImpl for PivotHeaderItem {}

impl TemplatedControlImpl for PivotHeaderItem {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.update_visual_state(false);
    }
}

impl InputElementImpl for PivotHeaderItem {
    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);
        if !this.is_pointer_over.replace(true) {
            this.update_visual_state(true);
        }
    }

    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        let pressed = this.is_pressed.replace(false);
        let over = this.is_pointer_over.replace(false);
        if pressed || over {
            this.update_visual_state(true);
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if !this.is_pressed.replace(true) {
            this.update_visual_state(true);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if this.is_pressed.replace(false) {
            this.update_visual_state(true);
        }
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        let pressed = this.is_pressed.replace(false);
        let over = this.is_pointer_over.replace(false);
        if pressed || over {
            this.update_visual_state(true);
        }
    }

    /// WinUI's `OnTapped`: the header manager moves to the item.
    fn on_tapped(this: &Self, e: &TappedEventArgs) {
        Self::parent_on_tapped(this, e);
        if let Some(pivot) = this.pivot() {
            pivot.on_header_item_tapped(&this.to_ref());
        }
    }
}

impl PivotHeaderItem {
    /// The pseudo-classes set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute =
        PseudoClassesAttribute::new(&[PC_SELECTED, PC_PRESSED, PC_LOCKED]);

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            pivot: RefCell::new(None),
            is_selected: Cell::new(false),
            is_pressed: Cell::new(false),
            is_pointer_over: Cell::new(false),
            is_locked: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn pivot(&self) -> Option<Ref<Pivot>> {
        self.pivot.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// WinUI's `SetHeaderManagerCallbacks`.
    pub(crate) fn set_pivot(&self, pivot: Option<&Pivot>) {
        *self.pivot.borrow_mut() = pivot.map(|pivot| pivot.to_ref().downgrade());
    }

    /// Whether the header is the header of the selected item.
    pub fn is_selected(&self) -> bool {
        self.is_selected.get()
    }

    pub(crate) fn set_is_selected(&self, value: bool) {
        self.is_selected.set(value);
    }

    pub(crate) fn set_is_locked(&self, value: bool) {
        self.is_locked.set(value);
    }

    /// WinUI's `ClearIsHovered`.
    pub(crate) fn clear_is_hovered(&self) {
        self.is_pointer_over.set(false);
    }

    /// WinUI's state of the header, by name.
    pub fn visual_state_name(&self) -> &'static str {
        if !self.is_effectively_enabled() {
            "Disabled"
        } else if self.is_selected.get() {
            if self.is_pressed.get() {
                "SelectedPressed"
            } else if self.is_pointer_over.get() {
                "SelectedPointerOver"
            } else {
                "Selected"
            }
        } else if self.is_locked.get() {
            "UnselectedLocked"
        } else if self.is_pressed.get() {
            "UnselectedPressed"
        } else if self.is_pointer_over.get() {
            "UnselectedPointerOver"
        } else {
            "Unselected"
        }
    }

    /// WinUI's `UpdateVisualState`.
    pub(crate) fn update_visual_state(&self, use_transitions: bool) {
        let state = self.visual_state_name();
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes.set(PC_SELECTED, self.is_selected.get());
        pseudo_classes.set(PC_PRESSED, self.is_pressed.get());
        pseudo_classes.set(PC_LOCKED, state == "UnselectedLocked");
        VisualStateManager::go_to_state(self, state, use_transitions);
    }
}
