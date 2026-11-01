//! Headless debugger for source documents using QtQml's built-in types.
use ferroui_qml::diagnostics::{Breakpoint, Debugger, Frame, Kind, Phase, Resume};
use ferroui_qml::{Instance, TypeRegistry};
use ferroui_qml_build::Compiler;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "ferroui-qml-debug [-I IMPORT_ROOT] [--trace] [--step] [--break KIND:NAME] [--call METHOD] INPUT.qml
Runs a linked QtQml document, calls each requested zero-argument root method, and prints its profile.
KIND is method, handler or binding. For an offset breakpoint use --at URL@BYTE.
While stopped: c continue, i step into, n step over, o step out, a abort, args, get PROPERTY.";

fn pause(frame: &Frame<'_>) -> Resume {
    eprintln!(
        "stopped at {:?} {} {} {:?} (depth {})",
        frame.site.kind,
        frame.site.name,
        frame.site.source.as_deref().unwrap_or("<memory>"),
        frame.site.span,
        frame.depth
    );
    loop {
        eprint!("qml> ");
        if io::stderr().flush().is_err() {
            return Resume::Abort;
        }
        let mut line = String::new();
        if !matches!(io::stdin().read_line(&mut line), Ok(n) if n > 0) {
            return Resume::Abort;
        }
        match line.trim() {
            "c" => return Resume::Continue,
            "i" => return Resume::StepInto,
            "n" => return Resume::StepOver,
            "o" => return Resume::StepOut,
            "a" => return Resume::Abort,
            "args" => {
                for (parameter, value) in frame.parameters.iter().zip(frame.arguments) {
                    eprintln!("{} = {value:?}", parameter.name);
                }
            }
            text if text.starts_with("get ") => {
                match frame.instance.get(frame.receiver, text[4..].trim()) {
                    Ok(value) => eprintln!("{value:?}"),
                    Err(error) => eprintln!("{error}"),
                }
            }
            _ => eprintln!("c continue, i into, n over, o out, a abort, args, get PROPERTY"),
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let debug = Debugger::default();
    let mut breaks = vec![];
    let mut calls = vec![];
    let mut compiler = Compiler::new();
    let mut input = None;
    debug.on_pause(pause);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            "-I" => compiler = compiler.import_path(args.next().ok_or("-I requires a directory")?),
            "--call" => calls.push(args.next().ok_or("--call requires a method")?),
            "--step" => debug.pause_next(),
            "--trace" => debug.on_trace(|event| {
                let phase = match event.phase {
                    Phase::Enter => "enter",
                    Phase::Exit { error: Some(_), .. } => "fail",
                    Phase::Exit { .. } => "exit",
                };
                eprintln!(
                    "{phase}\t{}\t{:?}\t{}\t{}",
                    event.depth,
                    event.site.kind,
                    event.site.name,
                    event.site.source.as_deref().unwrap_or("<memory>")
                );
            }),
            "--at" => {
                let site = args.next().ok_or("--at requires URL@BYTE")?;
                let (url, byte) = site.rsplit_once('@').ok_or("--at requires URL@BYTE")?;
                breaks.push(Breakpoint::Source {
                    url: url.to_owned(),
                    offset: byte.parse()?,
                });
            }
            "--break" => {
                let point = args.next().ok_or("--break requires KIND:NAME")?;
                let (kind, name) = point.split_once(':').ok_or("--break requires KIND:NAME")?;
                let kind = match kind {
                    "method" => Kind::Method,
                    "binding" => Kind::Binding,
                    "handler" => Kind::Handler,
                    _ => return Err("expected method, binding or handler".into()),
                };
                breaks.push(Breakpoint::Member {
                    kind,
                    name: name.to_owned(),
                });
            }
            arg if arg.starts_with('-') || input.is_some() => return Err(USAGE.into()),
            arg => input = Some(PathBuf::from(arg)),
        }
    }
    debug.set_breakpoints(breaks);
    let source = compiler.load(input.ok_or(USAGE)?)?;
    let result = Instance::with_debugger(
        &source.document,
        &TypeRegistry::new(),
        Default::default(),
        None,
        debug.clone(),
    )
    .and_then(|mut instance| {
        for method in calls {
            instance.call(instance.root(), &method, Vec::new())?;
        }
        Ok(())
    });
    let mut profile: Vec<_> = debug.take_profile().into_iter().collect();
    profile.sort_by_key(|(_, record)| std::cmp::Reverse(record.exclusive));
    println!("self_us\ttotal_us\tmax_us\tcalls\tfailures\tkind\tmember\tsource\tbyte_range");
    for (site, stats) in profile {
        let span = site
            .span
            .map(|s| format!("{}..{}", s.start, s.end))
            .unwrap_or_default();
        println!(
            "{}\t{}\t{}\t{}\t{}\t{:?}\t{}\t{}\t{}",
            stats.exclusive.as_micros(),
            stats.inclusive.as_micros(),
            stats.longest.as_micros(),
            stats.calls,
            stats.failures,
            site.kind,
            site.name,
            site.source.as_deref().unwrap_or("<memory>"),
            span
        );
    }
    Ok(result?)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
