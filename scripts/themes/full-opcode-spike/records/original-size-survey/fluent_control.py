from pathlib import Path
p=Path(__file__).with_name('relink.py')
s=p.read_text()
exec(s[:s.index('subprocess.run(flags')].replace('p.write_text(s)',''))
subprocess.run(flags+['src/FerroUI/examples/hello.rs','-C','link-arg=-Wl,-map,'+str(d/'fluent-control.map'),'-o',str(d/'fluent-control')],check=True)
print((d/'fluent-control').stat().st_size)
