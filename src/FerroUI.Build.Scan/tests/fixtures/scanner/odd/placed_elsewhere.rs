//! The file of the module `placed`, named by `#[path]`. Its own modules are next to it.

use ferroui_base::{ferro_class, Control};

mod inner;

pub struct Placed {
    base: Control,
}

ferro_class!(Placed: Control);
