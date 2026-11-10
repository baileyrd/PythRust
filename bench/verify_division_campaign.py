"""Cross-link #814 timing/probe bundles; historical evidence stays immutable."""
import json
from pathlib import Path
import sys
import division_campaign as c
import three_way as b
import verify_three_way as t

BASE = 'f3f93a31aad142d2498a5ea23da075a75ac59ee9'


def verify_baseline(record):
    t.verify(record)
    p=record['prepared']
    if record['schema']!=2 or record['runs']<18 or p['source']['head']!=BASE:
        raise ValueError('wrong admitted baseline')
    expected=json.loads((b.ROOT/'docs/benchmarks/card813-domain.json').read_text())
    if json.loads(p['domain'])!=expected: raise ValueError('input/domain oracle changed')
    required={'PATH','CARGO_TARGET_DIR','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','PYTHRUST_RT',
              'PYTHRUST_RT_SRC','LD_PRELOAD','LD_LIBRARY_PATH','PYTHONHASHSEED','PYTHONPATH',
              'MALLOC_CONF','GLIBC_TUNABLES'}
    if set(p['environment'])!=required: raise ValueError('incomplete environment')
    for key in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','PYTHRUST_RT_SRC','LD_PRELOAD'):
        if p['environment'][key]: raise ValueError('uncontrolled environment')
    if p['source']['index_diff']: raise ValueError('unexpected staged baseline changes')
    # Build inputs independently pinned to the merged #813 source manifest,
    # whose compiler/runtime tree is identical to admitted current main.
    old=json.loads((b.ROOT/'docs/benchmarks/card813.json').read_text())['prepared']['source']['build_inputs']
    if p['source']['build_inputs']!=old: raise ValueError('baseline build inputs changed')
    for row in record['cases'].values():
        for item in row['warmup_samples']:
            for key in ('user_seconds','system_seconds'):
                if type(item[key]) not in (int,float) or not b.math.isfinite(item[key]) or item[key]<0:
                    raise ValueError('invalid warmup resources')
            if len(item['switches'])!=2 or any(type(x) is not int or x<0 for x in item['switches']):
                raise ValueError('invalid warmup switches')
    return len(record['cases'])


def verify_profiles(record, prepared):
    if record['status']!='complete' or record['prepared']!=prepared:
        raise ValueError('unlinked/incomplete profile')
    if record['before']!=record['after']: raise ValueError('profile input identities changed')
    for key in ('compiler','runtime','harness'):
        if record['before'][key]!=prepared[key]: raise ValueError('stale profile preparation')
    if record['probe']['sha256']!=b.digest((b.ROOT/'bench/profile_counts.c').read_bytes()):
        raise ValueError('probe source mismatch')
    if set(record['cases'])!={'numeric','sorting','strings','tuples'}: raise ValueError('missing profiles')
    for name,row in record['cases'].items():
        if row['source']['sha256']!=prepared['sources'][name]['pythrust']['sha256']:
            raise ValueError('profile workload differs')
        if row['count_returncode']!=0 or row['count_stdout']!=b.EXPECTED[name]: raise ValueError('count output/status')
        if c.p.parse_counts(row['raw_counts'].encode())!=row['counts']: raise ValueError('bad counts')
        profile=row['profile']
        if len(profile['wall']['samples'])!=7 or 'no time accumulated' in profile['flat_profile']:
            raise ValueError('missing profile samples')
        if c.p.stats(profile['wall']['samples'])!=profile['wall']: raise ValueError('profile statistics')
        if 'GNU gprof -pg link-only' not in profile['method']: raise ValueError('wrong profile method')
        if row['profile_binary']==row['count_binary']: raise ValueError('instrumented variants not separated')
        for key in ('profile_binary','count_binary','assembly','count_object'):
            t.hash_record(row[key])
    return len(record['cases'])


def verify_bundle(baseline, pair, baseline_profiles, candidate_profiles):
    verify_baseline(baseline)
    c.verify_pair(pair)
    if pair['prepared']['baseline']['compiler']['sha256']==pair['prepared']['candidate']['compiler']['sha256']:
        raise ValueError('same compiler bytes')
    if pair['prepared']['baseline']!=baseline['prepared']: raise ValueError('unmatched baseline timing')
    verify_profiles(baseline_profiles,baseline['prepared'])
    verify_profiles(candidate_profiles,pair['prepared']['candidate'])
    return 4


if __name__=='__main__':
    args=[json.loads(Path(p).read_text()) for p in sys.argv[1:]]
    print('Verified campaign workloads:',verify_bundle(*args))


def verify_controls(record):
    import division_controls as extra
    if record['status']!='complete' or record['runs']<24: raise ValueError('incomplete controls')
    p=record['prepared']
    if p['status']!='prepared' or record['identities_after']!=p['identities']: raise ValueError('stale controls')
    sources=extra.lists.sources()
    sources={k:sources[k] for k in ('int_access','float_access')}
    sources['generic_access']=sources['int_access'].replace('xs: list[int]','xs').replace(' -> int','')
    sources['source_inline_diagnostic']=(b.ROOT/'bench/tuples.pyr').read_text().replace(
        '        n, d = div_mod(n, 10)','        quotient = n // 10\n        d = n % 10\n        n = quotient')
    if set(record['cases'])!=set(sources) or set(p['cases'])!=set(sources) or set(p['identities'])!=set(sources):
        raise ValueError('wrong controls')
    for ordinal,(name,row) in enumerate(record['cases'].items()):
        case=p['cases'][name]
        if case['source']!=sources[name]: raise ValueError('control source changed')
        if case['expected']!=('28000000.0\n' if name=='float_access' else b.EXPECTED['tuples'] if name=='source_inline_diagnostic' else '28000000\n'):
            raise ValueError('control oracle changed')
        if row['order']!=b.order(['baseline','candidate'],record['runs'],record['seed']+ordinal): raise ValueError('control order')
        if [(x['round'],x['kind']) for x in row['samples']]!=[(i,k) for i,seq in enumerate(row['order']) for k in seq]:
            raise ValueError('missing control sample')
        if [(x['kind'],x['warmup']) for x in row['warmup_samples']]!=[(k,i) for k in ('baseline','candidate') for i in range(2)]:
            raise ValueError('missing control warmup')
        if set(case['conditions'])!={'baseline','candidate'} or set(p['identities'][name])!={'baseline','candidate'}:
            raise ValueError('wrong control conditions')
        for label,item in case['conditions'].items():
            t.hash_record(item['binary'])
            t.hash_record(item['source'])
            if item['command']!=[item['binary']['path']]:
                raise ValueError('control invocation does not match executable/arguments')
            if item['binary']['path']!=str(Path(item['source']['path']).with_suffix('')):
                raise ValueError('unmatched control source/output')
            if item['binary']!=p['identities'][name][label]['binary'] or item['source']!=p['identities'][name][label]['source']:
                raise ValueError('unlinked control identity')
            expected=(b.ROOT/'bench/tuples.pyr').read_text() if name=='source_inline_diagnostic' and label=='baseline' else sources[name]
            if b.digest(expected.encode())!=item['source']['sha256']: raise ValueError('control input hash')
        for item in row['samples']+row['warmup_samples']:
            if item['returncode']!=0 or item['stderr'] or item['stdout']!=case['expected']: raise ValueError('bad control output/status')
            b.stats([item['wall_seconds']])
            for key in ('user_seconds','system_seconds'):
                if type(item[key]) not in (int,float) or not b.math.isfinite(item[key]) or item[key]<0: raise ValueError('control resources')
            if len(item['switches'])!=2 or any(type(x) is not int or x<0 for x in item['switches']): raise ValueError('control switches')
        for label in ('baseline','candidate'):
            if row['summary'][label]!=b.stats([x['wall_seconds'] for x in row['samples'] if x['kind']==label]): raise ValueError('control summary')
    for kind in ('mixed','dyn'):
        if p['refused'][kind]['returncode']!=1 or not p['refused'][kind]['stderr'] or p['refused'][kind]['stdout']:
            raise ValueError('unsupported list control accepted')
    return 4


def verify_path_control(record):
    if record['status']!='complete' or record['runs']!=42 or record['warmups']!=2:
        raise ValueError('incomplete path control')
    if set(record['cases'])!={'strings','tuples'}: raise ValueError('wrong path controls')
    for ordinal,(name,row) in enumerate(record['cases'].items()):
        if not row['byte_identical'] or row['binaries']['baseline']['sha256']!=row['binaries']['candidate']['sha256']:
            raise ValueError('negative control bytes differ')
        if row['post_binaries']!=row['binaries']: raise ValueError('changed path control')
        if row['source']['sha256']!=b.digest((b.ROOT/'bench'/(name+'.pyr')).read_bytes()): raise ValueError('changed path input')
        for label,command in row['build_commands'].items():
            if command!=[record['prepared'][label]['compiler']['path'],'build',row['source']['path']]:
                raise ValueError('different source path')
        if row['order']!=b.order(['baseline','candidate'],42,record['seed']+ordinal): raise ValueError('path order')
        if [(x['round'],x['kind']) for x in row['samples']]!=[(i,k) for i,seq in enumerate(row['order']) for k in seq]:
            raise ValueError('missing path trials')
        if [(x['kind'],x['warmup']) for x in row['warmup_samples']]!=[(k,i) for k in ('baseline','candidate') for i in range(2)]:
            raise ValueError('missing path warmups')
        for item in row['samples']+row['warmup_samples']:
            if item['returncode']!=0 or item['stderr'] or item['stdout']!=b.EXPECTED[name]: raise ValueError('path status/output')
            b.stats([item['wall_seconds']])
            for key in ('user_seconds','system_seconds'):
                if type(item[key]) not in (int,float) or not b.math.isfinite(item[key]) or item[key]<0: raise ValueError('path resources')
            if len(item['switches'])!=2 or any(type(x) is not int or x<0 for x in item['switches']): raise ValueError('path switches')
        for label in ('baseline','candidate'):
            if row['summary'][label]!=b.stats([x['wall_seconds'] for x in row['samples'] if x['kind']==label]): raise ValueError('path summary')
    return 2


def verify_probe_provenance(record, profiles):
    for label,data in zip(('base','candidate'),profiles):
        for name,row in data['cases'].items():
            p=record[label][name]
            for key,original in [('profile',row['profile_binary']),('counts',row['count_binary'])]:
                if p[key]['sha256']!=original['sha256']: raise ValueError('probe bytes not recovered')
            if p['count_object']!=row['count_object'] or p['source']!=row['source']: raise ValueError('unmatched probe inputs')
            for key in ('pg_linker','counts_linker'):
                if b.digest(p[key+'_source'].encode())!=p[key]['sha256']: raise ValueError('linker provenance')
            expected=[data['prepared']['compiler']['path'],'build',row['source']['path']]
            if p['rebuilt_profile_command']!=expected: raise ValueError('different probe compiler/source')
    for key in ('cc','ld','gprof','objdump'):
        t.hash_record(record['tools'][key]['identity'])
        if not record['tools'][key]['version']: raise ValueError('missing probe tool version')
    return 8
