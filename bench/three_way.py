"""#813: matched whole-process PythRust/CPython/handwritten Rust measurements.

Prepare once, then run without rebuilding. No dependency beyond Python stdlib.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import platform
import random
import resource
import shutil
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = dict(numeric="131434272 524\n", sorting="True 6 499909 999999\n",
                strings="9785790 20000\n", tuples="5888889 27000000\n", startup="")
KINDS = ('pythrust', 'cpython', 'rust')
FLAGS = ['--edition=2021', '-C', 'opt-level=3', '-C', 'overflow-checks=off',
         '-C', 'codegen-units=1', '-C', 'target-cpu=generic']


def digest(data):
    return hashlib.sha256(data).hexdigest()


def identity(path):
    return dict(path=str(path), sha256=digest(Path(path).read_bytes()))


def run(cmd, *, env=None, timeout=180):
    done = subprocess.run([str(x) for x in cmd], capture_output=True, env=env, timeout=timeout)
    if done.returncode:
        raise RuntimeError(f'{cmd}: exit {done.returncode}: {done.stderr.decode(errors="replace")}')
    return done


def stats(values):
    if not values or any(type(x) not in (float, int) or not math.isfinite(x) or x <= 0 for x in values):
        raise ValueError('nonempty finite positive wall samples required')
    med = statistics.median(values)
    return dict(median=med, mad=statistics.median(abs(x-med) for x in values),
                minimum=min(values), maximum=max(values))


def order(kinds, rounds, seed):
    permutations = list(itertools.permutations(kinds))
    if rounds < len(permutations) or rounds % len(permutations):
        raise ValueError('round count must be a positive multiple of permutation count')
    rng = random.Random(seed)
    result = []
    for _ in range(rounds // len(permutations)):
        block = permutations.copy()
        rng.shuffle(block)
        result.extend(block)
    return [list(x) for x in result]


def pin(cpu):
    if cpu not in os.sched_getaffinity(0):
        raise ValueError('CPU outside permitted affinity')
    os.sched_setaffinity(0, {cpu})
    if os.sched_getaffinity(0) != {cpu}:
        raise ValueError('affinity was not applied')


def sample(cmd, expected, *, env=None):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter_ns()
    done = run(cmd, env=env)
    elapsed = (time.perf_counter_ns() - start) / 1e9
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    if done.stdout != expected.encode() or done.stderr:
        raise ValueError(f'output mismatch for {cmd}: {done.stdout!r} / {done.stderr!r}')
    return dict(wall_seconds=elapsed, user_seconds=after.ru_utime-before.ru_utime,
                system_seconds=after.ru_stime-before.ru_stime,
                switches=[after.ru_nvcsw-before.ru_nvcsw, after.ru_nivcsw-before.ru_nivcsw],
                stdout=done.stdout.decode(), stderr=done.stderr.decode(), returncode=done.returncode)


def save(path, record):
    # Task-owned directory; replace checkpoints atomically, retain failure status.
    tmp = path.with_suffix(path.suffix+'.tmp')
    tmp.write_text(json.dumps(record, indent=2, allow_nan=False)+'\n')
    tmp.replace(path)


def source_record():
    names = run(['git', 'ls-files', 'crates', 'Cargo.toml', 'Cargo.lock']).stdout.decode().splitlines()
    hashes = {p: digest((ROOT/p).read_bytes()) for p in names}
    return dict(head=run(['git','rev-parse','HEAD']).stdout.decode().strip(),
                tree=run(['git','rev-parse','HEAD^{tree}']).stdout.decode().strip(),
                tracked_diff=run(['git','diff','HEAD']).stdout.decode(), build_inputs=hashes,
                index_diff=run(['git','diff','--cached']).stdout.decode(),
                status=run(['git','status','--porcelain=v1','--untracked-files=all']).stdout.decode())


def host():
    files = {}
    for p in ['/proc/cpuinfo', '/proc/self/cgroup', '/proc/self/mountinfo',
              '/sys/fs/cgroup/cpu.max', '/sys/fs/cgroup/cpuset.cpus.effective']:
        try:
            files[p] = Path(p).read_text()
        except OSError as exc:
            files[p] = f'unavailable: {exc}'
    return dict(platform=platform.platform(), libc=platform.libc_ver(),
                python=sys.version, python_executable=sys.executable,
                python_executable_identity=identity(Path(sys.executable).resolve()),
                gil_enabled=getattr(sys, '_is_gil_enabled', lambda: 'unknown')(),
                load=os.getloadavg(), permitted_affinity=sorted(os.sched_getaffinity(0)), files=files,
                caveats='shared host; no exclusive CPU/frequency control; warm source/binary caches; quota unknown if unavailable')


def prepare(args):
    compiler = args.compiler.resolve(strict=True)
    rustc = args.rustc.resolve(strict=True)
    runtime = compiler.parent/'libpythrust_rt.a'
    if not runtime.is_file():
        raise ValueError('matching adjacent release runtime required')
    original_host = host()
    pin(args.cpu)
    args.work.mkdir()  # Never overwrite an existing prepared condition.
    env = dict(os.environ, PYTHRUST_RT=str(runtime))
    record = dict(schema=2, control_build_protocol=1, status='preparing', timestamp=datetime.now(timezone.utc).isoformat(),
                  source=source_record(), host=original_host, cpu=args.cpu,
                  compiler=identity(compiler), runtime=identity(runtime),
                  rustc=run([rustc,'-Vv']).stdout.decode(), cargo=run(['cargo','-V']).stdout.decode(),
                  cc=run(['cc','--version']).stdout.decode(), linker=run(['ld','--version']).stdout.decode(),
                  rust_flags=FLAGS, command=sys.argv, conditions={}, builds={}, sources={},
                  harness=identity(Path(__file__).resolve()),
                  environment={k:os.environ.get(k) for k in ('PATH','CARGO_TARGET_DIR','RUSTFLAGS',
                      'CARGO_ENCODED_RUSTFLAGS','PYTHRUST_RT','PYTHRUST_RT_SRC','LD_PRELOAD',
                      'LD_LIBRARY_PATH','PYTHONHASHSEED','PYTHONPATH','MALLOC_CONF','GLIBC_TUNABLES')},
                  tools={k:identity(Path(shutil.which(k)).resolve()) for k in ('cargo','cc','ld')},
                  rustc_identity=identity(rustc), domain=run([sys.executable,'-B',ROOT/'bench/check_three_way_domain.py']).stdout.decode())
    manifest = args.work/'prepared.json'
    try:
        for name, expected in EXPECTED.items():
            pyr = args.work/(name+'.pyr')
            pyr.write_bytes(b'pass\n' if name == 'startup' else (ROOT/'bench'/pyr.name).read_bytes())
            rs = args.work/(name+'.rs')
            rs.write_bytes((ROOT/'bench/rust'/rs.name).read_bytes())
            record['sources'][name] = dict(pythrust=identity(pyr), rust=identity(rs),
                                           pythrust_source=pyr.read_text(), rust_source=rs.read_text())
            native = pyr.with_suffix('')
            rb = args.work/(name+'-rust')
            cmds = dict(pythrust=[str(compiler),'build',str(pyr)],
                        rust=[str(rustc),*FLAGS,str(rs),'-o',str(rb)])
            for kind, cmd in cmds.items():
                run(cmd, env=env)
                timings = []
                for _ in range(7):
                    start=time.perf_counter_ns(); run(cmd, env=env)
                    timings.append((time.perf_counter_ns()-start)/1e9)
                record['builds'][name+'/'+kind] = dict(command=cmd, samples=timings, **stats(timings))
            control_builds = {}
            conditions = dict(pythrust=[str(native)], cpython=[sys.executable,'-B',str(pyr)], rust=[str(rb)])
            if name != 'startup':
                checked = args.work/(name+'-rust-checked')
                flags=FLAGS.copy(); flags[flags.index('overflow-checks=off')]='overflow-checks=on'
                control_builds['rust_checked'] = [str(rustc), *flags, str(rs), '-o', str(checked)]
                run(control_builds['rust_checked'])
                conditions['rust_checked']=[str(checked)]
                test = args.work/(name+'-rust-tests')
                run([rustc,'--edition=2021','-O','-C','overflow-checks=on','--test',rs,'-o',test])
                record.setdefault('rust_tests',{})[name]=run([test]).stdout.decode()
                run([rustc,*FLAGS,'--emit=asm',rs,'-o',args.work/(name+'.s')])
            if name == 'strings':
                mat=args.work/'strings-rust-materialized'
                control_builds['rust_materialized'] = [str(rustc), *FLAGS, '--cfg', 'materialize', str(rs), '-o', str(mat)]
                run(control_builds['rust_materialized'])
                conditions['rust_materialized']=[str(mat)]
            for kind, cmd in conditions.items():
                sample(cmd,expected,env=env)
            if name != 'startup':
                small=args.work/(name+'-small.pyr')
                small.write_text(pyr.read_text().replace('1000000','1000').replace('300000','100'))
                oracle=run([sys.executable,'-B',small])
                interpreted=run([compiler,'run',small],env=env)
                if oracle.stderr or interpreted.stderr or oracle.stdout!=interpreted.stdout:
                    raise ValueError('reduced interpreter parity failed: '+name)
                record.setdefault('interpreter_checks',{})[name]=dict(source=small.read_text(),stdout=oracle.stdout.decode())
            record['conditions'][name] = {k:dict(command=v, executable=identity(Path(v[0]).resolve()))
                                          for k,v in conditions.items()}
            for kind, command in control_builds.items():
                record['conditions'][name][kind]['build_command'] = command
            record['status']='preparing'
            save(manifest,record)
        record['status']='prepared'
    except BaseException as exc:
        record['status']='failed'; record['error']=repr(exc)
        raise
    finally:
        save(manifest,record)
    print('Prepared',manifest,flush=True)


def check_identities(prepared):
    for what in ('compiler','runtime','harness'):
        item=prepared[what]
        if identity(Path(item['path'])) != item:
            raise ValueError('stale '+what)
    for row in prepared['sources'].values():
        for kind in ('pythrust','rust'):
            if identity(Path(row[kind]['path'])) != row[kind]:
                raise ValueError('stale source')
    for row in prepared['conditions'].values():
        for item in row.values():
            if identity(Path(item['executable']['path'])) != item['executable']:
                raise ValueError('stale executable')
    for path,sha in prepared['source']['build_inputs'].items():
        if digest((ROOT/path).read_bytes()) != sha:
            raise ValueError('changed production build input: '+path)


def measure(args):
    prepared=json.loads((args.work/'prepared.json').read_text())
    if prepared['status']!='prepared':
        raise ValueError('preparation incomplete')
    if args.output.exists():
        raise FileExistsError(args.output)
    check_identities(prepared)
    pin(prepared['cpu'])
    result=dict(schema=prepared['schema'],status='running',timestamp=datetime.now(timezone.utc).isoformat(),
                prepared=prepared,command=sys.argv,runs=args.runs,seed=args.seed,warmups=2,cases={})
    env=dict(os.environ,PYTHRUST_RT=prepared['runtime']['path'])
    try:
        for name, expected in EXPECTED.items():
            cond=prepared['conditions'][name]
            passes=[('primary',list(KINDS),args.runs)]
            if name!='startup': passes.append(('checked',['rust','rust_checked'],args.runs))
            if name=='strings': passes.append(('materialized',['rust','rust_materialized'],args.runs))
            for label,kinds,count in passes:
                key=name+'/'+label
                row=dict(order=order(kinds,count,args.seed+len(result['cases'])),warmup_samples=[],samples=[],summary={})
                result['cases'][key]=row
                for kind in kinds:
                    for warmup in range(2):
                        row['warmup_samples'].append(dict(kind=kind,warmup=warmup,
                            **sample(cond[kind]['command'],expected,env=env)))
                for round_id, sequence in enumerate(row['order']):
                    for kind in sequence:
                        row['samples'].append(dict(round=round_id,kind=kind,
                            **sample(cond[kind]['command'],expected,env=env)))
                    save(args.output,result)
                for kind in kinds:
                    row['summary'][kind]=stats([x['wall_seconds'] for x in row['samples'] if x['kind']==kind])
                print(key,row['summary'],flush=True)
        check_identities(prepared)
        result['status']='complete'; result['end_load']=os.getloadavg()
        result['post_identities']={k:identity(Path(prepared[k]['path'])) for k in ('compiler','runtime','harness')}
    except BaseException as exc:
        result['status']='failed'; result['error']=repr(exc)
        raise
    finally:
        save(args.output,result)


def verify(data):
    if data['schema'] not in (1,2) or data['status']!='complete' or data['prepared']['status']!='prepared':
        raise ValueError('incomplete bundle')
    expected_keys={name+'/primary' for name in EXPECTED}
    expected_keys.update(name+'/checked' for name in EXPECTED if name!='startup')
    expected_keys.add('strings/materialized')
    if set(data['cases'])!=expected_keys:
        raise ValueError('wrong case set')
    for index,(key,row) in enumerate(data['cases'].items()):
        name,label=key.split('/')
        kinds=list(KINDS) if label=='primary' else ['rust','rust_'+label]
        if row['order']!=order(kinds,data['runs'],data['seed']+index):
            raise ValueError('wrong order')
        ids=[(i,k) for i,seq in enumerate(row['order']) for k in seq]
        if [(x['round'],x['kind']) for x in row['samples']]!=ids:
            raise ValueError('missing/duplicate/out-of-order samples')
        for item in row['samples']:
            if item['stdout']!=EXPECTED[name] or item['stderr'] or item['returncode']!=0:
                raise ValueError('bad output')
            for field in ('user_seconds','system_seconds'):
                if type(item[field]) not in (float,int) or not math.isfinite(item[field]) or item[field]<0:
                    raise ValueError('bad resources')
            if len(item['switches'])!=2 or any(type(x) is not int or x<0 for x in item['switches']):
                raise ValueError('bad switches')
        for kind in kinds:
            actual=stats([x['wall_seconds'] for x in row['samples'] if x['kind']==kind])
            if actual!=row['summary'][kind]: raise ValueError('wrong statistics')
    return len(data['cases'])


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--prepare',action='store_true')
    ap.add_argument('--work',type=Path)
    ap.add_argument('--compiler',type=Path)
    ap.add_argument('--rustc',type=Path)
    ap.add_argument('--cpu',type=int,default=0)
    ap.add_argument('--runs',type=int,default=18)
    ap.add_argument('--seed',type=int,default=813)
    ap.add_argument('--output',type=Path)
    ap.add_argument('--verify',type=Path)
    args=ap.parse_args()
    if args.verify:
        print('Verified cases:',verify(json.loads(args.verify.read_text()))); return
    if sys.platform!='linux' or sys.version_info[:2]!=(3,13):
        ap.error('Linux and CPython 3.13 required')
    for key in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','PYTHRUST_RT_SRC','LD_PRELOAD'):
        if os.environ.get(key): ap.error('uncontrolled environment: '+key)
    if not args.work or not args.work.is_absolute(): ap.error('absolute --work required')
    if args.prepare:
        if not args.compiler or not args.rustc: ap.error('--compiler and --rustc required')
        prepare(args)
    else:
        order(KINDS,args.runs,args.seed)
        if not args.output: ap.error('--output required')
        measure(args)


if __name__=='__main__':
    main()
