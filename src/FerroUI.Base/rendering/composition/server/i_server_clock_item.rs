/// Something the server compositor advances once per frame.
pub trait IServerClockItem {
    fn on_tick(&self);
}
