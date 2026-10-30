// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{templates::IDataTemplate, Control};
use std::rc::Rc;
pub(crate) trait IOverlayContentHost {
    fn content(&self) -> Option<BoxedValue>;
    fn set_content(&self, value: Option<BoxedValue>);
    fn content_template(&self) -> Option<Rc<dyn IDataTemplate>>;
    fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>);
    fn blocks_input(&self) -> bool;
    fn set_blocks_input(&self, value: bool);
}
impl PartialEq for dyn IOverlayContentHost {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
pub(crate) fn content_host(control: &Ref<Control>) -> Option<Rc<dyn IOverlayContentHost>> {
    use ferroui_base::data::core::{ValueType, ValueTypes};
    ValueTypes::try_cast(
        &Control::boxed(control.clone()),
        ValueType::of::<Rc<dyn IOverlayContentHost>>(),
    )?
    .downcast_ref::<Rc<dyn IOverlayContentHost>>()
    .cloned()
}

ferroui_base::ferro_markup_type!(interface dyn IOverlayContentHost as "IOverlayContentHost" {
    this:Rc<dyn IOverlayContentHost>,
    handles:[Rc<dyn IOverlayContentHost>,Option<Rc<dyn IOverlayContentHost>>],
    properties:[
        Content:Option<BoxedValue>{get:|this:&Rc<dyn IOverlayContentHost>|this.content(),set:|this:&Rc<dyn IOverlayContentHost>,v|this.set_content(v)},
        ContentTemplate:Option<Rc<dyn IDataTemplate>>{get:|this:&Rc<dyn IOverlayContentHost>|this.content_template(),set:|this:&Rc<dyn IOverlayContentHost>,v|this.set_content_template(v)},
        BlocksInput:bool{get:|this:&Rc<dyn IOverlayContentHost>|this.blocks_input(),set:|this:&Rc<dyn IOverlayContentHost>,v|this.set_blocks_input(v)}
    ]
});
