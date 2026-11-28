from pathlib import Path
from collections import Counter, defaultdict
import subprocess,sys,hashlib,mmap
root=Path.cwd()
paths=subprocess.check_output(['git','ls-files','-z','--','src','samples','components']).decode().split('\0')
ext={'.png','.jpeg','.jpg','.webp','.bmp','.gif','.ico','.icns','.ttf','.otf','.ttc','.woff','.woff2','.mp4','.m4a','.mov','.wav','.svg','.json','.lottie','.onnx','.fxc'}
seen={}
for p in paths:
    f=Path(p)
    if f.suffix.lower() not in ext or f.stat().st_size<128: continue
    d=f.read_bytes()
    if d not in seen: seen[d]=[]
    seen[d].append(str(p))
with open(sys.argv[1],'rb') as f:
 m=mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ)
 # Index by representative sequence, then validate the whole payload at that location.
 hits=[]
 for d,paths in seen.items():
    off=m.find(d)
    if off>=0: hits.append((off,len(d),paths))
 hits.sort(key=lambda x:(x[0],-x[1]))
 totals=Counter(); end=0; unique=0
 for off,n,paths in hits:
    kept=max(0,off+n-max(off,end)); end=max(end,off+n)
    totals['/'.join(paths[0].split('/')[:2])]+=kept; unique+=kept
 print('Distinct source payloads with exact byte matches',len(hits),'nonoverlapping bytes',unique)
 print('Source path groups',totals.most_common(16))
 print('Largest individual payloads')
 for o,n,paths in sorted(hits,key=lambda x:-x[1])[:15]: print(n,' '.join(paths[:2]))
