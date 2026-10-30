// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockSelectorMode;
/// Provides control over the original dock selector overlay.
pub trait IDockSelectorService {
    /// Resolves the native receiver behind a control's interface handle.
    #[doc(hidden)]
    fn as_object(&self) -> Option<ferroui_base::Ref<ferroui_base::FerroObject>> {
        None
    }
    /// Shows the selector for the supplied scope.
    fn show_selector(&self, mode: DockSelectorMode);
    /// Hides the selector.
    fn hide_selector(&self);
    /// Gets whether the selector is open.
    fn is_open(&self) -> bool;
}
impl PartialEq for dyn IDockSelectorService {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(left), Some(right)) => left.ptr_eq(&right),
            (None, None) => std::ptr::addr_eq(self as *const Self, other as *const Self),
            _ => false,
        }
    }
}
ferroui_base::ferro_markup_type!(interface dyn IDockSelectorService as "IDockSelectorService" {
    namespace:"FerroDock.Ferro.Selectors",this:std::rc::Rc<dyn IDockSelectorService>,
    properties:[IsOpen:bool {get:|this:&std::rc::Rc<dyn IDockSelectorService>|this.is_open()}],
    methods:[fn ShowSelector(DockSelectorMode)=>(|this:&std::rc::Rc<dyn IDockSelectorService>,mode|this.show_selector(mode)),fn HideSelector()=>(|this:&std::rc::Rc<dyn IDockSelectorService>|this.hide_selector())],
});
