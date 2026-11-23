//! Not from upstream: the sizes of the elements, held against the blocks of the allocator.
//!
//! An element is kept in the block its size falls into: above 1,008 bytes the allocator of macOS
//! has four block sizes for each doubling (1,024, 1,280, 1,536, 1,792, 2,048), and the elements of
//! a tree are most of what a start leaves on the heap. Sections 9.65 and 9.67 to 9.71 of
//! `docs/porting/desktop-performance.md` took the common elements into blocks of 1,280 bytes. A
//! field added to a layer or to one of these classes that takes its class over the block fails
//! here: the field belongs behind a pointer if few elements use it (a `OnceCell` of a box, as the
//! rare state of `Control`), or the limit is raised with the bytes it costs every element in mind.

use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{Thumb, ToggleButton, Track};
use crate::shapes::{Path, Rectangle};
use crate::{
    Border, Button, Canvas, CheckBox, ContentControl, Control, Decorator, Grid, Panel, StackPanel, TextBlock,
};
use ferroui_base::input::InputElement;
use std::mem::size_of;

/// The two counts an element is allocated with.
const COUNTS: usize = 16;

/// The bytes an element of `T` asks the allocator for.
fn block_of<T>() -> usize {
    size_of::<T>() + COUNTS
}

#[test]
#[cfg(target_pointer_width = "64")]
fn the_common_elements_fit_their_blocks() {
    // A build with fields of its own in the layers (hot reload, tagged weak references) is not the
    // one that ships: its sizes are not held.
    if size_of::<ferroui_base::FerroObject>() > 248 {
        return;
    }

    // The layers below a control end at the first block size.
    assert!(block_of::<InputElement>() <= 1_024, "InputElement: {}", size_of::<InputElement>());

    let in_1280: [(&str, usize); 16] = [
        ("Control", block_of::<Control>()),
        ("Decorator", block_of::<Decorator>()),
        ("Panel", block_of::<Panel>()),
        ("StackPanel", block_of::<StackPanel>()),
        ("Canvas", block_of::<Canvas>()),
        ("Grid", block_of::<Grid>()),
        ("Border", block_of::<Border>()),
        ("TextBlock", block_of::<TextBlock>()),
        ("ContentPresenter", block_of::<ContentPresenter>()),
        ("ItemsPresenter", block_of::<ItemsPresenter>()),
        ("ContentControl", block_of::<ContentControl>()),
        ("Button", block_of::<Button>()),
        ("ToggleButton", block_of::<ToggleButton>()),
        ("CheckBox", block_of::<CheckBox>()),
        ("Rectangle", block_of::<Rectangle>()),
        ("Path", block_of::<Path>()),
    ];
    for (name, block) in in_1280 {
        assert!(block <= 1_280, "{name} asks for {block} bytes, more than a block of 1,280");
    }

    let in_1280_parts: [(&str, usize); 2] = [("Thumb", block_of::<Thumb>()), ("Track", block_of::<Track>())];
    for (name, block) in in_1280_parts {
        assert!(block <= 1_280, "{name} asks for {block} bytes, more than a block of 1,280");
    }
}
