//! Port of the sample object of the test project root: an object of the
//! object model with two registered properties whose names differ from the
//! names of their accessors.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};

#[repr(C)]
pub struct SampleFerroObject {
    base: FerroObject,
}

ferro_class!(SampleFerroObject: FerroObject);
ferro_impl_classes!(SampleFerroObject: FerroObjectImpl);
ferro_class_info!(SampleFerroObject {
    new: SampleFerroObject::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests",
        properties: [
            Int: i32 { get: SampleFerroObject::int, set: SampleFerroObject::set_int },
            String: String {
                get: SampleFerroObject::string,
                set: |this: &Ref<SampleFerroObject>, value: String| this.set_string(&value)
            },
        ],
    },
});

ferro_properties! {
    impl SampleFerroObject {
        pub fn string_property() -> StyledProperty<String> {
            FerroProperty::register::<FerroObject, _>("StrProp", String::new())
        }

        pub fn int_property() -> StyledProperty<i32> {
            FerroProperty::register::<FerroObject, _>("IntProp", 0)
        }
    }
}

impl SampleFerroObject {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn int(&self) -> i32 {
        self.get_value(Self::int_property())
    }

    pub fn set_int(&self, value: i32) {
        self.set_value(Self::int_property(), value)
    }

    pub fn string(&self) -> String {
        self.get_value(Self::string_property())
    }

    pub fn set_string(&self, value: &str) {
        self.set_value(Self::string_property(), value.to_string())
    }
}
