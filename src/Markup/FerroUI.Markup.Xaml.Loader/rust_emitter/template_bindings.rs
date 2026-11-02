//! Consecutive TemplateBinding assignments on one receiver use a slice of property pairs.
//! Every pair is a separate constant so an unused template cannot retain another pair.
use std::collections::HashMap;
use std::fmt::Write;

use super::style_setters::StyleSetters;

struct Site<'a> {
    object: &'a str,
    source: &'a str,
    target: &'a str,
    line: i32,
    column: i32,
}

impl<'a> Site<'a> {
    /// Only the scalar operation emitted for a resolved TemplateBinding can enter a run.
    fn read(line: &'a str) -> Option<Self> {
        let call = line
            .strip_prefix("    rt::apply_template_binding(")?
            .strip_suffix(")?;")?;
        let (object, args) = call.split_once(", ")?;
        let (source, args) = args.split_once(", || ")?;
        let (target, location) = args.split_once(", ")?;
        let (line, column) = location.split_once(", ")?;
        Some(Self {
            object,
            source,
            target,
            line: line.parse().ok()?,
            column: column.parse().ok()?,
        })
    }
}

#[derive(Default)]
pub(super) struct TemplateBindings {
    getter_ids: HashMap<String, usize>,
    getters: Vec<String>,
    pair_ids: HashMap<(usize, usize), usize>,
    pairs: Vec<(usize, usize)>,
    runs: Vec<Vec<(usize, i32, i32)>>,
}

impl TemplateBindings {
    fn getter(&mut self, expression: &str) -> usize {
        if let Some(&id) = self.getter_ids.get(expression) {
            return id;
        }
        let id = self.getters.len();
        self.getters.push(expression.to_owned());
        self.getter_ids.insert(expression.to_owned(), id);
        id
    }

    fn pair(&mut self, site: &Site<'_>) -> usize {
        let pair = (self.getter(site.source), self.getter(site.target));
        *self.pair_ids.entry(pair).or_insert_with(|| {
            self.pairs.push(pair);
            self.pairs.len() - 1
        })
    }

    /// Rows carry their own source positions. An intervening statement or different
    /// receiver ends the run before the next getter is evaluated.
    pub(super) fn lower(&mut self, document: &str, lines: &mut Vec<String>) {
        let mut i = 0;
        while i < lines.len() {
            let Some(first) = Site::read(&lines[i]) else {
                i += 1;
                continue;
            };
            let receiver = first.object.to_owned();
            let mut end = i + 1;
            let mut rows = vec![first];
            loop {
                let mut at = end;
                while lines
                    .get(at)
                    .is_some_and(|line| line.trim_start().starts_with("// "))
                {
                    at += 1;
                }
                let Some(next) = lines.get(at).and_then(|line| Site::read(line)) else {
                    break;
                };
                if next.object != receiver {
                    break;
                }
                rows.push(next);
                end = at + 1;
            }
            if rows.len() == 1 {
                i = end;
                continue;
            }
            let id = self.runs.len();
            let rows = rows
                .iter()
                .map(|site| (self.pair(site), site.line, site.column))
                .collect();
            self.runs.push(rows);
            let call = format!(
                "    rt::bind_template_rows({receiver}, self::__{document}_bindings::R{id})?;"
            );
            lines.splice(i..end, [call]);
            i += 1;
        }
    }

    pub(super) fn write(&self, document: &str, styles: &StyleSetters, source: &mut String) {
        if self.pairs.is_empty() {
            return;
        }
        writeln!(source, "mod __{document}_bindings {{\n    use super::*;").unwrap();
        for (id, expression) in self.getters.iter().enumerate() {
            if let Some(existing) = styles.getter_name(expression) {
                writeln!(source, "    use super::{existing} as g{id};").unwrap();
            } else {
                writeln!(
                    source,
                    "    fn g{id}() -> &'static ::ferroui_base::FerroProperty {{ {expression} }}"
                )
                .unwrap();
            }
        }
        for (id, (src, dst)) in self.pairs.iter().enumerate() {
            writeln!(
                source,
                "    const P{id}: rt::TemplateBindingProperties = (g{src}, g{dst});"
            )
            .unwrap();
        }
        for (id, rows) in self.runs.iter().enumerate() {
            writeln!(
                source,
                "    pub(super) const R{id}: &[rt::TemplateBindingRow] = &["
            )
            .unwrap();
            for (pair, line, column) in rows {
                writeln!(source, "        (P{pair}, {line}, {column}),").unwrap();
            }
            source.push_str("    ];\n");
        }
        source.push_str("}\n\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_stop_at_statements_and_receivers_and_map_each_source_row() {
        let bind = |receiver: &str, line: i32| {
            format!(
            "    rt::apply_template_binding({receiver}, rt::property(::Owner::src()), || rt::property(::Owner::dst()), {line}, 5)?;"
        )
        };
        let mut lines = vec![
            bind("a", 2),
            "    // foo.xaml(3,5) Background".into(),
            bind("a", 3),
            "    flush();".into(),
            bind("a", 4),
            bind("b", 5),
        ];
        let mut program = TemplateBindings::default();
        program.lower("build_foo", &mut lines);
        assert_eq!(
            lines[0],
            "    rt::bind_template_rows(a, self::__build_foo_bindings::R0)?;"
        );
        assert_eq!(lines[1..], ["    flush();", &bind("a", 4), &bind("b", 5)]);
        assert_eq!(program.pairs, [(0, 1)]);
        assert_eq!(program.runs[0], [(0, 2, 5), (0, 3, 5)]);

        let mut source = String::new();
        program.write("build_foo", &StyleSetters::default(), &mut source);
        let map = super::super::compiled::position_map(
            &source,
            &[("build_foo".into(), "foo.xaml".into())],
        );
        for line in [2, 3] {
            let row = source
                .lines()
                .position(|s| s.contains(&format!("(P0, {line}, 5),")))
                .unwrap()
                + 1;
            assert!(map.contains(&format!(r#""file_lines": [{row}, {row}], "document": "foo.xaml", "line": {line}, "position": 5"#)));
        }
    }
}
