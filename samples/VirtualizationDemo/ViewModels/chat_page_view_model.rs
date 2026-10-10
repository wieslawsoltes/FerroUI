//! Port of `ViewModels/ChatPageViewModel.cs`.

use crate::models::{ChatFile, ChatMessage};
use ferroui_base::collections::FerroList;
use ferroui_base::ferro_markup_type;
use std::rc::Rc;

pub struct ChatPageViewModel {
    messages: FerroList<Rc<ChatMessage>>,
}

impl PartialEq for ChatPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ChatPageViewModel {
    /// # Panics
    /// Panics if the chat file cannot be read (an exception of the constructor in the managed
    /// original).
    pub fn new() -> Rc<ChatPageViewModel> {
        let chat = ChatFile::load(&["Assets", "chat.json"].join("/"));
        Rc::new(Self { messages: FerroList::from_items(chat.chat().unwrap_or_default().iter().cloned()) })
    }

    pub fn messages(&self) -> FerroList<Rc<ChatMessage>> {
        self.messages.clone()
    }
}

ferro_markup_type!(class ChatPageViewModel {
    this: Rc<ChatPageViewModel>,
    handles: [ChatPageViewModel, Rc<ChatPageViewModel>, Option<Rc<ChatPageViewModel>>],
    constructors: [() => ChatPageViewModel::new],
    properties: [
        Messages: FerroList<Rc<ChatMessage>> { get: |this: &Rc<ChatPageViewModel>| this.messages() },
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_view_model_has_the_messages_of_the_chat_file() {
        let view_model = ChatPageViewModel::new();
        assert_eq!(37, view_model.messages().count());
        assert_eq!("Alice", view_model.messages().get(0).sender());
    }
}
