// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
// FerroUI 0.3.0's flags descriptor emits result = result | value. Confine
// its lint allowance to this enum/descriptor module; handwritten logic is absent.
#![allow(clippy::assign_op_pattern)]
bitflags::bitflags! {
    /// Defines resize edges for MDI windows.
    #[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Hash)]
    pub struct MdiResizeDirection:i32 {
        /// No resize edge.
        const NONE=0;
        /// Resize the left edge.
        const LEFT=1;
        /// Resize the top edge.
        const TOP=2;
        /// Resize the right edge.
        const RIGHT=4;
        /// Resize the bottom edge.
        const BOTTOM=8;
    }
}
ferroui_base::ferro_markup_enum!(flags MdiResizeDirection {None=MdiResizeDirection::NONE,Left=MdiResizeDirection::LEFT,Top=MdiResizeDirection::TOP,Right=MdiResizeDirection::RIGHT,Bottom=MdiResizeDirection::BOTTOM});
