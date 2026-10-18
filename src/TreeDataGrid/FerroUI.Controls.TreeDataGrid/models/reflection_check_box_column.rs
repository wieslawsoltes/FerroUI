//! `Models/TreeDataGrid/ReflectionCheckBoxColumn.cs`: the check box column of the declarative
//! columns of markup, over models that are untyped values, whose value is read by a delegate (or by
//! the name of a property of the model's type metadata).
//!
//! The reference's class is a check box column of `object` models; here it is a
//! [`CheckBoxColumn<BoxedValue>`](CheckBoxColumn) built from the delegates.

use super::object_hierarchical_expander_column::ObjectLink;
use super::reflection_text_column::{object_accessor, property_accessors};
use super::{CheckBoxColumn, CheckBoxColumnOptions};
use ferroui_base::BoxedValue;
use ferroui_controls::GridLength;
use std::rc::Rc;

/// `ReflectionCheckBoxColumn`.
pub struct ReflectionCheckBoxColumn;

impl ReflectionCheckBoxColumn {
    /// A check box column whose value `getter` reads and `setter` (if any) writes, re-read when
    /// one of the objects `links` returns changes (`ReflectionCheckBoxColumn(header, getter,
    /// setter, links, width, options, isThreeState)`).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        header: Option<BoxedValue>,
        getter: Rc<dyn Fn(&BoxedValue) -> Option<bool>>,
        setter: Option<Rc<dyn Fn(&BoxedValue, Option<bool>)>>,
        links: Vec<ObjectLink>,
        width: GridLength,
        options: Rc<CheckBoxColumnOptions<BoxedValue>>,
        is_three_state: bool,
    ) -> Rc<CheckBoxColumn<BoxedValue>> {
        CheckBoxColumn::with_three_state(
            header,
            object_accessor(getter, setter, links, None),
            Some(width),
            Some(options),
            is_three_state,
        )
    }

    /// A check box column of the property `property_name` of the models, read and written through
    /// the property metadata of the models' types: a `bool` or `bool?` value, null for any other.
    pub fn for_property(
        header: Option<BoxedValue>,
        property_name: &'static str,
        is_read_only: bool,
        width: GridLength,
        options: Rc<CheckBoxColumnOptions<BoxedValue>>,
        is_three_state: bool,
    ) -> Rc<CheckBoxColumn<BoxedValue>> {
        let (get, set) = property_accessors(property_name, is_read_only);
        let getter: Rc<dyn Fn(&BoxedValue) -> Option<bool>> = Rc::new(move |model: &BoxedValue| {
            let value = get(model)?;
            match value.downcast_ref::<bool>() {
                Some(value) => Some(*value),
                None => value.downcast_ref::<Option<bool>>().copied().flatten(),
            }
        });
        let setter = set.map(|set| {
            Rc::new(move |model: &BoxedValue, value: Option<bool>| set(model, value.map(|v| Rc::new(v) as BoxedValue)))
                as Rc<dyn Fn(&BoxedValue, Option<bool>)>
        });
        CheckBoxColumn::with_three_state(
            header,
            object_accessor(getter, setter, Vec::new(), Some(property_name)),
            Some(width),
            Some(options),
            is_three_state,
        )
    }
}
