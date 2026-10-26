fn main() {
    if let Err(error) = ferroui_designer_language::lsp::serve(
        std::io::BufReader::new(std::io::stdin()),
        std::io::stdout().lock(),
    ) {
        eprintln!("FerroUI XAML language server: {error}");
        std::process::exit(1);
    }
}
