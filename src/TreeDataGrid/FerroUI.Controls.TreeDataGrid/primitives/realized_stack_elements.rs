//! `Primitives/RealizedStackElements.cs`: the realized element state of a
//! [`TreeDataGridPresenterBase`](super::TreeDataGridPresenterBase).
//!
//! This is the control library's own variant of the framework's realized stack elements: the start
//! position is reported even when it is unstable (the stability is checked by the callers through
//! [`try_get_stable_range`](RealizedStackElements::try_get_stable_range) and
//! [`get_element_u`](RealizedStackElements::get_element_u)), the anchor element and the positions of
//! unrealized elements are estimated here, insertions and removals keep the controls whose items only
//! changed index, and moves are handled without recycling the controls that stay in the realized
//! window.

use ferroui_base::utilities::MathUtilities;
use ferroui_base::Ref;
use ferroui_controls::Control;
use std::cell::{Cell, Ref as CellRef, RefCell};

/// Stores the realized element state for a presenter of the control.
///
/// The state is interior-mutable: the methods that call back into the presenter (to recycle an
/// element or to update its index) never hold a borrow of the element list across the callback.
pub struct RealizedStackElements {
    trace_enabled: bool,
    first_index: Cell<i32>,
    elements: RefCell<Vec<Option<Ref<Control>>>>,
    sizes: RefCell<Vec<f64>>,
    start_u: Cell<f64>,
    start_u_unstable: Cell<bool>,
}

impl RealizedStackElements {
    pub fn new(trace_enabled: bool) -> Self {
        Self {
            trace_enabled,
            first_index: Cell::new(0),
            elements: RefCell::new(Vec::new()),
            sizes: RefCell::new(Vec::new()),
            start_u: Cell::new(0.0),
            start_u_unstable: Cell::new(false),
        }
    }

    /// Writes a diagnostic trace line (`Trace`, in debug builds only).
    fn trace(&self, message: impl FnOnce() -> String) {
        if cfg!(debug_assertions) && self.trace_enabled && crate::TreeDataGridDiagnostics::enable_tracing() {
            eprintln!("[RealizedStackElements] {}", message());
        }
    }

    /// Gets the number of realized elements.
    #[inline]
    pub fn count(&self) -> usize {
        self.elements.borrow().len()
    }

    /// Gets the index of the first realized element, or -1 if no elements are realized.
    #[inline]
    pub fn first_index(&self) -> i32 {
        if self.count() > 0 {
            self.first_index.get()
        } else {
            -1
        }
    }

    /// Gets the index of the last realized element, or -1 if no elements are realized.
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
    /// The returned borrow must not be held across a call that can run code outside the
    /// presenter; use [`element_at`](Self::element_at) for that.
    #[inline]
    pub fn elements(&self) -> CellRef<'_, Vec<Option<Ref<Control>>>> {
        self.elements.borrow()
    }

    /// A copy of the elements.
    pub fn elements_snapshot(&self) -> Vec<Option<Ref<Control>>> {
        self.elements.borrow().clone()
    }

    /// Gets the sizes of the elements on the primary axis.
    #[inline]
    pub fn size_u(&self) -> CellRef<'_, Vec<f64>> {
        self.sizes.borrow()
    }

    /// Gets the element at the specified position of the realized list (not of the source
    /// collection), if there is one.
    #[inline]
    pub fn element_at(&self, i: usize) -> Option<Ref<Control>> {
        self.elements.borrow().get(i).cloned().flatten()
    }

    /// Gets the size on the primary axis of the element at the specified position of the realized
    /// list (not of the source collection).
    #[inline]
    pub fn size_u_at(&self, i: usize) -> f64 {
        self.sizes.borrow()[i]
    }

    /// Gets the position of the first element on the primary axis.
    #[inline]
    pub fn start_u(&self) -> f64 {
        self.start_u.get()
    }

    /// Gets the range covered by contiguous, measured realized elements, as `(start, end)`.
    pub fn try_get_stable_range(&self) -> Option<(f64, f64)> {
        let elements = self.elements.borrow();
        let sizes = self.sizes.borrow();
        let start_u = self.start_u.get();

        if self.start_u_unstable.get() || elements.is_empty() || elements.len() != sizes.len() || !start_u.is_finite()
        {
            return None;
        }

        let mut end = start_u;

        for (element, size) in elements.iter().zip(sizes.iter()) {
            if element.is_none() || !size.is_finite() || *size < 0.0 {
                return None;
            }

            end += *size;
        }

        if end.is_finite() {
            Some((start_u, end))
        } else {
            None
        }
    }

    /// Adds a newly realized element to the collection.
    ///
    /// `index` is the index of the element, `u` the position of the element on the primary axis and
    /// `size_u` the size of the element on the primary axis.
    pub fn add(&self, index: i32, element: Ref<Control>, u: f64, size_u: f64) {
        if index < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'index')");
        }

        self.trace(|| format!("Add: index={index}, u={u:.1}, sizeU={size_u:.1}, currentCount={}", self.count()));

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

    /// Gets or estimates the index and start U position of the anchor element for the specified
    /// viewport.
    ///
    /// Returns the index of the anchor element, or -1 if an anchor could not be determined, and the
    /// U position of the start of the anchor element, if determined.
    ///
    /// This method tries to find an existing element in the specified viewport from which element
    /// realization can start. Failing that it estimates the first element in the viewport.
    pub fn get_or_estimate_anchor_element_for_viewport(
        &self,
        viewport_start_u: f64,
        viewport_end_u: f64,
        item_count: i32,
        estimated_element_size_u: &Cell<f64>,
    ) -> (i32, f64) {
        // We have no elements, nothing to do here.
        if item_count <= 0 {
            return (-1, 0.0);
        }

        // If we're at 0 then display the first item.
        if MathUtilities::is_zero(viewport_start_u) {
            return (0, 0.0);
        }

        if !self.start_u_unstable.get() {
            let sizes = self.sizes.borrow();
            let mut u = self.start_u.get();

            for (i, size) in sizes.iter().enumerate() {
                if size.is_nan() {
                    break;
                }

                let end_u = u + *size;

                if end_u > viewport_start_u && u < viewport_end_u {
                    return (self.first_index() + i as i32, u);
                }

                u = end_u;
            }
        }

        // We don't have any realized elements in the requested viewport, or can't rely on StartU
        // being valid. Estimate the index using only the estimated size. First, estimate the
        // element size, using the default element size if we don't have any realized elements.
        let estimated_size = match self.estimate_element_size_u() {
            v if v == -1.0 => estimated_element_size_u.get(),
            v => v,
        };

        // Store the estimated size for the next layout pass.
        estimated_element_size_u.set(estimated_size);

        // Estimate the element at the start of the viewport.
        let index = ((viewport_start_u / estimated_size) as i32).min(item_count - 1);
        (index, index as f64 * estimated_size)
    }

    /// Gets the position of the element with the requested index on the primary axis, if realized.
    ///
    /// Returns the position of the element, or NaN if the element is not realized.
    pub fn get_element_u(&self, index: i32) -> f64 {
        if self.start_u_unstable.get() || index < self.first_index() {
            return f64::NAN;
        }

        let end_index = (index - self.first_index()) as usize;
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

    /// Gets the position of the element with the requested index on the primary axis, estimating
    /// it from the realized range when the element is not realized.
    pub fn get_or_estimate_element_u(&self, index: i32, estimated_element_size_u: &Cell<f64>) -> f64 {
        // Return the position of the existing element if realized.
        let u = self.get_element_u(index);

        if !u.is_nan() {
            return u;
        }

        // Estimate the element size, using the default element size if we don't have any realized
        // elements.
        let estimated_size = match self.estimate_element_size_u() {
            v if v == -1.0 => estimated_element_size_u.get(),
            v => v,
        };

        // Store the estimated size for the next layout pass.
        estimated_element_size_u.set(estimated_size);

        // Anchor estimates to the realized range when its position is stable. Multiplying the
        // requested index by the current average loses all measured-size information before the
        // range and can place variable-sized rows hundreds of pixels away from their actual
        // neighborhood.
        if !self.start_u_unstable.get() && self.sizes.borrow().len() > 0 {
            let first_index = self.first_index();
            let last_index = self.last_index();

            if index < first_index {
                return self.start_u.get() - ((first_index - index) as f64 * estimated_size);
            }

            if index > last_index {
                let mut realized_end_u = self.start_u.get();

                for size in self.sizes.borrow().iter() {
                    if size.is_nan() {
                        return index as f64 * estimated_size;
                    }

                    realized_end_u += *size;
                }

                return realized_end_u + ((index - last_index - 1) as f64 * estimated_size);
            }
        }

        index as f64 * estimated_size
    }

    /// Estimates the average U size of all elements in the source collection based on the realized
    /// elements.
    ///
    /// Returns the estimated U size of an element, or -1 if not enough information is present to
    /// make an estimate.
    pub fn estimate_element_size_u(&self) -> f64 {
        let mut total = 0.0;
        let mut divisor = 0.0;

        // Average the size of the realized elements.
        for size in self.sizes.borrow().iter() {
            if size.is_nan() {
                continue;
            }
            total += *size;
            divisor += 1.0;
        }

        // We don't have any elements on which to base our estimate.
        if divisor == 0.0 || total == 0.0 {
            return -1.0;
        }

        total / divisor
    }

    /// Gets the index of the specified element.
    ///
    /// Returns the index or -1 if the element is not present in the collection.
    pub fn get_index(&self, element: &Ref<Control>) -> i32 {
        let elements = self.elements.borrow();
        match elements.iter().position(|e| e.as_ref() == Some(element)) {
            Some(index) => index as i32 + self.first_index.get(),
            None => -1,
        }
    }

    /// Updates the elements in response to items being inserted into the source collection.
    ///
    /// `index` is the index in the source collection of the insert, `count` the number of items
    /// inserted, `update_element_index` a method used to update the element indexes and
    /// `recycle_element` a method used to recycle elements.
    pub fn items_inserted(
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

        // Get the index within the realized elements collection.
        let first = self.first_index();
        let realized_index = index - first;

        if realized_index < self.count() as i32 {
            if realized_index < 0 {
                // The insertion point was before the first element, update the first index. The
                // data at each realized index is still correct (same items, just at different
                // indices).
                self.first_index.set(self.first_index.get() + count);
                self.start_u_unstable.set(true);

                let mut new_index = self.first_index.get();
                let mut i = 0;
                while i < self.count() {
                    if let Some(element) = self.element_at(i) {
                        update_element_index(&element, new_index - count, new_index);
                    }
                    new_index += 1;
                    i += 1;
                }
            } else {
                let realized_index = realized_index as usize;
                let suffix_count = self.count() - realized_index;

                if (count as usize) < suffix_count {
                    // Preserve existing elements that can remain in the current realization
                    // window. They still represent the same items, only at shifted indexes.
                    self.elements
                        .borrow_mut()
                        .splice(realized_index..realized_index, std::iter::repeat(None).take(count as usize));
                    self.sizes
                        .borrow_mut()
                        .splice(realized_index..realized_index, std::iter::repeat(f64::NAN).take(count as usize));

                    let mut i = realized_index + count as usize;
                    while i < self.count() {
                        if let Some(element) = self.element_at(i) {
                            let old_index = first + i as i32 - count;
                            update_element_index(&element, old_index, old_index + count);
                        }
                        i += 1;
                    }
                } else {
                    // A large insertion moves the whole suffix outside the current window; keeping
                    // a sparse range would cost more than realizing it on demand.
                    let mut i = realized_index;
                    while i < self.count() {
                        if let Some(element) = self.element_at(i) {
                            recycle_element(&element);
                        }
                        i += 1;
                    }

                    self.elements.borrow_mut().truncate(realized_index);
                    self.sizes.borrow_mut().truncate(realized_index);
                }
            }
        }
    }

    /// Updates the elements in response to items being removed from the source collection.
    ///
    /// `index` is the index in the source collection of the remove, `count` the number of items
    /// removed, `update_element_index` a method used to update the element indexes and
    /// `recycle_element` a method used to recycle elements.
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

        // Get the removal start and end index within the realized elements collection.
        let first = self.first_index();
        let start_index = index - first;
        let end_index = (index + count) - first;

        if end_index <= 0 {
            // The removed range was before the realized elements. Update the first index. No need
            // to recycle - the data at each index is still correct.
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
            // Recycle only controls whose items were removed. Controls after the removed range
            // still represent the same items and can be retained at their new indexes.
            let remove_start = start_index.max(0) as usize;
            let remove_end = end_index.min(self.count() as i32).max(0) as usize;
            let remove_count = remove_end.saturating_sub(remove_start);

            for i in remove_start..remove_end {
                if let Some(element) = self.take_at(i) {
                    recycle_element(&element);
                }
            }

            if remove_count > 0 {
                self.elements.borrow_mut().drain(remove_start..remove_end);
                self.sizes.borrow_mut().drain(remove_start..remove_end);
            }

            if start_index < 0 {
                self.first_index.set(first + remove_count as i32 - count);
                self.start_u_unstable.set(true);
            }

            // Elements following the removed range keep their data and move left by count.
            let mut i = remove_start;
            while i < self.count() {
                if let Some(element) = self.element_at(i) {
                    let old_index = first + i as i32 + remove_count as i32;
                    update_element_index(&element, old_index, old_index - count);
                }
                i += 1;
            }
        }
    }

    /// Updates the elements in response to items being moved within the source collection.
    ///
    /// `old_index` is the old index in the source collection, `new_index` the new index, `count`
    /// the number of items moved, `update_element_index` a method used to update the element
    /// indexes and `recycle_element` a method used to recycle elements.
    pub fn items_moved(
        &self,
        old_index: i32,
        new_index: i32,
        count: i32,
        update_element_index: impl Fn(&Ref<Control>, i32, i32),
        recycle_element: impl Fn(&Ref<Control>),
    ) {
        if old_index < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'oldIndex')");
        }
        if new_index < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'newIndex')");
        }
        if self.count() == 0 {
            return;
        }

        if old_index == new_index {
            return;
        }

        let first = self.first_index();
        let last = self.last_index();
        let affected_start = old_index.min(new_index);
        let affected_end = old_index.max(new_index) + count - 1;

        if affected_end < first || affected_start > last {
            return;
        }

        let realized_count = self.count();
        let old_elements: Vec<Option<Ref<Control>>> = std::mem::replace(
            &mut *self.elements.borrow_mut(),
            vec![None; realized_count],
        );
        let old_sizes: Vec<f64> = std::mem::replace(&mut *self.sizes.borrow_mut(), vec![f64::NAN; realized_count]);

        for (i, element) in old_elements.into_iter().enumerate() {
            let old_element_index = first + i as i32;
            let new_element_index = Self::map_moved_index(old_element_index, old_index, new_index, count);

            if new_element_index >= first && new_element_index <= last {
                let realized_index = (new_element_index - first) as usize;
                self.elements.borrow_mut()[realized_index] = element.clone();
                self.sizes.borrow_mut()[realized_index] = old_sizes[i];

                if let Some(element) = element.filter(|_| new_element_index != old_element_index) {
                    update_element_index(&element, old_element_index, new_element_index);
                }
            } else if let Some(element) = element {
                recycle_element(&element);
            }
        }

        // A move whose affected range starts at or after the realized window does not change the
        // accumulated size before FirstIndex, so the realized start remains exact.
        if affected_start < first {
            self.start_u_unstable.set(true);
        }
    }

    fn map_moved_index(index: i32, old_index: i32, new_index: i32, count: i32) -> i32 {
        if index >= old_index && index < old_index + count {
            return new_index + index - old_index;
        }
        if old_index < new_index && index >= old_index + count && index < new_index + count {
            return index - count;
        }
        if old_index > new_index && index >= new_index && index < old_index {
            return index + count;
        }
        index
    }

    /// Updates the elements in response to items being replaced in the source collection.
    ///
    /// `index` is the index in the source collection of the remove, `count` the number of items
    /// removed and `recycle_element` a method used to recycle elements.
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

    /// Recycles all elements in response to the source collection being reset.
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
    /// `index` is the index in the source collection of new first element and `recycle_element` a
    /// method used to recycle elements.
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
    /// `index` is the index in the source collection of new last element and `recycle_element` a
    /// method used to recycle elements.
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

        self.trace(|| format!("RecycleAllElements: recycling {} elements (FirstIndex={})", self.count(), self.first_index()));

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

    /// Removes and returns the element at the specified position of the realized list, leaving an
    /// empty slot.
    #[inline]
    fn take_at(&self, i: usize) -> Option<Ref<Control>> {
        self.elements.borrow_mut().get_mut(i).and_then(Option::take)
    }
}
