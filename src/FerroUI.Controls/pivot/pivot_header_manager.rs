//! WinUI's `PivotHeaderManager` (`dxaml/phone/lib/PivotHeaderManager.cpp`, `PivotHeaderManager.h` of
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft
//! Corporation, see NOTICE.md): the header items of a pivot, their panels, their states and the
//! parallax of the headers.

use super::pivot_common::{positive_mod, PivotAnimationDirection};
use super::pivot_curve_generator::{piecewise_linear_expression, PivotCurveGenerator};
use super::pivot_state_machine::PivotCollectionChange;
use super::{Pivot, PivotHeaderItem, PivotHeaderPanel, PivotItem};
use crate::templates::IDataTemplate;
use crate::{Control, ScrollViewer};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::{
    AsCompositionObject, CompositionVisual, ElementComposition, ICompositionObjectAnimations, RedirectVisual,
};
use ferroui_base::{BoxedValue, Ref, Vector, Vector3D};
use std::rc::Rc;

/// The parametric curve applied to a header panel: the visual whose translation follows it.
struct AppliedCurve {
    visual: Rc<CompositionVisual>,
}

/// WinUI's `PivotHeaderManager`.
#[derive(Default)]
pub(crate) struct PivotHeaderManager {
    header_items: Vec<Ref<PivotHeaderItem>>,
    /// The header binding of each item (a subscription to the `Header` of a `PivotItem`).
    header_bindings: Vec<Option<Rc<dyn IDisposable>>>,
    panel: Option<Ref<PivotHeaderPanel>>,
    static_panel: Option<Ref<PivotHeaderPanel>>,
    current_index: i32,
    is_locked: bool,
    use_static_headers: bool,
    should_show_focus_state_on_selected_item: bool,
    curve_generator: PivotCurveGenerator,
    dynamic_curve: Option<AppliedCurve>,
    static_curve: Option<AppliedCurve>,
    /// WinUI's ghost layout transition element: the header before the selected one, shown again
    /// left of it while the headers wrap. The port draws it with a redirect visual of the header.
    ghost: Option<RedirectVisual>,
}

impl PivotHeaderManager {
    pub(crate) fn header_items(&self) -> &[Ref<PivotHeaderItem>] {
        &self.header_items
    }

    /// WinUI's `ApplyTemplateEvent`: the headers move to the panels of the new template.
    pub(crate) fn apply_template_event(
        &mut self,
        pivot: &Pivot,
        panel: Option<Ref<PivotHeaderPanel>>,
        static_panel: Option<Ref<PivotHeaderPanel>>,
    ) {
        self.clear_ghost();
        for old in [self.panel.take(), self.static_panel.take()].into_iter().flatten() {
            old.set_pivot(None);
            old.children().clear();
        }
        self.dynamic_curve = None;
        self.static_curve = None;
        self.panel = panel;
        self.static_panel = static_panel;
        for panel in [&self.panel, &self.static_panel].into_iter().flatten() {
            panel.set_pivot(Some(pivot));
        }
        match (&self.panel, &self.static_panel) {
            (Some(_), None) => self.use_static_headers = false,
            (None, Some(_)) => self.use_static_headers = true,
            _ => {}
        }
        if let Some(target) = self.current_panel() {
            for item in &self.header_items {
                target.children().add(item.clone());
            }
        }
        self.update_items_states();
    }

    /// The panel the headers are in.
    pub(crate) fn current_panel(&self) -> Option<Ref<PivotHeaderPanel>> {
        if self.use_static_headers {
            self.static_panel.clone()
        } else {
            self.panel.clone()
        }
    }

    /// WinUI's `ItemsCollectionChangedEvent`.
    pub(crate) fn items_collection_changed_event(
        &mut self,
        pivot: &Pivot,
        new_item_count: i32,
        change_index: i32,
        change: PivotCollectionChange,
    ) {
        let index = change_index.max(0) as usize;
        if matches!(change, PivotCollectionChange::ItemChanged | PivotCollectionChange::ItemRemoved)
            && index < self.header_items.len()
        {
            self.remove_from_panel(index);
            self.header_items.remove(index);
            if let Some(Some(binding)) = self.header_bindings.get(index).cloned() {
                binding.dispose();
            }
            self.header_bindings.remove(index);
        }
        if matches!(change, PivotCollectionChange::ItemInserted | PivotCollectionChange::ItemChanged) {
            let item = pivot.items_view().get_at(index);
            let header_item = self.create_header_item(pivot);
            let binding = Self::set_binding(item.as_ref(), &header_item);
            let at = index.min(self.header_items.len());
            self.insert_into_panel(&header_item, at);
            self.header_items.insert(at, header_item);
            self.header_bindings.insert(at, binding);
        } else if change == PivotCollectionChange::Reset {
            self.clear_panel();
            for binding in self.header_bindings.drain(..).flatten() {
                binding.dispose();
            }
            self.header_items.clear();
            for index in 0..new_item_count.max(0) as usize {
                let item = pivot.items_view().get_at(index);
                let header_item = self.create_header_item(pivot);
                let binding = Self::set_binding(item.as_ref(), &header_item);
                self.insert_into_panel(&header_item, index);
                self.header_items.push(header_item);
                self.header_bindings.push(binding);
            }
        }
        self.update_items_states();
    }

    /// WinUI's `HeaderTemplateChangedEvent`.
    pub(crate) fn header_template_changed_event(&self, template: Option<Rc<dyn IDataTemplate>>) {
        for item in &self.header_items {
            item.set_content_template(template.clone());
        }
    }

    /// WinUI's `IsLockedChangedEvent`: the other headers fade out (`UnselectedLocked`).
    pub(crate) fn is_locked_changed_event(&mut self, is_locked: bool) {
        if self.is_locked != is_locked {
            self.is_locked = is_locked;
            for item in &self.header_items {
                item.set_is_locked(is_locked);
                item.update_visual_state(true);
            }
            if is_locked {
                self.clear_ghost();
            }
        }
    }

    /// WinUI's `HeaderStateChangedEvent`: the headers move between the dynamic and the static
    /// panel.
    pub(crate) fn header_state_changed_event(&mut self, use_static_headers: bool) {
        if self.use_static_headers == use_static_headers {
            return;
        }
        let target = if use_static_headers { &self.static_panel } else { &self.panel };
        if target.is_none() {
            return;
        }
        self.clear_panel();
        self.use_static_headers = use_static_headers;
        for (index, item) in self.header_items.clone().iter().enumerate() {
            self.insert_into_panel(item, index);
        }
    }

    /// WinUI's `FocusStateChangedEvent`.
    pub(crate) fn focus_state_changed_event(&mut self, should_show_focus_state_on_selected_item: bool) {
        self.should_show_focus_state_on_selected_item = should_show_focus_state_on_selected_item;
        self.update_items_states();
    }

    /// WinUI's `SetSelectedIndex`.
    pub(crate) fn set_selected_index(&mut self, index: i32, animation_hint: PivotAnimationDirection) {
        self.curve_generator.selected_index_changed_event(index, animation_hint);
        self.current_index = index;
        self.update_items_states();
    }

    /// The index of a tapped header (WinUI's `PivotHeaderItemTapped`).
    pub(crate) fn index_of(&self, item: &PivotHeaderItem) -> Option<usize> {
        self.header_items.iter().position(|candidate| std::ptr::eq(&**candidate, item))
    }

    /// WinUI's `UpdateItemsStates`.
    fn update_items_states(&self) {
        for (index, item) in self.header_items.iter().enumerate() {
            item.clear_is_hovered();
            item.set_is_selected(index as i32 == self.current_index);
            item.set_is_locked(self.is_locked);
            item.update_visual_state(true);
        }
    }

    fn create_header_item(&self, pivot: &Pivot) -> Ref<PivotHeaderItem> {
        let header_item = PivotHeaderItem::new();
        header_item.set_content_template(pivot.header_template());
        header_item.set_pivot(Some(pivot));
        header_item.set_is_locked(self.is_locked);
        header_item.update_visual_state(false);
        header_item
    }

    /// WinUI's `SetBinding`: the content of the header is the `Header` of a `PivotItem`, or the
    /// item itself.
    fn set_binding(item: Option<&BoxedValue>, header_item: &Ref<PivotHeaderItem>) -> Option<Rc<dyn IDisposable>> {
        let pivot_item = item.and_then(Control::from_boxed).and_then(|control| control.cast::<PivotItem>());
        match pivot_item {
            Some(pivot_item) => {
                header_item.set_content(pivot_item.header());
                let weak = header_item.downgrade();
                Some(pivot_item.property_changed(move |e| {
                    if e.property() == PivotItem::header_property().as_property() {
                        if let (Some(header_item), Some(pivot_item)) =
                            (weak.upgrade(), e.sender().downcast_ref::<PivotItem>())
                        {
                            header_item.set_content(pivot_item.header());
                        }
                    }
                }))
            }
            None => {
                header_item.set_content(item.cloned());
                None
            }
        }
    }

    fn insert_into_panel(&self, item: &Ref<PivotHeaderItem>, index: usize) {
        if let Some(panel) = self.current_panel() {
            let children = panel.children();
            let at = index.min(children.count());
            children.insert(at, item.clone().upcast::<crate::Control>());
        }
    }

    fn remove_from_panel(&mut self, index: usize) {
        self.clear_ghost();
        if let Some(panel) = self.current_panel() {
            let children = panel.children();
            if index < children.count() {
                children.remove_at(index);
            }
        }
    }

    fn clear_panel(&mut self) {
        self.clear_ghost();
        if let Some(panel) = self.current_panel() {
            panel.children().clear();
        }
    }

    /// The desired widths of the headers (WinUI's `GetHeaderWidths`).
    pub(crate) fn header_widths(&self) -> (Vec<f64>, f64) {
        let widths: Vec<f64> = self.header_items.iter().map(|item| item.desired_size().width).collect();
        let total = widths.iter().sum();
        (widths, total)
    }

    /// WinUI's `ApplyParallax`: the header panel follows the parametric curve of the generator
    /// through an expression animation of its composition `Translation` that reads the offset of
    /// the scroll viewer (WinUI's parallax secondary content relationship of DManip). The value
    /// at the current offset is applied at once as well, and the dynamic panel gets the offset of
    /// its first header.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_parallax(
        &mut self,
        scroll_viewer: &ScrollViewer,
        scroller: Option<Rc<dyn AsCompositionObject>>,
        section_offset: f64,
        section_width: f64,
        is_dynamic_header: bool,
        viewport_size: f32,
        panel_multiplier: u32,
    ) {
        self.curve_generator.sync_section_width(section_width);
        self.curve_generator.sync_selected_item_offset(section_offset);
        let (widths, total) = self.header_widths();
        self.curve_generator.sync_items_size_and_order(widths, total);

        let Some(panel) = (if is_dynamic_header { self.panel.clone() } else { self.static_panel.clone() }) else {
            return;
        };
        let visual = ElementComposition::get_element_visual(&panel);
        let applied = if is_dynamic_header { &self.dynamic_curve } else { &self.static_curve };
        let current = applied
            .as_ref()
            .is_some_and(|curve| visual.as_ref().is_some_and(|visual| Rc::ptr_eq(visual, &curve.visual)));
        if self.curve_generator.are_curves_dirty(is_dynamic_header) || !current {
            let (primary, mut secondary) = if is_dynamic_header {
                self.curve_generator.dynamic_curve_segments(panel_multiplier)
            } else {
                self.curve_generator.static_curve_segments(viewport_size as f64)
            };
            if is_dynamic_header && primary.len() > 2 {
                // The layout element follows the viewport, and the header panel is inside it.
                let last = primary.len() - 1;
                for i in 1..last {
                    secondary[i] += primary[i];
                }
                secondary[0] = secondary[1];
                secondary[last] = secondary[last - 1];
            }
            if let (Some(visual), Some(scroller)) = (&visual, scroller) {
                let expression = format!(
                    "Vector3(-{}, 0, 0)",
                    piecewise_linear_expression(&primary, &secondary, "(-scroller.Translation.X)")
                );
                let animation = visual.compositor().create_expression_animation_with(&expression);
                animation.set_reference_parameter("scroller", scroller);
                visual.start_animation("Translation", &*animation);
            }
            let curve = visual.map(|visual| AppliedCurve { visual });
            if is_dynamic_header {
                self.dynamic_curve = curve;
            } else {
                self.static_curve = curve;
            }
        }
        let _ = scroll_viewer;
        if is_dynamic_header {
            panel.set_current_offset(self.curve_generator.header_panel_offset());
            let count = self.header_items.len() as i32;
            panel.set_current_index(positive_mod(self.current_index, count.max(1)) as usize);
        }
    }

    /// WinUI's `HeaderPanelSetLteOffsetEvent` and `UpdateGhostItem`: the header before the
    /// selected one is drawn again at `ghost_offset`, left of the first header, while the
    /// headers wrap and the pivot is not locked.
    pub(crate) fn update_ghost(&mut self, panel: &PivotHeaderPanel, ghost_offset: f64, vertical_offset: f64) {
        let is_dynamic_panel = self.panel.as_ref().is_some_and(|candidate| std::ptr::eq(&**candidate, panel));
        let count = self.header_items.len() as i32;
        if !is_dynamic_panel || self.use_static_headers || self.is_locked || count == 0 {
            self.clear_ghost();
            return;
        }
        let ghost_item = &self.header_items[positive_mod(self.current_index - 1, count) as usize];
        let (Some(source), Some(panel_visual)) =
            (ElementComposition::get_element_visual(ghost_item), ElementComposition::get_element_visual(panel))
        else {
            return;
        };
        let ghost = match &self.ghost {
            Some(ghost) => {
                ghost.set_source(Some(source));
                ghost.clone()
            }
            None => {
                let ghost = panel_visual.compositor().create_redirect_visual_with_source_visual(Some(source));
                ElementComposition::set_element_child_visual(panel, Some((*ghost).clone()));
                self.ghost = Some(ghost.clone());
                ghost
            }
        };
        let size = ghost_item.bounds().size();
        ghost.set_offset(Vector3D::new(ghost_offset, vertical_offset, 0.0));
        ghost.set_size(Vector::new(size.width, size.height));
    }

    /// WinUI's `ClearExistingLtes`.
    pub(crate) fn clear_ghost(&mut self) {
        if self.ghost.take().is_some() {
            if let Some(panel) = &self.panel {
                ElementComposition::set_element_child_visual(panel, None);
            }
        }
    }

    /// WinUI's `UnloadedEvent`.
    pub(crate) fn unloaded_event(&mut self) {
        self.dynamic_curve = None;
        self.static_curve = None;
        self.clear_ghost();
    }
}
