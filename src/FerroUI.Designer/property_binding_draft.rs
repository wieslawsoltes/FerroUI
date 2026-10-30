//! A local, source-preserving binding draft. The caller owns the document transaction.
use ferroui_designer_core::{MarkupExtension, MarkupValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyBindingSource {
    DataContext,
    ElementName(String),
    SelfValue,
    TemplatedParent,
    Ancestor {
        type_name: String,
        level: i32,
        logical_tree: bool,
    },
    Explicit(MarkupValue),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyBindingOption {
    Converter,
    ConverterParameter,
    ConverterCulture,
    StringFormat,
    FallbackValue,
    TargetNullValue,
}

impl PropertyBindingOption {
    pub fn name(self) -> &'static str {
        match self {
            Self::Converter => "Converter",
            Self::ConverterParameter => "ConverterParameter",
            Self::ConverterCulture => "ConverterCulture",
            Self::StringFormat => "StringFormat",
            Self::FallbackValue => "FallbackValue",
            Self::TargetNullValue => "TargetNullValue",
        }
    }
}

#[derive(Clone)]
struct Argument {
    name: String,
    value: MarkupValue,
    raw: String,
}

/// Edits only arguments that were explicitly changed. Unknown arguments, nested
/// extensions, aliases, quoting and spacing in every untouched argument are retained.
/// `binding_extension` must be the host's namespace-resolved framework extension;
/// an application extension with the same local name is deliberately not accepted.
#[derive(Clone)]
pub struct PropertyBindingDraft {
    opening: String,
    closing: String,
    relative_source_extension: Option<String>,
    arguments: Vec<Argument>,
}

impl PropertyBindingDraft {
    pub fn new(source: &str, binding_extension: &str) -> Result<Self, String> {
        let MarkupValue::Extension(extension) = MarkupValue::parse(source)? else {
            return Err("Enter a Binding markup expression".into());
        };
        if extension.name != binding_extension
            || !matches!(
                binding_extension
                    .rsplit(':')
                    .next()
                    .map(|name| name.strip_suffix("Extension").unwrap_or(name)),
                Some("Binding" | "CompiledBinding" | "ReflectionBinding")
            )
        {
            return Err(
                "This expression is not the framework binding in the current namespace scope"
                    .into(),
            );
        }
        let name_start = source
            .find(|ch: char| ch != '{' && !ch.is_whitespace())
            .unwrap();
        let name_end = name_start + extension.name.len();
        let close = source.rfind('}').unwrap();
        let pieces = split_arguments(&source[name_end..close]);
        if pieces.len() != extension.arguments.len() {
            return Err("This binding syntax requires editing the expression directly".into());
        }
        let mut arguments = Vec::new();
        for (argument, raw) in extension.arguments.into_iter().zip(pieces) {
            let name = argument.name.unwrap_or_else(|| "Path".into());
            if arguments
                .iter()
                .any(|previous: &Argument| previous.name == name)
            {
                return Err(format!("The binding has more than one {name} argument"));
            }
            arguments.push(Argument {
                name,
                value: argument.value,
                raw: raw.into(),
            });
        }
        Ok(Self {
            opening: source[..name_end].into(),
            closing: source[if arguments.is_empty() {
                name_end
            } else {
                close
            }..]
                .into(),
            relative_source_extension: None,
            arguments,
        })
    }

    pub fn empty(binding_extension: &str) -> Result<Self, String> {
        Self::new(&format!("{{{binding_extension}}}"), binding_extension)
    }

    /// Supply the independently namespace-resolved RelativeSource extension. A CLR
    /// alias for Binding does not necessarily expose RelativeSource in the same scope.
    /// None keeps existing expressions available but disables creating relative sources.
    pub fn with_relative_source_extension(mut self, name: Option<&str>) -> Self {
        self.relative_source_extension = name.map(str::to_owned);
        self
    }

    pub fn source(&self) -> String {
        format!(
            "{}{}{}",
            self.opening,
            self.arguments
                .iter()
                .map(|argument| argument.raw.as_str())
                .collect::<Vec<_>>()
                .join(","),
            self.closing
        )
    }

    pub fn argument(&self, name: &str) -> Option<&MarkupValue> {
        self.arguments
            .iter()
            .find(|argument| argument.name == name)
            .map(|argument| &argument.value)
    }

    pub fn literal(&self, name: &str) -> Option<&str> {
        match self.argument(name)? {
            MarkupValue::Literal(value) => Some(value),
            _ => None,
        }
    }

    pub fn set_path(&mut self, path: &str) {
        self.set_argument("Path", Some(MarkupValue::Literal(path.into())));
    }

    pub fn set_mode(&mut self, mode: &str) -> Result<(), String> {
        if !matches!(
            mode,
            "Default" | "OneWay" | "TwoWay" | "OneTime" | "OneWayToSource"
        ) {
            return Err("Choose a supported binding mode".into());
        }
        self.set_argument(
            "Mode",
            (mode != "Default").then(|| MarkupValue::Literal(mode.into())),
        );
        Ok(())
    }

    pub fn set_update_source_trigger(&mut self, trigger: &str) -> Result<(), String> {
        if !matches!(
            trigger,
            "Default" | "PropertyChanged" | "LostFocus" | "Explicit"
        ) {
            return Err("Choose a supported update-source trigger".into());
        }
        self.set_argument(
            "UpdateSourceTrigger",
            (trigger != "Default").then(|| MarkupValue::Literal(trigger.into())),
        );
        Ok(())
    }

    pub fn set_delay(&mut self, delay: Option<i32>) -> Result<(), String> {
        if delay.is_some_and(|delay| delay < 0) {
            return Err("Binding delay must be a nonnegative number of milliseconds".into());
        }
        self.set_argument(
            "Delay",
            delay.map(|delay| MarkupValue::Literal(delay.to_string())),
        );
        Ok(())
    }

    pub fn set_option(
        &mut self,
        option: PropertyBindingOption,
        value: Option<MarkupValue>,
    ) -> Result<(), String> {
        if option == PropertyBindingOption::Converter
            && matches!(value, Some(MarkupValue::Literal(_)))
        {
            return Err(
                "A converter requires an object expression, such as a StaticResource reference"
                    .into(),
            );
        }
        self.set_argument(option.name(), value);
        Ok(())
    }

    pub fn set_source(&mut self, source: PropertyBindingSource) -> Result<(), String> {
        let (name, value) = match source {
            PropertyBindingSource::DataContext => ("", None),
            PropertyBindingSource::ElementName(name) => {
                if name.trim().is_empty() || name.chars().any(char::is_whitespace) {
                    return Err("Choose or enter an element name without whitespace".into());
                }
                ("ElementName", Some(MarkupValue::Literal(name)))
            }
            PropertyBindingSource::SelfValue => {
                ("RelativeSource", Some(self.relative("Self", Vec::new())?))
            }
            PropertyBindingSource::TemplatedParent => (
                "RelativeSource",
                Some(self.relative("TemplatedParent", Vec::new())?),
            ),
            PropertyBindingSource::Ancestor {
                type_name,
                level,
                logical_tree,
            } => {
                if level < 1 {
                    return Err("Ancestor level must be at least 1".into());
                }
                let type_name = type_name.trim();
                if type_name.is_empty()
                    || type_name.chars().any(char::is_whitespace)
                    || type_name
                        .chars()
                        .any(|ch| matches!(ch, '{' | '}' | ',' | '=' | '\'' | '"'))
                    || type_name.split(':').count() > 2
                    || type_name.split(':').any(str::is_empty)
                {
                    return Err(
                        "Enter the ancestor's XAML type name, such as Grid or app:Editor".into(),
                    );
                }
                (
                    "RelativeSource",
                    Some(self.relative(
                        "FindAncestor",
                        vec![
                            ("AncestorType", type_name.into()),
                            ("AncestorLevel", level.to_string()),
                            (
                                "Tree",
                                if logical_tree { "Logical" } else { "Visual" }.into(),
                            ),
                        ],
                    )?),
                )
            }
            PropertyBindingSource::Explicit(value) => ("Source", Some(value)),
        };
        for previous in ["ElementName", "RelativeSource", "Source"] {
            if previous != name {
                self.set_argument(previous, None);
            }
        }
        if !name.is_empty() {
            self.set_argument(name, value);
        }
        Ok(())
    }

    /// Recognized source fields can be populated by an editor; unknown relative-source
    /// extensions remain exact source until the user explicitly chooses another source.
    pub fn source_specification(&self) -> Option<PropertyBindingSource> {
        if ["ElementName", "RelativeSource", "Source"]
            .into_iter()
            .filter(|name| self.argument(name).is_some())
            .count()
            > 1
        {
            return None;
        }
        if let Some(value) = self.argument("Source") {
            return Some(PropertyBindingSource::Explicit(value.clone()));
        }
        if let Some(value) = self.literal("ElementName") {
            return Some(PropertyBindingSource::ElementName(value.into()));
        }
        let Some(value) = self.argument("RelativeSource") else {
            return self
                .argument("ElementName")
                .is_none()
                .then_some(PropertyBindingSource::DataContext);
        };
        let MarkupValue::Extension(relative) = value else {
            return None;
        };
        if self.relative_source_extension.as_deref() != Some(relative.name.as_str()) {
            return None;
        }
        if relative.arguments.iter().any(|argument| {
            argument.name.as_deref().is_some_and(|name| {
                !matches!(name, "Mode" | "AncestorType" | "AncestorLevel" | "Tree")
            })
        }) {
            return None;
        }
        let literal = |name: &str| {
            relative
                .arguments
                .iter()
                .find(|argument| {
                    argument.name.as_deref() == Some(name)
                        || name == "Mode" && argument.name.is_none()
                })
                .and_then(|argument| match &argument.value {
                    MarkupValue::Literal(value) => Some(value.as_str()),
                    _ => None,
                })
        };
        match literal("Mode").unwrap_or("FindAncestor") {
            "Self" if relative.arguments.len() == 1 => Some(PropertyBindingSource::SelfValue),
            "TemplatedParent" if relative.arguments.len() == 1 => {
                Some(PropertyBindingSource::TemplatedParent)
            }
            "FindAncestor" => Some(PropertyBindingSource::Ancestor {
                type_name: literal("AncestorType")?.into(),
                level: literal("AncestorLevel").unwrap_or("1").parse().ok()?,
                logical_tree: match literal("Tree").unwrap_or("Visual") {
                    "Logical" => true,
                    "Visual" => false,
                    _ => return None,
                },
            }),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if ["ElementName", "RelativeSource", "Source"]
            .into_iter()
            .filter(|name| self.argument(name).is_some())
            .count()
            > 1
        {
            return Err("A binding can use only one of ElementName, RelativeSource and Source; choose one source".into());
        }
        if self.literal("Mode").is_some_and(|mode| {
            !matches!(
                mode,
                "Default" | "OneWay" | "TwoWay" | "OneTime" | "OneWayToSource"
            )
        }) {
            return Err("Choose a supported binding mode".into());
        }
        if self.literal("UpdateSourceTrigger").is_some_and(|trigger| {
            !matches!(
                trigger,
                "Default" | "PropertyChanged" | "LostFocus" | "Explicit"
            )
        }) {
            return Err("Choose a supported update-source trigger".into());
        }
        if self.literal("Delay").is_some_and(|delay| {
            delay.parse::<i32>().is_err() || delay.parse::<i32>().is_ok_and(|delay| delay < 0)
        }) {
            return Err("Binding delay must be a nonnegative number of milliseconds".into());
        }
        MarkupValue::parse(&self.source()).map(|_| ())
    }

    fn relative(&self, mode: &str, fields: Vec<(&str, String)>) -> Result<MarkupValue, String> {
        use ferroui_designer_core::MarkupArgument;
        Ok(MarkupValue::Extension(MarkupExtension {
            name: self.relative_source_extension.clone().ok_or("Declare a framework namespace exposing RelativeSource before choosing a relative binding source")?,
            arguments: std::iter::once(MarkupArgument {
                name: Some("Mode".into()),
                value: MarkupValue::Literal(mode.into()),
            })
            .chain(fields.into_iter().map(|(name, value)| MarkupArgument {
                name: Some(name.into()),
                value: MarkupValue::Literal(value),
            }))
            .collect(),
        }))
    }

    fn set_argument(&mut self, name: &str, value: Option<MarkupValue>) {
        if self.argument(name) == value.as_ref() {
            return;
        }
        let index = self
            .arguments
            .iter()
            .position(|argument| argument.name == name);
        match (index, value) {
            (Some(index), None) => {
                self.arguments.remove(index);
            }
            (Some(index), Some(value)) => {
                let argument = &mut self.arguments[index];
                let start = argument.raw.len() - argument.raw.trim_start().len();
                let end = argument.raw.trim_end().len();
                let value_start = top_level_equals(&argument.raw).map_or(start, |equals| {
                    equals + 1 + argument.raw[equals + 1..].len()
                        - argument.raw[equals + 1..].trim_start().len()
                });
                argument.raw = format!(
                    "{}{}{}",
                    &argument.raw[..value_start],
                    value_source(&value),
                    &argument.raw[end..]
                );
                argument.value = value;
            }
            (None, Some(value)) => self.arguments.push(Argument {
                name: name.into(),
                raw: format!(" {name}={}", value_source(&value)),
                value,
            }),
            (None, None) => {}
        }
        if let Some(first) = self.arguments.first_mut() {
            if !first.raw.starts_with(char::is_whitespace) {
                first.raw.insert(0, ' ');
            }
        }
    }
}

/// Quote a decoded value without confusing braces, commas or quotes with XAML syntax.
pub(super) fn value_source(value: &MarkupValue) -> String {
    match value {
        MarkupValue::Literal(value) => format!(
            "'{}{}'",
            if value.starts_with('{') { "{}" } else { "" },
            value.replace('\\', "\\\\").replace('\'', "\\'")
        ),
        MarkupValue::Extension(extension) => format!(
            "{{{}{}}}",
            extension.name,
            if extension.arguments.is_empty() {
                String::new()
            } else {
                format!(
                    " {}",
                    extension
                        .arguments
                        .iter()
                        .map(|argument| format!(
                            "{}{}",
                            argument
                                .name
                                .as_ref()
                                .map(|name| format!("{name}="))
                                .unwrap_or_default(),
                            value_source(&argument.value)
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        ),
    }
}

fn top_level_separators(source: &str, separator: char) -> Vec<usize> {
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0usize;
    let mut offsets = Vec::new();
    for (offset, ch) in source.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(current) = quote {
            if ch == current {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ch if ch == separator && depth == 0 => offsets.push(offset),
            _ => {}
        }
    }
    offsets
}
fn top_level_equals(source: &str) -> Option<usize> {
    top_level_separators(source, '=').first().copied()
}
fn split_arguments(source: &str) -> Vec<&str> {
    if source.trim().is_empty() {
        return Vec::new();
    }
    let mut start = 0;
    let mut parts = Vec::new();
    for offset in top_level_separators(source, ',') {
        parts.push(&source[start..offset]);
        start = offset + 1;
    }
    parts.push(&source[start..]);
    parts
}

#[cfg(test)]
#[path = "property_binding_draft_tests.rs"]
mod tests;
