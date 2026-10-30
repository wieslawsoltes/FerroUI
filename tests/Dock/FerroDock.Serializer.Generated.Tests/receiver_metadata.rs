//! Shared source metadata for the original model's native class view.
#[path="payload_models.rs"] pub mod payload_models;
#[path="receiver_models.rs"] pub mod receiver_models;
use payload_models::Payload;
use receiver_models::PayloadView;
use std::rc::Rc;
ferro_markup_type!(class PayloadView as "GeneratedPayload" {
    this:PayloadView, handles:[PayloadView], constructors:[()=>PayloadView::create],
    properties:[
        Name:Option<String>{try_get:PayloadView::name,try_set:PayloadView::set_name},
        Self:Option<Rc<Payload>>{get:PayloadView::own,try_set:PayloadView::set_own},
    ],
});
