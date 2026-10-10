//! Three slices read as one: the text before the marked text, the marked
//! text and the text after it, without building their concatenation.

/// Three slices that are read as one sequence.
#[derive(Clone, Copy, Debug)]
pub struct CombinedSpan3<'a, T> {
    pub span1: &'a [T],
    pub span2: &'a [T],
    pub span3: &'a [T],
}

impl<'a, T: Copy> CombinedSpan3<'a, T> {
    /// Combines the three slices.
    pub fn new(span1: &'a [T], span2: &'a [T], span3: &'a [T]) -> Self {
        Self { span1, span2, span3 }
    }

    /// The number of elements of the three slices.
    pub fn length(&self) -> usize {
        self.span1.len() + self.span2.len() + self.span3.len()
    }

    fn copy_from_span(from: &[T], offset: &mut usize, to: &mut &mut [T]) {
        if to.is_empty() {
            return;
        }
        if *offset < from.len() {
            let copy_now = (from.len() - *offset).min(to.len());
            let (head, tail) = std::mem::take(to).split_at_mut(copy_now);
            head.copy_from_slice(&from[*offset..*offset + copy_now]);
            *to = tail;
            *offset = 0;
        } else {
            *offset -= from.len();
        }
    }

    /// Copies the elements from `offset` on into `to`, as many as it
    /// takes or as there are.
    pub fn copy_to(&self, to: &mut [T], offset: usize) {
        let mut offset = offset;
        let mut to = to;
        Self::copy_from_span(self.span1, &mut offset, &mut to);
        Self::copy_from_span(self.span2, &mut offset, &mut to);
        Self::copy_from_span(self.span3, &mut offset, &mut to);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    fn read(span: &CombinedSpan3<'_, u8>, offset: usize, length: usize) -> Vec<u8> {
        let mut buffer = vec![0; length];
        span.copy_to(&mut buffer, offset);
        buffer
    }

    #[test]
    fn the_three_slices_are_read_as_one() {
        let span = CombinedSpan3::new(b"abc", b"de", b"fgh");
        assert_eq!(8, span.length());
        assert_eq!(b"abcdefgh".to_vec(), read(&span, 0, 8));
        assert_eq!(b"cdef".to_vec(), read(&span, 2, 4));
        assert_eq!(b"de".to_vec(), read(&span, 3, 2));
        assert_eq!(b"h".to_vec(), read(&span, 7, 1));
        assert_eq!(Vec::<u8>::new(), read(&span, 4, 0));
    }

    #[test]
    fn what_is_past_the_end_stays_as_it_was() {
        let span = CombinedSpan3::new(b"ab", b"", b"c");
        assert_eq!(vec![b'b', b'c', 0, 0], read(&span, 1, 4));
        assert_eq!(vec![0, 0], read(&span, 3, 2));
        assert_eq!(vec![0], read(&span, 17, 1));
    }

    #[test]
    fn empty_slices_are_passed_over() {
        let span = CombinedSpan3::new(b"", b"xy", b"");
        assert_eq!(2, span.length());
        assert_eq!(b"xy".to_vec(), read(&span, 0, 2));
        assert_eq!(b"y".to_vec(), read(&span, 1, 1));
    }
}
