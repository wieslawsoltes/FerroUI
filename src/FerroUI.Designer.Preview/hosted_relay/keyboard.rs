use super::{FRAME_TIMEOUT, Relay, send};
use ferroui_inspect::{KeyboardFocus, KeyboardInput};
use ferroui_remote_protocol::designer::DesignKeyboardRequest;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Keyboard {
    revision: i64,
    sequence: i64,
    input: KeyboardInput,
}

impl Relay {
    pub(crate) fn keyboard(&mut self, input: Keyboard) -> Result<Option<KeyboardFocus>, String> {
        let connection = self
            .input_connection(input.revision, input.sequence)?
            .clone();
        if !self.keyboard_available {
            return Err("Rebuild the application with App Preview keyboard support.".into());
        }
        let (key, text) = match input.input {
            KeyboardInput::Focus => (None, None),
            KeyboardInput::Key { key } if !key.is_empty() && key.len() <= 64 => (Some(key), None),
            KeyboardInput::Text { text } if !text.is_empty() && text.len() <= 8192 => {
                (None, Some(text))
            }
            _ => return Err("Enter a key gesture or up to 8 KiB of text.".into()),
        };
        self.next_keyboard += 1;
        let request = self.next_keyboard;
        let (pending, changed) = &*self.events;
        {
            let mut state = pending.lock().unwrap();
            state.keyboard_request = Some(request);
            state.keyboard_result = None;
        }
        let sent = send(
            &connection,
            Arc::new(DesignKeyboardRequest {
                revision: input.revision,
                request,
                key,
                text,
            }),
        );
        let mut state = pending.lock().unwrap();
        if sent.is_ok() {
            (state, _) = changed
                .wait_timeout_while(state, FRAME_TIMEOUT, |state| {
                    !state.disconnected && state.keyboard_result.is_none()
                })
                .unwrap();
        }
        state.keyboard_request = None;
        sent?;
        let answer = state
            .keyboard_result
            .take()
            .ok_or("The application did not acknowledge preview keyboard input.")?;
        match answer.error {
            Some(error) => Err(error),
            None => Ok(answer.focus.map(|focus| KeyboardFocus {
                name: focus.name,
                role: focus.role,
            })),
        }
    }
}
