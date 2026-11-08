"""#810 same-host release dispatch pilot. Build first; timings never instrumented.

Prepare baseline before editing, then candidate, into one task-owned --work dir.
Measure only after all builds/tests stop. Raw source, hashes, trial order and
separate linked access counts are retained. No startup subtraction.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import random
import resource
import shutil
import statistics
import sys
import time

import profile_workloads as p

ROOT = Path(os.environ.get('PYTHRUST_BENCH_ROOT', Path(__file__).resolve().parent.parent)).resolve()
NAMES = ('sorting', 'numeric', 'strings', 'tuples', 'int_access', 'float_access')


def sources():
    result = {name: (ROOT / 'bench' / (name + '.pyr')).read_text()
              for name in NAMES[:4]}
    for ty in ('int', 'float'):
        value = '7' if ty == 'int' else '7.0'
        total = '0' if ty == 'int' else '0.0'
        result[ty + '_access'] = f'''# Isolated matched load/store control, #810.
def access(xs: list[{ty}]) -> {ty}:
    n = {total}
    for i in range(4000000):
        k = i % 16
        n += xs[k]
        xs[k] = {value}
    return n

print(access([{value}] * 16))
'''
    return result


def identity(path):
    return dict(path=str(path), sha256=p.sha(path.read_bytes()))


def prepare(work, label):
    folder = work / label
    folder.mkdir(parents=True)  # Refuse overwriting a previous condition.
    compiler = ROOT / 'target/release/pythrust'
    archive = ROOT / 'target/release/libpythrust_rt.a'
    shutil.copy2(compiler, folder / 'pythrust')
    shutil.copy2(archive, folder / archive.name)
    compiler = folder / 'pythrust'
    env = dict(os.environ, PYTHRUST_RT=str(folder / archive.name))
    metadata = dict(base=p.run(['git', 'rev-parse', 'HEAD'], cwd=ROOT).stdout.decode().strip(),
                    tree=p.run(['git', 'rev-parse', 'HEAD^{tree}'], cwd=ROOT).stdout.decode().strip(),
                    source_diff=p.run(['git', 'diff', '--', 'crates'], cwd=ROOT).stdout.decode(),
                    compiler=identity(compiler), archive=identity(folder / archive.name), cases={})
    for name, source in sources().items():
        src = folder / (name + '.pyr')
        src.write_text(source)
        p.run([compiler, 'build', src], env=env)
        metadata['cases'][name] = dict(source=source, source_sha256=p.sha(source.encode()),
                                      binary=identity(src.with_suffix('')))
    (folder / 'identity.json').write_text(json.dumps(metadata, indent=2) + '\n')


def quiet():
    # Inspect visible host processes, omit our shell and this harness. Never kill.
    raw = p.run(['ps', '-eo', 'comm=']).stdout.decode()
    competing = [line for line in raw.splitlines()
                 if line.split()[0] in {'cargo', 'rustc', 'cc1', 'ld', 'pythrust',
                                        'sorting', 'int_access', 'float_access'}]
    if competing:
        raise RuntimeError('competing build/test/benchmark processes: ' + repr(competing))
    return dict(loadavg=os.getloadavg(), processes=raw)


def verify(result):
    if result['schema'] != 1 or result['runs'] < 7:
        raise ValueError('invalid schema or sample count')
    expected = sources()
    if set(result['cases']) != set(expected):
        raise ValueError('workload set differs')
    for name, row in result['cases'].items():
        if row['source'] != expected[name] or row['source_sha256'] != p.sha(expected[name].encode()):
            raise ValueError('changed source')
        for label in ('before', 'after', 'cpython'):
            if len(row[label]['samples']) != result['runs'] or p.stats(row[label]['samples']) != row[label]:
                raise ValueError('incomplete or inconsistent statistics')
        if len(row['order']) != result['runs'] or any(sorted(x) != ['after', 'before', 'cpython'] for x in row['order']):
            raise ValueError('invalid trial order')
    return len(result['cases'])


def measure(work, output, runs):
    if sys.version_info[:2] != (3, 13) or runs < 7:
        raise ValueError('CPython 3.13 and >=7 runs required')
    original = os.sched_getaffinity(0)
    cpu = min(original)
    os.sched_setaffinity(0, {cpu})
    try:
        result = dict(schema=1, runs=runs, warmups=2, seed=810, utc=datetime.now(timezone.utc).isoformat(),
                      environment=dict(host=platform.platform(), toolchain=p.run(['rustc', '-Vv']).stdout.decode(),
                                       python=sys.version, cpu=cpu, affinity_original=sorted(original),
                                       cpuinfo=Path('/proc/cpuinfo').read_text().split('\n\n')[0], before=quiet()), cases={}, identities={})
        for label in ('before', 'after'):
            result['identities'][label] = json.loads((work / label / 'identity.json').read_text())
        rng = random.Random(810)
        for name, source in sources().items():
            commands = {label: [work / label / name] for label in ('before', 'after')}
            commands['cpython'] = [sys.executable, work / 'before' / (name + '.pyr')]
            expected = p.run(commands['cpython']).stdout
            for cmd in commands.values():
                for _ in range(2):
                    done = p.run(cmd)
                    if done.stdout != expected or done.stderr:
                        raise ValueError('warmup parity failure')
            values = {label: [] for label in commands}
            order = []
            for _ in range(runs):
                trial = list(commands)
                rng.shuffle(trial)
                order.append(trial)
                for label in trial:
                    start = time.perf_counter()
                    done = p.run(commands[label])
                    elapsed = time.perf_counter() - start
                    if done.stdout != expected or done.stderr:
                        raise ValueError('timed output/exit/stderr mismatch')
                    values[label].append(elapsed)
            result['cases'][name] = dict(source=source, source_sha256=p.sha(source.encode()),
                                        stdout=expected.decode(), order=order,
                                        **{label: p.stats(xs) for label, xs in values.items()})
            print(name, {label: round(statistics.median(xs), 6) for label, xs in values.items()}, flush=True)
        result['environment']['after'] = quiet()
        verify(result)
        output.write_text(json.dumps(result, indent=2) + '\n')
    finally:
        os.sched_setaffinity(0, original)


def repeat_native(work, output, runs):
    """Additional randomized native-only trials resolve near-noise controls.

    The primary record already retains CPython samples and parity. This phase
    measures identical preserved binaries, never a reconstructed historical host.
    """
    if runs < 7:
        raise ValueError('>=7 runs required')
    primary = json.loads((work / 'timings.json').read_text())
    verify(primary)
    verify_identities(primary)
    original = os.sched_getaffinity(0)
    os.sched_setaffinity(0, {primary['environment']['cpu']})
    result = dict(runs=runs, seed=1810, warmups=2, before=quiet(), cases={})
    rng = random.Random(1810)
    try:
        for name in NAMES:
            commands = {label: [work / label / name] for label in ('before', 'after')}
            expected = primary['cases'][name]['stdout'].encode()
            for command in commands.values():
                for _ in range(2):
                    done = p.run(command)
                    if done.stdout != expected or done.stderr:
                        raise ValueError('repeat warmup parity')
            times = {label: [] for label in commands}
            resources = {label: [] for label in commands}
            orders = []
            for _ in range(runs):
                order = list(commands)
                rng.shuffle(order)
                orders.append(order)
                for label in order:
                    before = resource.getrusage(resource.RUSAGE_CHILDREN)
                    start = time.perf_counter()
                    done = p.run(commands[label])
                    elapsed = time.perf_counter() - start
                    after = resource.getrusage(resource.RUSAGE_CHILDREN)
                    if done.stdout != expected or done.stderr:
                        raise ValueError('repeat output mismatch')
                    times[label].append(elapsed)
                    resources[label].append(dict(user=after.ru_utime-before.ru_utime,
                                                 system=after.ru_stime-before.ru_stime,
                                                 voluntary=after.ru_nvcsw-before.ru_nvcsw,
                                                 involuntary=after.ru_nivcsw-before.ru_nivcsw))
            result['cases'][name] = dict(order=orders, resources=resources,
                                        **{label: p.stats(xs) for label, xs in times.items()})
            print(name, {label: statistics.median(xs) for label, xs in times.items()}, flush=True)
        result['after'] = quiet()
        verify_repeat(result)
        output.write_text(json.dumps(result, indent=2) + '\n')
    finally:
        os.sched_setaffinity(0, original)


def verify_repeat(result):
    runs = result['runs']
    if runs < 7 or set(result['cases']) != set(NAMES):
        raise ValueError('incomplete repeat')
    for row in result['cases'].values():
        if len(row['order']) != runs or any(sorted(x) != ['after', 'before'] for x in row['order']):
            raise ValueError('invalid repeat order')
        for label in ('before', 'after'):
            if len(row[label]['samples']) != runs or p.stats(row[label]['samples']) != row[label]:
                raise ValueError('invalid repeat statistics')
            if len(row['resources'][label]) != runs:
                raise ValueError('incomplete resource samples')


def verify_bundle(folder):
    primary = json.loads((folder / 'card810.json').read_text())
    verify(primary)
    repeat = json.loads((folder / 'card810-repeat.json').read_text())
    verify_repeat(repeat)
    counts = json.loads((folder / 'card810-counts.json').read_text())
    if set(counts) != {'before', 'after'} or counts['before'] != counts['after']:
        raise ValueError('different/missing count conditions')
    for label, cases in counts.items():
        if set(cases) != {'sorting', 'int_access', 'float_access'}:
            raise ValueError('incomplete count workloads')
        for name, row in cases.items():
            parsed = parse_counts(''.join(f'{k}={v}\n' for k, v in row['counts'].items()).encode())
            if row['stdout'] != primary['cases'][name]['stdout']:
                raise ValueError('count parity differs')
            if name == 'sorting' and parsed != dict(reads=10852716, writes=2714780, holds=0, held_items=0):
                raise ValueError('sorting access mechanism differs')
            if name.endswith('_access') and parsed != dict(reads=4000000, writes=4000000, holds=0, held_items=0):
                raise ValueError('isolated access mechanism differs')
    identities = json.loads((folder / 'card810-identities.json').read_text())
    for record in primary['identities'].values():
        if record['base'] != identities['base'] or record['tree'] != identities['base_tree']:
            raise ValueError('different base identity')
        if set(record['cases']) != set(NAMES):
            raise ValueError('missing binary identity')
        for name, case in record['cases'].items():
            if case['source'] != primary['cases'][name]['source']:
                raise ValueError('source differs between conditions')
        for item in [record['compiler'], record['archive'], *[x['binary'] for x in record['cases'].values()]]:
            if len(item['sha256']) != 64 or any(c not in '0123456789abcdef' for c in item['sha256']):
                raise ValueError('invalid binary digest')
    return len(primary['cases'])


def parse_counts(raw):
    result = {}
    for line in raw.decode().splitlines():
        key, value = line.split('=')
        if key in result or not value.isascii() or not value.isdecimal():
            raise ValueError('invalid/duplicate access counter')
        result[key] = int(value)
    if set(result) != {'reads', 'writes', 'holds', 'held_items'}:
        raise ValueError('incomplete access counters')
    return result


def verify_identities(result):
    for label, record in result['identities'].items():
        if label not in ('before', 'after'):
            raise ValueError('unknown condition')
        for item in [record['compiler'], record['archive'],
                     *[case['binary'] for case in record['cases'].values()]]:
            if identity(Path(item['path'])) != item:
                raise ValueError('binary identity differs')
        for name, case in record['cases'].items():
            if case['source'] != result['cases'][name]['source']:
                raise ValueError('condition source differs')


def counts(work):
    """GNU linker probes run separately from all ordinary timings."""
    cc = shutil.which('cc')
    result = {}
    obj = work / 'int_list_counts.o'
    p.run([cc, '-O2', '-c', ROOT / 'bench/int_list_counts.c', '-o', obj])
    names = ('pt_seq_get', 'pt_list_set', 'pt_seq_hold', 'pt_seq_item_held')
    extras = [str(obj)] + ['-Wl,--wrap=' + name for name in names]
    for label in ('before', 'after'):
        folder = work / label
        env = p.write_linker(work / (label + '-count-linker'), cc, extras)
        env['PYTHRUST_RT'] = str(folder / 'libpythrust_rt.a')
        result[label] = {}
        for name in ('sorting', 'int_access', 'float_access'):
            src = folder / (name + '_count.pyr')
            src.write_text(sources()[name])
            p.run([folder / 'pythrust', 'build', src], env=env)
            done = p.run([src.with_suffix('')])
            expected = p.run([folder / name]).stdout
            if done.stdout != expected:
                raise ValueError('instrumented output differs')
            result[label][name] = dict(stdout=done.stdout.decode(), counts=parse_counts(done.stderr))
    if result['before'] != result['after']:
        raise ValueError('access counts changed between conditions')
    (work / 'counts.json').write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--work', type=Path)
    ap.add_argument('--verify-bundle', type=Path)
    ap.add_argument('--verify', type=Path)
    ap.add_argument('--identities', action='store_true')
    ap.add_argument('--prepare', choices=('before', 'after'))
    ap.add_argument('--repeat-native', action='store_true')
    ap.add_argument('--counts', action='store_true')
    ap.add_argument('--output', type=Path)
    ap.add_argument('--runs', type=int, default=15)
    args = ap.parse_args()
    if args.verify_bundle:
        print('verified complete bundle:', verify_bundle(args.verify_bundle), 'workloads')
        sys.exit(0)
    if not args.work:
        ap.error('--work required')
    work = args.work.resolve()
    if args.verify:
        record = json.loads(args.verify.read_text())
        print('verified', verify(record), 'workloads')
        if args.identities:
            verify_identities(record)
    elif args.prepare:
        prepare(work, args.prepare)
    elif args.repeat_native:
        repeat_native(work, args.output, args.runs)
    elif args.counts:
        counts(work)
    else:
        if not args.output:
            ap.error('--output required for measurement')
        measure(work, args.output, args.runs)
