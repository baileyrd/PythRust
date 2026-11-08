"""Independent #809 completeness, identity, order and statistics verification."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics

import profile_workloads as p


def stats(row, runs):
    xs=row['samples']
    if len(xs)!=runs or any(type(x) not in (int,float) or not math.isfinite(x) or x<=0 for x in xs):
        raise ValueError('invalid/missing samples')
    median=statistics.median(xs)
    expected=dict(samples=xs,median=median,mad=statistics.median([abs(x-median) for x in xs]),
                  minimum=min(xs),maximum=max(xs))
    if row!=expected:
        raise ValueError('incorrect statistics')


def verify(r, identities=False):
    if r['schema']!=809 or type(r['runs']) is not int or r['runs']<7 or r['warmups']!=1:
        raise ValueError('invalid schema/runs/warmups')
    if set(r['inputs'])!={'large_dup','large_unique','small_dup','small_unique'} or len(r['affinities'])!=2:
        raise ValueError('missing inputs/affinities')
    for item in r['inputs'].values():
        if hashlib.sha256(json.dumps(item['manifest'],sort_keys=True).encode()).hexdigest()!=item['manifest_sha256']:
            raise ValueError('manifest identity mismatch')
    sources=dict(startup='pass\n')
    for phase in ('discover','read','loaded_sha256'):
        sources['phase_'+phase]=p.phase_source(phase,8)
    for name in ('fclone_lite','fclone_tasks'):
        sources[name]=(p.ROOT/'apps/fclone-lite'/f'{name}.py').read_text()
    if r['cases'].keys()!=sources.keys() or set(r['micro'])!={'compression','loaded_digest'}:
        raise ValueError('missing cases/microbenchmarks')
    for name,source in sources.items():
        row=r['cases'][name]
        if row['source']!=source or row['source_sha256']!=p.sha(source.encode()):
            raise ValueError('source identity mismatch')
        datasets=list(r['inputs']) if name.startswith('fclone_') else ['large_dup'] if name.startswith('phase_') else ['default']
        expected={ds+'@'+cpu for ds in datasets for cpu in r['affinities']}
        if row['executions'].keys()!=expected or set(row['binary_sha256'])!={'baseline','candidate'}:
            raise ValueError('missing execution/binary')
        for timing in row['executions'].values():
            verify_timing(timing,r['runs'],{'baseline','candidate','cpython'})
    for row in r['micro'].values():
        verify_timing(row,r['runs'],{'baseline','candidate'})
        if set(row['internal_seconds'])!={'baseline','candidate'}:
            raise ValueError('missing micro times')
        for values in row['internal_seconds'].values():
            stats(values,r['runs'])
    if identities:
        for label,item in r['identities'].items():
            checkout=Path(item['checkout'])
            for name,digest in item['source_sha256'].items():
                if p.sha((checkout/name).read_bytes())!=digest:
                    raise ValueError(label+' measured source changed: '+name)
            for name,file in [('compiler','pythrust'),('runtime','libpythrust_rt.a')]:
                if p.sha((checkout/'target/release'/file).read_bytes())!=item[name+'_sha256']:
                    raise ValueError(label+' measured artifact changed: '+name)
        for name,digest in r['harness_sha256'].items():
            if p.sha((p.ROOT/'bench'/name).read_bytes())!=digest:
                raise ValueError('harness changed: '+name)
    return sum(len(row['executions']) for row in r['cases'].values())


def verify_timing(row,runs,kinds):
    if set(row['wall'])!=kinds or len(row['order'])!=runs or any(set(x)!=kinds or len(x)!=len(kinds) for x in row['order']):
        raise ValueError('missing order/runtime')
    for sample in row['wall'].values():
        stats(sample,runs)


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('results',type=Path)
    ap.add_argument('--identities',action='store_true')
    args=ap.parse_args()
    print('verified',verify(json.loads(args.results.read_text()),args.identities),'workload/affinity cases and two microbenchmarks')
