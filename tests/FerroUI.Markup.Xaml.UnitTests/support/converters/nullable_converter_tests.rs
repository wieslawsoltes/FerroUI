//! The test types declared by the upstream test file
//! `Converters/NullableConverterTests.cs`.

use std::cell::Cell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::layout::Orientation;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::Thickness;

use crate::support::TypeModule;

/// A plain class with properties of nullable value types.
pub struct ClassWithNullableProperties {
    thickness: Cell<Option<Thickness>>,
    orientation: Cell<Option<Orientation>>,
}

crate::test_identity_eq!(ClassWithNullableProperties);

impl ClassWithNullableProperties {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { thickness: Cell::new(None), orientation: Cell::new(None) })
    }

    pub fn thickness(&self) -> Option<Thickness> {
        self.thickness.get()
    }

    pub fn set_thickness(&self, value: Option<Thickness>) {
        self.thickness.set(value);
    }

    pub fn orientation(&self) -> Option<Orientation> {
        self.orientation.get()
    }

    pub fn set_orientation(&self, value: Option<Orientation>) {
        self.orientation.set(value);
    }
}

ferro_markup_type!(class ClassWithNullableProperties {
    this: Rc<ClassWithNullableProperties>,
    handles: [
        ClassWithNullableProperties,
        Rc<ClassWithNullableProperties>,
        Option<Rc<ClassWithNullableProperties>>,
    ],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Converters",
    constructors: [() => ClassWithNullableProperties::new],
    properties: [
        Thickness: Option<Thickness> {
            get: |this: &Rc<ClassWithNullableProperties>| this.thickness(),
            set: |this: &Rc<ClassWithNullableProperties>, value: Option<Thickness>| this.set_thickness(value)
        },
        Orientation: Option<Orientation> {
            get: |this: &Rc<ClassWithNullableProperties>| this.orientation(),
            set: |this: &Rc<ClassWithNullableProperties>, value: Option<Orientation>| this.set_orientation(value)
        },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<ClassWithNullableProperties as MarkupTyped>::MARKUP],
    value_types: || ValueTypes::register_reference::<ClassWithNullableProperties>(),
};
