from pathlib import Path
exec((Path(__file__).with_name('inspect_map.py')).read_text().split('crates=Counter()')[0])
tot=Counter(); modules=Counter()
for (_,b,i,raw),name in zip(rows,dm):
    path=obj[i]
    match=re.match(r'<?([a-zA-Z_][a-zA-Z0-9_]*)(?:::|\[)',name)
    if raw.startswith('__R') and match:
        origin=match[1]
    elif '.rlib(' in path or '.a(' in path:
        origin=path.split('/')[-1].split('(')[0].split('-')[0]
    else:
        origin='other'
    tot[origin]+=b
    match=re.match(r'<?ferroui_base::(property_store|styled_property|direct_property)::',name)
    if match: modules[match[1]]+=b
print('Mach-O __text',sz,'symbol bytes accounted',sum(tot.values()))
print('Largest origins (bytes, Rust names by leading crate in demangled symbol, native by archive)')
for k,v in tot.most_common(18): print(k,v)
print('Three property modules by leading path',dict(modules),'total',sum(modules.values()))
