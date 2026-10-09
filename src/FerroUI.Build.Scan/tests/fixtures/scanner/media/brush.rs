use ferroui_base::{ferro_class, FerroObject};

pub trait IBrush {}

pub struct Brush {
    base: FerroObject,
}

ferro_class!(Brush: FerroObject);
