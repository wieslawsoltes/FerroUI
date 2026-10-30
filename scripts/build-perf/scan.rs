//! Dependency-free differential workload for the FerroUI source scanner.
//!
//! Compile this exact driver against each revision's `ferroui_build_scan` rlib,
//! then run both binaries over the SAME source checkout. Cargo remains responsible
//! for compiling the library; this separates its compilation time from scan time.
//!
//! Usage: scan-bench REPOSITORY OUTPUT_DIRECTORY [REPETITIONS=3]
//! Timings are written separately from deterministic semantic output. Every repeat
//! starts with fresh models and must reproduce the first repeat byte for byte.

use ferroui_build_scan::model::AssemblyModel;
use ferroui_build_scan::scanner::{scan_crate, ScanOptions};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

// Actual framework sources with explicitly supplied transitive models. Markup
// is also scanned as an audit workload; its normal Cargo build has no metadata
// export script. Preserve the same inputs and dependency order in both binaries.
const CRATES: &[(&str, &str, &[&str])] = &[
    ("ferroui_base", "src/FerroUI.Base/lib.rs", &[]),
    ("ferroui_controls", "src/FerroUI.Controls/lib.rs", &["ferroui_base"]),
    ("ferroui_markup", "src/Markup/FerroUI.Markup/lib.rs", &["ferroui_base"]),
    (
        "ferroui_markup_xaml",
        "src/Markup/FerroUI.Markup.Xaml/lib.rs",
        &["ferroui_base", "ferroui_controls", "ferroui_markup"],
    ),
];

fn publish(path: &Path, bytes: &[u8], repeat: usize) -> Result<(), Box<dyn Error>> {
    if repeat == 0 {
        // Refuse to mix a previous measurement with a new one. The caller chooses
        // a distinct directory for each revision/profile/attempt.
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
        std::io::Write::write_all(&mut file, bytes)?;
    } else if fs::read(path)? != bytes {
        return Err(io::Error::other(format!("nondeterministic scanner output: {} on repeat {}", path.display(), repeat + 1)).into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().collect();
    if !(3..=4).contains(&args.len()) {
        return Err("usage: scan-bench REPOSITORY OUTPUT_DIRECTORY [REPETITIONS=3]".into());
    }
    let repository = fs::canonicalize(&args[1])?;
    let output = PathBuf::from(&args[2]);
    let repeats = args.get(3).map(|value| value.to_string_lossy().parse::<usize>()).transpose()?.unwrap_or(3);
    if repeats == 0 {
        return Err("REPETITIONS must be greater than zero".into());
    }
    fs::create_dir_all(&output)?;
    let mut timings = String::from("{\"schema_version\":1,\"workload\":\"framework-source-scanner\",\"samples\":[");
    let mut first = true;
    for repeat in 0..repeats {
        let mut models: BTreeMap<&str, AssemblyModel> = BTreeMap::new();
        for &(name, relative, dependencies) in CRATES {
            let dependencies = dependencies.iter().map(|name| models[*name].clone()).collect();
            let options = ScanOptions::new(name, repository.join(relative)).with_dependencies(dependencies);
            let started = Instant::now();
            let scan = scan_crate(&options);
            let elapsed = started.elapsed().as_secs_f64();
            // Serialization and output validation are outside the measured scan.
            let records = [
                ("xamlmeta", scan.model.to_json()),
                ("diagnostics", format!("{:#?}\n", scan.diagnostics)),
                ("files", format!("{:#?}\n", scan.files)),
                ("functions", format!("{:#?}\n", scan.functions)),
                ("statistics", format!("{:#?}\n", scan.statistics)),
            ];
            for (extension, record) in records {
                publish(&output.join(format!("{name}.{extension}")), record.as_bytes(), repeat)?;
            }
            if !first {
                timings.push(',');
            }
            first = false;
            timings.push_str(&format!(
                "{{\"repeat\":{},\"crate\":\"{name}\",\"seconds\":{elapsed:.9},\"files\":{},\"types\":{}}}",
                repeat + 1,
                scan.files.len(),
                scan.model.types.len(),
            ));
            println!("{name} repeat={} files={} types={} seconds={elapsed:.6}", repeat + 1, scan.files.len(), scan.model.types.len());
            models.insert(name, scan.model);
        }
    }
    timings.push_str("]}\n");
    publish(&output.join("timings.json"), timings.as_bytes(), 0)?;
    Ok(())
}
