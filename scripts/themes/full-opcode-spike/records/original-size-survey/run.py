from pathlib import Path
import os, subprocess, shutil, time, gzip
root=Path.cwd()
out=root/'target/size-investigation'
# Set SKIA_NINJA_COMMAND in the environment when Ninja is not on PATH.
env=dict(os.environ, SDKROOT=subprocess.check_output(['xcrun','--sdk','macosx','--show-sdk-path'],text=True).strip())
for label,profile,flags in [
  ('skia-dist','dist',[]),('quartz-metal-dist','dist',['--no-default-features','--features','quartz-metal']),
  ('skia-release','release',[]),('skia-size-s','size-s',['--config','profile.size-s.inherits="dist"','--config','profile.size-s.opt-level="s"'])]:
    cmd=['cargo','rustc','--locked','--profile',profile,'-p','hello-world','--bin','hello-world',*flags,'--','-C','link-arg=-Wl,-map,'+str(out/(label+'.map'))]
    print(label, ' '.join(cmd), flush=True)
    with (out/(label+'.log')).open('w') as log:
        res=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
    if res.returncode:
        print(label,'FAILED',res.returncode,flush=True)
        continue
    dest=out/label
    shutil.copy2(root/'target'/profile/'hello-world',dest)
    data=dest.read_bytes()
    print(label, len(data), 'gzip', len(gzip.compress(data,mtime=0)), flush=True)
    if profile=='release':
        stripped=out/'skia-release-stripped'
        subprocess.run(['strip','-o',stripped,dest],check=True)
        print('release stripped',stripped.stat().st_size,flush=True)
    try:
        with (out/(label+'.smoke')).open('w') as log:
            res=subprocess.run([dest],env=dict(env,FERROUI_SMOKE_EXIT_MS='800'),stdout=log,stderr=subprocess.STDOUT,timeout=18)
        print(label,'smoke',res.returncode,flush=True)
    except subprocess.TimeoutExpired:
        print(label,'smoke timed out',flush=True)
