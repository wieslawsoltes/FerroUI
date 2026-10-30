// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{BoxedValue, FerroObject, Ref};
use ferroui_controls::Control;
/// Complete source layer contract, including optional native property-object projection.
pub trait IOverlayLayer {
    /// Gets z-order.
    fn z_index(&self) -> i32;
    /// Gets visibility.
    fn is_visible(&self) -> bool;
    /// Gets whether this layer blocks hosted input.
    fn blocks_input(&self) -> bool;
    /// Gets the optional theme resource key.
    fn style_key(&self) -> Option<BoxedValue>;
    /// Gets the optional native overlay control.
    fn overlay(&self) -> Option<Ref<Control>>;
    /// Queries native property notifications when the layer is a framework object.
    fn as_object(&self) -> Option<Ref<FerroObject>> {
        None
    }
}
impl PartialEq for dyn IOverlayLayer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(interface dyn IOverlayLayer as "IOverlayLayer" {
    this:std::rc::Rc<dyn IOverlayLayer>,
    handles:[std::rc::Rc<dyn IOverlayLayer>,Option<std::rc::Rc<dyn IOverlayLayer>>],
    properties:[
        ZIndex:i32{get:|this:&std::rc::Rc<dyn IOverlayLayer>|this.z_index()},
        IsVisible:bool{get:|this:&std::rc::Rc<dyn IOverlayLayer>|this.is_visible()},
        BlocksInput:bool{get:|this:&std::rc::Rc<dyn IOverlayLayer>|this.blocks_input()},
        StyleKey:Option<BoxedValue>{get:|this:&std::rc::Rc<dyn IOverlayLayer>|this.style_key()},
        Overlay:Option<Ref<Control>>{get:|this:&std::rc::Rc<dyn IOverlayLayer>|this.overlay()}
    ]
});
