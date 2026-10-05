//! Generic span types: a vector of spans (an element and the number of
//! character positions it covers), a position in it and a rider that
//! navigates it by character index.

use std::rc::Rc;

/// An element and the number of character positions it covers.
#[derive(Clone)]
pub(crate) struct Span<T> {
    pub element: Option<T>,
    pub length: i32,
}

impl<T> Span<T> {
    pub fn new(element: Option<T>, length: i32) -> Self {
        Self { element, length }
    }
}

/// Equality check method.
type Equals<T> = fn(Option<&T>, Option<&T>) -> bool;

fn value_equals<T: PartialEq>(first: Option<&T>, second: Option<&T>) -> bool {
    first == second
}

fn reference_equals<T: ?Sized>(first: Option<&Rc<T>>, second: Option<&Rc<T>>) -> bool {
    match (first, second) {
        (None, None) => true,
        (Some(first), Some(second)) => Rc::ptr_eq(first, second),
        _ => false,
    }
}

/// VECTOR: A series of spans.
pub(crate) struct SpanVector<T> {
    default: Option<T>,
    spans: Vec<Span<T>>,
}

#[allow(dead_code)] // upstream members that the formatted text (the only user so far) does not call
impl<T: Clone> SpanVector<T> {
    pub fn new(default_object: Option<T>) -> Self {
        Self::with_spans(default_object, Vec::new())
    }

    pub fn with_spans(default_object: Option<T>, spans: Vec<Span<T>>) -> Self {
        Self { default: default_object, spans }
    }

    /// Get enumerator to vector.
    pub fn get_enumerator(&self) -> SpanEnumerator<'_, T> {
        SpanEnumerator::new(self)
    }

    /// Add a new span to vector.
    fn add(&mut self, span: Span<T>) {
        self.spans.push(span);
    }

    /// Delete n elements of vector. Returns the latest position to continue with.
    pub fn delete(&mut self, index: i32, count: i32, latest_position: SpanPosition) -> SpanPosition {
        self.delete_internal(index, count);

        if index <= latest_position.index() {
            SpanPosition::default()
        } else {
            latest_position
        }
    }

    fn delete_internal(&mut self, index: i32, count: i32) {
        // Do removes highest index to lowest to minimize the number
        // of array entries copied.
        let mut i = index + count - 1;

        while i >= index {
            self.spans.remove(i as usize);
            i -= 1;
        }
    }

    /// Insert n elements to vector.
    fn insert(&mut self, index: i32, count: i32) {
        for _ in 0..count {
            self.spans.insert(index as usize, Span::new(None, 0));
        }
    }

    /// Finds the span that contains the specified character position.
    ///
    /// * `cp` — position to find
    /// * `latest_position` — position of the most recently accessed span (e.g., the current span
    ///   of a `SpanRider`) for performance; `find_span` runs in O(1) time if the specified cp is
    ///   in the same span or an adjacent span.
    ///
    /// Returns whether cp is in range and the index and first cp of the span
    /// that contains the specified position or, if the position is past the
    /// end of the vector, the index and cp just past the end of the last span.
    pub fn find_span(&self, cp: i32, latest_position: SpanPosition) -> (bool, SpanPosition) {
        debug_assert!(cp >= 0);

        let span_count = self.spans.len() as i32;
        let mut span_index;
        let mut span_cp;

        if cp == 0 {
            // CP zero always corresponds to span index zero
            span_index = 0;
            span_cp = 0;
        } else if cp >= latest_position.offset() || cp * 2 < latest_position.offset() {
            // One of the following is true:
            //  1.  cp is after the latest position (the most recently accessed span)
            //  2.  cp is closer to zero than to the latest position
            if cp >= latest_position.offset() {
                // case 1: scan forward from the latest position
                span_index = latest_position.index();
                span_cp = latest_position.offset();
            } else {
                // case 2: scan forward from the start of the span vector
                span_index = 0;
                span_cp = 0;
            }

            // Scan forward until we find the Span that contains the specified CP or
            // reach the end of the SpanVector
            while span_index < span_count {
                let span_length = self.spans[span_index as usize].length;

                if cp < span_cp + span_length {
                    break;
                }

                span_cp += span_length;
                span_index += 1;
            }
        } else {
            // The specified CP is before the latest position but closer to it than to zero;
            // therefore scan backwards from the latest position
            span_index = latest_position.index();
            span_cp = latest_position.offset();

            while span_cp > cp {
                debug_assert!(span_index > 0);
                span_index -= 1;
                span_cp -= self.spans[span_index as usize].length;
            }
        }

        // Return index and cp of span, and true if the span is in range.
        (span_index != span_count, SpanPosition::new(span_index, span_cp))
    }

    fn set(
        &mut self,
        mut first: i32,
        mut length: i32,
        element: Option<T>,
        equals: Equals<T>,
        span_position: SpanPosition,
    ) -> SpanPosition {
        let (in_range, span_position) = self.find_span(first, span_position);

        // fs = index of first span partly or completely updated
        // fc = character index at start of fs
        let mut fs = span_position.index();
        let mut fc = span_position.offset();

        // Find the span that contains the first affected cp
        if !in_range {
            // The first cp is past the end of the last span
            if fc < first {
                // Create default run up to first
                self.add(Span::new(self.default.clone(), first - fc));
            }

            if self.count() > 0 && equals(self.spans[self.count() as usize - 1].element.as_ref(), element.as_ref()) {
                // New Element matches end Element, just extend end Element
                let last = self.count() as usize - 1;

                self.spans[last].length += length;

                // Make sure fs and fc still agree
                if fs == self.count() {
                    fc += length;
                }
            } else {
                self.add(Span::new(element, length));
            }
        } else {
            // Now find the last span affected by the update

            let mut ls = fs;
            let mut lc = fc;

            while ls < self.count() && lc + self.spans[ls as usize].length <= first + length {
                lc += self.spans[ls as usize].length;
                ls += 1;
            }
            // ls = first span following update to remain unchanged in part or in whole
            // lc = character index at start of ls

            // expand update region backwards to include existing Spans of identical
            // Element type

            if first == fc {
                // Item at [fs] is completely replaced. Check prior item

                if fs > 0 && equals(self.spans[fs as usize - 1].element.as_ref(), element.as_ref()) {
                    // Expand update area over previous run of equal classification
                    fs -= 1;
                    fc -= self.spans[fs as usize].length;
                    first = fc;
                    length += self.spans[fs as usize].length;
                }
            } else {
                // Item at [fs] is partially replaced. Check if it is same as update
                if equals(self.spans[fs as usize].element.as_ref(), element.as_ref()) {
                    // Expand update area back to start of first affected equal valued run
                    length = first + length - fc;
                    first = fc;
                }
            }

            // Expand update region forwards to include existing Spans of identical
            // Element type

            if ls < self.count() && equals(self.spans[ls as usize].element.as_ref(), element.as_ref()) {
                // Extend update region to end of existing split run

                length = lc + self.spans[ls as usize].length - first;
                lc += self.spans[ls as usize].length;
                ls += 1;
            }

            // If no old Spans remain beyond area affected by update, handle easily:

            if ls >= self.count() {
                // None of the old span list extended beyond the update region

                if fc < first {
                    // Updated region leaves some of [fs]

                    if self.count() != fs + 2 {
                        self.resize(fs + 2);
                    }

                    self.spans[fs as usize].length = first - fc;
                    self.spans[fs as usize + 1] = Span::new(element, length);
                } else {
                    // Updated item replaces [fs]
                    if self.count() != fs + 1 {
                        self.resize(fs + 1);
                    }

                    self.spans[fs as usize] = Span::new(element, length);
                }
            } else {
                // Record partial element type at end, if any

                let mut trailing_element: Option<T> = None;
                let mut trailing_length = 0;

                if first + length > lc {
                    trailing_element = self.spans[ls as usize].element.clone();
                    trailing_length = lc + self.spans[ls as usize].length - (first + length);
                }

                // Calculate change in number of Spans

                let span_delta = 1                      // The new span
                    + if first > fc { 1 } else { 0 }    // part span at start
                    - (ls - fs); // existing affected span count

                // Note part span at end doesn't affect the calculation - the run may need
                // updating, but it doesn't need creating.

                if span_delta < 0 {
                    self.delete_internal(fs + 1, -span_delta);
                } else if span_delta > 0 {
                    // The inserted spans are initialized (empty, without element).
                    self.insert(fs + 1, span_delta);
                }

                // Assign Element values

                // Correct Length of split span before updated range

                if fc < first {
                    self.spans[fs as usize].length = first - fc;
                    fs += 1;
                    fc = first;
                }

                // Record Element type for updated range

                self.spans[fs as usize] = Span::new(element, length);
                fs += 1;
                fc += length;

                // Correct Length of split span following updated range

                if lc < first + length {
                    self.spans[fs as usize] = Span::new(trailing_element, trailing_length);
                }
            }
        }

        // Return a known valid span position.
        SpanPosition::new(fs, fc)
    }

    /// Number of spans in vector.
    pub fn count(&self) -> i32 {
        self.spans.len() as i32
    }

    /// The default element of vector.
    pub fn default(&self) -> Option<&T> {
        self.default.as_ref()
    }

    /// Span accessor at nth element.
    pub fn get(&self, index: i32) -> &Span<T> {
        &self.spans[index as usize]
    }

    fn resize(&mut self, target_count: i32) {
        if target_count > self.count() {
            // Upstream re-reads the growing count in the loop condition and so
            // adds only half of the missing spans (rounded up); the callers
            // always grow by at most one span, where both agree.
            let mut c = 0;

            while c < target_count - self.count() {
                self.spans.push(Span::new(None, 0));
                c += 1;
            }
        } else if target_count < self.count() {
            self.delete_internal(target_count, self.count() - target_count);
        }
    }
}

#[allow(dead_code)] // upstream members that the formatted text (the only user so far) does not call
impl<T: Clone + PartialEq> SpanVector<T> {
    /// Set an element as a value to a character range.
    ///
    /// The element type has value equality (`PartialEq`).
    pub fn set_value(&mut self, first: i32, length: i32, element: T) {
        self.set(first, length, Some(element), value_equals::<T>, SpanPosition::default());
    }

    /// Set an element as a value to a character range; takes a `SpanPosition`
    /// of a recently accessed span for performance and returns a known valid
    /// `SpanPosition`.
    pub fn set_value_at(&mut self, first: i32, length: i32, element: T, span_position: SpanPosition) -> SpanPosition {
        self.set(first, length, Some(element), value_equals::<T>, span_position)
    }
}

#[allow(dead_code)] // upstream members that the formatted text (the only user so far) does not call
impl<T: ?Sized> SpanVector<Rc<T>> {
    /// Set an element as a reference to a character range.
    pub fn set_reference(&mut self, first: i32, length: i32, element: Rc<T>) {
        self.set(first, length, Some(element), reference_equals::<T>, SpanPosition::default());
    }

    /// Set an element as a reference to a character range; takes a
    /// `SpanPosition` of a recently accessed span for performance and returns
    /// a known valid `SpanPosition`.
    pub fn set_reference_at(
        &mut self,
        first: i32,
        length: i32,
        element: Rc<T>,
        span_position: SpanPosition,
    ) -> SpanPosition {
        self.set(first, length, Some(element), reference_equals::<T>, span_position)
    }
}

/// ENUMERATOR: To navigate a vector through its element.
#[allow(dead_code)] // upstream type nothing uses yet
pub(crate) struct SpanEnumerator<'a, T> {
    spans: &'a SpanVector<T>,
    current: i32, // current span
}

#[allow(dead_code)] // upstream type nothing uses yet
impl<'a, T: Clone> SpanEnumerator<'a, T> {
    pub(crate) fn new(spans: &'a SpanVector<T>) -> Self {
        Self { spans, current: -1 }
    }

    /// The current span.
    pub fn current(&self) -> &'a Span<T> {
        self.spans.get(self.current)
    }

    /// Move to the next span.
    pub fn move_next(&mut self) -> bool {
        self.current += 1;

        self.current < self.spans.count()
    }

    /// Reset the enumerator.
    pub fn reset(&mut self) {
        self.current = -1;
    }
}

/// Represents a Span's position as a pair of related values: its index in the
/// `SpanVector` its CP offset from the start of the `SpanVector`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpanPosition {
    span_index: i32,
    span_offset: i32,
}

impl SpanPosition {
    pub(crate) fn new(span_index: i32, span_offset: i32) -> Self {
        Self { span_index, span_offset }
    }

    pub(crate) fn index(&self) -> i32 {
        self.span_index
    }

    pub(crate) fn offset(&self) -> i32 {
        self.span_offset
    }
}

/// RIDER: To navigate a vector through character index.
pub(crate) struct SpanRider<'a, T> {
    spans: &'a SpanVector<T>,    // vector of spans
    span_position: SpanPosition, // index and cp of current span
    length: i32,
    current_position: i32,
}

#[allow(dead_code)] // upstream members that the formatted text (the only user so far) does not call
impl<'a, T: Clone> SpanRider<'a, T> {
    /// A rider at the cp of the latest position.
    pub fn at_latest_position(spans: &'a SpanVector<T>, latest_position: SpanPosition) -> Self {
        Self::new(spans, latest_position, latest_position.offset())
    }

    /// A rider at the given cp (upstream defaults: the default position and cp 0).
    pub fn new(spans: &'a SpanVector<T>, latest_position: SpanPosition, cp: i32) -> Self {
        let mut rider = Self { spans, span_position: SpanPosition::default(), current_position: 0, length: 0 };

        rider.at_from(latest_position, cp);

        rider
    }

    /// Move rider to a given cp.
    pub fn at(&mut self, cp: i32) -> bool {
        self.at_from(self.span_position, cp)
    }

    /// Move rider to a given cp, searching from the given position.
    pub fn at_from(&mut self, latest_position: SpanPosition, cp: i32) -> bool {
        let (in_range, span_position) = self.spans.find_span(cp, latest_position);

        self.span_position = span_position;

        if in_range {
            // cp is in range:
            //  - Length is the distance to the end of the span
            //  - CurrentPosition is cp
            self.length = self.spans.get(self.span_position.index()).length - (cp - self.span_position.offset());
            self.current_position = cp;
        } else {
            // cp is out of range:
            //  - Length is the default span length
            //  - CurrentPosition is the end of the last span
            self.length = i32::MAX;
            self.current_position = self.span_position.offset();
        }

        in_range
    }

    /// The first cp of the current span.
    pub fn current_span_start(&self) -> i32 {
        self.span_position.offset()
    }

    /// The length of current span start from the current cp.
    pub fn length(&self) -> i32 {
        self.length
    }

    /// The current position.
    pub fn current_position(&self) -> i32 {
        self.current_position
    }

    /// The element of the current span.
    pub fn current_element(&self) -> Option<&'a T> {
        if self.span_position.index() >= self.spans.count() {
            self.spans.default()
        } else {
            self.spans.get(self.span_position.index()).element.as_ref()
        }
    }

    /// Index of the span at the current position.
    pub fn current_span_index(&self) -> i32 {
        self.span_position.index()
    }

    /// Index and first cp of the current span.
    pub fn span_position(&self) -> SpanPosition {
        self.span_position
    }
}

#[cfg(test)]
mod tests {
    // Additions: upstream has no unit tests for the span types.
    use super::*;

    fn spans(vector: &SpanVector<char>) -> Vec<(Option<char>, i32)> {
        let mut result = Vec::new();
        let mut enumerator = vector.get_enumerator();

        while enumerator.move_next() {
            let span = enumerator.current();

            result.push((span.element, span.length));
        }

        result
    }

    #[test]
    fn set_value_splits_and_merges_spans() {
        let mut vector = SpanVector::<char>::new(None);

        let position = vector.set_value_at(0, 10, 'a', SpanPosition::default());

        assert_eq!(spans(&vector), [(Some('a'), 10)]);
        assert_eq!(position, SpanPosition::new(0, 0));

        // In the middle: three spans.
        let position = vector.set_value_at(3, 4, 'b', position);

        assert_eq!(spans(&vector), [(Some('a'), 3), (Some('b'), 4), (Some('a'), 3)]);
        assert_eq!(position, SpanPosition::new(2, 7));

        // Extending an equal neighbour merges.
        vector.set_value(7, 1, 'b');

        assert_eq!(spans(&vector), [(Some('a'), 3), (Some('b'), 5), (Some('a'), 2)]);

        // At the end.
        vector.set_value(8, 2, 'c');

        assert_eq!(spans(&vector), [(Some('a'), 3), (Some('b'), 5), (Some('c'), 2)]);

        // Covering everything collapses to one span.
        vector.set_value(0, 10, 'a');

        assert_eq!(spans(&vector), [(Some('a'), 10)]);

        // Replacing the start.
        vector.set_value(0, 2, 'd');

        assert_eq!(spans(&vector), [(Some('d'), 2), (Some('a'), 8)]);

        // Past the end: the default element fills the gap.
        vector.set_value(12, 2, 'e');

        assert_eq!(spans(&vector), [(Some('d'), 2), (Some('a'), 8), (None, 2), (Some('e'), 2)]);
    }

    #[test]
    fn rider_finds_spans_from_any_latest_position() {
        let mut vector = SpanVector::<char>::new(Some('z'));

        vector.set_value(0, 4, 'a');
        vector.set_value(4, 4, 'b');
        vector.set_value(8, 4, 'c');

        for latest in [SpanPosition::default(), SpanPosition::new(1, 4), SpanPosition::new(2, 8)] {
            let rider = SpanRider::new(&vector, latest, 5);

            assert_eq!(rider.current_element(), Some(&'b'));
            assert_eq!(rider.length(), 3);
            assert_eq!(rider.current_position(), 5);
            assert_eq!(rider.current_span_start(), 4);
            assert_eq!(rider.current_span_index(), 1);

            let rider = SpanRider::new(&vector, latest, 1);

            assert_eq!(rider.current_element(), Some(&'a'));
            assert_eq!(rider.length(), 3);
        }

        let mut rider = SpanRider::at_latest_position(&vector, SpanPosition::new(2, 8));

        assert_eq!(rider.current_element(), Some(&'c'));
        assert_eq!(rider.length(), 4);

        // Out of range: the default element.
        assert!(!rider.at(12));
        assert_eq!(rider.current_element(), Some(&'z'));
        assert_eq!(rider.length(), i32::MAX);
        assert_eq!(rider.current_position(), 12);
    }

    #[test]
    fn set_reference_compares_handles() {
        let mut vector = SpanVector::<Rc<str>>::new(None);

        let first: Rc<str> = Rc::from("x");
        let second: Rc<str> = Rc::from("x");

        vector.set_reference(0, 2, first.clone());
        vector.set_reference(2, 2, second);

        assert_eq!(vector.count(), 2);

        let position = vector.set_reference_at(2, 2, first, SpanPosition::default());

        assert_eq!(vector.count(), 1);
        assert_eq!(vector.get(0).length, 4);
        assert_eq!(position, SpanPosition::new(0, 0));

        assert_eq!(vector.delete(0, 1, position), SpanPosition::default());
        assert_eq!(vector.count(), 0);
    }
}
