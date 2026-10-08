use ferroui_base::ferro_static_type;

pub struct Deep;

ferro_static_type!(Deep);

/// A type whose runtime type is written by hand.
pub struct Root;

impl ferroui_base::StaticType for Root {
    const TYPE: &'static ferroui_base::TypeInfo = &ROOT;
}

unsafe impl ferroui_base::ObjectType for Root {
    type Parent = Root;
}
