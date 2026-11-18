//! Instruction tags shared by the build-time XAML emitter and its typed execution host.
//! The program and imports are linked into the consuming crate together.

pub const NEW: u8 = 0;
pub const BEGIN_INIT: u8 = 1;
pub const END_INIT: u8 = 2;
pub const SET_F64: u8 = 3;
pub const SET_STRING: u8 = 4;
pub const SET_CONSTANT: u8 = 5;
pub const SET_OBJECT: u8 = 6;
pub const TEMPLATE_BIND: u8 = 7;
pub const REGISTER_NAME: u8 = 8;
pub const DYNAMIC_RESOURCE: u8 = 9;
pub const PUSH_PARENT: u8 = 10;
pub const POP_PARENT: u8 = 11;
pub const CALL: u8 = 12;
pub const CALL_RESULT: u8 = 13;
pub const COMPLETE_SCOPE: u8 = 14;
pub const COMPLETE_PLAIN_SCOPE: u8 = 15;
pub const INTERMEDIATE_ROOT: u8 = 16;
pub const SET_INT: u8 = 17;
pub const SET_FOUR: u8 = 18;
pub const BUILD_PATH: u8 = 19;
pub const NEW_BINDING: u8 = 20;
pub const PROVIDE_BINDING: u8 = 21;
pub const DEFER: u8 = 22;
pub const STATIC_RESOURCE: u8 = 23;
pub const BUILD_SELECTOR: u8 = 24;
pub const ADD_STYLE_LITERAL: u8 = 25;
pub const ADD_STYLE_DYNAMIC: u8 = 26;
pub const SET_PROPERTY_LITERAL: u8 = 27;
pub const SET_TARGET_PROPERTY: u8 = 28;
pub const CLEAR_TARGET_PROPERTY: u8 = 29;
pub const MOVE_TO_MARKUP: u8 = 30;
pub const APPLY_THEME_UNIT: u8 = 31;
pub const PUSH_NAMESPACE: u8 = 32;
pub const DROP_NAMESPACE: u8 = 33;
/// Write a resolved property from another register. The VM reads both registers once.
pub const SET_REGISTER: u8 = 34;
/// Construct an object from compiler-resolved binary literal operands.
pub const OBJECT_LITERAL: u8 = 35;
/// Create the actual Setter before evaluating the statements of its value.
pub const BEGIN_SETTER: u8 = 36;
/// Push the new Setter on the context's parent stack during value evaluation.
pub const BEGIN_PARENTED_SETTER: u8 = 37;
/// Convert the value register, write the Setter, then attach it to the style.
pub const FINISH_SETTER: u8 = 38;
/// Pop the Setter after assigning its value and before attaching it.
pub const FINISH_PARENTED_SETTER: u8 = 39;
/// Template binding whose source accessor precedes its target accessor.
pub const TEMPLATE_BIND_FROM: u8 = 60;

/// Predicates in the transformed setter overload plan, tested in source order.
pub mod choice {
    pub const ALWAYS: u8 = 0;
    pub const NON_NULL: u8 = 1;
    pub const NULL: u8 = 2;
    pub const INSTANCE: u8 = 3;
}
