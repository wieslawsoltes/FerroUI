use ferroui_designer_language::lsp::{serve_with, LspServer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::for_qml()?;
    serve_with(
        server,
        std::io::BufReader::new(std::io::stdin()),
        std::io::stdout().lock(),
    )?;
    Ok(())
}
