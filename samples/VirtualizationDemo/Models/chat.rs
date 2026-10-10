//! Port of `Models/Chat.cs`.

use ferroui_base::ferro_markup_type;
use ferroui_base::utilities::DateTimeOffset;
use std::rc::Rc;

ferroui_controls::ferro_markup_list!(pub ChatMessageList: Rc<ChatMessage>);

pub struct ChatFile {
    chat: Option<Vec<Rc<ChatMessage>>>,
}

impl ChatFile {
    pub fn chat(&self) -> Option<&[Rc<ChatMessage>]> {
        self.chat.as_deref()
    }

    pub fn set_chat(&mut self, value: Option<Vec<Rc<ChatMessage>>>) {
        self.chat = value;
    }

    /// Reads the chat file with the path `path` below the directory of the sample
    /// (`Assets/chat.json`).
    ///
    /// The managed original opens the file in the current directory, where its build copies
    /// it. Here the file is an asset the build of the crate embeds (`build.rs`), and the path
    /// names the embedded asset.
    ///
    /// # Panics
    /// Panics if there is no such asset or it is not a chat file (the exceptions of the file
    /// and of the JSON reader in the managed original).
    pub fn load(path: &str) -> ChatFile {
        let rooted = format!("/{}", path.replace('\\', "/"));
        let content = crate::SAMPLE.asset(&rooted).unwrap_or_else(|| panic!("Could not find file '{path}'."));
        let text = std::str::from_utf8(content).unwrap_or_else(|e| panic!("{path}: {e}"));
        Self::deserialize(text).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    /// `JsonSerializer.Deserialize<ChatFile>(s, options)` with
    /// `PropertyNameCaseInsensitive = true`: the property names of the file are matched
    /// without regard to their case.
    fn deserialize(text: &str) -> Result<ChatFile, String> {
        let root = json::parse(text)?;
        let root = root.as_object().ok_or("The JSON value could not be converted to ChatFile.")?;
        let chat = match property(root, "Chat") {
            None | Some(Value::Null) => None,
            Some(Value::Array(messages)) => Some(messages.iter().map(ChatMessage::deserialize).collect::<Result<Vec<_>, _>>()?),
            Some(_) => return Err("The JSON value could not be converted to ChatMessage[].".to_string()),
        };
        Ok(ChatFile { chat })
    }
}

/// The property of a JSON object with the name `name`, in any case.
fn property<'a>(object: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    object.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value)
}

/// `record ChatMessage(string Sender, string Message, DateTimeOffset Timestamp)`: equal when
/// its three values are.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    sender: String,
    message: String,
    timestamp: DateTimeOffset,
}

impl ChatMessage {
    pub fn new(sender: String, message: String, timestamp: DateTimeOffset) -> Rc<ChatMessage> {
        Rc::new(Self { sender, message, timestamp })
    }

    pub fn sender(&self) -> String {
        self.sender.clone()
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }

    pub fn timestamp(&self) -> DateTimeOffset {
        self.timestamp
    }

    /// A message of the chat file: a parameter of the constructor the object does not state
    /// has the default of its type (the managed original then holds a null text).
    fn deserialize(value: &Value) -> Result<Rc<ChatMessage>, String> {
        let object = value.as_object().ok_or("The JSON value could not be converted to ChatMessage.")?;
        let text = |name: &str| -> Result<String, String> {
            match property(object, name) {
                None | Some(Value::Null) => Ok(String::new()),
                Some(Value::String(text)) => Ok(text.clone()),
                Some(_) => Err(format!("The JSON value could not be converted to System.String. Path: $.{name}.")),
            }
        };
        let timestamp = match property(object, "Timestamp") {
            None => DateTimeOffset::default(),
            // An ISO 8601 date and time; one without an offset is a local time.
            Some(Value::String(text)) => text
                .parse::<DateTimeOffset>()
                .map_err(|_| "The JSON value could not be converted to System.DateTimeOffset. Path: $.Timestamp.".to_string())?,
            Some(_) => return Err("The JSON value could not be converted to System.DateTimeOffset. Path: $.Timestamp.".to_string()),
        };
        Ok(ChatMessage::new(text("Sender")?, text("Message")?, timestamp))
    }
}

ferro_markup_type!(class ChatMessage {
    this: Rc<ChatMessage>,
    handles: [ChatMessage, Rc<ChatMessage>, Option<Rc<ChatMessage>>],
    constructors: [(String, String, DateTimeOffset) => ChatMessage::new],
    properties: [
        Sender: String { get: |this: &Rc<ChatMessage>| this.sender() },
        Message: String { get: |this: &Rc<ChatMessage>| this.message() },
        Timestamp: DateTimeOffset { get: |this: &Rc<ChatMessage>| this.timestamp() },
    ],
});

use json::Value;

/// The reader of the JSON text of the chat file (the role of the JSON reader of the runtime
/// library in the managed original; the framework has none): objects, arrays, texts with
/// their escapes, numbers, `true`, `false` and `null`.
mod json {
    pub(super) enum Value {
        Null,
        /// `true` or `false`: no type of the chat file has a member of this kind.
        Bool,
        /// A number, likewise.
        Number,
        String(String),
        Array(Vec<Value>),
        /// The properties in the order of the text.
        Object(Vec<(String, Value)>),
    }

    impl Value {
        pub(super) fn as_object(&self) -> Option<&[(String, Value)]> {
            match self {
                Value::Object(properties) => Some(properties),
                _ => None,
            }
        }
    }

    pub(super) fn parse(text: &str) -> Result<Value, String> {
        let mut reader = Reader { chars: text.chars().collect(), at: 0 };
        let value = reader.value()?;
        reader.skip_white_space();
        if reader.at != reader.chars.len() {
            return Err(reader.error("the end of the text"));
        }
        Ok(value)
    }

    struct Reader {
        chars: Vec<char>,
        at: usize,
    }

    impl Reader {
        fn error(&self, expected: &str) -> String {
            format!("The JSON text is invalid at character {}: expected {expected}.", self.at)
        }

        fn peek(&self) -> Option<char> {
            self.chars.get(self.at).copied()
        }

        fn skip_white_space(&mut self) {
            while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
                self.at += 1;
            }
        }

        fn expect(&mut self, c: char) -> Result<(), String> {
            if self.peek() == Some(c) {
                self.at += 1;
                Ok(())
            } else {
                Err(self.error(&format!("'{c}'")))
            }
        }

        fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
            for c in word.chars() {
                self.expect(c)?;
            }
            Ok(value)
        }

        fn value(&mut self) -> Result<Value, String> {
            self.skip_white_space();
            match self.peek() {
                Some('{') => self.object(),
                Some('[') => self.array(),
                Some('"') => Ok(Value::String(self.string()?)),
                Some('t') => self.literal("true", Value::Bool),
                Some('f') => self.literal("false", Value::Bool),
                Some('n') => self.literal("null", Value::Null),
                Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
                _ => Err(self.error("a value")),
            }
        }

        fn object(&mut self) -> Result<Value, String> {
            self.expect('{')?;
            let mut properties = Vec::new();
            self.skip_white_space();
            if self.peek() == Some('}') {
                self.at += 1;
                return Ok(Value::Object(properties));
            }
            loop {
                self.skip_white_space();
                let name = self.string()?;
                self.skip_white_space();
                self.expect(':')?;
                properties.push((name, self.value()?));
                self.skip_white_space();
                match self.peek() {
                    Some(',') => self.at += 1,
                    Some('}') => {
                        self.at += 1;
                        return Ok(Value::Object(properties));
                    }
                    _ => return Err(self.error("',' or '}'")),
                }
            }
        }

        fn array(&mut self) -> Result<Value, String> {
            self.expect('[')?;
            let mut items = Vec::new();
            self.skip_white_space();
            if self.peek() == Some(']') {
                self.at += 1;
                return Ok(Value::Array(items));
            }
            loop {
                items.push(self.value()?);
                self.skip_white_space();
                match self.peek() {
                    Some(',') => self.at += 1,
                    Some(']') => {
                        self.at += 1;
                        return Ok(Value::Array(items));
                    }
                    _ => return Err(self.error("',' or ']'")),
                }
            }
        }

        fn hex4(&mut self) -> Result<u32, String> {
            let digits: String = self.chars.iter().skip(self.at).take(4).collect();
            if digits.len() != 4 {
                return Err(self.error("four hexadecimal digits"));
            }
            let unit = u32::from_str_radix(&digits, 16).map_err(|_| self.error("four hexadecimal digits"))?;
            self.at += 4;
            Ok(unit)
        }

        fn string(&mut self) -> Result<String, String> {
            self.expect('"')?;
            let mut text = String::new();
            loop {
                let Some(c) = self.peek() else { return Err(self.error("the end of a text")) };
                self.at += 1;
                match c {
                    '"' => return Ok(text),
                    '\\' => {
                        let Some(escape) = self.peek() else { return Err(self.error("an escape")) };
                        self.at += 1;
                        match escape {
                            '"' | '\\' | '/' => text.push(escape),
                            'b' => text.push('\u{8}'),
                            'f' => text.push('\u{c}'),
                            'n' => text.push('\n'),
                            'r' => text.push('\r'),
                            't' => text.push('\t'),
                            'u' => {
                                let mut unit = self.hex4()?;
                                // A surrogate pair.
                                if (0xD800..0xDC00).contains(&unit) && self.peek() == Some('\\') && self.chars.get(self.at + 1) == Some(&'u') {
                                    self.at += 2;
                                    let low = self.hex4()?;
                                    unit = 0x10000 + ((unit - 0xD800) << 10) + (low.wrapping_sub(0xDC00) & 0x3FF);
                                }
                                text.push(char::from_u32(unit).unwrap_or('\u{FFFD}'));
                            }
                            _ => return Err(self.error("an escape")),
                        }
                    }
                    c if (c as u32) < 0x20 => return Err(self.error("a character of a text")),
                    c => text.push(c),
                }
            }
        }

        fn number(&mut self) -> Result<Value, String> {
            let start = self.at;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E')) {
                self.at += 1;
            }
            let digits: String = self.chars[start..self.at].iter().collect();
            digits.parse::<f64>().map(|_| Value::Number).map_err(|_| self.error("a number"))
        }
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_chat_file_of_the_sample_loads() {
        let file = ChatFile::load("Assets/chat.json");
        let chat = file.chat().expect("the messages of the file");
        assert_eq!(37, chat.len());
        assert_eq!("Alice", chat[0].sender());
        assert_eq!("Hey Bob! How was your weekend?", chat[0].message());
        let timestamp = chat[0].timestamp();
        assert_eq!((2023, 4, 1), (timestamp.year(), timestamp.month(), timestamp.day()));
        assert_eq!("Lisa", chat[36].sender());
        assert_eq!("That would be awesome, thanks so much!", chat[36].message());
    }

    #[test]
    fn the_property_names_are_matched_in_any_case() {
        let file = ChatFile::deserialize(r#"{ "CHAT": [ { "SENDER": "a", "Message": "b", "timeStamp": "2023-04-01T10:00:00+02:00" } ] }"#)
            .expect("a chat file");
        let chat = file.chat().expect("the messages");
        assert_eq!(("a".to_string(), "b".to_string()), (chat[0].sender(), chat[0].message()));
        assert_eq!(*chat[0], *chat[0].clone());

        assert!(ChatFile::deserialize("{}").expect("a chat file").chat().is_none());
        assert!(ChatFile::deserialize("[]").is_err());
        assert!(ChatFile::deserialize(r#"{ "chat": [ { "timestamp": "soon" } ] }"#).is_err());
    }

    #[test]
    fn the_reader_reads_the_values_of_a_json_text() {
        let file = ChatFile::deserialize(r#"{ "other": [1, -2.5e3, true, false, null, {}], "chat": [ { "sender": "a\"\u0041\n", "message": "" } ] }"#)
            .expect("a chat file");
        assert_eq!("a\"A\n", file.chat().expect("the messages")[0].sender());
        assert!(ChatFile::deserialize(r#"{ "chat": [ "#).is_err());
        assert!(ChatFile::deserialize(r#"{ "chat": null } x"#).is_err());
    }
}
