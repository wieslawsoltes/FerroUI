use super::CollectionUtils;
use crate::Control;
use ferroui_base::layout::Orientation;
use ferroui_base::Ref;
use std::cell::{Cell, Ref as CellRef, RefCell};

/// Stores the realized element state for a virtualizing panel that arranges
/// its children in a stack layout, such as `VirtualizingStackPanel`.
///
/// The state is interior-mutable: the methods that call back into the panel
/// (to recycle an element or to update its index) never hold a borrow of the
/// element list across the callback.
pub(crate) struct RealizedStackElements {
    first_index: Cell<i32>,
    elements: RefCell<Vec<Option<Ref<Control>>>>,
    sizes: RefCell<Vec<f64>>,
    start_u: Cell<f64>,
    start_u_unstable: Cell<bool>,
}

impl RealizedStackElements {
    pub fn new() -> Self {
        Self {
            first_index: Cell::new(0),
            elements: RefCell::new(Vec::new()),
            sizes: RefCell::new(Vec::new()),
            start_u: Cell::new(0.0),
            start_u_unstable: Cell::new(false),
        }
    }

    /// Gets the number of realized elements.
    #[inline]
    pub fn count(&self) -> usize {
        self.elements.borrow().len()
    }

    /// Gets the index of the first realized element, or -1 if no elements
    /// are realized.
    #[inline]
    pub fn first_index(&self) -> i32 {
        if self.count() > 0 {
            self.first_index.get()
        } else {
            -1
        }
    }

    /// Gets the index of the last realized element, or -1 if no elements are
    /// realized.
    #[inline]
    pub fn last_index(&self) -> i32 {
        let count = self.count();
        if count > 0 {
            self.first_index.get() + count as i32 - 1
        } else {
            -1
        }
    }

    /// Gets the elements.
    ///
    /// The returned borrow must not be held across a call that can run code
    /// outside the panel; use [`element_at`](Self::element_at) for that.
    #[inline]
    pub fn elements(&self) -> CellRef<'_, Vec<Option<Ref<Control>>>> {
        self.elements.borrow()
    }

    /// Gets the sizes of the elements on the primary axis.
    #[inline]
    pub fn size_u(&self) -> CellRef<'_, Vec<f64>> {
        self.sizes.borrow()
    }

    /// Gets the element at the specified position of the realized list (not
    /// of the source collection), if there is one.
    #[inline]
    pub fn element_at(&self, i: usize) -> Option<Ref<Control>> {
        self.elements.borrow().get(i).cloned().flatten()
    }

    /// Gets the size on the primary axis of the element at the specified
    /// position of the realized list (not of the source collection).
    #[inline]
    pub fn size_u_at(&self, i: usize) -> f64 {
        self.sizes.borrow()[i]
    }

    /// Gets the position of the first element on the primary axis, or NaN if
    /// the position is unstable.
    #[inline]
    pub fn start_u(&self) -> f64 {
        if self.start_u_unstable.get() {
            f64::NAN
        } else {
            self.start_u.get()
        }
    }

    /// Adds a newly realized element to the collection.
    ///
    /// `index` is the index of the element, `u` the position of the element
    /// on the primary axis and `size_u` the size of the element on the
    /// primary axis.
    pub fn add(&self, index: i32, element: Ref<Control>, u: f64, size_u: f64) {
        if index < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'index')");
        }

        if self.count() == 0 {
            self.elements.borrow_mut().push(Some(element));
            self.sizes.borrow_mut().push(size_u);
            self.start_u.set(u);
            self.first_index.set(index);
        } else if index == self.last_index() + 1 {
            self.elements.borrow_mut().push(Some(element));
            self.sizes.borrow_mut().push(size_u);
        } else if index == self.first_index() - 1 {
            self.first_index.set(self.first_index.get() - 1);
            self.elements.borrow_mut().insert(0, Some(element));
            self.sizes.borrow_mut().insert(0, size_u);
            self.start_u.set(u);
        } else {
            panic!("Can only add items to the beginning or end of realized elements.");
        }
    }

    /// Gets the element at the specified index, if realized.
    ///
    /// `index` is the index in the source collection of the element to get.
    /// Returns the element if realized; otherwise `None`.
    #[inline]
    pub fn get_element(&self, index: i32) -> Option<Ref<Control>> {
        let elements = self.elements.borrow();
        let first_index = if elements.is_empty() { -1 } else { self.first_index.get() };
        let i = index - first_index;
        if i >= 0 && (i as usize) < elements.len() {
            return elements[i as usize].clone();
        }
        None
    }

    /// Gets the position of the element with the requested index on the
    /// primary axis, if realized.
    ///
    /// Returns the position of the element, or NaN if the element is not
    /// realized.
    pub fn get_element_u(&self, index: i32) -> f64 {
        let first_index = self.first_index();

        if index < first_index {
            return f64::NAN;
        }

        let end_index = (index - first_index) as usize;
        let sizes = self.sizes.borrow();

        if end_index >= sizes.len() {
            return f64::NAN;
        }

        let mut u = self.start_u();

        for size in &sizes[..end_index] {
            u += *size;
        }

        u
    }

    /// Gets the index of the specified element.
    ///
    /// Returns the index or -1 if the element is not present in the
    /// collection.
    pub fn get_index(&self, element: &Ref<Control>) -> i32 {
        let elements = self.elements.borrow();
        match elements.iter().position(|e| e.as_ref() == Some(element)) {
            Some(index) => index as i32 + self.first_index.get(),
            None => -1,
        }
    }

    /// Updates the elements in response to items being inserted into the
    /// source collection.
    ///
    /// `index` is the index in the source collection of the insert, `count`
    /// the number of items inserted and `update_element_index` a method used
    /// to update the element indexes.
    pub fn items_inserted(&self, index: i32, count: i32, update_element_index: impl Fn(&Ref<Control>, i32, i32)) {
        if index < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'index')");
        }
        if self.count() == 0 {
            return;
        }

        // Get the index within the realized elements collection.
        let first = self.first_index();
        let realized_index = index - first;

        if realized_index < self.count() as i32 {
            // The insertion point affects the realized elements. Update the
            // index of the elements after the insertion point.
            let element_count = self.count();
            let start = realized_index.max(0) as usize;

            for i in start..element_count {
                let Some(element) = self.element_at(i) else { continue };
                let old_index = i as i32 + first;
                update_element_index(&element, old_index, old_index + count);
            }

            if realized_index < 0 {
                // The insertion point was before the first element, update
                // the first index.
                self.first_index.set(self.first_index.get() + count);
                self.start_u_unstable.set(true);
            } else {
                // The insertion point was within the realized elements,
                // insert an empty space in the elements and sizes.
                CollectionUtils::insert_many(
                    &mut self.elements.borrow_mut(),
                    realized_index as usize,
                    None,
                    count as usize,
                );
                CollectionUtils::insert_many(
                    &mut self.sizes.borrow_mut(),
                    realized_index as usize,
                    f64::NAN,
                    count as usize,
                );
            }
        }
    }

    /// Updates the elements in response to items being removed from the
    /// source collection.
    ///
    /// `index` is the index in the source collection of the remove, `count`
    /// the number of items removed, `update_element_index` a method used to
    /// update the element indexes and `recycle_element` a method used to
    /// recycle elements.
    pub fn items_removed(
        &self,
        index: i32,
        count: i32,
        update_element_index: impl Fn(&Ref<Control>, i32, i32),
        recycle_element: impl Fn(&Ref<Control>),
    ) {
        if index < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'index')");
        }
        if self.count() == 0 {
            return;
        }

        // Get the removal start and end index within the realized elements
        // collection.
        let mut first = self.first_index();
        let last = self.last_index();
        let start_index = index - first;
        let end_index = (index + count) - first;

        if end_index < 0 {
            // The removed range was before the realized elements. Update the
            // first index and the indexes of the realized elements.
            self.first_index.set(self.first_index.get() - count);
            self.start_u_unstable.set(true);

            let mut new_index = self.first_index.get();
            let mut i = 0;
            while i < self.count() {
                if let Some(element) = self.element_at(i) {
                    update_element_index(&element, new_index + count, new_index);
                }
                new_index += 1;
                i += 1;
            }
        } else if start_index < self.count() as i32 {
            // Recycle and remove the affected elements.
            let start = start_index.max(0) as usize;
            let mut end = end_index.min(self.count() as i32) as usize;

            for i in start..end {
                if let Some(element) = self.take_at(i) {
                    recycle_element(&element);
                }
            }

            self.elements.borrow_mut().drain(start..end);
            self.sizes.borrow_mut().drain(start..end);

            // If the remove started before and ended within our realized
            // elements, then our new first index will be the index where the
            // remove started. Mark the start position as unstable because we
            // can't rely on it now to estimate element heights.
            if start_index <= 0 && (end as i32) < last {
                self.first_index.set(index);
                first = index;
                self.start_u_unstable.set(true);
            }

            // Update the indexes of the elements after the removed range.
            end = self.count();
            let mut new_index = first + start as i32;
            for i in start..end {
                if let Some(element) = self.element_at(i) {
                    update_element_index(&element, new_index + count, new_index);
                }
                new_index += 1;
            }
        }
    }

    /// Updates the elements in response to items being replaced in the
    /// source collection.
    ///
    /// `index` is the index in the source collection of the remove, `count`
    /// the number of items removed and `recycle_element` a method used to
    /// recycle elements.
    pub fn items_replaced(&self, index: i32, count: i32, recycle_element: impl Fn(&Ref<Control>)) {
        if index < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'index')");
        }
        if self.count() == 0 {
            return;
        }

        // Get the index within the realized elements collection.
        let start_index = index - self.first_index();
        let end_index = (start_index + count).min(self.count() as i32);

        if start_index >= 0 && end_index > start_index {
            for i in start_index as usize..end_index as usize {
                if let Some(element) = self.element_at(i) {
                    recycle_element(&element);
                    self.elements.borrow_mut()[i] = None;
                    self.sizes.borrow_mut()[i] = f64::NAN;
                }
            }
        }
    }

    /// Recycles all elements in response to the source collection being
    /// reset.
    ///
    /// `recycle_element` is a method used to recycle elements.
    pub fn items_reset(&self, recycle_element: impl Fn(&Ref<Control>)) {
        if self.count() == 0 {
            return;
        }

        let mut i = 0;
        while i < self.count() {
            if let Some(e) = self.take_at(i) {
                recycle_element(&e);
            }
            i += 1;
        }

        self.start_u.set(0.0);
        self.first_index.set(0);
        self.elements.borrow_mut().clear();
        self.sizes.borrow_mut().clear();
    }

    /// Recycles elements before a specific index.
    ///
    /// `index` is the index in the source collection of new first element
    /// and `recycle_element` a method used to recycle elements.
    pub fn recycle_elements_before(&self, index: i32, recycle_element: impl Fn(&Ref<Control>, i32)) {
        if index <= self.first_index() || self.count() == 0 {
            return;
        }

        if index > self.last_index() {
            self.recycle_all_elements(recycle_element);
        } else {
            let end_index = (index - self.first_index()) as usize;

            for i in 0..end_index {
                if let Some(e) = self.take_at(i) {
                    recycle_element(&e, i as i32 + self.first_index());
                }
            }

            self.elements.borrow_mut().drain(0..end_index);
            self.sizes.borrow_mut().drain(0..end_index);
            self.first_index.set(index);
        }
    }

    /// Recycles elements after a specific index.
    ///
    /// `index` is the index in the source collection of new last element and
    /// `recycle_element` a method used to recycle elements.
    pub fn recycle_elements_after(&self, index: i32, recycle_element: impl Fn(&Ref<Control>, i32)) {
        if index >= self.last_index() || self.count() == 0 {
            return;
        }

        if index < self.first_index() {
            self.recycle_all_elements(recycle_element);
        } else {
            let start_index = ((index + 1) - self.first_index()) as usize;
            let count = self.count();

            for i in start_index..count {
                if let Some(e) = self.take_at(i) {
                    recycle_element(&e, i as i32 + self.first_index());
                }
            }

            self.elements.borrow_mut().truncate(start_index);
            self.sizes.borrow_mut().truncate(start_index);
        }
    }

    /// Recycles all realized elements.
    ///
    /// `recycle_element` is a method used to recycle elements.
    pub fn recycle_all_elements(&self, recycle_element: impl Fn(&Ref<Control>, i32)) {
        if self.count() == 0 {
            return;
        }

        let mut i = 0;
        while i < self.count() {
            if let Some(e) = self.take_at(i) {
                recycle_element(&e, i as i32 + self.first_index());
            }
            i += 1;
        }

        self.start_u.set(0.0);
        self.first_index.set(0);
        self.elements.borrow_mut().clear();
        self.sizes.borrow_mut().clear();
    }

    /// Resets the element list and prepares it for reuse.
    pub fn reset_for_reuse(&self) {
        self.start_u.set(0.0);
        self.first_index.set(0);
        self.start_u_unstable.set(false);
        self.elements.borrow_mut().clear();
        self.sizes.borrow_mut().clear();
    }

    /// Validates that [`start_u`](Self::start_u) is still valid.
    ///
    /// `orientation` is the panel orientation.
    ///
    /// If the U size of any element in the realized elements has changed,
    /// then the value of [`start_u`](Self::start_u) should be considered
    /// unstable.
    pub fn validate_start_u(&self, orientation: Orientation) {
        if self.start_u_unstable.get() {
            return;
        }

        let elements = self.elements.borrow();
        let sizes = self.sizes.borrow();

        for (i, element) in elements.iter().enumerate() {
            let Some(element) = element else { continue };

            let size_u = if orientation == Orientation::Horizontal {
                element.desired_size().width
            } else {
                element.desired_size().height
            };

            if size_u != sizes[i] {
                self.start_u_unstable.set(true);
                break;
            }
        }
    }

    /// Removes and returns the element at the specified position of the
    /// realized list, leaving an empty slot.
    #[inline]
    fn take_at(&self, i: usize) -> Option<Ref<Control>> {
        self.elements.borrow_mut().get_mut(i).and_then(Option::take)
    }
}
