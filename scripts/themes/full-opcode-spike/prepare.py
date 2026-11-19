"""Prepare isolated native and binary-token-archive WinUI probe projects."""
from collections import Counter
from pathlib import Path
import json
import shutil
from xml.parsers import expat

ROOT=Path.cwd()
HERE=Path(__file__).resolve().parent
OUT=ROOT/"target/full-opcode-spike"
THEME=ROOT/"src/FerroUI.Themes.WinUI"

def uint(v):
    out=bytearray()
    while v>=128:
        out.append((v&127)|128)
        v >>= 7
    out.append(v)
    return out

def events(path):
    result=[]
    parser=expat.ParserCreate()
    parser.ordered_attributes=True
    parser.buffer_text=True
    parser.StartElementHandler=lambda n,attrs: result.append((1,[n,*attrs]))
    parser.EndElementHandler=lambda n: result.append((2,[]))
    parser.CharacterDataHandler=lambda s: result.append((3,[s]))
    parser.Parse(path.read_bytes(),True)
    return result

def archive():
    docs=[(str(p.relative_to(THEME)),events(p)) for p in sorted(THEME.rglob("*.xaml"))]
    atoms=Counter(s for name, ops in docs for _, vals in ops for s in vals)
    atoms.update(name for name,_ in docs)
    keys=sorted(atoms,key=lambda s:(-atoms[s],s))
    indices={value:i for i,value in enumerate(keys)}
    blob=bytearray(b"FXTA\x01")+uint(len(keys))
    for v in keys:
        raw=v.encode()
        blob+=uint(len(raw))+raw
    blob+=uint(len(docs))
    for name,ops in docs:
        blob+=uint(indices[name])
        for op,values in ops:
            blob.append(op)
            if op==1:
                blob+=uint(len(values)//2)
            for v in values:
                blob+=uint(indices[v])
        blob.append(0)
    (OUT/"winui.bin").write_bytes(blob)
    print(f"{len(docs)} documents, {len(keys)} atoms, {len(blob)} bytes")

def dep(name,relative):
    return f'{name} = {{ path = {json.dumps(str(ROOT/relative))} }}\n'
def deps():
    return ''.join(dep(a,b) for a,b in [
        ("ferroui-base","src/FerroUI.Base"),("ferroui-controls","src/FerroUI.Controls"),
        ("ferroui-markup-xaml","src/Markup/FerroUI.Markup.Xaml")])

def projects():
    copied=OUT/"runtime/theme"
    copied.mkdir(parents=True,exist_ok=True)
    for directory in ["Accents","Primitives","Converters"]:
        shutil.copytree(THEME/directory,copied/directory,dirs_exist_ok=True)
    for file in ["winui_theme.rs","rust_paths.rs","register_types.rs"]:
        shutil.copy2(THEME/file,copied/file)
    shutil.copy2(HERE/"runtime.rs",copied/"compiled_xaml.rs")
    shutil.copy2(OUT/"winui.bin",copied/"winui.bin")
    s=(THEME/"lib.rs").read_text().replace('ferroui_markup_xaml::include_compiled_xaml!();','mod compiled_xaml;')
    (copied/"lib.rs").write_text(s)
    (copied/"assets.rs").write_text('pub struct ExcludedDocument; pub fn excluded_documents() -> &\'static [ExcludedDocument] { &[] } pub fn register() {}')
    (copied/"Cargo.toml").write_text('[package]\nname="ferroui-themes-winui"\nversion="0.5.1-preview.1"\nedition="2021"\n[lib]\npath="lib.rs"\n[features]\ndesigner-sources=[]\n[dependencies]\n'+deps()+''.join(dep(a,b) for a,b in [
        ("ferroui-dialogs","src/FerroUI.Dialogs"),("ferroui-media-camera-controls","src/FerroUI.Media.Camera.Controls"),("ferroui-media-video-controls","src/FerroUI.Media.Video.Controls"),
        ("ferroui-markup-xaml-loader","src/Markup/FerroUI.Markup.Xaml.Loader")]))
    headless = f'ferroui-headless = {{ path = {json.dumps(str(ROOT/"src/Headless/FerroUI.Headless"))}, default-features = false, features = ["testing"] }}\n'
    for mode, theme in [('native',THEME),('runtime',copied)]:
        out=OUT/mode
        out.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(ROOT/"Cargo.lock",out/"Cargo.lock")
        driver=(ROOT/"scripts/themes/compact-spike/template_bench.rs").read_text()
        driver=driver.replace('    println!("{registration_ns}', '    verify_full(&theme);\n    println!("{registration_ns}')
        (out/"main.rs").write_text(driver+"\n"+(HERE/"verify.rs").read_text())
        (out/"Cargo.toml").write_text('[workspace]\nresolver="2"\n[package]\nname="full-opcode-probe"\nversion="0.0.0"\nedition="2021"\n[[bin]]\nname="full-opcode-probe"\npath="main.rs"\n[dependencies]\n'+deps()+headless+dep("ferroui-themes-winui",theme)+'\n[profile.dist]\ninherits="release"\nlto="fat"\ncodegen-units=1\npanic="abort"\nstrip="symbols"\n[profile.dist.package.ferroui-themes-winui]\nopt-level="z"\n')
OUT.mkdir(parents=True,exist_ok=True)
archive()
projects()
