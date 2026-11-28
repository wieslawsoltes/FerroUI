//! Resolved compiled binding path instructions. Import indices and index counts are varints.
pub const NOT: u8 = 0;
pub const OBSERVABLE: u8 = 1;
pub const TASK: u8 = 2;
pub const SELF: u8 = 3;
pub const TEMPLATE_PARENT: u8 = 4;
pub const NAMED: u8 = 5;
pub const PROPERTY: u8 = 6;
pub const NULLABLE_PROPERTY: u8 = 7;
pub const ARRAY: u8 = 8;
pub const IMPORTED_STEP: u8 = 9;
pub const ANCESTOR: u8 = 10;
pub const VISUAL_ANCESTOR: u8 = 11;
