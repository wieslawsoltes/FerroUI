/// The label of the return key of an on-screen keyboard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextInputReturnKeyType {
    #[default]
    Default = 0,
    Return = 1,
    Done = 2,
    Go = 3,
    Send = 4,
    Search = 5,
    Next = 6,
    Previous = 7,
}
