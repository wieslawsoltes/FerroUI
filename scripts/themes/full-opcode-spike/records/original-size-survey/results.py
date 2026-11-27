import gzip,hashlib,json,os,platform,re,subprocess
from pathlib import Path
out=Path(__file__).resolve().parent
root=out.parent.parent
rows={
 'hello_world_dist_skia':out/'skia-dist',
 'hello_world_winui_16_units_comparison_stripped':out/'stripped/skia-dist',
 'hello_world_winui_166_units_comparison_stripped':out/'stripped/full-winui-dist',
 'hello_world_release_skia':out/'skia-release',
 'hello_world_release_stripped_skia':out/'skia-release-stripped',
 'hello_world_dist_opt_s_skia':out/'skia-size-s',
 'hello_world_dist_quartz_metal':out/'quartz-metal-dist',
 'facade_hello_dist_fluent':root/'target/dist/examples/hello',
 'facade_hello_control_relink':out/'fluent-control',
 'facade_hello_5_fluent_units':out/'fluent-trim-dist',
 'control_catalog_desktop_dist':out/'control-catalog-desktop',
 'control_catalog_desktop_optional_diagnostics_off_dist':out/'catalog-optional-diagnostics',
}
def read(*args):return subprocess.check_output(list(args),text=True).strip()
summary={'commit':read('git','rev-parse','HEAD'),'platform':platform.machine()+' macOS '+platform.mac_ver()[0],'rustc':read('rustc','-V'),'size_units':'bytes; decimal MB = 1,000,000 bytes','measurement':'single architecture executable on disk','artifacts':{}}
for name,path in rows.items():
    content=path.read_bytes()
    sect=read('size','-m',str(path))
    libs=[r.strip().split(' (compatibility version')[0] for r in read('otool','-L',str(path)).splitlines()[1:]]
    summary['artifacts'][name]={'file':os.path.relpath(path,root),'bytes':len(content),'gzip_9_bytes':len(gzip.compress(content,mtime=0)),'text_section_bytes':int(re.search(r'Section __text: (\d+)',sect)[1]),'sha256':hashlib.sha256(content).hexdigest(),'linked_libraries':libs}
summary['methods'] = {
    'control_catalog':'Stock catalog and an isolated desktop manifest with ferroui-diagnostics optional and disabled. Package versions verified against the workspace lockfile. Both used dist, Skia and compiled XAML.',
    'theme_experiment':'Switched only the generated WinUI theme-parts reference in Hello World from the computed 16 units to THEME_UNITS (166). Runtime metadata features stayed fixed. Apple strip applied to both comparison executables after the Rust strip helper failed on the full-theme probe. Both stripped binaries pass codesign verification.',
    'fluent':'The facade example uses FluentTheme::new(). The paired direct relink uses the same dist libraries and linker settings for the original and a 5-unit FluentTheme::with_parts variant. The native title bar accounts for the pixel differences on initial display.',
    'symbol_attribution':'Counted __text symbols by the leading crate in each demangled linker-map symbol. Shared runtime code, inlining, and anonymous data limit attribution.',
}
summary['catalog_smoke'] = (out/'catalog-pixels.txt').read_text().splitlines()
(out/'measurements.json').write_text(json.dumps(summary,indent=2)+'\n')
for k,v in summary['artifacts'].items(): print(k,v['bytes'],v['text_section_bytes'],v['gzip_9_bytes'])
