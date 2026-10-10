/// Who draws the decorations of a top-level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationMode {
    ClientSide,
    ServerSide,
}
