use super::ContentPresenter;
use ferroui_base::collections::FerroList;
use ferroui_base::{FerroObject, ObjectType, Ref, StyledElement, TypeInfo};
use std::cell::RefCell;

/// Represents a control which hosts a content presenter.
///
/// This interface is implemented by `ContentControl` and is used as follows:
///
/// - `register_content_presenter` is called by a content presenter when its
///   templated parent changes; the host decides whether the presenter is
///   the one that displays its content.
/// - The presenter of a host adds the control it creates to the logical
///   children of the host rather than to its own.
///
/// A class implementing the trait is made known with
/// [`register_content_presenter_host`], normally from its class
/// initialization.
pub trait IContentPresenterHost {
    /// The logical children of the host control.
    fn logical_children(&self) -> &FerroList<Ref<StyledElement>>;

    /// Registers a content presenter with the host. Returns true if the
    /// presenter was registered as the host's presenter.
    fn register_content_presenter(&self, presenter: &ContentPresenter) -> bool;
}

type HostCast = for<'a> fn(&'a FerroObject) -> Option<&'a dyn IContentPresenterHost>;

thread_local! {
    static HOST_TYPES: RefCell<Vec<(&'static TypeInfo, HostCast)>> = const { RefCell::new(Vec::new()) };
}

fn cast_host<T: ObjectType + IContentPresenterHost>(object: &FerroObject) -> Option<&dyn IContentPresenterHost> {
    object.downcast_ref::<T>().map(|host| host as &dyn IContentPresenterHost)
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`IContentPresenterHost`].
pub fn register_content_presenter_host<T: ObjectType + IContentPresenterHost>() {
    HOST_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            types.push((T::TYPE, cast_host::<T> as HostCast));
        }
    });
}

/// The object viewed as a content presenter host, if its class implements
/// [`IContentPresenterHost`].
pub fn as_content_presenter_host(object: &FerroObject) -> Option<&dyn IContentPresenterHost> {
    let cast = HOST_TYPES.with(|types| {
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
