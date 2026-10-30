// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native interface metadata dispatching the original document factory contract.
use crate::DocumentFactoryReference;
use ferrodock_model::{
    controls::IDocumentDockFactory,
    core::{IDock, IDockable},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
/// Descriptor carrier for the original document creation contract.
#[doc(hidden)]
pub trait IDocumentDockFactoryMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDocumentDockFactoryMetadata as "IDocumentDockFactory" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IDocumentDockFactory>,handles:[Rc<dyn IDocumentDockFactory>,Option<Rc<dyn IDocumentDockFactory>>],
    properties:[DocumentFactory:Option<DocumentFactoryReference>{get:|this:&Rc<dyn IDocumentDockFactory>|this.document_factory().map(DocumentFactoryReference::new),set:|this:&Rc<dyn IDocumentDockFactory>,value:Option<DocumentFactoryReference>|this.set_document_factory(value.map(|v|v.model()))}],
});
pub(crate) fn register() {
    MarkupType::register(<dyn IDocumentDockFactoryMetadata>::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IDocumentDockFactory>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocumentDockFactory>>(|v| {
        Rc::as_ptr(v).cast::<()>()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDocumentDockFactory>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_document_dock_factory()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IDocumentDockFactory>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_document_dock_factory()
    });
}
