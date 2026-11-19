#!/usr/bin/env python3
from pathlib import Path
import os,sys,re,shutil
out=Path(__file__).resolve().parent
compiler,*args=sys.argv[1:]
if 'samples/HelloWorld/lib.rs' in args and 'full_winui_size_probe' in args:
    emit=Path(args[args.index('--out-dir')+1])
    suffix=next(a.split('=',1)[1] for a in args if a.startswith('extra-filename='))
    (out/'full-theme-rlib-path.txt').write_text(str(emit/('libhello_world'+suffix+'.rlib')))
    dest=out/'full-theme-generated'
    shutil.copytree(Path(os.environ['OUT_DIR']),dest,dirs_exist_ok=True)
    f=dest/'xaml/mod.rs'
    code,n=re.subn(r'(pub static THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME:[^=]+) = &\[.*?\n    \];',r'\1 = ::ferroui_themes_winui::compiled_xaml::THEME_UNITS;',f.read_text(),flags=re.S)
    if n!=1: raise RuntimeError('expected one generated theme table')
    f.write_text(code)
    os.environ['OUT_DIR']=str(dest)
elif 'samples/HelloWorld/program.rs' in args and any('full-winui-dist.map' in a for a in args):
    pos=next(i for i,a in enumerate(args) if a.startswith('hello_world='))
    args[pos]='hello_world='+(out/'full-theme-rlib-path.txt').read_text()
os.execv(compiler,[compiler,*args])
