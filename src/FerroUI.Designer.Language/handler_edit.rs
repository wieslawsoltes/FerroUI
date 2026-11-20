use crate::{LanguageError, Result};
use ferroui_build_scan::scanner::{ClassRegistration, DeclarationList};
use ferroui_designer_core::{SourceSpan, TextPatch};

pub(crate) fn insert_registration(
    source: &str,
    registration: &ClassRegistration,
    class: &str,
    name: &str,
    parameters: &[String],
) -> Result<Vec<TextPatch>> {
    let list = registration
        .methods
        .as_ref()
        .or(registration.markup.as_ref())
        .unwrap_or(&registration.declaration);
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let line_start = source[..list.close].rfind('\n').map_or(0, |line| line + 1);
    let close_prefix = &source[line_start..list.close];
    let own_line = close_prefix.bytes().all(|ch| ch == b' ' || ch == b'\t');
    let closing = indentation(close_prefix);
    let existing = list
        .first
        .map(|start| line_prefix(source, start))
        .filter(|prefix| {
            prefix.bytes().all(|ch| ch == b' ' || ch == b'\t')
                && prefix.starts_with(closing)
                && prefix.len() > closing.len()
        });
    let step = existing.map_or(
        if closing.contains('\t') { "\t" } else { "    " },
        |indent| &indent[closing.len()..],
    );
    let mut lines = Vec::new();
    let mut depth = 1;
    if registration.markup.is_none() {
        lines.push((depth, "markup: {".into()));
        depth += 1;
    }
    if registration.methods.is_none() {
        lines.push((depth, "methods: [".into()));
        depth += 1;
    }
    let arguments = (0..parameters.len())
        .map(|index| match (parameters.len(), index) {
            (2, 0) => "_sender".into(),
            (2, 1) => "_event".into(),
            (_, index) => format!("_argument{}", index + 1),
        })
        .collect::<Vec<String>>();
    let declaration = format!("fn {name}({}) =>", parameters.join(", "));
    if declaration.len() + closing.len() + depth * step.len() <= 100 {
        lines.push((depth, declaration));
    } else {
        lines.push((depth, format!("fn {name}(")));
        lines.extend(parameters.iter().map(|ty| (depth + 1, format!("{ty},"))));
        lines.push((depth, ") =>".into()));
    }
    lines.push((depth + 1, "|".into()));
    lines.push((depth + 2, format!("_this: &::ferroui_base::Ref<{class}>,")));
    for (argument, parameter) in arguments.iter().zip(parameters) {
        lines.push((depth + 2, format!("{argument}: {parameter},")));
    }
    lines.push((depth + 1, "| {".into()));
    lines.push((depth + 2, "// TODO: Handle this event.".into()));
    lines.push((depth + 1, "},".into()));
    if registration.methods.is_none() {
        depth -= 1;
        lines.push((depth, "],".into()));
    }
    if registration.markup.is_none() {
        lines.push((depth - 1, "},".into()));
    }

    let mut body = String::new();
    if !own_line {
        body.push_str(newline);
    }
    for (depth, line) in lines {
        body.push_str(closing);
        body.push_str(&step.repeat(depth));
        body.push_str(&line);
        body.push_str(newline);
    }
    if !own_line {
        body.push_str(closing);
    }
    let at = if own_line { line_start } else { list.close };
    append(list, at, body)
}

fn append(list: &DeclarationList, at: usize, mut body: String) -> Result<Vec<TextPatch>> {
    let mut edits = Vec::new();
    if let Some(comma) = list.needs_comma_after {
        if comma > at {
            return Err(LanguageError("Invalid Rust insertion anchor".into()));
        }
        if comma == at {
            body.insert(0, ',');
        } else {
            edits.push(insertion(comma, ",".into()));
        }
    }
    edits.push(insertion(at, body));
    Ok(edits)
}

fn insertion(offset: usize, replacement: String) -> TextPatch {
    TextPatch {
        span: SourceSpan::new(offset, offset),
        replacement,
    }
}

fn line_prefix(source: &str, at: usize) -> &str {
    source[..at].rsplit('\n').next().unwrap_or("")
}

fn indentation(line: &str) -> &str {
    &line[..line
        .bytes()
        .take_while(|ch| matches!(ch, b' ' | b'\t'))
        .count()]
}
