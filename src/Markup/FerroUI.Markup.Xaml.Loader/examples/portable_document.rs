//! Compare in-memory XML parsing with FXD expansion and replay of the same syntax.
//! Excludes framework transformation, object construction, file I/O and output.
use ferroui_markup_xaml_loader::binary_document::{
    compile, decode, encode, encode_stored,
    pack::{compile_directory, ThemePack},
};
use std::{env, error::Error, fs, hint::black_box, path::Path, time::Instant};
use xamlx::parsers::XDocumentXamlParser;

fn pack_benchmark(root: &Path, selected: &str) -> Result<(), Box<dyn Error>> {
    let mut builds = Vec::new();
    let mut image = Vec::new();
    for _ in 0..9 {
        let start = Instant::now();
        let next = compile_directory(root)?;
        builds.push(start.elapsed().as_micros());
        if !image.is_empty() {
            assert_eq!(image, next);
        }
        image = next;
    }
    let pack = ThemePack::open(&image)?;
    let mut xml_bytes = 0;
    let mut loose_bytes = 0;
    let mut members = Vec::new();
    for path in pack.paths() {
        let xml = fs::read_to_string(root.join(path))?;
        let loose = compile(&xml)?;
        let packed = pack.document(path)?;
        // Stored syntax includes source positions, namespaces and shared references.
        assert_eq!(
            encode_stored(&decode(&loose)?)?,
            encode_stored(&decode(packed)?)?
        );
        xml_bytes += xml.len();
        loose_bytes += loose.len();
        members.push((path, loose, packed));
    }
    let selected_images = members
        .iter()
        .find(|(path, _, _)| *path == selected)
        .ok_or("requested path is absent from the pack")?;
    let mut index_ns = Vec::new();
    let mut samples = [const { Vec::new() }; 4];
    for pass in 0..51 {
        let start = Instant::now();
        drop(black_box(ThemePack::open(black_box(&image))?));
        index_ns.push(start.elapsed().as_nanos());
        for offset in 0..4 {
            let case = (offset + pass) % 4;
            let start = Instant::now();
            match case {
                0 => drop(black_box(decode(black_box(&selected_images.1))?)),
                1 => drop(black_box(decode(black_box(pack.document(selected)?))?)),
                _ => {
                    for (_, loose, packed) in &members {
                        let bytes = if case == 2 { loose.as_slice() } else { packed };
                        drop(black_box(decode(black_box(bytes))?));
                    }
                }
            }
            samples[case].push(start.elapsed().as_micros());
        }
    }
    builds.sort_unstable();
    index_ns.sort_unstable();
    println!(
        "documents={} xml_bytes={xml_bytes} loose_fxd_bytes={loose_bytes} fxp_bytes={}",
        members.len(),
        image.len()
    );
    println!(
        "pack_compile_us_median={} index_open_ns_median={} index_open_ns_p95={}",
        builds[4], index_ns[25], index_ns[48]
    );
    for (name, mut times) in [
        "loose_selected",
        "packed_selected",
        "loose_all",
        "packed_all",
    ]
    .into_iter()
    .zip(samples)
    {
        times.sort_unstable();
        println!(
            "{name}_decode_us_median={} {name}_decode_us_p95={}",
            times[25], times[48]
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let paths = env::args_os().skip(1).collect::<Vec<_>>();
    if let [flag, root, asset] = paths.as_slice() {
        if flag == "--pack" {
            return pack_benchmark(Path::new(root), asset.to_str().ok_or("path must be UTF-8")?);
        }
    }
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
