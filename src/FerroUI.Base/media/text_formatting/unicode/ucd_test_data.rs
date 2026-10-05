//! Shared helpers of the unit tests of this module.
//!
//! The conformance tests are data driven from the test files of the Unicode
//! Character Database. Upstream downloads them at test time; here they are
//! read from the directory named by the environment variable
//! `FERROUI_UCD_DIR`, which must mirror
//! `https://www.unicode.org/Public/<version>/ucd/` for the Unicode version of
//! the generated tables (see `UnicodeDataSource::VERSION`):
//!
//! ```text
//! $FERROUI_UCD_DIR/BidiTest.txt
//! $FERROUI_UCD_DIR/BidiCharacterTest.txt
//! $FERROUI_UCD_DIR/auxiliary/GraphemeBreakTest.txt
//! $FERROUI_UCD_DIR/auxiliary/LineBreakTest.txt
//! $FERROUI_UCD_DIR/auxiliary/WordBreakTest.txt
//! $FERROUI_UCD_DIR/auxiliary/SentenceBreakTest.txt
//! ```
//!
//! (a file placed directly in the directory, without `auxiliary/`, is found
//! too). When the variable is unset or a file is missing the test prints a
//! note and passes without checking anything.

use std::path::PathBuf;

use super::unicode_data_source::UnicodeDataSource;

/// The name of the environment variable naming the UCD directory.
pub(super) const UCD_DIR_VARIABLE: &str = "FERROUI_UCD_DIR";

/// Reads a UCD test file (`relative_path` is relative to the `ucd/` directory),
/// or returns `None` after printing why the calling test is skipped.
pub(super) fn read_ucd_file(relative_path: &str) -> Option<String> {
    let Some(directory) = std::env::var_os(UCD_DIR_VARIABLE) else {
        eprintln!(
            "skipped: set {UCD_DIR_VARIABLE} to a copy of the Unicode {} ucd directory to run the {relative_path} conformance test",
            UnicodeDataSource::VERSION
        );
        return None;
    };

    let directory = PathBuf::from(directory);
    let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);

    for candidate in [directory.join(relative_path), directory.join(file_name)] {
        if let Ok(content) = std::fs::read_to_string(&candidate) {
            return Some(content);
        }
    }

    eprintln!("skipped: {relative_path} not found in {}", directory.display());
    None
}

/// Encodes a string as UTF-16 code units.
pub(super) fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// `string.Join(null, codePoints.Select(char.ConvertFromUtf32))`; surrogate
/// code points (which that call rejects) are kept as single code units.
pub(super) fn utf16_from_code_points(code_points: &[u32]) -> Vec<u16> {
    let mut text = Vec::with_capacity(code_points.len());

    for &code_point in code_points {
        match char::from_u32(code_point) {
            Some(c) => {
                let mut buffer = [0u16; 2];
                text.extend_from_slice(c.encode_utf16(&mut buffer));
            }
            None => text.push(code_point as u16),
        }
    }

    text
}

/// `Encoding.UTF32.GetString`: invalid code points become U+FFFD.
pub(super) fn utf16_from_utf32(code_points: &[u32]) -> Vec<u16> {
    let mut text = Vec::with_capacity(code_points.len());

    for &code_point in code_points {
        let c = char::from_u32(code_point).unwrap_or('\u{FFFD}');
        let mut buffer = [0u16; 2];
        text.extend_from_slice(c.encode_utf16(&mut buffer));
    }

    text
}

/// Parses one line of a `*BreakTest.txt` file (comment already removed) into
/// its code points and the indices (in code points) of the `÷` marks.
pub(super) fn read_break_test_line(line: &str) -> (Vec<u32>, Vec<usize>) {
    let mut code_points = Vec::new();
    let mut break_points = Vec::new();

    for token in line.split_whitespace() {
        match token {
            "×" => {}
            "÷" => break_points.push(code_points.len()),
            hex => code_points.push(u32::from_str_radix(hex, 16).expect("a hexadecimal code point")),
        }
    }

    (code_points, break_points)
}

/// The data lines of a `*BreakTest.txt` file:
/// `(line number, code points, break points in code points, rules comment)`.
pub(super) fn read_break_test_file(content: &str) -> Vec<(usize, Vec<u32>, Vec<usize>, String)> {
    let mut tests = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let mut segments = line.split('#');
        let data = segments.next().unwrap_or("");

        // Ignore blank/comment only lines
        if data.trim().is_empty() {
            continue;
        }

        let (code_points, break_points) = read_break_test_line(data.trim());
        tests.push((index + 1, code_points, break_points, segments.next().unwrap_or("").to_string()));
    }

    tests
}

/// Prints the outcome of a conformance test and fails it when a case failed.
pub(super) fn report(file: &str, passed: usize, failures: &[String]) {
    eprintln!("{file}: {passed} passed, {} failed", failures.len());

    for failure in failures.iter().take(20) {
        eprintln!("{failure}");
    }

    assert!(failures.is_empty(), "{file}: {} of {} cases failed", failures.len(), passed + failures.len());
}
