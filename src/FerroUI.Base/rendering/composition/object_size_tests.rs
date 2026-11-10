//! The sizes of the two objects every element of a tree has in the compositor.
//!
//! A tree of a few hundred thousand elements has as many visuals on the UI
//! thread and as many on the server, each in a block of its own, and an
//! allocator hands out blocks of fixed sizes: above a kilobyte the next sizes
//! are a quarter apart, so one field too many costs every visual a quarter of
//! its size. The sizes below are the ones the visuals fit in now
//! (desktop-performance.md, section 9.39); a field that is added to one of
//! them belongs in a box, behind an `Option`, if most visuals do not use it.

use super::server::ServerCompositionVisual;
use super::visual::CompositionVisual;
use std::mem::size_of;

/// What an `Rc` keeps in front of its value: the two counts.
const RC_HEADER: usize = 2 * size_of::<usize>();

#[test]
#[cfg(target_pointer_width = "64")]
fn a_visual_of_the_ui_thread_fits_a_block_of_896_bytes() {
    assert!(
        size_of::<CompositionVisual>() + RC_HEADER <= 896,
        "CompositionVisual is {} bytes",
        size_of::<CompositionVisual>()
    );
}

#[test]
#[cfg(target_pointer_width = "64")]
fn a_visual_of_the_server_fits_a_block_of_1024_bytes() {
    assert!(
        size_of::<ServerCompositionVisual>() + RC_HEADER <= 1024,
        "ServerCompositionVisual is {} bytes",
        size_of::<ServerCompositionVisual>()
    );
}
