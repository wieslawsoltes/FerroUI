// Standalone format timing. Load all source bytes before timing either representation.
use ferro_spike_codec::binary_document::{
    decode,
    pack::{expand_solid, ThemePack},
};
use std::{collections::HashMap, env, fs, hint::black_box, path::PathBuf, time::Instant};
use xamlx::{parsers::XDocumentXamlParser, XamlNamespaces};

fn main() {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let root = args.next().unwrap();
    let bytes = fs::read(args.next().unwrap()).unwrap();
    let expanded = expand_solid(&bytes).unwrap();
    let index = ThemePack::open(&expanded).unwrap();
    let texts: Vec<_> = index
        .paths()
        .map(|p| fs::read_to_string(root.join(p)).unwrap())
        .collect();
    let dev = "https://github.com/ferroui/hot-reload";
    let mappings = HashMap::from([
        (
            XamlNamespaces::BLEND2008.to_string(),
            XamlNamespaces::BLEND2008.to_string(),
        ),
        (dev.to_owned(), dev.to_owned()),
    ]);
    let parse_xml = || {
        for text in &texts {
            black_box(XDocumentXamlParser::parse(black_box(text), Some(&mappings)).unwrap());
        }
    };
    let parse_archive = || {
        let expanded = expand_solid(black_box(&bytes)).unwrap();
        let index = ThemePack::open(&expanded).unwrap();
        let open = Instant::now();
        for path in index.paths() {
            black_box(decode(index.document(path).unwrap()).unwrap());
        }
        (open, expanded.len())
    };
    parse_xml();
    parse_archive();
    println!("xml_ns,expand_index_ns,decode_drop_ns,expanded_bytes");
    for i in 0..21 {
        let measure_xml = || {
            let start = Instant::now();
            parse_xml();
            start.elapsed().as_nanos()
        };
        let measure_archive = || {
            let start = Instant::now();
            let (opened, size) = parse_archive();
            (
                opened.duration_since(start).as_nanos(),
                opened.elapsed().as_nanos(),
                size,
            )
        };
        let (xml, (open, decode, size)) = if i % 2 == 0 {
            (measure_xml(), measure_archive())
        } else {
            let archive = measure_archive();
            (measure_xml(), archive)
        };
        println!("{xml},{open},{decode},{size}");
    }
}
