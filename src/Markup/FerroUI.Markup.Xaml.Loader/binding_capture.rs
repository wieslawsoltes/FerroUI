//! Scoped capture of owned root bindings before candidate activation.
use ferroui_base::{
    data::BindingBase, metadata::MarkupValue, FerroObject, FerroProperty, Ref, WeakRef,
};
use ferroui_markup_xaml::FromXamlObject;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

struct Capture {
    uri: String,
    properties: Vec<u32>,
    root: RefCell<Option<WeakRef<FerroObject>>>,
    bindings: RefCell<BTreeMap<u32, Rc<dyn BindingBase>>>,
}

ferroui_base::ferro_thread_local! {
    static CAPTURES: RefCell<Vec<Rc<Capture>>> = const { RefCell::new(Vec::new()) };
}

/// Defers only the selected document root's selected properties. Other roots,
/// includes and child bindings retain ordinary runtime loading behavior.
/// Descriptions are retained only for this preparation scope, never on a target.
pub struct BindingCaptureScope(Rc<Capture>);

impl BindingCaptureScope {
    pub fn new(uri: String, properties: impl IntoIterator<Item = &'static FerroProperty>) -> Self {
        let capture = Rc::new(Capture {
            uri,
            properties: properties.into_iter().map(FerroProperty::id).collect(),
            root: RefCell::new(None),
            bindings: RefCell::new(BTreeMap::new()),
        });
        CAPTURES.with(|captures| captures.borrow_mut().push(capture.clone()));
        Self(capture)
    }

    pub fn binding(
        &self,
        root: &Ref<FerroObject>,
        property: &'static FerroProperty,
    ) -> Option<Rc<dyn BindingBase>> {
        if !self
            .0
            .root
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .is_some_and(|candidate| candidate.ptr_eq(root))
        {
            return None;
        }
        self.0.bindings.borrow().get(&property.id()).cloned()
    }
}

impl Drop for BindingCaptureScope {
    fn drop(&mut self) {
        // Drop descriptors outside the TLS borrow: their destructors may load markup.
        let retired = CAPTURES.with(|captures| {
            let mut captures = captures.borrow_mut();
            captures
                .iter()
                .position(|capture| Rc::ptr_eq(capture, &self.0))
                .map(|index| captures.remove(index))
        });
        drop(retired);
    }
}

pub(crate) fn register_root(uri: Option<&str>, value: &MarkupValue) {
    let capture = CAPTURES.with(|captures| captures.borrow().last().cloned());
    let Some(capture) = capture else { return };
    if uri != Some(capture.uri.as_str()) || capture.root.borrow().is_some() {
        return;
    }
    if let Some(root) = value
        .as_ref()
        .and_then(Ref::<FerroObject>::from_xaml_object)
    {
        *capture.root.borrow_mut() = Some(root.downgrade());
    }
}

pub(crate) fn defer(
    target: &Ref<FerroObject>,
    property: &'static FerroProperty,
    binding: &Rc<dyn BindingBase>,
) -> bool {
    let capture = CAPTURES.with(|captures| captures.borrow().last().cloned());
    let Some(capture) = capture else { return false };
    if !capture.properties.contains(&property.id())
        || !capture
            .root
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .is_some_and(|root| root.ptr_eq(target))
    {
        return false;
    }
    let retired = capture
        .bindings
        .borrow_mut()
        .insert(property.id(), binding.clone());
    drop(retired);
    true
}
