//! Text input methods (IME): the contract between text editing controls
//! and the input method of the platform.

mod i_text_input_method_impl;
mod input_method_manager;
mod text_input_content_type;
mod text_input_method_client;
mod text_input_method_client_requery_requested_event_args;
mod text_input_method_client_requested_event_args;
mod text_input_options;
mod text_input_return_key_type;
mod transform_tracking_helper;

pub use i_text_input_method_impl::ITextInputMethodImpl;
pub(crate) use input_method_manager::TextInputMethodManager;
pub use text_input_content_type::TextInputContentType;
pub use text_input_method_client::{
    ContextMenuAction, TextInputMethodClient, TextInputMethodClientEvents, TextSelection,
};
pub use text_input_method_client_requery_requested_event_args::TextInputMethodClientRequeryRequestedEventArgs;
pub use text_input_method_client_requested_event_args::TextInputMethodClientRequestedEventArgs;
pub use text_input_options::TextInputOptions;
pub use text_input_return_key_type::TextInputReturnKeyType;
pub use transform_tracking_helper::TransformTrackingHelper;

