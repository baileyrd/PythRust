"""Recompute #808 raw statistics and reject incomplete/stale evidence."""
import argparse
import json
from pathlib import Path

import profile_workloads as p


def verify_timing(timing, runs):
    values = timing['wall']['samples']
    if len(values) != runs or p.stats(values) != timing['wall']:
        raise ValueError('incomplete samples or incorrect statistics')
    for key in ('user_seconds','system_seconds','context_switches_voluntary_involuntary'):
        if len(timing[key]) != runs:
            raise ValueError('incomplete resource samples')


def verify(result):
    runs = result['environment']['runs']
    if type(runs) is not int or runs < 3 or result['schema'] != 1:
        raise ValueError('invalid runs/schema')
    cases = result['cases']
    suite = {path.stem:path.read_text() for path in (p.ROOT/'bench').glob('*.pyr')}
    sources = dict(suite,startup='pass\n')
    for phase in ('discover','read','loaded_sha256'):
        sources['phase_'+phase] = p.phase_source(phase,8)
    for chunks in (8,128,8192):
        for parallel in (False,True):
            sources[f'granularity_{chunks}_'+('tasks' if parallel else 'serial')] = p.granularity_source(chunks,parallel)
    for name in ('fclone_lite','fclone_tasks'):
        sources[name] = (p.ROOT/'apps/fclone-lite'/f'{name}.py').read_text()
    if cases.keys() != sources.keys():
        raise ValueError('missing/unexpected workload')
    if set(result['inputs']) != {'small_dup','small_unique','large_dup','large_unique','unique_sizes'}:
        raise ValueError('missing/unexpected dataset')
    for dataset in result['inputs'].values():
        manifest = dataset['manifest']
        if (dataset['manifest_sha256'] != p.sha(json.dumps(manifest,sort_keys=True).encode())
                or dataset['bytes'] != sum(x['bytes'] for x in manifest)):
            raise ValueError('invalid dataset identity')
    affinities = result['environment']['affinities']
    for name,row in cases.items():
        if row['source'] != sources[name] or row['source_sha256'] != p.sha(sources[name].encode()):
            raise ValueError('stale/mismatched source: '+name)
        verify_timing(row['build']['compiler_process'],runs)
        links = row['build']['nested_link_wall']
        if len(links['samples']) != runs or p.stats(links['samples']) != links:
            raise ValueError('missing link samples')
        if name.startswith('fclone_'):
            datasets = [*result['inputs'],'file_root']
        elif name.startswith('phase_'):
            datasets = ['large_dup']
        else:
            datasets = ['default']
        multi = name.startswith(('tasks_','shared','granularity_','fclone_'))
        wanted = {dataset+'@'+cpus for dataset in datasets for cpus in (affinities if multi else affinities[1:])}
        if row['executions'].keys() != wanted:
            raise ValueError('missing execution: '+name)
        for pair in row['executions'].values():
            for runtime in ('native','cpython'):
                verify_timing(pair[runtime],runs)
    if result['profiles']:
        for name in ('numeric','sorting','strings','phase_loaded_sha256'):
            row=result['profiles'][name]
            if len(row['wall']['samples']) != runs or p.stats(row['wall']['samples']) != row['wall']:
                raise ValueError('incomplete profile')
            if 'no time accumulated' in row['flat_profile'] or 'Flat profile:' not in row['flat_profile']:
                raise ValueError('empty profile')
        for row in result['profiles'].values():
            if 'counts' in row:
                p.parse_counts(''.join(f'{k}={v}\n' for k,v in row['counts'].items()).encode())
    return len(cases)


if __name__ == '__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('results',type=Path)
    args=ap.parse_args()
    print(f'verified {verify(json.loads(args.results.read_text()))} workloads')
