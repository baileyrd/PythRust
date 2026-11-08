"""#808: strict same-host timing, deterministic fixtures and separate GNU profiles.
No production source changes; Linux, CPython 3.13, GNU cc/gprof required.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import resource
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent


def sha(data):
    return hashlib.sha256(data).hexdigest()


def run(cmd, *, env=None, cwd=None):
    done = subprocess.run([str(x) for x in cmd], env=env, cwd=cwd,
                          capture_output=True, timeout=180)
    if done.returncode:
        raise RuntimeError(f'failed ({done.returncode}) {cmd}: {done.stderr.decode(errors="replace")}')
    return done


def stats(samples):
    if not samples or any(not math.isfinite(x) or x <= 0 for x in samples):
        raise ValueError('expected nonempty finite positive samples')
    median = statistics.median(samples)
    return dict(samples=samples, median=median, minimum=min(samples), maximum=max(samples),
                mad=statistics.median(abs(x - median) for x in samples))


def measure(cmd, count, expected, *, env=None, cwd=None, stderr=b'', warmups=1):
    if count < 1 or warmups < 0:
        raise ValueError('invalid sample count')
    elapsed, user, system, switches = [], [], [], []
    for i in range(warmups + count):
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        start = time.perf_counter()
        done = run(cmd, env=env, cwd=cwd)
        wall = time.perf_counter() - start
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        if done.stdout != expected or done.stderr != stderr:
            raise ValueError(f'output mismatch: {cmd}: {done.stdout!r}, {done.stderr!r}')
        if i < warmups:
            continue
        elapsed.append(wall)
        user.append(after.ru_utime - before.ru_utime)
        system.append(after.ru_stime - before.ru_stime)
        switches.append([after.ru_nvcsw - before.ru_nvcsw,
                         after.ru_nivcsw - before.ru_nivcsw])
    return dict(wall=stats(elapsed), user_seconds=user, system_seconds=system,
                context_switches_voluntary_involuntary=switches)


def generate_tree(root, *, count, size, duplicates, nested=False, unique_sizes=False):
    """Create only a new task-owned directory; each equal payload has a known ID."""
    if count < 1 or size < 1 or (duplicates and unique_sizes):
        raise ValueError('invalid fixture shape')
    root.mkdir()  # Refuse existing data, symlinks included.
    entries = []
    for i in range(count):
        payload_id = i // 2 if duplicates else i
        length = size + (i if unique_sizes else (i // 2 % 4 if duplicates else i % 4))
        block = hashlib.shake_256(f'pythrust-808-v1:{payload_id}'.encode()).digest(4096)
        data = (block * ((length + 4095) // 4096))[:length]
        rel = Path(f'd{i % 8:02d}' if nested else '.') / f'f{i:05d}'
        path = root / rel
        path.parent.mkdir(exist_ok=True)
        path.write_bytes(data)
        entries.append(dict(path=str(rel), bytes=length, sha256=sha(data)))
    return entries


def expected_groups(root, manifest):
    groups = {}
    for item in manifest:
        groups.setdefault((item['bytes'], item['sha256']), []).append(str(root / item['path']))
    blocks = []
    for (size, digest), paths in sorted(groups.items()):
        if len(paths) > 1:
            blocks.append(str(size) + ' bytes:\n' + ''.join('  ' + p + '\n' for p in sorted(paths)))
    return '\n'.join(blocks).encode()


def phase_source(mode, repeats=1):
    """Controlled discovery/read/loaded-SHA diagnostics, not subtraction estimates."""
    collect = (ROOT / 'apps/fclone-lite/fclone_lite.py').read_text().split('def collect', 1)[1].split('\ndef digest', 1)[0]
    source = 'import hashlib\nimport os\nimport sys\n\ndef collect' + collect
    source += '\ndef main() -> None:\n    by_size: dict[int, list[str]] = {}\n    collect(sys.argv[1], by_size)\n'
    if mode == 'discover':
        source += '    n = 0\n    total = 0\n    for size in sorted(by_size):\n        n += len(by_size[size])\n        total += size * len(by_size[size])\n    print(n, total)\n'
    elif mode == 'read':
        source += '    n = 0\n    for size in sorted(by_size):\n        for path in by_size[size]:\n            f = open(path, "rb")\n            data = f.read()\n            f.close()\n            n += len(data)\n    print(n)\n'
    elif mode == 'loaded_sha256':
        source += '    buffers: list[bytes] = []\n    for size in sorted(by_size):\n        for path in by_size[size]:\n            f = open(path, "rb")\n            buffers.append(f.read())\n            f.close()\n    last = ""\n    total = 0\n'
        source += f'    for i in range({repeats}):\n        for data in buffers:\n            last = hashlib.sha256(data).hexdigest()\n            total += len(last)\n    print(total, last)\n'
    else:
        raise ValueError(mode)
    return source + '\nmain()\n'


def granularity_source(chunks, parallel):
    if chunks not in (8, 128, 8192):
        raise ValueError(chunks)
    size = 3276800 // chunks
    source = 'import concurrent.futures\n\ndef work(seed: int) -> int:\n    n = 0\n'
    source += f'    for i in range({size}):\n        n += i % 97\n    return n\n\ndef main() -> None:\n    total = 0\n'
    if parallel:
        source += f'    with concurrent.futures.ThreadPoolExecutor() as ex:\n        for n in ex.map(work, range({chunks})):\n            total += n\n'
    else:
        source += f'    for seed in range({chunks}):\n        total += work(seed)\n'
    return source + '    print(total)\n\nmain()\n'


def write_linker(directory, cc, extra, log=None):
    directory.mkdir()
    wrapper = directory / 'cc'
    # exec-list arguments, never shell text; logging measures nested link wall time.
    script = '#!' + sys.executable + '\nimport subprocess,sys,time\n'
    script += 'start=time.perf_counter()\np=subprocess.run(' + repr([cc]) + '+sys.argv[1:]+' + repr(extra) + ')\n'
    if log:
        script += 'with open(' + repr(str(log)) + ',"a") as f: f.write(str(time.perf_counter()-start)+"\\n")\n'
    script += 'sys.exit(p.returncode)\n'
    wrapper.write_text(script)
    wrapper.chmod(0o700)
    return dict(os.environ, PATH=str(directory) + os.pathsep + os.environ['PATH'])


def build(compiler, src, count, env, tmp, cc):
    log = tmp / (src.stem + '-links.txt')
    wrapped = write_linker(tmp / (src.stem + '-linker'), cc, [], log)
    wrapped.update({k: env[k] for k in ('PYTHRUST_RT',)})
    command = [compiler, 'build', src]
    first = run(command, env=env)
    if first.stdout or first.stderr:
        raise ValueError(f'unexpected build output: {first}')
    times = measure(command, count, b'', env=env)
    measure(command, count, b'', env=wrapped, warmups=0)
    # Link diagnostics are separate runs: Python wrapper overhead is excluded
    # from compiler timing but included in the nested linker diagnostic.
    links = [float(x) for x in log.read_text().splitlines()]
    if len(links) != count:
        raise ValueError('missing linker sample')
    return dict(compiler_process=times, nested_link_wall=stats(links))


def profile(binary, args, expected, work, cpus, count):
    """-pg software PC histogram: single-thread only, no mcount call attribution."""
    paths = []
    durations = []
    for i in range(count):
        prefix = work / f'gmon-{i}'
        env = dict(os.environ, GMON_OUT_PREFIX=str(prefix))
        start = time.perf_counter()
        done = run(['taskset', '-c', cpus, binary, *args], env=env, cwd=work)
        durations.append(time.perf_counter() - start)
        if done.stdout != expected or done.stderr:
            raise ValueError('profile parity failed')
        found = list(work.glob(f'gmon-{i}.*'))
        if len(found) != 1:
            raise ValueError('missing software profile')
        paths.extend(found)
    report = run(['gprof', '-b', '-p', binary, *paths]).stdout.decode()
    if 'no time accumulated' in report:
        raise ValueError('empty software profile')
    for path in paths:
        path.unlink()
    return dict(flat_profile=report, wall=stats(durations),
                method='GNU gprof -pg link-only PC histogram, ~100Hz; single-thread; excludes shared-library PCs; no call-graph counts')


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--compiler', type=Path, required=True)
    ap.add_argument('--runs', type=int, default=7)
    ap.add_argument('--output', type=Path, required=True)
    ap.add_argument('--profiles', action='store_true')
    args = ap.parse_args()
    if sys.version_info[:2] != (3, 13) or sys.platform != 'linux' or args.runs < 3:
        ap.error('requires Linux, CPython 3.13 and at least three samples')
    compiler = args.compiler.resolve(strict=True)
    runtime = compiler.parent / 'libpythrust_rt.a'
    cc = shutil.which('cc')
    if not cc or not runtime.is_file():
        ap.error('cc and matching runtime beside compiler required')
    for key in ('PYTHRUST_RT_SRC', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'LD_PRELOAD'):
        if os.environ.get(key):
            ap.error(f'uncontrolled environment: {key}')
    allowed = sorted(os.sched_getaffinity(0))
    if len(allowed) < 4:
        ap.error('requires at least four permitted CPUs')
    affinities = [str(allowed[0]), ','.join(map(str, allowed[:4]))]
    env = dict(os.environ, PYTHRUST_RT=str(runtime))
    result = dict(schema=1, generated_at=datetime.now(timezone.utc).isoformat(), command=sys.argv,
                  source_commit=run(['git', 'rev-parse', 'HEAD'], cwd=ROOT).stdout.decode().strip(),
                  compiler_sha256=sha(compiler.read_bytes()), runtime_sha256=sha(runtime.read_bytes()),
                  environment=dict(profiling_limit="strace child tracing denied: PTRACE_TRACEME Operation not permitted; no security changes; concurrency uses uninstrumented rusage and controlled worker/affinity comparisons", platform=platform.platform(), libc=platform.libc_ver(), python=sys.version,
                      python_path=sys.executable, rustc=run(['rustc','-Vv']).stdout.decode(),
                      cc=run([cc,'--version']).stdout.decode().splitlines()[0],
                      cpu=Path('/proc/cpuinfo').read_text().split('model name',1)[1].splitlines()[0],
                      cpu_flags=Path('/proc/cpuinfo').read_text().split('flags',1)[1].splitlines()[0],
                      allowed_cpus=allowed, affinities=affinities, native_worker_caps=[5,8],
                      python_worker_caps=[5,8],
                      build='cargo build --locked --release -p pythrust-compiler -p pythrust-rt; default release opt-level=3',
                      warmups=1, runs=args.runs, cache='warm after generation + parity + warmups; no eviction; no cold-cache claim'),
                  inputs={}, cases={}, profiles={})
    with tempfile.TemporaryDirectory(prefix='pythrust-808-') as temp:
        tmp = Path(temp)
        datasets = {}
        shapes = dict(small_dup=(256,4096,True,True,False), small_unique=(256,4096,False,True,False),
                      large_dup=(8,4*1024*1024,True,False,False), large_unique=(8,4*1024*1024,False,False,False),
                      unique_sizes=(64,1024,False,True,True))
        for name,(count,size,dup,nested,unique) in shapes.items():
            directory = tmp / name
            manifest = generate_tree(directory,count=count,size=size,duplicates=dup,nested=nested,unique_sizes=unique)
            datasets[name] = directory
            result['inputs'][name] = dict(manifest=manifest, manifest_sha256=sha(json.dumps(manifest,sort_keys=True).encode()),
                                         bytes=sum(x['bytes'] for x in manifest))
        sources = {p.stem:p.read_text() for p in sorted((ROOT/'bench').glob('*.pyr'))}
        sources['startup'] = 'pass\n'
        for phase in ('discover','read','loaded_sha256'):
            sources['phase_'+phase] = phase_source(phase, repeats=8)
        for chunks in (8,128,8192):
            for parallel in (False,True):
                sources[f'granularity_{chunks}_'+('tasks' if parallel else 'serial')] = granularity_source(chunks,parallel)
        for name in ('fclone_lite','fclone_tasks'):
            sources[name] = (ROOT/'apps/fclone-lite'/f'{name}.py').read_text()
        # Record harness/fixture/probe identities as well as each generated source.
        result['harness_sha256'] = {p.name:sha(p.read_bytes()) for p in [Path(__file__),ROOT/'bench/profile_counts.c',ROOT/'bench/string_allocations.c']}
        for name,source in sources.items():
            work = tmp / ('program-'+name)
            work.mkdir()
            src = work / 'case.pyr'
            src.write_text(source)
            build_row = build(compiler,src,args.runs,env,work,cc)
            binary = src.with_suffix('')
            row = dict(source_sha256=sha(source.encode()), source=source, build=build_row,
                       binary_sha256=sha(binary.read_bytes()), executions={})
            if name.startswith('fclone_'):
                cases = [(key,[str(path)]) for key,path in datasets.items()]
                cases.append(('file_root',[str(datasets['small_dup'] / result['inputs']['small_dup']['manifest'][0]['path'])]))
            elif name.startswith('phase_'):
                cases = [('large_dup',[str(datasets['large_dup'])])]
            else:
                cases = [('default',[])]
            # Primary suite: four CPUs. Additional controlled one-CPU comparisons.
            multi = name.startswith(('tasks_','shared','granularity_','fclone_'))
            for dataset,argv in cases:
                oracle = run([sys.executable,src,*argv])
                if oracle.stderr:
                    raise ValueError('oracle stderr')
                if name.startswith('fclone_'):
                    expected = b'' if dataset == 'file_root' else expected_groups(datasets[dataset],result['inputs'][dataset]['manifest'])
                    if oracle.stdout != expected:
                        raise ValueError('independent fixture/group oracle disagrees')
                for cpus in affinities if multi else affinities[1:]:
                    key = dataset + '@' + cpus
                    native_cmd = ['taskset','-c',cpus,binary,*argv]
                    python_cmd = ['taskset','-c',cpus,sys.executable,src,*argv]
                    # Alternate order by case; no startup/cache subtraction.
                    pair = [('native',native_cmd),('cpython',python_cmd)]
                    if len(result['cases']) % 2:
                        pair.reverse()
                    timings = {kind:measure(cmd,args.runs,oracle.stdout) for kind,cmd in pair}
                    row['executions'][key] = dict(stdout_sha256=sha(oracle.stdout), stdout=oracle.stdout.decode().replace(str(tmp),'<DATA>'), **timings)
                    print(f'{name} {key}: native={timings["native"]["wall"]["median"]:.6f} python={timings["cpython"]["wall"]["median"]:.6f}',flush=True)
            result['cases'][name] = row
            if not args.profiles:
                continue
            # Profiles use separately linked executables, never the timed artifact.
            if name in ('numeric','sorting','strings','phase_loaded_sha256'):
                penv = write_linker(work/'gprof-linker',cc,['-pg','-no-pie'])
                penv['PYTHRUST_RT'] = str(runtime)
                run([compiler,'build',src],env=penv)
                argv = cases[0][1]
                expected = run([sys.executable,src,*argv]).stdout
                result['profiles'][name] = profile(binary,argv,expected,work,affinities[0],args.runs)
            if name in ('strings','sorting','tuples','shared_baseline','shared_4','tasks_alloc_serial','granularity_8192_tasks','fclone_tasks'):
                obj = work/'counts.o'
                run([cc,'-std=c11','-O2','-fno-builtin','-Wall','-Wextra','-Werror','-c',ROOT/'bench/profile_counts.c','-o',obj])
                wraps = ['malloc','calloc','realloc','memcpy','memmove','pt_str_inc','pt_str_dec','pt_seq_inc','pt_seq_dec','pt_seq_get','pt_seq_get_held','pt_seq_item_held','pt_seq_hold','pt_submit']
                penv = write_linker(work/'counts-linker',cc,[str(obj),'-Wl,'+','.join('--wrap='+x for x in wraps)])
                penv['PYTHRUST_RT'] = str(runtime)
                run([compiler,'build',src],env=penv)
                argv = [str(datasets['small_dup'])] if name == 'fclone_tasks' else cases[0][1]
                expected = run([sys.executable,src,*argv]).stdout
                start=time.perf_counter()
                done = run(['taskset','-c',affinities[0],binary,*argv])
                if done.stdout != expected:
                    raise ValueError('counted output mismatch')
                result['profiles'].setdefault(name,{})['counts'] = parse_counts(done.stderr)
                result['profiles'][name]['count_probe_wall'] = time.perf_counter()-start
            # ptrace is denied by this host; do not retry or relax its settings.
            args.output.write_text(json.dumps(result,indent=2)+'\n')
        args.output.write_text(json.dumps(result,indent=2)+'\n')


def parse_counts(raw):
    keys = {'heap_allocations','heap_reallocations','requested_bytes','copy_bytes','str_inc','str_dec',
            'seq_inc','seq_dec','seq_get','seq_get_held','seq_item_held','seq_hold','submit'}
    try:
        lines = [line.split('=') for line in raw.decode().splitlines()]
        counts = {k:int(v) for k,v in lines}
    except (ValueError,UnicodeError) as exc:
        raise ValueError('malformed counts') from exc
    if len(lines) != len(keys) or counts.keys() != keys or any(x < 0 for x in counts.values()):
        raise ValueError('missing, duplicate or invalid counts')
    return counts


if __name__ == '__main__':
    main()
