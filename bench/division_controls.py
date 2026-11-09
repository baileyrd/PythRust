"""#814 ordinary list controls and source-level inlining diagnostic.

The tuple source variant estimates a lead; it is not a compiler transformation
or a replacement primary workload. Lists retain current homogeneous rules.
"""
import json
import os
from pathlib import Path
import sys
import three_way as b
import int_list_access as lists
import division_campaign as c


def prepare(base_dir,candidate_dir,work):
    work.mkdir()
    manifests={k:json.loads((d/'prepared.json').read_text()) for k,d in [('baseline',base_dir),('candidate',candidate_dir)]}
    record=dict(status='prepared',host=b.host(),harness=b.identity(Path(__file__)),manifests=manifests,cases={},refused={})
    b.pin(manifests['baseline']['cpu'])
    sources=lists.sources()
    sources={k:sources[k] for k in ('int_access','float_access')}
    sources['generic_access']=sources['int_access'].replace('xs: list[int]','xs').replace(' -> int','')
    for name,source in sources.items():
        conditions={}
        oracle=None
        for label,manifest in manifests.items():
            folder=work/(name+'-'+label);folder.mkdir()
            path=folder/(name+'.pyr');path.write_text(source)
            env=dict(os.environ,PYTHRUST_RT=manifest['runtime']['path'])
            b.run([manifest['compiler']['path'],'build',path],env=env)
            if oracle is None: oracle=b.run([sys.executable,'-B',path]).stdout.decode()
            b.sample([path.with_suffix('')],oracle)
            conditions[label]=dict(command=[str(path.with_suffix(''))],binary=b.identity(path.with_suffix('')),source=b.identity(path))
        record['cases'][name]=dict(source=source,expected=oracle,conditions=conditions)
    source=manifests['baseline']['sources']['tuples']['pythrust_source'].replace('        n, d = div_mod(n, 10)','        quotient = n // 10\n        d = n % 10\n        n = quotient')
    path=work/'tuples_source_inline.pyr';path.write_text(source)
    manifest=manifests['baseline'];env=dict(os.environ,PYTHRUST_RT=manifest['runtime']['path'])
    b.run([manifest['compiler']['path'],'build',path],env=env)
    b.sample([path.with_suffix('')],b.EXPECTED['tuples'])
    original=manifest['conditions']['tuples']['pythrust']
    record['cases']['source_inline_diagnostic']=dict(source=source,expected=b.EXPECTED['tuples'],conditions={
        'baseline':dict(command=original['command'],binary=original['executable'],source=manifest['sources']['tuples']['pythrust']),
        'candidate':dict(command=[str(path.with_suffix(''))],binary=b.identity(path.with_suffix('')),source=b.identity(path))})
    for name in ['mixed','dyn']:
        path=work/(name+'.pyr');path.write_text('xs = [1, 2.0]\nprint(xs)\n' if name=='mixed' else 'xs: list[dyn] = [1, 2.0]\nprint(xs)\n')
        done=b.subprocess.run([manifest['compiler']['path'],'check',path],capture_output=True)
        if done.returncode!=1: raise ValueError('unsupported control unexpectedly accepted')
        record['refused'][name]=dict(source=path.read_text(),returncode=done.returncode,stdout=done.stdout.decode(),stderr=done.stderr.decode())
    record['identities']={name:{label:{'binary':b.identity(Path(row['binary']['path'])),'source':b.identity(Path(row['source']['path']))}
                                  for label,row in case['conditions'].items()} for name,case in record['cases'].items()}
    b.save(work/'prepared.json',record)


def measure(work,output,runs=24,seed=8140):
    if output.exists(): raise FileExistsError(output)
    prepared=json.loads((work/'prepared.json').read_text())
    b.pin(prepared['manifests']['baseline']['cpu'])
    result=dict(status='running',prepared=prepared,runs=runs,seed=seed,cases={},host=b.host(),command=sys.argv)
    for ordinal,(name,case) in enumerate(prepared['cases'].items()):
        row=dict(order=b.order(['baseline','candidate'],runs,seed+ordinal),samples=[],warmup_samples=[],summary={})
        result['cases'][name]=row
        for label,condition in case['conditions'].items():
            if b.identity(Path(condition['binary']['path']))!=condition['binary']: raise ValueError('stale binary')
            for i in range(2): row['warmup_samples'].append(dict(kind=label,warmup=i,**b.sample(condition['command'],case['expected'])))
        for i,sequence in enumerate(row['order']):
            for label in sequence: row['samples'].append(dict(round=i,kind=label,**b.sample(case['conditions'][label]['command'],case['expected'])))
            b.save(output,result)
        row['summary']={label:b.stats([x['wall_seconds'] for x in row['samples'] if x['kind']==label]) for label in ('baseline','candidate')}
        print(name,row['summary'],flush=True)
    result['identities_after']={name:{label:{'binary':b.identity(Path(row['binary']['path'])),'source':b.identity(Path(row['source']['path']))}
                                  for label,row in case['conditions'].items()} for name,case in prepared['cases'].items()}
    if result['identities_after']!=prepared['identities']: raise ValueError('controls changed')
    result['status']='complete';b.save(output,result)


if __name__=='__main__':
    action=sys.argv[1]
    if action=='prepare': prepare(*map(Path,sys.argv[2:]))
    elif action=='measure': measure(*map(Path,sys.argv[2:]))
    else: raise ValueError('prepare or measure required')
