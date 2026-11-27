import sys, subprocess, re
from pathlib import Path
from collections import Counter
data=Path(sys.argv[1]).read_text(errors='replace')
live=data.split('# Dead Stripped Symbols:')[0]
obj={int(i):p for i,p in re.findall(r'^\[\s*(\d+)\]\s+(.+)$',live,re.M)}
sects=[(int(a,16),int(b,16),s,t) for a,b,s,t in re.findall(r'^(0x\w+)\s+(0x\w+)\s+(\S+)\s+(\S+)$',live.split('# Sections:')[-1].split('# Symbols:')[0],re.M)]
base,sz=next((a,b) for a,b,s,t in sects if t=='__text')
rows=[(int(a,16),int(b,16),int(i),n) for a,b,i,n in re.findall(r'^(0x\w+)\s+(0x\w+)\s+\[\s*(\d+)\]\s+(.+)$',live,re.M) if base <= int(a,16) < base+sz and int(b,16)]
names=[n[1:] if n.startswith('_') else n for a,b,i,n in rows]
dm=subprocess.run(['xcrun','llvm-cxxfilt','-n'], input='\n'.join(names)+'\n',text=True,capture_output=True,check=True).stdout.splitlines()
crates=Counter()
native=Counter()
prop=Counter()
counts=Counter()
families=Counter()
fc=Counter()
for (a,b,i,raw),name in zip(rows,dm):
    m=re.search(r'(?<![\w])((?:ferroui_[a-z0-9_]+|std|alloc|core|hashbrown|smallvec|hello_world|sample_support))(?:::|\[)',name)
    o=obj.get(i,'?')
    if raw.startswith(('__R','__ZN')) and m:
        origin=m[1]
    else:
        origin=re.sub(r'\(.*','',o.split('/')[-1])
    crates[origin]+=b
    for mech in ['::property_store::','::styled_property::','::direct_property::','::metadata_table::']:
        if mech in name:
            prop[mech]+=b
            counts[mech]+=1
            break
    key=name
    for _ in range(14):
        z=re.sub(r'<[^<>]*>','',key)
        if z==key: break
        key=z
    key=re.sub(r'\[[0-9a-f]+\]','',key)
    if 'ferroui_base' in key and not key.startswith('::'):
        families[key]+=b
        fc[key]+=1
print('text',sz,'accounted',sum(crates.values()),'symbols',len(rows))
print(crates.most_common(26))
print(prop,counts)
for k,v in families.most_common(22):
    print(v,fc[k],k[:200])
