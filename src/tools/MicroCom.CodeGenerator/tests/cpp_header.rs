//! Checks the C++ generator against a header produced by the original
//! MicroCom generator.
//!
//! `fixtures/ferro-native.5378af03f1.h` is the header the reference generator
//! emitted for `fixtures/frn.5378af03f1.idl` (an older revision of the IDL —
//! the only one for which reference output was available; both files went
//! through the same identifier rename as the rest of the native sources).
//! The comparison ignores whitespace only.

use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Whitespace-insensitive token stream: identifiers/numbers stay whole,
/// every other non-space character is its own token.
fn tokens(s: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (line_no, line) in s.lines().enumerate() {
        let mut cur = String::new();
        for c in line.chars() {
            if c.is_ascii_alphanumeric() || c == '_' {
                cur.push(c);
                continue;
            }
            if !cur.is_empty() {
                out.push((line_no + 1, std::mem::take(&mut cur)));
            }
            if !c.is_whitespace() {
                out.push((line_no + 1, c.to_string()));
            }
        }
        if !cur.is_empty() {
            out.push((line_no + 1, cur));
        }
    }
    out
}

#[test]
fn header_matches_reference_generator_output() {
    let idl = microcom_codegen::parse(&fixture("frn.5378af03f1.idl")).expect("parse");
    let generated = microcom_codegen::cpp::generate(&idl).expect("generate");
    let expected = fixture("ferro-native.5378af03f1.h");

    let got = tokens(&generated);
    let want = tokens(&expected);
    for (i, (g, e)) in got.iter().zip(want.iter()).enumerate() {
        assert_eq!(
            g.1, e.1,
            "token #{i} differs: generated line {} vs reference line {}",
            g.0, e.0
        );
    }
    assert_eq!(got.len(), want.len(), "token count differs");
    assert!(want.len() > 5000, "reference header unexpectedly small");
}

/// The current IDL must parse and generate; its declarations are a superset
/// change of the fixture, so spot-check a few of the newer items.
#[test]
fn current_idl_generates() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../FerroUI.Native/frn.idl");
    let Ok(src) = std::fs::read_to_string(&path) else {
        eprintln!("skipping: {} not found", path.display());
        return;
    };
    let idl = microcom_codegen::parse(&src).expect("parse");
    let h = microcom_codegen::cpp::generate(&idl).expect("generate");
    assert!(h.starts_with("#pragma once\n#include \"com.h\""));
    assert!(h.contains("COMINTERFACE(IFerroNativeFactory, 809c652e, 7396, 11d2, 97, 71, 00, a0, c9, b4, d5, 0c) : IUnknown"));
    assert!(h.contains(": virtual IFrnWindowBase"));
    assert!(h.contains("virtual FrnShutdownReply TryShutdown (\n        bool isOSShutdown\n    ) = 0;"));
    let rs = microcom_codegen::rust::generate(&idl, &Default::default()).expect("rust");
    assert!(rs.contains("pub trait IFrnWindowEventsImpl: IFrnWindowBaseEventsImpl"));
}
