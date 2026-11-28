from pathlib import Path
import re,subprocess,os,sys
build=Path('target/dist').resolve()
d=build.parent/'size-investigation'
p=d/'hello_size_probe.rs'
s=p.read_text().replace('with_parts(None, &[','with_parts(None, const { &[').replace('THEME_UNIT_37]).as_style()', 'THEME_UNIT_37] }).as_style()')
p.write_text(s)
flags=['rustc','--crate-name','hello','--edition','2021','-C','panic=abort','-C','opt-level=3','-C','lto=fat','-C','codegen-units=1','-C','strip=symbols','--extern','ferroui='+str(next((build/'deps').glob('libferroui-*.rlib'))),'-L','dependency='+str(build/'deps')]
search=set()
for f in (build/'build').glob('*/output'):
 for lib in re.findall(r'cargo::?rustc-link-search=(.+)',f.read_text(errors='replace')): search.add(lib.strip())
for lib in sorted(search): flags+=['-L',lib]
subprocess.run(flags+[str(p),'--emit=metadata','-o',str(d/'probe.rmeta')],check=True)
subprocess.run(flags+[str(p),'-C','link-arg=-Wl,-map,'+str(d/'fluent-trim-dist.map'),'-o',str(d/'fluent-trim-dist')],check=True)
subprocess.run([d/'fluent-trim-dist'],env=dict(os.environ,FERROUI_SMOKE_EXIT_MS='800'),timeout=20,check=True)
