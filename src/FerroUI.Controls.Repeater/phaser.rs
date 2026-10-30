// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::build_tree_scheduler::BuildTreeScheduler;
use super::virtualization_info::VirtualizationInfo;
use ferroui_controls::Control;
use ferroui_base::layout::LayoutInformation;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_properties, ferro_static_type, AttachedProperty, BoxedValue, FerroProperty, Rect, Ref,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Binding component attached by a template with phased bindings. Phase zero
/// initializes the element; positive returned phases are deferred.
pub trait IDataTemplateComponent {
    fn process_bindings(
        &self,
        data: Option<BoxedValue>,
        item_index: i32,
        current_phase: i32,
    ) -> i32;
    fn recycle(&self);
}
impl PartialEq for dyn IDataTemplateComponent {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// Native attached-property counterpart of the template-component binding hook.
pub struct RepeaterBindingHelper;
ferro_static_type!(RepeaterBindingHelper);
ferro_properties! {
    impl RepeaterBindingHelper {
        pub fn data_template_component_property() -> AttachedProperty<Option<Rc<dyn IDataTemplateComponent>>> {
            FerroProperty::register_attached::<RepeaterBindingHelper, Control, _>("DataTemplateComponent", None)
        }
    }
}

#[derive(Clone)]
struct ElementInfo {
    element: Ref<Control>,
    info: Rc<VirtualizationInfo>,
    order: u64,
}

/// Coordinates phase work for one repeater. Callbacks capture the owner weakly;
/// recycling cancels outstanding work before an element can be reused.
pub struct Phaser {
    visible_window: Rc<dyn Fn() -> Rect>,
    item_at: Rc<dyn Fn(i32) -> Option<BoxedValue>>,
    pending: RefCell<Vec<ElementInfo>>,
    registration: RefCell<Option<Rc<dyn IDisposable>>>,
    next_order: std::cell::Cell<u64>,
}
impl Phaser {
    pub fn new(
        visible_window: Rc<dyn Fn() -> Rect>,
        item_at: Rc<dyn Fn(i32) -> Option<BoxedValue>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            visible_window,
            item_at,
            pending: RefCell::new(Vec::new()),
            registration: RefCell::new(None),
            next_order: std::cell::Cell::new(0),
        })
    }
    pub fn get_data_template_component(
        element: &Control,
    ) -> Option<Rc<dyn IDataTemplateComponent>> {
        element.get_value(RepeaterBindingHelper::data_template_component_property())
    }
    pub fn set_data_template_component(
        element: &Control,
        component: Option<Rc<dyn IDataTemplateComponent>>,
    ) {
        element.set_value(
            RepeaterBindingHelper::data_template_component_property(),
            component,
        );
    }
    /// Runs phase zero before the host assigns DataContext. Deferred work is
    /// registered separately, after ElementPrepared has notified application
    /// code and the new root is in the visual tree.
    pub fn initialize_phase_zero(
        element: &Ref<Control>,
        info: &Rc<VirtualizationInfo>,
        data: Option<BoxedValue>,
        index: i32,
    ) {
        if let Some(component) = Self::get_data_template_component(element) {
            let generation = info.phase_generation();
            component.recycle();
            let next = component.process_bindings(data.clone(), index, 0);
            Self::validate_phase_ordering(0, next);
            if info.phase_generation() == generation {
                info.update_phasing_info(
                    next,
                    if next > 0 { data } else { None },
                    if next > 0 { Some(component) } else { None },
                );
            }
        }
    }
    pub fn phase_element(self: &Rc<Self>, element: &Ref<Control>, info: &Rc<VirtualizationInfo>) {
        let phase = info.phase();
        if phase == VirtualizationInfo::PHASE_NOT_SPECIFIED
            && Self::get_data_template_component(element).is_some()
        {
            Self::initialize_phase_zero(element, info, (self.item_at)(info.index()), info.index());
        }
        if info.phase() > 0 {
            assert!(
                info.data_template_component().is_some(),
                "Phase was set without a data-template component."
            );
            if !self
                .pending
                .borrow()
                .iter()
                .any(|entry| entry.element == *element)
            {
                let order = self.next_order.get();
                self.next_order.set(order.wrapping_add(1));
                self.pending.borrow_mut().insert(
                    0,
                    ElementInfo {
                        element: element.clone(),
                        info: info.clone(),
                        order,
                    },
                );
            }
            self.register_for_callback();
        }
    }
    pub fn stop_phasing(&self, element: &Ref<Control>, info: &Rc<VirtualizationInfo>) {
        let removed = {
            let mut pending = self.pending.borrow_mut();
            pending
                .iter()
                .position(|entry| entry.element == *element)
                .map(|index| pending.remove(index))
        };
        drop(removed);
        info.update_phasing_info(VirtualizationInfo::PHASE_NOT_SPECIFIED, None, None);
        if self.pending.borrow().is_empty() {
            let registration = self.registration.borrow_mut().take();
            if let Some(registration) = registration {
                registration.dispose();
            }
        }
    }
    pub fn pending_count(&self) -> usize {
        self.pending.borrow().len()
    }
    pub fn validate_phase_ordering(current: i32, next: i32) {
        assert!(
            next <= 0 || next > current,
            "Phases are required to be monotonically increasing."
        );
    }
    fn register_for_callback(self: &Rc<Self>) {
        if self.registration.borrow().is_some() {
            return;
        }
        let priority = self.pending.borrow().last().map(|entry| entry.info.phase());
        if let Some(priority) = priority {
            let weak = Rc::downgrade(self);
            let registration = BuildTreeScheduler::register_work(priority, move || {
                if let Some(this) = weak.upgrade() {
                    this.do_phased_work_callback();
                }
            });
            *self.registration.borrow_mut() = Some(registration);
        }
    }
    fn do_phased_work_callback(self: &Rc<Self>) {
        self.registration.borrow_mut().take();
        let visible = (self.visible_window)();
        // Preserve the source's comparator and tail-first traversal together.
        // In each visibility group the highest pending phase is initially at
        // the tail; later phases round-robin over the remaining elements.
        self.pending.borrow_mut().sort_by(|left, right| {
            left.info
                .arrange_bounds()
                .intersects(visible)
                .cmp(&right.info.arrange_bounds().intersects(visible))
                .then_with(|| left.info.phase().cmp(&right.info.phase()))
                .then_with(|| right.order.cmp(&left.order))
        });
        let mut current_index = self.pending.borrow().len() as isize - 1;
        while !BuildTreeScheduler::should_yield() {
            let next = self.pending.borrow().get(current_index as usize).cloned();
            let Some(entry) = next else {
                break;
            };
            let current = entry.info.phase();
            let generation = entry.info.phase_generation();
            assert!(
                current > 0,
                "A cleared element was found in the pending phase list."
            );
            let component = entry
                .info
                .data_template_component()
                .expect("A pending phase lost its template component.");
            let next = component.process_bindings(entry.info.data(), -1, current);
            Self::validate_phase_ordering(current, next);
            // A binding callback may recycle the element. Do not reschedule it
            // or overwrite a different item's phase state after that callback.
            if entry.info.phase_generation() == generation && entry.info.phase() == current {
                if let Some(size) =
                    LayoutInformation::get_previous_measure_constraint(&entry.element)
                {
                    entry.element.measure(size);
                }
                let position = self
                    .pending
                    .borrow()
                    .iter()
                    .position(|pending| pending.order == entry.order);
                if let Some(position) = position {
                    current_index = position as isize;
                    if next > 0 {
                        entry.info.set_phase(next);
                        if position == 0 || next > self.pending.borrow()[position - 1].info.phase()
                        {
                            current_index -= 1;
                        }
                    } else {
                        self.pending.borrow_mut().remove(position);
                        // Source retains the last positive phase and its weak
                        // data/component references until StopPhasing resets
                        // them during recycling.
                        current_index -= 1;
                    }
                }
            }
            let pending = self.pending.borrow();
            if pending.is_empty() {
                break;
            }
            if current_index < 0 || current_index as usize >= pending.len() {
                current_index = pending.len() as isize - 1;
            } else if (current_index as usize) < pending.len() - 1
                && !pending[current_index as usize]
                    .info
                    .arrange_bounds()
                    .intersects(visible)
                && pending
                    .last()
                    .expect("Pending elements exist.")
                    .info
                    .arrange_bounds()
                    .intersects(visible)
            {
                current_index = pending.len() as isize - 1;
            }
        }
        self.register_for_callback();
    }
}
impl Drop for Phaser {
    fn drop(&mut self) {
        if let Some(registration) = self.registration.get_mut().take() {
            registration.dispose();
        }
    }
}
