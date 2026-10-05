/// The type of a raw drag event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RawDragEventType {
    DragEnter = 0,
    DragOver = 1,
    DragLeave = 2,
    Drop = 3,
}
