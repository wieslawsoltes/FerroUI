use crate::templates::IDataTemplate;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{BoxedValue, FerroObject, ObjectType, TypeInfo};
use std::cell::RefCell;
use std::rc::Rc;

/// Defines a control that displays content according to a data template.
///
/// A class implementing the trait is made known with
/// [`register_content_control`], normally from its class initialization;
/// [`as_content_control`] then views an object of the class (or of a class
/// derived from it) as the interface.
pub trait IContentControl {
    /// The content to display.
    fn content(&self) -> Option<BoxedValue>;

    /// Sets the content to display.
    fn set_content(&self, value: Option<BoxedValue>);

    /// The data template used to display the content of the control.
    fn content_template(&self) -> Option<Rc<dyn IDataTemplate>>;

    /// Sets the data template used to display the content of the control.
    fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>);

    /// The horizontal alignment of the content within the control.
    fn horizontal_content_alignment(&self) -> HorizontalAlignment;

    /// Sets the horizontal alignment of the content within the control.
    fn set_horizontal_content_alignment(&self, value: HorizontalAlignment);

    /// The vertical alignment of the content within the control.
    fn vertical_content_alignment(&self) -> VerticalAlignment;

    /// Sets the vertical alignment of the content within the control.
    fn set_vertical_content_alignment(&self, value: VerticalAlignment);
}

type ContentControlCast = for<'a> fn(&'a FerroObject) -> Option<&'a dyn IContentControl>;

thread_local! {
    static CONTENT_CONTROL_TYPES: RefCell<Vec<(&'static TypeInfo, ContentControlCast)>> = const { RefCell::new(Vec::new()) };
}

fn cast_content_control<T: ObjectType + IContentControl>(object: &FerroObject) -> Option<&dyn IContentControl> {
    object.downcast_ref::<T>().map(|content_control| content_control as &dyn IContentControl)
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`IContentControl`].
pub fn register_content_control<T: ObjectType + IContentControl>() {
    CONTENT_CONTROL_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            types.push((T::TYPE, cast_content_control::<T> as ContentControlCast));
        }
    });
}

/// The object viewed as a content control, if its class implements
/// [`IContentControl`].
pub fn as_content_control(object: &FerroObject) -> Option<&dyn IContentControl> {
    let cast = CONTENT_CONTROL_TYPES.with(|types| {
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
