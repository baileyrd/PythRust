"""#809 one-variant SHA pilot: same-host interleaved ordinary release timings.
Build fresh locked baseline/candidate first; no tracing, new packages or user data.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import random
import re
import ssl
import sys
import tempfile
import time

import profile_workloads as p

ROOT = Path(__file__).resolve().parent.parent
SHA = Path('crates/pythrust-rt/src/sha256.rs')


def checked(cmd, expected=None, **kwargs):
    done = p.run(cmd, **kwargs)
    if done.stderr or (expected is not None and done.stdout != expected):
        raise ValueError(f'parity/stderr failure: {cmd}: {done.stdout!r} {done.stderr!r}')
    return done.stdout


def order(seed, runs, kinds):
    rng = random.Random(seed)
    rows = []
    for _ in range(runs):
        row = list(kinds)
        rng.shuffle(row)
        rows.append(row)
    return rows


def interleaved(commands, expected, cpus, runs, seed, micro=False):
    samples = {kind: [] for kind in commands}
    internal = {kind: [] for kind in commands}
    rounds = order(seed, runs, commands)
    for i, kinds in enumerate([list(commands), *rounds]):
        for kind in kinds:
            start = time.perf_counter()
            out = checked(['taskset', '-c', cpus, *commands[kind]], None if micro else expected)
            wall = time.perf_counter() - start
            if micro:
                found = re.findall(rb'sha256_micro_seconds=([0-9.]+)', out)
                if len(found) != 1 or b'1 passed; 0 failed' not in out:
                    raise ValueError('missing/failed microbenchmark')
                if i:
                    internal[kind].append(float(found[0]))
            if i:
                samples[kind].append(wall)
    return dict(order=rounds, wall={k: p.stats(v) for k, v in samples.items()},
                internal_seconds={k: p.stats(v) for k, v in internal.items()} if micro else {})


def identity(checkout):
    compiler = checkout / 'target/release/pythrust'
    runtime = compiler.parent / 'libpythrust_rt.a'
    files = [SHA, Path('Cargo.lock')]
    return dict(checkout=str(checkout), commit=checked(['git','rev-parse','HEAD'],cwd=checkout).decode().strip(),
                tree=checked(['git','rev-parse','HEAD^{tree}'],cwd=checkout).decode().strip(),
                diff=checked(['git','diff','--',str(SHA)],cwd=checkout).decode(),
                source_sha256={str(f):p.sha((checkout/f).read_bytes()) for f in files},
                compiler_sha256=p.sha(compiler.read_bytes()), runtime_sha256=p.sha(runtime.read_bytes()))


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--baseline', type=Path, required=True)
    ap.add_argument('--candidate', type=Path, default=ROOT)
    ap.add_argument('--output', type=Path, required=True)
    ap.add_argument('--runs', type=int, default=15)
    args = ap.parse_args()
    if sys.version_info[:2] != (3,13) or sys.platform != 'linux' or args.runs < 7:
        ap.error('requires Linux, CPython 3.13 and at least seven samples')
    for key in ('PYTHRUST_RT','PYTHRUST_RT_SRC','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','LD_PRELOAD'):
        if os.environ.get(key):
            ap.error('uncontrolled environment: '+key)
    checkouts = dict(baseline=args.baseline.resolve(), candidate=args.candidate.resolve())
    allowed = sorted(os.sched_getaffinity(0))
    if len(allowed) < 4:
        ap.error('four permitted CPUs required')
    affinities = [str(allowed[0]), ','.join(map(str,allowed[:4]))]
    result = dict(schema=809, generated_at=datetime.now(timezone.utc).isoformat(), command=sys.argv,
                  runs=args.runs, warmups=1, seed=809, affinities=affinities,
                  environment=dict(platform=platform.platform(), python=sys.version, python_path=sys.executable,
                    python_sha256=p.sha(Path(sys.executable).read_bytes()), openssl=ssl.OPENSSL_VERSION,
                    hash_module=p.hashlib.sha256.__module__, rustc=checked(['rustc','-Vv']).decode(),
                    cc=checked(['cc','--version']).decode(), allowed_cpus=allowed,
                    cpu=Path('/proc/cpuinfo').read_text().split('model name',1)[1].splitlines()[0],
                    filesystem=checked(['df','-T','/tmp']).decode(),
                    build='cargo build --locked --release -p pythrust-compiler -p pythrust-rt',
                    limits='warm synthetic tmpfs; shared host; whole-process wall; no phase subtraction or tracing'),
                  identities={k:identity(v) for k,v in checkouts.items()}, inputs={}, cases={}, micro={})
    result['harness_sha256'] = {f:p.sha((ROOT/'bench'/f).read_bytes()) for f in
                              ('sha256_pilot.py','profile_workloads.py')}
    with tempfile.TemporaryDirectory(prefix='pythrust-809-') as temp:
        tmp = Path(temp)
        datasets = {}
        for name,count,size,dup,nested in [('large_dup',8,4*1024*1024,True,False),
                ('large_unique',8,4*1024*1024,False,False),('small_dup',256,4096,True,True),
                ('small_unique',256,4096,False,True)]:
            datasets[name] = tmp/name
            manifest=p.generate_tree(datasets[name],count=count,size=size,duplicates=dup,nested=nested)
            result['inputs'][name]=dict(manifest=manifest,manifest_sha256=p.sha(json.dumps(manifest,sort_keys=True).encode()))
        sources = dict(startup='pass\n')
        for phase in ('discover','read','loaded_sha256'):
            sources['phase_'+phase] = p.phase_source(phase,8)
        for name in ('fclone_lite','fclone_tasks'):
            sources[name] = (ROOT/'apps/fclone-lite'/f'{name}.py').read_text()
        prior=json.loads((ROOT/'docs/benchmarks/card808.json').read_text())
        assert sources['phase_loaded_sha256'] == prior['cases']['phase_loaded_sha256']['source']
        for name,source in sources.items():
            src = tmp/(name+'.pyr'); src.write_text(source)
            binaries={}
            for kind,checkout in checkouts.items():
                work=tmp/(name+'-'+kind); work.mkdir()
                copy=work/'case.pyr'; copy.write_text(source)
                compiler=checkout/'target/release/pythrust'
                env=dict(os.environ,PYTHRUST_RT=str(compiler.parent/'libpythrust_rt.a'))
                checked([compiler,'build',copy],b'',env=env)
                binaries[kind]=copy.with_suffix('')
            row=dict(source=source,source_sha256=p.sha(source.encode()),
                     binary_sha256={k:p.sha(b.read_bytes()) for k,b in binaries.items()},executions={})
            cases = datasets.items() if name.startswith('fclone_') else [('large_dup',datasets['large_dup'])] if name.startswith('phase_') else [('default',None)]
            for dataset,directory in cases:
                argv=[] if directory is None else [directory]
                want=checked([sys.executable,src,*argv])
                if name.startswith('fclone_') and want != p.expected_groups(directory,result['inputs'][dataset]['manifest']):
                    raise ValueError('independent group oracle mismatch')
                # Interpreter parity is checked separately, outside timing.
                for kind,checkout in checkouts.items():
                    checked([checkout/'target/release/pythrust','run',src,*argv],want)
                commands={k:[b,*argv] for k,b in binaries.items()}
                commands['cpython']=[sys.executable,src,*argv]
                for cpus in affinities:
                    key=dataset+'@'+cpus
                    row['executions'][key]=dict(stdout=want.decode().replace(str(tmp),'<DATA>'),
                        stdout_sha256=p.sha(want),**interleaved(commands,want,cpus,args.runs,809))
                    timing=row['executions'][key]['wall']
                    print(name,key,{k:round(v['median'],6) for k,v in timing.items()},flush=True)
            result['cases'][name]=row
            args.output.write_text(json.dumps(result,indent=2)+'\n')
        micro=(checkouts['candidate']/SHA).read_text().split('/// Opt-in ordinary release microbenchmarks',1)[1]
        micro='/// Opt-in ordinary release microbenchmarks'+micro
        microbins={}
        for kind,checkout in checkouts.items():
            source=(checkout/SHA).read_text().split('/// Opt-in ordinary release microbenchmarks',1)[0]+micro
            src=tmp/(kind+'-micro.rs'); src.write_text(source)
            binary=src.with_suffix('')
            checked(['rustc','--edition=2021','-O','--test',src,'-o',binary])
            microbins[kind]=binary
            result['identities'][kind]['micro_source_sha256']=p.sha(source.encode())
            result['identities'][kind]['micro_binary_sha256']=p.sha(binary.read_bytes())
        result['micro_module']=micro
        for case in ('compression','loaded_digest'):
            commands={k:[v,'--ignored','--exact','microbench::'+case,'--nocapture'] for k,v in microbins.items()}
            result['micro'][case]=interleaved(commands,None,affinities[0],args.runs,809,micro=True)
        args.output.write_text(json.dumps(result,indent=2)+'\n')


if __name__ == '__main__':
    main()
