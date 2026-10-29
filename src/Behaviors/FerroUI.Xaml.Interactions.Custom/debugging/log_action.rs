//! Port of `Debugging/LogAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_enum, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Specifies the severity level of the log message.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LogActionLevel {
    /// Information level.
    #[default]
    Info,

    /// Warning level.
    Warning,

    /// Error level.
    Error,

    /// Debug level.
    Debug,
}

impl std::fmt::Display for LogActionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::Debug => "Debug",
        })
    }
}

ferro_markup_enum!(LogActionLevel { Info, Warning, Error, Debug });

/// The category of the messages of the action.
const CATEGORY: &str = "LogAction";

/// An action that logs a message to the debug output.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference writes the message to the trace listeners
/// (`Trace.TraceInformation`, `TraceWarning`, `TraceError`, `Debug.WriteLine`) and to the console.
/// The port writes it to the log sink of the framework ([`Logger`], area `LogAction`, at the level
/// of [`level`](Self::level)) and to the standard output. The message is formatted as
/// `string.Format(message, argument)` does with one argument (`{0}`, its alignment, `{{` and
/// `}}`); the format string of the item (`{0:N2}`) is not applied, the text form of the argument
/// is used.
#[repr(C)]
pub struct LogAction {
    base: StyledElementAction,
}

ferro_class!(LogAction: StyledElementAction);
ferro_class_info!(LogAction { new: LogAction::new });
ferroui_base::ferro_impl_classes!(LogAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl LogAction {
        /// Identifies the `Message` property.
        pub fn message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<LogAction, _>("Message", None)
        }

        /// Identifies the `Argument` property.
        pub fn argument_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<LogAction, _>("Argument", None)
        }

        /// Identifies the `Level` property.
        pub fn level_property() -> StyledProperty<LogActionLevel> {
            FerroProperty::register::<LogAction, _>("Level", LogActionLevel::Info)
        }
    }
}

impl StyledElementActionImpl for LogAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return None;
        }

        let message = this.message();
        let argument = this.argument();

        let output = match message {
            None => match &argument {
                Some(argument) => ValueTypes::to_display_string(Some(argument)),
                None => "LogAction invoked with no message or argument.".to_string(),
            },
            Some(message) => match &argument {
                // A malformed format (`FormatException`) logs the message as it is.
                Some(argument) => {
                    format_message(&message, &ValueTypes::to_display_string(Some(argument))).unwrap_or(message)
                }
                None => message,
            },
        };

        let level = this.level();
        let text = format!("{CATEGORY}: {output}");

        let log_level = match level {
            LogActionLevel::Info => LogEventLevel::Information,
            LogActionLevel::Warning => LogEventLevel::Warning,
            LogActionLevel::Error => LogEventLevel::Error,
            LogActionLevel::Debug => LogEventLevel::Debug,
        };
        if let Some(logger) = Logger::try_get(log_level, CATEGORY) {
            logger.log_with_values(None, "{Message}", &[&text]);
        }

        // Also write to the console for visibility in some environments.
        println!("[{level}] {text}");

        Some(Rc::new(true))
    }
}

impl LogAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the message format string.
    pub fn message(&self) -> Option<String> {
        self.get_value(Self::message_property())
    }

    pub fn set_message(&self, value: Option<String>) {
        self.set_value(Self::message_property(), value)
    }

    /// Gets or sets an optional argument to format the message with.
    pub fn argument(&self) -> Option<BoxedValue> {
        self.get_value(Self::argument_property())
    }

    pub fn set_argument(&self, value: Option<BoxedValue>) {
        self.set_value(Self::argument_property(), value)
    }

    /// Gets or sets the log level.
    pub fn level(&self) -> LogActionLevel {
        self.get_value(Self::level_property())
    }

    pub fn set_level(&self, value: LogActionLevel) {
        self.set_value(Self::level_property(), value)
    }
}

/// `string.Format(message, argument)` with one argument: `{0}` (with an optional alignment and an
/// ignored format string) is the argument, `{{` and `}}` are braces. `None` for a malformed
/// format or an item other than `{0}` (the `FormatException` of the reference).
fn format_message(message: &str, argument: &str) -> Option<String> {
    let mut output = String::with_capacity(message.len() + argument.len());
    let mut chars = message.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' => {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    output.push('{');
                    continue;
                }

                let mut item = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(c) => item.push(c),
                        None => return None,
                    }
                }

                let index_and_alignment = match item.split_once(':') {
                    Some((index_and_alignment, _format)) => index_and_alignment,
                    None => item.as_str(),
                };
                let (index, alignment) = match index_and_alignment.split_once(',') {
                    Some((index, alignment)) => (index, Some(alignment)),
                    None => (index_and_alignment, None),
                };

                let index = index.trim_end();
                if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) || index.parse::<usize>().ok()? != 0 {
                    return None;
                }

                let width = match alignment {
                    Some(alignment) => alignment.trim().parse::<i32>().ok()?,
                    None => 0,
                };
                let length = argument.chars().count();
                let padding = (width.unsigned_abs() as usize).saturating_sub(length);
                if width > 0 {
                    output.extend(std::iter::repeat(' ').take(padding));
                    output.push_str(argument);
                } else {
                    output.push_str(argument);
                    output.extend(std::iter::repeat(' ').take(padding));
                }
            }
            '}' => {
                if chars.peek() == Some(&'}') {
                    chars.next();
                    output.push('}');
                } else {
                    return None;
                }
            }
            c => output.push(c),
        }
    }

    Some(output)
}
