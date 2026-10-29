//! WinUI's `PageStackEntry` (`dxaml/xcp/dxaml/lib/PageStackEntry_Partial.cpp` of microsoft-ui-xaml,
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! ../NOTICE.md).

use crate::Frame;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref,
    TypeInfo, WeakRef,
};
use std::cell::RefCell;

/// An entry of the journal of a [`Frame`]: the type of a page and the parameter of its navigation.
/// The parameter is kept as it is (by identity), as in WinUI.
///
/// Missing (docs/porting/winui-controls.md, section 10.4): `NavigationTransitionInfo`, FerroUI's
/// frame has no navigation transitions.
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
    /// Creates an entry of the page type `source_page_type` with the parameter of its navigation.
    pub fn new(source_page_type: &'static TypeInfo, parameter: Option<BoxedValue>) -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct(), source_page_type, parameter, frame: RefCell::new(None) })
    }

    /// Gets the type of the page of the entry.
    pub fn source_page_type(&self) -> &'static TypeInfo {
        self.source_page_type
    }

    /// Gets the parameter of the navigation.
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }

    /// The frame whose journal holds the entry (weakly, so that an entry keeps no frame alive).
    pub(crate) fn frame(&self) -> Option<Ref<Frame>> {
        self.frame.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_frame(&self, frame: Option<&Ref<Frame>>) {
        *self.frame.borrow_mut() = frame.map(Ref::downgrade);
    }
}
