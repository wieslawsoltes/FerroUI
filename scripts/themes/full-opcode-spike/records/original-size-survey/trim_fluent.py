import json, re
from collections import defaultdict
from pathlib import Path
root=Path.cwd()
out=root/'target/size-investigation'
hello=(root/'src/FerroUI/examples/hello.rs').read_text()
theme=json.loads((root/'src/FerroUI.Themes.Fluent/compiled_xaml.xamlmeta').read_text())['theme_units'][0]['units']
types=[]
for f in (root/'target/dist/build').glob('*/out/*.xamlmeta'):
    types.extend(json.loads(f.read_bytes()).get('types',[]))
local={v['rust_path'].split('::')[-1]:v['name'] for v in types}
graph=defaultdict(set)
for v in types:
    if v.get('base'):
        base=v['base'].split('::')[-1]
        graph[v['name']].add(local.get(base,base))
    graph[v['name']].update(n.split('.')[-1] for n in v.get('creates',[]))
alive=set(re.findall(r'\b[A-Z][A-Za-z0-9_]+\b',hello))
keys=set()
kept={0}
while True:
    prev=(len(alive),len(keys),len(kept))
    for i in kept:
        alive.update(theme[i].get('types',[]))
        keys.update(theme[i].get('references',[]))
    for n in list(alive):
        alive.update(graph[n])
    for u in theme:
        if alive.intersection(u.get('type_keys',[])) or keys.intersection(u.get('keys',[])):
            kept.add(u['index'])
    if prev==(len(alive),len(keys),len(kept)): break
print(len(kept),'of',len(theme),[(i,theme[i]['name']) for i in sorted(kept)])
table=', '.join('&theme::THEME_UNIT_'+str(i) for i in sorted(kept))
hello=hello.replace('        this.styles().add(FluentTheme::new().as_style());',f'        use ferroui::themes_fluent::compiled_xaml as theme;\n        this.styles().add(FluentTheme::with_parts(None, &[{table}]).as_style());')
(out/'hello_size_probe.rs').write_text(hello)
