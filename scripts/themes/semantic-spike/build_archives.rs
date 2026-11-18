use ferro_spike_codec::{binary_document::pack::compile_solid_directory, Archive};
use std::{env, fs, path::PathBuf};

fn main() {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let src = args.next().unwrap();
    let out = args.next().unwrap();
    for theme in ["WinUI", "Fluent"] {
        let root = src.join(format!("FerroUI.Themes.{theme}"));
        let archive = compile_solid_directory(&root).unwrap();
        let count = Archive::open(&archive).verify(&root);
        let dest = out.join(format!("{}.bin", theme.to_lowercase()));
        fs::write(&dest, &archive).unwrap();
        println!(
            "{theme}: {count} verified documents, {} archived bytes",
            archive.len()
        );
    }
}
