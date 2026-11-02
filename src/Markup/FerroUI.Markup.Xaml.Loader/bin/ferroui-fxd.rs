//! Compile one portable XamlX document and publish it as a complete file.
use ferroui_markup_xaml_loader::binary_document;
use std::{
    env,
    error::Error,
    fs,
    io::{self, Write},
    path::Path,
    process,
};

const USAGE: &str = "Usage: ferroui-fxd <input.xaml> <output.fxd>";

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let [input, output] = args.as_slice() else {
        return Err(USAGE.into());
    };
    let input = Path::new(input);
    let output = Path::new(output);
    if output.exists() && input.canonicalize()? == output.canonicalize()? {
        return Err("Input and output name the same file".into());
    }
    let xml = fs::read_to_string(input)?;
    let compiled = binary_document::compile(&xml).map_err(|e| e.message())?;
    let mut temp_name = output
        .file_name()
        .ok_or("Output must name a file")?
        .to_os_string();
    temp_name.push(format!(".{}.tmp", process::id()));
    let temp = output.with_file_name(temp_name);
    let mut file = fs::File::create_new(&temp)?;
    let result = (|| -> io::Result<()> {
        file.write_all(&compiled)?;
        drop(file);
        fs::rename(&temp, output)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
