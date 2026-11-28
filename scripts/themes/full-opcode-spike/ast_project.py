"""Create the scratch loader variant that accepts offline XamlDocument programs."""
from pathlib import Path
import json
import shutil

def setup(root,out,decoder,archive,theme,path_names):
    runtime=out/"runtime"
    ast=out/"ast"
    shutil.copytree(runtime,ast,dirs_exist_ok=True,ignore=shutil.ignore_patterns("probe","keys.txt","winui.bin"))
    theme_dir=ast/"theme"
    loader=ast/"loader"
    source=root/"src/Markup/FerroUI.Markup.Xaml.Loader"
    shutil.copytree(source,loader,dirs_exist_ok=True)
    spec=(source/"Cargo.toml").read_text()
    spec=spec.replace("\nversion.workspace = true",'\nversion="0.5.1-preview.1"').replace("edition.workspace = true",'edition="2021"')
    spec='\n'.join(s for s in spec.splitlines() if not s.endswith('.workspace = true'))
    paths={"xamlx":"external/XamlX/src/XamlX","ferroui-base":"src/FerroUI.Base","ferroui-markup":"src/Markup/FerroUI.Markup","ferroui-build-scan":"src/FerroUI.Build.Scan","ferroui-markup-xaml":"src/Markup/FerroUI.Markup.Xaml"}
    import re
    for crate,path in paths.items():
        spec=re.sub(r"(?m)^"+crate+r" = \{ workspace = true",crate+" = { path = "+json.dumps(str(root/path)),spec)
    for crate,version in [("url","2.5.8"),("roxmltree","0.20")]:
        spec=spec.replace(crate+" = { workspace = true }",crate+" = "+json.dumps(version))
    spec += '\n[dependencies.ferro-spike-codec]\npath = ' + json.dumps(str(root/"target/semantic-spike/codec")) + '\n'
    # The build-time compiler and this private, runtime-only loader can coexist in Cargo.lock.
    spec = spec.replace('name = "ferroui-markup-xaml-loader"', 'name = "ferroui-full-theme-decoder"')
    (loader/"Cargo.toml").write_text(spec+'\n')
    t=theme_dir/"Cargo.toml"
    t.write_text(t.read_text().replace(str(source),str(loader)).replace('ferroui-markup-xaml-loader = {', 'ferroui-markup-xaml-loader = { package = "ferroui-full-theme-decoder",'))
    a=ast/"Cargo.toml"
    a.write_text(a.read_text().replace(str(runtime/"theme"),str(theme_dir)))
    compiler=loader/"compiler_extensions/ferro_xaml_il_compiler.rs"
    orig=compiler.read_text()
    parse='let parsed = XDocumentXamlParser::parse(xaml, Some(&compatibility_mappings))?;'
    assert orig.count(parse)==1
    compiler.write_text(orig.replace(parse,"let parsed = if let Some(index)=xaml.strip_prefix(\"FXAST:\") { crate::binary_input::decode(index) } else { XDocumentXamlParser::parse(xaml, Some(&compatibility_mappings))? };"))
    runtime_compiler=loader/"ferro_xaml_il_runtime_compiler.rs"
    orig=runtime_compiler.read_text()
    marker="let name = document.document.clone();"
    assert orig.count(marker)==1
    # The group transforms all linked documents. Only its class document is requested by the theme.
    orig=orig.replace("let mut loaded = Vec::with_capacity(documents.len());", """let archive_group = sources.iter().all(|s| s.xaml.starts_with("FXAST:"));
        let mut loaded = Vec::with_capacity(documents.len());""")
    runtime_compiler.write_text(orig.replace(marker, """if archive_group && document.root_instance.is_none() { loaded.push(None); continue; }
            """+marker))
    l=loader/"lib.rs"
    l.write_text(l.read_text()+'\nmod binary_input;\n')
    (loader/"binary_input.rs").write_text(decoder)
    shutil.copyfile(archive,loader/"theme.bin")
    (theme_dir/"compiled_xaml.rs").write_text(theme)
    (theme_dir/"paths.rs").write_text('&['+',\n'.join(json.dumps(p) for p in path_names)+']')


if __name__ == "__main__":
    root=Path.cwd()
    out=root/"target/full-opcode-spike"
    here=Path(__file__).resolve().parent
    archive=root/"target/semantic-spike/winui.bin"
    codec=root/"scripts/themes/semantic-spike/archive.rs"
    theme=(here/"runtime.rs").read_text()
    i=theme.index("fn integer(")
    j=theme.index("pub fn populate_with_parts",i)
    theme=theme[:i]+"""fn documents() -> Vec<(String,String)> {
        let names: &[&str]=include!("paths.rs");
        names.iter().map(|name|(name.to_string(),format!("FXAST:{name}"))).collect()
    }
    """+theme[j:]
    loader='#[path="archive.rs"] mod semantic;\nthread_local! { static ARCHIVE: semantic::Archive<\'static> = semantic::Archive::open(include_bytes!("theme.bin")); }\npub fn decode(path: &str) -> xamlx::ast::XamlDocument { ARCHIVE.with(|a| a.document(path)) }\n'
    source=root/"src/FerroUI.Themes.WinUI"
    names=[str(p.relative_to(source)) for p in sorted(source.rglob("*.xaml"))]
    setup(root,out,loader,archive,theme,names)
    shutil.copyfile(codec,out/"ast/loader/archive.rs")
