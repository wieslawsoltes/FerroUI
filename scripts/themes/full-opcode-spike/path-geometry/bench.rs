//! Time the production path constructors after the compiler's work has completed.
use ferroui_base::media::{PathCommand, StreamGeometry};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_markup::path_opcodes;
use ferroui_markup_xaml::xaml_il::runtime::compiled::compiled_path_geometry;
use std::{hint::black_box, time::Instant};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let commands = PathCommand::record(&args[2]).unwrap();
    let bytes = path_opcodes::encode(&commands);
    let mut remaining = bytes.as_slice();
    for expected in &commands {
        assert_eq!(path_opcodes::read(&mut remaining).unwrap(), *expected);
    }
    assert!(remaining.is_empty());
    ferroui_base::FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(ferroui_headless::testing::headless_platform_render_interface());
    let make = || match args[1].as_str() {
        "native" => StreamGeometry::from_commands(black_box(&commands)).upcast(),
        "bytecode" => compiled_path_geometry(black_box(&bytes)).unwrap(),
        _ => panic!("unknown path constructor"),
    };
    for _ in 0..64 { black_box(make()); }
    let start = Instant::now();
    for _ in 0..1024 { black_box(make()); }
    println!("{} {} {} {}", start.elapsed().as_nanos() / 1024, commands.len(),
        commands.len() * std::mem::size_of::<PathCommand>(), bytes.len());
}
