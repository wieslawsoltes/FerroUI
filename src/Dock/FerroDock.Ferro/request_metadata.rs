//! UI-owned descriptors exposing the original immutable request models.
use ferrodock_controls_recycling::from_recycling_value;
use ferrodock_model::services::{ConfirmationRequest, DialogRequest};
use ferroui_base::BoxedValue;
use ferroui_reactiveui::ICommand;
use std::rc::Rc;
/// Metadata carrier for the original DialogRequest shared object.
#[doc(hidden)]
pub trait DialogRequestMetadata {}
ferroui_base::ferro_markup_type!(class dyn DialogRequestMetadata as "DialogRequest" {
    namespace:"FerroDock.Model.Services",this:Rc<DialogRequest>,handles:[Rc<DialogRequest>,Option<Rc<DialogRequest>>],
    properties:[
        Content:BoxedValue{get:|this:&Rc<DialogRequest>|from_recycling_value(&this.content())},
        Title:Option<String>{get:|this:&Rc<DialogRequest>|this.title()},
        Close:Rc<dyn ICommand>{get:|this:&Rc<DialogRequest>|this.close()}
    ]
});
/// Metadata carrier for the original ConfirmationRequest shared object.
#[doc(hidden)]
pub trait ConfirmationRequestMetadata {}
ferroui_base::ferro_markup_type!(class dyn ConfirmationRequestMetadata as "ConfirmationRequest" {
    namespace:"FerroDock.Model.Services",this:Rc<ConfirmationRequest>,handles:[Rc<ConfirmationRequest>,Option<Rc<ConfirmationRequest>>],
    properties:[
        Title:String{get:|this:&Rc<ConfirmationRequest>|this.title().to_owned()},
        Message:String{get:|this:&Rc<ConfirmationRequest>|this.message().to_owned()},
        ConfirmText:String{get:|this:&Rc<ConfirmationRequest>|this.confirm_text().to_owned()},
        CancelText:String{get:|this:&Rc<ConfirmationRequest>|this.cancel_text().to_owned()},
        Confirm:Rc<dyn ICommand>{get:|this:&Rc<ConfirmationRequest>|this.confirm()},
        Cancel:Rc<dyn ICommand>{get:|this:&Rc<ConfirmationRequest>|this.cancel()}
    ]
});
