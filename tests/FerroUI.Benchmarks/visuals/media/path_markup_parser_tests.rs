//! Parsing the path data of a large path into a stream geometry.

use crate::harness::Registry;
use ferroui_base::media::{PathMarkupParser, StreamGeometry};
use ferroui_base::platform::IGeometryContext;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};

pub struct PathMarkupParserTests {
    /// Disposed when the benchmark is dropped.
    _app: UnitTestApplicationScope,
}

impl PathMarkupParserTests {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());

        Self { _app: app }
    }

    pub fn parse_large_path(&self) {
        const PATH_DATA: &str = concat!(
            "F1 M 16.6309 18.6563C 17.1309 8.15625 29.8809 14.1563 29.8809 14.1563C 30.8809 11.1563 34.1308 11.4063",
            " 34.1308 11.4063C 33.5 12 34.6309 13.1563 34.6309 13.1563C 32.1309 13.1562 31.1309 14.9062 31.1309 14.9",
            "062C 41.1309 23.9062 32.6309 27.9063 32.6309 27.9062C 24.6309 24.9063 21.1309 22.1562 16.6309 18.6563 Z",
            " M 16.6309 19.9063C 21.6309 24.1563 25.1309 26.1562 31.6309 28.6562C 31.6309 28.6562 26.3809 39.1562 18",
            ".3809 36.1563C 18.3809 36.1563 18 38 16.3809 36.9063C 15 36 16.3809 34.9063 16.3809 34.9063C 16.3809 34",
            ".9063 10.1309 30.9062 16.6309 19.9063 Z ",
        );

        let stream_geometry = StreamGeometry::new();

        let mut context = stream_geometry.open();
        let mut parser = PathMarkupParser::new(&mut context);
        parser.parse(PATH_DATA).expect("the path data is valid");
        parser.dispose();
        context.dispose();
    }
}

impl Default for PathMarkupParserTests {
    fn default() -> Self {
        Self::new()
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("visuals", "PathMarkupParserTests");
    class.benchmark("parse_large_path", "", PathMarkupParserTests::new, |b| b.parse_large_path());
}

#[cfg(test)]
mod tests {
    #[test]
    fn path_markup_parser_tests() {
        crate::harness::smoke_class(super::register, "PathMarkupParserTests");
    }
}
