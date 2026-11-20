//! WinUI's `PivotPanel` (`dxaml/phone/lib/PivotPanel_Partial.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::pivot_common::PIVOT_PANEL_MULTIPLIER;
use super::Pivot;
use crate::primitives::{
    register_scroll_snap_points_info, IScrollSnapPointsInfo, SnapPointsAlignment, SnapPointsChangedHandler,
};
use crate::{Control, ControlImpl, Panel, PanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_routed_event, instantiate, FerroObjectImpl, Rect, Ref, Size,
    StyledElementImpl, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The names of the children the panel arranges in a particular way (the template parts of the
/// pivot).
pub(crate) const LAYOUT_ELEMENT_NAME: &str = "PivotLayoutElement";
pub(crate) const ITEMS_PRESENTER_NAME: &str = "PivotItemPresenter";
pub(crate) const HEADERS_CONTROL_NAME: &str = "Header";

/// The panel inside the scroll viewer of a [`Pivot`] (WinUI's `PivotPanel`): as wide as
/// [`PIVOT_PANEL_MULTIPLIER`] sections while the headers wrap, with a regular snap point at every
/// section, so that the scroll viewer pans one section at a time.
///
/// Its child named `PivotLayoutElement` (the headers and the items) is measured and arranged one
/// section wide. WinUI arranges it at the start of the panel and keeps it in the viewport with a
/// secondary content relationship of the scroll viewer; here it is arranged at the section the
/// viewport rests on (the pivot's section offset), and the pivot's composition expression keeps it
/// in place during a pan (a recorded deviation: layout and hit testing stay where the content is
/// shown at rest).
#[repr(C)]
pub struct PivotPanel {
    base: Panel,
    available_width: Cell<f64>,
    header_height: Cell<f64>,
    section_offset: Cell<f64>,
    parent_pivot: RefCell<Option<WeakRef<Pivot>>>,
}

ferro_class!(PivotPanel: Panel);
ferroui_base::ferro_class_info!(PivotPanel {
    new: PivotPanel::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(
    PivotPanel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    FerroObjectImpl
);

impl PivotPanel {
    ferro_routed_event!(
        /// Defines the `HorizontalSnapPointsChanged` event.
        pub fn horizontal_snap_points_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<PivotPanel, _>("HorizontalSnapPointsChanged", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `VerticalSnapPointsChanged` event.
        pub fn vertical_snap_points_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<PivotPanel, _>("VerticalSnapPointsChanged", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        register_scroll_snap_points_info::<PivotPanel>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            available_width: Cell::new(0.0),
            header_height: Cell::new(0.0),
            section_offset: Cell::new(0.0),
            parent_pivot: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub(crate) fn set_parent_pivot(&self, pivot: Option<&Pivot>) {
        *self.parent_pivot.borrow_mut() = pivot.map(|pivot| pivot.to_ref().downgrade());
    }

    fn parent_pivot(&self) -> Option<Ref<Pivot>> {
        self.parent_pivot.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// The width of a section (WinUI's `SetSectionWidth`).
    pub(crate) fn set_section_width(&self, width: f64) {
        self.available_width.set(width);
        self.invalidate_measure();
        self.raise_snap_points_changed_events();
    }

    /// The width of a section.
    pub fn section_width(&self) -> f64 {
        self.available_width.get()
    }

    /// The offset of the section the viewport rests on, where the layout element is arranged.
    pub(crate) fn set_section_offset(&self, offset: f64) {
        if self.section_offset.get() != offset {
            self.section_offset.set(offset);
            self.invalidate_arrange();
        }
    }

    pub(crate) fn section_offset(&self) -> f64 {
        self.section_offset.get()
    }

    fn panel_multiplier(&self) -> f64 {
        self.parent_pivot().map_or(PIVOT_PANEL_MULTIPLIER, |pivot| pivot.pivot_panel_multiplier_impl()) as f64
    }

    fn raise_snap_points_changed_events(&self) {
        self.raise_event(&RoutedEventArgs::with_event(Self::vertical_snap_points_changed_event()));
        self.raise_event(&RoutedEventArgs::with_event(Self::horizontal_snap_points_changed_event()));
    }

    /// Whether the horizontal snap points are regular: they are.
    pub fn are_horizontal_snap_points_regular(&self) -> bool {
        true
    }

    /// Whether the vertical snap points are regular: they are.
    pub fn are_vertical_snap_points_regular(&self) -> bool {
        true
    }

    /// The regular snap points: one per section, the first at the middle of the first section
    /// (WinUI's `GetRegularSnapPoints`); the distance first, then the offset.
    pub fn get_regular_snap_points(&self, _orientation: Orientation, _alignment: SnapPointsAlignment) -> (f64, f64) {
        let width = self.available_width.get();
        (width, width / 2.0)
    }
}

fn child_name(child: &Control) -> Option<String> {
    child.name()
}

impl LayoutableImpl for PivotPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut layout_element = None;
        let mut header_control = None;
        let mut items_presenter = None;
        for child in this.children().snapshot().iter() {
            match child_name(child).as_deref() {
                Some(LAYOUT_ELEMENT_NAME) => layout_element = Some(child.clone()),
                Some(HEADERS_CONTROL_NAME) => header_control = Some(child.clone()),
                Some(ITEMS_PRESENTER_NAME) => items_presenter = Some(child.clone()),
                _ => child.measure(available_size),
            }
        }

        let mut desired = Size::default();
        if let Some(layout_element) = &layout_element {
            layout_element.measure(Size::new(this.available_width.get(), available_size.height));
            desired = layout_element.desired_size();
        }
        if let Some(header_control) = &header_control {
            header_control.measure(available_size);
            this.header_height.set(header_control.desired_size().height);
        }
        if let Some(items_presenter) = &items_presenter {
            let height = (available_size.height - this.header_height.get()).max(0.0);
            items_presenter.measure(Size::new(this.available_width.get(), height));
            desired = items_presenter.desired_size();
        }

        if this.available_width.get().is_infinite() {
            this.available_width.set(desired.width);
        }
        Size::new(this.available_width.get() * this.panel_multiplier(), desired.height + this.header_height.get())
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(pivot) = this.parent_pivot() {
            pivot.invalidate_header_secondary_content_relationships();
        }
        let width = this.available_width.get();
        let offset = this.section_offset.get();
        for child in this.children().snapshot().iter() {
            let rect = match child_name(child).as_deref() {
                Some(LAYOUT_ELEMENT_NAME) => Rect::new(offset, 0.0, width, final_size.height),
                Some(ITEMS_PRESENTER_NAME) => Rect::new(
                    offset,
                    this.header_height.get(),
                    width,
                    (final_size.height - this.header_height.get()).max(0.0),
                ),
                Some(HEADERS_CONTROL_NAME) => Rect::new(0.0, 0.0, final_size.width, this.header_height.get()),
                _ => Rect::from_size(final_size),
            };
            child.arrange(rect);
        }
        final_size
    }
}

impl IScrollSnapPointsInfo for PivotPanel {
    fn are_horizontal_snap_points_regular(&self) -> bool {
        true
    }

    fn set_are_horizontal_snap_points_regular(&self, _value: bool) {}

    fn are_vertical_snap_points_regular(&self) -> bool {
        true
    }

    fn set_are_vertical_snap_points_regular(&self, _value: bool) {}

    fn get_irregular_snap_points(&self, _orientation: Orientation, _alignment: SnapPointsAlignment) -> Vec<f64> {
        // WinUI returns E_NOTIMPL: the snap points are regular.
        Vec::new()
    }

    fn get_regular_snap_points(&self, orientation: Orientation, alignment: SnapPointsAlignment) -> (f64, f64) {
        PivotPanel::get_regular_snap_points(self, orientation, alignment)
    }

    fn horizontal_snap_points_changed(&self, handler: SnapPointsChangedHandler) -> Rc<dyn IDisposable> {
        self.add_disposable_handler(Self::horizontal_snap_points_changed_event(), move |sender, e| handler(sender, e))
            .detach()
    }

    fn vertical_snap_points_changed(&self, handler: SnapPointsChangedHandler) -> Rc<dyn IDisposable> {
        self.add_disposable_handler(Self::vertical_snap_points_changed_event(), move |sender, e| handler(sender, e))
            .detach()
    }
}
