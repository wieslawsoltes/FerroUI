//! Compare in-memory XML parsing with FXD expansion and replay of the same syntax.
//! Excludes framework transformation, object construction, file I/O and output.
use ferroui_markup_xaml_loader::binary_document::{decode, encode, encode_stored};
use std::{env, error::Error, fs, hint::black_box, time::Instant};
use xamlx::parsers::XDocumentXamlParser;

fn main() -> Result<(), Box<dyn Error>> {
    let paths = env::args_os().skip(1).collect::<Vec<_>>();
    if paths.is_empty() {
        return Err("pass one or more XAML source paths".into());
    }
    println!("file\txml_bytes\tstored_bytes\tfxd_bytes\txml_p50_us\tstored_p50_us\tfxd_p50_us");
    for path in paths {
        let xml = fs::read_to_string(&path)?;
        let syntax = XDocumentXamlParser::parse(&xml, None)?;
        let raw = encode_stored(&syntax)?;
        let fxd = encode(&syntax)?;
        assert_eq!(encode_stored(&decode(&fxd)?)?, raw);
        let mut samples = [
            Vec::with_capacity(51),
            Vec::with_capacity(51),
            Vec::with_capacity(51),
        ];
        for pass in 0..51 {
            for offset in 0..3 {
                let case = (pass + offset) % 3;
                let start = Instant::now();
                let doc = match case {
                    0 => XDocumentXamlParser::parse(black_box(&xml), None)?,
                    1 => decode(black_box(&raw))?,
                    _ => decode(black_box(&fxd))?,
                };
                drop(black_box(doc));
                samples[case].push(start.elapsed());
            }
        }
        let [parse, stored, compressed] = samples.map(|mut times| {
            times.sort_unstable();
            times[25].as_micros()
        });
        println!(
            "{}\t{}\t{}\t{}\t{parse}\t{stored}\t{compressed}",
            path.to_string_lossy(),
            xml.len(),
            raw.len(),
            fxd.len()
        );
    }
    Ok(())
}
