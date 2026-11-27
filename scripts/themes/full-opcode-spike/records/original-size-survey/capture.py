import subprocess, sys, time, os
from pathlib import Path
out=Path(__file__).resolve().parent
for binary,label in [('hello','full'),('hello_size_probe','trimmed')]:
    if label not in sys.argv[1:]: continue
    exe=out.parent/'dist/examples'/binary if label=='full' else out/'fluent-trim-dist'
    with (out/(label+'.visual-smoke')).open('w') as log:
        proc=subprocess.Popen([exe],env=dict(os.environ,FERROUI_SMOKE_EXIT_MS='7000'),stdout=log,stderr=subprocess.STDOUT)
        try:
            for _ in range(45):
                win=subprocess.run([out/'window_id',str(proc.pid)],capture_output=True,text=True)
                if win.returncode==0: break
                if proc.poll() is not None: break
                time.sleep(0.1)
            if win.returncode: raise SystemExit('Window missing: '+label)
            time.sleep(1.2)
            subprocess.run(['screencapture','-x','-o','-l',win.stdout.strip(),out/(label+'.png')], check=True)
        finally:
            proc.wait(timeout=12)
