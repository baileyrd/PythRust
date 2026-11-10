"""#815 same-source-path binary identity and ordinary string negative control."""
import json
from pathlib import Path
import shutil
import sys
import int_list_campaign as c
import three_way as b

NAMES=('numeric','strings','tuples','float_access','generic_float','task_float')


def collect(work,output):
    if output.exists():raise FileExistsError(output)
    root=work/'path-control-final';root.mkdir()
    p={k:json.loads((work/k/'prepared.json').read_text()) for k in c.LABELS}
    b.pin(p['baseline']['cpu'])
    record=dict(status='preparing',prepared=p,harness=b.identity(Path(__file__).resolve()),
                command=sys.argv,host=b.host(),cases={},runs=42,seed=81500,warmups=2)
    for name in NAMES:
        src=root/(name+'.pyr');src.write_text(c.sources()[name]);rows={}
        for label in c.LABELS:
            cmd=[p[label]['compiler']['path'],'build',str(src)]
            b.run(cmd,env=dict(b.os.environ,PYTHRUST_RT=p[label]['runtime']['path']))
            out=root/(name+'-'+label);shutil.copy2(src.with_suffix(''),out)
            b.sample([out],c.EXPECTED[name])
            rows[label]=dict(build_command=cmd,binary=b.identity(out),command=[str(out)])
        record['cases'][name]=dict(source=b.identity(src),conditions=rows,
            byte_identical=rows['baseline']['binary']['sha256']==rows['candidate']['binary']['sha256'])
    comparison=record['cases']['strings']['conditions']
    self_conditions={}
    for label in c.LABELS:
        out=root/('strings-self-'+label)
        shutil.copy2(Path(comparison['candidate']['binary']['path']),out)
        self_conditions[label]=dict(binary=b.identity(out),command=[str(out)])
    record['timings']={}
    record['before_quiet']=c.quiet()
    for phase,conditions in [('comparison',comparison),('self',self_conditions)]:
        row=dict(conditions=conditions,order=b.order(c.LABELS,42,81500),warmup_samples=[],samples=[],summary={})
        record['timings'][phase]=row
        for label in c.LABELS:
            for i in range(2):row['warmup_samples'].append(dict(kind=label,warmup=i,**b.sample(conditions[label]['command'],c.EXPECTED['strings'])))
        for i,sequence in enumerate(row['order']):
            for label in sequence:row['samples'].append(dict(kind=label,round=i,**b.sample(conditions[label]['command'],c.EXPECTED['strings'])))
            b.save(output,record)
        row['summary']={k:b.stats([s['wall_seconds'] for s in row['samples'] if s['kind']==k]) for k in c.LABELS}
        print(phase,row['summary'],flush=True)
    record.update(status='complete',after_quiet=c.quiet(),end_host=b.host())
    record['post_binaries']={n:{k:b.identity(Path(x['binary']['path'])) for k,x in v['conditions'].items()} for n,v in record['cases'].items()}
    record['self_post']={k:b.identity(Path(x['binary']['path'])) for k,x in self_conditions.items()}
    verify(record);b.save(output,record)


def verify(data, canonical=None):
    c.verify_prepared(data['prepared'],c.canonical_prepared() if canonical is None else canonical)
    c.valid_identity(data['harness'])
    if data['harness']!=c.frozen()['path_harness']:raise ValueError('unmatched path-control harness')
    if data['status']!='complete' or set(data['cases'])!=set(NAMES) or data['runs']!=42 or data['seed']!=81500 or data['warmups']!=2:raise ValueError('incomplete path control')
    for name,row in data['cases'].items():
        c.valid_identity(row['source'])
        if row['source']['sha256']!=c.frozen()['workloads'][name]['sha256']:raise ValueError('path input')
        if set(row['conditions'])!=set(c.LABELS):raise ValueError('path conditions')
        for label,item in row['conditions'].items():
            c.valid_identity(item['binary'])
            c.valid_identity(data['post_binaries'][name][label])
            if item['build_command']!=[data['prepared'][label]['compiler']['path'],'build',row['source']['path']]:raise ValueError('path build')
            if item['command']!=[item['binary']['path']] or item['binary']!=data['post_binaries'][name][label]:raise ValueError('path execution/identity')
        a=row['conditions']['baseline']['binary']['sha256'];d=row['conditions']['candidate']['binary']['sha256']
        if row['byte_identical']!=(a==d):raise ValueError('wrong byte identity claim')
    if set(data['timings'])!={'comparison','self'}:raise ValueError('missing diagnostic phases')
    for phase,row in data['timings'].items():
        if set(row['conditions'])!=set(c.LABELS):raise ValueError('diagnostic conditions')
        if phase=='comparison':
            if row['conditions']!=data['cases']['strings']['conditions']:raise ValueError('unlinked comparison')
        else:
            candidate=data['cases']['strings']['conditions']['candidate']['binary']['sha256']
            for label,item in row['conditions'].items():
                c.valid_identity(item['binary'])
                c.valid_identity(data['self_post'][label])
                if item['binary']['sha256']!=candidate or item['binary']!=data['self_post'][label]:raise ValueError('not identical-byte self control')
                if item['command']!=[item['binary']['path']]:raise ValueError('self execution')
        if row['order']!=b.order(c.LABELS,42,81500):raise ValueError('unbalanced diagnostic')
        if [(s['round'],s['kind']) for s in row['samples']]!=[(i,k) for i,seq in enumerate(row['order']) for k in seq]:raise ValueError('missing diagnostic trials')
        if [(s['kind'],s['warmup']) for s in row['warmup_samples']]!=[(k,i) for k in c.LABELS for i in range(2)]:raise ValueError('missing diagnostic warmups')
        for s in row['samples']+row['warmup_samples']:
            if s['stdout']!=c.frozen()['workloads']['strings']['expected'] or s['stderr'] or s['returncode']!=0:raise ValueError('diagnostic parity')
            b.stats([s['wall_seconds']])
            for key in ('user_seconds','system_seconds'):
                if type(s[key]) not in (int,float) or not b.math.isfinite(s[key]) or s[key]<0:raise ValueError('diagnostic resources')
            if len(s['switches'])!=2 or any(type(x) is not int or x<0 for x in s['switches']):raise ValueError('diagnostic switches')
        for label in c.LABELS:
            if row['summary'][label]!=b.stats([s['wall_seconds'] for s in row['samples'] if s['kind']==label]):raise ValueError('diagnostic statistics')
    return len(NAMES)


if __name__=='__main__':
    if sys.argv[1]=='collect':collect(Path(sys.argv[2]),Path(sys.argv[3]))
    else: print('Verified path controls:',verify(json.loads(Path(sys.argv[3]).read_text())))
