// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../NOTICE.md.

use crate::Frame;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, Ref, TypeInfo, WeakRef,
};
use std::cell::RefCell;

/// A navigation journal entry. Parameters are retained by identity, as in WinUI.
#[repr(C)]
pub struct PageStackEntry {
    base: FerroObject,
    source_page_type: &'static TypeInfo,
    parameter: Option<BoxedValue>,
    frame: RefCell<Option<WeakRef<Frame>>>,
}

ferro_class!(PageStackEntry: FerroObject);
ferro_impl_classes!(PageStackEntry: FerroObjectImpl);
ferro_class_info!(PageStackEntry {
    markup: {
        namespace: "FerroUI.Navigation",
        constructors: [
            (&'static TypeInfo, Option<BoxedValue>) => PageStackEntry::new,
        ],
        properties: [
            SourcePageType: &'static TypeInfo { get: PageStackEntry::source_page_type },
            Parameter: Option<BoxedValue> { get: PageStackEntry::parameter },
        ],
    },
});

impl PageStackEntry {
    pub fn new(source_page_type: &'static TypeInfo, parameter: Option<BoxedValue>) -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            source_page_type,
            parameter,
            frame: RefCell::new(None),
        })
    }
    pub fn source_page_type(&self) -> &'static TypeInfo {
        self.source_page_type
    }
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }
    pub(crate) fn frame(&self) -> Option<Ref<Frame>> {
        self.frame.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub(crate) fn set_frame(&self, frame: Option<&Ref<Frame>>) {
        *self.frame.borrow_mut() = frame.map(Ref::downgrade);
    }
}
