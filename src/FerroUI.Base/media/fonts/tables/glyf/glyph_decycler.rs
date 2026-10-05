use std::cell::RefCell;
use std::ops::Deref;

use crate::media::fonts::tables::decycler::Decycler;

/// The number of idle instances the pool keeps.
const POOL_MAX_SIZE: usize = 16;

thread_local! {
    static POOL: RefCell<Vec<GlyphDecycler>> = const { RefCell::new(Vec::new()) };
}

/// The cycle guard used while building composite glyph outlines.
pub struct GlyphDecycler {
    base: Decycler<i32>,
}

impl GlyphDecycler {
    pub const MAX_TRAVERSAL_DEPTH: i32 = 64;

    pub fn new() -> Self {
        Self { base: Decycler::new(Self::MAX_TRAVERSAL_DEPTH) }
    }

    /// Takes an instance from the pool, creating one when the pool is empty.
    pub fn rent() -> GlyphDecycler {
        POOL.with(|pool| pool.borrow_mut().pop()).unwrap_or_default()
    }

    /// Resets the instance and gives it back to the pool (dropped when the
    /// pool is full).
    pub fn return_to_pool(decycler: GlyphDecycler) {
        decycler.reset();

        POOL.with(|pool| {
            let mut pool = pool.borrow_mut();

            if pool.len() < POOL_MAX_SIZE {
                pool.push(decycler);
            }
        });
    }
}

impl Default for GlyphDecycler {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for GlyphDecycler {
    type Target = Decycler<i32>;

    fn deref(&self) -> &Decycler<i32> {
        &self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rented_instances_are_reset() {
        let decycler = GlyphDecycler::rent();

        assert_eq!(decycler.max_depth(), GlyphDecycler::MAX_TRAVERSAL_DEPTH);

        // An abandoned traversal leaves a visited id behind.
        std::mem::forget(decycler.enter(7).unwrap());
        assert_eq!(decycler.current_depth(), 1);

        GlyphDecycler::return_to_pool(decycler);

        let decycler = GlyphDecycler::rent();

        assert_eq!(decycler.current_depth(), 0);
        assert!(decycler.enter(7).is_ok());

        GlyphDecycler::return_to_pool(decycler);
    }
}
