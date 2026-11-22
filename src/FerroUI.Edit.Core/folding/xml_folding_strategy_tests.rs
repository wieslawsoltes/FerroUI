use super::*;

type Expected<'a> = (
    &'a str,
    &'a str,
    Option<usize>,
    &'a [(usize, usize, &'a str, &'a str)],
);

// Captured from the pinned reference editor. Each range has the title with
// attribute display off and on.
#[rustfmt::skip]
const CASES: &[Expected<'_>] = &[
    ("multiline", "<x a='😀'>\n<!-- hi\nthere -->\n<y/>\n</x>", None, &[(0, 38, "<x>", "<x a=\"😀\">"), (11, 28, "<!-- hi-->", "<!-- hi-->")]),
    ("crlf", "<x>\r\n<!-- hi\r\nthere -->\r\n</x>", None, &[(0, 29, "<x>", "<x>"), (5, 22, "<!-- hi-->", "<!-- hi-->")]),
    ("cr", "<x>\r<!-- hi\rthere -->\r</x>", None, &[(0, 26, "<x>", "<x>"), (4, 21, "<!-- hi-->", "<!-- hi-->")]),
    ("astral-prefix", "<x>😀<!-- title\nnext --><n>\n</n></x>", None, &[(0, 36, "<x>", "<x>"), (5, 24, "<!-- title-->", "<!-- title-->"), (24, 32, "<n>", "<n>")]),
    ("empty", "<root>\n<x\n id='a' />\n<x\n id='b'></x>\n</root>", None, &[(0, 44, "<root>", "<root>"), (21, 36, "<x>", "<x id=\"b\">")]),
    ("attributes", "<ns:r xmlns:ns='urn:ex'  p='a  b &quot; &lt; &gt; &amp; &apos; &#x9;' \n id = '😀'>\ntext\n</ns:r   >", None, &[(0, 95, "<ns:r>", "<ns:r xmlns:ns=\"urn:ex\" p=\"a  b &quot; &lt; &gt; &amp; ' \t\" id=\"😀\">")]),
    ("attributes-newlines", "<r a='v\r\n w\t x\n z' b='&#xD;&#xA;&#x9;'>\n</r>", None, &[(0, 44, "<r>", "<r a=\"v  w  x  z\" b=\"\r\n\t\">")]),
    ("partial", "<root>\n<good>\nline\n</good>\n<!-- hi\nmore -->\n<bad>\n</root>", Some(52), &[(7, 26, "<good>", "<good>"), (27, 43, "<!-- hi-->", "<!-- hi-->")]),
    ("truncated", "<root>\n<good>\n</good>\n<bad", Some(26), &[(7, 21, "<good>", "<good>")]),
    ("truncated-content", "<root>\n<good>\n</good>\n<bad>😀", Some(29), &[(7, 21, "<good>", "<good>")]),
    ("truncated-comment", "<root>\n<good>\n</good>\n<!--comment\nmore", Some(38), &[(7, 21, "<good>", "<good>")]),
    ("extra-root", "<root>\n</root><x/\n>", Some(15), &[(0, 14, "<root>", "<root>")]),
    ("trailing-text", "<root>\n</root>😀", Some(14), &[(0, 14, "<root>", "<root>")]),
    ("bad-entity", "<root>\n<good>\n</good>\n<bad>&unknown;</bad>\n</root>", Some(28), &[(7, 21, "<good>", "<good>")]),
    ("bad-attr-entity", "<root>\n<good>\n</good>\n<bad a='&unknown;'>\n</bad>\n</root>", Some(31), &[(7, 21, "<good>", "<good>")]),
    ("late-bad-attr", "<root>\n<good>\n</good>\n<bad a='x' a='y'>\n</bad>\n</root>", Some(33), &[(7, 21, "<good>", "<good>")]),
    ("bad-namespace", "<root>\n<good>\n</good>\n<p:bad>\n</p:bad>\n</root>", Some(23), &[(7, 21, "<good>", "<good>")]),
    ("qname-open-incomplete", "<r>\r\n<g>\r\n</g>\r\n<bad:", Some(21), &[(5, 14, "<g>", "<g>")]),
    ("qname-close-incomplete", "<r>\n<g>\n</g>\n<bad>\n</bad:", Some(25), &[(4, 12, "<g>", "<g>")]),
    ("qname-open-leading-colon", "<r>\n<g>\n</g>\n<:x/>\n</r>", Some(14), &[(4, 12, "<g>", "<g>")]),
    ("qname-open-incomplete-leading-colon", "<r>\n<g>\n</g>\n<:x", Some(14), &[(4, 12, "<g>", "<g>")]),
    ("qname-open-invalid-local-start", "<r>\n<g>\n</g>\n<bad:5bad/>\n</r>", Some(18), &[(4, 12, "<g>", "<g>")]),
    ("qname-open-extra-colon", "<r>\n<g>\n</g>\n<bad:name:more/>\n</r>", Some(22), &[(4, 12, "<g>", "<g>")]),
    ("qname-attribute-leading-colon", "<r>\n<g>\n</g>\n<x :thing='1'>\n</x>\n</r>", Some(16), &[(4, 12, "<g>", "<g>")]),
    ("qname-attribute-incomplete-leading-colon", "<r>\n<g>\n</g>\n<x :thing='1'", Some(16), &[(4, 12, "<g>", "<g>")]),
    ("qname-attribute-incomplete", "<r>\n<g>\n</g>\n<v a='😀'\r\n other:", Some(31), &[(4, 12, "<g>", "<g>")]),
    ("qname-attribute-extra-colon", "<r>\n<g>\n</g>\n<x wrong:name:more='1'>\n</x>\n</r>", Some(26), &[(4, 12, "<g>", "<g>")]),
    ("invalid-char", "<root>\n<good>\n</good>\n<bad>😀\u{1}</bad>\n</root>", Some(29), &[(7, 21, "<good>", "<good>")]),
    ("dtd", "<!--a\nb-->\n<!DOCTYPE r [<!ENTITY e 'value'>]>\n<r a='&e;'>\n</r>", Some(0), &[(0, 10, "<!--a-->", "<!--a-->")]),
    ("cdata", "<root>\n<![CDATA[<x>\n</x>]]>\n<!--x\ny-->\n</root>", None, &[(0, 46, "<root>", "<root>"), (28, 38, "<!--x-->", "<!--x-->")]),
    ("cdata-before-root", "<!--a\nb-->\r\n<![CDATA[c]]>\n<r/>", Some(12), &[(0, 10, "<!--a-->", "<!--a-->")]),
    ("cdata-after-root", "<r>\n</r><![CDATA[tail]]>", Some(8), &[(0, 8, "<r>", "<r>")]),
    ("cdata-partial-marker", "<r>\n</r><![CDATA", Some(8), &[(0, 8, "<r>", "<r>")]),
    ("comment-before-root", "<!--x\ny-->\n<r/>", None, &[(0, 10, "<!--x-->", "<!--x-->")]),
    ("comment-only", "<!--x\ny-->", Some(0), &[(0, 10, "<!--x-->", "<!--x-->")]),
    ("malformed-comment", "<root>\n<!--x\nz--h-->\n</root>", Some(14), &[]),
    ("bom", "\u{feff}<x>\n</x>", Some(0), &[]),
    ("empty-file", "", Some(0), &[]),
    ("newline-only", "\n", Some(0), &[]),
    ("unclosed-root", "<r>", Some(0), &[]),
    ("invalid-syntax", "<root>\n<good>\n</good>\n<>\n</root>", Some(23), &[(7, 21, "<good>", "<good>")]),
    ("invalid-name", "<root>\n<good>\n</good>\n<4x>\n</4x>\n</root>", Some(23), &[(7, 21, "<good>", "<good>")]),
    ("unexpected-decl", "<!--a\nb-->\n<?xml version='1.0'?>\n<r/>", Some(13), &[(0, 10, "<!--a-->", "<!--a-->")]),
    ("declaration-version", "<?xml version='1.1'?><r>\n</r>", Some(15), &[]),
    ("declaration-version-prefix", "<?xml version='1.0.2'?>\n<r>\n</r>", None, &[(24, 32, "<r>", "<r>")]),
    ("declaration-version-before-standalone", "<?xml version='1.1' encoding='a:b' standalone='bad'?>\n<r>\n</r>", Some(15), &[]),
    ("declaration-valid-standalone", "<?xml version='1.0' encoding='Shift_JIS' standalone='yes'?>\r\n<r>😀\r\n</r>", None, &[(61, 72, "<r>", "<r>")]),
    ("declaration-invalid-standalone", "<?xml version='1.0' standalone='true'?>\n<r>\n</r>", Some(31), &[]),
    ("declaration-empty-standalone", "<?xml version='1.0' standalone=''?>\n<r>\n</r>", Some(31), &[]),
    ("declaration-unfinished-standalone", "<?xml version='1.0' standalone='bad'", Some(31), &[]),
    ("declaration-unclosed-value", "<?xml version='1.0' standalone='bad", Some(32), &[]),
    ("declaration-escaped-standalone", "<?xml version='1.0' standalone='y&#101;s'?>\n<r>\n</r>", Some(32), &[]),
    ("declaration-non-bmp-encoding", "<?xml version='1.0' encoding='a😀b'?>\n<r>\n</r>", Some(30), &[]),
    ("declaration-control-in-encoding", "<?xml version='1.0' encoding='utf\t8'?>\n<r>\n</r>", Some(30), &[]),
    ("declaration-bmp-encoding", "<?xml version='1.0' encoding='ümlaut'?>\n<r>\n</r>", None, &[(40, 48, "<r>", "<r>")]),
    ("declaration-multiline-standalone", "<?xml version='1.0'\r\n standalone \t= \"😀\"?>\r\n<r>\r\n</r>", Some(37), &[]),
    ("malformed-decl", "<?xml version = >\n<r>\n</r>", Some(16), &[]),
    ("malformed-entity", "<root>\n<good>\n</good>\n<z>a& b</z>\n</root>", Some(27), &[(7, 21, "<good>", "<good>")]),
    ("invalid-attribute", "<r>\n<g>\n</g>\n<x a='hey<there'/>\n</r>", Some(22), &[(4, 12, "<g>", "<g>")]),
    ("leading-cr", "<r>\r<g>\r</g>\r<bad a='x' a='y' />\r</r>", Some(24), &[(4, 12, "<g>", "<g>")]),
    ("mismatch-crlf", "<r>\r\n<g>\r\n</g>\r\n<bad>\r\n</r>", Some(25), &[(5, 14, "<g>", "<g>")]),
    ("extra-end", "<r>\n</r>\n</x>", Some(11), &[(0, 8, "<r>", "<r>")]),
    ("no-close", "<r>\n<g>\n</g>\n", Some(13), &[(4, 12, "<g>", "<g>")]),
    ("tag-space", "<r>\n</r\n>", None, &[(0, 7, "<r>", "<r>")]),
    ("text-whitespace", "<r>\n</r>  ", Some(9), &[(0, 8, "<r>", "<r>")]),
    ("truncated-good-close", "<r>\n<g>\n</g", Some(11), &[]),
    ("duplicate-expanded", "<r>\n<g>\n</g>\n<x xmlns:a='n' xmlns:b='n' a:m='x' b:m='y'/>\n</r>", Some(48), &[(4, 12, "<g>", "<g>")]),
    ("namespace-duplicate", "<r>\n<x xmlns:p='a' xmlns:p='b'/>\n</r>", Some(19), &[]),
    ("bad-comment-hyphen", "<r>\n<!--one\nline--->\n</r>", Some(16), &[]),
    ("dtd-before-comment", "<!DOCTYPE r>\n<!--x\ny-->\n<r/>", Some(0), &[]),
    ("truncated-dtd", "<!--x\ny-->\n<!DOCTYPE r [", Some(0), &[(0, 10, "<!--x-->", "<!--x-->")]),
    ("bad-cdata", "<r>\n<g>\n</g>\ntext]]>\n</r>", Some(17), &[(4, 12, "<g>", "<g>")]),
    ("crlf-crlf", "<r>\r\n<!--one\r\nanother\r\nthird-->\r\n</r>", None, &[(0, 37, "<r>", "<r>"), (5, 29, "<!--one-->", "<!--one-->")]),
    ("unterminated-reference", "<r>\n<g>\n</g>&thing</r>", Some(18), &[(4, 12, "<g>", "<g>")]),
    ("unterminated-attr-reference", "<r>\n<g>\n</g>\n<x a='&thing' />\n</r>", Some(25), &[(4, 12, "<g>", "<g>")]),
    ("numeric-invalid", "<r>\n<g>\n</g>&#0;</r>", Some(14), &[(4, 12, "<g>", "<g>")]),
    ("numeric-invalid-digit", "<r>\n<g>\n</g>&#xQ;</r>", Some(15), &[(4, 12, "<g>", "<g>")]),
    ("numeric-missing-semi", "<r>\n<g>\n</g>&#12</r>", Some(16), &[(4, 12, "<g>", "<g>")]),
    ("invalid-control-after-entity", "<r>\n<g>\n</g>&#x1;</r>", Some(15), &[(4, 12, "<g>", "<g>")]),
    ("valid-multiple-refs", "<r q='a&amp;&#x20;&#x9;&quot;y'>x\n</r>", None, &[(0, 38, "<r>", "<r q=\"a&amp; \t&quot;y\">")]),
    ("two-cdata-ends", "<r>\n<g>\n</g>\ntext]]>other]]>\n</r>", Some(17), &[(4, 12, "<g>", "<g>")]),
    ("valid-cdata-marker-attr", "<r q=']]>'>\n<g>\n</g>\ntext]]>\n</r>", Some(25), &[(12, 20, "<g>", "<g>")]),
    ("closing-space-cr", "<r>\r<g>\r</g\r   >\r</r\t\t>", None, &[(0, 21, "<r>", "<r>"), (4, 11, "<g>", "<g>")]),
    ("comment-crlf-and-astral", "<r>😀<!--abc\r\n😀\r\nb-->\n</r>", None, &[(0, 27, "<r>", "<r>"), (5, 20, "<!--abc-->", "<!--abc-->")]),
    ("no-version", "<?xml encoding='UTF-8'?><r>\n</r>", Some(6), &[]),
    ("invalid-namespace-uri", "<r>\n<x xmlns:xml='not-xml'>\n</x>\n</r>", Some(7), &[]),
    ("unbound-prefix-attr", "<r>\n<x p:id='0'>\n</x>\n</r>", Some(7), &[]),
    ("outside-invalid", "<r>\n</r> <!-- x\r\n z --><?p?> junk", Some(29), &[(0, 8, "<r>", "<r>"), (9, 22, "<!-- x-->", "<!-- x-->")]),
    ("valid-fffD", "<r>\n<g a='�'>\n</g>\n</r>", None, &[(0, 23, "<r>", "<r>"), (4, 18, "<g>", "<g a=\"�\">")]),
    ("space-invalid", "<r>\n<g>\n</g>\n<x xml:space='keep'>\n</x>\n</r>", Some(16), &[(4, 12, "<g>", "<g>")]),
    ("space-caps", "<r>\n<g>\n</g>\n<x xml:space='Preserve'>\n</x>\n</r>", Some(16), &[(4, 12, "<g>", "<g>")]),
    ("space-empty", "<r>\n<g>\n</g>\n<x xml:space=''>\n</x>\n</r>", Some(16), &[(4, 12, "<g>", "<g>")]),
    ("space-astral", "<r>\r<g>\r</g>\r<x y='😀' xml:space='keep' />\r</r>", Some(23), &[(4, 12, "<g>", "<g>")]),
    ("space-later-namespace", "<r>\n<g>\n</g>\n<q:x xml:space='keep'/>\n</r>", Some(18), &[(4, 12, "<g>", "<g>")]),
    ("space-tag-next-line", "<r>\n<g>\n</g>\n<x\n xml:space = \"novel\"\n/ >\n</r>", Some(17), &[(4, 12, "<g>", "<g>")]),
    ("space-after-multiline-quote", "<r>\n<g>\n</g>\n<x b=\"line1\r\nline2\" xml:space = \"novel\"\n />\n</r>", Some(33), &[(4, 12, "<g>", "<g>")]),
    ("space-mixed-first-syntax", "<r>\n<g>\n</g>\n<x broken=& apos xml:space='novel'>\n</x>\n</r>", Some(23), &[(4, 12, "<g>", "<g>")]),
    ("space-numeric-padding", "<r>\n<g xml:space='&#xA;preserve&#x9;'>\n</g>\n</r>", None, &[(0, 48, "<r>", "<r>"), (4, 43, "<g>", "<g xml:space=\"\npreserve\t\">")]),
    ("space-markup", "<r>\n<g>\n</g>\n<x q=\"xml:space='keep'\">\n</x>\n</r>", None, &[(0, 47, "<r>", "<r>"), (4, 12, "<g>", "<g>"), (13, 42, "<x>", "<x q=\"xml:space='keep'\">")]),
    ("missing-attr-end", "<r>\n<g>\n</g>\n<x xml:space='novel'", Some(16), &[(4, 12, "<g>", "<g>")]),
    ("case-pi", "<?XML version='1.0'?>\n<r>\n</r>", Some(2), &[]),
    ("pi-invalid-separator", "<r>\n<g>\n</g>\n<?target:x#?>\n</r>", Some(21), &[(4, 12, "<g>", "<g>")]),
    ("pi-colon-in-data", "<r>\n<g>\n</g>\n<?target a:b#?>\n</r>", None, &[(0, 33, "<r>", "<r>"), (4, 12, "<g>", "<g>")]),
    ("ns-undeclare", "<r xmlns:p='urn:foo'>\n<g>\n</g>\n<x xmlns:p=''>\n</x>\n</r>", Some(42), &[(22, 30, "<g>", "<g>")]),
    ("ns-default-xmlns-uri", "<r>\n<g>\n</g>\n<x xmlns='http://www.w3.org/2000/xmlns/'>\n</x>\n</r>", Some(23), &[(4, 12, "<g>", "<g>")]),
    ("ns-default-xml-uri", "<r>\n<g>\n</g>\n<x xmlns='http://www.w3.org/XML/1998/namespace'>\n</x>\n</r>", Some(23), &[(4, 12, "<g>", "<g>")]),
    ("ns-reserved-uri-escaped", "<r>\n<g>\n</g>\n<x xmlns:p='http:&#47;/www.w3.org/XML/1998/namespace'>\n</x>\n</r>", Some(25), &[(4, 12, "<g>", "<g>")]),
    ("ns-reserved-uri-spaced", "<r>\r\n<g>\n</g>\n<x a='😀' xmlns:p \r\n= \t\"http://www.w3.org/XML/1998/namespace\">\n</x>\n</r>", Some(38), &[(5, 13, "<g>", "<g>")]),
    ("ns-reserved-uri-incomplete", "<r>\n<g>\n</g>\n<x xmlns:p='http://www.w3.org/XML/1998/namespace'", Some(25), &[(4, 12, "<g>", "<g>")]),
    ("ns-xmlns-prefix", "<r>\n<g>\n</g>\n<x xmlns:xmlns='urn:no'>\n</x>\n</r>", Some(16), &[(4, 12, "<g>", "<g>")]),
    ("ns-xmlns-prefix-incomplete", "<r>\n<g>\n</g>\n<x xmlns:xmlns='urn:foo'", Some(16), &[(4, 12, "<g>", "<g>")]),
    ("ns-xmlns-prefix-canonical", "<r>\n<g>\n</g>\n<x xmlns:xmlns='http://www.w3.org/2000/xmlns/'>\n</x>\n</r>", Some(29), &[(4, 12, "<g>", "<g>")]),
    ("ns-xmlns-prefix-unknown-entity", "<r>\n<g>\n</g>\n<x xmlns:xmlns='&bogus;'>\n</x>\n</r>", Some(30), &[(4, 12, "<g>", "<g>")]),
    ("default-ns-empty", "<r xmlns='urn:r'>\n<g xmlns=''>\n</g>\n</r>", None, &[(0, 40, "<r>", "<r xmlns=\"urn:r\">"), (18, 35, "<g>", "<g xmlns=\"\">")]),
    ("doctype-after-root", "<r>\n<g>\n</g>\n</r>\n<!DOCTYPE r>", Some(0), &[(0, 17, "<r>", "<r>"), (4, 12, "<g>", "<g>")]),
];

#[test]
fn xml_folds_match_the_pinned_reference() {
    for &(case, text, error, expected) in CASES {
        for show in [false, true] {
            let strategy = XmlFoldingStrategy {
                show_attributes_when_folded: show,
            };
            let (folds, actual_error) = strategy.create_new_foldings(text);
            let ranges: Vec<_> = folds
                .iter()
                .map(|fold| {
                    (
                        fold.start_offset,
                        fold.end_offset,
                        fold.name.as_deref().unwrap(),
                    )
                })
                .collect();
            let titles: Vec<_> = expected
                .iter()
                .map(|&(start, end, plain, attributes)| {
                    (start, end, if show { attributes } else { plain })
                })
                .collect();
            assert_eq!(
                (actual_error, ranges),
                (error, titles),
                "{case}, show_attributes={show}"
            );
        }
    }
}

#[test]
fn the_reserved_xmlns_prefix_is_rejected_on_elements() {
    let text = "<r>\n<g>\n</g>\n<xmlns:x>\n</xmlns:x>\n</r>";
    let (folds, error) = XmlFoldingStrategy::default().create_new_foldings(text);
    assert_eq!(error, Some(14));
    assert_eq!(folds.len(), 1);
    assert_eq!(folds[0].name.as_deref(), Some("<g>"));
}

#[test]
fn updating_after_a_parse_error_reuses_closed_folds_and_keeps_the_uncertain_suffix() {
    for (bad, end) in [
        ("<bad>\n</root>", "</tail>"),
        ("<bad xml:space='keep'/>", "</tail>\n</root>"),
        ("<bad xmlns:xmlns='urn:no'/>", "</tail>\n</root>"),
    ] {
        let text = format!("<root>\n<good>\nline\n</good>\n{bad}\n<tail>\n{end}");
        let document = TextDocument::from_text(&text);
        let manager = FoldingManager::new(document.clone());
        let stale_prefix = manager.create_folding(0, 6).unwrap();
        stale_prefix.set_is_folded(true);
        let good = manager.create_folding(7, 12).unwrap();
        good.set_is_folded(true);
        let tail = manager
            .create_folding(text.find("<tail>").unwrap(), text.find("</tail>").unwrap() + 7)
            .unwrap();
        tail.set_is_folded(true);

        XmlFoldingStrategy::default()
            .update_foldings(&manager, &document)
            .unwrap();
        let updated = manager.all_foldings();
        assert_eq!(updated.len(), 2);
        assert!(!stale_prefix.is_folded());
        assert!(updated[0] == good && good.is_folded());
        assert_eq!(good.end_offset(), 26);
        assert_eq!(good.title().as_deref(), Some("<good>"));
        assert!(updated[1] == tail && tail.is_folded());
    }
}

#[test]
fn a_lone_surrogate_stops_folding_at_a_document_error() {
    let mut source: Vec<_> = "<r>\n<g>\n</g>\n".encode_utf16().collect();
    source.push(0xd800);
    source.extend("<tail>\n</tail>\n</r>".encode_utf16());
    let document = TextDocument::from_utf16(&source);
    let manager = FoldingManager::new(document.clone());
    XmlFoldingStrategy::default()
        .update_foldings(&manager, &document)
        .unwrap();
    let folds = manager.all_foldings();
    assert_eq!(folds.len(), 1);
    assert_eq!(folds[0].title().as_deref(), Some("<g>"));
}
