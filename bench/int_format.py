"""#811 same-host decimal formatting experiment; instrumented counts never timed."""
import argparse
from datetime import datetime, timezone
import json
import math
import os
from pathlib import Path
import platform
import random
import resource
import shutil
import sys
import time

import profile_workloads as p

ROOT = Path(__file__).resolve().parent.parent
NAMES = ('tasks_alloc_serial', 'tasks_alloc', 'strings', 'int_format')
LABELS = ('before', 'after')
FIELDS = {'user', 'system', 'voluntary', 'involuntary'}


def sources():
    result = {n: (ROOT / 'bench' / (n + '.pyr')).read_text() for n in NAMES[:3]}
    result['int_format'] = (ROOT / 'bench/int_format.pyr').read_text()
    return result


def identity(path):
    return dict(path=str(path), sha256=p.sha(path.read_bytes()))


def prepare(work, label, target):
    folder = work / label
    folder.mkdir(parents=True)
    for name in ('pythrust', 'libpythrust_rt.a'):
        shutil.copy2(target / 'release' / name, folder / name)
    record = dict(commit=p.run(['git', 'rev-parse', 'HEAD'], cwd=ROOT).stdout.decode().strip(),
                  tree=p.run(['git', 'rev-parse', 'HEAD^{tree}'], cwd=ROOT).stdout.decode().strip(),
                  source_diff=p.run(['git', 'diff', '--', 'crates'], cwd=ROOT).stdout.decode(),
                  compiler=identity(folder / 'pythrust'), archive=identity(folder / 'libpythrust_rt.a'), cases={})
    env = dict(os.environ, PYTHRUST_RT=str(folder / 'libpythrust_rt.a'))
    for name, source in sources().items():
        src = folder / (name + '.pyr')
        src.write_text(source)
        p.run([folder / 'pythrust', 'build', src], env=env)
        oracle = p.run([sys.executable, src])
        interpreted = p.run([folder / 'pythrust', 'run', src])
        native = p.run([src.with_suffix('')])
        if oracle.stdout != native.stdout or interpreted.stdout != native.stdout or any(x.stderr for x in (oracle, interpreted, native)):
            raise ValueError('preparation CPython/interpreter/native mismatch')
        record['cases'][name] = dict(source=source, source_sha256=p.sha(source.encode()),
                                     stdout=oracle.stdout.decode(), binary=identity(src.with_suffix('')))
    (folder / 'identity.json').write_text(json.dumps(record, indent=2) + '\n')


def digest(value, size):
    if not isinstance(value, str) or len(value) != size or any(c not in '0123456789abcdef' for c in value):
        raise ValueError('invalid digest')


def verify_identities(records, files=False):
    if set(records) != set(LABELS):
        raise ValueError('exact before/after identities required')
    for record in records.values():
        digest(record['commit'], 40)
        digest(record['tree'], 40)
        if record['source_diff'] != '' or set(record['cases']) != set(NAMES):
            raise ValueError('dirty or incomplete identity')
        for name, case in record['cases'].items():
            source = sources()[name]
            if case['source'] != source or case['source_sha256'] != p.sha(source.encode()):
                raise ValueError('identity source mismatch')
        items = [record['compiler'], record['archive'], *[x['binary'] for x in record['cases'].values()]]
        for item in items:
            digest(item['sha256'], 64)
            if not item['path'] or (files and identity(Path(item['path'])) != item):
                raise ValueError('binary identity mismatch')
    for name in NAMES:
        if records['before']['cases'][name]['stdout'] != records['after']['cases'][name]['stdout']:
            raise ValueError('condition parity mismatch')


def verify(result):
    if result['schema'] != 1 or type(result['runs']) is not int or result['runs'] < 7:
        raise ValueError('invalid schema/sample count')
    verify_identities(result['identities'])
    expected = {f'{n}/{cpus}' for n in NAMES for cpus in (1, 4)}
    if set(result['cases']) != expected:
        raise ValueError('incomplete workload/affinity matrix')
    for key, row in result['cases'].items():
        name, cpus = key.split('/')
        if len(row['affinity']) != int(cpus) or len(set(row['affinity'])) != int(cpus):
            raise ValueError('invalid affinity')
        if row['stdout'] != result['identities']['before']['cases'][name]['stdout']:
            raise ValueError('output differs')
        runs = result['runs']
        if len(row['order']) != runs or any(sorted(order) != sorted(LABELS) for order in row['order']):
            raise ValueError('invalid trial order')
        if set(row['resources']) != set(LABELS):
            raise ValueError('missing resource conditions')
        for label in LABELS:
            if len(row[label]['samples']) != runs or p.stats(row[label]['samples']) != row[label]:
                raise ValueError('inconsistent statistics')
            if len(row['resources'][label]) != runs:
                raise ValueError('missing resource samples')
            for sample in row['resources'][label]:
                if set(sample) != FIELDS:
                    raise ValueError('missing resource fields')
                for k in ('user', 'system'):
                    if type(sample[k]) not in (float, int) or not math.isfinite(sample[k]) or sample[k] < 0:
                        raise ValueError('invalid CPU seconds')
                for k in ('voluntary', 'involuntary'):
                    if type(sample[k]) is not int or sample[k] < 0:
                        raise ValueError('invalid context-switch count')
    return len(result['cases'])


def quiet():
    raw = p.run(['ps', '-eo', 'comm=']).stdout.decode()
    competing = [x for x in raw.splitlines() if x.strip() in {'cargo', 'rustc', 'cc1', 'ld', 'pythrust', *NAMES}]
    if competing:
        raise RuntimeError('visible competing activity: ' + repr(competing))
    return dict(loadavg=os.getloadavg(), visible_processes=raw,
                limitation='Process namespaces can hide host activity; no hardware exclusivity claim.')


def measure(work, output, runs):
    if sys.version_info[:2] != (3, 13) or runs < 7:
        raise ValueError('CPython 3.13 and >=7 samples required')
    original = os.sched_getaffinity(0)
    if len(original) < 4:
        raise ValueError('four allowed CPUs required')
    records = {label: json.loads((work / label / 'identity.json').read_text()) for label in LABELS}
    verify_identities(records, files=True)
    result = dict(schema=1, runs=runs, warmups=2, seed=811, identities=records,
                  environment=dict(utc=datetime.now(timezone.utc).isoformat(), python=sys.version,
                                   platform=platform.platform(), rustc=p.run(['rustc', '-Vv']).stdout.decode(),
                                   allowed_cpus=sorted(original), cpuinfo=Path('/proc/cpuinfo').read_text().split('\n\n')[0],
                                   before=quiet()), cases={})
    rng = random.Random(811)
    try:
        matrix = [(name, cpus) for name in NAMES for cpus in (1, 4)]
        rng.shuffle(matrix)
        for name, cpus in matrix:
            affinity = sorted(original)[:cpus]
            os.sched_setaffinity(0, affinity)
            expected = records['before']['cases'][name]['stdout'].encode()
            commands = {label: [work / label / name] for label in LABELS}
            for command in commands.values():
                for _ in range(2):
                    done = p.run(command)
                    if done.stdout != expected or done.stderr:
                        raise ValueError('warmup parity')
            values = {label: [] for label in LABELS}
            resources = {label: [] for label in LABELS}
            orders = []
            for _ in range(runs):
                order = list(LABELS)
                rng.shuffle(order)
                orders.append(order)
                for label in order:
                    before = resource.getrusage(resource.RUSAGE_CHILDREN)
                    start = time.perf_counter()
                    done = p.run(commands[label])
                    wall = time.perf_counter() - start
                    after = resource.getrusage(resource.RUSAGE_CHILDREN)
                    if done.stdout != expected or done.stderr:
                        raise ValueError('timed parity')
                    values[label].append(wall)
                    resources[label].append(dict(user=after.ru_utime-before.ru_utime, system=after.ru_stime-before.ru_stime,
                                                 voluntary=after.ru_nvcsw-before.ru_nvcsw, involuntary=after.ru_nivcsw-before.ru_nivcsw))
            result['cases'][f'{name}/{cpus}'] = dict(affinity=affinity, stdout=expected.decode(), order=orders,
                                                   resources=resources, **{label: p.stats(v) for label, v in values.items()})
            print(name, cpus, {label: p.stats(v)['median'] for label, v in values.items()}, flush=True)
        result['environment']['after'] = quiet()
        verify(result)
        output.write_text(json.dumps(result, indent=2) + '\n')
    finally:
        os.sched_setaffinity(0, original)


def counts(work, output):
    """GNU linked-edge counts and deterministic text/object allocation failure."""
    obj = work / 'int_format_counts.o'
    cc = shutil.which('cc')
    p.run([cc, '-O2', '-fno-builtin', '-c', ROOT / 'bench/int_format_counts.c', '-o', obj])
    wraps = ('pt_str_from_int', 'malloc', 'calloc', 'free', 'realloc', 'memcpy', 'memmove')
    result = {}
    for label in LABELS:
        folder = work / label
        env = p.write_linker(work / (label + '-counts-linker'), cc, [str(obj), *['-Wl,--wrap=' + n for n in wraps]])
        env['PYTHRUST_RT'] = str(folder / 'libpythrust_rt.a')
        result[label] = {}
        for name, source in sources().items():
            src = folder / (name + '_count.pyr')
            src.write_text(source)
            p.run([folder / 'pythrust', 'build', src], env=env)
            done = p.run([src.with_suffix('')])
            values = dict((k, int(v)) for k, v in (x.split('=') for x in done.stderr.decode().splitlines()))
            if done.stdout.decode() != json.loads((folder / 'identity.json').read_text())['cases'][name]['stdout']:
                raise ValueError('count parity')
            if values['decimal_reallocations'] != 0 or values['decimal_allocations'] != 2 * values['conversions']:
                raise ValueError('exact allocation invariant')
            result[label][name] = dict(counts=values, stdout=done.stdout.decode())
        src = folder / 'failure.pyr'
        src.write_text('try:\n    print(str(-9223372036854775808))\nexcept Exception:\n    print("caught")\n')
        p.run([folder / 'pythrust', 'build', src], env=env)
        import subprocess
        failures = {}
        for index in (1, 2):
            done = subprocess.run([src.with_suffix('')], env=dict(os.environ, PYTHRUST_811_FAIL_ALLOC=str(index)), capture_output=True)
            if done.returncode == 0 or b'memory allocation' not in done.stderr or done.stdout:
                raise ValueError('allocation failure policy')
            failures[str(index)] = dict(returncode=done.returncode, stderr=done.stderr.decode(), stdout=done.stdout.decode())
        result[label]['failures'] = failures
    output.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--work', type=Path)
    ap.add_argument('--prepare', choices=LABELS)
    ap.add_argument('--target', type=Path)
    ap.add_argument('--output', type=Path)
    ap.add_argument('--runs', type=int, default=21)
    ap.add_argument('--verify', type=Path)
    ap.add_argument('--counts', action='store_true')
    args = ap.parse_args()
    if args.verify:
        print('verified', verify(json.loads(args.verify.read_text())), 'workload/affinity rows')
    elif args.prepare:
        prepare(args.work.resolve(), args.prepare, args.target.resolve())
    elif args.counts:
        counts(args.work.resolve(), args.output)
    else:
        measure(args.work.resolve(), args.output, args.runs)
