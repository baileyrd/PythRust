"""#815 typed integer-list experiment. Ordinary timings exclude all probes/builds.

Uses #814's balanced order, resource sampling, hash and host helpers. Sources,
commands, tool/build identities and every warmup/trial are retained. No startup
subtraction. Prepare baseline before any production edits, candidate afterwards.
"""
import argparse
from datetime import datetime, timezone
import json
import os
import re
from pathlib import Path
import shutil
import sys

import three_way as b
import int_list_access as old
import verify_three_way as v

BASE = '626a3487223bc48569b627541da5920a653508aa'
LABELS = ('baseline', 'candidate')
EXPECTED = dict(b.EXPECTED, int_access='28000000\n', float_access='28000000.0\n', generic_int='28000000\n', generic_float='28000000.0\n', task_int='56000000\n', task_float='56000000.0\n')
DELTA = {'crates/pythrust-compiler/src/codegen/mod.rs',
         'crates/pythrust-compiler/src/codegen/seq.rs', 'crates/pythrust-rt/src/seq.rs'}
ENV_KEYS = ('PATH', 'CARGO_BUILD_JOBS', 'CARGO_TARGET_DIR', 'RUSTFLAGS',
            'CARGO_ENCODED_RUSTFLAGS', 'PYTHRUST_RT', 'PYTHRUST_RT_SRC',
            'LD_PRELOAD', 'LD_LIBRARY_PATH', 'PYTHONHASHSEED', 'MALLOC_CONF', 'GLIBC_TUNABLES')


def sources():
    result = old.sources()
    result['generic_int'] = result['int_access'].replace('xs: list[int]', 'xs').replace(' -> int', '')
    result['generic_float'] = result['float_access'].replace('xs: list[float]', 'xs').replace(' -> float', '')
    for ty in ('int', 'float'):
        result['task_' + ty] = ('import concurrent.futures\n' + result[ty + '_access'].split('print(access(')[0]
            + '\ndef main() -> None:\n    xs = [' + ('7' if ty == 'int' else '7.0') + '''] * 16
    with concurrent.futures.ThreadPoolExecutor() as ex:
        a = ex.submit(read, xs)
        c = ex.submit(read, xs)
        print(a.result() + c.result())

main()
''').replace('def access(', 'def read(').replace('        xs[k] = '+('7' if ty == 'int' else '7.0')+'\n', '')
    return result


# Historical acceptance uses this immutable Git-object export, never future
# production/workload files or whichever checkout happens to run the tests.
SOURCE_MANIFEST_SHA256 = 'd30b154127688a98e405445f862da626e6a65a71e32db63169790da75bf40221'
REVIEWED = '83d06f3905e4ce296e924658481d4777ea096aa7'


def evidence_root():
    return Path(__file__).resolve().parents[1]/'docs/benchmarks'


def frozen():
    raw=(evidence_root()/'card815-source-manifest.json').read_bytes()
    if b.digest(raw)!=SOURCE_MANIFEST_SHA256:
        raise ValueError('changed #815 source export')
    return json.loads(raw)


def valid_identity(item):
    if not isinstance(item,dict) or set(item)!={'path','sha256'}:
        raise ValueError('incomplete identity')
    if not isinstance(item['path'],str) or not item['path'].startswith('/'):
        raise ValueError('invalid identity path')
    if not isinstance(item['sha256'],str) or not re.fullmatch('[0-9a-f]{64}',item['sha256']):
        raise ValueError('invalid identity digest')


def valid_tools(tools, names):
    if not isinstance(tools,dict) or set(tools)!=set(names):
        raise ValueError('incomplete tool inventory')
    for tool in tools.values():
        if not isinstance(tool,dict) or set(tool)!={'identity','version'}:
            raise ValueError('incomplete tool record')
        valid_identity(tool['identity'])
        if not isinstance(tool['version'],str) or not tool['version'].strip():
            raise ValueError('missing tool version')


def canonical_prepared():
    return json.loads((evidence_root()/'card815-pair24.json').read_text())['prepared']


def verify_prepared(prepared, canonical=None):
    export=frozen()
    if set(prepared)!=set(LABELS):raise ValueError('preparation conditions')
    for label,p in prepared.items():
        source=export['versions'][label]
        if p['source']['head']!=BASE or p['source']['tree']!=export['versions']['baseline']['tree']:
            raise ValueError('wrong admitted source checkpoint')
        if p['source']['build_inputs']!=source['inputs']:
            raise ValueError('incomplete or mismatched production source inventory')
        valid_tools(p['tools'],('cargo','rustc','cc','ld'))
        if p['tools']!=export['tools'][label]:raise ValueError('unmatched measured tools')
        for key in ('compiler','runtime','harness'):
            valid_identity(p[key])
            if p[key]!=export['artifacts'][label][key]:raise ValueError('unmatched measured '+key)
        if set(p['cases'])!=set(export['workloads']):raise ValueError('preparation case inventory')
        for name,row in p['cases'].items():
            valid_identity(row['source_identity'])
            valid_identity(row['binary'])
            if row['source']!=export['workloads'][name]['source'] or row['source_identity']['sha256']!=export['workloads'][name]['sha256']:
                raise ValueError('unmatched frozen workload')
    if canonical is not None and prepared!=canonical:
        raise ValueError('preparation differs from canonical primary campaign')


def quiet():
    return old.quiet()


def prepare(work, label, build_log):
    if sys.version_info[:2] != (3, 13):
        raise ValueError('CPython 3.13 required')
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'PYTHRUST_RT_SRC', 'LD_PRELOAD'):
        if os.environ.get(key):
            raise ValueError('uncontrolled '+key)
    folder = work / label
    folder.mkdir()
    record = dict(status='preparing', label=label, source=b.source_record(), host=b.host(),
                  cpu=min(os.sched_getaffinity(0)), timestamp=datetime.now(timezone.utc).isoformat(),
                  environment={k:os.environ.get(k) for k in ENV_KEYS},
                  build_command=['cargo','build','--locked','--release','-p','pythrust-compiler','-p','pythrust-rt'],
                  build_log=b.identity(build_log), build_log_text=build_log.read_text(),
                  harness=b.identity(Path(__file__).resolve()), tools={}, cases={}, refused={})
    for name in ('cargo', 'rustc', 'cc', 'ld'):
        path=Path(shutil.which(name)).resolve()
        record['tools'][name]=dict(identity=b.identity(path), version=b.run([name,'-Vv' if name=='rustc' else '--version']).stdout.decode())
    for name in ('pythrust', 'libpythrust_rt.a'):
        shutil.copy2(b.ROOT/'target/release'/name, folder/name)
    record['compiler']=b.identity(folder/'pythrust')
    record['runtime']=b.identity(folder/'libpythrust_rt.a')
    inputs=work/'inputs'; inputs.mkdir(exist_ok=True)
    env=dict(os.environ, PYTHRUST_RT=record['runtime']['path'])
    for name,source in sources().items():
        src=inputs/(name+'.pyr')
        if src.exists() and src.read_text()!=source:
            raise ValueError('source changed')
        src.write_text(source)
        command=[record['compiler']['path'],'build',str(src)]
        b.run(command,env=env)
        out=folder/name
        shutil.copy2(src.with_suffix(''),out)
        oracle=b.run([sys.executable,'-B',src])
        if oracle.stderr: raise ValueError('oracle stderr')
        expected=oracle.stdout.decode()
        b.sample([out],expected)
        small=inputs/(name+'-small.pyr')
        small.write_text(source.replace('4000000','400').replace('1000000','1000').replace('300000','100'))
        ref=b.run([sys.executable,'-B',small]); interpreted=b.run([folder/'pythrust','run',small],env=env)
        if ref.stdout!=interpreted.stdout or ref.stderr or interpreted.stderr:
            raise ValueError('interpreter parity: '+name)
        record['cases'][name]=dict(source=source, source_identity=b.identity(src),
            build_command=command, build_output=b.identity(src.with_suffix('')), binary=b.identity(out),
            command=[str(out)], expected=expected,
            interpreter=dict(source=small.read_text(), oracle=ref.stdout.decode(), result=interpreted.stdout.decode()))
    for name,source in dict(mixed='xs = [1, 2.0]\nprint(xs)\n', dyn='xs: list[dyn] = [1, 2.0]\nprint(xs)\n').items():
        src=inputs/(name+'.pyr');src.write_text(source)
        command=[record['compiler']['path'],'check',str(src)]
        done=b.subprocess.run(command,capture_output=True)
        record['refused'][name]=dict(source=source, command=command, source_identity=b.identity(src),
            returncode=done.returncode, stdout=done.stdout.decode(), stderr=done.stderr.decode())
    record['status']='prepared'
    b.save(folder/'prepared.json',record)
    return record


def identities(p):
    result={k:b.identity(Path(p[k]['path'])) for k in ('compiler','runtime','harness')}
    result['cases']={n:{k:b.identity(Path(c[k]['path'])) for k in ('binary','source_identity')}
                     for n,c in p['cases'].items()}
    return result


def measure(work, output, runs, seed):
    if output.exists(): raise FileExistsError(output)
    prepared={k:json.loads((work/k/'prepared.json').read_text()) for k in LABELS}
    data=dict(schema=1,status='running',prepared=prepared,runs=runs,seed=seed,warmups=2,
              host=b.host(), timestamp=datetime.now(timezone.utc).isoformat(), command=sys.argv,
              before_quiet=quiet(),before={k:identities(p) for k,p in prepared.items()},cases={})
    b.pin(prepared['baseline']['cpu'])
    try:
        for ordinal,name in enumerate(sources()):
            expected=prepared['baseline']['cases'][name]['expected']
            row=dict(order=b.order(LABELS,runs,seed+ordinal),samples=[],warmup_samples=[],summary={})
            data['cases'][name]=row
            for label in LABELS:
                for i in range(2):
                    row['warmup_samples'].append(dict(kind=label,warmup=i,**b.sample(prepared[label]['cases'][name]['command'],expected)))
            for i,sequence in enumerate(row['order']):
                for label in sequence:
                    row['samples'].append(dict(kind=label,round=i,**b.sample(prepared[label]['cases'][name]['command'],expected)))
                b.save(output,data)
            row['summary']={k:b.stats([s['wall_seconds'] for s in row['samples'] if s['kind']==k]) for k in LABELS}
            print(name,row['summary'],flush=True)
        data['after']={k:identities(p) for k,p in prepared.items()}
        data['after_quiet']=quiet();data['end_host']=b.host();data['status']='complete'
        verify(data)
    except BaseException as exc:
        data['status']='failed';data['error']=repr(exc)
        raise
    finally: b.save(output,data)


def verify(data):
    verify_prepared(data['prepared'])
    workloads=frozen()['workloads']
    if data['schema']!=1 or data['status']!='complete' or data['runs']<24 or data['warmups']!=2:
        raise ValueError('incomplete campaign')
    if set(data['prepared'])!=set(LABELS) or set(data['cases'])!=set(workloads):
        raise ValueError('wrong cases/conditions')
    if data['before']!=data['after']: raise ValueError('changed timing identities')
    for label,p in data['prepared'].items():
        if p['status']!='prepared' or p['label']!=label or p['source']['head']!=BASE:
            raise ValueError('wrong baseline/preparation')
        if p['build_command']!=['cargo','build','--locked','--release','-p','pythrust-compiler','-p','pythrust-rt']:
            raise ValueError('wrong build settings')
        if set(p['environment'])!=set(ENV_KEYS) or any(p['environment'][k] for k in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','PYTHRUST_RT_SRC','LD_PRELOAD')):
            raise ValueError('uncontrolled build environment')
        if b.digest(p['build_log_text'].encode())!=p['build_log']['sha256'] or 'Finished `release`' not in p['build_log_text']:
            raise ValueError('missing release build log')
        for key in ('compiler','runtime','harness'):
            valid_identity(p[key])
            if p[key]!=data['before'][label][key]: raise ValueError('unlinked '+key)
        for name,fixed in workloads.items():
            source=fixed['source']
            c=p['cases'][name]
            for key in ('binary','source_identity','build_output'): valid_identity(c[key])
            if c['source']!=source or c['source_identity']['sha256']!=b.digest(source.encode()): raise ValueError('changed input')
            if c['command']!=[c['binary']['path']]: raise ValueError('execution identity/arguments')
            if c['build_command']!=[p['compiler']['path'],'build',c['source_identity']['path']]: raise ValueError('build identity')
            if c['build_output']['path']!=str(Path(c['source_identity']['path']).with_suffix('')) or c['build_output']['sha256']!=c['binary']['sha256']:
                raise ValueError('source/build/output linkage')
            if any(c[k]!=data['before'][label]['cases'][name][k] for k in ('binary','source_identity')): raise ValueError('unlinked case')
            if c['expected']!=fixed['expected']: raise ValueError('oracle differs')
            if c['interpreter']['oracle']!=c['interpreter']['result']: raise ValueError('interpreter parity')
        for name,r in p['refused'].items():
            if name not in ('mixed','dyn') or r['returncode']!=1 or r['stdout'] or not r['stderr']:
                raise ValueError('unsupported control status')
            if r['command']!=[p['compiler']['path'],'check',r['source_identity']['path']] or b.digest(r['source'].encode())!=r['source_identity']['sha256']:
                raise ValueError('unsupported control provenance')
        if set(p['refused'])!={'mixed','dyn'}: raise ValueError('missing unsupported controls')
    left=data['prepared']['baseline'];right=data['prepared']['candidate']
    if left['cpu']!=right['cpu'] or left['cpu'] not in data['host']['permitted_affinity']: raise ValueError('CPU mismatch')
    a=left['source']['build_inputs'];c=right['source']['build_inputs']
    if set(a)!=set(c) or {k for k in a if a[k]!=c[k]}!=DELTA: raise ValueError('unexpected source delta')
    if any('diff --git a/'+prefix in left['source'][key] for prefix in ('crates/', 'Cargo.') for key in ('tracked_diff','index_diff')): raise ValueError('dirty production baseline')
    for key in ('tools','environment'):
        if left[key]!=right[key]: raise ValueError('build controls differ')
    for p in (left,right):
        if not p['host']['files'] or not p['host']['load']: raise ValueError('missing host evidence')
        for tool in p['tools'].values():
            valid_identity(tool['identity'])
            if not tool['version']: raise ValueError('missing tool version')
    for ordinal,(name,row) in enumerate(data['cases'].items()):
        if row['order']!=b.order(LABELS,data['runs'],data['seed']+ordinal): raise ValueError('unbalanced order')
        if [(s['round'],s['kind']) for s in row['samples']]!=[(i,k) for i,seq in enumerate(row['order']) for k in seq]: raise ValueError('missing trials')
        if [(s['kind'],s['warmup']) for s in row['warmup_samples']]!=[(k,i) for k in LABELS for i in range(2)]: raise ValueError('missing warmups')
        for s in row['samples']+row['warmup_samples']:
            if s['returncode']!=0 or s['stderr'] or s['stdout']!=left['cases'][name]['expected']: raise ValueError('sample output/status')
            b.stats([s['wall_seconds']])
            for key in ('user_seconds','system_seconds'):
                if type(s[key]) not in (int,float) or not b.math.isfinite(s[key]) or s[key]<0: raise ValueError('bad resources')
            if len(s['switches'])!=2 or any(type(x) is not int or x<0 for x in s['switches']): raise ValueError('bad switches')
        for label in LABELS:
            if row['summary'][label]!=b.stats([s['wall_seconds'] for s in row['samples'] if s['kind']==label]): raise ValueError('bad summary')
    return len(data['cases'])


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('action',choices=['prepare','measure','verify'])
    ap.add_argument('--work',type=Path);ap.add_argument('--label',choices=LABELS)
    ap.add_argument('--build-log',type=Path);ap.add_argument('--output',type=Path)
    ap.add_argument('--runs',type=int,default=24);ap.add_argument('--seed',type=int,default=815)
    args=ap.parse_args()
    if args.action=='prepare': prepare(args.work,args.label,args.build_log)
    elif args.action=='measure': measure(args.work,args.output,args.runs,args.seed)
    else: print('Verified cases:',verify(json.loads(args.output.read_text())))
