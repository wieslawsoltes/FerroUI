// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use ferrodock_model::{
    collections::ReadOnlyListHandle,
    command_bars::{DockCommandBarDefinition, DockCommandBarKind},
};
use ferroui_base::Ref;
use ferroui_controls::Control;
use std::rc::Rc;
/// Converts original command bar definitions into native controls.
pub trait IDockCommandBarAdapter {
    /// Builds controls for the supplied kind and original read-only definitions.
    fn build_bars(
        &self,
        kind: DockCommandBarKind,
        definitions: &ReadOnlyListHandle<Rc<DockCommandBarDefinition>>,
    ) -> ReadOnlyListHandle<Ref<Control>>;
}
impl PartialEq for dyn IDockCommandBarAdapter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(interface dyn IDockCommandBarAdapter as "IDockCommandBarAdapter" {
    namespace:"FerroDock.Ferro.CommandBars",this:Rc<dyn IDockCommandBarAdapter>,
    methods:[fn BuildBars(DockCommandBarKind,ReadOnlyListHandle<Rc<DockCommandBarDefinition>>) -> ReadOnlyListHandle<Ref<Control>> => (|this:&Rc<dyn IDockCommandBarAdapter>,kind,definitions|this.build_bars(kind,&definitions))],
});
