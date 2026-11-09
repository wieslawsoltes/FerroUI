#!/usr/bin/env python3
"""Run a prebuilt benchmark binary; never compile during a measurement session."""
import argparse, hashlib, json, os, platform, statistics, subprocess, time
from pathlib import Path


def identity(path):
    p=Path(path).resolve()
    return {"path":str(p),"bytes":p.stat().st_size,"sha256":hashlib.sha256(p.read_bytes()).hexdigest()}


def load_snapshot():
    lines=subprocess.check_output(['ps','-axo','pcpu=,comm='],text=True).splitlines()
    busy=[]
    for line in lines:
        parts=line.strip().split(None,1)
        if len(parts)==2:
            try: busy.append({'cpu_percent':float(parts[0]),'process':Path(parts[1]).name})
            except ValueError: pass
    return {'load_average':list(os.getloadavg()),'top_processes':sorted(busy,key=lambda x:x['cpu_percent'],reverse=True)[:8]}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,default=Path('target/release/rendering-benchmarks'))
    parser.add_argument('--baseline-binary',type=Path,help='Optional prebuilt earlier revision; interleave before/after runs')
    parser.add_argument('--baseline-source-commit',help='Source revision used to build --baseline-binary; binary identity is recorded')
    parser.add_argument('--backends',default='skia,quartz')
    parser.add_argument('--trials',type=int,default=6)
    parser.add_argument('--samples',type=int,default=200)
    parser.add_argument('--warmup',type=int,default=30)
    parser.add_argument('--scale',type=float,default=2)
    parser.add_argument('--other-work-paused',action='store_true')
    parser.add_argument('--out',type=Path,default=Path('target/rendering-perf'))
    args=parser.parse_args()
    if args.trials<2:parser.error('Use at least two trials to expose order effects')
    args.binary=args.binary.resolve();args.out.mkdir(parents=True,exist_ok=True)
    backends=args.backends.split(',')
    enabled=subprocess.check_output([str(args.binary),'--list'],text=True).strip().split(',')
    if any(b not in enabled for b in backends):parser.error(f'Available backends: {enabled}')
    variants={b:(args.binary,b) for b in backends}
    baseline_identity=None
    baseline_commit=None
    if bool(args.baseline_binary) != bool(args.baseline_source_commit):
        parser.error('--baseline-binary and --baseline-source-commit must be supplied together')
    if args.baseline_binary:
        args.baseline_binary=args.baseline_binary.resolve()
        baseline_commit=subprocess.check_output(['git','rev-parse',args.baseline_source_commit+'^{commit}'],text=True).strip()
        baseline_enabled=subprocess.check_output([str(args.baseline_binary),'--list'],text=True).strip().split(',')
        if any(b not in baseline_enabled for b in backends):parser.error(f'Baseline backends: {baseline_enabled}')
        baseline_identity=identity(args.baseline_binary)
        variants={label:(binary,b) for b in backends for label,binary in
                  [('before-'+b,args.baseline_binary),('after-'+b,args.binary)]}
        backends=list(variants)

    source_paths = [*Path('tests/FerroUI.Rendering.Benchmarks').glob('*.rs'),
                    Path('src/FerroUI.Metal/offscreen.rs'),Path('scripts/rendering-perf/compare.py'),
                    Path('Cargo.toml'),Path('Cargo.lock')]
    source_paths += [p for p in Path('src/Quartz/FerroUI.Quartz').rglob('*')
                     if p.is_file() and p.suffix in {'.rs','.m','.h','.metal','.toml'}]
    sources_before = {str(p):identity(p) for p in source_paths}
    binary_before = identity(args.binary)
    commit_before = subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
    runs=[]
    for trial in range(args.trials):
        order=backends if trial%2==0 else list(reversed(backends))
        for backend in order:
            path=args.out/f'{trial:02}-{backend}.json'
            # The executable names BGRA captures by its actual adapter. Keep
            # revisions separate so their untimed validation images survive.
            if args.baseline_binary:
                path=args.out/backend/path.name
                path.parent.mkdir(parents=True,exist_ok=True)
            binary,adapter=variants[backend]
            command=[str(binary),'--backend',adapter,'--samples',str(args.samples),'--warmup',str(args.warmup),'--scale',str(args.scale),'--output',str(path),'--capture',str(trial==0).lower()]
            with path.with_suffix('.log').open('w') as log:
                load_before=load_snapshot()
                start=time.monotonic();result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
                load_after=load_snapshot()
            if result.returncode:raise RuntimeError(f'{backend} failed; see {path.with_suffix(".log")}')
            report=json.loads(path.read_text());runs.append({'trial':trial,'backend':backend,'seconds':time.monotonic()-start,'report':report,'artifact':identity(path),'load_before':load_before,'load_after':load_after})
            print(f'trial {trial+1}/{args.trials}: {backend} {runs[-1]["seconds"]:.1f}s',flush=True)
            time.sleep(2)
    rows=[]
    for name in [r['name'] for r in runs[0]['report']['rows']]:
        row={'name':name,'backends':{}}
        for backend in backends:
            cases=[next(c for c in r['report']['rows'] if c['name']==name) for r in runs if r['backend']==backend]
            medians=[c['completed']['median_ms'] for c in cases]
            entry={'median_ms':statistics.median(medians),'p95_ms':statistics.median(c['completed']['p95_ms'] for c in cases),'trial_medians_ms':medians,'median_range_ms':[min(medians),max(medians)]}
            if 'cpu_before_wait' in cases[0]:entry['cpu_before_wait_median_ms']=statistics.median(c['cpu_before_wait']['median_ms'] for c in cases)
            row['backends'][backend]=entry
        baseline=row['backends'][backends[0]]['median_ms']
        for entry in row['backends'].values():entry['time_relative_to_first_backend']=entry['median_ms']/baseline
        rows.append(row)
    if identity(args.binary) != binary_before:
        raise RuntimeError('Benchmark binary changed during the session')
    if baseline_identity and identity(args.baseline_binary) != baseline_identity:
        raise RuntimeError('Baseline benchmark binary changed during the session')
    if {str(p):identity(p) for p in source_paths} != sources_before:
        raise RuntimeError('Benchmark/backend source changed during the session')
    # Keep machine identity useful without serial numbers, UUIDs or other identifiers.
    machine={'os':platform.platform(),'cpu':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),'memory_bytes':int(subprocess.check_output(['sysctl','-n','hw.memsize'],text=True))}
    report={'schema':1,'source_commit':commit_before,'working_tree_modified':bool(subprocess.check_output(['git','status','--porcelain'],text=True)),'binary':binary_before,'machine':machine,'other_work_paused_by_user':args.other_work_paused,'runtime_environment':{k:os.environ.get(k) for k in ['MallocNanoZone','MallocStackLogging','RUSTFLAGS','FERROUI_QUARTZ_FORCE_SEPARATE_DESTINATION']},'source_files':sources_before,'font_files':[identity(Path('src/Skia/FerroUI.Skia/test_assets/assets')/f) for f in ['Inter-Regular.ttf','NotoSansArabic-Regular.ttf','NotoSansHebrew-Regular.ttf']],'config':{'backends':backends,'trials':args.trials,'samples':args.samples,'warmup':args.warmup,'scale':args.scale,'order':'alternating forward/reverse; fresh process per backend/trial'},'rows':rows,'runs':runs,'limitations':['Offscreen throughput; no display/vsync/input-to-photon measurements','Completed frame includes CPU encoding, queueing and GPU execution; it is not GPU timestamp time','CPU-before-wait subtracts an explicitly timed completion fence from a serial frame; not asynchronous throughput','Fixed font files, Light hinting and grayscale AA requested; backend rasterization quality can differ','Not full application startup, layout, scrolling or catalog navigation performance']}
    if baseline_identity:
        report['baseline']={'source_commit':baseline_commit,'binary':baseline_identity,'source_revision_is_caller_supplied':True}
        report['config']['order']='interleaved revisions; alternating forward/reverse; fresh process per adapter/revision/trial'
    (args.out/'comparison.json').write_text(json.dumps(report,indent=2)+'\n')
    lines=['| Workload | '+' | '.join(backends)+' |','|---|'+'---:|'*len(backends)]
    for row in rows:lines.append('| '+row['name']+' | '+' | '.join(f'{row["backends"][b]["median_ms"]:.4f} ms' for b in backends)+' |')
    (args.out/'comparison.md').write_text('\n'.join(lines)+'\n')
    print('\n'.join(lines))

if __name__=='__main__':main()
