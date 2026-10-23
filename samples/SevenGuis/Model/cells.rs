//! The formulas of the cells task: what a cell holds, how a formula is read
//! and computed. A cell holds a number, text, or a formula after `=`:
//!
//! ```text
//! =A1 + B2 * 2
//! =sum(A0:A9) / count(A0:A9)
//! =max(B1, C1, 10) - (A3 - 1)
//! ```
//!
//! Cells are named by a column `A` to `Z` and a row `0` to `99`. Ranges
//! (`A0:B3`) are arguments of the functions `sum`, `prod`, `avg`, `min`,
//! `max` and `count`. A formula computes its value from the values of the
//! cells it names, which the caller looks up: the user interface keeps each
//! value in a memo, so a cell is computed again when a cell it names changes.

/// The number of columns, `A` to `Z`.
pub const COLUMNS: usize = 26;
/// The number of rows, `0` to `99`.
pub const ROWS: usize = 100;

/// A cell: its column (0 is `A`) and its row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellRef {
    /// The column, 0 to 25.
    pub column: usize,
    /// The row, 0 to 99.
    pub row: usize,
}

impl CellRef {
    /// The name of the column (`A`).
    pub fn column_name(column: usize) -> String {
        char::from(b'A' + column as u8).to_string()
    }
}

impl std::fmt::Display for CellRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::column_name(self.column), self.row)
    }
}

/// The value of a cell.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Nothing.
    Empty,
    /// A number.
    Number(f64),
    /// Text.
    Text(String),
    /// A formula that cannot be computed, and why.
    Error(String),
}

impl Value {
    /// The value as a number in a computation: empty is 0, text is an error.
    fn number(&self) -> Result<f64, String> {
        match self {
            Value::Empty => Ok(0.0),
            Value::Number(number) => Ok(*number),
            Value::Text(text) => Err(format!("`{text}` is not a number")),
            Value::Error(error) => Err(error.clone()),
        }
    }

    /// The text of the value in its cell.
    pub fn display(&self) -> String {
        match self {
            Value::Empty => String::new(),
            Value::Number(number) if number.fract() == 0.0 && number.abs() < 1e15 => format!("{}", *number as i64),
            Value::Number(number) => format!("{number:.4}").trim_end_matches('0').trim_end_matches('.').to_string(),
            Value::Text(text) => text.clone(),
            Value::Error(error) => format!("#{error}"),
        }
    }
}

/// What a cell holds, read.
#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    /// Nothing.
    Empty,
    /// A number.
    Number(f64),
    /// Text.
    Text(String),
    /// A formula.
    Formula(Expr),
    /// A formula that does not parse, and why.
    Invalid(String),
}

/// An expression of a formula.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A number.
    Number(f64),
    /// The value of a cell.
    Cell(CellRef),
    /// A range of cells, only as an argument of a function.
    Range(CellRef, CellRef),
    /// A negation.
    Negate(Box<Expr>),
    /// An operation of two expressions: `+`, `-`, `*`, `/`.
    Binary(char, Box<Expr>, Box<Expr>),
    /// A function of its arguments.
    Call(String, Vec<Expr>),
}

/// Reads what a cell holds from its text.
pub fn parse(text: &str) -> Content {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Content::Empty;
    }
    if let Some(formula) = trimmed.strip_prefix('=') {
        let mut parser = Parser { chars: formula.chars().collect(), position: 0 };
        return match parser.expression().and_then(|expr| parser.end().map(|()| expr)) {
            Ok(expr) => Content::Formula(expr),
            Err(error) => Content::Invalid(error),
        };
    }
    match trimmed.parse::<f64>() {
        Ok(number) => Content::Number(number),
        Err(_) => Content::Text(trimmed.to_string()),
    }
}

impl Content {
    /// The value, with the values of the cells it names from `lookup`.
    pub fn evaluate(&self, lookup: &dyn Fn(CellRef) -> Value) -> Value {
        match self {
            Content::Empty => Value::Empty,
            Content::Number(number) => Value::Number(*number),
            Content::Text(text) => Value::Text(text.clone()),
            Content::Invalid(error) => Value::Error(error.clone()),
            Content::Formula(expr) => match expr.evaluate(lookup) {
                Ok(number) => Value::Number(number),
                Err(error) => Value::Error(error),
            },
        }
    }
}

impl Expr {
    fn evaluate(&self, lookup: &dyn Fn(CellRef) -> Value) -> Result<f64, String> {
        match self {
            Expr::Number(number) => Ok(*number),
            Expr::Cell(cell) => lookup(*cell).number(),
            Expr::Range(..) => Err(String::from("a range is an argument of a function")),
            Expr::Negate(expr) => Ok(-expr.evaluate(lookup)?),
            Expr::Binary(operator, left, right) => {
                let (left, right) = (left.evaluate(lookup)?, right.evaluate(lookup)?);
                match operator {
                    '+' => Ok(left + right),
                    '-' => Ok(left - right),
                    '*' => Ok(left * right),
                    _ if right == 0.0 => Err(String::from("division by zero")),
                    _ => Ok(left / right),
                }
            }
            Expr::Call(name, args) => {
                let mut values = Vec::new();
                for arg in args {
                    match arg {
                        Expr::Range(from, to) => {
                            for column in from.column.min(to.column)..=from.column.max(to.column) {
                                for row in from.row.min(to.row)..=from.row.max(to.row) {
                                    match lookup(CellRef { column, row }) {
                                        Value::Empty => {}
                                        value => values.push(value.number()?),
                                    }
                                }
                            }
                        }
                        arg => values.push(arg.evaluate(lookup)?),
                    }
                }
                let count = values.len() as f64;
                match name.as_str() {
                    "sum" => Ok(values.iter().sum()),
                    "prod" => Ok(values.iter().product()),
                    "count" => Ok(count),
                    "avg" if count == 0.0 => Err(String::from("avg of nothing")),
                    "avg" => Ok(values.iter().sum::<f64>() / count),
                    "min" => values.into_iter().reduce(f64::min).ok_or_else(|| String::from("min of nothing")),
                    "max" => values.into_iter().reduce(f64::max).ok_or_else(|| String::from("max of nothing")),
                    other => Err(format!("no function `{other}`")),
                }
            }
        }
    }
}

struct Parser {
    chars: Vec<char>,
    position: usize,
}

impl Parser {
    fn skip_spaces(&mut self) {
        while self.chars.get(self.position).is_some_and(|c| c.is_whitespace()) {
            self.position += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_spaces();
        self.chars.get(self.position).copied()
    }

    fn end(&mut self) -> Result<(), String> {
        match self.peek() {
            None => Ok(()),
            Some(c) => Err(format!("unexpected `{c}`")),
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        if self.peek() == Some(expected) {
            self.position += 1;
            Ok(())
        } else {
            Err(format!("expected `{expected}`"))
        }
    }

    fn expression(&mut self) -> Result<Expr, String> {
        let mut left = self.term()?;
        while let Some(operator @ ('+' | '-')) = self.peek() {
            self.position += 1;
            left = Expr::Binary(operator, Box::new(left), Box::new(self.term()?));
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut left = self.factor()?;
        while let Some(operator @ ('*' | '/')) = self.peek() {
            self.position += 1;
            left = Expr::Binary(operator, Box::new(left), Box::new(self.factor()?));
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some('-') => {
                self.position += 1;
                Ok(Expr::Negate(Box::new(self.factor()?)))
            }
            Some('(') => {
                self.position += 1;
                let expr = self.expression()?;
                self.expect(')')?;
                Ok(expr)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            Some(c) if c.is_ascii_alphabetic() => {
                let start = self.position;
                while self.chars.get(self.position).is_some_and(|c| c.is_ascii_alphanumeric()) {
                    self.position += 1;
                }
                let word: String = self.chars[start..self.position].iter().collect();
                if self.peek() == Some('(') {
                    self.position += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(')') {
                        loop {
                            args.push(self.argument()?);
                            if self.peek() == Some(',') {
                                self.position += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(')')?;
                    return Ok(Expr::Call(word.to_lowercase(), args));
                }
                cell_ref(&word).map(Expr::Cell).ok_or_else(|| format!("`{word}` is not a cell"))
            }
            Some(c) => Err(format!("unexpected `{c}`")),
            None => Err(String::from("the formula ends early")),
        }
    }

    fn argument(&mut self) -> Result<Expr, String> {
        let start = self.position;
        let expr = self.expression()?;
        if let (Expr::Cell(from), Some(':')) = (&expr, self.peek()) {
            self.position += 1;
            let to = self.expression()?;
            return match to {
                Expr::Cell(to) => Ok(Expr::Range(*from, to)),
                _ => {
                    self.position = start;
                    Err(String::from("a range ends with a cell"))
                }
            };
        }
        Ok(expr)
    }

    fn number(&mut self) -> Result<Expr, String> {
        let start = self.position;
        while self.chars.get(self.position).is_some_and(|c| c.is_ascii_digit() || *c == '.') {
            self.position += 1;
        }
        let text: String = self.chars[start..self.position].iter().collect();
        text.parse::<f64>().map(Expr::Number).map_err(|_| format!("`{text}` is not a number"))
    }
}

/// A cell named like `B12`.
pub fn cell_ref(name: &str) -> Option<CellRef> {
    let mut chars = name.chars();
    let column = chars.next()?.to_ascii_uppercase();
    if !column.is_ascii_uppercase() {
        return None;
    }
    let row: usize = chars.as_str().parse().ok()?;
    (row < ROWS).then_some(CellRef { column: (column as u8 - b'A') as usize, row })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sheet(cells: &[(&str, &str)]) -> impl Fn(CellRef) -> Value {
        let contents: HashMap<CellRef, Content> =
            cells.iter().map(|(name, text)| (cell_ref(name).unwrap(), parse(text))).collect();
        move |cell| evaluate_in(&contents, cell, 0)
    }

    fn evaluate_in(contents: &HashMap<CellRef, Content>, cell: CellRef, depth: usize) -> Value {
        if depth > 50 {
            return Value::Error(String::from("cycle"));
        }
        match contents.get(&cell) {
            Some(content) => content.evaluate(&|other| evaluate_in(contents, other, depth + 1)),
            None => Value::Empty,
        }
    }

    #[test]
    fn formulas_compute_from_the_cells_they_name() {
        let lookup = sheet(&[("A0", "1"), ("A1", "2"), ("A2", "3"), ("B0", "=sum(A0:A2) * 2"), ("B1", "=B0 / (A1 + 1)")]);
        assert_eq!(lookup(cell_ref("B0").unwrap()), Value::Number(12.0));
        assert_eq!(lookup(cell_ref("B1").unwrap()), Value::Number(4.0));
        assert_eq!(parse("=max(1, 2, -3)").evaluate(&lookup), Value::Number(2.0));
        assert_eq!(parse("=avg(A0:A2)").evaluate(&lookup).display(), "2");
    }

    #[test]
    fn errors_say_what_is_wrong() {
        let lookup = sheet(&[("A0", "text")]);
        assert_eq!(parse("=A0 + 1").evaluate(&lookup), Value::Error(String::from("`text` is not a number")));
        assert_eq!(parse("=1 +").evaluate(&lookup), Value::Error(String::from("the formula ends early")));
        assert_eq!(parse("=1/0").evaluate(&lookup).display(), "#division by zero");
        assert_eq!(parse("hello"), Content::Text(String::from("hello")));
    }
}
