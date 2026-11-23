use crate::{literal::Literal, registered::Class};
use ferroui_designer_animation::BehaviorModel;
use ferroui_designer_core::{SourceSpan, TextPatch};
use std::collections::BTreeMap;
use std::fmt::Write;

pub(crate) const MAX_DEPTH: usize = 96;
const INDENT: &str = "    ";

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Element {
    pub(crate) class: Class,
    pub(crate) properties: BTreeMap<String, String>,
    pub(crate) children: Vec<Element>,
}

impl Element {
    pub(crate) fn new(class: Class) -> Self {
        Self {
            class,
            properties: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    pub(crate) fn xaml(&self) -> String {
        crate::markup::serialize(self, &[], &BTreeMap::new(), &BTreeMap::new(), &Default::default()).source
    }

    pub(crate) fn rsx_body(&self) -> String {
        self.rsx_with_behaviors(&[])
    }

    pub(crate) fn rsx_with_behaviors(&self, behaviors: &[Vec<BehaviorModel>]) -> String {
        let mut output = String::new();
        self.write_rsx(&mut output, 0, &mut behaviors.iter());
        output
    }

    fn write_rsx(&self, output: &mut String, depth: usize, models: &mut std::slice::Iter<'_, Vec<BehaviorModel>>) {
        let pad = INDENT.repeat(depth);
        let _ = write!(output, "{pad}{} {{", self.class.rust_name());
        for (name, value) in &self.properties {
            let code = self.rust_value(name, value).expect("validated literal");
            let _ = write!(
                output,
                "\n{pad}{INDENT}{}: {code},",
                self.class.setter(name)
            );
        }
        let behaviors = models.next().filter(|models| !models.is_empty());
        if let Some(behaviors) = behaviors {
            let _ = write!(output, "\n{pad}{INDENT}behaviors: [");
            let indent = format!("{pad}{INDENT}{INDENT}");
            for model in behaviors {
                let expression = crate::behaviors::declaration(model, &indent, "\n").expect("normalized behavior");
                let _ = write!(output, "\n{indent}{expression},");
            }
            let _ = write!(output, "\n{pad}{INDENT}],");
        }
        for child in &self.children {
            output.push('\n');
            child.write_rsx(output, depth + 1, models);
        }
        if behaviors.is_some() || !self.properties.is_empty() || !self.children.is_empty() {
            let _ = write!(output, "\n{pad}");
        }
        output.push('}');
    }

    pub(crate) fn rust_value(&self, name: &str, value: &str) -> Option<String> {
        self.class.literal(name)?.rust(value)
    }

    pub(crate) fn changes<'a>(&'a self, target: &'a Self) -> Option<Vec<ValueChange<'a>>> {
        let mut changes = Vec::new();
        self.compare(target, &mut 0, &mut changes)
            .then_some(changes)
    }

    fn compare<'a>(
        &'a self,
        target: &'a Self,
        index: &mut usize,
        changes: &mut Vec<ValueChange<'a>>,
    ) -> bool {
        if self.class != target.class || self.children.len() != target.children.len() {
            return false;
        }
        for (name, next) in &target.properties {
            if self.properties.get(name) != Some(next) {
                changes.push(ValueChange {
                    index: *index,
                    name,
                    value: Some(next),
                });
            }
        }
        changes.extend(
            self.properties
                .keys()
                .filter(|name| !target.properties.contains_key(*name))
                .map(|name| ValueChange { index: *index, name, value: None }),
        );
        self.children
            .iter()
            .zip(&target.children)
            .all(|(current, next)| {
                *index += 1;
                current.compare(next, index, changes)
            })
    }
}

pub(crate) struct ValueChange<'a> {
    pub(crate) index: usize,
    pub(crate) name: &'a str,
    pub(crate) value: Option<&'a str>,
}

pub(crate) fn replacement(source: &str, span: SourceSpan, output: &str) -> TextPatch {
    let line = source[..span.start].rsplit('\n').next().unwrap_or("");
    let indent: String = line
        .chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .collect();
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    TextPatch {
        span,
        replacement: output.replace('\n', &format!("{newline}{indent}")),
    }
}

pub(crate) fn escape_xml(value: &str, quote: Option<char>) -> String {
    let mut result = String::new();
    for character in value.chars() {
        result.push_str(match character {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' if quote == Some('"') => "&quot;",
            '\'' if quote == Some('\'') => "&apos;",
            '\r' => "&#xD;",
            '\n' if quote.is_some() => "&#xA;",
            '\t' if quote.is_some() => "&#x9;",
            _ => {
                result.push(character);
                continue;
            }
        });
    }
    result
}

pub(crate) fn inline_content(value: &str) -> Option<String> {
    Literal::Text.rust(value)?;
    (!value.is_empty()
        && value.trim() == value
        && !value.starts_with('{')
        && !value.contains(['\n', '\r', '\t']))
    .then(|| value.to_owned())
}
