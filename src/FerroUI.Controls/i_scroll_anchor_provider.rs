use crate::Control;
use ferroui_base::{FerroObject, ObjectType, Ref, TypeInfo};
use std::cell::RefCell;

/// Specifies a contract for a scrolling control that supports scroll
/// anchoring.
///
/// A class implementing the trait is made known with
/// [`register_scroll_anchor_provider`], normally from its class
/// initialization; [`as_scroll_anchor_provider`] then views an object of the
/// class (or of a class derived from it) as the interface.
pub trait IScrollAnchorProvider {
    /// The currently chosen anchor element to use for scroll anchoring.
    fn current_anchor(&self) -> Option<Ref<Control>>;

    /// Registers a control as a potential scroll anchor candidate.
    ///
    /// `element` is a control within the subtree of the provider.
    fn register_anchor_candidate(&self, element: &Ref<Control>);

    /// Unregisters a control as a potential scroll anchor candidate.
    ///
    /// `element` is a control within the subtree of the provider.
    fn unregister_anchor_candidate(&self, element: &Ref<Control>);
}

type ProviderCast = for<'a> fn(&'a FerroObject) -> Option<&'a dyn IScrollAnchorProvider>;

thread_local! {
    static PROVIDER_TYPES: RefCell<Vec<(&'static TypeInfo, ProviderCast)>> = const { RefCell::new(Vec::new()) };
}

fn cast_provider<T: ObjectType + IScrollAnchorProvider>(object: &FerroObject) -> Option<&dyn IScrollAnchorProvider> {
    object
        .downcast_ref::<T>()
        .map(|provider| provider as &dyn IScrollAnchorProvider)
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`IScrollAnchorProvider`].
pub fn register_scroll_anchor_provider<T: ObjectType + IScrollAnchorProvider>() {
    PROVIDER_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            types.push((T::TYPE, cast_provider::<T> as ProviderCast));
        }
    });
}

/// The object viewed as a scroll anchor provider, if its class implements
/// [`IScrollAnchorProvider`].
pub fn as_scroll_anchor_provider(object: &FerroObject) -> Option<&dyn IScrollAnchorProvider> {
    let cast = PROVIDER_TYPES.with(|types| {
        let types = types.borrow();
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            if let Some((_, cast)) = types.iter().find(|(candidate, _)| *candidate == type_) {
                return Some(*cast);
            }
            current = type_.base_type();
        }
        None
    });
    cast.and_then(|cast| cast(object))
}
