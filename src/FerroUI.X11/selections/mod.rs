//! The selections of the X server: the clipboard and, later, the drag and
//! drop transfers (the port of the `Selections` directory).

pub mod clipboard;
pub mod data_format_helper;
pub mod i_x_event_waiter;
pub mod selection_data_provider;
pub mod selection_data_reader;
pub mod selection_helper;
pub mod selection_read_session;
pub mod uri_list_helper;
