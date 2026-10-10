//! Value types of one to eight integers, for the benchmarks of properties
//! whose values differ in size.

macro_rules! test_struct {
    ($name:ident { $($field:ident),+ }) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct $name {
            $(pub $field: i32,)+
        }

        impl $name {
            pub fn new(value: i32) -> Self {
                Self { $($field: value,)+ }
            }
        }
    };
}

test_struct!(Struct1 { int1 });
test_struct!(Struct2 { int1, int2 });
test_struct!(Struct3 { int1, int2, int3 });
test_struct!(Struct4 { int1, int2, int3, int4 });
test_struct!(Struct5 { int1, int2, int3, int4, int5 });
test_struct!(Struct6 { int1, int2, int3, int4, int5, int6 });
test_struct!(Struct7 { int1, int2, int3, int4, int5, int6, int7 });
test_struct!(Struct8 { int1, int2, int3, int4, int5, int6, int7, int8 });
