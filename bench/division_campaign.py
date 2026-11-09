"""#814 paired timing and separate profiles of the frozen #813 workloads.

Prepare both conditions first. No builds or instrumented runs during timings.
The baseline must be current main at admission; all differences are recorded.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import sys

import three_way as b
import profile_workloads as p
import verify_three_way as verifier

NAMES = tuple(b.EXPECTED)


def identities(prepared):
    result = {k: b.identity(Path(prepared[k]['path'])) for k in ('compiler', 'runtime', 'harness')}
    for name, row in prepared['conditions'].items():
        for kind, item in row.items():
            result[name+'/'+kind] = b.identity(Path(item['executable']['path']))
    for name, row in prepared['sources'].items():
        for kind in ('pythrust', 'rust'):
            result[name+'/source/'+kind] = b.identity(Path(row[kind]['path']))
    return result


def prepare(args):
    base = json.loads((args.baseline/'prepared.json').read_text())
    if base['status'] != 'prepared': raise ValueError('baseline not prepared')
    args.work.mkdir()
    compiler = args.compiler.resolve(strict=True)
    runtime = compiler.parent/'libpythrust_rt.a'
    env = dict(os.environ, PYTHRUST_RT=str(runtime))
    candidate = dict(schema=2, status='preparing', source=b.source_record(), host=b.host(),
                     cpu=base['cpu'], compiler=b.identity(compiler), runtime=b.identity(runtime),
                     harness=b.identity(Path(b.__file__)), sources={}, conditions={})
    b.pin(base['cpu'])
    for name, expected in b.EXPECTED.items():
        source = args.work/(name+'.pyr')
        source.write_text(base['sources'][name]['pythrust_source'])
        b.run([compiler, 'build', source], env=env)
        b.sample([source.with_suffix('')], expected, env=env)
        candidate['sources'][name] = dict(base['sources'][name], pythrust=b.identity(source))
        candidate['conditions'][name] = dict(base['conditions'][name],
            pythrust=dict(command=[str(source.with_suffix(''))], executable=b.identity(source.with_suffix(''))))
    candidate['status'] = 'prepared'
    b.save(args.work/'prepared.json', candidate)


def pair(args):
    if args.output.exists(): raise FileExistsError(args.output)
    prepared = {k: json.loads((folder/'prepared.json').read_text())
                for k, folder in [('baseline', args.baseline), ('candidate', args.work)]}
    before = {k: identities(v) for k, v in prepared.items()}
    for label, cond in prepared.items():
        for key in ('compiler', 'runtime', 'harness'):
            if before[label][key] != cond[key]: raise ValueError('stale '+key)
        for name, row in cond['conditions'].items():
            for kind, item in row.items():
                if before[label][name+'/'+kind] != item['executable']: raise ValueError('stale binary')
    if prepared['baseline']['cpu'] != prepared['candidate']['cpu']: raise ValueError('different CPUs')
    b.pin(prepared['baseline']['cpu'])
    result = dict(schema=1, status='running', timestamp=datetime.now(timezone.utc).isoformat(),
                  command=sys.argv, harness=b.identity(Path(__file__)), prepared=prepared,
                  identities_before=before, host=b.host(), runs=args.runs, seed=args.seed,
                  warmups=2, cases={})
    try:
        for ordinal, (name, expected) in enumerate(b.EXPECTED.items()):
            row = dict(order=b.order(['baseline','candidate'], args.runs, args.seed+ordinal),
                       samples=[], warmup_samples=[], summary={})
            result['cases'][name] = row
            for label in prepared:
                cmd = prepared[label]['conditions'][name]['pythrust']['command']
                for i in range(2):
                    row['warmup_samples'].append(dict(kind=label,warmup=i,**b.sample(cmd,expected)))
            for i, sequence in enumerate(row['order']):
                for label in sequence:
                    cmd = prepared[label]['conditions'][name]['pythrust']['command']
                    row['samples'].append(dict(kind=label,round=i,**b.sample(cmd,expected)))
                b.save(args.output,result)
            row['summary'] = {label:b.stats([x['wall_seconds'] for x in row['samples'] if x['kind']==label])
                              for label in prepared}
            print(name,row['summary'],flush=True)
        result['identities_after']={k:identities(v) for k,v in prepared.items()}
        if result['identities_before'] != result['identities_after']: raise ValueError('identity changed')
        result['status']='complete'
        verify_pair(result)
    except BaseException as exc:
        result['status']='failed';result['error']=repr(exc)
        raise
    finally:
        b.save(args.output,result)


def verify_pair(data):
    if data['status']!='complete' or data['schema']!=1 or data['warmups']!=2 or data['runs']<18:
        raise ValueError('incomplete pair')
    if set(data['cases'])!=set(NAMES): raise ValueError('wrong cases')
    if data['identities_before']!=data['identities_after']: raise ValueError('identities changed')
    for key in ('platform','python','files','load','permitted_affinity'):
        if not data['host'][key]: raise ValueError('missing host evidence')
    if set(data['prepared']) != {'baseline', 'candidate'}: raise ValueError('wrong conditions')
    for label, prepared in data['prepared'].items():
        if prepared['status']!='prepared': raise ValueError('bad preparation')
        if type(prepared['cpu']) is not int or prepared['cpu'] not in prepared['host']['permitted_affinity']:
            raise ValueError('bad CPU')
        actual=data['identities_before'][label]
        for key in ('compiler','runtime','harness'):
            if prepared[key]!=actual[key]: raise ValueError('unlinked identity')
        for name in NAMES:
            verifier.verify_invocation(prepared, name, 'pythrust')
            row=prepared['sources'][name]
            if row['pythrust'] != actual[name+'/source/pythrust']:
                raise ValueError('unlinked timing source')
            original=b'pass\n' if name=='startup' else (b.ROOT/'bench'/(name+'.pyr')).read_bytes()
            if b.digest(original)!=row['pythrust']['sha256']: raise ValueError('changed workload')
            if prepared['conditions'][name]['pythrust']['executable']!=actual[name+'/pythrust']:
                raise ValueError('unlinked timing binary')
    first=data['prepared']['baseline'];second=data['prepared']['candidate']
    if first['compiler']==second['compiler']: raise ValueError('self comparison')
    if first['cpu']!=second['cpu']: raise ValueError('different CPU')
    left=first['source']['build_inputs'];right=second['source']['build_inputs']
    if set(left) != set(right): raise ValueError('added/deleted production build input')
    changed={k for k in left.keys() | right.keys() if left.get(k)!=right.get(k)}
    if changed != {'crates/pythrust-compiler/src/codegen/mod.rs', 'crates/pythrust-compiler/tests/conformance.rs'}: raise ValueError('unexpected production delta')
    for ordinal,(name,row) in enumerate(data['cases'].items()):
        if row['order']!=b.order(['baseline','candidate'],data['runs'],data['seed']+ordinal):
            raise ValueError('unreproducible order')
        wanted=[(i,k) for i,seq in enumerate(row['order']) for k in seq]
        if [(x['round'],x['kind']) for x in row['samples']]!=wanted: raise ValueError('missing trials')
        if [(x['kind'],x['warmup']) for x in row['warmup_samples']]!=[(k,i) for k in ('baseline','candidate') for i in range(2)]:
            raise ValueError('missing warmups')
        for item in row['samples']+row['warmup_samples']:
            if item['returncode']!=0 or item['stderr'] or item['stdout']!=b.EXPECTED[name]: raise ValueError('bad output/status')
            b.stats([item['wall_seconds']])
            for key in ('user_seconds','system_seconds'):
                if type(item[key]) not in (int,float) or not b.math.isfinite(item[key]) or item[key]<0:
                    raise ValueError('bad resource')
            if len(item['switches'])!=2 or any(type(x) is not int or x<0 for x in item['switches']):
                raise ValueError('bad switches')
        for label in ('baseline','candidate'):
            if row['summary'][label]!=b.stats([x['wall_seconds'] for x in row['samples'] if x['kind']==label]):
                raise ValueError('bad statistics')
    return len(data['cases'])


def profiles(args):
    """GNU in-image PC histograms and linked-edge counts, never ordinary timing."""
    prepared=json.loads((args.baseline/'prepared.json').read_text())
    args.work.mkdir()
    b.pin(prepared['cpu'])
    result=dict(status='running', command=sys.argv, prepared=prepared, host=b.host(),
                harness=b.identity(Path(__file__)), probe=b.identity(b.ROOT/'bench/profile_counts.c'),
                before=identities(prepared), cases={})
    cc=shutil.which('cc')
    for name in ('numeric','sorting','strings','tuples'):
        folder=args.work/name;folder.mkdir()
        source=folder/(name+'.pyr');source.write_text(prepared['sources'][name]['pythrust_source'])
        env=p.write_linker(folder/'pg-linker',cc,['-pg','-no-pie'])
        env['PYTHRUST_RT']=prepared['runtime']['path']
        b.run([prepared['compiler']['path'],'build',source],env=env)
        binary=source.with_suffix('')
        row=dict(profile_binary=b.identity(binary), source=b.identity(source),
                 profile=p.profile(binary,[],b.EXPECTED[name].encode(),folder,str(prepared['cpu']),7))
        asm=b.run(['objdump','-d',binary]).stdout.decode()
        (folder/'generated.s').write_text(asm)
        row['assembly']=b.identity(folder/'generated.s')
        obj=folder/'counts.o'
        b.run([cc,'-std=c11','-O2','-fno-builtin','-Wall','-Wextra','-Werror','-c',b.ROOT/'bench/profile_counts.c','-o',obj])
        wraps=['malloc','calloc','realloc','memcpy','memmove','pt_str_inc','pt_str_dec','pt_seq_inc','pt_seq_dec',
               'pt_seq_get','pt_seq_get_held','pt_seq_item_held','pt_seq_hold','pt_submit']
        env=p.write_linker(folder/'counts-linker',cc,[str(obj),'-Wl,'+','.join('--wrap='+x for x in wraps)])
        env['PYTHRUST_RT']=prepared['runtime']['path']
        b.run([prepared['compiler']['path'],'build',source],env=env)
        done=b.run([binary])
        if done.stdout!=b.EXPECTED[name].encode(): raise ValueError('probe output mismatch')
        row.update(count_binary=b.identity(binary), count_object=b.identity(obj), counts=p.parse_counts(done.stderr),
                   count_returncode=done.returncode, count_stdout=done.stdout.decode(), raw_counts=done.stderr.decode())
        result['cases'][name]=row
        b.save(args.output,result)
    result['after']=identities(prepared)
    if result['before']!=result['after']: raise ValueError('prepared binary changed')
    result['status']='complete';b.save(args.output,result)


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('action',choices=['prepare','pair','profiles','verify'])
    ap.add_argument('--baseline',type=Path)
    ap.add_argument('--work',type=Path)
    ap.add_argument('--compiler',type=Path)
    ap.add_argument('--output',type=Path)
    ap.add_argument('--runs',type=int,default=24)
    ap.add_argument('--seed',type=int,default=814)
    args=ap.parse_args()
    if args.action=='verify': print('Verified paired cases:',verify_pair(json.loads(args.output.read_text())));return
    if sys.version_info[:2]!=(3,13): ap.error('CPython 3.13 required')
    for key in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','PYTHRUST_RT_SRC','LD_PRELOAD'):
        if os.environ.get(key): ap.error('uncontrolled '+key)
    if not args.work.is_absolute() or not args.baseline.is_absolute(): ap.error('absolute paths required')
    {'prepare':prepare,'pair':pair,'profiles':profiles}[args.action](args)


if __name__=='__main__': main()
