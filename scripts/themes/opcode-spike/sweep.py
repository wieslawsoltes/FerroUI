"""Synthetic scale test using the catalog compiler and shared typed WrapPanel host."""
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET
import sys
sys.dont_write_bytecode = True
from build_driver import build
from catalog_compile import lower

here = Path(__file__).resolve().parent
def document(index):
    panel = ET.Element("WrapPanel", {"ItemSpacing":str(1 + index % 19), "LineSpacing":str(2 + index % 7)})
    for i in range(2 + index % 4):
        props = [("Content",f"Choice {index}.{i}")]
        if i == 0 or (index+i)%5==0:
            props.append(("Name",f"Toggle{index}_{i}"))
        if (index+i)%3==0:
            props.append(("IsThreeState","True"))
        if (index+2*i)%4==0:
            props.append(("IsChecked","True"))
        if (index+3*i)%7==0:
            props.append(("IsEnabled","False"))
        split=(index+i)%len(props)
        ET.SubElement(panel, "{https://github.com/ferroui}ToggleButton",dict(props[split:]+props[:split]))
    return lower(panel)

for count in (1, 8, 32, 128):
    out=Path("target/opcode-spike/sweep").resolve()/str(count)
    out.mkdir(parents=True,exist_ok=True)
    stream=bytearray()
    offsets=[0]
    source=[f"const DOCUMENTS: usize = {count};"]
    for i in range(count):
        data, native=document(i)
        stream.extend(data)
        offsets.append(len(stream))
        source.append(f"#[cfg(native)] #[inline(never)] fn body_{i}() -> (Ref<WrapPanel>, Scope) {{ let scope: Scope = Rc::new(NameScope::new()); {native} }}")
    functions=", ".join(f"body_{i}" for i in range(count))
    source.append(f"#[cfg(native)] fn build(id:usize) -> (Ref<WrapPanel>, Scope) {{ let bodies:[fn()->(Ref<WrapPanel>,Scope);{count}]=[{functions}]; bodies[id]() }}")
    source.append(f'#[cfg(vm)] fn build(id:usize) -> (Ref<WrapPanel>, Scope) {{ static OFFSETS:[u32;{count+1}]={offsets}; let data=include_bytes!(concat!(env!("SPIKE_OUT"),"/documents.bin")); execute(black_box(&data[OFFSETS[id] as usize..OFFSETS[id+1] as usize])) }}')
    (out/"documents.rs").write_text("\n".join(source))
    (out/"documents.bin").write_bytes(stream)
    for mode in ("native","vm"):
        build(here/"sweep.rs",mode,out,cache=out.parent/"thin-cache")
        exe=out/mode
        subprocess.run([exe],check=True)
        sections=subprocess.check_output(["size","-m",exe],text=True)
        text_size=int(re.search(r"Section __text: (\d+)",sections)[1])
        file_backed = sum(int(size) for size, suffix in re.findall(r"Section \S+: (\d+)([^\n]*)", sections)
                          if "zerofill" not in suffix)
        print(count, mode, len(stream), exe.stat().st_size, text_size, file_backed, flush=True)
