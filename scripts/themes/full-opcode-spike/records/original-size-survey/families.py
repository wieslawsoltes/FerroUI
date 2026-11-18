from pathlib import Path
p=Path(__file__).with_name('inspect_map.py')
exec(p.read_text().split('crates=Counter()')[0])
families=Counter()
fc=Counter()
modules=Counter()
for (_,b,i,raw),name in zip(rows,dm):
    if not raw.startswith('__R'): continue
    for x in ['property_store','styled_property','direct_property','metadata_table']:
        if name.startswith(('ferroui_base::'+x+'::','<ferroui_base::'+x+'::')):
            modules[x]+=b
            break
    if not name.startswith(('ferroui_base::','<ferroui_base::')): continue
    key=name
    if key.startswith('<'):
        key=key[1:]
        for _ in range(16):
            changed=re.sub(r'<[^<>]*>', '',key)
            if changed==key: break
            key=changed
        key=key.split(' as ')[0]+'::'+key.split('>::')[-1] if ' as ' in key else key.replace('>::','::')
    else:
        for _ in range(16):
            changed=re.sub(r'<[^<>]*>', '',key)
            if changed==key: break
            key=changed
    key=key.removesuffix('::')
    if '{closure' not in key:
        families[key]+=b; fc[key]+=1
print('Modules by literal path prefix',modules)
for k,v in families.most_common(22): print(v,fc[k],k[:180])
