// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::IOverlayLayer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use ferroui_controls::Control;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
/// A native property object wrapping an overlay with its source metadata.
#[repr(C)]
pub struct OverlayLayer {
    base: FerroObject,
    handle: RefCell<Weak<LayerHandle>>,
}
ferro_class!(OverlayLayer:FerroObject);
ferro_impl_classes!(OverlayLayer:FerroObjectImpl);
ferro_class_info!(OverlayLayer {new:OverlayLayer::new,interfaces:[Rc<dyn IOverlayLayer> => OverlayLayer::as_layer],markup:{content:Overlay}});
ferro_properties! {impl OverlayLayer {
    /// Identifies z-order, zero by default.
    pub fn z_index_property()->StyledProperty<i32>{FerroProperty::register::<OverlayLayer,_>("ZIndex",0)}
    /// Identifies layer visibility, true by default.
    pub fn is_visible_property()->StyledProperty<bool>{FerroProperty::register::<OverlayLayer,_>("IsVisible",true)}
    /// Identifies input blocking, true by default.
    pub fn blocks_input_property()->StyledProperty<bool>{FerroProperty::register::<OverlayLayer,_>("BlocksInput",true)}
    /// Identifies the optional theme key.
    pub fn style_key_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<OverlayLayer,_>("StyleKey",None)}
    /// Identifies the optional overlay and markup content.
    pub fn overlay_property()->StyledProperty<Option<Ref<Control>>>{FerroProperty::register::<OverlayLayer,_>("Overlay",None)}
}}
impl OverlayLayer {
    fn static_constructor() {
        ferroui_base::data::core::ValueTypes::register_interface_object::<Rc<dyn IOverlayLayer>>(
            |h| h.as_object(),
        );
    }
    /// Constructs source class state.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            handle: RefCell::new(Weak::new()),
        }
    }
    /// Creates a source-default layer.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets z-order.
    pub fn z_index(&self) -> i32 {
        self.get_value(Self::z_index_property())
    }
    /// Sets z-order.
    pub fn set_z_index(&self, value: i32) {
        self.set_value(Self::z_index_property(), value);
    }
    /// Gets visibility.
    pub fn is_visible(&self) -> bool {
        self.get_value(Self::is_visible_property())
    }
    /// Sets visibility.
    pub fn set_is_visible(&self, value: bool) {
        self.set_value(Self::is_visible_property(), value);
    }
    /// Gets input blocking.
    pub fn blocks_input(&self) -> bool {
        self.get_value(Self::blocks_input_property())
    }
    /// Sets input blocking.
    pub fn set_blocks_input(&self, value: bool) {
        self.set_value(Self::blocks_input_property(), value);
    }
    /// Gets the optional theme key.
    pub fn style_key(&self) -> Option<BoxedValue> {
        self.get_value(Self::style_key_property())
    }
    /// Sets the optional theme key.
    pub fn set_style_key(&self, value: Option<BoxedValue>) {
        self.set_value(Self::style_key_property(), value);
    }
    /// Gets the overlay control.
    pub fn overlay(&self) -> Option<Ref<Control>> {
        self.get_value(Self::overlay_property())
    }
    /// Sets the overlay control.
    pub fn set_overlay(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::overlay_property(), value);
    }
    /// Projects this same native object to its source layer interface.
    pub fn as_layer(this: Ref<Self>) -> Rc<dyn IOverlayLayer> {
        if let Some(handle) = this.handle.borrow().upgrade() {
            return handle;
        }
        let handle = Rc::new(LayerHandle(this.clone()));
        this.handle.replace(Rc::downgrade(&handle));
        handle
    }
}
struct LayerHandle(Ref<OverlayLayer>);
impl IOverlayLayer for LayerHandle {
    fn z_index(&self) -> i32 {
        self.0.z_index()
    }
    fn is_visible(&self) -> bool {
        self.0.is_visible()
    }
    fn blocks_input(&self) -> bool {
        self.0.blocks_input()
    }
    fn style_key(&self) -> Option<BoxedValue> {
        self.0.style_key()
    }
    fn overlay(&self) -> Option<Ref<Control>> {
        self.0.overlay()
    }
    fn as_object(&self) -> Option<Ref<FerroObject>> {
        Some(self.0.clone().upcast())
    }
}
