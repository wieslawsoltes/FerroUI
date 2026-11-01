//! cargo bench --locked -p ferroui-qml-build --bench compiler
use ferroui_qml_build::Compiler;
use std::fmt::Write;
use std::{fs, hint::black_box, path::PathBuf, time::Instant};

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn median_us<T>(mut work: impl FnMut() -> T) -> u128 {
    black_box(work());
    let mut samples = Vec::new();
    for _ in 0..9 {
        let now = Instant::now();
        black_box(work());
        samples.push(now.elapsed().as_micros());
    }
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn main() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("ferroui-qml-bench-{}", std::process::id())));
    fs::create_dir(&fixture.0).unwrap();
    let module = fixture.0.join("Bench/Tiles");
    fs::create_dir_all(&module).unwrap();
    fs::write(
        module.join("qmldir"),
        "module Bench.Tiles\nTile 1.0 Tile.qml\n",
    )
    .unwrap();
    fs::write(module.join("Tile.qml"), "import QtQuick\nRectangle { property real level: 0; width: parent.width - 4; opacity: level / 100 }").unwrap();
    let mut source = String::from("import QtQuick\nimport Bench.Tiles 1.0\nItem {\n");
    for i in 0..256 {
        writeln!(
            source,
            "  Tile {{ level: {i}; height: parent.height / 8; visible: parent.visible }}"
        )
        .unwrap();
    }
    source.push('}');
    let path = fixture.0.join("Bench.qml");
    fs::write(&path, &source).unwrap();
    let compiler = Compiler::new().import_path(&fixture.0);
    println!("256 module components, median of nine warm samples");
    println!(
        "parse: {} us",
        median_us(|| ferroui_qml_compiler::parse(&source).unwrap())
    );
    println!(
        "load and link: {} us",
        median_us(|| compiler.load(&path).unwrap())
    );
    println!("check: {} us", median_us(|| compiler.check(&path).unwrap()));
    println!(
        "compile to Rust: {} us",
        median_us(|| compiler.compile(&path).unwrap())
    );
    println!(
        "generated Rust: {} bytes",
        compiler.compile(&path).unwrap().rust.len()
    );
}
