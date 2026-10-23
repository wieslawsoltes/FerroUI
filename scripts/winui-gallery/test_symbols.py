import unittest

import symbol_map


MODEL = "public enum Symbol {\n Play = 0xE102,\n Print = 0xE749,\n}\n"
RENDERER = """WCHAR CSymbolIcon::ConvertSymbolValueToGlyph(const UINT32 symbolValue)
{
 switch (symbolValue) {
 case 0xE102: return static_cast<WCHAR>(0xE768);
 default: return static_cast<WCHAR>(symbolValue);
 }
}
"""


class SymbolMapTests(unittest.TestCase):
    def test_legacy_value_is_not_rendered_codepoint(self):
        self.assertEqual(symbol_map.extract(MODEL, RENDERER), [("Play", 0xE102, 0xE768), ("Print", 0xE749, 0xE749)])

    def test_duplicate_names_are_rejected(self):
        with self.assertRaises(ValueError):
            symbol_map.extract(MODEL.replace("Print", "Play"), RENDERER)

    def test_unknown_renderer_default_fails_closed(self):
        with self.assertRaises(ValueError):
            symbol_map.extract(MODEL, RENDERER.replace("static_cast<WCHAR>(symbolValue)", "0"))

    def test_rust_glyph_drift_is_rejected(self):
        source = """pub enum Symbol { Play = 0xE102,
}
ferro_markup_enum!(Symbol { Play }, { namespace: "FerroUI.Controls" });
pub const ALL: &'static [Self] = &[Self::Play];
Self::Play => '\\u{e102}'
"""
        with self.assertRaisesRegex(ValueError, "glyph"):
            symbol_map.verify_rust(source, [("Play", 0xE102, 0xE768)])


if __name__ == "__main__":
    unittest.main()
