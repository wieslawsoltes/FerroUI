#![allow(dead_code)]

mod baseline;
mod current;
mod old_escape;
mod old_rewrite;

use std::fmt::Write;
use std::hint::black_box;
use std::time::Instant;

struct Output {
    svg: String,
    aliases: usize,
}

type Export = Box<dyn Fn(&str, bool) -> Output>;

struct Backend {
    name: &'static str,
    export: Export,
}

macro_rules! backend {
    ($module:ident) => {{
        let options = [false, true].map(|deduplicate| $module::XmlOptions {
            precision: None,
            pretty: false,
            id_prefix: "r".into(),
            deduplicate,
        });
        Backend {
            name: stringify!($module),
            export: Box::new(move |input, dedup| {
                let output = $module::optimize(input, &options[usize::from(dedup)]).unwrap();
                Output { svg: output.svg, aliases: output.stats.definitions_deduplicated }
            }),
        }
    }};
}

fn main() {
    let target = std::env::args().nth(1).expect("CSV destination argument");
    let normal = std::fs::read_to_string("raster-normal.svg").unwrap();
    let expanded = std::fs::read_to_string("raster-expanded.svg").unwrap();
    let backends = [backend!(baseline), backend!(current), backend!(old_escape), backend!(old_rewrite)];
    let cases = [
        ("repeated_dedup", expanded.as_str(), true, 7),
        ("repeated_no_dedup", expanded.as_str(), false, 0),
        ("normalized_dedup", normal.as_str(), true, 0),
    ];
    // Input preparation, byte identity, alias checks, and eight warmups precede
    // every timed case. The same exact input is used for all four variants.
    let mut csv = String::from("case,variant,sample,order,elapsed_ns,output_bytes,aliases\n");
    const SAMPLES: usize = 800;
    for (case_name, input, dedup, aliases) in cases {
        let expected = (backends[0].export)(input, dedup).svg;
        if case_name == "repeated_dedup" {
            assert_eq!(expected, normal, "expanded fixture must canonicalize to the exact original SVG");
        }
        for backend in &backends {
            for _ in 0..8 {
                let actual = (backend.export)(input, dedup);
                assert_eq!(actual.svg, expected, "{} / {case_name}", backend.name);
                assert_eq!(actual.aliases, aliases);
            }
        }
        let mut samples: Vec<Vec<u128>> = (0..backends.len()).map(|_| Vec::with_capacity(SAMPLES)).collect();
        for sample in 0..SAMPLES {
            // Rotating order balances each variant across all four positions.
            // Output destruction and all bookkeeping occur after the timer.
            for position in 0..backends.len() {
                let index = (sample + position) % backends.len();
                let backend = &backends[index];
                let started = Instant::now();
                let output = (backend.export)(black_box(input), black_box(dedup));
                let elapsed = started.elapsed().as_nanos();
                black_box(&output.svg);
                assert_eq!(output.svg.len(), expected.len());
                assert_eq!(output.aliases, aliases);
                samples[index].push(elapsed);
                writeln!(csv, "{case_name},{},{sample},{position},{elapsed},{},{}", backend.name, output.svg.len(), output.aliases).unwrap();
            }
        }
        for (index, backend) in backends.iter().enumerate() {
            samples[index].sort_unstable();
            let values = &samples[index];
            let median = (values[SAMPLES / 2 - 1] + values[SAMPLES / 2]) as f64 / 2e6;
            let p95 = values[(SAMPLES * 95).div_ceil(100) - 1] as f64 / 1e6;
            println!("{case_name:20} {:12} median={median:.6} ms p95={p95:.6} ms bytes={} aliases={aliases}", backend.name, expected.len());
        }
    }
    std::fs::write(target, csv).unwrap();
}
