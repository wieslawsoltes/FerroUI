//! `Models/TreeDataGrid/ReflectionTextColumn.cs`: the text column of the declarative columns of
//! markup, over models that are untyped values, whose value is read by a delegate (or by the name
//! of a property of the model's type metadata, the stand-in of reflection).
//!
//! The reference's class is a text column of `object` values; here it is a
//! [`TextColumn<BoxedValue, Option<BoxedValue>>`](TextColumn) built from the delegates, sorted by
//! the default comparison of untyped values.

use super::object_hierarchical_expander_column::ObjectLink;
use super::{TextColumn, TextColumnOptions};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::ModelTypes;
use ferroui_base::BoxedValue;
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::models::ValueComparer;
use ferroui_treedatagrid_core::Accessor;
use std::cmp::Ordering;
use std::rc::Rc;

/// The getter of a value of an untyped model (`Func<object, object?>`).
pub type ObjectGetter = Rc<dyn Fn(&BoxedValue) -> Option<BoxedValue>>;

/// The setter of a value of an untyped model (`Action<object, object?>`).
pub type ObjectSetter = Rc<dyn Fn(&BoxedValue, Option<BoxedValue>)>;

/// `ReflectionTextColumn`.
pub struct ReflectionTextColumn;

impl ReflectionTextColumn {
    /// A text column whose value `getter` reads and `setter` (if any) writes, re-read when one of
    /// the objects `links` returns changes (`ReflectionTextColumn(header, getter, setter, links,
    /// width, options)`).
    pub fn new(
        header: Option<BoxedValue>,
        getter: ObjectGetter,
        setter: Option<ObjectSetter>,
        links: Vec<ObjectLink>,
        width: GridLength,
        options: Rc<TextColumnOptions<BoxedValue>>,
    ) -> Rc<TextColumn<BoxedValue, Option<BoxedValue>>> {
        TextColumn::with_comparer(
            header,
            object_accessor(getter, setter, links, None),
            Some(object_comparer()),
            Some(width),
            Some(options),
        )
    }

    /// A text column of the property `property_name` of the models, read and written through the
    /// property metadata of the models' types (`ModelTypes`), and observed by the models'
    /// `PropertyChanged` for that name.
    pub fn for_property(
        header: Option<BoxedValue>,
        property_name: &'static str,
        is_read_only: bool,
        width: GridLength,
        options: Rc<TextColumnOptions<BoxedValue>>,
    ) -> Rc<TextColumn<BoxedValue, Option<BoxedValue>>> {
        let (getter, setter) = property_accessors(property_name, is_read_only);
        TextColumn::with_comparer(
            header,
            object_accessor(getter, setter, Vec::new(), Some(property_name)),
            Some(object_comparer()),
            Some(width),
            Some(options),
        )
    }
}

/// The getter and setter of the property `property_name` of a model, through the property
/// metadata of the model's type.
pub(crate) fn property_accessors(property_name: &'static str, is_read_only: bool) -> (ObjectGetter, Option<ObjectSetter>) {
    let getter: ObjectGetter = Rc::new(move |model: &BoxedValue| {
        let property = ModelTypes::find(&**model)?.find_property(property_name)?;
        property.get_boxed(model)
    });
    let setter: Option<ObjectSetter> = if is_read_only {
        None
    } else {
        Some(Rc::new(move |model: &BoxedValue, value: Option<BoxedValue>| {
            if let Some(property) = ModelTypes::find(&**model).and_then(|t| t.find_property(property_name)) {
                if property.can_set() {
                    // A value the property does not take is not written (a data validation error
                    // of the reference).
                    let _ = property.set_boxed(model, value.as_ref());
                }
            }
        }))
    };
    (getter, setter)
}

/// The accessor of an untyped value: the delegates over the untyped model, the member name, and the
/// objects observed (the model itself when there are no links and no member name).
pub(crate) fn object_accessor<V: 'static>(
    getter: Rc<dyn Fn(&BoxedValue) -> V>,
    setter: Option<Rc<dyn Fn(&BoxedValue, V)>>,
    links: Vec<ObjectLink>,
    property_name: Option<&'static str>,
) -> Accessor<BoxedValue, V> {
    let mut accessor = Accessor::new(move |model: &Rc<BoxedValue>| getter(&**model));
    if let Some(name) = property_name {
        accessor = accessor.named(name);
    }
    if let Some(setter) = setter {
        accessor = accessor.with_setter(move |model: &Rc<BoxedValue>, value: V| setter(&**model, value));
    }
    for link in links {
        accessor = accessor.through(move |model: &Rc<BoxedValue>| link(&**model), "");
    }
    accessor
}

/// `Comparer<object>.Default`: null before any value; numbers, text, booleans and characters by
/// their order; other values by their text.
pub(crate) fn object_comparer() -> ValueComparer<Option<BoxedValue>> {
    Rc::new(|a: &Option<BoxedValue>, b: &Option<BoxedValue>| match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => compare_boxed(a, b),
    })
}

fn as_number(value: &BoxedValue) -> Option<f64> {
    macro_rules! number {
        ($($ty:ty),*) => {
            $(
                if let Some(v) = value.downcast_ref::<$ty>() {
                    return Some(*v as f64);
                }
            )*
        };
    }
    number!(f64, f32, i8, i16, i32, i64, u8, u16, u32, u64, isize, usize);
    None
}

fn compare_boxed(a: &BoxedValue, b: &BoxedValue) -> Ordering {
    if let (Some(a), Some(b)) = (as_number(a), as_number(b)) {
        return a.partial_cmp(&b).unwrap_or_else(|| a.is_nan().cmp(&b.is_nan()).reverse());
    }
    if let (Some(a), Some(b)) = (a.downcast_ref::<String>(), b.downcast_ref::<String>()) {
        return a.cmp(b);
    }
    if let (Some(a), Some(b)) = (a.downcast_ref::<bool>(), b.downcast_ref::<bool>()) {
        return a.cmp(b);
    }
    if let (Some(a), Some(b)) = (a.downcast_ref::<char>(), b.downcast_ref::<char>()) {
        return a.cmp(b);
    }
    ValueTypes::to_display_string(Some(a)).cmp(&ValueTypes::to_display_string(Some(b)))
}
